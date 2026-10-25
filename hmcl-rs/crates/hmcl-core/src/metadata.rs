//! The metadata of the launcher and the directories it reads and writes.
//!
//! The directories are the ones of the Java launcher, so the two launchers
//! use the same files of the user.

use std::path::PathBuf;

use crate::util::operating_system::OsType;

/// The short name of the launcher.
pub const NAME: &str = "HMCL";

/// The full name of the launcher, used in the window title and in the logs.
pub const FULL_NAME: &str = "Hello Minecraft! Launcher";

/// The version of the launcher, taken from the crate manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The title of the window of the launcher.
pub const TITLE: &str = "HMCL";

/// The AppUserModelID that groups the windows of the launcher on the Windows
/// taskbar.
pub const WINDOWS_APP_USER_MODEL_ID: &str = "org.jackhuang.hmcl";

/// The update channel the launcher follows unless the user selects another.
pub const DEFAULT_UPDATE_CHANNEL: &str = "stable";

/// The name of the environment variable that holds the home of the launcher.
pub const USER_HOME_VARIABLE: &str = "HMCL_USER_HOME";

/// The name of the environment variable that holds the local home of the
/// launcher.
pub const LOCAL_HOME_VARIABLE: &str = "HMCL_LOCAL_HOME";

/// The name of the environment variable that holds the directory the launcher
/// keeps the dependencies it needs at run time in.
pub const DEPENDENCIES_DIRECTORY_VARIABLE: &str = "HMCL_DEPENDENCIES_DIR";

/// Returns the directory the launcher was started from.
///
/// The current directory is where relative paths of the command line are
/// resolved, so it is captured once instead of queried again later.
pub fn current_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Returns the home of the launcher, where the settings of the user are kept.
///
/// The home is the one of the Java launcher:
///
/// * the variable `HMCL_USER_HOME` when it is set,
/// * the directory of the data of the system on Linux,
/// * the directory of the application data of the user on Windows,
/// * the directory of the application support of the user on macOS.
pub fn user_home() -> PathBuf {
    if let Some(path) = std::env::var_os(USER_HOME_VARIABLE) {
        return absolute(PathBuf::from(path));
    }

    match OsType::current() {
        OsType::Linux => {
            if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
                return absolute(PathBuf::from(path).join("hmcl"));
            }
            home_directory()
                .map(|home| home.join(".local").join("share").join("hmcl"))
                .unwrap_or_else(|| PathBuf::from("hmcl"))
        }
        OsType::Windows => match std::env::var_os("APPDATA") {
            Some(path) => absolute(PathBuf::from(path).join(".hmcl")),
            None => home_directory()
                .map(|home| home.join(".hmcl"))
                .unwrap_or_else(|| PathBuf::from(".hmcl")),
        },
        OsType::MacOs => home_directory()
            .map(|home| {
                home.join("Library")
                    .join("Application Support")
                    .join("hmcl")
            })
            .unwrap_or_else(|| PathBuf::from("hmcl")),
        OsType::Unknown => home_directory()
            .map(|home| home.join("hmcl"))
            .unwrap_or_else(|| PathBuf::from("hmcl")),
    }
}

/// Returns the local home of the launcher, where the files it downloads are
/// kept.
///
/// The local home is the one of the Java launcher: the variable
/// `HMCL_LOCAL_HOME` when it is set, and the directory `.hmcl` of the
/// directory the launcher was started from otherwise.
pub fn local_home() -> PathBuf {
    if let Some(path) = std::env::var_os(LOCAL_HOME_VARIABLE) {
        return absolute(PathBuf::from(path));
    }

    current_directory().join(".hmcl")
}

/// Returns the directory the launcher keeps the dependencies it needs at run
/// time in.
///
/// The directory holds the files the launcher writes itself rather than the
/// files of the game, such as the authlib-injector the launcher starts the game
/// with. The directory is the one of the Java launcher: the variable
/// `HMCL_DEPENDENCIES_DIR` when it is set, and the directory `dependencies` of
/// the local home otherwise.
pub fn dependencies_directory() -> PathBuf {
    match std::env::var_os(DEPENDENCIES_DIRECTORY_VARIABLE) {
        Some(path) if !path.is_empty() => absolute(PathBuf::from(path)),
        _ => local_home().join("dependencies"),
    }
}

/// Returns the directory the game of the launcher of Mojang is kept in.
///
/// The directory is the one the launcher of Java uses, so the game the player
/// downloaded with the launcher of Mojang is found by this launcher as well:
///
/// * `~/.minecraft` on Linux,
/// * `%APPDATA%\.minecraft` on Windows,
/// * `~/Library/Application Support/minecraft` on macOS.
pub fn minecraft_directory() -> PathBuf {
    match OsType::current() {
        OsType::Linux => home_directory()
            .map(|home| home.join(".minecraft"))
            .unwrap_or_else(|| PathBuf::from(".minecraft")),
        OsType::Windows => match std::env::var_os("APPDATA") {
            Some(path) => PathBuf::from(path).join(".minecraft"),
            None => home_directory()
                .map(|home| home.join(".minecraft"))
                .unwrap_or_else(|| PathBuf::from(".minecraft")),
        },
        OsType::MacOs => home_directory()
            .map(|home| {
                home.join("Library")
                    .join("Application Support")
                    .join("minecraft")
            })
            .unwrap_or_else(|| PathBuf::from("minecraft")),
        OsType::Unknown => home_directory()
            .map(|home| home.join(".minecraft"))
            .unwrap_or_else(|| PathBuf::from(".minecraft")),
    }
}

/// Returns the home directory of the current user, if the platform reports
/// one.
fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Returns the path as it is written in the settings of the launcher.
fn absolute(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_directory().join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn the_directories_are_absolute() {
        let user_home = user_home();
        assert!(user_home.is_absolute(), "{user_home:?} is not absolute");

        let local_home = local_home();
        assert!(local_home.is_absolute(), "{local_home:?} is not absolute");
    }

    #[test]
    fn the_local_home_is_in_the_current_directory() {
        // The variable of the environment is not set in the tests, so the
        // local home is the directory the launcher was started from.
        if std::env::var_os(LOCAL_HOME_VARIABLE).is_none() {
            assert_eq!(local_home(), current_directory().join(".hmcl"));
        }
    }

    #[test]
    fn the_dependencies_are_kept_in_the_local_home() {
        // The directory of the dependencies follows the local home unless the
        // player names another directory.
        if std::env::var_os(DEPENDENCIES_DIRECTORY_VARIABLE).is_none() {
            assert_eq!(dependencies_directory(), local_home().join("dependencies"));
        }
        assert!(dependencies_directory().is_absolute());
    }
}
