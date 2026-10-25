//! The settings of the launcher.
//!
//! The launcher keeps its settings in a few JSON files, like the Java
//! launcher does:
//!
//! * `config/launcher-settings.json` holds the settings of the launcher,
//! * `config/user-settings.json` holds the settings of the user,
//! * `state/launcher-state.json` holds the state of the launcher, such as
//!   the window of the launcher.

pub mod accounts;
pub mod authlib_injectors;
pub mod authlib_servers;
pub mod game_directory;
pub mod game_settings;
pub mod game_settings_presets;
pub mod launcher;
pub mod launcher_state;
pub mod legacy_game_settings;
pub mod legacy_migration;
pub mod schema;
pub mod storage;
pub mod user;

pub use accounts::{
    account_private_data_path, account_private_data_schema, accounts_path, accounts_schema, reveal,
    user_account_private_data_path, user_accounts_path, AccountMetadataStore, AccountPrivateData,
    ACCOUNTS_DIRECTORY, ACCOUNTS_FILE, ACCOUNT_PRIVATE_DATA_FILE, PRIVATE_DIRECTORY,
    PROPERTY_NONCE, PROPERTY_PAYLOAD, PROPERTY_PROTECTION, PROTECTION_OBFUSCATED_V1,
    PROTECTION_PLAIN, USER_ACCOUNTS_FILE, USER_ACCOUNT_PRIVATE_DATA_FILE,
};
pub use authlib_injectors::{authlib_injectors_path, read_urls as read_authlib_injectors_urls};
pub use authlib_servers::{
    authlib_injector_metadata_path, authlib_injector_servers_path, normalize_url,
    AuthlibInjectorError, AuthlibInjectorMetadata, AuthlibInjectorServer,
    AuthlibInjectorServerList, AuthlibInjectorServerMetadataCache, LITTLE_SKIN_URL,
};
pub use launcher_state::{
    initial_content_bounds, ContentBounds, LauncherState, ScreenBounds, MIN_CONTENT_HEIGHT,
    MIN_CONTENT_WIDTH, MIN_VISIBLE_WINDOW_AREA,
};
pub use legacy_game_settings::{
    legacy_instance_settings, legacy_instance_settings_path, legacy_instance_settings_receipt,
    legacy_instance_settings_receipt_path, legacy_preset, migrate_legacy_instance_settings,
    migrate_legacy_instances, LegacyInstanceSettingsMigration, INSTANCE_CONFIG_DIRECTORY,
    INSTANCE_METADATA_DIRECTORY, INSTANCE_STATE_DIRECTORY, LEGACY_INSTANCE_SETTINGS_FILE,
    LEGACY_INSTANCE_SETTINGS_RECEIPT_FILE,
};
pub use legacy_migration::{
    assign_legacy_account_ids, has_legacy_config, legacy_account_id, legacy_game_settings_id,
    legacy_migration_receipt_path, legacy_profile_id, legacy_source_digest,
    legacy_user_accounts_path, legacy_user_accounts_receipt_path, locate_legacy_config,
    migrate_legacy_accounts, migrate_legacy_config, migrate_legacy_config_text,
    migrate_legacy_settings, migrate_legacy_user_accounts, migration_receipt,
    migration_receipt_matches, AccountMigration, DetachedSettings, LegacyConfigMigration,
    LEGACY_ACCOUNT_ID_NAMESPACE, LEGACY_CONFIG_FILENAME, LEGACY_CONFIG_FILENAME_LINUX,
    LEGACY_CURRENT_CONFIG_VERSION, LEGACY_GAME_SETTINGS_ID_NAMESPACE, LEGACY_GLOBAL_ACCOUNT_PREFIX,
    LEGACY_PROFILE_ID_NAMESPACE,
};

pub use game_directory::{
    default_home_game_directory, game_directories_schema, GameDirectories, GameDirectory,
    GameDirectoryId, GameDirectoryIdError, CURRENT_GAME_DIRECTORY_PATH, DEFAULT_GAME_DIRECTORY_ID,
    GAME_DIRECTORIES_FILE, GAME_DIRECTORY_ID_PREFIX, HOME_GAME_DIRECTORY_ID,
    USER_GAME_DIRECTORIES_FILE,
};
pub use game_settings::{
    instance_game_settings_path, instance_game_settings_schema, suggested_memory,
    DefaultIsolationType, DetectedJava, Effective, GameInstanceIconType, GameSettingsPresetId,
    GameSettingsPresetIdError, GameSettingsValues, GameWindowType, InstanceSettings,
    JavaVersionType, LauncherVisibility, Preset, GAME_SETTINGS_PRESET_ID_PREFIX,
    INSTANCE_GAME_SETTINGS_FILE, INSTANCE_GAME_SETTINGS_SCHEMA,
};
pub use game_settings_presets::{
    game_settings_schema, GameSettingsPresets, GAME_SETTINGS_FILE, GAME_SETTINGS_SCHEMA,
};
pub use launcher::{
    default_download_threads, BackgroundFallbackType, BackgroundLoadPolicy, BackgroundType,
    CommonDirectoryType, DownloadSource, LauncherSettings, NetworkBackgroundImageCachePolicy,
    ProxyAddress, ProxyCredentials, ProxyType, ThemeColorStyle, ThemeColorType, ThemeSelection,
};
pub use schema::{Compatibility, CompatibilityStatus, Schema, SchemaSettings, SchemaVersion};
pub use storage::{SettingsFile, LAUNCHER_SETTINGS_FILE, LAUNCHER_STATE_FILE, USER_SETTINGS_FILE};
pub use user::UserSettings;

use std::fs;
use std::path::{Path, PathBuf};

/// Returns the directory the settings of the launcher are kept in.
///
/// The directory is the `config` directory of the home directory of the
/// launcher.
pub fn config_directory(user_home: &Path) -> PathBuf {
    user_home.join("config")
}

/// Returns the directory the data the launcher reads again is kept in.
///
/// The directory is the `cache` directory of the home directory of the
/// launcher; the files in it may be dropped at any time, for the launcher
/// reads them from where they came from again.
pub fn cache_directory(user_home: &Path) -> PathBuf {
    user_home.join("cache")
}

/// Returns the path of the file that holds the settings of the launcher.
pub fn launcher_settings_path(user_home: &Path) -> PathBuf {
    config_directory(user_home).join(LAUNCHER_SETTINGS_FILE)
}

/// Returns the path of the file that holds the settings of the user.
pub fn user_settings_path(user_home: &Path) -> PathBuf {
    config_directory(user_home).join(USER_SETTINGS_FILE)
}

/// Returns the directory the state of the launcher is kept in.
///
/// The directory is the `state` directory of the home directory of the
/// launcher, which the files the launcher writes the state it had when it
/// was closed last into.
pub fn state_directory(user_home: &Path) -> PathBuf {
    user_home.join("state")
}

/// Returns the path of the file that holds the state of the launcher.
pub fn launcher_state_path(user_home: &Path) -> PathBuf {
    state_directory(user_home).join(LAUNCHER_STATE_FILE)
}

/// Returns the path of the file that holds the presets of the game settings.
pub fn game_settings_path(user_home: &Path) -> PathBuf {
    config_directory(user_home).join(GAME_SETTINGS_FILE)
}

/// Returns the path of the file that holds the game directories of the
/// workspace.
pub fn game_directories_path(home: &Path) -> PathBuf {
    config_directory(home).join(GAME_DIRECTORIES_FILE)
}

/// Returns the path of the file that holds the game directories every
/// workspace shares.
pub fn user_game_directories_path(user_home: &Path) -> PathBuf {
    config_directory(user_home).join(USER_GAME_DIRECTORIES_FILE)
}

/// Returns whether the workspace the launcher starts in is a new one.
///
/// A workspace is new when the launcher and the launcher of Java have written
/// no file of their settings into it yet: the settings of the launcher, the
/// state of its window, the servers of the authlib-injector, the directories
/// and the settings of the game, and the accounts of the player. The settings
/// of an older launcher next to the launcher count as written files as well,
/// because the migration of them writes the files above afterwards.
pub fn is_new_workspace(user_home: &Path, local_home: &Path) -> bool {
    !(file_exists(&launcher_settings_path(user_home))
        || file_exists(&launcher_state_path(user_home))
        || file_exists(&authlib_injector_servers_path(user_home))
        || file_exists(&game_directories_path(user_home))
        || file_exists(&game_settings_path(user_home))
        || file_exists(&accounts_path(user_home))
        || file_exists(&account_private_data_path(user_home))
        || file_exists(&local_home.join("settings.json"))
        || legacy_migration::has_legacy_config(local_home))
}

/// Returns whether a path names a file, and not a directory or nothing.
fn file_exists(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counts the directories the tests make, so tests that run in parallel
    /// do not share one.
    static INDEX: AtomicUsize = AtomicUsize::new(0);

    /// Creates a directory of the tests and forgets what was in it.
    fn directory(name: &str) -> PathBuf {
        let index = INDEX.fetch_add(1, Ordering::SeqCst);
        let directory = std::env::temp_dir()
            .join("hmcl-setting-test")
            .join(format!("{name}-{}-{index}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("the directory should be created");
        directory
    }

    #[test]
    fn the_settings_are_kept_in_the_config_directory() {
        let user_home = Path::new("/home/user/.hmcl");

        assert_eq!(
            launcher_settings_path(user_home),
            Path::new("/home/user/.hmcl/config/launcher-settings.json")
        );
        assert_eq!(
            user_settings_path(user_home),
            Path::new("/home/user/.hmcl/config/user-settings.json")
        );
        assert_eq!(
            launcher_state_path(user_home),
            Path::new("/home/user/.hmcl/state/launcher-state.json")
        );
        assert_eq!(
            game_settings_path(user_home),
            Path::new("/home/user/.hmcl/config/game-settings.json")
        );
    }

    #[test]
    fn a_workspace_without_files_is_a_new_one() {
        let user_home = directory("new");
        let local_home = directory("new-local");

        assert!(!file_exists(&user_home.join("config")));
        assert!(
            is_new_workspace(&user_home, &local_home),
            "a workspace where no launcher started is a new one"
        );

        let _ = fs::remove_dir_all(&user_home);
        let _ = fs::remove_dir_all(&local_home);
    }

    #[test]
    fn a_workspace_that_holds_a_file_of_the_launcher_is_not_new() {
        let user_home = directory("old");
        let local_home = directory("old-local");
        fs::create_dir_all(config_directory(&user_home))
            .expect("the config directory should be created");

        // Every file the launcher keeps its settings and state in makes the
        // workspace one a launcher already ran in; the servers of the
        // authlib-injector and the accounts are such files as well.
        fs::write(launcher_settings_path(&user_home), "{}")
            .expect("the settings should be written");
        assert!(!is_new_workspace(&user_home, &local_home));

        let _ = fs::remove_dir_all(&user_home);
        let _ = fs::remove_dir_all(&local_home);

        let user_home = directory("old-state");
        let local_home = directory("old-state-local");
        fs::create_dir_all(state_directory(&user_home))
            .expect("the state directory should be created");
        fs::write(launcher_state_path(&user_home), "{}").expect("the state should be written");
        assert!(!is_new_workspace(&user_home, &local_home));

        let _ = fs::remove_dir_all(&user_home);
        let _ = fs::remove_dir_all(&local_home);

        let user_home = directory("old-servers");
        let local_home = directory("old-servers-local");
        fs::create_dir_all(config_directory(&user_home))
            .expect("the config directory should be created");
        fs::write(authlib_injector_servers_path(&user_home), "{}")
            .expect("the servers should be written");
        assert!(!is_new_workspace(&user_home, &local_home));

        let _ = fs::remove_dir_all(&user_home);
        let _ = fs::remove_dir_all(&local_home);

        let user_home = directory("old-accounts");
        let local_home = directory("old-accounts-local");
        fs::create_dir_all(config_directory(&user_home))
            .expect("the config directory should be created");
        fs::write(accounts_path(&user_home), "{}").expect("the accounts should be written");
        assert!(!is_new_workspace(&user_home, &local_home));

        let _ = fs::remove_dir_all(&user_home);
        let _ = fs::remove_dir_all(&local_home);
    }

    #[test]
    fn a_workspace_that_holds_the_settings_of_an_older_launcher_is_not_new() {
        let user_home = directory("legacy");
        let local_home = directory("legacy-local");

        // The launcher of Java keeps its settings next to the launcher, and
        // the launcher of Rust both migrates and reads them, so a workspace
        // whose launcher named one is not new.
        fs::write(local_home.join("settings.json"), "{}")
            .expect("the settings of the older launcher should be written");
        assert!(!is_new_workspace(&user_home, &local_home));

        let _ = fs::remove_dir_all(&user_home);
        let _ = fs::remove_dir_all(&local_home);
    }
}
