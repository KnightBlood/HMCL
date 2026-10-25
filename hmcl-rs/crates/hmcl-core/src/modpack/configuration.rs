//! The configuration the launcher keeps of the modpack it installed.
//!
//! An instance does not remember the archive it was installed from: the
//! launcher writes the manifest of the modpack next to the instance, together
//! with the digests of the files the archive unpacked. The next time the
//! launcher updates or completes the instance it reads the configuration to
//! know which files the modpack owns, so a file the player changed is not
//! replaced and a file the player added is not deleted.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// A file the launcher unpacked from a modpack archive.
///
/// The path is relative to the run directory of the instance and is written
/// with `/` as the separator, whichever system the launcher runs on. The
/// digest is the SHA-1 digest of the file as the archive held it, so the
/// launcher can tell whether the player changed the file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileInformation {
    /// The path of the file relative to the run directory of the instance.
    pub path: String,
    /// The SHA-1 digest of the file, if the launcher knows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// The address the file is downloaded from, if the archive did not hold it.
    #[serde(
        default,
        rename = "downloadURL",
        skip_serializing_if = "Option::is_none"
    )]
    pub download_url: Option<String>,
}

impl FileInformation {
    /// Creates the information of a file the launcher knows the digest of.
    pub fn new(path: impl Into<String>, hash: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            hash: Some(hash.into()),
            download_url: None,
        }
    }

    /// Returns the information of a file the launcher downloads from an
    /// address instead of taking it from the archive.
    pub fn downloaded(
        path: impl Into<String>,
        hash: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            hash: Some(hash.into()),
            download_url: Some(url.into()),
        }
    }
}

/// The manifest of a modpack and the files the launcher unpacked from it.
///
/// The kind is the name of the provider that wrote the manifest (`Curse`,
/// `Modrinth`, `Mcbbs`, `Server` or `MultiMC`). The launcher reads the kind
/// before it updates an instance, so an instance of one format is not updated
/// with the archive of another one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModpackConfiguration<M> {
    /// The manifest of the modpack.
    pub manifest: M,
    /// The name of the provider that wrote the manifest.
    #[serde(rename = "type")]
    pub kind: String,
    /// The name of the modpack.
    #[serde(default)]
    pub name: String,
    /// The version of the modpack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The files the archive of the modpack unpacked.
    #[serde(default)]
    pub overrides: Vec<FileInformation>,
}

impl<M> ModpackConfiguration<M> {
    /// Creates the configuration of a modpack.
    pub fn new(
        manifest: M,
        kind: impl Into<String>,
        name: impl Into<String>,
        version: Option<String>,
        overrides: Vec<FileInformation>,
    ) -> Self {
        Self {
            manifest,
            kind: kind.into(),
            name: name.into(),
            version,
            overrides,
        }
    }

    /// Returns the paths of the files the archive unpacked.
    pub fn override_paths(&self) -> impl Iterator<Item = &str> {
        self.overrides.iter().map(|file| file.path.as_str())
    }
}

impl<M: serde::de::DeserializeOwned> ModpackConfiguration<M> {
    /// Reads the configuration of an instance.
    ///
    /// A file that is not there is reported as [`ConfigurationError::NotFound`],
    /// so the caller can tell an instance that is not a modpack from a
    /// configuration the launcher cannot read.
    pub fn read(path: &Path) -> Result<Self, ConfigurationError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ConfigurationError::NotFound {
                    path: path.to_path_buf(),
                })
            }
            Err(error) => {
                return Err(ConfigurationError::Read {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })
            }
        };

        let configuration: Self =
            serde_json::from_str(&text).map_err(|error| ConfigurationError::Malformed {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        configuration.validate()?;
        Ok(configuration)
    }
}

impl<M: Serialize> ModpackConfiguration<M> {
    /// Writes the configuration of an instance.
    ///
    /// The directories of the file are created, so an instance that has not
    /// been written yet can be configured.
    pub fn write(&self, path: &Path) -> Result<(), ConfigurationError> {
        self.validate_kind()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| ConfigurationError::Write {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        }

        let text =
            serde_json::to_string_pretty(self).map_err(|error| ConfigurationError::Malformed {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        std::fs::write(path, text).map_err(|error| ConfigurationError::Write {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    }
}

impl<M> ModpackConfiguration<M> {
    /// Returns whether the configuration names the provider that wrote it.
    ///
    /// The launcher refuses to update an instance whose configuration names
    /// another provider, because the files of the two formats are not the
    /// same.
    pub fn is_of(&self, kind: &str) -> bool {
        self.kind == kind
    }

    /// Refuses a configuration the launcher cannot trust.
    ///
    /// The manifest and the kind are what the update needs; the name and the
    /// version are informational and may be missing.
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        self.validate_kind()
    }

    /// Refuses a configuration that names no provider.
    fn validate_kind(&self) -> Result<(), ConfigurationError> {
        if self.kind.trim().is_empty() {
            Err(ConfigurationError::MissingKind)
        } else {
            Ok(())
        }
    }
}

/// The configuration of a modpack the launcher cannot read.
#[derive(Debug)]
pub enum ConfigurationError {
    /// The instance is not a modpack, because the launcher wrote no
    /// configuration for it.
    NotFound {
        /// The file the launcher looked for.
        path: std::path::PathBuf,
    },
    /// The file cannot be read.
    Read {
        /// The file the launcher read.
        path: std::path::PathBuf,
        /// The reason the file cannot be read.
        message: String,
    },
    /// The file cannot be written.
    Write {
        /// The file the launcher wrote.
        path: std::path::PathBuf,
        /// The reason the file cannot be written.
        message: String,
    },
    /// The file is not a configuration of a modpack.
    Malformed {
        /// The file the launcher read.
        path: std::path::PathBuf,
        /// The reason the file was refused.
        message: String,
    },
    /// The configuration names no provider, so the launcher cannot update the
    /// instance.
    MissingKind,
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { path } => write!(
                formatter,
                "{} is not a modpack instance: the launcher wrote no configuration for it",
                path.display()
            ),
            Self::Read { path, message } => {
                write!(formatter, "cannot read {}: {message}", path.display())
            }
            Self::Write { path, message } => {
                write!(formatter, "cannot write {}: {message}", path.display())
            }
            Self::Malformed { path, message } => {
                write!(
                    formatter,
                    "{} is not a configuration: {message}",
                    path.display()
                )
            }
            Self::MissingKind => formatter.write_str("the configuration names no modpack provider"),
        }
    }
}

impl std::error::Error for ConfigurationError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small manifest, which is enough to test the configuration.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Manifest {
        name: String,
    }

    /// Returns a directory of the tests that the other tests do not use.
    fn directory(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hmcl-rs-modpack-configuration-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn the_configuration_of_a_modpack_is_written_and_read() {
        let base = directory("written");
        let path = base.join("versions/world/modpack.json");

        let configuration = ModpackConfiguration::new(
            Manifest {
                name: "World".to_string(),
            },
            "Server",
            "World",
            Some("1.0".to_string()),
            vec![FileInformation::new("mods/a.jar", "abc")],
        );
        configuration
            .write(&path)
            .expect("the configuration should be written");

        let read: ModpackConfiguration<Manifest> =
            ModpackConfiguration::read(&path).expect("the configuration should be read");
        assert_eq!(read, configuration);
        assert!(read.is_of("Server"));
        assert!(!read.is_of("Mcbbs"));
        assert_eq!(
            read.override_paths().collect::<Vec<_>>(),
            vec!["mods/a.jar"]
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_missing_configuration_is_reported_separately() {
        let base = directory("missing");
        let error = ModpackConfiguration::<Manifest>::read(&base.join("modpack.json"))
            .expect_err("the configuration should be missing");
        assert!(matches!(error, ConfigurationError::NotFound { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_files_of_the_manifest_keep_their_download_address() {
        let text = r#"{
            "manifest": { "name": "World" },
            "type": "Mcbbs",
            "name": "World",
            "overrides": [
                { "path": "mods/a.jar", "hash": "abc", "downloadURL": "https://example.com/a.jar" }
            ]
        }"#;

        let configuration: ModpackConfiguration<Manifest> =
            serde_json::from_str(text).expect("the configuration should be read");
        let file = &configuration.overrides[0];
        assert_eq!(
            file.download_url.as_deref(),
            Some("https://example.com/a.jar")
        );

        let written =
            serde_json::to_string(&configuration).expect("the configuration should be written");
        assert!(written.contains("\"downloadURL\""));
        assert!(written.contains("\"type\":\"Mcbbs\""));
    }

    #[test]
    fn a_configuration_without_a_provider_is_refused() {
        let base = directory("kind");
        let path = base.join("modpack.json");
        std::fs::create_dir_all(&base).expect("the directory should be created");
        std::fs::write(
            &path,
            r#"{ "manifest": { "name": "World" }, "type": "", "name": "World" }"#,
        )
        .expect("the configuration should be written");

        let error = ModpackConfiguration::<Manifest>::read(&path)
            .expect_err("the configuration should be refused");
        assert!(matches!(error, ConfigurationError::MissingKind));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_broken_configuration_is_reported_as_malformed() {
        let base = directory("broken");
        let path = base.join("modpack.json");
        std::fs::create_dir_all(&base).expect("the directory should be created");
        std::fs::write(&path, "this is not json").expect("the configuration should be written");

        let error = ModpackConfiguration::<Manifest>::read(&path)
            .expect_err("the configuration should be refused");
        assert!(matches!(error, ConfigurationError::Malformed { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }
}
