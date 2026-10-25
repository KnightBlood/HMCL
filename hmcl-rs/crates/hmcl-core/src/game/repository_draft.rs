//! The exclusive write session of a game directory.
//!
//! A draft collects the changes to the manifests of the instances of a game
//! directory and applies them to the disk at once. Until the draft is
//! committed, the published index of the repository is untouched: the
//! launcher keeps showing the instances it has while a new instance is
//! installed or an existing one is changed. A commit that fails on the disk
//! restores the files it had already replaced, so a half written instance is
//! not left behind.
//!
//! The shared libraries, assets and download caches are outside the rollback
//! boundary: a draft never reverts them.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::download::game::InstanceManifest;

use super::repository::GameRepository;
use super::repository_snapshot::RepositorySnapshot;

/// The directory the repository keeps its draft scratch files in.
pub const DRAFT_DIRECTORY: [&str; 2] = [".hmcl", "repository-drafts"];

/// The lifecycle state of a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftState {
    /// The draft accepts changes and may be committed or aborted.
    Open,
    /// The draft is applying its files and publishing the successor index.
    Committing,
    /// The draft completed its commit.
    Committed,
    /// The draft discarded its changes.
    Aborted,
    /// The draft could not complete a commit or a cleanup.
    Failed,
}

impl DraftState {
    /// Returns the name of the state, as the messages of the errors use it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Committing => "committing",
            Self::Committed => "committed",
            Self::Aborted => "aborted",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for DraftState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The error of a draft operation.
#[derive(Debug)]
pub enum DraftError {
    /// The draft no longer accepts the operation.
    NotOpen {
        /// The state the draft was in.
        state: DraftState,
    },
    /// The draft does not hold the instance.
    UnknownInstance {
        /// The identifier that is not in the draft.
        id: String,
    },
    /// The manifest the draft was given has no identifier.
    MissingId,
    /// The identifier cannot name an instance.
    InvalidId {
        /// The identifier that cannot name an instance.
        id: String,
    },
    /// The draft already holds an instance with the identifier.
    InstanceExists {
        /// The identifier that is already taken.
        id: String,
    },
    /// The instance was created by the same draft, so it has no directory to
    /// rename yet.
    CreatedInstance {
        /// The identifier of the instance the draft created.
        id: String,
    },
    /// The directory of an instance that the draft wanted to reserve exists.
    AlreadyExists {
        /// The directory that exists.
        path: PathBuf,
    },
    /// A file the draft needs is not a regular file.
    NotRegularFile {
        /// The file that is not regular.
        path: PathBuf,
    },
    /// A file the draft would write is outside of the directory of its
    /// instance.
    EscapesInstanceRoot {
        /// The file that is outside of the instance directory.
        path: PathBuf,
    },
    /// A file or directory cannot be read, written or moved.
    Write {
        /// The file or directory that cannot be written.
        path: PathBuf,
        /// The reason it cannot be written.
        error: std::io::Error,
    },
}

impl fmt::Display for DraftError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOpen { state } => {
                write!(formatter, "The draft is {state} and accepts no change")
            }
            Self::UnknownInstance { id } => {
                write!(formatter, "The draft does not hold the instance {id}")
            }
            Self::MissingId => formatter.write_str("The manifest has no instance id"),
            Self::InvalidId { id } => {
                write!(formatter, "The id {id} cannot name an instance")
            }
            Self::InstanceExists { id } => {
                write!(formatter, "The draft already holds the instance {id}")
            }
            Self::CreatedInstance { id } => write!(
                formatter,
                "The instance {id} was created by the same draft and cannot be renamed"
            ),
            Self::AlreadyExists { path } => {
                write!(formatter, "The path {} already exists", path.display())
            }
            Self::NotRegularFile { path } => {
                write!(
                    formatter,
                    "The path {} is not a regular file",
                    path.display()
                )
            }
            Self::EscapesInstanceRoot { path } => write!(
                formatter,
                "The path {} is outside of its instance directory",
                path.display()
            ),
            Self::Write { path, error } => {
                write!(
                    formatter,
                    "The path {} cannot be written: {error}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DraftError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// An exclusive session over the instances of a game directory.
///
/// The draft borrows the repository, so the borrow checker keeps the
/// repository from being refreshed or written while the draft is open. The
/// manifests the draft holds are kept in memory until [`Self::commit`];
/// dropping a draft that was not committed discards its changes.
pub struct GameRepositoryDraft<'a> {
    /// The repository the draft publishes its successor index to.
    repository: &'a mut GameRepository,
    /// The published index the draft started from.
    base: RepositorySnapshot,
    /// The unpublished manifests, by identifier.
    manifests: BTreeMap<String, InstanceManifest>,
    /// The identifiers whose final manifest differs from the base index.
    modified: BTreeSet<String>,
    /// The completed client files to copy while committing, by identifier.
    primary_jars: BTreeMap<String, PathBuf>,
    /// The identifiers whose directory did not exist before the draft.
    created: BTreeSet<String>,
    /// The identifiers that are absent from the final manifest set.
    removed: BTreeSet<String>,
    /// The manifest files of the instances the draft renamed, by their new
    /// identifiers.
    renamed_manifests: BTreeMap<String, PathBuf>,
    /// The renames to apply to the disk while committing, in order.
    renames: Vec<(String, String)>,
    /// The lifecycle state of the draft.
    state: DraftState,
}

impl GameRepository {
    /// Opens the write session of the repository.
    ///
    /// The repository borrows the draft, so only one draft can be open and no
    /// other write can happen until the draft is committed or dropped.
    pub fn open_draft(&mut self) -> GameRepositoryDraft<'_> {
        let base = self.snapshot.clone();
        let manifests = base.manifests.clone();

        GameRepositoryDraft {
            repository: self,
            base,
            manifests,
            modified: BTreeSet::new(),
            primary_jars: BTreeMap::new(),
            created: BTreeSet::new(),
            removed: BTreeSet::new(),
            renamed_manifests: BTreeMap::new(),
            renames: Vec::new(),
            state: DraftState::Open,
        }
    }

    /// Writes the manifest of an instance and publishes a new index in one
    /// commit.
    ///
    /// An instance the repository does not hold is created from the manifest;
    /// the directory of a new instance is reserved but only created while the
    /// commit applies its files.
    pub fn save(&mut self, manifest: InstanceManifest) -> Result<(), super::RepositoryError> {
        let mut draft = self.open_draft();
        draft
            .put(manifest)
            .map_err(|error| super::RepositoryError::Commit {
                error: Box::new(error),
            })?;
        draft
            .commit()
            .map_err(|error| super::RepositoryError::Commit {
                error: Box::new(error),
            })?;
        Ok(())
    }

    /// Removes an instance from the index and from the disk.
    ///
    /// An instance the index holds is removed through a draft, so the new
    /// index is published once and the directory is deleted afterwards. A
    /// directory the index does not hold is removed directly and the
    /// repository is refreshed. The directory is deleted rather than moved to
    /// the trash of the operating system.
    ///
    /// Returns whether the instance is gone; a directory that cannot be
    /// removed is reported as `false`.
    pub fn remove_instance_from_disk(&mut self, id: &str) -> bool {
        if self.contains(id) {
            let mut draft = self.open_draft();
            if draft.remove(id).is_err() {
                return false;
            }
            return draft.commit().is_ok();
        }

        let root = self.instance_root(id);
        let removed = match root.file_name().and_then(|name| name.to_str()) {
            Some(name) => root.with_file_name(format!("{name}_removed")),
            None => {
                self.refresh();
                return false;
            }
        };

        if root.exists() {
            if std::fs::rename(&root, &removed).is_err() {
                self.refresh();
                return false;
            }
            if std::fs::remove_dir_all(&removed).is_err() {
                self.refresh();
                return false;
            }
        }

        self.refresh();
        true
    }
}

impl<'a> GameRepositoryDraft<'a> {
    /// Returns the repository the draft publishes to.
    pub fn repository(&self) -> &GameRepository {
        self.repository
    }

    /// Returns the published index the draft started from.
    pub fn base_snapshot(&self) -> &RepositorySnapshot {
        &self.base
    }

    /// Returns the state of the draft.
    pub fn state(&self) -> DraftState {
        self.state
    }

    /// Returns whether the draft still accepts changes.
    pub fn is_open(&self) -> bool {
        self.state == DraftState::Open
    }

    /// Returns whether the draft committed its changes.
    pub fn is_committed(&self) -> bool {
        self.state == DraftState::Committed
    }

    /// Adds or replaces the manifest of an instance.
    ///
    /// The manifest stays in memory until the commit, so nothing is written
    /// and no instance directory is created by this method. An identifier the
    /// repository does not hold yet reserves the directory of the instance;
    /// the directory has to be free, because the draft would remove it when it
    /// is discarded.
    pub fn put(&mut self, manifest: InstanceManifest) -> Result<(), DraftError> {
        self.check_open()?;
        self.put_manifest(manifest, true)
    }

    /// Records a completed client file to copy into an instance while
    /// committing.
    ///
    /// The source stays where it is; the commit copies it to the file of the
    /// game of the instance. The source has to be a regular file, and the
    /// target has to be inside the directory of the instance.
    pub fn put_primary_jar(&mut self, id: &str, source: &Path) -> Result<(), DraftError> {
        self.check_open()?;
        if !self.manifests.contains_key(id) {
            return Err(DraftError::UnknownInstance { id: id.to_string() });
        }

        let source = absolute(source);
        if !source.is_file() {
            return Err(DraftError::NotRegularFile { path: source });
        }

        let target = self.primary_jar_target(id)?;
        if source == target {
            self.primary_jars.remove(id);
        } else {
            self.primary_jars.insert(id.to_string(), source);
        }
        Ok(())
    }

    /// Removes an instance from the draft.
    ///
    /// The published instance and its files stay untouched until the draft is
    /// committed; the directory is moved into the scratch directory of the
    /// commit and removed once the new index is published.
    pub fn remove(&mut self, id: &str) -> Result<(), DraftError> {
        self.check_open()?;
        if self.manifests.remove(id).is_none() {
            return Err(DraftError::UnknownInstance { id: id.to_string() });
        }

        self.modified.remove(id);
        self.primary_jars.remove(id);
        self.removed.insert(id.to_string());
        Ok(())
    }

    /// Renames an instance in the draft.
    ///
    /// The versions that inherit from the instance directly are updated in the
    /// same draft. The directory of the instance stays at its published
    /// location until the commit. An instance the same draft created cannot be
    /// renamed, because its directory is not materialized yet.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), DraftError> {
        self.check_open()?;

        let mut renamed = match self.manifests.get(from) {
            Some(manifest) => manifest.clone(),
            None => {
                return Err(DraftError::UnknownInstance {
                    id: from.to_string(),
                })
            }
        };
        if self.created.contains(from) {
            return Err(DraftError::CreatedInstance {
                id: from.to_string(),
            });
        }
        if self.manifests.contains_key(to) {
            return Err(DraftError::InstanceExists { id: to.to_string() });
        }
        if !super::repository::is_valid_instance_id(to) {
            return Err(DraftError::InvalidId { id: to.to_string() });
        }

        let target_root = self.base.instance_root(to);
        if target_root.exists() {
            return Err(DraftError::AlreadyExists { path: target_root });
        }

        // A client owned by the renamed instance moves with it; a reference to
        // the version the instance inherited from follows `inheritsFrom`
        // instead, so the reference is dropped.
        if renamed.jar.as_deref() == Some(from) {
            renamed.jar = None;
        }
        renamed.id = Some(to.to_string());

        self.manifests.remove(from);
        self.modified.remove(from);
        self.removed.remove(from);
        self.put_manifest(renamed, false)?;

        if let Some(source) = self.primary_jars.remove(from) {
            self.primary_jars.insert(to.to_string(), source);
        }

        // The instances that inherit from the renamed one follow it, so the
        // launcher does not lose the chain of the instance.
        for (id, manifest) in &mut self.manifests {
            if manifest.inherits_from.as_deref() != Some(from) {
                continue;
            }
            manifest.inherits_from = Some(to.to_string());
            self.modified.insert(id.clone());
        }

        self.renames.push((from.to_string(), to.to_string()));

        // The files move with the directory: a manifest named after the
        // instance is renamed as well, and any other manifest keeps its name.
        let manifest_name = self
            .base
            .manifest_file(from)
            .and_then(|file| file.file_name().map(|name| name.to_os_string()))
            .map(|name| {
                if name == format!("{from}.json").as_str() {
                    format!("{to}.json").into()
                } else {
                    name
                }
            })
            .unwrap_or_else(|| format!("{to}.json").into());
        self.renamed_manifests.insert(
            to.to_string(),
            self.base.instance_root(to).join(manifest_name),
        );

        Ok(())
    }

    /// Applies the changes to the disk and publishes the new index.
    ///
    /// The manifests, the client files and the removals are applied in the
    /// order the draft recorded them; renames are applied first, because the
    /// later files are placed in the renamed directories. A failure restores
    /// every file the commit already replaced and leaves the published index
    /// untouched.
    pub fn commit(mut self) -> Result<RepositorySnapshot, DraftError> {
        self.check_open()?;
        self.state = DraftState::Committing;

        let committed = self.build_committed_snapshot()?;

        let mut applied_renames: Vec<(String, String)> = Vec::new();
        let mut removed_roots: Vec<RemovedRoot> = Vec::new();
        let mut applied_files: Vec<AppliedFile> = Vec::new();
        let mut created_roots: Vec<PathBuf> = Vec::new();
        let mut rollback_directory: Option<PathBuf> = None;

        let result = (|| -> Result<(), DraftError> {
            for (from, to) in &self.renames {
                self.apply_rename(from, to)?;
                applied_renames.push((from.clone(), to.clone()));
            }

            for id in &self.created {
                if !self.manifests.contains_key(id) {
                    continue;
                }
                let root = self.base.instance_root(id);
                std::fs::create_dir_all(&root).map_err(|error| write_error(&root, error))?;
                created_roots.push(root);
            }

            if !self.removed.is_empty()
                || !self.modified.is_empty()
                || !self.primary_jars.is_empty()
            {
                let directory = self.create_rollback_directory()?;
                for id in &self.removed {
                    self.remove_instance_root(id, &directory, &mut removed_roots)?;
                }
                for (id, source) in &self.primary_jars {
                    self.apply_primary_jar(id, source, &directory, &mut applied_files)?;
                }
                for id in &self.modified {
                    let manifest = self
                        .manifests
                        .get(id)
                        .ok_or_else(|| DraftError::UnknownInstance { id: id.to_string() })?;
                    self.apply_manifest(id, manifest, &directory, &mut applied_files)?;
                }
                rollback_directory = Some(directory);
            }

            Ok(())
        })();

        if let Err(error) = result {
            rollback_files(&mut applied_files);
            rollback_removed_roots(&mut removed_roots);
            rollback_renames(&self.base, &mut applied_renames);

            self.state = DraftState::Failed;
            for root in created_roots {
                let _ = std::fs::remove_dir_all(root);
            }
            if let Some(directory) = rollback_directory {
                let _ = std::fs::remove_dir_all(directory);
            }
            return Err(error);
        }

        let mut committed = committed;

        // The game file of a new or changed instance may be the file the
        // launcher just copied for it, so the version of the game is read again
        // for the instances the draft touched.
        for id in self.modified.iter().chain(self.created.iter()) {
            let Some(jar) = committed.jar_file(id) else {
                continue;
            };
            match super::game_version::minecraft_version(&jar) {
                Ok(Some(version)) => {
                    committed.versions.insert(
                        id.clone(),
                        crate::util::versioning::GameVersionNumber::as_game_version(&version),
                    );
                }
                _ => {
                    committed.versions.remove(id);
                }
            }
        }

        self.repository.snapshot = committed.clone();
        self.state = DraftState::Committed;

        if let Some(directory) = rollback_directory {
            // The commit already happened, so a scratch directory that cannot
            // be removed must not turn the commit into a failure.
            let _ = std::fs::remove_dir_all(directory);
        }

        Ok(committed)
    }

    /// Discards the changes of the draft.
    ///
    /// The directories the draft created are removed, including the files
    /// other work placed in them while the draft was open. Aborting a draft
    /// that was already aborted or failed does nothing; aborting a committed
    /// draft is an error, because its files are published.
    pub fn abort(&mut self) -> Result<(), DraftError> {
        match self.state {
            DraftState::Aborted | DraftState::Failed => return Ok(()),
            DraftState::Committed => return Err(DraftError::NotOpen { state: self.state }),
            DraftState::Committing => return Err(DraftError::NotOpen { state: self.state }),
            DraftState::Open => {}
        }

        let mut first_error: Option<DraftError> = None;
        for id in &self.created {
            let root = self.base.instance_root(id);
            if !root.exists() {
                continue;
            }
            if let Err(error) = std::fs::remove_dir_all(&root) {
                first_error.get_or_insert_with(|| write_error(&root, error));
            }
        }

        match first_error {
            Some(error) => {
                self.state = DraftState::Failed;
                Err(error)
            }
            None => {
                self.state = DraftState::Aborted;
                Ok(())
            }
        }
    }

    /// Updates one manifest in the write set of the draft.
    fn put_manifest(
        &mut self,
        manifest: InstanceManifest,
        claim_new_root: bool,
    ) -> Result<(), DraftError> {
        let id = manifest.id.clone().ok_or(DraftError::MissingId)?;
        if !super::repository::is_valid_instance_id(&id) {
            return Err(DraftError::InvalidId { id });
        }

        if claim_new_root
            && !self.manifests.contains_key(&id)
            && !self.base.contains(&id)
            && !self.created.contains(&id)
        {
            let root = self.base.instance_root(&id);
            if root.exists() {
                return Err(DraftError::AlreadyExists { path: root });
            }
            self.created.insert(id.clone());
        }

        self.manifests.insert(id.clone(), manifest);
        self.removed.remove(&id);
        self.modified.insert(id);
        Ok(())
    }

    /// Builds the index the draft publishes once its files are applied.
    fn build_committed_snapshot(&self) -> Result<RepositorySnapshot, DraftError> {
        let mut committed = self.base.clone();

        for (from, to) in &self.renames {
            committed.manifests.remove(from);
            committed.manifest_files.remove(from);
            let version = committed.versions.remove(from);
            if let Some(version) = version {
                // The game file moves with the instance, so the version the
                // launcher read from it still describes the renamed instance.
                committed.versions.insert(to.clone(), version);
            }

            // A renamed instance keeps the name of its manifest file: the file
            // moves with the directory, so the launcher still reads the
            // manifest the instance was read from.
            if let Some(file) = self.renamed_manifests.get(to) {
                committed.manifest_files.insert(to.clone(), file.clone());
            }
        }

        for id in &self.removed {
            committed.manifests.remove(id);
            committed.manifest_files.remove(id);
            committed.versions.remove(id);
        }

        for id in &self.modified {
            let manifest = self
                .manifests
                .get(id)
                .ok_or_else(|| DraftError::UnknownInstance { id: id.to_string() })?;
            committed.manifests.insert(id.clone(), manifest.clone());
        }

        remove_invalid_instances(&mut committed);
        Ok(committed)
    }

    /// Returns the published directory of an instance.
    fn instance_root_of(&self, id: &str) -> PathBuf {
        absolute(&self.base.instance_root(id))
    }

    /// Returns the file the manifest of an instance is written to.
    ///
    /// An instance the repository already holds keeps the file it was read
    /// from; a renamed instance keeps the file whose name moved with its
    /// directory; a new instance uses the conventional file of the layout.
    fn manifest_target(&self, id: &str) -> Result<PathBuf, DraftError> {
        let target = if let Some(file) = self.renamed_manifests.get(id) {
            absolute(file)
        } else if let Some(file) = self.base.manifest_file(id) {
            absolute(file)
        } else {
            self.instance_root_of(id).join(format!("{id}.json"))
        };
        self.validate_target(id, &target)?;
        Ok(target)
    }

    /// Returns the file of the game of an instance.
    ///
    /// An instance the repository already holds keeps the file it uses; a new
    /// or renamed instance uses the conventional file of the layout.
    fn primary_jar_target(&self, id: &str) -> Result<PathBuf, DraftError> {
        let target = match self.base.jar_file(id) {
            Some(file) if self.base.contains(id) => absolute(&file),
            _ => self.instance_root_of(id).join(format!("{id}.jar")),
        };
        self.validate_target(id, &target)?;
        Ok(target)
    }

    /// Verifies that a file is written inside the directory of its instance.
    fn validate_target(&self, id: &str, target: &Path) -> Result<(), DraftError> {
        let root = self.instance_root_of(id);
        if target == root || !target.starts_with(&root) {
            return Err(DraftError::EscapesInstanceRoot {
                path: target.to_path_buf(),
            });
        }
        Ok(())
    }

    /// Creates the scratch directory of a commit attempt.
    fn create_rollback_directory(&self) -> Result<PathBuf, DraftError> {
        let parent = self
            .base
            .base_directory()
            .join(DRAFT_DIRECTORY[0])
            .join(DRAFT_DIRECTORY[1]);
        std::fs::create_dir_all(&parent).map_err(|error| write_error(&parent, error))?;
        create_unique_directory(&parent, "commit-")
    }

    /// Moves the directory of an instance to its new identifier.
    fn apply_rename(&self, from: &str, to: &str) -> Result<(), DraftError> {
        let source = self.base.instance_root(from);
        let target = self.base.instance_root(to);
        if !source.is_dir() {
            return Err(DraftError::Write {
                path: source,
                error: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "the directory of the instance does not exist",
                ),
            });
        }
        if target.exists() {
            return Err(DraftError::AlreadyExists { path: target });
        }

        move_instance_files(self.base.base_directory(), from, to)
    }

    /// Moves the directory of a removed instance into the scratch directory.
    fn remove_instance_root(
        &self,
        id: &str,
        rollback_directory: &Path,
        removed: &mut Vec<RemovedRoot>,
    ) -> Result<(), DraftError> {
        let root = self.base.instance_root(id);
        if !root.exists() {
            return Ok(());
        }

        let parent = rollback_directory.join("removed");
        std::fs::create_dir_all(&parent).map_err(|error| write_error(&parent, error))?;
        let rollback = create_unique_directory(&parent, "instance-")?;
        // `create_unique_directory` created the directory; moving the root onto
        // it requires the path to be free again.
        std::fs::remove_dir(&rollback).map_err(|error| write_error(&rollback, error))?;
        move_replacing(&root, &rollback)?;
        removed.push(RemovedRoot {
            original: root,
            rollback,
        });
        Ok(())
    }

    /// Writes one manifest while keeping a copy of the replaced file.
    fn apply_manifest(
        &self,
        id: &str,
        manifest: &InstanceManifest,
        rollback_directory: &Path,
        applied: &mut Vec<AppliedFile>,
    ) -> Result<(), DraftError> {
        let target = self.manifest_target(id)?;
        let contents =
            serde_json::to_string_pretty(manifest).map_err(|error| DraftError::Write {
                path: target.clone(),
                error: std::io::Error::new(std::io::ErrorKind::InvalidData, error),
            })?;

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| write_error(parent, error))?;
        }
        let backup = backup_file(&target, rollback_directory, "manifest-", ".json")?;
        applied.push(AppliedFile {
            target: target.clone(),
            backup,
        });
        std::fs::write(&target, contents).map_err(|error| write_error(&target, error))
    }

    /// Copies a completed client file while keeping a copy of the replaced
    /// file.
    fn apply_primary_jar(
        &self,
        id: &str,
        source: &Path,
        rollback_directory: &Path,
        applied: &mut Vec<AppliedFile>,
    ) -> Result<(), DraftError> {
        if !source.is_file() {
            return Err(DraftError::NotRegularFile {
                path: source.to_path_buf(),
            });
        }

        let target = self.primary_jar_target(id)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| write_error(parent, error))?;
        }
        let backup = backup_file(&target, rollback_directory, "jar-", ".jar")?;
        applied.push(AppliedFile {
            target: target.clone(),
            backup,
        });
        std::fs::copy(source, &target).map_err(|error| write_error(&target, error))?;
        Ok(())
    }

    /// Ensures the draft accepts changes.
    fn check_open(&self) -> Result<(), DraftError> {
        if self.state == DraftState::Open {
            Ok(())
        } else {
            Err(DraftError::NotOpen { state: self.state })
        }
    }
}

impl Drop for GameRepositoryDraft<'_> {
    /// Discards the changes of a draft that was neither committed nor aborted,
    /// so a forgotten draft does not leave the directories it reserved behind.
    fn drop(&mut self) {
        if self.state == DraftState::Open {
            let _ = self.abort();
        }
    }
}

/// A file replacement of a commit attempt, with the file it replaced.
struct AppliedFile {
    /// The permanent file path.
    target: PathBuf,
    /// The copy of the file that was replaced, if it existed.
    backup: Option<PathBuf>,
}

/// An instance directory moved aside by a commit attempt.
struct RemovedRoot {
    /// The published directory of the instance.
    original: PathBuf,
    /// The scratch directory that holds it.
    rollback: PathBuf,
}

/// Restores the files a failed commit replaced.
fn rollback_files(applied: &mut Vec<AppliedFile>) {
    while let Some(file) = applied.pop() {
        let _ = std::fs::remove_file(&file.target);
        if let Some(backup) = file.backup {
            let _ = move_replacing(&backup, &file.target);
        }
    }
}

/// Moves the removed instance directories back to their published places.
fn rollback_removed_roots(removed: &mut Vec<RemovedRoot>) {
    while let Some(root) = removed.pop() {
        let _ = move_replacing(&root.rollback, &root.original);
    }
}

/// Reverts the renames a failed commit applied.
fn rollback_renames(base: &RepositorySnapshot, renames: &mut Vec<(String, String)>) {
    while let Some((from, to)) = renames.pop() {
        let _ = move_instance_files(base.base_directory(), &to, &from);
    }
}

/// Moves the files of an instance to another identifier.
///
/// The directory is moved first, and the manifest and the game file named
/// after the old identifier are renamed inside it. A file that is not named
/// after the instance keeps its name, because the launcher reads the manifest
/// from the file it recorded and the game from the file with the same name.
pub(crate) fn move_instance_files(
    base_directory: &Path,
    from: &str,
    to: &str,
) -> Result<(), DraftError> {
    let versions = base_directory.join("versions");
    let from_directory = versions.join(from);
    let to_directory = versions.join(to);

    std::fs::rename(&from_directory, &to_directory)
        .map_err(|error| write_error(&to_directory, error))?;

    let from_json = to_directory.join(format!("{from}.json"));
    let to_json = to_directory.join(format!("{to}.json"));
    let from_jar = to_directory.join(format!("{from}.jar"));
    let to_jar = to_directory.join(format!("{to}.jar"));
    let moved_jar = from_jar.is_file();

    let move_files = || -> Result<(), DraftError> {
        if from_json.is_file() {
            std::fs::rename(&from_json, &to_json).map_err(|error| write_error(&to_json, error))?;
        }
        if moved_jar {
            std::fs::rename(&from_jar, &to_jar).map_err(|error| write_error(&to_jar, error))?;
        }
        Ok(())
    };

    if let Err(error) = move_files() {
        // Put the files and the directory back, so the failed rename leaves
        // the published instance as it was.
        if to_json.is_file() && !from_json.exists() {
            let _ = std::fs::rename(&to_json, &from_json);
        }
        if moved_jar && to_jar.is_file() && !from_jar.exists() {
            let _ = std::fs::rename(&to_jar, &from_jar);
        }
        let _ = std::fs::rename(&to_directory, &from_directory);
        return Err(error);
    }

    Ok(())
}

/// Returns a path as an absolute, normalized path.
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Builds the error of a file that cannot be read, written or moved.
fn write_error(path: &Path, error: std::io::Error) -> DraftError {
    DraftError::Write {
        path: path.to_path_buf(),
        error,
    }
}

/// Creates a directory whose name is unique inside its parent.
fn create_unique_directory(parent: &Path, prefix: &str) -> Result<PathBuf, DraftError> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    for _ in 0..256 {
        let number = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!("{prefix}{}-{number}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(write_error(&path, error)),
        }
    }

    Err(DraftError::AlreadyExists {
        path: parent.to_path_buf(),
    })
}

/// Moves a file or directory, replacing the target when it exists.
fn move_replacing(source: &Path, target: &Path) -> Result<(), DraftError> {
    match std::fs::rename(source, target) {
        Ok(()) => Ok(()),
        // The standard library replaces an existing file on every platform the
        // launcher runs on, but not an existing directory, so a directory is
        // removed first.
        Err(_) if target.is_dir() => {
            std::fs::remove_dir_all(target).map_err(|error| write_error(target, error))?;
            std::fs::rename(source, target).map_err(|error| write_error(target, error))
        }
        Err(error) => Err(write_error(target, error)),
    }
}

/// Moves the file an instance is about to replace into the scratch directory.
///
/// The backup keeps the extension of the replaced file, so a leftover backup
/// is recognizable when the launcher fails to remove the scratch directory.
fn backup_file(
    target: &Path,
    rollback_directory: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<Option<PathBuf>, DraftError> {
    if !target.exists() {
        return Ok(None);
    }

    let parent = rollback_directory.join("backups");
    std::fs::create_dir_all(&parent).map_err(|error| write_error(&parent, error))?;

    // Reserve the name with a directory, then swap it for the file: the
    // counter keeps two backups of one commit from colliding.
    let reserved = create_unique_directory(&parent, prefix)?;
    std::fs::remove_dir(&reserved).map_err(|error| write_error(&reserved, error))?;
    let mut name = reserved.into_os_string();
    name.push(suffix);
    let backup = PathBuf::from(name);

    move_replacing(target, &backup)?;
    Ok(Some(backup))
}

/// Removes the invalid instances of an index.
///
/// An instance whose chain of inherited versions names a version the index
/// does not hold cannot be started, so it is left out. A chain that loops is
/// kept, because the resolver stops at the second visit of a version.
pub(crate) fn remove_invalid_instances(snapshot: &mut RepositorySnapshot) {
    let invalid: Vec<String> = snapshot
        .manifests
        .iter()
        .filter(|(_, manifest)| manifest.inherits_from.is_some())
        .filter(|(id, manifest)| !has_complete_chain(snapshot, id, manifest))
        .map(|(id, _)| id.clone())
        .collect();

    for id in invalid {
        let mut current = Some(id);
        while let Some(id) = current {
            let parent = snapshot
                .manifests
                .get(&id)
                .and_then(|manifest| manifest.inherits_from.clone());
            snapshot.manifests.remove(&id);
            snapshot.manifest_files.remove(&id);
            snapshot.versions.remove(&id);
            current = parent.filter(|parent| snapshot.manifests.contains_key(parent));
        }
    }
}

/// Returns whether every version an instance inherits from is in the index.
fn has_complete_chain(
    snapshot: &RepositorySnapshot,
    id: &str,
    manifest: &InstanceManifest,
) -> bool {
    let mut visited = HashSet::new();
    visited.insert(id.to_string());

    let mut current = manifest;
    while let Some(parent_id) = &current.inherits_from {
        if !visited.insert(parent_id.clone()) {
            return true;
        }
        match snapshot.manifests.get(parent_id) {
            Some(parent) => current = parent,
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a game directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-draft-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(directory.join("versions")).expect("the directory should be made");
        directory
    }

    /// Writes the manifest of an instance the repository reads.
    fn manifest(base: &Path, id: &str, contents: &str) {
        let directory = base.join("versions").join(id);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        std::fs::write(directory.join(format!("{id}.json")), contents)
            .expect("the manifest should be written");
    }

    /// Returns the manifest of an instance with the main class.
    fn with_main_class(id: &str, main_class: &str) -> InstanceManifest {
        InstanceManifest {
            id: Some(id.to_string()),
            main_class: Some(main_class.to_string()),
            ..InstanceManifest::default()
        }
    }

    #[test]
    fn a_new_manifest_is_published_by_the_commit() {
        let base = directory("commit");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let mut draft = repository.open_draft();
        draft
            .put(with_main_class("instance", "example.Main"))
            .expect("the manifest should be accepted");

        // Nothing is visible before the commit.
        assert!(!draft.base_snapshot().contains("instance"));
        assert!(!base.join("versions/instance").exists());
        assert!(!base.join(".hmcl/repository-drafts").exists());

        let committed = draft.commit().expect("the commit should succeed");
        assert!(committed.contains("instance"));
        assert_eq!(
            committed
                .manifest("instance")
                .and_then(|manifest| manifest.main_class.as_deref()),
            Some("example.Main")
        );
        assert!(repository.contains("instance"));

        let stored = std::fs::read_to_string(base.join("versions/instance/instance.json"))
            .expect("the manifest should be written");
        assert!(stored.contains("example.Main"), "{stored}");
        assert!(!base
            .join(".hmcl/repository-drafts")
            .join("commit-")
            .exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_client_file_is_copied_while_committing() {
        let base = directory("primary-jar");
        let cache = base.join("cache");
        std::fs::create_dir_all(&cache).expect("the cache should be made");
        let source = cache.join("client.jar");
        std::fs::write(&source, b"client").expect("the client should be written");

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let mut draft = repository.open_draft();
        draft
            .put(with_main_class("instance", "example.Main"))
            .expect("the manifest should be accepted");
        draft
            .put_primary_jar("instance", &source)
            .expect("the client should be accepted");
        assert!(!base.join("versions/instance").exists());

        draft.commit().expect("the commit should succeed");

        assert_eq!(
            std::fs::read(base.join("versions/instance/instance.jar"))
                .expect("the client should be read"),
            b"client"
        );
        assert_eq!(
            std::fs::read(&source).expect("the source should be read"),
            b"client"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_abandoned_draft_removes_its_new_directories() {
        let base = directory("abort");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        {
            let mut draft = repository.open_draft();
            draft
                .put(with_main_class("instance", "example.Main"))
                .expect("the manifest should be accepted");
            let root = base.join("versions/instance");
            std::fs::create_dir_all(&root).expect("the directory should be made");
            std::fs::write(root.join("downloaded.jar"), b"content")
                .expect("the file should be written");
        }

        assert!(!base.join("versions/instance").exists());
        assert!(!repository.contains("instance"));

        let draft = repository.open_draft();
        assert!(draft.is_open());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn aborting_leaves_a_published_manifest_unchanged() {
        let base = directory("abort-existing");
        manifest(
            &base,
            "instance",
            r#"{"id": "instance", "mainClass": "original.Main"}"#,
        );
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        {
            let mut draft = repository.open_draft();
            draft
                .put(with_main_class("instance", "updated.Main"))
                .expect("the manifest should be accepted");
            draft.abort().expect("the draft should abort");
        }

        assert_eq!(
            repository
                .manifest("instance")
                .and_then(|manifest| manifest.main_class.as_deref()),
            Some("original.Main")
        );
        let stored = std::fs::read_to_string(base.join("versions/instance/instance.json"))
            .expect("the manifest should be read");
        assert!(stored.contains("original.Main"), "{stored}");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_directory_that_predates_the_draft_is_not_claimed() {
        let base = directory("preexisting");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let root = base.join("versions/instance");
        std::fs::create_dir_all(&root).expect("the directory should be made");
        let retained = root.join("retained.txt");
        std::fs::write(&retained, b"content").expect("the file should be written");

        let mut draft = repository.open_draft();
        let error = draft
            .put(with_main_class("instance", "example.Main"))
            .expect_err("the draft should refuse the directory");
        assert!(matches!(error, DraftError::AlreadyExists { .. }));
        drop(draft);

        assert!(retained.exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_aborted_removal_keeps_the_instance() {
        let base = directory("abort-remove");
        manifest(&base, "instance", r#"{"id": "instance"}"#);
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        {
            let mut draft = repository.open_draft();
            draft
                .remove("instance")
                .expect("the instance should be removed");
        }

        assert!(repository.contains("instance"));
        assert!(base.join("versions/instance").is_dir());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_rename_moves_the_files_and_the_inheriting_instances() {
        let base = directory("rename");
        manifest(&base, "parent", r#"{"id": "parent"}"#);
        manifest(
            &base,
            "child",
            r#"{"id": "child", "inheritsFrom": "parent"}"#,
        );
        std::fs::write(base.join("versions/parent/parent.jar"), b"client")
            .expect("the client should be written");

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let mut draft = repository.open_draft();
        draft
            .rename("parent", "renamed")
            .expect("the instance should be renamed");
        draft.commit().expect("the commit should succeed");

        assert!(!repository.contains("parent"));
        assert!(repository.contains("renamed"));
        assert_eq!(
            repository
                .manifest("child")
                .and_then(|manifest| manifest.inherits_from.as_deref()),
            Some("renamed")
        );
        assert!(!base.join("versions/parent").exists());
        assert!(base.join("versions/renamed/renamed.json").is_file());
        assert!(base.join("versions/renamed/renamed.jar").is_file());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_rename_can_be_applied_through_the_repository() {
        let base = directory("rename-repository");
        manifest(&base, "old", r#"{"id": "old"}"#);
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        repository
            .rename("old", "new")
            .expect("the instance should be renamed");

        assert!(repository.contains("new"));
        assert_eq!(
            repository
                .manifest("new")
                .and_then(|manifest| manifest.id.as_deref()),
            Some("new")
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_failed_commit_restores_the_replaced_manifest() {
        let base = directory("rollback");
        manifest(
            &base,
            "instance",
            r#"{"id": "instance", "mainClass": "original.Main"}"#,
        );
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let mut draft = repository.open_draft();
        draft
            .put(with_main_class("instance", "updated.Main"))
            .expect("the manifest should be accepted");

        // A client file that disappears before the commit makes the commit
        // fail after the manifest was already replaced.
        let missing = base.join("missing.jar");
        draft
            .primary_jars
            .insert("instance".to_string(), missing.clone());

        let error = draft.commit().expect_err("the commit should fail");
        assert!(matches!(error, DraftError::NotRegularFile { .. }));

        assert_eq!(
            repository
                .manifest("instance")
                .and_then(|manifest| manifest.main_class.as_deref()),
            Some("original.Main")
        );
        let stored = std::fs::read_to_string(base.join("versions/instance/instance.json"))
            .expect("the manifest should be read");
        assert!(stored.contains("original.Main"), "{stored}");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_manifest_change_keeps_the_client_file() {
        let base = directory("keep-jar");
        manifest(&base, "instance", r#"{"id": "instance"}"#);
        std::fs::write(
            base.join("versions/instance/instance.jar"),
            b"custom client",
        )
        .expect("the client should be written");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        repository
            .save(with_main_class("instance", "updated.Main"))
            .expect("the manifest should be saved");

        assert_eq!(
            std::fs::read(base.join("versions/instance/instance.jar"))
                .expect("the client should be read"),
            b"custom client"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_instance_can_be_removed_from_the_disk() {
        let base = directory("remove-disk");
        manifest(&base, "instance", r#"{"id": "instance"}"#);
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        assert!(repository.remove_instance_from_disk("instance"));

        assert!(!repository.contains("instance"));
        assert!(!base.join("versions/instance").exists());
        assert!(!base.join("versions/instance_removed").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_unknown_directory_is_removed_without_a_draft() {
        let base = directory("remove-orphan");
        let root = base.join("versions/orphan");
        std::fs::create_dir_all(&root).expect("the directory should be made");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        assert!(repository.remove_instance_from_disk("orphan"));
        assert!(!root.exists());
        // Removing a directory that is gone succeeds as well.
        assert!(repository.remove_instance_from_disk("orphan"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_manifest_without_an_id_is_refused() {
        let base = directory("missing-id");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let mut draft = repository.open_draft();
        assert!(matches!(
            draft.put(InstanceManifest::default()),
            Err(DraftError::MissingId)
        ));
        // An identifier that names more than one path segment cannot name an
        // instance directory.
        assert!(matches!(
            draft.put(with_main_class("a/b", "example.Main")),
            Err(DraftError::InvalidId { .. })
        ));
        assert!(draft.rename("missing", "../escape").is_err());
        assert!(draft.commit().is_ok());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_rename_keeps_the_name_of_a_manifest_that_is_not_conventional() {
        let base = directory("rename-custom-manifest");
        std::fs::create_dir_all(base.join("versions/old")).expect("the directory should be made");
        std::fs::write(base.join("versions/old/custom.json"), r#"{"id": "old"}"#)
            .expect("the manifest should be written");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let mut draft = repository.open_draft();
        draft
            .rename("old", "new")
            .expect("the instance should be renamed");
        draft.commit().expect("the commit should succeed");

        // The manifest file keeps its name, so the launcher reads it again.
        assert!(base.join("versions/new/custom.json").is_file());
        assert_eq!(
            repository.manifest_file("new"),
            Some(base.join("versions/new/custom.json").as_path())
        );
        assert_eq!(
            repository.jar_file("new"),
            Some(base.join("versions/new/custom.jar"))
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_renamed_instance_keeps_the_version_of_its_game() {
        let base = directory("rename-version");
        manifest(&base, "old", r#"{"id": "old"}"#);
        std::fs::write(base.join("versions/old/old.jar"), client_jar("1.20.1"))
            .expect("the client should be written");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(repository.version("old").is_some());

        repository
            .rename("old", "new")
            .expect("the rename should work");

        assert!(repository.version("new").is_some());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Builds a client file that names the version of the game in its classes.
    ///
    /// The archive holds a class whose constants name the version, which is
    /// the way the launcher reads the version of an old file of the game.
    fn client_jar(version: &str) -> Vec<u8> {
        let text = format!("Minecraft Minecraft {version}");
        let mut class = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52];
        class.extend_from_slice(&3u16.to_be_bytes());
        class.push(1);
        class.extend_from_slice(&(text.len() as u16).to_be_bytes());
        class.extend_from_slice(text.as_bytes());
        class.push(8);
        class.extend_from_slice(&1u16.to_be_bytes());

        let path = std::env::temp_dir().join(format!(
            "hmcl-rs-draft-client-{}-{}.jar",
            std::process::id(),
            version
        ));
        let mut writer =
            crate::util::zip::ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("net/minecraft/client/Minecraft.class", &class)
            .expect("the class should be added");
        writer.finish().expect("the archive should be finished");

        let contents = std::fs::read(&path).expect("the archive should be read");
        let _ = std::fs::remove_file(&path);
        contents
    }

    #[test]
    fn a_draft_that_was_not_committed_refuses_changes() {
        let base = directory("closed");
        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let mut draft = repository.open_draft();
        draft
            .put(with_main_class("instance", "example.Main"))
            .expect("the manifest should be accepted");
        draft.commit().expect("the commit should succeed");

        let mut draft = repository.open_draft();
        draft
            .put(with_main_class("other", "example.Main"))
            .expect("the manifest should be accepted");
        draft.abort().expect("the draft should abort");
        assert_eq!(draft.state(), DraftState::Aborted);
        assert!(matches!(
            draft.put(with_main_class("another", "example.Main")),
            Err(DraftError::NotOpen { .. })
        ));
        assert!(matches!(
            draft.remove("instance"),
            Err(DraftError::NotOpen { .. })
        ));
        assert!(matches!(
            draft.put_primary_jar("instance", &base),
            Err(DraftError::NotOpen { .. })
        ));

        let _ = std::fs::remove_dir_all(&base);
    }
}
