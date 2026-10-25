//! The failures the core of the launcher reports.
//!
//! The errors carry the path of the file they are about, when there is one,
//! so the launcher can tell the user which file is broken.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// The failure of an operation of the launcher.
#[derive(Debug)]
pub enum Error {
    /// A file of the file system could not be read or written.
    Io {
        /// The path of the file, if the operation named one.
        path: Option<PathBuf>,
        /// The failure the operating system reported.
        source: io::Error,
    },
    /// A file does not have the content the launcher expected.
    InvalidFormat {
        /// The path of the file, if the operation named one.
        path: Option<PathBuf>,
        /// The reason the file was rejected.
        reason: String,
    },
    /// A version number could not be parsed.
    InvalidVersion {
        /// The text that could not be parsed.
        version: String,
    },
    /// The launcher does not support the requested feature.
    Unsupported {
        /// The name of the feature.
        feature: String,
    },
}

impl Error {
    /// Creates an error that reports a failure of the file system.
    pub fn io(path: impl AsRef<Path>, source: io::Error) -> Self {
        Self::Io {
            path: Some(path.as_ref().to_path_buf()),
            source,
        }
    }

    /// Creates an error that reports a failure of the file system without a
    /// path.
    pub fn io_without_path(source: io::Error) -> Self {
        Self::Io { path: None, source }
    }

    /// Creates an error that reports a file with unexpected content.
    pub fn invalid_format(path: impl AsRef<Path>, reason: impl Into<String>) -> Self {
        Self::InvalidFormat {
            path: Some(path.as_ref().to_path_buf()),
            reason: reason.into(),
        }
    }

    /// Creates an error that reports a version number that could not be
    /// parsed.
    pub fn invalid_version(version: impl Into<String>) -> Self {
        Self::InvalidVersion {
            version: version.into(),
        }
    }

    /// Creates an error that reports a feature the launcher does not support.
    pub fn unsupported(feature: impl Into<String>) -> Self {
        Self::Unsupported {
            feature: feature.into(),
        }
    }

    /// Returns the path of the file the error is about, if there is one.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io { path, .. } | Self::InvalidFormat { path, .. } => path.as_deref(),
            Self::InvalidVersion { .. } | Self::Unsupported { .. } => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => match path {
                Some(path) => write!(formatter, "{}: {source}", path.display()),
                None => write!(formatter, "{source}"),
            },
            Self::InvalidFormat { path, reason } => match path {
                Some(path) => write!(formatter, "{}: {reason}", path.display()),
                None => formatter.write_str(reason),
            },
            Self::InvalidVersion { version } => {
                write!(formatter, "The version number '{version}' is invalid")
            }
            Self::Unsupported { feature } => {
                write!(formatter, "The feature '{feature}' is not supported")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidFormat { .. } | Self::InvalidVersion { .. } | Self::Unsupported { .. } => {
                None
            }
        }
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::io_without_path(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_errors_describe_themselves() {
        let error = Error::invalid_version("1.2.3.4.5");
        assert!(error.to_string().contains("1.2.3.4.5"));
        assert_eq!(error.path(), None);

        let error = Error::unsupported("the theme packs");
        assert!(error.to_string().contains("the theme packs"));
    }

    #[test]
    fn the_errors_keep_the_path_of_the_file() {
        let path = Path::new("config").join("launcher-settings.json");
        let error = Error::invalid_format(&path, "the file is not JSON");
        assert_eq!(error.path(), Some(path.as_path()));
        assert!(error.to_string().contains("launcher-settings.json"));
    }

    #[test]
    fn the_errors_of_the_file_system_keep_their_cause() {
        let cause = io::Error::new(io::ErrorKind::NotFound, "not found");
        let error = Error::io("config", cause);
        assert!(std::error::Error::source(&error).is_some());
    }
}
