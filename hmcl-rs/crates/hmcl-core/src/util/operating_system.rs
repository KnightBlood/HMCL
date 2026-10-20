//! Detection of the operating system the launcher runs on.

use std::fmt;

/// The operating systems the launcher distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsType {
    /// Microsoft Windows.
    Windows,
    /// Linux or another distribution that reports `linux`.
    Linux,
    /// Apple macOS.
    MacOs,
    /// An operating system the launcher was not built for.
    Unknown,
}

impl OsType {
    /// Returns the operating system the current binary was compiled for.
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Unknown
        }
    }

    /// Returns whether this is a Windows system.
    pub const fn is_windows(self) -> bool {
        matches!(self, Self::Windows)
    }

    /// Returns whether this is a Linux system.
    pub const fn is_linux(self) -> bool {
        matches!(self, Self::Linux)
    }

    /// Returns whether this is a macOS system.
    pub const fn is_macos(self) -> bool {
        matches!(self, Self::MacOs)
    }

    /// Returns whether this is a Unix-like system.
    pub const fn is_unix_like(self) -> bool {
        matches!(self, Self::Linux | Self::MacOs)
    }
}

impl fmt::Display for OsType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::MacOs => "macOS",
            Self::Unknown => "Unknown",
        };
        formatter.write_str(name)
    }
}

/// Returns the operating system of the running process.
pub const fn current_os() -> OsType {
    OsType::current()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_one_system_is_selected() {
        let selected = [
            current_os().is_windows(),
            current_os().is_linux(),
            current_os().is_macos(),
        ]
        .into_iter()
        .filter(|selected| *selected)
        .count();

        assert!(selected <= 1);
    }

    #[test]
    fn display_names_are_not_empty() {
        for os in [OsType::Windows, OsType::Linux, OsType::MacOs, OsType::Unknown] {
            assert!(!os.to_string().is_empty());
        }
    }
}
