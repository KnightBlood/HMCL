//! Reading the instances of a game directory.
//!
//! A game directory keeps its instances in `versions`: every instance has a
//! directory named after the instance and a file that tells how the game is
//! started. The launcher reads those files, so it can show the instances the
//! player has and start them.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::download::game::InstanceManifest;
use crate::util::versioning::GameVersionNumber;

use super::game_version::minecraft_version;
use super::repository_draft::{remove_invalid_instances, DraftError};
use super::repository_snapshot::RepositorySnapshot;
use super::resolver::{InstanceResolver, ResolveError};

/// The error of reading or writing the instances of a game directory.
///
/// The launcher keeps reading the other instances when one of them cannot be
/// read, so the errors tell the player which instances are broken.
#[derive(Debug)]
pub enum RepositoryError {
    /// The directory of the instances cannot be read.
    ReadDirectory {
        /// The directory that cannot be read.
        directory: PathBuf,
        /// The reason the directory cannot be read.
        error: std::io::Error,
    },
    /// The file of an instance cannot be read.
    ReadManifest {
        /// The file that cannot be read.
        file: PathBuf,
        /// The reason the file cannot be read.
        error: std::io::Error,
    },
    /// The file of an instance does not hold a manifest.
    ParseManifest {
        /// The file that does not hold a manifest.
        file: PathBuf,
        /// The reason the file does not hold a manifest.
        error: serde_json::Error,
    },
    /// An instance cannot be found.
    InstanceNotFound {
        /// The identifier of the missing instance.
        id: String,
    },
    /// An instance already exists with the identifier.
    InstanceExists {
        /// The identifier that is already taken.
        id: String,
    },
    /// A file or directory of an instance cannot be written.
    WriteInstance {
        /// The file or directory that cannot be written.
        path: PathBuf,
        /// The reason it cannot be written.
        error: std::io::Error,
    },
    /// A draft could not apply the changes of the instances.
    Commit {
        /// The reason the commit failed.
        error: Box<DraftError>,
    },
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDirectory { directory, error } => {
                write!(
                    formatter,
                    "The directory {} cannot be read: {error}",
                    directory.display()
                )
            }
            Self::ReadManifest { file, error } => {
                write!(
                    formatter,
                    "The manifest {} cannot be read: {error}",
                    file.display()
                )
            }
            Self::ParseManifest { file, error } => {
                write!(
                    formatter,
                    "The manifest {} is malformed: {error}",
                    file.display()
                )
            }
            Self::InstanceNotFound { id } => {
                write!(formatter, "The instance {id} does not exist")
            }
            Self::InstanceExists { id } => {
                write!(formatter, "The instance {id} already exists")
            }
            Self::WriteInstance { path, error } => {
                write!(
                    formatter,
                    "The file {} cannot be written: {error}",
                    path.display()
                )
            }
            Self::Commit { error } => write!(formatter, "The instances cannot be saved: {error}"),
        }
    }
}

impl std::error::Error for RepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadDirectory { error, .. } | Self::ReadManifest { error, .. } => Some(error),
            Self::ParseManifest { error, .. } => Some(error),
            Self::WriteInstance { error, .. } => Some(error),
            Self::Commit { error } => Some(error),
            _ => None,
        }
    }
}

/// Returns whether an identifier can name an instance of a game directory.
///
/// The identifier is the name of the directory of the instance, so it has to be
/// one path segment: an empty name, the names of the current and the parent
/// directory, and the names that hold a separator are refused.
pub fn is_valid_instance_id(id: &str) -> bool {
    !id.trim().is_empty() && id != "." && id != ".." && !id.contains('/') && !id.contains('\\')
}

/// The extensions of the image files the launcher shows as an instance icon.
///
/// The order is the one the launcher reads the files in, so an instance with
/// more than one icon shows the first of them.
pub const IMAGE_EXTENSIONS: [&str; 8] = ["png", "jpg", "jpeg", "bmp", "gif", "webp", "svg", "apng"];

/// The name of the file that tells the game of an instance stopped in a way the
/// launcher did not see.
pub const ABNORMAL_MARKER: &str = ".abnormal";

/// The instances of a game directory.
///
/// The repository reads the manifests of the instances and the version of the
/// game of each instance. The manifests are kept as they are written on the
/// disk; [`GameRepository::resolve`] applies the versions an instance inherits
/// from, so the launcher starts the game from one manifest.
///
/// The repository publishes its index as an immutable
/// [`RepositorySnapshot`]: a reader that needs a stable view keeps the snapshot
/// instead of asking the repository again. Changes are staged in a
/// [`GameRepositoryDraft`] and published once, so the launcher never reads a
/// half written instance.
#[derive(Debug, Clone, Default)]
pub struct GameRepository {
    /// The published index of the instances.
    pub(crate) snapshot: RepositorySnapshot,
}

impl GameRepository {
    /// Reads the instances of a game directory.
    ///
    /// A directory of the instances without a manifest is skipped, and a
    /// directory with more than one manifest is skipped as well, because the
    /// launcher cannot tell which manifest is the one of the instance.
    pub fn read(base: impl Into<PathBuf>) -> (Self, Vec<RepositoryError>) {
        let mut repository = Self {
            snapshot: RepositorySnapshot::new(base),
        };
        let errors = repository.refresh();

        (repository, errors)
    }

    /// Reads the instances of the game directory again.
    ///
    /// The instances the directory no longer holds are forgotten; the instances
    /// that cannot be read are left out and returned as errors. The new index
    /// replaces the published one as a whole, so readers that kept the old
    /// snapshot still see the instances they were given.
    pub fn refresh(&mut self) -> Vec<RepositoryError> {
        let mut errors = Vec::new();
        let (manifests, manifest_files, versions) =
            read_instances(self.snapshot.base_directory(), &mut errors);

        self.snapshot.manifests = manifests;
        self.snapshot.manifest_files = manifest_files;
        self.snapshot.versions = versions;
        remove_invalid_instances(&mut self.snapshot);
        errors
    }

    /// Returns the published index of the instances.
    pub fn snapshot(&self) -> &RepositorySnapshot {
        &self.snapshot
    }

    /// Returns the directory the game directory keeps its files in.
    pub fn base_directory(&self) -> &Path {
        self.snapshot.base_directory()
    }

    /// Returns the identifiers of the instances.
    pub fn instance_ids(&self) -> impl Iterator<Item = &str> {
        self.snapshot.instance_ids()
    }

    /// Returns how many instances the repository holds.
    pub fn len(&self) -> usize {
        self.snapshot.len()
    }

    /// Returns whether the repository holds no instance.
    pub fn is_empty(&self) -> bool {
        self.snapshot.is_empty()
    }

    /// Returns whether the repository holds an instance.
    pub fn contains(&self, id: &str) -> bool {
        self.snapshot.contains(id)
    }

    /// Returns the manifest of an instance, as it is written on the disk.
    pub fn manifest(&self, id: &str) -> Option<&InstanceManifest> {
        self.snapshot.manifest(id)
    }

    /// Returns the file the manifest of an instance is read from.
    pub fn manifest_file(&self, id: &str) -> Option<&Path> {
        self.snapshot.manifest_file(id)
    }

    /// Returns the file of the game of an instance.
    ///
    /// The file is the file of the manifest with the extension of the game, so
    /// an instance whose manifest is not named after it still finds its game.
    pub fn jar_file(&self, id: &str) -> Option<PathBuf> {
        self.snapshot.jar_file(id)
    }

    /// Returns the version of the game of an instance, if the launcher read it.
    pub fn version(&self, id: &str) -> Option<&GameVersionNumber> {
        self.snapshot.version(id)
    }

    /// Returns the root directory of an instance.
    ///
    /// The directory is the one the instance keeps its files in, whether or not
    /// the instance can be read; an instance the repository does not know has
    /// the directory its identifier names.
    pub fn instance_root(&self, id: &str) -> PathBuf {
        self.snapshot.instance_root(id)
    }

    /// Returns the file of the icon of an instance, if it has one.
    ///
    /// The launcher looks for `icon.png`, `icon.jpg` and the other files the
    /// game and the launchers write, in the order of [`IMAGE_EXTENSIONS`].
    pub fn icon_file(&self, id: &str) -> Option<PathBuf> {
        let root = self.instance_root(id);
        IMAGE_EXTENSIONS
            .iter()
            .map(|extension| root.join(format!("icon.{extension}")))
            .find(|file| file.is_file())
    }

    /// Replaces the icon of an instance with an image file.
    ///
    /// The images the launcher already has for the instance are removed, so the
    /// game and the launcher show the new image. The extension has to be one of
    /// [`IMAGE_EXTENSIONS`].
    pub fn set_icon_file(&self, id: &str, image: &Path) -> Result<PathBuf, RepositoryError> {
        if !self.contains(id) {
            return Err(RepositoryError::InstanceNotFound { id: id.to_string() });
        }

        let extension = image
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if !IMAGE_EXTENSIONS.contains(&extension.as_str()) {
            return Err(RepositoryError::WriteInstance {
                path: image.to_path_buf(),
                error: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("the image file {extension} is not an image the launcher shows"),
                ),
            });
        }

        let root = self.instance_root(id);
        self.clear_icon_files(&root);

        let target = root.join(format!("icon.{extension}"));
        std::fs::copy(image, &target).map_err(|error| RepositoryError::WriteInstance {
            path: target.clone(),
            error,
        })?;

        Ok(target)
    }

    /// Removes the icons of an instance, so the launcher shows the picture it
    /// chooses from the components of the instance.
    ///
    /// An icon that does not exist is not an error.
    fn clear_icon_files(&self, root: &Path) {
        for extension in IMAGE_EXTENSIONS {
            let _ = std::fs::remove_file(root.join(format!("icon.{extension}")));
        }
    }

    /// Removes the icon of an instance, so the launcher shows the picture it
    /// chooses from the components of the instance.
    pub fn delete_icon_file(&self, id: &str) -> Result<(), RepositoryError> {
        if !self.contains(id) {
            return Err(RepositoryError::InstanceNotFound { id: id.to_string() });
        }
        self.clear_icon_files(&self.instance_root(id));
        Ok(())
    }

    /// Marks an instance as one whose game stopped abnormally.
    ///
    /// The marker is written before the game is started and removed when the
    /// game stops in a way the launcher knows, so a game the player stopped by
    /// closing the window or that crashed leaves the marker. The launcher reads
    /// the marker to tell whether the files of the instance need the check of
    /// their digests. A marker that cannot be written is ignored, because the
    /// worst the launcher does is check the files once more.
    pub fn mark_launched_abnormally(&self, id: &str) {
        let _ = std::fs::File::create(self.instance_root(id).join(ABNORMAL_MARKER));
    }

    /// Removes the marker that tells the game of an instance stopped
    /// abnormally.
    ///
    /// Returns whether the marker was there, which is the answer of the
    /// launcher to the question whether the previous start of the game ended in
    /// a way the launcher did not see.
    pub fn unmark_launched_abnormally(&self, id: &str) -> bool {
        let marker = self.instance_root(id).join(ABNORMAL_MARKER);
        if !marker.is_file() {
            return false;
        }

        let _ = std::fs::remove_file(&marker);
        true
    }

    /// Renames an instance.
    ///
    /// The instance is renamed in a draft, so the files move and the new index
    /// is published together: the directory, the manifest named after the
    /// instance, its game file and the instances that inherit from it. The new
    /// identifier has to name an instance that does not exist yet.
    pub fn rename(&mut self, id: &str, new_id: &str) -> Result<(), RepositoryError> {
        if !is_valid_instance_id(new_id) {
            return Err(RepositoryError::InstanceExists {
                id: new_id.to_string(),
            });
        }
        if !self.contains(id) {
            return Err(RepositoryError::InstanceNotFound { id: id.to_string() });
        }
        if id == new_id {
            return Ok(());
        }
        if self.contains(new_id) {
            return Err(RepositoryError::InstanceExists {
                id: new_id.to_string(),
            });
        }

        let mut draft = self.open_draft();
        draft
            .rename(id, new_id)
            .map_err(|error| commit_error(id, error))?;
        draft.commit().map_err(|error| commit_error(id, error))?;
        Ok(())
    }

    /// Deletes an instance.
    ///
    /// The directory of the instance is removed from the disk and the index is
    /// published without it. An instance the index does not hold is not an
    /// error only when its directory is gone as well.
    pub fn delete(&mut self, id: &str) -> Result<(), RepositoryError> {
        if !self.contains(id) {
            return Err(RepositoryError::InstanceNotFound { id: id.to_string() });
        }

        if self.remove_instance_from_disk(id) {
            Ok(())
        } else {
            Err(RepositoryError::WriteInstance {
                path: self.instance_root(id),
                error: std::io::Error::other(format!("the instance {id} cannot be removed")),
            })
        }
    }

    /// Copies an instance.
    ///
    /// The directory of the instance is copied, its manifest is rewritten with
    /// the new identifier, and the repository is refreshed. The new identifier
    /// has to name an instance that does not exist yet.
    ///
    /// The copy is not a snapshot transaction: the directory appears on the
    /// disk before the new index is published.
    pub fn copy(&mut self, id: &str, new_id: &str) -> Result<(), RepositoryError> {
        if !is_valid_instance_id(new_id) {
            return Err(RepositoryError::InstanceExists {
                id: new_id.to_string(),
            });
        }
        if !self.contains(id) {
            return Err(RepositoryError::InstanceNotFound { id: id.to_string() });
        }
        if self.contains(new_id) {
            return Err(RepositoryError::InstanceExists {
                id: new_id.to_string(),
            });
        }

        let instances = self.base_directory().join("versions");
        let old_directory = instances.join(id);
        let new_directory = instances.join(new_id);

        copy_directory(&old_directory, &new_directory).map_err(|error| {
            RepositoryError::WriteInstance {
                path: new_directory.clone(),
                error,
            }
        })?;

        // Rename the manifest file when it is named after the old instance.
        let old_manifest_name = format!("{id}.json");
        let new_manifest_name = format!("{new_id}.json");
        let old_manifest_file = new_directory.join(&old_manifest_name);
        if old_manifest_file.is_file() {
            let new_manifest_file = new_directory.join(&new_manifest_name);
            std::fs::rename(&old_manifest_file, &new_manifest_file).map_err(|error| {
                RepositoryError::WriteInstance {
                    path: new_manifest_file.clone(),
                    error,
                }
            })?;
        }

        // Rewrite the manifest with the new identifier.
        let manifest_file = new_directory.join(&new_manifest_name);
        let actual_manifest_file = if manifest_file.is_file() {
            manifest_file
        } else {
            // Fall back to any single manifest in the directory.
            match manifest_file_of(&new_directory, new_id) {
                Some(file) => file,
                None => new_directory.join(&new_manifest_name),
            }
        };

        if let Some(manifest) = self.manifest(id) {
            let mut manifest = manifest.clone();
            manifest.id = Some(new_id.to_string());
            let contents = serde_json::to_string_pretty(&manifest).map_err(|error| {
                RepositoryError::ParseManifest {
                    file: actual_manifest_file.clone(),
                    error,
                }
            })?;
            std::fs::write(&actual_manifest_file, contents).map_err(|error| {
                RepositoryError::WriteInstance {
                    path: actual_manifest_file.clone(),
                    error,
                }
            })?;
        }

        self.refresh();
        Ok(())
    }

    /// Resolves an instance.
    ///
    /// The versions the instance inherits from are applied and the patches are
    /// applied in the order of their priorities.
    pub fn resolve(&self, id: &str) -> Result<InstanceManifest, ResolveError> {
        InstanceResolver::new(&self.snapshot.manifests).resolve(id)
    }
}

/// Builds the error of a draft that cannot change an instance.
fn commit_error(id: &str, error: DraftError) -> RepositoryError {
    match error {
        DraftError::UnknownInstance { .. } => {
            RepositoryError::InstanceNotFound { id: id.to_string() }
        }
        DraftError::InstanceExists { id } => RepositoryError::InstanceExists { id },
        DraftError::MissingId | DraftError::InvalidId { .. } => RepositoryError::Commit {
            error: Box::new(error),
        },
        DraftError::Write { path, error } => RepositoryError::WriteInstance { path, error },
        error => RepositoryError::Commit {
            error: Box::new(error),
        },
    }
}

/// Copies a directory and its contents recursively.
fn copy_directory(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let destination = to.join(entry.file_name());
        if source.is_dir() {
            copy_directory(&source, &destination)?;
        } else {
            std::fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}

/// Reads the instances of a game directory.
fn read_instances(
    base: &Path,
    errors: &mut Vec<RepositoryError>,
) -> (
    BTreeMap<String, InstanceManifest>,
    BTreeMap<String, PathBuf>,
    BTreeMap<String, GameVersionNumber>,
) {
    let mut manifests = BTreeMap::new();
    let mut manifest_files = BTreeMap::new();
    let mut versions = BTreeMap::new();

    let directory = base.join("versions");
    if !directory.exists() {
        return (manifests, manifest_files, versions);
    }

    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(RepositoryError::ReadDirectory { directory, error });
            return (manifests, manifest_files, versions);
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(RepositoryError::ReadDirectory {
                    directory: directory.clone(),
                    error,
                });
                continue;
            }
        };

        let instance_directory = entry.path();
        if !instance_directory.is_dir() {
            continue;
        }
        let Some(id) = instance_directory
            .file_name()
            .and_then(|name| name.to_str())
        else {
            continue;
        };
        if !is_valid_instance_id(id) {
            continue;
        }

        let Some(manifest_file) = manifest_file_of(&instance_directory, id) else {
            continue;
        };

        let contents = match std::fs::read_to_string(&manifest_file) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(RepositoryError::ReadManifest {
                    file: manifest_file,
                    error,
                });
                continue;
            }
        };
        let mut manifest: InstanceManifest = match serde_json::from_str(&contents) {
            Ok(manifest) => manifest,
            Err(error) => {
                errors.push(RepositoryError::ParseManifest {
                    file: manifest_file,
                    error,
                });
                continue;
            }
        };

        // The name of the directory is the identity of the instance, so a
        // manifest that names another instance is read with the name of the
        // directory; the files on the disk are not renamed.
        manifest.id = Some(id.to_string());

        let jar_file = manifest_file.with_extension("jar");
        if let Ok(Some(version)) = minecraft_version(&jar_file) {
            versions.insert(id.to_string(), GameVersionNumber::as_game_version(&version));
        }

        manifests.insert(id.to_string(), manifest);
        manifest_files.insert(id.to_string(), manifest_file);
    }

    (manifests, manifest_files, versions)
}

/// Returns the file of the manifest of an instance.
///
/// The official launcher names the manifest after the instance; an instance
/// whose manifest is named otherwise is read when the directory holds exactly
/// one manifest, because the launcher cannot tell which one is the manifest
/// when the directory holds more of them.
fn manifest_file_of(directory: &Path, id: &str) -> Option<PathBuf> {
    let conventional = directory.join(format!("{id}.json"));
    if conventional.is_file() {
        return Some(conventional);
    }

    let mut manifests = std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        });

    let manifest = manifests.next()?;
    if manifests.next().is_some() {
        None
    } else {
        Some(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a game directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-repository-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(directory.join("versions")).expect("the directory should be made");
        directory
    }

    /// Writes the manifest of an instance.
    fn manifest(base: &Path, directory: &str, file: &str, contents: &str) {
        let directory = base.join("versions").join(directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        std::fs::write(directory.join(file), contents).expect("the manifest should be written");
    }

    #[test]
    fn the_instances_of_the_directory_are_read() {
        let base = directory("read");
        manifest(
            &base,
            "1.20.1",
            "1.20.1.json",
            r#"{"id": "1.20.1", "mainClass": "net.minecraft.client.main.Main"}"#,
        );
        manifest(
            &base,
            "my-instance",
            "custom.json",
            r#"{"id": "not-my-instance"}"#,
        );
        // A directory without a manifest is not an instance.
        std::fs::create_dir_all(base.join("versions/empty")).expect("the directory should be made");
        // A directory with more than one manifest is not read.
        manifest(&base, "two", "first.json", "{}");
        manifest(&base, "two", "second.json", "{}");

        let (repository, errors) = GameRepository::read(&base);

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(repository.len(), 2);
        assert_eq!(
            repository
                .manifest("my-instance")
                .and_then(|manifest| manifest.id.as_deref()),
            Some("my-instance")
        );
        assert_eq!(
            repository.manifest_file("my-instance"),
            Some(base.join("versions/my-instance/custom.json").as_path())
        );
        assert_eq!(
            repository.jar_file("my-instance"),
            Some(base.join("versions/my-instance/custom.jar"))
        );
        assert_eq!(repository.version("my-instance"), None);

        // The snapshot the repository publishes describes the same instances.
        assert_eq!(repository.snapshot().len(), 2);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_marker_of_an_abnormal_launch_is_written_and_removed() {
        let base = directory("abnormal");
        manifest(&base, "1.20.1", "1.20.1.json", r#"{"id": "1.20.1"}"#);
        let (repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let marker = base.join("versions/1.20.1").join(ABNORMAL_MARKER);
        // An instance the launcher did not start has no marker.
        assert!(!repository.unmark_launched_abnormally("1.20.1"));
        assert!(!marker.exists());

        repository.mark_launched_abnormally("1.20.1");
        assert!(marker.is_file());

        // The first start after the abnormal one reads the marker and removes
        // it, so the files are checked only once.
        assert!(repository.unmark_launched_abnormally("1.20.1"));
        assert!(!marker.exists());
        assert!(!repository.unmark_launched_abnormally("1.20.1"));

        // A marker the player wrote as a directory is not a marker.
        std::fs::create_dir_all(&marker).expect("the directory should be made");
        assert!(!repository.unmark_launched_abnormally("1.20.1"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_malformed_manifest_is_reported() {
        let base = directory("malformed");
        manifest(&base, "broken", "broken.json", "not json");
        manifest(&base, "1.20.1", "1.20.1.json", r#"{"id": "1.20.1"}"#);

        let (repository, errors) = GameRepository::read(&base);

        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], RepositoryError::ParseManifest { .. }));
        assert_eq!(repository.len(), 1);
        assert!(repository.contains("1.20.1"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_instance_can_be_renamed() {
        let base = directory("rename");
        manifest(&base, "old", "old.json", r#"{"id": "old"}"#);

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(repository.contains("old"));

        repository
            .rename("old", "new")
            .expect("the instance should be renamed");

        assert!(!repository.contains("old"));
        assert!(repository.contains("new"));
        assert!(base.join("versions/new").is_dir());
        assert!(base.join("versions/new/new.json").is_file());
        assert!(!base.join("versions/old").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn renaming_an_unknown_instance_fails() {
        let base = directory("rename-missing");
        manifest(&base, "old", "old.json", r#"{"id": "old"}"#);

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let error = repository
            .rename("missing", "new")
            .expect_err("the rename should fail");
        assert!(matches!(error, RepositoryError::InstanceNotFound { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn renaming_to_an_existing_instance_fails() {
        let base = directory("rename-exists");
        manifest(&base, "old", "old.json", r#"{"id": "old"}"#);
        manifest(&base, "other", "other.json", r#"{"id": "other"}"#);

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let error = repository
            .rename("old", "other")
            .expect_err("the rename should fail");
        assert!(matches!(error, RepositoryError::InstanceExists { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_instance_can_be_deleted() {
        let base = directory("delete");
        manifest(&base, "doomed", "doomed.json", r#"{"id": "doomed"}"#);

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(repository.contains("doomed"));

        repository
            .delete("doomed")
            .expect("the instance should be deleted");

        assert!(!repository.contains("doomed"));
        assert!(!base.join("versions/doomed").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn deleting_an_unknown_instance_fails() {
        let base = directory("delete-missing");
        manifest(&base, "old", "old.json", r#"{"id": "old"}"#);

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let error = repository
            .delete("missing")
            .expect_err("the delete should fail");
        assert!(matches!(error, RepositoryError::InstanceNotFound { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_instance_can_be_copied() {
        let base = directory("copy");
        manifest(&base, "source", "source.json", r#"{"id": "source"}"#);

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(repository.contains("source"));

        repository
            .copy("source", "target")
            .expect("the instance should be copied");

        assert!(repository.contains("source"));
        assert!(repository.contains("target"));
        assert!(base.join("versions/source").is_dir());
        assert!(base.join("versions/target").is_dir());
        assert!(base.join("versions/target/target.json").is_file());
        assert_eq!(
            repository
                .manifest("target")
                .and_then(|manifest| manifest.id.as_deref()),
            Some("target")
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn copying_to_an_existing_instance_fails() {
        let base = directory("copy-exists");
        manifest(&base, "source", "source.json", r#"{"id": "source"}"#);
        manifest(&base, "target", "target.json", r#"{"id": "target"}"#);

        let (mut repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let error = repository
            .copy("source", "target")
            .expect_err("the copy should fail");
        assert!(matches!(error, RepositoryError::InstanceExists { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_inherited_versions_are_resolved() {
        let base = directory("resolve");
        manifest(&base, "1.20.1", "1.20.1.json", r#"{"id": "1.20.1"}"#);
        manifest(
            &base,
            "fabric",
            "fabric.json",
            r#"{"id": "fabric", "inheritsFrom": "1.20.1", "mainClass": "fabric.Main"}"#,
        );

        let (repository, errors) = GameRepository::read(&base);

        assert!(errors.is_empty(), "{errors:?}");
        let resolved = repository
            .resolve("fabric")
            .expect("the instance should be resolved");
        assert_eq!(resolved.main_class.as_deref(), Some("fabric.Main"));
        assert_eq!(resolved.inherits_from, None);
        assert_eq!(resolved.jar.as_deref(), Some("1.20.1"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_instance_whose_parent_is_missing_is_left_out() {
        let base = directory("missing-parent");
        manifest(
            &base,
            "orphan",
            "orphan.json",
            r#"{"id": "orphan", "inheritsFrom": "gone"}"#,
        );
        manifest(&base, "1.20.1", "1.20.1.json", r#"{"id": "1.20.1"}"#);

        let (repository, errors) = GameRepository::read(&base);

        assert!(errors.is_empty(), "{errors:?}");
        assert!(!repository.contains("orphan"));
        assert!(repository.contains("1.20.1"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_missing_directory_holds_no_instance() {
        let base =
            std::env::temp_dir().join(format!("hmcl-rs-repository-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);

        let (repository, errors) = GameRepository::read(&base);

        assert!(errors.is_empty());
        assert!(repository.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_identifiers_of_the_instances_are_read() {
        assert!(is_valid_instance_id("1.20.1"));
        assert!(is_valid_instance_id("my instance"));
        assert!(!is_valid_instance_id(""));
        assert!(!is_valid_instance_id("   "));
        assert!(!is_valid_instance_id("."));
        assert!(!is_valid_instance_id(".."));
        assert!(!is_valid_instance_id("a/b"));
        assert!(!is_valid_instance_id("a\\b"));
    }

    #[test]
    fn the_icon_of_an_instance_is_set_and_cleared() {
        let base = directory("icon");
        manifest(&base, "1.20.1", "1.20.1.json", r#"{"id": "1.20.1"}"#);
        let (repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        assert_eq!(
            repository.instance_root("1.20.1"),
            base.join("versions/1.20.1")
        );
        assert_eq!(repository.icon_file("1.20.1"), None);

        // The image is copied under the extension the launcher reads, and the
        // case of the extension does not matter.
        let image = base.join("new.PNG");
        std::fs::write(&image, b"image").expect("the image should be written");
        let icon = repository
            .set_icon_file("1.20.1", &image)
            .expect("the icon should be set");
        assert_eq!(icon, base.join("versions/1.20.1/icon.png"));
        assert_eq!(
            std::fs::read(&icon).expect("the icon should be read"),
            b"image"
        );
        assert_eq!(repository.icon_file("1.20.1"), Some(icon.clone()));

        // Setting another icon replaces the old one, so the launcher does not
        // find two icons of the instance.
        let other = base.join("other.jpg");
        std::fs::write(&other, b"other").expect("the image should be written");
        repository
            .set_icon_file("1.20.1", &other)
            .expect("the icon should be set");
        assert!(!icon.exists());
        assert_eq!(
            repository.icon_file("1.20.1"),
            Some(base.join("versions/1.20.1/icon.jpg"))
        );

        repository
            .delete_icon_file("1.20.1")
            .expect("the icon should be removed");
        assert_eq!(repository.icon_file("1.20.1"), None);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_image_that_is_not_an_icon_is_refused() {
        let base = directory("icon-refused");
        manifest(&base, "1.20.1", "1.20.1.json", r#"{"id": "1.20.1"}"#);
        let (repository, errors) = GameRepository::read(&base);
        assert!(errors.is_empty(), "{errors:?}");

        let text = base.join("icon.txt");
        std::fs::write(&text, b"text").expect("the file should be written");
        assert!(repository.set_icon_file("1.20.1", &text).is_err());
        assert_eq!(repository.icon_file("1.20.1"), None);

        // An instance the repository does not know cannot hold an icon.
        let image = base.join("icon.png");
        std::fs::write(&image, b"image").expect("the image should be written");
        assert!(repository.set_icon_file("unknown", &image).is_err());
        assert!(repository.delete_icon_file("unknown").is_err());

        let _ = std::fs::remove_dir_all(&base);
    }
}
