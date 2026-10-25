//! The paths the launcher writes into its settings files.
//!
//! A path is stored together with whether it is absolute, so a path that is
//! relative stays relative when the settings file is moved to another machine,
//! while a path the player wrote as an absolute one keeps the separators of the
//! system it was written on.
//!
//! The relative paths use `/` as their separator, like the launcher of Java
//! writes them, so a settings file written on Windows is read on Linux and the
//! other way around.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The separator of the relative portable paths.
pub const SEPARATOR: char = '/';

/// A path that is stored the way the settings of the launcher store it.
///
/// The path of a relative path is normalized to the separator [`SEPARATOR`],
/// while the path of an absolute one is kept as it was written, so the
/// separators of the system it names are preserved.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PortablePath {
    /// The path as it is stored in the settings file.
    path: String,
    /// Whether the path is absolute.
    absolute: bool,
}

impl PortablePath {
    /// Creates a portable path from its text.
    ///
    /// A relative path has its `\` separators replaced with [`SEPARATOR`]; an
    /// absolute path is kept as it is, so `C:\games` stays `C:\games`.
    pub fn of(path: impl Into<String>) -> Self {
        let path = path.into();
        let absolute = is_absolute(&path);
        Self {
            path: if absolute {
                path
            } else {
                path.replace('\\', &SEPARATOR.to_string())
            },
            absolute,
        }
    }

    /// Creates a portable path from a path of the file system.
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        Self::of(path.as_ref().to_string_lossy().into_owned())
    }

    /// Returns the path as it is stored in the settings file.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns whether the path is absolute.
    pub fn is_absolute(&self) -> bool {
        self.absolute
    }

    /// Returns whether the path is empty.
    pub fn is_empty(&self) -> bool {
        self.path.trim().is_empty()
    }

    /// Returns the path of the file system the portable path names.
    ///
    /// The path is read on the system the launcher runs on; a relative path is
    /// relative to the directory the launcher was started from.
    pub fn to_path(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }

    /// Returns the path of the file system, resolved against a directory.
    ///
    /// An absolute path is used as it is; a relative path is resolved against
    /// `base_directory`.
    pub fn resolve(&self, base_directory: &Path) -> PathBuf {
        if self.absolute {
            self.to_path()
        } else {
            base_directory.join(&self.path)
        }
    }
}

impl fmt::Display for PortablePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.path)
    }
}

impl From<String> for PortablePath {
    fn from(path: String) -> Self {
        Self::of(path)
    }
}

impl From<&str> for PortablePath {
    fn from(path: &str) -> Self {
        Self::of(path)
    }
}

/// Returns whether the text of a path names an absolute path.
///
/// The check is the one of the launcher of Java: a path that starts with a
/// separator, or that starts with a letter and a colon, is absolute. The check
/// does not ask the system, so a path written on Windows is still read as an
/// absolute one on Linux.
fn is_absolute(path: &str) -> bool {
    if path.starts_with('/') || path.starts_with('\\') {
        return true;
    }

    let mut characters = path.chars();
    match (characters.next(), characters.next()) {
        (Some(drive), Some(':')) => drive.is_ascii_alphabetic(),
        _ => false,
    }
}

impl Serialize for PortablePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.path)
    }
}

impl<'de> Deserialize<'de> for PortablePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Ok(Self::of(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_relative_paths_use_the_portable_separator() {
        let path = PortablePath::of("versions\\1.20.1");

        assert_eq!(path.path(), "versions/1.20.1");
        assert!(!path.is_absolute());
    }

    #[test]
    fn the_absolute_paths_keep_their_separators() {
        let windows = PortablePath::of("C:\\Users\\Name");
        let posix = PortablePath::of("/home/user");

        assert_eq!(windows.path(), "C:\\Users\\Name");
        assert!(windows.is_absolute());
        assert_eq!(posix.path(), "/home/user");
        assert!(posix.is_absolute());
    }

    #[test]
    fn a_path_is_written_and_read_as_a_string() {
        let path = PortablePath::of("game\\dir");
        let json = serde_json::to_string(&path).expect("the path should be written");

        assert_eq!(json, "\"game/dir\"");

        let read: PortablePath =
            serde_json::from_str("\"game/dir\"").expect("the path should be read");
        assert_eq!(read.path(), "game/dir");
        assert!(!read.is_absolute());

        // A path that is not a string is not a portable path.
        assert!(serde_json::from_str::<PortablePath>("42").is_err());
    }

    #[test]
    fn a_relative_path_is_resolved_against_a_directory() {
        let path = PortablePath::of("versions/1.20.1");

        assert_eq!(
            path.resolve(Path::new("/home/user/.hmcl")),
            PathBuf::from("/home/user/.hmcl/versions/1.20.1")
        );
        assert_eq!(path.to_path(), PathBuf::from("versions/1.20.1"));
    }

    #[test]
    fn an_absolute_path_is_not_resolved() {
        let path = PortablePath::of("/workspace/Dev");

        assert_eq!(
            path.resolve(Path::new("/home/user/.hmcl")),
            PathBuf::from("/workspace/Dev")
        );
    }

    #[test]
    fn a_path_of_the_file_system_keeps_its_absolute_flag() {
        let relative = PortablePath::from_path(Path::new("versions"));
        assert!(!relative.is_absolute());

        let absolute = std::env::current_dir().expect("the directory should be known");
        let portable = PortablePath::from_path(&absolute);
        assert!(portable.is_absolute());
    }
}
