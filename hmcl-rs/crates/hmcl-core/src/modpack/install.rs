//! Installing the files of a modpack archive, and undoing an update that fails.
//!
//! The launcher does not write the files of an archive over the files of an
//! instance without looking at them. A modpack update unpacks the new archive
//! over the instance, and the files the player changed must survive: a player
//! who edited a configuration file or replaced a mod with the build they want
//! does not expect the launcher to undo that. The launcher therefore keeps the
//! digests of the files it unpacked, and it replaces a file only when the file
//! is still the one it unpacked.
//!
//! An update is not atomic either: the archive is unpacked, the files the new
//! version dropped are removed, and the game itself may be downloaded again.
//! Any of these steps can fail, so the launcher copies the instance before the
//! update and restores the copy when the update fails, which is what
//! [`update_with_backup`] does.

use std::fmt;
use std::path::{Path, PathBuf};

use super::configuration::FileInformation;
use super::plan::{relative_path, sha1_of};
use crate::util::digest::Algorithm;
use crate::util::zip;

/// The directory of an archive that holds the files of the modpack.
///
/// The archives of the formats that were built on CurseForge write the files
/// of the instance in `overrides`.
pub const OVERRIDES: &str = "overrides";

/// The files an install wrote, kept and removed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallOutcome {
    /// The files the install wrote.
    pub written: Vec<PathBuf>,
    /// The files the install left as they are, because the player changed them.
    pub kept: Vec<PathBuf>,
    /// The files of the previous version the install removed.
    pub removed: Vec<PathBuf>,
    /// What the install could not read of the archive.
    pub warnings: Vec<String>,
}

impl InstallOutcome {
    /// Returns whether the install changed nothing.
    pub fn is_empty(&self) -> bool {
        self.written.is_empty() && self.removed.is_empty()
    }
}

/// Returns the files an archive holds under a directory, with their digests.
///
/// The launcher keeps the list in the configuration of the instance, so it
/// knows which files the modpack owns the next time it updates the instance.
pub fn archive_overrides(
    archive: &Path,
    directories: &[&str],
) -> Result<Vec<FileInformation>, InstallError> {
    let mut overrides = Vec::new();

    for directory in directories {
        let prefix = format!("{}/", directory.trim_matches('/'));
        let entries = zip::entries(archive).map_err(InstallError::Zip)?;

        for entry in entries {
            if entry.directory {
                continue;
            }
            let Some(path) = entry.name.strip_prefix(&prefix) else {
                continue;
            };
            if path.is_empty() {
                continue;
            }

            let data = zip::read_entry(archive, &entry.name)
                .map_err(InstallError::Zip)?
                .unwrap_or_default();
            overrides.push(FileInformation::new(
                path.to_string(),
                Algorithm::Sha1.digest_to_string(&data),
            ));
        }
    }

    Ok(overrides)
}

/// Unpacks the files of an archive over an instance.
///
/// A file the previous version of the modpack did not name is written, so a file
/// the player added is replaced by the file of the modpack. A file the previous
/// version named is written only when the file is still the one the launcher
/// unpacked: a file the player changed, and a file the player deleted, are left
/// as they are.
///
/// The files the previous version named and the new archive does not hold are
/// removed, because the modpack does not use them any more. The filter tells
/// whether a file of the archive is installed at all, which the formats that
/// let the player skip an optional file need.
pub fn install(
    archive: &Path,
    destination: &Path,
    directories: &[&str],
    previous: &[FileInformation],
    filter: impl Fn(&str) -> bool,
) -> Result<InstallOutcome, InstallError> {
    std::fs::create_dir_all(destination).map_err(|error| InstallError::Io {
        path: destination.to_path_buf(),
        message: error.to_string(),
    })?;

    let mut outcome = InstallOutcome::default();
    let known: std::collections::BTreeMap<&str, &FileInformation> = previous
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect();
    let mut installed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for directory in directories {
        let prefix = format!("{}/", directory.trim_matches('/'));
        let entries = zip::entries(archive).map_err(InstallError::Zip)?;

        for entry in entries {
            if entry.directory {
                continue;
            }
            let Some(path) = entry.name.strip_prefix(&prefix) else {
                continue;
            };
            if path.is_empty() || !filter(path) {
                continue;
            }

            let Some(target) = relative_path(destination, path) else {
                outcome
                    .warnings
                    .push(format!("`{}` leaves the instance", entry.name));
                continue;
            };

            if !should_write(&target, known.get(path)) {
                installed.insert(path.to_string());
                if target.is_file() {
                    outcome.kept.push(target);
                }
                continue;
            }

            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| InstallError::Io {
                    path: parent.to_path_buf(),
                    message: error.to_string(),
                })?;
            }
            let data = zip::read_entry(archive, &entry.name)
                .map_err(InstallError::Zip)?
                .ok_or_else(|| InstallError::MissingEntry(entry.name.clone()))?;
            std::fs::write(&target, data).map_err(|error| InstallError::Io {
                path: target.clone(),
                message: error.to_string(),
            })?;

            installed.insert(path.to_string());
            outcome.written.push(target);
        }
    }

    // A file the previous version named and the new archive does not hold is
    // removed, unless the player changed it.
    for file in previous {
        if installed.contains(&file.path) {
            continue;
        }
        let Some(target) = relative_path(destination, &file.path) else {
            continue;
        };
        if !target.is_file() {
            continue;
        }

        let unchanged = file.hash.as_deref().is_some_and(|hash| {
            sha1_of(&target).is_some_and(|actual| actual.eq_ignore_ascii_case(hash))
        });
        if !unchanged {
            outcome.kept.push(target);
            continue;
        }

        std::fs::remove_file(&target).map_err(|error| InstallError::Io {
            path: target.clone(),
            message: error.to_string(),
        })?;
        outcome.removed.push(target);
    }

    Ok(outcome)
}

/// Returns whether a file of an archive is written over a file of the instance.
fn should_write(target: &Path, known: Option<&&FileInformation>) -> bool {
    let Some(known) = known else {
        // The previous version of the modpack did not name the file, so the
        // file is new and it is written over whatever the player put there.
        return true;
    };

    if !target.is_file() {
        // The player deleted the file the modpack installed, so the launcher
        // leaves it deleted.
        return false;
    }

    known
        .hash
        .as_deref()
        .is_some_and(|hash| sha1_of(target).is_some_and(|actual| actual.eq_ignore_ascii_case(hash)))
}

/// Runs an update of an instance, with a copy the launcher restores if the
/// update fails.
///
/// The copy is made before the update and kept after a success, so the player
/// can go back to the version they had. A failed update is undone by removing
/// the half-updated instance and copying the backup back, which leaves the
/// instance as it was before the update.
///
/// The name of the backup is returned, so the caller can tell the player where
/// the copy is.
pub fn update_with_backup<T, E>(
    instance_root: &Path,
    backup_root: &Path,
    instance_id: &str,
    update: impl FnOnce() -> Result<T, E>,
) -> Result<(T, PathBuf), UpdateError<E>> {
    let backup = unique_backup_directory(backup_root, instance_id);

    copy_directory(instance_root, &backup).map_err(|error| UpdateError::Backup {
        path: backup.clone(),
        message: error.to_string(),
    })?;

    match update() {
        Ok(value) => Ok((value, backup)),
        Err(error) => {
            let rollback = roll_back(instance_root, &backup);
            match rollback {
                Ok(()) => Err(UpdateError::Update { source: error }),
                Err((path, message)) => Err(UpdateError::Rollback {
                    path,
                    message,
                    source: error,
                }),
            }
        }
    }
}

/// Restores an instance from the copy the launcher made before the update.
///
/// The instance is removed first, because the half-updated copy may hold files
/// the backup does not, and the restored instance must be the one the player
/// had.
fn roll_back(instance_root: &Path, backup: &Path) -> Result<(), (PathBuf, String)> {
    if instance_root.exists() {
        std::fs::remove_dir_all(instance_root).map_err(|error| {
            (
                instance_root.to_path_buf(),
                format!("the half-updated instance cannot be removed: {error}"),
            )
        })?;
    }

    copy_directory(backup, instance_root)
        .map_err(|error| (instance_root.to_path_buf(), error.to_string()))
}

/// Returns the name of a backup directory that is not used yet.
///
/// The name holds a number the launcher draws from the counter and the clock,
/// so two updates of the same instance do not write the same backup.
fn unique_backup_directory(backup_root: &Path, instance_id: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let number = COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos() as u64)
            .unwrap_or(0),
    ) % 10_000_000;

    let mut candidate = backup_root.join(format!("{instance_id}-{number}"));
    let mut attempt = 0u64;
    while candidate.exists() {
        attempt = attempt.wrapping_add(1);
        candidate = backup_root.join(format!("{instance_id}-{number}-{attempt}"));
    }
    candidate
}

/// Copies a directory and its contents.
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

/// The failure of an install.
#[derive(Debug)]
pub enum InstallError {
    /// A file of the instance cannot be written or removed.
    Io {
        /// The file the launcher worked with.
        path: PathBuf,
        /// The reason the file cannot be written.
        message: String,
    },
    /// The archive cannot be read.
    Zip(zip::ZipError),
    /// The archive changed while the launcher read it.
    MissingEntry(String),
}

impl fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(formatter, "cannot write {}: {message}", path.display())
            }
            Self::Zip(error) => write!(formatter, "the archive cannot be read: {error}"),
            Self::MissingEntry(name) => {
                write!(formatter, "the archive no longer holds `{name}`")
            }
        }
    }
}

impl std::error::Error for InstallError {}

/// The failure of an update.
#[derive(Debug)]
pub enum UpdateError<E> {
    /// The instance cannot be copied before the update starts.
    Backup {
        /// The directory the copy was written to.
        path: PathBuf,
        /// The reason the copy failed.
        message: String,
    },
    /// The update failed.
    Update {
        /// The failure of the update.
        source: E,
    },
    /// The update failed and the instance cannot be restored.
    Rollback {
        /// The directory the launcher restored.
        path: PathBuf,
        /// The reason the restore failed.
        message: String,
        /// The failure of the update that made the restore necessary.
        source: E,
    },
}

impl<E: fmt::Display> fmt::Display for UpdateError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backup { path, message } => write!(
                formatter,
                "cannot copy the instance to {} before the update: {message}",
                path.display()
            ),
            Self::Update { source } => write!(formatter, "the update failed: {source}"),
            Self::Rollback {
                path,
                message,
                source,
            } => write!(
                formatter,
                "the update failed ({source}) and the instance cannot be restored from {}: {message}",
                path.display()
            ),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for UpdateError<E> {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a directory of the tests that the other tests do not use.
    fn directory(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("hmcl-rs-install-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Writes an archive of a modpack with the files the test names.
    fn archive(base: &Path, files: &[(&str, &str)]) -> PathBuf {
        let path = base.join("pack.zip");
        let mut writer = zip::ZipWriter::create(&path).expect("the archive should be created");
        for (name, text) in files {
            writer
                .add_file(name, text.as_bytes())
                .expect("the file should be added");
        }
        writer.finish().expect("the archive should be written");
        path
    }

    /// Writes a file of the instance.
    fn write(base: &Path, path: &str, text: &str) {
        let file = base.join(path);
        std::fs::create_dir_all(file.parent().expect("the file has a parent"))
            .expect("the directory should be created");
        std::fs::write(file, text).expect("the file should be written");
    }

    /// Returns the information of a file the archive holds.
    fn info(path: &str, text: &str) -> FileInformation {
        FileInformation::new(path, Algorithm::Sha1.digest_to_string(text.as_bytes()))
    }

    #[test]
    fn the_files_of_an_archive_are_listed_with_their_digests() {
        let base = directory("list");
        let archive = archive(
            &base,
            &[
                ("overrides/config/a.toml", "a"),
                ("overrides/mods/b.jar", "b"),
                ("manifest.json", "{}"),
            ],
        );

        let overrides =
            archive_overrides(&archive, &[OVERRIDES]).expect("the archive should be read");
        let paths: Vec<&str> = overrides.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, vec!["config/a.toml", "mods/b.jar"]);
        assert_eq!(
            overrides[0].hash.as_deref(),
            Some(Algorithm::Sha1.digest_to_string(b"a").as_str())
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_new_file_of_the_modpack_is_written() {
        let base = directory("new");
        let archive = archive(&base, &[("overrides/config/a.toml", "a")]);
        let instance = base.join("instance");

        let outcome = install(&archive, &instance, &[OVERRIDES], &[], |_| true)
            .expect("the archive should be installed");
        assert_eq!(outcome.written, vec![instance.join("config/a.toml")]);
        assert_eq!(
            std::fs::read_to_string(instance.join("config/a.toml"))
                .expect("the file should be read"),
            "a"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_player_changed_is_kept() {
        let base = directory("kept");
        let archive = archive(&base, &[("overrides/config/a.toml", "new")]);
        let instance = base.join("instance");
        write(&instance, "config/a.toml", "changed by the player");

        let previous = vec![info("config/a.toml", "old")];
        let outcome = install(&archive, &instance, &[OVERRIDES], &previous, |_| true)
            .expect("the archive should be installed");

        assert!(outcome.written.is_empty());
        assert_eq!(outcome.kept, vec![instance.join("config/a.toml")]);
        assert_eq!(
            std::fs::read_to_string(instance.join("config/a.toml"))
                .expect("the file should be read"),
            "changed by the player"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_player_did_not_change_is_replaced() {
        let base = directory("replaced");
        let archive = archive(&base, &[("overrides/config/a.toml", "new")]);
        let instance = base.join("instance");
        write(&instance, "config/a.toml", "old");

        let previous = vec![info("config/a.toml", "old")];
        let outcome = install(&archive, &instance, &[OVERRIDES], &previous, |_| true)
            .expect("the archive should be installed");

        assert_eq!(outcome.written, vec![instance.join("config/a.toml")]);
        assert_eq!(
            std::fs::read_to_string(instance.join("config/a.toml"))
                .expect("the file should be read"),
            "new"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_player_deleted_stays_deleted() {
        let base = directory("deleted");
        let archive = archive(&base, &[("overrides/config/a.toml", "new")]);
        let instance = base.join("instance");
        std::fs::create_dir_all(&instance).expect("the directory should be created");

        let previous = vec![info("config/a.toml", "old")];
        let outcome = install(&archive, &instance, &[OVERRIDES], &previous, |_| true)
            .expect("the archive should be installed");

        assert!(outcome.written.is_empty());
        assert!(!instance.join("config/a.toml").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_new_version_dropped_is_removed() {
        let base = directory("dropped");
        let archive = archive(&base, &[("overrides/config/a.toml", "a")]);
        let instance = base.join("instance");
        write(&instance, "config/a.toml", "a");
        write(&instance, "config/b.toml", "b");

        let previous = vec![info("config/a.toml", "a"), info("config/b.toml", "b")];
        let outcome = install(&archive, &instance, &[OVERRIDES], &previous, |_| true)
            .expect("the archive should be installed");

        assert_eq!(outcome.removed, vec![instance.join("config/b.toml")]);
        assert!(!instance.join("config/b.toml").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_player_changed_is_not_removed() {
        let base = directory("dropped-changed");
        let archive = archive(&base, &[]);
        let instance = base.join("instance");
        write(&instance, "config/b.toml", "changed by the player");

        let previous = vec![info("config/b.toml", "b")];
        let outcome = install(&archive, &instance, &[OVERRIDES], &previous, |_| true)
            .expect("the archive should be installed");

        assert!(outcome.removed.is_empty());
        assert_eq!(outcome.kept, vec![instance.join("config/b.toml")]);
        assert!(instance.join("config/b.toml").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_filter_skips_the_files_the_player_did_not_choose() {
        let base = directory("filter");
        let archive = archive(
            &base,
            &[("overrides/mods/a.jar", "a"), ("overrides/mods/b.jar", "b")],
        );
        let instance = base.join("instance");

        let outcome = install(&archive, &instance, &[OVERRIDES], &[], |path| {
            path != "mods/b.jar"
        })
        .expect("the archive should be installed");

        assert_eq!(outcome.written, vec![instance.join("mods/a.jar")]);
        assert!(!instance.join("mods/b.jar").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_archive_without_the_directory_installs_nothing() {
        let base = directory("no-directory");
        let archive = archive(&base, &[("files/a.txt", "a")]);
        let instance = base.join("instance");

        let outcome = install(&archive, &instance, &[OVERRIDES], &[], |_| true)
            .expect("the archive should be installed");
        assert!(outcome.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_failed_update_restores_the_instance() {
        let base = directory("rollback");
        let instance = base.join("instance");
        let backups = base.join("backup");
        write(&instance, "config/a.toml", "the original");

        let result: Result<((), PathBuf), UpdateError<&str>> =
            update_with_backup(&instance, &backups, "world", || {
                // The update changes the instance before it fails.
                std::fs::write(instance.join("config/a.toml"), "half updated")
                    .expect("the file should be written");
                std::fs::write(instance.join("new-file.txt"), "new")
                    .expect("the file should be written");
                Err("the game could not be downloaded")
            });

        assert!(matches!(result, Err(UpdateError::Update { .. })));
        assert_eq!(
            std::fs::read_to_string(instance.join("config/a.toml"))
                .expect("the file should be read"),
            "the original"
        );
        // The file the failed update wrote does not survive the restore.
        assert!(!instance.join("new-file.txt").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_successful_update_keeps_the_backup() {
        let base = directory("backup-kept");
        let instance = base.join("instance");
        let backups = base.join("backup");
        write(&instance, "config/a.toml", "the original");

        let (value, backup) = update_with_backup(&instance, &backups, "world", || {
            std::fs::write(instance.join("config/a.toml"), "updated")
                .expect("the file should be written");
            Ok::<_, &str>(42)
        })
        .expect("the update should succeed");

        assert_eq!(value, 42);
        assert!(backup.starts_with(&backups));
        assert_eq!(
            std::fs::read_to_string(backup.join("config/a.toml")).expect("the file should be read"),
            "the original"
        );
        assert_eq!(
            std::fs::read_to_string(instance.join("config/a.toml"))
                .expect("the file should be read"),
            "updated"
        );

        // The second update writes another backup, so the first one survives.
        let (_, second) = update_with_backup(&instance, &backups, "world", || Ok::<_, &str>(()))
            .expect("the update should succeed");
        assert_ne!(backup, second);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_path_that_leaves_the_instance_is_not_written() {
        let base = directory("outside");
        let archive = archive(&base, &[("overrides/../outside.txt", "a")]);
        let instance = base.join("instance");

        let outcome = install(&archive, &instance, &[OVERRIDES], &[], |_| true)
            .expect("the archive should be installed");
        assert!(outcome.written.is_empty());
        assert_eq!(outcome.warnings.len(), 1);
        assert!(!base.join("outside.txt").exists());

        let _ = std::fs::remove_dir_all(&base);
    }
}
