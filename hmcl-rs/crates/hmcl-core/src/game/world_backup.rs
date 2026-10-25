//! The backups the launcher makes of the worlds.
//!
//! A backup is an archive of a world, named after the moment it was made and
//! the name of the world. The launcher keeps the backups in `backups`, so a
//! player whose world is broken can put one back.

use std::path::{Path, PathBuf};

use super::world::World;
use crate::util::datetime::DateTime;
use crate::util::zip::{self, ZipWriter};

/// The directory of the backups of a game directory.
pub const BACKUPS_DIRECTORY: &str = "backups";

/// The number of the backups of the same moment the launcher tries before it
/// gives up.
///
/// The backups of a moment are numbered, so a player who makes many backups in
/// a second still gets them all; the limit keeps a broken clock from looping.
const MAX_ATTEMPTS: u32 = 256;

/// A backup of a world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    /// The archive of the backup.
    file: PathBuf,
    /// The moment the backup was made.
    time: DateTime,
    /// The number of the backups that were made at the same moment before this
    /// one.
    count: u32,
    /// The name of the world the backup holds.
    world_name: String,
    /// The version of the game the world was played with.
    game_version: Option<String>,
}

impl Backup {
    /// Reads a backup of a world.
    ///
    /// The name of the archive has to be one the launcher writes, so a file of
    /// another kind in the directory of the backups is not a backup of the
    /// world.
    pub fn open(path: impl AsRef<Path>, world_name: &str) -> Option<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            return None;
        }

        let file_name = path.file_name()?.to_str()?;
        let (time, count) = parse_name(file_name, world_name)?;

        // The level of the backup tells the name and the version of the world
        // it holds, which is what the launcher shows in the list.
        let (backup_world_name, game_version) = match world_archive_root(path) {
            Some(root) => {
                let level = read_archive_level(path, &root)?;
                let world_name = level
                    .get("Data")
                    .and_then(|data| data.get("LevelName"))
                    .and_then(|name| name.as_str())
                    .unwrap_or_default()
                    .to_string();
                let version = level
                    .get("Data")
                    .and_then(|data| data.get("Version"))
                    .and_then(|version| version.get("Name"))
                    .and_then(|name| name.as_str())
                    .map(str::to_string);
                (world_name, version)
            }
            None => (String::new(), None),
        };

        Some(Self {
            file: path.to_path_buf(),
            time,
            count,
            world_name: backup_world_name,
            game_version,
        })
    }

    /// Returns the archive of the backup.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the moment the backup was made.
    pub fn time(&self) -> DateTime {
        self.time
    }

    /// Returns the number of the backups that were made at the same moment
    /// before this one.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Returns the name of the world the backup holds, as the level records it.
    pub fn world_name(&self) -> &str {
        &self.world_name
    }

    /// Returns the version of the game the world was played with.
    pub fn game_version(&self) -> Option<&str> {
        self.game_version.as_deref()
    }

    /// Deletes the archive of the backup.
    pub fn delete(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.file) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

impl PartialOrd for Backup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Backup {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time
            .cmp(&other.time)
            .then_with(|| self.count.cmp(&other.count))
    }
}

/// Returns the backups of a world, oldest first.
///
/// The files of the other worlds and the files the launcher did not write are
/// left out.
pub fn list_backups(directory: &Path, world_name: &str) -> Vec<Backup> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut backups = entries
        .flatten()
        .filter_map(|entry| Backup::open(entry.path(), world_name))
        .collect::<Vec<_>>();

    backups.sort();
    backups
}

/// Makes a backup of a world.
///
/// The archive is written into the directory of the backups, which is made when
/// it is missing. The lock file of the world is left out, because the backup is
/// not the world the game runs.
pub fn backup(world: &World, directory: &Path) -> Result<PathBuf, BackupError> {
    if !world.file().is_dir() {
        return Err(BackupError::NotAWorld {
            path: world.file().to_path_buf(),
        });
    }

    // The backups of a world that is being played are not consistent, and the
    // game holds the lock of the world for its whole run.
    if world.is_locked() {
        return Err(BackupError::WorldLocked {
            path: world.file().to_path_buf(),
        });
    }

    std::fs::create_dir_all(directory).map_err(|error| BackupError::Write {
        path: directory.to_path_buf(),
        error,
    })?;

    let time = DateTime::now();
    let name = world.file_name();
    let base = format!("{}_{}", time.file_name().replace('T', "_"), name);

    for count in 0..MAX_ATTEMPTS {
        let suffix = if count == 0 {
            String::new()
        } else {
            format!(" {count}")
        };
        let file = directory.join(format!("{base}{suffix}.zip"));

        // The backup is not written over another one, so a file that is taken
        // moves the launcher to the next number.
        if file.exists() {
            continue;
        }

        write_archive(world, &file, name)
            .map_err(|error| BackupError::Archive { path: file, error })?;
        return Ok(directory.join(format!("{base}{suffix}.zip")));
    }

    Err(BackupError::TooManyAttempts {
        directory: directory.to_path_buf(),
    })
}

/// Writes the files of a world into the archive of a backup.
fn write_archive(world: &World, file: &Path, name: &str) -> Result<(), zip::ZipError> {
    let mut writer = ZipWriter::create(file)?;
    writer.add_directory_filtered(name, world.file(), &|path: &Path| {
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| file_name == "session.lock")
    })?;
    writer.finish()
}

/// Returns the name of the world of an archive of a backup.
///
/// The archive of a backup holds the level of the world either at its root or in
/// one directory of its own, so the launcher reads the name the level records.
fn world_archive_root(path: &Path) -> Option<String> {
    let entries = zip::entries(path).ok()?;
    if entries.iter().any(|entry| entry.name == "level.dat") {
        return Some(String::new());
    }

    let root = entries
        .iter()
        .filter_map(|entry| entry.name.split('/').next())
        .find(|name| !name.is_empty())?;

    Some(root.to_string())
}

/// Reads the level of a world archive, under the directory it is in.
fn read_archive_level(path: &Path, root: &str) -> Option<crate::util::nbt::Tag> {
    let name = if root.is_empty() {
        "level.dat".to_string()
    } else {
        format!("{root}/level.dat")
    };

    let data = zip::read_entry(path, &name).ok()??;
    let decompressed = crate::util::compression::decompress_gzip(&data).ok()?;
    crate::util::nbt::Tag::read(&mut decompressed.as_slice()).ok()
}

/// Reads the moment and the number of a backup of a world from the name of its
/// archive.
///
/// The launcher writes the name as `<moment>_<world>[ <count>].zip`, where the
/// moment is written as `yyyy-MM-dd_HH-mm-ss` and the count is left out for the
/// first backup of the moment.
fn parse_name(file_name: &str, world_name: &str) -> Option<(DateTime, u32)> {
    let rest = file_name.strip_suffix(".zip")?;

    // The moment is nineteen characters long and is followed by an underscore.
    let moment = rest.get(..19)?;
    let rest = rest.get(19..)?.strip_prefix('_')?;
    let count = rest.strip_prefix(world_name)?;

    let time = parse_time(moment)?;

    if count.is_empty() {
        return Some((time, 0));
    }

    let count = count.strip_prefix(' ')?.parse::<u32>().ok()?;
    Some((time, count))
}

/// Reads the moment of the name of a backup, which is written as
/// `yyyy-MM-dd_HH-mm-ss`.
fn parse_time(text: &str) -> Option<DateTime> {
    let bytes = text.as_bytes();
    if bytes.len() != 19 {
        return None;
    }

    let number = |start: usize, end: usize| text.get(start..end)?.parse::<u32>().ok();

    Some(DateTime::new(
        number(0, 4)? as i32,
        number(5, 7)?,
        number(8, 10)?,
        number(11, 13)?,
        number(14, 16)?,
        number(17, 19)?,
    ))
}

/// The error of making a backup of a world.
#[derive(Debug)]
pub enum BackupError {
    /// The path is not a world directory.
    NotAWorld {
        /// The path that is not a world.
        path: PathBuf,
    },
    /// The world is locked by another process.
    WorldLocked {
        /// The directory of the locked world.
        path: PathBuf,
    },
    /// The directory of the backups cannot be made.
    Write {
        /// The directory of the backups.
        path: PathBuf,
        /// The reason the directory cannot be made.
        error: std::io::Error,
    },
    /// The archive of the backup cannot be written.
    Archive {
        /// The archive of the backup.
        path: PathBuf,
        /// The reason the archive cannot be written.
        error: zip::ZipError,
    },
    /// The backups of the same moment are all taken.
    TooManyAttempts {
        /// The directory of the backups.
        directory: PathBuf,
    },
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAWorld { path } => {
                write!(formatter, "{} is not a world directory", path.display())
            }
            Self::WorldLocked { path } => {
                write!(formatter, "the world {} is locked", path.display())
            }
            Self::Write { path, error } => write!(
                formatter,
                "the directory {} of the backups cannot be made: {error}",
                path.display()
            ),
            Self::Archive { path, error } => write!(
                formatter,
                "the backup {} cannot be written: {error}",
                path.display()
            ),
            Self::TooManyAttempts { directory } => write!(
                formatter,
                "the backups of the same moment in {} are all taken",
                directory.display()
            ),
        }
    }
}

impl std::error::Error for BackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write { error, .. } => Some(error),
            Self::Archive { error, .. } => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::nbt::{write_compressed, Compound, Tag};

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-backup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    /// Writes a world directory the tests work in.
    fn world(directory: &Path, name: &str) {
        std::fs::create_dir_all(directory).expect("the directory should be made");

        let mut data = Compound::new();
        data.insert("LevelName".to_string(), Tag::String(name.to_string()));
        data.insert("LastPlayed".to_string(), Tag::Long(0));

        let mut version = Compound::new();
        version.insert("Name".to_string(), Tag::String("1.20.1".to_string()));
        data.insert("Version".to_string(), Tag::Compound(version));

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));

        write_compressed(&directory.join("level.dat"), &Tag::Compound(level))
            .expect("the level should be written");
    }

    #[test]
    fn a_backup_of_the_world_is_made() {
        let base = directory("make");
        world(&base.join("saves/My World"), "My World");
        std::fs::write(base.join("saves/My World/session.lock"), b"snowman").expect("the lock");

        let world = World::open(base.join("saves/My World")).expect("the world should be read");
        let backups = base.join("backups");
        let file = backup(&world, &backups).expect("the world should be backed up");

        assert!(file.is_file());
        // The name holds the moment, the name of the world and the extension.
        let name = file.file_name().and_then(|name| name.to_str()).unwrap();
        assert!(name.ends_with("_My World.zip"), "{name}");

        // The archive holds the level and leaves out the lock.
        let entries = zip::entries(&file).expect("the archive should be read");
        assert!(entries
            .iter()
            .any(|entry| entry.name == "My World/level.dat"));
        assert!(!entries
            .iter()
            .any(|entry| entry.name.ends_with("session.lock")));

        let backups = list_backups(&backups, "My World");
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].world_name(), "My World");
        assert_eq!(backups[0].game_version(), Some("1.20.1"));
        assert_eq!(backups[0].file(), file);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_backups_of_the_same_moment_are_numbered() {
        let base = directory("numbered");
        world(&base.join("saves/world"), "world");

        let world = World::open(base.join("saves/world")).expect("the world should be read");
        let backups = base.join("backups");

        // Two backups made in a row may share a second, in which case the
        // launcher numbers the second one instead of writing over the first.
        let first = backup(&world, &backups).expect("the world should be backed up");
        let second = backup(&world, &backups).expect("the world should be backed up");
        assert_ne!(first, second);
        assert!(first.is_file());
        assert!(second.is_file());

        let listed = list_backups(&backups, "world");
        assert_eq!(listed.len(), 2);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_backup_is_installed_as_a_world() {
        let base = directory("round-trip");
        world(&base.join("saves/world"), "world");

        let world = World::open(base.join("saves/world")).expect("the world should be read");
        let file = backup(&world, &base.join("backups")).expect("the world should be backed up");

        // The backup is the world again, so restoring it gives the same world.
        let saves = base.join("restored");
        World::install(&file, &saves, "Restored").expect("the backup should be installed");
        let restored = World::open(saves.join("Restored")).expect("the world should be read");
        assert_eq!(restored.world_name(), "Restored");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_path_that_is_not_a_world_is_not_backed_up() {
        let base = directory("not-a-world");
        std::fs::create_dir_all(base.join("empty")).expect("the directory should be made");

        // A directory without a level is not a world, so the launcher refuses
        // to back it up instead of writing an archive the game cannot read.
        let error = World::open(base.join("empty")).expect_err("the world should be refused");
        assert!(matches!(
            error,
            crate::game::world::WorldError::NotAWorld { .. }
        ));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_backups_of_another_world_are_left_out() {
        let base = directory("other-world");
        world(&base.join("saves/world"), "world");
        world(&base.join("saves/other"), "other");

        let world = World::open(base.join("saves/world")).expect("the world should be read");
        let other = World::open(base.join("saves/other")).expect("the world should be read");
        let backups = base.join("backups");

        let file = backup(&world, &backups).expect("the world should be backed up");
        backup(&other, &backups).expect("the world should be backed up");
        // A file the launcher did not write is not a backup.
        std::fs::write(backups.join("notes.txt"), b"text").expect("the file should be written");

        let listed = list_backups(&backups, "world");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].file(), file);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_backup_is_deleted() {
        let base = directory("delete");
        world(&base.join("saves/world"), "world");

        let world = World::open(base.join("saves/world")).expect("the world should be read");
        let backups = base.join("backups");
        let file = backup(&world, &backups).expect("the world should be backed up");

        let backup = Backup::open(&file, "world").expect("the backup should be read");
        backup.delete().expect("the backup should be deleted");
        assert!(!file.exists());

        // Deleting a backup that is gone is not a failure.
        backup.delete().expect("the backup should be deleted");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_names_of_the_backups_are_read() {
        let (time, count) =
            parse_name("2026-01-02_03-04-05_My World.zip", "My World").expect("the name");
        assert_eq!(time, DateTime::new(2026, 1, 2, 3, 4, 5));
        assert_eq!(count, 0);

        let (time, count) =
            parse_name("2026-01-02_03-04-05_My World 3.zip", "My World").expect("the name");
        assert_eq!(time, DateTime::new(2026, 1, 2, 3, 4, 5));
        assert_eq!(count, 3);

        assert!(parse_name("2026-01-02_03-04-05_Other.zip", "My World").is_none());
        assert!(parse_name("notes.txt", "My World").is_none());
        assert!(parse_name("2026-01-02_03-04-05_My World.rar", "My World").is_none());
    }
}
