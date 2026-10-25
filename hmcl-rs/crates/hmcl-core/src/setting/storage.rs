//! The storage of the settings of the launcher.
//!
//! The settings are kept in JSON files, like in the Java launcher, so the
//! files can be inspected and edited by hand. Every file starts with the
//! address of the JSON schema that describes it.

use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::Error;

use super::schema::{check, Compatibility, Schema, SchemaSettings};

/// The name of the file that holds the settings of the launcher.
pub const LAUNCHER_SETTINGS_FILE: &str = "launcher-settings.json";

/// The name of the file that holds the settings of the user.
pub const USER_SETTINGS_FILE: &str = "user-settings.json";

/// The name of the file that holds the state of the launcher.
pub const LAUNCHER_STATE_FILE: &str = "launcher-state.json";

/// A file that holds a part of the settings of the launcher.
///
/// The settings are read when the file is loaded and written when they are
/// saved; the file is written to a temporary file first, so an interrupted
/// save does not destroy the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsFile<T> {
    /// The path of the file.
    path: PathBuf,
    /// The settings the file holds.
    settings: T,
    /// Whether the file may be written again.
    savable: bool,
    /// Whether the file on disk is moved to a backup before it is written.
    ///
    /// A file that could not be read is kept this way, so the bytes the
    /// launcher could not understand are not lost when the defaults replace
    /// them.
    backup_on_next_save: bool,
    /// The way the schema of the file was compared with the schema the
    /// launcher knows.
    compatibility: Option<Compatibility>,
}

impl<T> SettingsFile<T> {
    /// Creates a file that holds the settings, without reading it.
    pub fn new(path: impl Into<PathBuf>, settings: T) -> Self {
        Self {
            path: path.into(),
            settings,
            savable: true,
            backup_on_next_save: false,
            compatibility: None,
        }
    }

    /// Returns the path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the settings the file holds.
    pub fn settings(&self) -> &T {
        &self.settings
    }

    /// Returns the settings the file holds, so they can be changed.
    pub fn settings_mut(&mut self) -> &mut T {
        &mut self.settings
    }

    /// Takes the settings out of the file.
    pub fn into_settings(self) -> T {
        self.settings
    }

    /// Returns whether the file may be written again.
    ///
    /// A file of a newer launcher is only read, so the launcher does not
    /// destroy the settings it does not know.
    pub fn is_savable(&self) -> bool {
        self.savable
    }

    /// Returns whether the file on disk is moved to a backup before the next
    /// save.
    pub fn is_backup_on_next_save(&self) -> bool {
        self.backup_on_next_save
    }

    /// Selects whether the file on disk is moved to a backup before the next
    /// save.
    ///
    /// The launcher sets this when it replaces a file it could not read, so
    /// the bytes of that file are kept.
    pub fn set_backup_on_next_save(&mut self, backup: bool) {
        self.backup_on_next_save = backup;
    }

    /// Returns the way the schema of the file was compared with the schema
    /// the launcher knows, if the file was read with a schema.
    pub fn compatibility(&self) -> Option<&Compatibility> {
        self.compatibility.as_ref()
    }
}

impl<T: Serialize + DeserializeOwned + Default> SettingsFile<T> {
    /// Reads the settings from the file.
    ///
    /// A file that does not exist yet yields the default settings, so the
    /// launcher starts with the settings of a fresh installation.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let path = path.into();

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::new(path, T::default()));
            }
            Err(error) => return Err(Error::io(&path, error)),
        };

        let settings = serde_json::from_str(&content)
            .map_err(|error| Error::invalid_format(&path, error.to_string()))?;

        Ok(Self::new(path, settings))
    }

    /// Reads the settings from the file, keeping a broken file as a backup.
    ///
    /// A file that cannot be read is kept as a numbered backup when the
    /// launcher writes the file next, so the user does not lose it, and the
    /// default settings are used instead.
    pub fn load_with_recovery(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let path = path.into();

        match Self::load(&path) {
            Ok(file) => Ok(file),
            Err(error) => {
                if error.path() != Some(path.as_path()) {
                    return Err(error);
                }

                // The file is not moved now: the launcher may read it again
                // before it writes anything, and the backup belongs to the
                // moment the file is replaced.
                let mut file = Self::new(path, T::default());
                file.backup_on_next_save = true;
                Ok(file)
            }
        }
    }

    /// Writes the settings to the file.
    ///
    /// The settings are written to a temporary file first, which then
    /// replaces the file, so an interrupted write does not destroy the
    /// settings.
    ///
    /// A file that holds the settings of a newer launcher is not written
    /// again, so the settings the launcher does not know are kept.
    ///
    /// A file the launcher could not read is moved to a numbered backup
    /// before it is replaced, so the bytes the launcher could not understand
    /// are kept for inspection.
    pub fn save(&self) -> Result<(), Error> {
        if !self.savable {
            return Err(Error::invalid_format(
                &self.path,
                "the file holds the settings of a newer launcher",
            ));
        }

        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;

        let content = serde_json::to_string_pretty(&self.settings)
            .map_err(|error| Error::invalid_format(&self.path, error.to_string()))?;

        // The file that could not be read is kept before the defaults replace
        // it; a file that was written by this launcher is replaced as it is.
        if self.backup_on_next_save && self.path.exists() {
            backup_invalid_file(&self.path)?;
        }

        let temporary = temporary_path(&self.path);
        fs::write(&temporary, content.as_bytes()).map_err(|error| Error::io(&temporary, error))?;

        match fs::rename(&temporary, &self.path) {
            Ok(()) => Ok(()),
            Err(error) => {
                // The temporary file is of no use once the write failed.
                let _ = fs::remove_file(&temporary);
                Err(Error::io(&self.path, error))
            }
        }
    }
}

/// Returns the path of the backup of a settings file.
///
/// The backup is numbered, like the backup the launcher of Java writes:
/// `settings.json.1`, `settings.json.2` and so on, so a file that is broken
/// more than once does not overwrite the first backup.
fn backup_path(path: &Path) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    let mut index = 1u32;
    loop {
        let candidate = parent.join(format!("{name}.{index}"));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

/// Moves a settings file that cannot be read to a numbered backup.
///
/// The bytes of the file are kept so the file can be inspected, and the
/// launcher writes the default settings in its place on the next save.
pub fn backup_invalid_file(path: &Path) -> Result<PathBuf, Error> {
    let backup = backup_path(path);
    fs::rename(path, &backup).map_err(|error| Error::io(&backup, error))?;
    Ok(backup)
}

/// Returns the path of the temporary file a settings file is written to.
fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

impl<T: SchemaSettings + Serialize + DeserializeOwned + Default> SettingsFile<T> {
    /// Reads the settings from the file, after the schema of the file was
    /// compared with the schema the launcher knows.
    ///
    /// A file that holds the settings of a newer launcher is read, but it is
    /// not written again, so the settings the launcher does not know are
    /// kept. A file of another kind of settings, or a file that cannot be
    /// read at all, yields the default settings that are not written again,
    /// so the launcher still starts; a file whose text cannot be parsed is
    /// replaced by the default settings after it is moved to a backup.
    pub fn load_with_schema(path: impl Into<PathBuf>, expected: &Schema) -> Result<Self, Error> {
        let path = path.into();

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut file = Self::new(path, T::default());
                file.settings.set_schema(expected.url());
                return Ok(file);
            }
            Err(error) => return Err(Error::io(&path, error)),
        };

        let defaults = || {
            let mut file = Self::new(path.clone(), T::default());
            file.settings.set_schema(expected.url());
            // The launcher cannot read the file, so it does not write over
            // it; the player keeps the file and the launcher keeps running.
            file.savable = false;
            file
        };

        let value: serde_json::Value = match serde_json::from_str(&content) {
            Ok(value) => value,
            // The text cannot be parsed, so the file is moved to a backup
            // when the launcher writes the default settings.
            Err(_) => {
                let mut file = defaults();
                file.savable = true;
                file.backup_on_next_save = true;
                return Ok(file);
            }
        };

        let compatibility = check(&value, expected);
        if !compatibility.readable() {
            return Ok(defaults());
        }

        let Ok(mut settings) = serde_json::from_value::<T>(value) else {
            return Ok(defaults());
        };

        if !compatibility.preserve_schema() {
            // The file holds an older schema, so the address of the schema
            // the launcher knows is written instead.
            settings.set_schema(expected.url());
        }

        Ok(Self {
            path,
            settings,
            savable: compatibility.allow_save(),
            backup_on_next_save: false,
            compatibility: Some(compatibility),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setting::schema::{CompatibilityStatus, SchemaVersion};
    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Sample {
        name: String,
        count: u32,
    }

    impl Default for Sample {
        fn default() -> Self {
            Self {
                name: "default".to_string(),
                count: 0,
            }
        }
    }

    /// Returns a directory that is removed when the test ends.
    fn temporary_directory(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("hmcl-core-test-{name}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the test directory should be created");
        path
    }

    #[test]
    fn a_missing_file_yields_the_default_settings() {
        let directory = temporary_directory("missing");
        let path = directory.join("settings.json");

        let file = SettingsFile::<Sample>::load(&path).expect("the settings should be loaded");
        assert_eq!(file.settings(), &Sample::default());
        assert_eq!(file.path(), path.as_path());
    }

    #[test]
    fn the_settings_are_written_and_read_back() {
        let directory = temporary_directory("round-trip");
        let path = directory.join("settings.json");

        let file = SettingsFile::new(
            &path,
            Sample {
                name: "launcher".to_string(),
                count: 3,
            },
        );
        file.save().expect("the settings should be saved");

        let loaded = SettingsFile::<Sample>::load(&path).expect("the settings should be loaded");
        assert_eq!(
            loaded.settings(),
            &Sample {
                name: "launcher".to_string(),
                count: 3,
            }
        );
    }

    #[test]
    fn a_broken_file_is_kept_as_a_backup() {
        let directory = temporary_directory("broken");
        let path = directory.join("settings.json");
        fs::write(&path, "{ this is not JSON").expect("the file should be written");

        let file = SettingsFile::<Sample>::load_with_recovery(&path)
            .expect("the settings should be loaded");
        assert_eq!(file.settings(), &Sample::default());
        // The file is kept when the defaults replace it, not before.
        assert!(file.is_backup_on_next_save());
        assert!(!backup_path(&path).exists());

        file.save().expect("the settings should be saved");
        let backup = directory.join("settings.json.1");
        assert!(backup.exists());
        assert_eq!(
            fs::read_to_string(&backup).expect("the backup should be read"),
            "{ this is not JSON"
        );
    }

    #[test]
    fn the_backups_of_a_file_are_numbered() {
        let directory = temporary_directory("numbered-backups");
        let path = directory.join("settings.json");

        for _ in 0..3 {
            fs::write(&path, "not json").expect("the file should be written");
            let file = SettingsFile::<Sample>::load_with_recovery(&path)
                .expect("the settings should be loaded");
            file.save().expect("the settings should be saved");
        }

        // Every backup keeps the bytes of the file it replaced, so a file
        // that is broken more than once does not overwrite the first backup.
        for index in 1..=3 {
            let backup = directory.join(format!("settings.json.{index}"));
            assert!(backup.exists(), "{backup:?} should exist");
        }
    }

    /// The settings that keep the address of their schema.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Versioned {
        #[serde(rename = "$schema")]
        schema: String,
        name: String,
    }

    impl Default for Versioned {
        fn default() -> Self {
            Self {
                schema: "https://schemas.glavo.site/hmcl/sample/1.0.0".to_string(),
                name: "default".to_string(),
            }
        }
    }

    impl crate::setting::schema::SchemaSettings for Versioned {
        fn schema(&self) -> &str {
            &self.schema
        }

        fn set_schema(&mut self, schema: &str) {
            self.schema = schema.to_string();
        }
    }

    /// The schema the launcher knows.
    fn sample_schema() -> Schema {
        Schema::new("sample", SchemaVersion::new(1, 0, 0))
    }

    /// The schema of a newer version of the launcher.
    fn newer_sample_schema() -> Schema {
        Schema::new("sample", SchemaVersion::new(1, 1, 0))
    }

    #[test]
    fn the_schema_of_the_file_is_checked() {
        let directory = temporary_directory("schema");
        let path = directory.join("settings.json");
        let schema = sample_schema();

        // A file of the same schema may be written again.
        fs::write(
            &path,
            "{\"$schema\":\"https://schemas.glavo.site/hmcl/sample/1.0.0\",\"name\":\"test\"}",
        )
        .expect("the file should be written");

        let file = SettingsFile::<Versioned>::load_with_schema(&path, &schema)
            .expect("the settings should be loaded");
        assert_eq!(file.settings().name, "test");
        assert!(file.is_savable());
        assert_eq!(
            file.compatibility()
                .map(|compatibility| compatibility.status),
            Some(CompatibilityStatus::ReadWritePreserveSchema)
        );

        // A file of a newer launcher is only read.
        fs::write(
            &path,
            "{\"$schema\":\"https://schemas.glavo.site/hmcl/sample/1.1.0\",\"name\":\"test\"}",
        )
        .expect("the file should be written");

        let file = SettingsFile::<Versioned>::load_with_schema(&path, &schema)
            .expect("the settings should be loaded");
        assert_eq!(file.settings().name, "test");
        assert!(!file.is_savable());
        assert!(file.save().is_err());

        // A file of another kind of settings is not read, but the launcher
        // still starts with the defaults, which it does not write over the
        // file.
        fs::write(
            &path,
            "{\"$schema\":\"https://schemas.glavo.site/hmcl/other/1.0.0\",\"name\":\"test\"}",
        )
        .expect("the file should be written");

        let file = SettingsFile::<Versioned>::load_with_schema(&path, &schema)
            .expect("the settings should be loaded");
        assert_eq!(file.settings().name, "default");
        assert!(!file.is_savable());
        assert!(file.save().is_err());
        // The file of the other kind is left as it is.
        assert!(fs::read_to_string(&path)
            .expect("the file should be read")
            .contains("other/1.0.0"));
    }

    #[test]
    fn the_address_of_an_older_schema_is_replaced() {
        let directory = temporary_directory("older-schema");
        let path = directory.join("settings.json");
        let schema = newer_sample_schema();

        fs::write(
            &path,
            "{\"$schema\":\"https://schemas.glavo.site/hmcl/sample/1.0.0\",\"name\":\"test\"}",
        )
        .expect("the file should be written");

        let file = SettingsFile::<Versioned>::load_with_schema(&path, &schema)
            .expect("the settings should be loaded");
        assert_eq!(
            file.settings().schema,
            "https://schemas.glavo.site/hmcl/sample/1.1.0"
        );
        assert!(file.is_savable());

        file.save().expect("the settings should be saved");
        let content = fs::read_to_string(&path).expect("the file should be read");
        assert!(content.contains("https://schemas.glavo.site/hmcl/sample/1.1.0"));
    }
}
