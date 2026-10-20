//! Launcher metadata and the directories the launcher reads and writes.

use std::path::PathBuf;

/// The title of the launcher, used in the window title and in the logs.
pub const TITLE: &str = "Hello Minecraft! Launcher";

/// The short title of the launcher.
pub const SHORT_TITLE: &str = "HMCL";

/// The version of the launcher, taken from the crate manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The AppUserModelID that groups the windows of the launcher on the Windows
/// taskbar.
pub const WINDOWS_APP_USER_MODEL_ID: &str = "org.jackhuang.hmcl";

/// The update channel the launcher follows unless the user selects another.
pub const DEFAULT_UPDATE_CHANNEL: &str = "stable";

/// Returns the directory the launcher was started from.
///
/// The current directory is where relative paths of the command line are
/// resolved, so it is captured once instead of queried again later.
pub fn current_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Returns the per-user home directory of the launcher (`~/.hmcl`).
///
/// `HMCL_USER_HOME` overrides the location; the user profile is used when the
/// variable is absent or the platform does not report one.
pub fn user_home() -> PathBuf {
    if let Some(path) = std::env::var_os("HMCL_USER_HOME") {
        return PathBuf::from(path);
    }

    home_directory()
        .map(|home| home.join(".hmcl"))
        .unwrap_or_else(|| PathBuf::from(".hmcl"))
}

/// Returns the local data directory of the launcher, where caches and
/// downloaded libraries are kept.
///
/// `HMCL_LOCAL_HOME` overrides the location; otherwise the platform specific
/// application data directory is used.
pub fn local_home() -> PathBuf {
    if let Some(path) = std::env::var_os("HMCL_LOCAL_HOME") {
        return PathBuf::from(path);
    }

    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path).join("hmcl");
    }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("hmcl");
    }

    home_directory()
        .map(|home| home.join(".local").join("share").join("hmcl"))
        .unwrap_or_else(|| PathBuf::from("hmcl"))
}

/// Returns the home directory of the current user, if the platform reports
/// one.
fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }
}
