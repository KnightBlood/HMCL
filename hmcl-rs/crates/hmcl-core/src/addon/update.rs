//! Replacing the addon files of a player with the versions the repositories
//! offer.
//!
//! Checking for an update only finds the version a file can be replaced with;
//! replacing the file is the work of this module. The launcher of Java
//! downloads the new file to a temporary place, marks the file of the player as
//! a backup, moves the new file where the game reads the addon, and then
//! decides what to do with the backup: a mod keeps the backup, so the player
//! can roll back, while a resource pack deletes it, for the game reads the packs
//! by the name of their file and an old pack would show up in the list.
//!
//! A failure restores the file of the player, so a player who loses the
//! connection while an addon is updated keeps the version that worked.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::addon::local_mod::{display_name, ModFile, DISABLED_EXTENSION, OLD_EXTENSION};
use crate::addon::mod_loader::ModLoaderType;
use crate::addon::pack::{Pack, PackKind};
use crate::addon::remote_addon::{AddonVersion, UpdateRestriction};
use crate::download::cache::CacheRepository;
use crate::download::provider::DownloadProvider;
use crate::download::task::{
    DownloadError, DownloadedFile, Fetcher, FileDownloadTask, IntegrityCheck,
};
use crate::util::digest::Algorithm;

/// The kind of a local addon file the launcher updates.
///
/// The kind decides which versions the file can be replaced with and what
/// happens to the file of the player once the new file is in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonKind {
    /// A mod of the game, which is written for one loader.
    Mod {
        /// The loader the mod was read for.
        loader: ModLoaderType,
    },
    /// An archive of a resource pack.
    ///
    /// A resource pack names no loader, so any version the game runs is taken.
    /// A pack that is a directory rather than an archive is not updated, for
    /// the repository offers archives only.
    ResourcePackArchive,
}

impl AddonKind {
    /// Returns the conditions the target version of the file has to satisfy.
    ///
    /// A mod is only replaced by a version that is written for the loader of
    /// the mod; a resource pack archive is loaded whatever loader the version
    /// names, so it accepts every version. The launcher of Java lets a local
    /// addon file name these conditions, and every file it does not cover is
    /// not upgradable at all.
    pub fn restrictions(self) -> Vec<UpdateRestriction> {
        match self {
            Self::Mod { loader } => vec![UpdateRestriction::loader(loader)],
            Self::ResourcePackArchive => Vec::new(),
        }
    }

    /// Returns whether the launcher keeps the file of the player after the
    /// update.
    ///
    /// A mod is kept as a backup, so the player can roll back to the version
    /// that worked. A resource pack is deleted, so the list of the packs of the
    /// game does not grow with every update.
    pub const fn keeps_old_files(self) -> bool {
        matches!(self, Self::Mod { .. })
    }

    /// Returns whether the launcher can disable the file.
    ///
    /// A mod is disabled by the suffix of its name. A resource pack is enabled
    /// by the options of the game rather than by its name, so a file name that
    /// carries the suffix of a disabled mod means nothing to the pack.
    pub const fn is_disabled_by_name(self) -> bool {
        matches!(self, Self::Mod { .. })
    }

    /// Returns the kind of a mod the launcher read.
    ///
    /// The loader of the mod is the one the file was read for, which is the
    /// loader the game needs, so a version of another loader is not an update
    /// of it.
    pub fn of_mod(mod_file: &ModFile) -> Self {
        Self::Mod {
            loader: mod_file.loader(),
        }
    }

    /// Returns the kind of a pack the launcher read, if the launcher updates
    /// the pack at all.
    ///
    /// Only an archive of a resource pack is updated: a data pack is not
    /// offered by the repositories, and a pack that is a directory has no file
    /// to replace. The answer is `None` for both, so a caller does not have to
    /// guess which files of the directory are updatable.
    pub fn of_pack(pack: &Pack) -> Option<Self> {
        if pack.kind() != PackKind::ResourcePack || pack.path().is_dir() {
            return None;
        }
        Some(Self::ResourcePackArchive)
    }
}

/// An addon file the launcher replaces with a version of a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonUpdateRequest {
    /// The directory the addon belongs in, which is where the new file is put.
    pub directory: PathBuf,
    /// The file of the player.
    pub current_path: PathBuf,
    /// The version the file is replaced with.
    pub target: AddonVersion,
    /// The kind of the file, which decides the conditions and the cleanup.
    pub kind: AddonKind,
}

impl AddonUpdateRequest {
    /// Creates a request that replaces a file with a version.
    pub fn new(
        directory: impl Into<PathBuf>,
        current_path: impl Into<PathBuf>,
        target: AddonVersion,
        kind: AddonKind,
    ) -> Self {
        Self {
            directory: directory.into(),
            current_path: current_path.into(),
            target,
            kind,
        }
    }

    /// Creates a request that replaces a mod the launcher read.
    ///
    /// The directory of the mod is the one the file is in, so a caller does not
    /// have to tell it apart from the file of the mod.
    pub fn of_mod(mod_file: &ModFile, target: AddonVersion) -> Self {
        Self {
            directory: directory_of(mod_file.path()),
            current_path: mod_file.path().to_path_buf(),
            target,
            kind: AddonKind::of_mod(mod_file),
        }
    }

    /// Creates a request that replaces a pack the launcher read, if the pack is
    /// one the launcher updates.
    ///
    /// The answer is `None` for a pack that is not an archive of a resource
    /// pack, which is the case of the data packs and of the packs that are
    /// directories.
    pub fn of_pack(pack: &Pack, target: AddonVersion) -> Option<Self> {
        Some(Self {
            directory: directory_of(pack.path()),
            current_path: pack.path().to_path_buf(),
            target,
            kind: AddonKind::of_pack(pack)?,
        })
    }

    /// Returns whether the file of the player is disabled.
    pub fn is_disabled(&self) -> bool {
        self.current_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(DISABLED_EXTENSION))
    }

    /// Returns the name of the file the addon is put in.
    ///
    /// The name comes from the repository, so the launcher knows which file it
    /// holds without reading the metadata again. A disabled mod stays disabled,
    /// because the player chose not to load it.
    pub fn file_name(&self) -> String {
        let name = &self.target.file.filename;
        if self.is_disabled() && self.kind.is_disabled_by_name() {
            format!("{name}{DISABLED_EXTENSION}")
        } else {
            name.clone()
        }
    }

    /// Returns the place the new file is put in.
    pub fn target_path(&self) -> PathBuf {
        self.directory.join(self.file_name())
    }

    /// Returns the place the file of the player is kept as a backup.
    ///
    /// A backup is never disabled, for the game would not load it either way;
    /// the launcher of Java moves a disabled file to an enabled backup name.
    pub fn backup_path(&self) -> PathBuf {
        let name = self
            .current_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        self.current_path
            .with_file_name(format!("{}{OLD_EXTENSION}", display_name(name)))
    }
}

/// An addon file the launcher replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledUpdate {
    /// The file of the player before the update.
    pub current_path: PathBuf,
    /// The file the addon is now in.
    pub new_path: PathBuf,
    /// The backup of the file of the player, if the launcher kept one.
    ///
    /// The backup is `None` for an addon whose old files are not kept, and for
    /// a file the player no longer had when the update was applied.
    pub backup_path: Option<PathBuf>,
    /// The version the addon is now.
    pub version: AddonVersion,
}

/// The failure of replacing an addon file.
#[derive(Debug)]
pub enum UpdateError {
    /// The new file could not be downloaded.
    Download(DownloadError),
    /// A file could not be moved, or the directory of the addon cannot be used.
    Io {
        /// The path the operation was about.
        path: PathBuf,
        /// The reason the operation failed.
        error: std::io::Error,
    },
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Download(error) => write!(formatter, "the addon cannot be downloaded: {error}"),
            Self::Io { path, error } => {
                write!(formatter, "{} cannot be used: {error}", path.display())
            }
        }
    }
}

impl std::error::Error for UpdateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Download(error) => Some(error),
            Self::Io { error, .. } => Some(error),
        }
    }
}

impl From<DownloadError> for UpdateError {
    fn from(error: DownloadError) -> Self {
        Self::Download(error)
    }
}

/// The result of replacing a list of addon files.
///
/// The launcher of Java updates the addons one by one and remembers the ones
/// it could not update, so one broken repository or a lost connection does not
/// stop the others.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddonUpdateReport {
    /// The files the launcher replaced.
    pub updated: Vec<InstalledUpdate>,
    /// The files the launcher could not replace, with the reason.
    pub failed: Vec<(PathBuf, String)>,
}

impl AddonUpdateReport {
    /// Returns whether every file was replaced.
    pub fn is_complete(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Returns the directory a file is in.
///
/// A file with no parent is taken to be in the directory the launcher runs in,
/// which is what a path of a bare file name means.
fn directory_of(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The number of the updates of this run of the launcher, so two updates of the
/// same file do not write the same temporary file.
static UPDATE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Returns a place to download a file to before it is put in place.
///
/// The place is inside the directory of the addon, so the move that puts the
/// file in place does not cross a file system. The launcher of Java uses the
/// temporary directory of the system, which fails when the directory of the
/// game is on another drive.
fn temporary_path(directory: &Path) -> PathBuf {
    let number = UPDATE_COUNTER.fetch_add(1, Ordering::Relaxed);
    directory.join(format!(
        ".hmcl-addon-update-{}-{number}.tmp",
        std::process::id()
    ))
}

/// Moves a file, replacing the file that is already at the destination.
///
/// A missing source is not an error, because the file of the player may be
/// gone; the destination is then left as it is. The destination is removed
/// first, because a file cannot be moved over an existing file on every file
/// system.
fn move_file(from: &Path, to: &Path) -> Result<(), UpdateError> {
    if !from.exists() {
        return Ok(());
    }
    if to.exists() && to != from {
        std::fs::remove_file(to).map_err(|error| UpdateError::Io {
            path: to.to_path_buf(),
            error,
        })?;
    }
    if to == from {
        return Ok(());
    }
    std::fs::rename(from, to).map_err(|error| UpdateError::Io {
        path: from.to_path_buf(),
        error,
    })
}

/// Returns the check the file of a version is held to, if the repository told
/// a digest.
fn integrity_check(version: &AddonVersion) -> Option<IntegrityCheck> {
    let (algorithm, digest) = version.file.integrity_check()?;
    let algorithm = match algorithm {
        "sha512" => Algorithm::Sha512,
        "sha256" => Algorithm::Sha256,
        "sha1" => Algorithm::Sha1,
        "md5" => Algorithm::Md5,
        _ => return None,
    };
    IntegrityCheck::of(algorithm, Some(digest))
}

/// Downloads the new file of an addon to its temporary place.
fn download_new_file(
    request: &AddonUpdateRequest,
    provider: &DownloadProvider,
    fetcher: &dyn Fetcher,
    cache: Option<&mut CacheRepository>,
) -> Result<(PathBuf, DownloadedFile), UpdateError> {
    let temporary = temporary_path(&request.directory);
    let candidates = provider.download_candidates(std::slice::from_ref(&request.target.file.url));
    let task = FileDownloadTask::new(candidates, temporary.clone())
        .with_optional_integrity_check(integrity_check(&request.target));

    match task.download(fetcher, cache) {
        Ok(downloaded) => Ok((temporary, downloaded)),
        Err(error) => {
            // A half written file is of no use to anybody.
            let _ = std::fs::remove_file(&temporary);
            Err(UpdateError::Download(error))
        }
    }
}

/// Replaces the file of the player with the version of a repository.
///
/// The file of the player is moved to a backup name before the new file is put
/// in place, so a failure never leaves the player without the addon. When the
/// move of the new file fails, the backup is restored, and a mod that was
/// disabled is disabled again. On success the backup is kept or deleted as the
/// kind of the addon tells.
///
/// The managers of the addons keep the files they read, so a caller that uses
/// one has to read the directory again after an update.
pub fn install_update(
    request: &AddonUpdateRequest,
    provider: &DownloadProvider,
    fetcher: &dyn Fetcher,
    cache: Option<&mut CacheRepository>,
) -> Result<InstalledUpdate, UpdateError> {
    if let Err(error) = std::fs::create_dir_all(&request.directory) {
        return Err(UpdateError::Io {
            path: request.directory.clone(),
            error,
        });
    }

    let (temporary, _downloaded) = download_new_file(request, provider, fetcher, cache)?;

    let backup = request.backup_path();
    let target = request.target_path();
    let disabled = request.is_disabled();

    // The addon is backed up before the new file is put in place, so the player
    // keeps the version that worked however the update ends.
    if let Err(error) = move_file(&request.current_path, &backup) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }

    if let Err(error) = move_file(&temporary, &target) {
        let _ = std::fs::remove_file(&temporary);
        restore(&request.current_path, &backup, disabled, request.kind);
        return Err(error);
    }

    let kept = if request.kind.keeps_old_files() && backup.exists() {
        Some(backup)
    } else {
        let _ = std::fs::remove_file(&backup);
        None
    };

    Ok(InstalledUpdate {
        current_path: request.current_path.clone(),
        new_path: target,
        backup_path: kept,
        version: request.target.clone(),
    })
}

/// Puts the file of the player back after a failed update.
///
/// The backup is moved back to the name the file had. A mod that was disabled
/// is disabled again, because a backup is never disabled; a resource pack is
/// not renamed, for the game reads its state from the options.
fn restore(current_path: &Path, backup: &Path, disabled: bool, kind: AddonKind) {
    let restored = if disabled && kind.is_disabled_by_name() {
        current_path.with_file_name(format!(
            "{}{DISABLED_EXTENSION}",
            display_name(
                current_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
            )
        ))
    } else {
        current_path.to_path_buf()
    };

    if move_file(backup, &restored).is_ok() && restored != current_path {
        // The file is disabled again, and the name without the suffix is no
        // longer needed.
        let _ = std::fs::remove_file(current_path);
    }
}

/// Replaces a list of addon files, keeping the ones that could not be updated.
///
/// Every file is updated on its own, so a repository that cannot be reached, a
/// file that cannot be written or a digest that does not match only stops the
/// update of that file.
pub fn install_updates(
    requests: &[AddonUpdateRequest],
    provider: &DownloadProvider,
    fetcher: &dyn Fetcher,
    mut cache: Option<&mut CacheRepository>,
) -> AddonUpdateReport {
    let mut report = AddonUpdateReport::default();

    for request in requests {
        match install_update(request, provider, fetcher, cache.as_deref_mut()) {
            Ok(update) => report.updated.push(update),
            Err(error) => report
                .failed
                .push((request.current_path.clone(), error.to_string())),
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io::Write;

    use super::*;
    use crate::addon::remote_addon::{AddonFile, AddonSource, VersionType};
    use crate::download::task::{DownloadError, ResponseInfo};

    /// A server that answers every address with the same body.
    struct FakeServer {
        /// The body the server answers with.
        body: Vec<u8>,
        /// The addresses the launcher asked, in the order it asked them.
        asked: RefCell<Vec<String>>,
        /// Whether the server reports the body as broken.
        broken: bool,
    }

    impl FakeServer {
        fn new(body: &[u8]) -> Self {
            Self {
                body: body.to_vec(),
                asked: RefCell::new(Vec::new()),
                broken: false,
            }
        }

        fn broken(body: &[u8]) -> Self {
            Self {
                body: body.to_vec(),
                asked: RefCell::new(Vec::new()),
                broken: true,
            }
        }
    }

    impl Fetcher for FakeServer {
        fn fetch(
            &self,
            url: &str,
            _headers: &BTreeMap<String, String>,
            sink: &mut dyn Write,
        ) -> Result<ResponseInfo, String> {
            self.asked.borrow_mut().push(url.to_string());
            if self.broken {
                return Err("the connection broke".to_string());
            }
            sink.write_all(&self.body)
                .map_err(|error| error.to_string())?;
            Ok(ResponseInfo {
                url: url.to_string(),
                status: 200,
                headers: BTreeMap::new(),
            })
        }
    }

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-rs-addon-update-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    /// Returns a version of an addon with a file name and a digest.
    fn version(file_name: &str, digest: Option<&str>) -> AddonVersion {
        let mut hashes = BTreeMap::new();
        if let Some(digest) = digest {
            hashes.insert("sha1".to_string(), digest.to_string());
        }
        AddonVersion {
            source: AddonSource::Modrinth,
            version_id: "2".to_string(),
            project_id: "AANobbMI".to_string(),
            name: file_name.to_string(),
            version: "2.0".to_string(),
            date_published: Some("2024-05-01T12:00:00Z".to_string()),
            version_type: VersionType::Release,
            file: AddonFile {
                hashes,
                url: "https://example.invalid/file.jar".to_string(),
                filename: file_name.to_string(),
            },
            dependencies: Vec::new(),
            game_versions: vec!["1.20.1".to_string()],
            loaders: Vec::new(),
        }
    }

    #[test]
    fn a_mod_keeps_the_file_of_the_player_as_a_backup() {
        let directory = directory("mod");
        let current = directory.join("sodium.jar");
        std::fs::write(&current, b"old").expect("the file should be written");

        let request = AddonUpdateRequest::new(
            &directory,
            &current,
            version("sodium-2.0.jar", None),
            AddonKind::Mod {
                loader: ModLoaderType::Fabric,
            },
        );

        let server = FakeServer::new(b"new");
        let installed = install_update(&request, &DownloadProvider::hmcl(false), &server, None)
            .expect("the update should be applied");

        assert_eq!(installed.new_path, directory.join("sodium-2.0.jar"));
        assert_eq!(
            std::fs::read(&installed.new_path).expect("the file should be read"),
            b"new"
        );
        // The mod keeps the version the player had, so it can be rolled back.
        let backup = installed.backup_path.expect("a backup should be kept");
        assert_eq!(backup, directory.join("sodium.jar.old"));
        assert_eq!(
            std::fs::read(&backup).expect("the file should be read"),
            b"old"
        );
        assert!(!current.exists());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_resource_pack_deletes_the_file_of_the_player() {
        let directory = directory("pack");
        let current = directory.join("shiny.zip");
        std::fs::write(&current, b"old").expect("the file should be written");

        let request = AddonUpdateRequest::new(
            &directory,
            &current,
            version("shiny-2.0.zip", None),
            AddonKind::ResourcePackArchive,
        );

        let server = FakeServer::new(b"new");
        let installed = install_update(&request, &DownloadProvider::hmcl(false), &server, None)
            .expect("the update should be applied");

        assert!(installed.backup_path.is_none());
        assert!(!directory.join("shiny.zip.old").exists());
        assert!(installed.new_path.exists());
        assert!(!current.exists());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_disabled_mod_is_updated_disabled() {
        let directory = directory("disabled");
        let current = directory.join("sodium.jar.disabled");
        std::fs::write(&current, b"old").expect("the file should be written");

        let request = AddonUpdateRequest::new(
            &directory,
            &current,
            version("sodium-2.0.jar", None),
            AddonKind::Mod {
                loader: ModLoaderType::Fabric,
            },
        );

        assert_eq!(request.file_name(), "sodium-2.0.jar.disabled");
        assert_eq!(request.backup_path(), directory.join("sodium.jar.old"));

        let server = FakeServer::new(b"new");
        let installed = install_update(&request, &DownloadProvider::hmcl(false), &server, None)
            .expect("the update should be applied");

        assert_eq!(
            installed.new_path,
            directory.join("sodium-2.0.jar.disabled")
        );
        assert!(installed.new_path.exists());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_failed_download_keeps_the_file_of_the_player() {
        let directory = directory("failed-download");
        let current = directory.join("sodium.jar");
        std::fs::write(&current, b"old").expect("the file should be written");

        let request = AddonUpdateRequest::new(
            &directory,
            &current,
            version("sodium-2.0.jar", None),
            AddonKind::Mod {
                loader: ModLoaderType::Fabric,
            },
        );

        let server = FakeServer::broken(b"new");
        let error = install_update(&request, &DownloadProvider::hmcl(false), &server, None)
            .expect_err("the update should fail");

        assert!(matches!(error, UpdateError::Download(_)));
        // The file is where it was, with the content it had.
        assert_eq!(
            std::fs::read(&current).expect("the file should be read"),
            b"old"
        );
        assert!(!directory.join("sodium-2.0.jar").exists());
        // No temporary file is left behind.
        let left: Vec<_> = std::fs::read_dir(&directory)
            .expect("the directory should be read")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, vec!["sodium.jar".to_string()]);

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_file_of_another_digest_is_refused() {
        let directory = directory("digest");
        let current = directory.join("sodium.jar");
        std::fs::write(&current, b"old").expect("the file should be written");

        // The repository names a digest, but the file does not match it.
        let request = AddonUpdateRequest::new(
            &directory,
            &current,
            version(
                "sodium-2.0.jar",
                Some("0000000000000000000000000000000000000000"),
            ),
            AddonKind::Mod {
                loader: ModLoaderType::Fabric,
            },
        );

        let server = FakeServer::new(b"new");
        let error = install_update(&request, &DownloadProvider::hmcl(false), &server, None)
            .expect_err("the update should fail");

        assert!(matches!(
            error,
            UpdateError::Download(DownloadError::Checksum { .. })
        ));
        assert_eq!(
            std::fs::read(&current).expect("the file should be read"),
            b"old"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_conditions_of_the_kinds_are_the_ones_of_the_launcher_of_java() {
        // A mod asks for a version of its loader; a resource pack asks nothing.
        assert_eq!(
            AddonKind::Mod {
                loader: ModLoaderType::Forge,
            }
            .restrictions(),
            vec![UpdateRestriction::loader(ModLoaderType::Forge)]
        );
        assert!(AddonKind::ResourcePackArchive.restrictions().is_empty());
        // Only the mod keeps a backup, and only a mod is disabled by its name.
        assert!(AddonKind::Mod {
            loader: ModLoaderType::Fabric,
        }
        .keeps_old_files());
        assert!(!AddonKind::ResourcePackArchive.keeps_old_files());
        assert!(AddonKind::Mod {
            loader: ModLoaderType::Fabric,
        }
        .is_disabled_by_name());
        assert!(!AddonKind::ResourcePackArchive.is_disabled_by_name());
    }

    #[test]
    fn a_mod_file_becomes_a_request_of_its_own_directory() {
        let directory = directory("request-mod");
        let path = directory.join("sodium.jar");
        std::fs::write(&path, b"old").expect("the file should be written");

        let mod_file = ModFile::open(&path, &[ModLoaderType::Fabric]);
        let request = AddonUpdateRequest::of_mod(&mod_file, version("sodium-2.0.jar", None));

        assert_eq!(request.directory, directory);
        assert_eq!(request.current_path, path);
        // The loader the file was read for is the one the update asks for.
        assert_eq!(
            request.kind,
            AddonKind::Mod {
                loader: mod_file.loader()
            }
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn only_an_archive_of_a_resource_pack_becomes_a_request() {
        let directory = directory("request-pack");
        let archive = directory.join("shiny.zip");
        let mut writer =
            crate::util::zip::ZipWriter::create(&archive).expect("the archive should be made");
        writer
            .add_file("pack.mcmeta", br#"{ "pack": { "pack_format": 15 } }"#)
            .expect("the metadata should be added");
        writer.finish().expect("the archive should be written");

        let pack =
            Pack::open(&archive, PackKind::ResourcePack).expect("the archive should be read");
        let request = AddonUpdateRequest::of_pack(&pack, version("shiny-2.0.zip", None))
            .expect("an archive of a resource pack should be updatable");

        assert_eq!(request.directory, directory);
        assert_eq!(request.kind, AddonKind::ResourcePackArchive);

        // A data pack is not offered by the repositories.
        let data_pack =
            Pack::open(&archive, PackKind::DataPack).expect("the archive should be read");
        assert!(AddonUpdateRequest::of_pack(&data_pack, version("shiny-2.0.zip", None)).is_none());

        // A directory holds no file to replace.
        let folder = directory.join("folder");
        std::fs::create_dir_all(&folder).expect("the directory should be made");
        std::fs::write(folder.join("pack.mcmeta"), b"{}").expect("the file should be written");
        let pack = Pack::open(&folder, PackKind::ResourcePack).expect("the pack should be read");
        assert!(AddonUpdateRequest::of_pack(&pack, version("shiny-2.0.zip", None)).is_none());

        let _ = std::fs::remove_dir_all(&directory);
    }
}
