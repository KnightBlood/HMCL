//! The migration of the settings of an older launcher.
//!
//! The older versions of the launcher kept every setting in one file,
//! `hmcl.json` of the directory of the launcher. The current launcher keeps
//! them in a few files of their own, so the file of an older launcher is read
//! once and its settings are written the way the current launcher reads them.
//!
//! The migration is pure: it takes the text of the legacy file and returns the
//! settings of the launcher, the state of the window, the servers of the
//! authlib-injector and the directories of the game, without touching the
//! disk. The caller decides when the files are written, which keeps the
//! migration testable and lets the launcher of today apply it before it reads
//! any setting.
//!
//! The migration follows the launcher of Java, which is
//! `LegacyConfigMigrator` there: a field that was renamed is read under the
//! old name, the settings that were replaced by others are converted, and the
//! parts that the current launcher keeps in a file of their own are taken out
//! of the settings. The fields that the launcher of Java drops are dropped
//! here as well, so the two launchers read the same settings out of the same
//! file.

use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

use super::accounts::{AccountMetadataStore, AccountPrivateData};
use super::authlib_servers::AuthlibInjectorServerList;
use super::game_directory::{GameDirectories, GameDirectory, GameDirectoryId};
use super::game_settings::{GameSettingsPresetId, Preset};
use super::game_settings_presets::GameSettingsPresets;
use super::launcher::{
    BackgroundType, DownloadSource, LauncherSettings, ProxyType, THEME_APPEARANCE_BACKGROUND,
    THEME_APPEARANCE_BACKGROUND_OPACITY, THEME_APPEARANCE_BRIGHTNESS_MODE, THEME_APPEARANCE_COLOR,
    THEME_APPEARANCE_TITLE_BAR_TRANSPARENT, THEME_APPEARANCE_WINDOW_TRANSPARENT,
};
use super::launcher_state::LauncherState;
use super::legacy_game_settings::{legacy_preset, migrate_legacy_instances};
use super::user::UserSettings;
use crate::auth::account_id::AccountId;
use crate::auth::uuid::Uuid;
use crate::error::Error;
use crate::i18n::LocalizedText;
use crate::util::digest::Sha256;
use crate::util::portable_path::PortablePath;

/// The version of the format of the legacy file the launcher reads.
///
/// The file names the version in `_version`; a file that names a newer one is
/// the file of a launcher the launcher of today does not know, so it is not
/// read at all.
pub const LEGACY_CURRENT_CONFIG_VERSION: i32 = 2;

/// The name of the legacy file of the launcher.
pub const LEGACY_CONFIG_FILENAME: &str = "hmcl.json";

/// The name of the legacy file of the launcher of Linux.
///
/// The older versions of the launcher wrote the name without the dot on the
/// systems that hide the files that start with a dot, so both names are read.
pub const LEGACY_CONFIG_FILENAME_LINUX: &str = ".hmcl.json";

/// The name the launcher of Java gives the profile it starts with.
pub const LEGACY_DEFAULT_PROFILE: &str = "Default";

/// The name the launcher of Java gives the profile of the home directory.
pub const LEGACY_HOME_PROFILE: &str = "Home";

/// The namespace the identifiers of the game directories are made in.
pub const LEGACY_PROFILE_ID_NAMESPACE: Uuid = Uuid::from_bytes([
    0xd5, 0x19, 0x34, 0x64, 0xa3, 0x2e, 0x52, 0xad, 0xb5, 0x0a, 0xd8, 0x8f, 0xe9, 0x67, 0xc3, 0xa2,
]);

/// The namespace the identifiers of the presets of the game settings are made
/// in.
pub const LEGACY_GAME_SETTINGS_ID_NAMESPACE: Uuid = Uuid::from_bytes([
    0x4b, 0xa5, 0x6d, 0xdd, 0x06, 0xb4, 0x5a, 0xa7, 0xb4, 0xda, 0x9e, 0x3c, 0x97, 0x07, 0xf8, 0x73,
]);

/// The namespace the identifiers of the accounts are made in.
pub const LEGACY_ACCOUNT_ID_NAMESPACE: Uuid = Uuid::from_bytes([
    0xcd, 0xc6, 0x08, 0xbb, 0x42, 0x6e, 0x5b, 0x93, 0x94, 0x8d, 0xb7, 0x96, 0x57, 0x05, 0xc9, 0xfe,
]);

/// The prefix the older launcher wrote before the identifier of an account
/// that was kept for every workspace.
pub const LEGACY_GLOBAL_ACCOUNT_PREFIX: &str = "$GLOBAL:";

/// The width the window of the older launcher drew around its content, which
/// the size of the window is reduced by when the size is read.
///
/// The older launcher wrote the size of its window including the shadow it
/// drew around it; the launcher of today writes the size of the content, so
/// the shadow is taken away.
pub const LEGACY_CUSTOM_DECORATION_SHADOW_EXTENT: f64 = 16.0;

/// The identifier of the wallpaper the older launcher shows for a background
/// it does not know.
pub const LEGACY_BUILTIN_BACKGROUND: &str = "2016-02-25";

/// The names of the backgrounds the older launcher numbered.
///
/// The older launcher wrote the background as a number; the names are the
/// ones the launcher of Java reads the numbers as, in the order it reads them.
const LEGACY_BACKGROUND_IMAGE_TYPES: [&str; 6] = [
    "DEFAULT",
    "CUSTOM",
    "BUILTIN",
    "NETWORK",
    "TRANSLUCENT",
    "PAINT",
];

/// The names of the proxies the older launcher numbered.
///
/// The older launcher wrote the proxy as the number of the proxy of Java; the
/// names are the ones the launcher of Java reads the numbers as, in the order
/// it reads them.
const LEGACY_PROXY_TYPES: [&str; 3] = ["DIRECT", "HTTP", "SOCKS"];

/// The parts of the settings of an older launcher that the launcher of today
/// keeps in a file of their own.
///
/// The settings of the launcher hold the rest; the parts here are written into
/// the files the current launcher reads them from.
#[derive(Debug, Clone, PartialEq)]
pub struct DetachedSettings {
    /// The state of the window, which is kept in `state/launcher-state.json`.
    pub launcher_state: LauncherState,
    /// The servers of the authlib-injector, which are kept in
    /// `config/authlib-injector-servers.json`.
    pub authlib_injector_servers: AuthlibInjectorServerList,
    /// The directories of the game, which are kept in
    /// `config/game-directories.json`.
    ///
    /// The list is empty when the file of the older launcher named no
    /// directory.
    pub game_directories: Vec<GameDirectory>,
    /// The presets of the game settings, which are kept in
    /// `config/game-settings.json`.
    ///
    /// The store is empty when the older launcher kept no settings of the
    /// game.
    pub presets: GameSettingsPresets,
    /// The accounts of the older launcher, split into the public data and the
    /// credentials.
    ///
    /// The accounts of the older launcher were kept in the settings of the
    /// launcher; the launcher of today keeps them in `config/accounts.json`
    /// and `private/account-private-data.json`.
    pub accounts: AccountMigration,
}

/// The accounts of an older launcher, read the way the launcher of today
/// keeps them.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountMigration {
    /// The public data of the accounts, which is kept in
    /// `config/accounts.json`.
    pub metadata: AccountMetadataStore,
    /// The credentials of the accounts, which are kept in
    /// `private/account-private-data.json`.
    pub private_data: AccountPrivateData,
}

impl AccountMigration {
    /// Creates a migration that found no account.
    pub fn empty() -> Self {
        Self {
            metadata: AccountMetadataStore::new(),
            private_data: AccountPrivateData::new(),
        }
    }

    /// Returns whether the older launcher kept no account.
    pub fn is_empty(&self) -> bool {
        self.metadata.accounts.is_empty()
    }
}

/// The settings of an older launcher, read the way the launcher of today
/// reads them.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyConfigMigration {
    /// The path of the file the settings were read from.
    pub path: String,
    /// The settings of the launcher.
    pub settings: LauncherSettings,
    /// The settings of the user, which are kept in `config/user-settings.json`.
    pub user_settings: UserSettings,
    /// The parts of the settings the launcher keeps in files of their own.
    pub detached: DetachedSettings,
}

/// Returns the identifier of the game directory the older launcher named.
///
/// The identifier is made of the name of the profile, so the same profile has
/// the same identifier every time the migration runs, and the identifier of
/// the directory the launcher of Java makes for the profile is the same.
pub fn legacy_profile_id(profile_name: &str) -> GameDirectoryId {
    GameDirectoryId::from_uuid(Uuid::generate_v5(LEGACY_PROFILE_ID_NAMESPACE, profile_name))
}

/// Returns the identifier of the preset the older launcher named.
///
/// The identifier is made of the name of the profile, the way
/// [`legacy_profile_id`] is, so a preset is named the same way every time.
pub fn legacy_game_settings_id(profile_name: &str) -> GameSettingsPresetId {
    GameSettingsPresetId::from_uuid(Uuid::generate_v5(
        LEGACY_GAME_SETTINGS_ID_NAMESPACE,
        profile_name,
    ))
}

/// Returns the identifier of the account record of a legacy identifier.
///
/// The older launcher named an account with a text that told its kind and the
/// name of the player; the identifier of the record is made of the text, so
/// the account the player selected in the older launcher is the account the
/// launcher of today selects.
pub fn legacy_account_id(legacy_identifier: &str) -> String {
    format!(
        "account-{}",
        Uuid::generate_v5(LEGACY_ACCOUNT_ID_NAMESPACE, legacy_identifier)
    )
}

/// Reads the text of a legacy settings file.
///
/// The returned settings are `None` when the text is not a file of an older
/// launcher, which is the case when the text is not a JSON object of the
/// format, or when it names a version the launcher of today does not know.
pub fn migrate_legacy_config_text(
    path: impl AsRef<Path>,
    text: &str,
) -> Option<LegacyConfigMigration> {
    let root: Value = serde_json::from_str(text).ok()?;
    let mut root = root.as_object()?.clone();

    // The version belongs to the format of the older launcher only; a file
    // that names a newer version is the file of a launcher the launcher of
    // today does not know, so it is left alone.
    let version = match root.remove("_version") {
        Some(Value::Number(number)) => number.as_i64().unwrap_or_default() as i32,
        _ => 0,
    };
    if version > LEGACY_CURRENT_CONFIG_VERSION {
        return None;
    }

    if version < LEGACY_CURRENT_CONFIG_VERSION {
        upgrade_config(&mut root, version);
    }

    // The settings of the user are kept in a file of their own by the
    // launcher of today.
    let user_settings = extract_user_settings(&root);

    // The settings of the game of every profile are read into a preset of the
    // game settings, so they are kept before the profiles are taken out of
    // the settings of the launcher.
    let legacy_configurations = root
        .get("configurations")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let launcher_state = extract_launcher_state(&mut root);
    let authlib_injector_servers = extract_authlib_injector_servers(&mut root);

    // The settings the older launcher kept for a whole launcher are read into
    // the presets of the game settings by the launcher of today.
    let legacy_allow_auto_agent = root
        .remove("allowAutoAgent")
        .and_then(|value| value.as_bool());
    let legacy_disable_auto_game_options = root
        .remove("disableAutoGameOptions")
        .and_then(|value| value.as_bool());

    // The accounts are kept in files of their own by the launcher of today,
    // so they are taken out of the settings before the rest is read.
    let accounts = extract_accounts(&mut root);
    migrate_legacy_selected_account(&mut root);

    let mut theme_appearance_overrides: Vec<String> = Vec::new();
    migrate_legacy_background_type(&mut root, &mut theme_appearance_overrides);
    migrate_legacy_proxy_type(&mut root);
    migrate_legacy_download_sources(&mut root);

    rename_member(&mut root, "commonDirType", "commonDirectoryType");
    rename_member(&mut root, "commonpath", "commonDirectory");
    migrate_legacy_language(&mut root);

    if rename_member(&mut root, "themeBrightness", "themeBrightnessMode") {
        add_theme_appearance_override(
            &mut theme_appearance_overrides,
            THEME_APPEARANCE_BRIGHTNESS_MODE,
        );
    }
    if rename_member(&mut root, "titleTransparent", "titleBarTransparent") {
        add_theme_appearance_override(
            &mut theme_appearance_overrides,
            THEME_APPEARANCE_TITLE_BAR_TRANSPARENT,
        );
    }
    if rename_member(&mut root, "theme", "customThemeColor") {
        root.insert(
            "themeColorType".to_string(),
            Value::String("CUSTOM".to_string()),
        );
        add_theme_appearance_override(&mut theme_appearance_overrides, THEME_APPEARANCE_COLOR);
    }
    rename_member(&mut root, "fontFamily", "logFontFamily");
    rename_member(&mut root, "fontSize", "logFontSize");

    let mut migrated_background_source = false;
    let mut migrated_network_background_source = false;
    migrated_background_source |= rename_member(&mut root, "bgpath", "customBackgroundImagePath");
    migrated_background_source |=
        rename_member(&mut root, "backgroundImage", "customBackgroundImagePath");
    migrated_network_background_source |=
        rename_member(&mut root, "bgurl", "networkBackgroundImageUrl");
    migrated_network_background_source |=
        rename_member(&mut root, "backgroundImageUrl", "networkBackgroundImageUrl");
    migrated_background_source |= migrated_network_background_source;
    if migrated_background_source {
        add_theme_appearance_override(&mut theme_appearance_overrides, THEME_APPEARANCE_BACKGROUND);
    }
    if migrated_network_background_source {
        // The images the older launcher downloaded were not cached, so the
        // address of the image the player selected is read again rather than
        // shown from a cache the launcher of today does not have.
        root.insert(
            "networkBackgroundImageCachePolicy".to_string(),
            Value::String("DISABLED".to_string()),
        );
    }

    rename_member(&mut root, "bgpaint", "customBackgroundPaint");
    rename_member(&mut root, "backgroundPaint", "customBackgroundPaint");
    migrate_background_opacity(&mut root, &mut theme_appearance_overrides);
    root.insert("windowTransparent".to_string(), Value::Bool(true));
    add_theme_appearance_override(
        &mut theme_appearance_overrides,
        THEME_APPEARANCE_WINDOW_TRANSPARENT,
    );
    if !theme_appearance_overrides.is_empty() {
        root.insert(
            "themeAppearanceOverrides".to_string(),
            Value::Array(
                theme_appearance_overrides
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }

    rename_member(&mut root, "proxyUserName", "proxyUser");

    migrate_legacy_selected_versions(&mut root);
    migrate_legacy_selected_game_directory(&mut root);

    let game_directories = extract_game_directories(&mut root);

    let settings: LauncherSettings = serde_json::from_value(Value::Object(root)).ok()?;

    let presets = migrate_legacy_presets(
        &game_directories,
        &legacy_configurations,
        legacy_allow_auto_agent,
        legacy_disable_auto_game_options,
    );

    Some(LegacyConfigMigration {
        path: path.as_ref().to_string_lossy().into_owned(),
        settings,
        user_settings,
        detached: DetachedSettings {
            launcher_state,
            authlib_injector_servers,
            game_directories,
            presets,
            accounts,
        },
    })
}

/// Reads the settings the older launcher kept for every profile into presets.
///
/// A profile that kept settings of the game becomes a preset that the
/// instances of the directory inherit. The switches the older launcher kept
/// for the whole launcher are read into the default preset, which is created
/// when the older launcher kept no settings at all, so a player who turned the
/// automatic settings of the game off keeps them off.
fn migrate_legacy_presets(
    game_directories: &[GameDirectory],
    configurations: &Map<String, Value>,
    legacy_allow_auto_agent: Option<bool>,
    legacy_disable_auto_game_options: Option<bool>,
) -> GameSettingsPresets {
    let mut presets = GameSettingsPresets::new();

    // The profiles are in the order the older launcher wrote them, so the
    // presets are in the order the player saw them.
    for directory in game_directories {
        let Some(profile_name) = legacy_profile_name(directory) else {
            continue;
        };
        let profile = configurations.get(&profile_name).and_then(Value::as_object);

        // The older launcher kept the settings of the game of a profile in
        // the member `global` of the profile.
        let legacy_settings = profile
            .and_then(|profile| profile.get("global"))
            .and_then(Value::as_object);

        let Some(id) = directory.legacy_game_settings else {
            continue;
        };
        if presets.get_preset(id).is_some() {
            continue;
        }

        let auto_name_number = presets.new_preset_auto_name_number();
        presets
            .presets
            .push(legacy_preset(id, auto_name_number, legacy_settings));
    }

    let allow_auto_agent = legacy_allow_auto_agent.unwrap_or(false);
    let disable_auto_game_options = legacy_disable_auto_game_options.unwrap_or(false);
    if allow_auto_agent || disable_auto_game_options {
        if presets.presets.is_empty() {
            let id = presets.new_preset_id();
            let mut preset = Preset::new(id);
            preset.auto_name_number = Some(1);
            presets.presets.push(preset);
        }
        for preset in &mut presets.presets {
            if allow_auto_agent {
                preset.settings.allow_auto_agent = Some(true);
            }
            if disable_auto_game_options {
                preset.settings.disable_auto_game_options = Some(true);
            }
        }
    }

    presets
}

/// Returns the name the older launcher gave the profile of a directory.
///
/// The names of the two profiles the launcher creates by itself are not kept
/// as the names of the directories, so they are derived from the identifier of
/// the directory instead.
fn legacy_profile_name(directory: &GameDirectory) -> Option<String> {
    if directory.id == legacy_profile_id(LEGACY_DEFAULT_PROFILE) {
        return Some(LEGACY_DEFAULT_PROFILE.to_string());
    }
    if directory.id == legacy_profile_id(LEGACY_HOME_PROFILE) {
        return Some(LEGACY_HOME_PROFILE.to_string());
    }
    directory
        .name
        .as_ref()
        .map(|name| name.text(&[]).to_string())
}

/// Reads the legacy settings file at a path.
///
/// The settings are `None` when the file is missing or cannot be read; the
/// migration of the launcher of Java ignores such a file rather than failing
/// to start.
pub fn migrate_legacy_config(path: impl AsRef<Path>) -> Option<LegacyConfigMigration> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).ok()?;
    migrate_legacy_config_text(path, &text)
}

/// Returns the path of the legacy file of a directory, when there is one.
///
/// The file the launcher of today wrote is looked for first, then the name of
/// the systems that hide the files that start with a dot; the launcher of Java
/// reads the two names in the same order.
pub fn locate_legacy_config(directory: impl AsRef<Path>) -> Option<std::path::PathBuf> {
    let directory = directory.as_ref();
    for name in [LEGACY_CONFIG_FILENAME, LEGACY_CONFIG_FILENAME_LINUX] {
        let path = directory.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// Returns whether a directory holds the file of an older launcher.
pub fn has_legacy_config(directory: impl AsRef<Path>) -> bool {
    locate_legacy_config(directory).is_some()
}

/// Returns the path of the receipt that records the migration of the settings.
///
/// The receipt is kept with the state of the launcher rather than with the
/// settings, so a player who deletes the settings of the launcher does not
/// migrate the same file of the older launcher again.
pub fn legacy_migration_receipt_path(user_home: &Path) -> std::path::PathBuf {
    super::state_directory(user_home).join("launcher-settings.migration-receipt.json")
}

/// Returns the path of the receipt that records the migration of the accounts
/// every workspace shares.
pub fn legacy_user_accounts_receipt_path(user_home: &Path) -> std::path::PathBuf {
    super::state_directory(user_home).join("user-accounts.migration-receipt.json")
}

/// Returns the path of the file the older launcher kept the accounts of every
/// workspace in.
pub fn legacy_user_accounts_path(user_home: &Path) -> std::path::PathBuf {
    user_home.join("accounts.json")
}

/// Migrates the accounts the older launcher shared between the workspaces.
///
/// The older launcher kept the accounts that every workspace sees in
/// `accounts.json` of the home of the user, which is the file of the launcher
/// of today as well; the accounts of today are kept in `config/user-accounts.json`
/// and `private/user-account-private-data.json`.
///
/// The accounts of the shared file are named with the prefix of the shared
/// accounts, so an account of the shared file does not take the identifier of
/// an account of a single workspace. The file is read once: the receipt
/// records the text of the file, so a launcher that starts again does not read
/// the file again.
///
/// A file of the launcher of today is never replaced, and a file of the older
/// launcher that cannot be read is reported rather than stopping the launcher.
pub fn migrate_legacy_user_accounts(user_home: &Path) -> Result<Option<AccountMigration>, Error> {
    let path = legacy_user_accounts_path(user_home);
    if !path.is_file() {
        return Ok(None);
    }

    let text = std::fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let receipt_path = legacy_user_accounts_receipt_path(user_home);
    if let Ok(receipt) = std::fs::read_to_string(&receipt_path) {
        if migration_receipt_matches(&receipt, &text) {
            return Ok(None);
        }
    }

    let records: Vec<Map<String, Value>> = match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(records)) => records
            .into_iter()
            .filter_map(|record| record.as_object().cloned())
            .collect(),
        // The file of the older launcher holds a list of records; anything
        // else names no account, so nothing is migrated and the file is left
        // for a launcher that can read it.
        _ => return Ok(None),
    };

    let migration = migrate_legacy_accounts(&records, true);

    // The accounts of the launcher of today are not replaced, so a player who
    // already added accounts here keeps them.
    let accounts_path = super::user_accounts_path(user_home);
    if !accounts_path.exists() && !migration.is_empty() {
        super::AccountMetadataStore::save_to(&accounts_path, &migration.metadata)?;
        if !migration.private_data.is_empty() {
            migration
                .private_data
                .save_to(&super::user_account_private_data_path(user_home))?;
        }
    }

    let receipt = migration_receipt(
        &text,
        &crate::util::datetime::DateTime::now_utc().to_string(),
    );
    let parent = receipt_path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
    std::fs::write(&receipt_path, receipt.as_bytes())
        .map_err(|error| Error::io(&receipt_path, error))?;

    Ok(Some(migration))
}

/// Migrates the settings of an older launcher and writes the files of the
/// launcher of today.
///
/// The file of the older launcher is looked for in the directory the launcher
/// was started from, which is `local_home`, and the settings are written into
/// `user_home`, where the launcher of today keeps its files.
///
/// The migration runs once: the receipt records the text of the file it read,
/// so a launcher that starts again does not read the file again, while a file
/// the player changed in the older launcher is read again. A settings file the
/// launcher of today wrote is never replaced, so a player who already uses
/// this launcher keeps what they set.
///
/// The directories of the game are not written, for the launcher of today
/// keeps them in a file the migration of the settings does not create; the
/// caller reads them from the returned value and writes them the way it keeps
/// its own. The presets of the game settings and the settings of the instances
/// are written by the caller as well, since they live under the directories of
/// the game.
pub fn migrate_legacy_settings(
    local_home: &Path,
    user_home: &Path,
) -> Result<Option<LegacyConfigMigration>, Error> {
    let Some(path) = locate_legacy_config(local_home) else {
        return Ok(None);
    };

    let text = std::fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let receipt_path = legacy_migration_receipt_path(user_home);
    if let Ok(receipt) = std::fs::read_to_string(&receipt_path) {
        if migration_receipt_matches(&receipt, &text) {
            return Ok(None);
        }
    }

    let Some(migration) = migrate_legacy_config_text(&path, &text) else {
        return Ok(None);
    };

    // The settings of the launcher of today are not replaced by the settings
    // of the older launcher, so a player who already uses this launcher keeps
    // what they set.
    let settings_path = super::launcher_settings_path(user_home);
    if !settings_path.exists() {
        super::SettingsFile::new(settings_path, migration.settings.clone()).save()?;
    }

    let user_settings_path = super::user_settings_path(user_home);
    if !user_settings_path.exists() {
        super::SettingsFile::new(user_settings_path, migration.user_settings.clone()).save()?;
    }

    let state_path = super::launcher_state_path(user_home);
    if !state_path.exists() {
        migration.detached.launcher_state.save(user_home)?;
    }

    let servers_path = super::authlib_injector_servers_path(user_home);
    if !servers_path.exists() {
        super::SettingsFile::new(
            servers_path,
            migration.detached.authlib_injector_servers.clone(),
        )
        .save()?;
    }

    // The presets of the game settings are kept in a file of their own, which
    // is written when the launcher of today has no presets yet.
    let presets_path = super::game_settings_path(user_home);
    if !presets_path.exists() && !migration.detached.presets.presets.is_empty() {
        super::SettingsFile::new(presets_path, migration.detached.presets.clone()).save()?;
    }

    // The directories of the game are kept in a file of their own, which is
    // written when the launcher of today has no directories yet. The
    // directories of the older launcher are merged into the directories of the
    // user, so a workspace that has none of its own shows the shared ones.
    let directories_path = super::game_directories_path(user_home);
    if !directories_path.exists() && !migration.detached.game_directories.is_empty() {
        let directories =
            GameDirectories::from_directories(migration.detached.game_directories.clone());
        super::SettingsFile::new(directories_path, directories).save()?;
    }

    // The instances of the game keep their own settings in a file of their
    // own, which is written when the launcher of today has none yet. The
    // instances of a directory inherit the settings of the profile it became.
    for directory in &migration.detached.game_directories {
        let Some(parent) = directory.legacy_game_settings else {
            continue;
        };
        let game_directory = directory.absolute_path(local_home);
        let _ =
            migrate_legacy_instances(&game_directory, &migration.detached.presets, Some(parent));
    }

    // The accounts are split into the public data and the credentials, which
    // are kept in files of their own. The files of the launcher of today are
    // not replaced, so a player who already uses this launcher keeps the
    // accounts they added here.
    let accounts_path = super::accounts_path(user_home);
    if !accounts_path.exists() && !migration.detached.accounts.is_empty() {
        super::AccountMetadataStore::save(user_home, &migration.detached.accounts.metadata)?;
        if !migration.detached.accounts.private_data.is_empty() {
            migration.detached.accounts.private_data.save(user_home)?;
        }
    }

    let receipt = migration_receipt(
        &text,
        &crate::util::datetime::DateTime::now_utc().to_string(),
    );
    let parent = receipt_path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
    std::fs::write(&receipt_path, receipt.as_bytes())
        .map_err(|error| Error::io(&receipt_path, error))?;

    Ok(Some(migration))
}

/// Reads the fingerprint of a legacy file, so the launcher knows whether the
/// migration was applied to the file it reads now.
///
/// The fingerprint is the digest of the text of the file; a file that was
/// written again after the migration is migrated again, so the settings the
/// player changed in the older launcher are not lost.
pub fn legacy_source_digest(text: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(text.as_bytes());
    crate::util::digest::to_hex(&digest.finish())
}

/// Returns whether a migration receipt records the text of a legacy file.
///
/// A receipt that is missing, that cannot be read, or that holds no digest is
/// not a record of the text, so the migration runs again.
pub fn migration_receipt_matches(receipt: &str, text: &str) -> bool {
    let Ok(receipt) = serde_json::from_str::<Value>(receipt) else {
        return false;
    };

    receipt
        .get("sourceSHA256")
        .and_then(Value::as_str)
        .is_some_and(|digest| digest.eq_ignore_ascii_case(&legacy_source_digest(text)))
}

/// Writes the text of a migration receipt for a legacy file.
///
/// The receipt records the fingerprint of the file and the moment of the
/// migration; the launcher writes it after the settings are saved, so a
/// migration that was interrupted runs again.
pub fn migration_receipt(text: &str, migrated_at: &str) -> String {
    let receipt = serde_json::json!({
        "sourceSize": text.len(),
        "sourceSHA256": legacy_source_digest(text),
        "migratedAt": migrated_at,
    });

    serde_json::to_string_pretty(&receipt).unwrap_or_default()
}

/// Moves a member of an object to the name the launcher of today reads it
/// under, and returns whether the object had the member.
fn rename_member(object: &mut Map<String, Value>, legacy_name: &str, current_name: &str) -> bool {
    match object.remove(legacy_name) {
        Some(value) => {
            object.insert(current_name.to_string(), value);
            true
        }
        None => false,
    }
}

/// Adds a key to the parts of the theme the player changed, unless the key is
/// already there.
fn add_theme_appearance_override(overrides: &mut Vec<String>, key: &str) {
    if !overrides.iter().any(|named| named == key) {
        overrides.push(key.to_string());
    }
}

/// Returns the string of a member, or `None` when the member is missing or is
/// not a string.
pub(super) fn member_string(object: &Map<String, Value>, name: &str) -> Option<String> {
    object.get(name).and_then(Value::as_str).map(str::to_string)
}

/// Returns the boolean of a member, or the default when the member is missing
/// or is not a boolean.
pub(super) fn member_bool(object: &Map<String, Value>, name: &str, default: bool) -> bool {
    object.get(name).and_then(Value::as_bool).unwrap_or(default)
}

/// Returns the number of a member, or `None` when the member is missing or is
/// not a number.
///
/// The older launcher sometimes wrote a number as the text of it, which the
/// launcher of Java reads as a number as well.
pub(super) fn member_number(object: &Map<String, Value>, name: &str) -> Option<f64> {
    match object.get(name)? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

/// Reads a member as the number of a background or a proxy of the older
/// launcher.
pub(super) fn member_ordinal(object: &Map<String, Value>, name: &str) -> Option<usize> {
    let value = member_number(object, name)?;
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return None;
    }
    Some(value as usize)
}

/// Converts the settings of the launcher of 2.x into the format of the last
/// legacy version.
///
/// The fields the older launcher wrote under other names are converted, so the
/// rest of the migration reads one format only.
fn upgrade_config(root: &mut Map<String, Value>, version: i32) {
    if version >= 1 {
        return;
    }

    // The launcher of 2.x kept the accounts that play without an account in a
    // map of names to identifiers, which is a list of records by now.
    let offline_accounts = root
        .get("auth")
        .and_then(Value::as_object)
        .and_then(|auth| auth.get("offline"))
        .and_then(Value::as_object)
        .cloned();
    if let Some(offline) = offline_accounts {
        if let Some(uuid_map) = offline.get("uuidMap").and_then(Value::as_object) {
            let selected = if root.contains_key("selectedAccount") {
                None
            } else {
                member_string(&offline, "IAuthenticator_UserName")
            };

            let mut accounts = Vec::new();
            for (name, identifier) in uuid_map {
                let mut storage = Map::new();
                storage.insert("type".to_string(), Value::String("offline".to_string()));
                storage.insert("username".to_string(), Value::String(name.clone()));
                storage.insert("uuid".to_string(), identifier.clone());

                if selected.as_deref() == Some(name.as_str()) {
                    root.insert(
                        "selectedAccount".to_string(),
                        Value::String(format!("{name}:{name}")),
                    );
                }

                accounts.push(Value::Object(storage));
            }
            root.insert("accounts".to_string(), Value::Array(accounts));
        }
    }

    // The launcher before 3.1.70 kept the common directory as a path only, so
    // the type of the directory is derived from the path.
    if !root.contains_key("commonDirType") {
        let directory = member_string(root, "commonpath").unwrap_or_default();
        let kind = if directory.is_empty() {
            "DEFAULT"
        } else {
            "CUSTOM"
        };
        root.insert(
            "commonDirectoryType".to_string(),
            Value::String(kind.to_string()),
        );
    }

    let background = member_string(root, "bgpath").unwrap_or_default();
    root.insert(
        "backgroundType".to_string(),
        Value::String(
            if background.trim().is_empty() {
                "DEFAULT"
            } else {
                "CUSTOM"
            }
            .to_string(),
        ),
    );

    let proxy_host = member_string(root, "proxyHost").unwrap_or_default();
    root.insert(
        "hasProxy".to_string(),
        Value::Bool(!proxy_host.trim().is_empty()),
    );
    let proxy_user = member_string(root, "proxyUserName").unwrap_or_default();
    root.insert(
        "hasProxyAuth".to_string(),
        Value::Bool(!proxy_user.trim().is_empty()),
    );

    // The download source of the launcher before 3.x was a number.
    if let Some(id) = member_number(root, "downloadtype") {
        let name = match id as i64 {
            0 => Some("mojang"),
            1 => Some("bmclapi"),
            _ => None,
        };
        if let Some(name) = name {
            root.insert("downloadType".to_string(), Value::String(name.to_string()));
        }
    }
}

/// Takes the settings of the user out of the settings of the launcher.
///
/// The settings of the user were kept in the same file by the older launcher;
/// the launcher of today keeps them in `config/user-settings.json`, so the
/// members are read and left in the file, which the migration does not write.
fn extract_user_settings(root: &Map<String, Value>) -> UserSettings {
    let mut settings = UserSettings::default();

    if let Some(retention) = member_number(root, "logRetention") {
        settings.log_retention = Some(retention.max(0.0) as u32);
    }
    if let Some(enabled) = root.get("enableOfflineAccount").and_then(Value::as_bool) {
        settings.enable_offline_account = enabled;
    }
    settings.font_anti_aliasing = member_string(root, "fontAntiAliasing");
    settings.user_java = member_strings(root, "userJava");
    settings.disabled_java = member_strings(root, "disabledJava");

    settings
}

/// Returns the strings of a member that is a list of strings.
fn member_strings(object: &Map<String, Value>, name: &str) -> Vec<String> {
    object
        .get(name)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Takes the state of the window out of the settings of the launcher.
///
/// The older launcher kept the place and the size of its window with the other
/// settings; the launcher of today keeps them in
/// `state/launcher-state.json`, so the members are moved out.
///
/// The size the older launcher wrote included the shadow it drew around the
/// window, so the shadow is taken away. The place was written in pixels, which
/// the launcher of Java keeps as it is and the launcher of today reads as a
/// share of the screen; the window is put in the middle of the screen when the
/// place it names is not on a screen, so a window the player never moved is
/// shown where the launcher of today shows it.
fn extract_launcher_state(root: &mut Map<String, Value>) -> LauncherState {
    let mut state = LauncherState::new();

    let x = member_number(root, "x").unwrap_or_default();
    let y = member_number(root, "y").unwrap_or_default();
    let width = member_number(root, "width")
        .map(legacy_window_size)
        .unwrap_or_default();
    let height = member_number(root, "height")
        .map(legacy_window_size)
        .unwrap_or_default();

    for name in ["x", "y", "width", "height"] {
        root.remove(name);
    }

    state.set_legacy_bounds(x, y, width, height);

    if let Some(prompted) = root.remove("promptedVersion") {
        if let Some(version) = prompted.as_str() {
            state.set_prompted_version(Some(version.to_string()));
        }
    }
    if let Some(tips) = root.remove("shownTips").and_then(|tips| match tips {
        Value::Object(map) => Some(map),
        _ => None,
    }) {
        for (id, value) in tips {
            state.mark_tip_shown(id, value);
        }
    }

    state
}

/// Returns the size of the content of the window of the older launcher.
fn legacy_window_size(size: f64) -> f64 {
    if !size.is_finite() {
        return 0.0;
    }
    (size - LEGACY_CUSTOM_DECORATION_SHADOW_EXTENT).max(0.0)
}

/// Takes the servers of the authlib-injector out of the settings of the
/// launcher.
///
/// The servers are kept in `config/authlib-injector-servers.json` by the
/// launcher of today. The older launcher named LittleSkin as a server every
/// player has unless the player removed it, which the flag `addedLittleSkin`
/// records; the list the launcher of today starts with names LittleSkin, so
/// the server is added when the flag is missing or false.
fn extract_authlib_injector_servers(root: &mut Map<String, Value>) -> AuthlibInjectorServerList {
    let mut list = AuthlibInjectorServerList::empty();

    if let Some(servers) = root.remove("authlibInjectorServers") {
        if let Ok(servers) =
            serde_json::from_value::<AuthlibInjectorServerList>(serde_json::json!({
                "$schema": "https://schemas.glavo.site/hmcl/authlib-injector-servers/1.0.0",
                "servers": servers,
            }))
        {
            list = servers;
        }
    }

    let added_little_skin = member_bool(root, "addedLittleSkin", false);
    root.remove("addedLittleSkin");
    if !added_little_skin {
        list.add_little_skin_if_absent();
    }

    list
}

/// Converts the language of the older launcher into the language of today.
fn migrate_legacy_language(root: &mut Map<String, Value>) {
    let Some(language) = root.remove("localization") else {
        return;
    };
    let Some(language) = language.as_str() else {
        return;
    };

    // The older launcher named the two forms of the Chinese language with the
    // names of the locale of Java, which the launcher of today writes the way
    // the language of the system is named.
    let language = match language {
        "zh_CN" => "zh-Hans",
        "zh" => "zh-Hant",
        other => other,
    };
    root.insert("language".to_string(), Value::String(language.to_string()));
}

/// Converts the number of the background of the older launcher into its name.
fn migrate_legacy_background_type(root: &mut Map<String, Value>, overrides: &mut Vec<String>) {
    if let Some(ordinal) = member_ordinal(root, "backgroundType") {
        if let Some(name) = LEGACY_BACKGROUND_IMAGE_TYPES.get(ordinal) {
            root.insert(
                "backgroundType".to_string(),
                Value::String((*name).to_string()),
            );
            add_theme_appearance_override(overrides, THEME_APPEARANCE_BACKGROUND);

            // The third background was the wallpaper of 2016, which the
            // launcher of today names by its identifier.
            if ordinal == 2 {
                root.insert(
                    "builtinBackgroundId".to_string(),
                    Value::String(LEGACY_BUILTIN_BACKGROUND.to_string()),
                );
            }
        }
    }

    let background_type = member_string(root, "backgroundType");

    // The launcher of today names the wallpaper by its identifier rather than
    // having a background of its own for it.
    if background_type.as_deref() == Some("CLASSIC") {
        root.insert(
            "backgroundType".to_string(),
            Value::String("BUILTIN".to_string()),
        );
        root.insert(
            "builtinBackgroundId".to_string(),
            Value::String(LEGACY_BUILTIN_BACKGROUND.to_string()),
        );
        add_theme_appearance_override(overrides, THEME_APPEARANCE_BACKGROUND);
    }

    // The translucent background was a white picture the player saw through,
    // which is a painted background by now.
    if background_type.as_deref() == Some("TRANSLUCENT") {
        root.insert(
            "backgroundType".to_string(),
            Value::String("PAINT".to_string()),
        );
        root.insert(
            "customBackgroundPaint".to_string(),
            Value::String("#ffffff".to_string()),
        );
        root.insert("backgroundOpacity".to_string(), serde_json::json!(0.5));
        add_theme_appearance_override(overrides, THEME_APPEARANCE_BACKGROUND);
        add_theme_appearance_override(overrides, THEME_APPEARANCE_BACKGROUND_OPACITY);
        root.remove("bgpaint");
    }

    // A background the player chose is a part of the theme the player changed,
    // so the launcher of today does not replace it with the one of the theme.
    if let Some(background_type) = member_string(root, "backgroundType") {
        if BackgroundType::names()
            .iter()
            .any(|name| *name == background_type && *name != BackgroundType::Default.name())
        {
            add_theme_appearance_override(overrides, THEME_APPEARANCE_BACKGROUND);
        }
    }
}

/// Converts the opacity of the background of the older launcher.
///
/// The older launcher kept the opacity as a percentage, which the launcher of
/// today keeps as a share.
fn migrate_background_opacity(root: &mut Map<String, Value>, overrides: &mut Vec<String>) {
    let Some(value) = root.remove("bgImageOpacity") else {
        return;
    };

    if overrides
        .iter()
        .any(|key| key == THEME_APPEARANCE_BACKGROUND_OPACITY)
    {
        return;
    }

    let Some(percent) = value.as_f64().or_else(|| {
        value
            .as_str()
            .and_then(|text| text.trim().parse::<f64>().ok())
    }) else {
        return;
    };
    if !percent.is_finite() {
        return;
    }

    root.insert(
        "backgroundOpacity".to_string(),
        serde_json::json!((percent / 100.0).clamp(0.0, 1.0)),
    );
    add_theme_appearance_override(overrides, THEME_APPEARANCE_BACKGROUND_OPACITY);
}

/// Converts the flag and the number of the proxy of the older launcher into
/// the proxy of the launcher of today.
///
/// The older launcher kept whether the player uses a proxy at all, and the
/// number of the proxy of Java; the launcher of today keeps the name of the
/// proxy only, and a player who does not use a proxy uses the one of the
/// system.
fn migrate_legacy_proxy_type(root: &mut Map<String, Value>) {
    let has_proxy = root
        .remove("hasProxy")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if !has_proxy {
        root.insert(
            "proxyType".to_string(),
            Value::String(ProxyType::System.name().to_string()),
        );
        return;
    }

    if let Some(ordinal) = member_ordinal(root, "proxyType") {
        let name = LEGACY_PROXY_TYPES
            .get(ordinal)
            .copied()
            .unwrap_or(ProxyType::Http.name());
        root.insert("proxyType".to_string(), Value::String(name.to_string()));
        return;
    }

    // The file may name the proxy already; a name the launcher of today does
    // not know is read as an HTTP proxy, which is the proxy the older launcher
    // used when the player did not choose another one.
    let name = member_string(root, "proxyType");
    let known = name.as_deref().is_some_and(|name| {
        name == ProxyType::Direct.name()
            || name == ProxyType::Http.name()
            || name == ProxyType::Socks.name()
    });
    if !known {
        root.insert(
            "proxyType".to_string(),
            Value::String(ProxyType::Http.name().to_string()),
        );
    }
}

/// Converts the sources of the downloads of the older launcher.
///
/// The older launcher chose one source for everything, and could choose it
/// automatically; the launcher of today keeps a source for the list of the
/// versions and a source for the files, which the older choice is used for.
fn migrate_legacy_download_sources(root: &mut Map<String, Value>) {
    let auto_choose = root.remove("autoChooseDownloadType");
    let legacy_download_type = root.remove("downloadType");
    let legacy_version_list_source = root.remove("versionListSource");

    if auto_choose.is_none()
        && legacy_download_type.is_none()
        && legacy_version_list_source.is_none()
    {
        return;
    }

    let source = if auto_choose
        .as_ref()
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        parse_legacy_download_source(legacy_version_list_source.as_ref(), DownloadSource::Default)
    } else {
        parse_legacy_download_source(legacy_download_type.as_ref(), DownloadSource::Official)
    };

    root.insert(
        "versionListSource".to_string(),
        Value::String(source.name().to_string()),
    );
    root.insert(
        "fileDownloadSource".to_string(),
        Value::String(source.name().to_string()),
    );
}

/// Reads a source of the downloads of the older launcher.
///
/// The older launcher named the sources with a few names, some of which are
/// the names of the mirrors rather than the names the launcher of today uses.
fn parse_legacy_download_source(value: Option<&Value>, default: DownloadSource) -> DownloadSource {
    let Some(value) = value.and_then(Value::as_str) else {
        return default;
    };

    match value.to_ascii_lowercase().as_str() {
        "default" | "balanced" => DownloadSource::Default,
        "official" | "mojang" => DownloadSource::Official,
        "mirror" | "bmclapi" => DownloadSource::Mirror,
        other => DownloadSource::of(other).unwrap_or(default),
    }
}

/// Converts the account the player selected in the older launcher into the
/// identifier of the account record of today.
fn migrate_legacy_selected_account(root: &mut Map<String, Value>) {
    let Some(selected) = root.get("selectedAccount") else {
        return;
    };

    let Some(identifier) = selected.as_str().map(str::to_string) else {
        root.remove("selectedAccount");
        return;
    };
    if identifier.trim().is_empty() {
        root.remove("selectedAccount");
        return;
    }

    root.insert(
        "selectedAccount".to_string(),
        Value::String(legacy_account_id(&identifier)),
    );
}

/// Takes the accounts out of the settings of the older launcher.
///
/// The older launcher kept every account in the list `accounts` of its
/// settings, and named the account the player selected in `selectedAccount`.
/// The launcher of today keeps the public data and the credentials of the
/// accounts in files of their own, so the list is read and the accounts are
/// split into the two stores.
///
/// The account the player selected is read from the record that marks itself
/// as selected when the file names none, which is what the older launcher
/// wrote when the player selected an account in the list.
fn extract_accounts(root: &mut Map<String, Value>) -> AccountMigration {
    let Some(accounts) = root.remove("accounts") else {
        return AccountMigration::empty();
    };

    let Some(accounts) = accounts.as_array() else {
        // The member the older launcher wrote is a list; anything else names
        // no account at all.
        return AccountMigration::empty();
    };

    let mut records = Vec::with_capacity(accounts.len());
    let mut selected_from_record = None;
    for account in accounts {
        let Some(record) = account.as_object() else {
            continue;
        };

        if !root.contains_key("selectedAccount")
            && selected_from_record.is_none()
            && member_bool(record, "selected", false)
        {
            selected_from_record = legacy_identifier_of_storage(record);
        }
        records.push(record.clone());
    }

    if let Some(identifier) = selected_from_record {
        root.insert("selectedAccount".to_string(), Value::String(identifier));
    }

    migrate_legacy_accounts(&records, false)
}

/// Creates the stores of the accounts of today from the records of an older
/// launcher.
///
/// The records are split into the public data and the credentials, and every
/// record is given the identifier of the account of today, which is derived
/// from the record so the same record is named the same way every time.
///
/// The records of the file the older launcher shared between the workspaces
/// are named with a prefix, so an account of the shared file does not take the
/// identifier of an account of a single workspace.
pub fn migrate_legacy_accounts(accounts: &[Map<String, Value>], shared: bool) -> AccountMigration {
    let mut metadata = AccountMetadataStore::new();
    let mut private_data = AccountPrivateData::new();

    for account in accounts {
        let (mut record, credentials) = migrate_legacy_account_record(account);
        record.remove("selected");

        let id = legacy_account_record_id(&record, shared);
        record.insert("accountID".to_string(), Value::String(id.to_string()));

        if !credentials.is_empty() {
            private_data.put(id, credentials);
        }
        metadata.accounts.push(record);
    }

    AccountMigration {
        metadata,
        private_data,
    }
}

/// Splits one record of an older launcher into the public data and the
/// credentials of today.
///
/// A record whose kind the launcher of today does not know is kept as it is,
/// so a launcher of a newer version does not lose an account it wrote.
fn migrate_legacy_account_record(
    account: &Map<String, Value>,
) -> (Map<String, Value>, Map<String, Value>) {
    let Some(kind) = member_string(account, "type") else {
        return (account.clone(), Map::new());
    };

    match kind.as_str() {
        "offline" => {
            let mut record = legacy_account_metadata("offline");
            let credentials = Map::new();

            add_string_member(
                &mut record,
                "profileName",
                member_string(account, "username"),
            );
            add_normalized_uuid_member(&mut record, "profileID", member_string(account, "uuid"));
            if !record.contains_key("profileID") {
                // The identifier of a player that plays without an account is
                // derived from the name, so the record of the older launcher
                // that wrote no identifier still names the player.
                if let Some(name) = member_string(&record, "profileName") {
                    record.insert(
                        "profileID".to_string(),
                        Value::String(Uuid::offline(&name).to_string()),
                    );
                }
            }
            copy_member(account, &mut record, "skin");

            (record, credentials)
        }
        "microsoft" => {
            let mut record = legacy_account_metadata("microsoft");
            let mut credentials = Map::new();

            add_normalized_uuid_member(&mut record, "profileID", member_string(account, "uuid"));
            add_string_member(
                &mut credentials,
                "profileName",
                member_string(account, "displayName"),
            );
            copy_member(account, &mut credentials, "tokenType");
            copy_member(account, &mut credentials, "accessToken");
            copy_member(account, &mut credentials, "refreshToken");
            copy_member(account, &mut credentials, "notAfter");
            copy_member(account, &mut credentials, "userid");

            (record, credentials)
        }
        "yggdrasil" => {
            let mut record = legacy_account_metadata("yggdrasil");
            let mut credentials = Map::new();

            add_string_member(&mut record, "loginName", member_string(account, "username"));
            add_normalized_uuid_member(&mut record, "profileID", member_string(account, "uuid"));
            add_string_member(
                &mut credentials,
                "profileName",
                member_string(account, "displayName"),
            );
            copy_member(account, &mut credentials, "clientToken");
            copy_member(account, &mut credentials, "accessToken");
            copy_member(account, &mut credentials, "userProperties");

            (record, credentials)
        }
        "authlibInjector" => {
            let mut record = legacy_account_metadata("authlibInjector");
            let mut credentials = Map::new();

            copy_member(account, &mut record, "serverBaseURL");
            add_string_member(&mut record, "loginName", member_string(account, "username"));
            add_normalized_uuid_member(&mut record, "profileID", member_string(account, "uuid"));
            add_string_member(
                &mut credentials,
                "profileName",
                member_string(account, "displayName"),
            );
            copy_member(account, &mut credentials, "clientToken");
            copy_member(account, &mut credentials, "accessToken");
            copy_member(account, &mut credentials, "userProperties");
            copy_member(account, &mut credentials, "profileProperties");

            (record, credentials)
        }
        _ => (account.clone(), Map::new()),
    }
}

/// Creates the public data of an account of the given kind.
fn legacy_account_metadata(kind: &str) -> Map<String, Value> {
    let mut record = Map::new();
    record.insert("type".to_string(), Value::String(kind.to_string()));
    record
}

/// Adds a member of the source to the target when it holds a string.
fn add_string_member(target: &mut Map<String, Value>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        target.insert(name.to_string(), Value::String(value));
    }
}

/// Adds a member of the source to the target when it holds an identifier that
/// can be read.
///
/// The identifier is written the way the launcher of today writes it, so an
/// identifier the older launcher wrote without the hyphens is read.
fn add_normalized_uuid_member(target: &mut Map<String, Value>, name: &str, value: Option<String>) {
    if let Some(normalized) = value.and_then(|text| Uuid::parse(&text).ok()) {
        target.insert(name.to_string(), Value::String(normalized.to_string()));
    }
}

/// Copies a member of the source to the target, unless the member is missing
/// or holds nothing.
fn copy_member(source: &Map<String, Value>, target: &mut Map<String, Value>, name: &str) {
    if let Some(value) = source.get(name) {
        if !value.is_null() {
            target.insert(name.to_string(), value.clone());
        }
    }
}

/// Returns the identifier of the account of today a record of an older
/// launcher names.
///
/// The identifier is made of the text the older launcher named the account
/// with, so the account the player selected in the older launcher is the
/// account the launcher of today selects. A record the older launcher named
/// no account with is named by its text, so the same record still keeps its
/// identifier.
fn legacy_account_record_id(record: &Map<String, Value>, shared: bool) -> AccountId {
    let prefix = if shared {
        LEGACY_GLOBAL_ACCOUNT_PREFIX
    } else {
        ""
    };
    let identifier = legacy_identifier_of_metadata(record)
        .unwrap_or_else(|| serde_json::to_string(record).unwrap_or_default());

    AccountId::new(Uuid::generate_v5(
        LEGACY_ACCOUNT_ID_NAMESPACE,
        &format!("{prefix}{identifier}"),
    ))
}

/// Returns the text the older launcher named an account of today with.
///
/// The text tells the kind of the account and the name of the player, which
/// is what the older launcher kept in its own record of the account.
fn legacy_identifier_of_metadata(record: &Map<String, Value>) -> Option<String> {
    let kind = member_string(record, "type")?;

    match kind.as_str() {
        "offline" => {
            let name = member_string(record, "profileName")?;
            Some(format!("{name}:{name}"))
        }
        "microsoft" => {
            let id = member_string(record, "profileID")?;
            Some(format!("microsoft:{id}"))
        }
        "yggdrasil" => {
            let name = member_string(record, "loginName")?;
            let id = member_string(record, "profileID")?;
            Some(format!("{name}:{id}"))
        }
        "authlibInjector" => {
            let server = member_string(record, "serverBaseURL")?;
            let name = member_string(record, "loginName")?;
            let id = member_string(record, "profileID")?;
            Some(format!("{server}:{name}:{id}"))
        }
        _ => None,
    }
}

/// Returns the text the older launcher named a record of its own with.
///
/// The records of the older launcher keep the name of the player in other
/// members than the records of today, so the text is made of the members the
/// older launcher wrote.
fn legacy_identifier_of_storage(record: &Map<String, Value>) -> Option<String> {
    let kind = member_string(record, "type")?;

    match kind.as_str() {
        "offline" => {
            let name = member_string(record, "username")?;
            Some(format!("{name}:{name}"))
        }
        "microsoft" => {
            let id = normalize_uuid_text(&member_string(record, "uuid")?)?;
            Some(format!("microsoft:{id}"))
        }
        "yggdrasil" => {
            let name = member_string(record, "username")?;
            let id = normalize_uuid_text(&member_string(record, "uuid")?)?;
            Some(format!("{name}:{id}"))
        }
        "authlibInjector" => {
            let server = member_string(record, "serverBaseURL")?;
            let name = member_string(record, "username")?;
            let id = normalize_uuid_text(&member_string(record, "uuid")?)?;
            Some(format!("{server}:{name}:{id}"))
        }
        _ => None,
    }
}

/// Returns an identifier written the way the launcher of today writes it.
///
/// The older launcher wrote the identifier without the hyphens in some of its
/// files, so the text is read rather than used as it is.
fn normalize_uuid_text(text: &str) -> Option<String> {
    Uuid::parse(text).ok().map(|uuid| uuid.to_string())
}

/// Gives every record of the accounts the identifier of the account of today.
///
/// A record that already names an account keeps its identifier, unless another
/// store already named that account, in which case the identifier is derived
/// from the record again. Returns whether any record was changed.
pub fn assign_legacy_account_ids(
    metadata: &mut AccountMetadataStore,
    used: &mut BTreeSet<String>,
    shared: bool,
) -> bool {
    let mut changed = false;
    let mut records = Vec::with_capacity(metadata.accounts.len());

    for record in &metadata.accounts {
        let mut record = record.clone();
        let existing = AccountMetadataStore::account_id(&record);

        match existing {
            Some(id) if used.insert(id.to_string()) => {
                let text = id.to_string();
                changed |= record.get("accountID").and_then(Value::as_str) != Some(text.as_str());
                record.insert("accountID".to_string(), Value::String(text));
            }
            _ => {
                let id = legacy_account_record_id(&record, shared);
                used.insert(id.to_string());
                changed |= record.get("accountID").and_then(Value::as_str)
                    != Some(id.to_string().as_str());
                record.insert("accountID".to_string(), Value::String(id.to_string()));
            }
        }

        records.push(record);
    }

    if changed {
        metadata.accounts = records;
    }
    changed
}

/// Converts the version the player selected for every profile into the
/// selected instance of the game directory of the profile.
fn migrate_legacy_selected_versions(root: &mut Map<String, Value>) -> bool {
    let Some(configurations) = root.get("configurations").and_then(Value::as_object) else {
        return false;
    };

    let mut selected = Map::new();
    for (name, profile) in configurations {
        let Some(profile) = profile.as_object() else {
            continue;
        };
        let Some(version) = member_string(profile, "selectedMinecraftVersion") else {
            continue;
        };
        if version.trim().is_empty() {
            continue;
        }

        selected.insert(legacy_profile_id(name).to_string(), Value::String(version));
    }

    if selected.is_empty() {
        return false;
    }

    root.insert("selectedInstance".to_string(), Value::Object(selected));
    true
}

/// Converts the profile the player selected into the game directory of today.
fn migrate_legacy_selected_game_directory(root: &mut Map<String, Value>) -> bool {
    let Some(last) = root.remove("last") else {
        return false;
    };

    let Some(name) = last.as_str() else {
        return true;
    };
    if name.trim().is_empty() || !has_legacy_profile(root, name) {
        return true;
    }

    root.insert(
        "selectedGameDirectory".to_string(),
        Value::String(legacy_profile_id(name).to_string()),
    );
    true
}

/// Returns whether the file of the older launcher names a profile.
fn has_legacy_profile(root: &Map<String, Value>, name: &str) -> bool {
    root.get("configurations")
        .and_then(Value::as_object)
        .and_then(|configurations| configurations.get(name))
        .is_some_and(Value::is_object)
}

/// Takes the game directories out of the settings of the launcher.
///
/// The older launcher kept a profile for every directory the player plays in;
/// the launcher of today keeps the directories in
/// `config/game-directories.json`. The name of a profile that the launcher of
/// Java gives itself is not kept, so the directory is named by its path, which
/// is what the player sees.
fn extract_game_directories(root: &mut Map<String, Value>) -> Vec<GameDirectory> {
    let Some(configurations) = root.remove("configurations") else {
        return Vec::new();
    };
    let Some(configurations) = configurations.as_object() else {
        return Vec::new();
    };

    let mut directories = Vec::new();
    for (name, profile) in configurations {
        let Some(profile) = profile.as_object() else {
            continue;
        };

        let path = member_string(profile, "gameDir")
            .or_else(|| member_string(profile, "path"))
            .unwrap_or_default();

        let mut directory = GameDirectory::new(legacy_profile_id(name), PortablePath::of(path));
        if !is_built_in_profile_name(name) {
            directory.name = Some(LocalizedText::plain(name.clone()));
        }
        if profile.get("global").is_some_and(Value::is_object) {
            directory.legacy_game_settings = Some(legacy_game_settings_id(name));
        }

        directories.push(directory);
    }

    directories
}

/// Returns whether a name is the name the launcher of Java gives a profile.
fn is_built_in_profile_name(name: &str) -> bool {
    name == LEGACY_DEFAULT_PROFILE || name == LEGACY_HOME_PROFILE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setting::game_settings::DefaultIsolationType;
    use crate::setting::launcher::{NetworkBackgroundImageCachePolicy, ThemeColorType};

    /// Returns the settings of a legacy file.
    fn migrate(text: &str) -> LegacyConfigMigration {
        migrate_legacy_config_text("hmcl.json", text).expect("the settings should be migrated")
    }

    /// Returns the settings of a legacy file as JSON.
    fn settings_json(migration: &LegacyConfigMigration) -> Value {
        serde_json::to_value(&migration.settings).expect("the settings should be written")
    }

    #[test]
    fn a_file_of_a_newer_launcher_is_not_migrated() {
        assert!(migrate_legacy_config_text("hmcl.json", "{\"_version\": 3}").is_none());
        assert!(migrate_legacy_config_text("hmcl.json", "not json").is_none());
        assert!(migrate_legacy_config_text("hmcl.json", "[]").is_none());
    }

    #[test]
    fn a_file_without_a_version_is_read_as_the_last_legacy_version() {
        let migration = migrate(r#"{"language": "en"}"#);

        assert_eq!(migration.settings.language.as_deref(), Some("en"));
        assert_eq!(migration.path, "hmcl.json");
    }

    #[test]
    fn the_names_of_the_fields_of_an_older_launcher_are_converted() {
        let migration = migrate(
            r##"{
                "_version": 2,
                "themeBrightness": "dark",
                "theme": "#336699",
                "titleTransparent": true,
                "backgroundImage": "/pictures/background.png",
                "backgroundImageUrl": "https://example.com/background.png",
                "backgroundPaint": "#336699"
            }"##,
        );

        let settings = &migration.settings;
        assert_eq!(settings.theme_brightness_mode, "dark");
        assert_eq!(settings.custom_theme_color.as_deref(), Some("#336699"));
        assert_eq!(settings.theme_color_type, ThemeColorType::Custom);
        assert!(settings.title_bar_transparent);
        assert!(settings.window_transparent);
        assert_eq!(
            settings.custom_background_image_path.as_deref(),
            Some("/pictures/background.png")
        );
        assert_eq!(
            settings.network_background_image_url.as_deref(),
            Some("https://example.com/background.png")
        );
        assert_eq!(
            settings.network_background_image_cache_policy,
            NetworkBackgroundImageCachePolicy::Disabled
        );
        assert_eq!(settings.custom_background_paint.as_deref(), Some("#336699"));

        // The parts of the theme the player changed are kept, so the launcher
        // does not replace them with the ones of the theme.
        for key in [
            THEME_APPEARANCE_BACKGROUND,
            THEME_APPEARANCE_BRIGHTNESS_MODE,
            THEME_APPEARANCE_COLOR,
            THEME_APPEARANCE_TITLE_BAR_TRANSPARENT,
            THEME_APPEARANCE_WINDOW_TRANSPARENT,
        ] {
            assert!(
                settings
                    .theme_appearance_overrides
                    .iter()
                    .any(|name| name == key),
                "{key} should be overridden"
            );
        }

        // The names of the older launcher are not written again.
        let json = settings_json(&migration);
        for name in [
            "backgroundImage",
            "backgroundImageUrl",
            "backgroundPaint",
            "themeBrightness",
            "theme",
            "titleTransparent",
        ] {
            assert!(json.get(name).is_none(), "{name} should not be written");
        }
    }

    #[test]
    fn the_classic_background_becomes_a_builtin_wallpaper() {
        let migration = migrate(r#"{"_version": 2, "backgroundType": "CLASSIC"}"#);

        assert_eq!(migration.settings.background_type, BackgroundType::Builtin);
        assert_eq!(migration.settings.builtin_background_id, "2016-02-25");
        assert!(migration
            .settings
            .theme_appearance_overrides
            .iter()
            .any(|name| name == THEME_APPEARANCE_BACKGROUND));
    }

    #[test]
    fn the_numbered_backgrounds_are_named() {
        // The third background the older launcher numbered was the wallpaper
        // of 2016.
        let migration = migrate(r#"{"_version": 2, "backgroundType": 2}"#);
        assert_eq!(migration.settings.background_type, BackgroundType::Builtin);
        assert_eq!(migration.settings.builtin_background_id, "2016-02-25");

        // The translucent background is a painted one by now.
        let migration = migrate(r#"{"_version": 2, "backgroundType": 4}"#);
        assert_eq!(migration.settings.background_type, BackgroundType::Paint);
        assert_eq!(
            migration.settings.custom_background_paint.as_deref(),
            Some("#ffffff")
        );
        assert_eq!(migration.settings.background_opacity, 0.5);

        // The first background is the one the launcher chooses by itself. The
        // launcher of Java still marks the background as a part of the theme
        // the player changed, so the launcher of today does the same.
        let migration = migrate(r#"{"_version": 2, "backgroundType": 0}"#);
        assert_eq!(migration.settings.background_type, BackgroundType::Default);
        assert!(migration
            .settings
            .theme_appearance_overrides
            .iter()
            .any(|name| name == THEME_APPEARANCE_BACKGROUND));
    }

    #[test]
    fn the_opacity_of_the_background_is_read_as_a_share() {
        let migration = migrate(r#"{"_version": 2, "bgImageOpacity": 60}"#);
        assert!((migration.settings.background_opacity - 0.6).abs() < 1e-9);
        assert!(migration
            .settings
            .theme_appearance_overrides
            .iter()
            .any(|name| name == THEME_APPEARANCE_BACKGROUND_OPACITY));

        // A share the player could not see is clamped, and the number of the
        // older launcher may be written as text.
        let migration = migrate(r#"{"_version": 2, "bgImageOpacity": "250"}"#);
        assert!((migration.settings.background_opacity - 1.0).abs() < 1e-9);

        // A value the launcher cannot read is no opacity at all.
        let migration = migrate(r#"{"_version": 2, "bgImageOpacity": "wide"}"#);
        assert_eq!(migration.settings.background_opacity, 1.0);
        assert!(!migration
            .settings
            .theme_appearance_overrides
            .iter()
            .any(|name| name == THEME_APPEARANCE_BACKGROUND_OPACITY));
    }

    #[test]
    fn the_proxy_of_the_older_launcher_is_read() {
        // A player who does not use a proxy uses the one of the system.
        let migration = migrate(r#"{"_version": 2, "hasProxy": false}"#);
        assert_eq!(migration.settings.proxy_type, ProxyType::System);

        // The proxy of Java was numbered: direct, HTTP and SOCKS.
        let migration = migrate(r#"{"_version": 2, "hasProxy": true, "proxyType": 2}"#);
        assert_eq!(migration.settings.proxy_type, ProxyType::Socks);

        let migration = migrate(r#"{"_version": 2, "hasProxy": true, "proxyType": 0}"#);
        assert_eq!(migration.settings.proxy_type, ProxyType::Direct);

        // A number the launcher of Java does not know is an HTTP proxy.
        let migration = migrate(r#"{"_version": 2, "hasProxy": true, "proxyType": 7}"#);
        assert_eq!(migration.settings.proxy_type, ProxyType::Http);

        // A name the launcher of today does not know is an HTTP proxy too.
        let migration = migrate(r#"{"_version": 2, "hasProxy": true, "proxyType": "ftp"}"#);
        assert_eq!(migration.settings.proxy_type, ProxyType::Http);

        // A name the launcher of today knows is kept.
        let migration = migrate(r#"{"_version": 2, "hasProxy": true, "proxyType": "SOCKS"}"#);
        assert_eq!(migration.settings.proxy_type, ProxyType::Socks);

        // The user name of the proxy was named another way.
        let migration = migrate(
            r#"{"_version": 2, "hasProxy": true, "proxyType": 1,
                "proxyHost": "proxy.example.com", "proxyPort": 8080,
                "proxyUserName": "player"}"#,
        );
        assert_eq!(
            migration.settings.proxy_host.as_deref(),
            Some("proxy.example.com")
        );
        assert_eq!(migration.settings.proxy_port, Some(8080));
        assert_eq!(migration.settings.proxy_user.as_deref(), Some("player"));
    }

    #[test]
    fn the_sources_of_the_downloads_are_read() {
        // A player who let the launcher choose used the default source.
        let migration = migrate(r#"{"_version": 2, "autoChooseDownloadType": true}"#);
        assert_eq!(
            migration.settings.version_list_source,
            DownloadSource::Default
        );
        assert_eq!(
            migration.settings.file_download_source,
            DownloadSource::Default
        );

        // A player who chose the source used the one they chose, whatever the
        // automatic choice says.
        let migration = migrate(
            r#"{"_version": 2, "autoChooseDownloadType": false, "downloadType": "bmclapi"}"#,
        );
        assert_eq!(
            migration.settings.version_list_source,
            DownloadSource::Mirror
        );
        assert_eq!(
            migration.settings.file_download_source,
            DownloadSource::Mirror
        );

        // The source of the list of the versions is read when the launcher
        // chose the source by itself.
        let migration = migrate(
            r#"{"_version": 2, "autoChooseDownloadType": true, "versionListSource": "mojang"}"#,
        );
        assert_eq!(
            migration.settings.version_list_source,
            DownloadSource::Official
        );

        // A source the launcher of today does not know is the source the
        // launcher chooses by itself, which is the one a file that does not
        // turn the automatic choice off gets.
        let migration = migrate(r#"{"_version": 2, "downloadType": "somewhere"}"#);
        assert_eq!(
            migration.settings.version_list_source,
            DownloadSource::Default
        );

        // A file that turns the automatic choice off and names no source uses
        // the official one.
        let migration = migrate(r#"{"_version": 2, "autoChooseDownloadType": false}"#);
        assert_eq!(
            migration.settings.version_list_source,
            DownloadSource::Official
        );
    }

    #[test]
    fn the_language_of_the_older_launcher_is_named_the_way_of_today() {
        let migration = migrate(r#"{"_version": 2, "localization": "zh_CN"}"#);
        assert_eq!(migration.settings.language.as_deref(), Some("zh-Hans"));

        let migration = migrate(r#"{"_version": 2, "localization": "zh"}"#);
        assert_eq!(migration.settings.language.as_deref(), Some("zh-Hant"));

        let migration = migrate(r#"{"_version": 2, "localization": "en"}"#);
        assert_eq!(migration.settings.language.as_deref(), Some("en"));
    }

    #[test]
    fn the_window_of_the_older_launcher_is_taken_out_of_the_settings() {
        let migration = migrate(
            r#"{"_version": 2, "x": 100, "y": 200, "width": 1216, "height": 716,
                "promptedVersion": "3.5.0", "shownTips": {"welcome": true}}"#,
        );

        let state = &migration.detached.launcher_state;
        // The size of the older launcher included the shadow of the window.
        assert_eq!(state.width(), 1200.0);
        assert_eq!(state.height(), 700.0);
        assert_eq!(state.prompted_version(), Some("3.5.0"));
        assert!(state.is_tip_shown("welcome"));

        // The place of the window is not a setting of the launcher.
        let json = settings_json(&migration);
        for name in ["x", "y", "width", "height", "promptedVersion", "shownTips"] {
            assert!(json.get(name).is_none(), "{name} should not be a setting");
        }
    }

    #[test]
    fn the_servers_of_the_authlib_injector_are_taken_out_of_the_settings() {
        // A file that does not say that the player removed LittleSkin names it.
        let migration = migrate(r#"{"_version": 2}"#);
        assert!(migration
            .detached
            .authlib_injector_servers
            .get("https://littleskin.cn/api/yggdrasil/")
            .is_some());

        // A player who removed it does not get it back.
        let migration = migrate(r#"{"_version": 2, "addedLittleSkin": true}"#);
        assert!(migration
            .detached
            .authlib_injector_servers
            .get("https://littleskin.cn/api/yggdrasil/")
            .is_none());

        // The servers the player named are read.
        let migration = migrate(
            r#"{"_version": 2, "addedLittleSkin": true,
                "authlibInjectorServers": [{"url": "https://example.com/api/yggdrasil/"}]}"#,
        );
        assert!(migration
            .detached
            .authlib_injector_servers
            .get("https://example.com/api/yggdrasil/")
            .is_some());
        assert!(settings_json(&migration)
            .get("authlibInjectorServers")
            .is_none());
    }

    #[test]
    fn the_profiles_of_the_older_launcher_become_game_directories() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "last": "My games",
                "configurations": {
                    "Default": {"gameDir": "/games/.minecraft", "selectedMinecraftVersion": "1.20.1"},
                    "Home": {"gameDir": "/home/user/.minecraft"},
                    "My games": {"path": "/games/shared", "global": {}}
                }
            }"#,
        );

        let directories = &migration.detached.game_directories;
        assert_eq!(directories.len(), 3);

        let default = &directories[0];
        assert_eq!(default.id, legacy_profile_id("Default"));
        assert_eq!(default.path.path(), "/games/.minecraft");
        // The name the launcher of Java gives the profile is not a name the
        // player chose, so the directory is named by its path.
        assert_eq!(default.name, None);
        assert_eq!(default.legacy_game_settings, None);

        let named = &directories[2];
        assert_eq!(named.name, Some(LocalizedText::plain("My games")));
        assert_eq!(named.path.path(), "/games/shared");
        assert_eq!(
            named.legacy_game_settings,
            Some(legacy_game_settings_id("My games"))
        );

        // The profile the player selected is the directory the launcher opens.
        assert_eq!(
            migration.settings.selected_game_directory.as_deref(),
            Some(legacy_profile_id("My games").to_string().as_str())
        );
        // The version of every profile is the instance the player selected.
        assert_eq!(
            migration
                .settings
                .selected_instance
                .get(&legacy_profile_id("Default").to_string()),
            Some(&"1.20.1".to_string())
        );

        // The profiles are not a setting of the launcher.
        assert!(settings_json(&migration).get("configurations").is_none());
        assert!(settings_json(&migration).get("last").is_none());
    }

    #[test]
    fn the_settings_of_a_profile_become_a_preset() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "configurations": {
                    "Default": {"gameDir": "/games/.minecraft", "maxMemory": 2048},
                    "My games": {
                        "path": "/games/shared",
                        "global": {"maxMemory": 8192, "gameDirType": 1}
                    }
                }
            }"#,
        );

        let presets = &migration.detached.presets;
        // Only the profile that kept settings of the game becomes a preset.
        assert_eq!(presets.presets.len(), 1);

        let preset = &presets.presets[0];
        assert_eq!(preset.id, legacy_game_settings_id("My games"));
        assert_eq!(preset.auto_name_number, Some(1));
        assert_eq!(preset.settings.max_memory, Some(8192));
        // The profile kept the files of every instance apart.
        assert_eq!(
            preset.default_isolation_type,
            Some(DefaultIsolationType::Always)
        );
    }

    #[test]
    fn the_switches_of_the_whole_launcher_become_a_preset() {
        // A launcher that never named a preset gets one, so the switches the
        // player turned off stay off.
        let migration = migrate(
            r#"{
                "_version": 2,
                "allowAutoAgent": true,
                "disableAutoGameOptions": true
            }"#,
        );

        let presets = &migration.detached.presets;
        assert_eq!(presets.presets.len(), 1);
        assert_eq!(presets.presets[0].settings.allow_auto_agent, Some(true));
        assert_eq!(
            presets.presets[0].settings.disable_auto_game_options,
            Some(true)
        );

        // A file that does not turn them on adds no preset.
        let migration = migrate(r#"{"_version": 2, "allowAutoAgent": false}"#);
        assert!(migration.detached.presets.presets.is_empty());
    }

    #[test]
    fn the_switches_are_read_into_every_preset() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "allowAutoAgent": true,
                "configurations": {
                    "Default": {"gameDir": "/games/.minecraft"},
                    "My games": {"path": "/games/shared", "global": {"maxMemory": 8192}}
                }
            }"#,
        );

        let presets = &migration.detached.presets;
        assert_eq!(presets.presets.len(), 1);
        assert_eq!(presets.presets[0].settings.allow_auto_agent, Some(true));
        assert_eq!(presets.presets[0].settings.disable_auto_game_options, None);
    }

    #[test]
    fn the_migrated_presets_are_written_to_their_own_file() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-legacy-preset-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        std::fs::write(
            directory.join(LEGACY_CONFIG_FILENAME),
            r#"{
                "_version": 2,
                "configurations": {
                    "My games": {"path": "/games/shared", "global": {"maxMemory": 8192}}
                }
            }"#,
        )
        .expect("the file should be written");

        migrate_legacy_settings(&directory, &directory)
            .expect("the migration should run")
            .expect("the file should be migrated");

        let presets = GameSettingsPresets::load(&directory).expect("the presets should be read");
        assert_eq!(presets.settings().presets.len(), 1);
        assert_eq!(
            presets.settings().presets[0].settings.max_memory,
            Some(8192)
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_account_of_the_older_launcher_is_named_the_way_of_today() {
        let migration = migrate(r#"{"_version": 2, "selectedAccount": "Notch:Notch"}"#);

        assert_eq!(
            migration.settings.selected_account.as_deref(),
            Some(legacy_account_id("Notch:Notch").as_str())
        );

        // A player who selected no account selects none.
        let migration = migrate(r#"{"_version": 2, "selectedAccount": "  "}"#);
        assert_eq!(migration.settings.selected_account, None);
    }

    #[test]
    fn the_accounts_of_the_launcher_of_2_are_read() {
        let migration = migrate(
            r#"{
                "_version": 0,
                "auth": {"offline": {
                    "IAuthenticator_UserName": "Alex",
                    "uuidMap": {"Alex": "00000000000000000000000000000001"}
                }}
            }"#,
        );

        // The account the player used is the account the launcher selects.
        assert_eq!(
            migration.settings.selected_account.as_deref(),
            Some(legacy_account_id("Alex:Alex").as_str())
        );
    }

    /// Returns the record of an account of a legacy file.
    fn account_record(migration: &LegacyConfigMigration, index: usize) -> &Map<String, Value> {
        &migration.detached.accounts.metadata.accounts[index]
    }

    #[test]
    fn an_account_that_plays_without_an_account_is_read() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [
                    {"type": "offline", "username": "Steve", "selected": true}
                ]
            }"#,
        );

        let accounts = &migration.detached.accounts;
        assert_eq!(accounts.metadata.accounts.len(), 1);
        assert!(accounts.private_data.is_empty());

        let record = account_record(&migration, 0);
        assert_eq!(member_string(record, "type").as_deref(), Some("offline"));
        assert_eq!(
            member_string(record, "profileName").as_deref(),
            Some("Steve")
        );
        assert_eq!(
            member_string(record, "profileID").as_deref(),
            Some(Uuid::offline("Steve").to_string().as_str())
        );
        assert_eq!(
            member_string(record, "accountID").as_deref(),
            Some(legacy_account_id("Steve:Steve").as_str())
        );

        // The member that marked the account as selected belongs to the
        // older launcher and is not written by the launcher of today.
        assert!(!record.contains_key("selected"));
    }

    #[test]
    fn an_account_that_plays_without_an_account_keeps_its_identifier() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [
                    {"type": "offline", "username": "Steve",
                     "uuid": "B50AD385-829D-3141-A216-7E7D7539BA7F",
                     "skin": {"model": "slim", "texture": "abc"}}
                ]
            }"#,
        );

        let record = account_record(&migration, 0);
        // The identifier is written the way the launcher of today writes it.
        assert_eq!(
            member_string(record, "profileID").as_deref(),
            Some("b50ad385-829d-3141-a216-7e7d7539ba7f")
        );
        assert_eq!(
            record.get("skin").and_then(|skin| skin.get("model")),
            Some(&Value::String("slim".to_string()))
        );
    }

    #[test]
    fn the_credentials_of_a_microsoft_account_are_kept_apart() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [
                    {"type": "microsoft", "uuid": "b50ad385-829d-3141-a216-7e7d7539ba7f",
                     "displayName": "Steve", "tokenType": "Bearer",
                     "accessToken": "access", "refreshToken": "refresh",
                     "notAfter": 1700000000000, "userid": "user"}
                ]
            }"#,
        );

        let accounts = &migration.detached.accounts;
        let record = account_record(&migration, 0);
        assert_eq!(member_string(record, "type").as_deref(), Some("microsoft"));
        assert_eq!(
            member_string(record, "profileID").as_deref(),
            Some("b50ad385-829d-3141-a216-7e7d7539ba7f")
        );

        // The public data names the account but holds no credential.
        assert!(!record.contains_key("accessToken"));
        assert!(!record.contains_key("refreshToken"));
        assert!(!record.contains_key("displayName"));

        let id = AccountMetadataStore::account_id(record).expect("the record names an account");
        let credentials = accounts
            .private_data
            .get(id)
            .expect("the credentials should be kept");
        assert_eq!(
            member_string(credentials, "profileName").as_deref(),
            Some("Steve")
        );
        assert_eq!(
            member_string(credentials, "accessToken").as_deref(),
            Some("access")
        );
        assert_eq!(
            member_string(credentials, "refreshToken").as_deref(),
            Some("refresh")
        );
        assert_eq!(
            member_string(credentials, "tokenType").as_deref(),
            Some("Bearer")
        );
        assert_eq!(
            member_string(credentials, "userid").as_deref(),
            Some("user")
        );
        assert_eq!(
            credentials.get("notAfter"),
            Some(&Value::Number(1_700_000_000_000i64.into()))
        );

        // The account the player selected is named the way the older
        // launcher named the account of Microsoft.
        assert_eq!(
            member_string(record, "accountID").as_deref(),
            Some("account-66685133-b180-55d6-bb36-aedaf092ef64")
        );
    }

    #[test]
    fn a_yggdrasil_account_keeps_its_login_name_and_its_credentials() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [
                    {"type": "yggdrasil", "username": "player@example.com",
                     "uuid": "b50ad385-829d-3141-a216-7e7d7539ba7f",
                     "displayName": "Steve", "clientToken": "client",
                     "accessToken": "access",
                     "userProperties": {"preferredLanguage": "en"}}
                ]
            }"#,
        );

        let record = account_record(&migration, 0);
        assert_eq!(member_string(record, "type").as_deref(), Some("yggdrasil"));
        assert_eq!(
            member_string(record, "loginName").as_deref(),
            Some("player@example.com")
        );
        assert!(!record.contains_key("clientToken"));

        let id = AccountMetadataStore::account_id(record).expect("the record names an account");
        let credentials = migration
            .detached
            .accounts
            .private_data
            .get(id)
            .expect("the credentials should be kept");
        assert_eq!(
            member_string(credentials, "clientToken").as_deref(),
            Some("client")
        );
        assert_eq!(
            member_string(credentials, "accessToken").as_deref(),
            Some("access")
        );
        assert!(credentials.contains_key("userProperties"));

        assert_eq!(
            member_string(record, "accountID").as_deref(),
            Some("account-af378ac7-2fd2-551b-a946-1343254078b3")
        );
    }

    #[test]
    fn an_authlib_injector_account_keeps_the_server_it_belongs_to() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [
                    {"type": "authlibInjector",
                     "serverBaseURL": "https://example.com/api/yggdrasil",
                     "username": "player",
                     "uuid": "b50ad385-829d-3141-a216-7e7d7539ba7f",
                     "displayName": "Steve", "accessToken": "access",
                     "profileProperties": {"textures": "x"}}
                ]
            }"#,
        );

        let record = account_record(&migration, 0);
        assert_eq!(
            member_string(record, "type").as_deref(),
            Some("authlibInjector")
        );
        assert_eq!(
            member_string(record, "serverBaseURL").as_deref(),
            Some("https://example.com/api/yggdrasil")
        );
        assert_eq!(
            member_string(record, "loginName").as_deref(),
            Some("player")
        );
        assert_eq!(
            member_string(record, "accountID").as_deref(),
            Some("account-94ce38e7-d2f7-5a60-bc24-f45c3a6761c1")
        );

        let id = AccountMetadataStore::account_id(record).expect("the record names an account");
        let credentials = migration
            .detached
            .accounts
            .private_data
            .get(id)
            .expect("the credentials should be kept");
        assert!(credentials.contains_key("profileProperties"));
    }

    #[test]
    fn an_account_of_a_kind_the_launcher_does_not_know_is_kept() {
        // A launcher of a newer version wrote the record, so it is kept as it
        // is rather than dropped.
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [
                    {"type": "future", "name": "x", "token": "secret"},
                    {"name": "no kind at all"}
                ]
            }"#,
        );

        let accounts = &migration.detached.accounts;
        assert_eq!(accounts.metadata.accounts.len(), 2);
        assert!(accounts.private_data.is_empty());

        // The record names the account it describes, so it is not lost.
        for index in 0..2 {
            assert!(AccountMetadataStore::account_id(account_record(&migration, index)).is_some());
        }
        assert_eq!(
            member_string(account_record(&migration, 0), "token").as_deref(),
            Some("secret")
        );
    }

    #[test]
    fn the_account_the_record_marks_as_selected_is_selected() {
        // The older launcher wrote no selected account when the player picked
        // one in the list, so the record itself marks the account.
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [
                    {"type": "offline", "username": "Alex"},
                    {"type": "offline", "username": "Steve", "selected": true}
                ]
            }"#,
        );

        assert_eq!(
            migration.settings.selected_account.as_deref(),
            Some(legacy_account_id("Steve:Steve").as_str())
        );
    }

    #[test]
    fn the_account_the_file_names_is_selected() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "selectedAccount": "microsoft:b50ad385-829d-3141-a216-7e7d7539ba7f",
                "accounts": [
                    {"type": "microsoft", "uuid": "b50ad385-829d-3141-a216-7e7d7539ba7f",
                     "selected": true}
                ]
            }"#,
        );

        // The account the file names wins over the record that marks itself
        // as selected, and the two name the same account here.
        assert_eq!(
            migration.settings.selected_account.as_deref(),
            Some("account-66685133-b180-55d6-bb36-aedaf092ef64")
        );
    }

    #[test]
    fn the_accounts_are_taken_out_of_the_settings() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [{"type": "offline", "username": "Steve"}],
                "selectedAccount": "Steve:Steve"
            }"#,
        );

        // The launcher of today keeps the accounts in files of their own, so
        // the settings hold none of them.
        let settings = settings_json(&migration);
        assert!(!settings
            .as_object()
            .expect("an object")
            .contains_key("accounts"));
        assert_eq!(
            migration.settings.selected_account.as_deref(),
            Some(legacy_account_id("Steve:Steve").as_str())
        );
    }

    #[test]
    fn a_shared_account_is_named_apart_from_the_account_of_a_workspace() {
        let records = vec![serde_json::json!({
            "type": "offline",
            "username": "Steve"
        })
        .as_object()
        .expect("an object")
        .clone()];

        let local = migrate_legacy_accounts(&records, false);
        let shared = migrate_legacy_accounts(&records, true);

        // The account of the file every workspace shares does not take the
        // identifier of the account of a single workspace.
        assert_eq!(
            member_string(&local.metadata.accounts[0], "accountID").as_deref(),
            Some("account-39af6455-df88-58e1-a0f1-5a5100699178")
        );
        assert_eq!(
            member_string(&shared.metadata.accounts[0], "accountID").as_deref(),
            Some("account-f492f253-ef2a-509e-af7b-45c68e8cd041")
        );
    }

    #[test]
    fn the_identifier_of_an_account_is_the_one_of_the_launcher_of_java() {
        // The identifiers were made by the launcher of Java from the same
        // records, so an account is named the same way by both launchers.
        let records: Vec<Map<String, Value>> = vec![
            serde_json::json!({"type": "offline", "username": "Notch"})
                .as_object()
                .expect("an object")
                .clone(),
            serde_json::json!({"type": "microsoft",
                "uuid": "b50ad385829d3141a2167e7d7539ba7f"})
            .as_object()
            .expect("an object")
            .clone(),
        ];
        let migration = migrate_legacy_accounts(&records, false);

        assert_eq!(
            member_string(&migration.metadata.accounts[0], "accountID").as_deref(),
            Some("account-8822e492-9d3d-5feb-a657-e2d657a8f051")
        );
        // The identifier the older launcher wrote without the hyphens is read
        // the way the launcher of today writes it.
        assert_eq!(
            member_string(&migration.metadata.accounts[1], "profileID").as_deref(),
            Some("b50ad385-829d-3141-a216-7e7d7539ba7f")
        );
        assert_eq!(
            member_string(&migration.metadata.accounts[1], "accountID").as_deref(),
            Some("account-66685133-b180-55d6-bb36-aedaf092ef64")
        );
    }

    #[test]
    fn an_identifier_that_cannot_be_read_names_no_player() {
        // The identifier is dropped rather than kept broken, and the account
        // that plays without an account is named by its name instead.
        let migration = migrate(
            r#"{
                "_version": 2,
                "accounts": [
                    {"type": "offline", "username": "Steve", "uuid": "not-a-uuid"},
                    {"type": "microsoft", "uuid": "not-a-uuid"}
                ]
            }"#,
        );

        let offline = account_record(&migration, 0);
        assert_eq!(
            member_string(offline, "profileID").as_deref(),
            Some(Uuid::offline("Steve").to_string().as_str())
        );

        let microsoft = account_record(&migration, 1);
        assert!(!microsoft.contains_key("profileID"));
        // The account is still kept and named, so the player can sign in
        // again rather than losing the record.
        assert!(AccountMetadataStore::account_id(microsoft).is_some());
    }

    #[test]
    fn the_identifiers_of_the_records_of_today_are_kept() {
        let mut metadata = AccountMetadataStore::from_records(vec![serde_json::json!({
            "accountID": "account-0199a3b4-1111-7222-8333-444444444444",
            "type": "offline",
            "profileName": "Steve"
        })
        .as_object()
        .expect("an object")
        .clone()]);

        let mut used = BTreeSet::new();
        let changed = assign_legacy_account_ids(&mut metadata, &mut used, false);

        // The record already names an account, so it keeps its identifier.
        assert!(!changed);
        assert_eq!(
            member_string(&metadata.accounts[0], "accountID").as_deref(),
            Some("account-0199a3b4-1111-7222-8333-444444444444")
        );
        assert_eq!(used.len(), 1);
    }

    #[test]
    fn an_account_that_two_stores_name_is_named_again() {
        // The account is already named by a store that was read before, so
        // the second store gives it another identifier rather than two
        // records that name the same account.
        let mut metadata = AccountMetadataStore::from_records(vec![serde_json::json!({
            "accountID": "account-0199a3b4-1111-7222-8333-444444444444",
            "type": "offline",
            "profileName": "Steve"
        })
        .as_object()
        .expect("an object")
        .clone()]);

        let mut used = BTreeSet::new();
        used.insert("account-0199a3b4-1111-7222-8333-444444444444".to_string());
        let changed = assign_legacy_account_ids(&mut metadata, &mut used, false);

        assert!(changed);
        assert_eq!(
            member_string(&metadata.accounts[0], "accountID").as_deref(),
            Some("account-39af6455-df88-58e1-a0f1-5a5100699178")
        );
        assert_eq!(used.len(), 2);
    }

    #[test]
    fn the_settings_of_the_user_are_taken_out_of_the_settings_of_the_launcher() {
        let migration = migrate(
            r#"{
                "_version": 2,
                "logRetention": 5,
                "enableOfflineAccount": false,
                "fontAntiAliasing": "gray",
                "userJava": ["/java/21"],
                "disabledJava": ["/java/8"]
            }"#,
        );

        let user = &migration.user_settings;
        assert_eq!(user.log_retention, Some(5));
        assert!(!user.enable_offline_account);
        assert_eq!(user.font_anti_aliasing.as_deref(), Some("gray"));
        assert_eq!(user.user_java, vec!["/java/21".to_string()]);
        assert_eq!(user.disabled_java, vec!["/java/8".to_string()]);
    }

    #[test]
    fn the_identifiers_of_the_migrated_records_are_stable() {
        // The identifiers are the ones the launcher of Java makes, so a player
        // who uses both launchers finds the same directory and the same
        // preset.
        assert_eq!(
            legacy_profile_id("Default").to_string(),
            "game-directory:7105bc1f-490e-5e8c-878c-f5844c3d4bc3"
        );
        assert_eq!(
            legacy_game_settings_id("Default").to_string(),
            "game-settings-preset:6daf91a8-0ce7-5347-9a0d-35f886a85679"
        );
        assert_eq!(
            legacy_account_id("Notch:Notch"),
            "account-8822e492-9d3d-5feb-a657-e2d657a8f051"
        );
    }

    #[test]
    fn the_receipt_records_the_text_of_the_file() {
        let text = r#"{"_version": 2}"#;
        let receipt = migration_receipt(text, "2026-10-24T12:00:00Z");

        assert!(migration_receipt_matches(&receipt, text));
        // A file that was written again is migrated again.
        assert!(!migration_receipt_matches(&receipt, r#"{"_version": 2 }"#));
        // A receipt the launcher cannot read is no record at all.
        assert!(!migration_receipt_matches("not json", text));
        assert!(!migration_receipt_matches("{}", text));
    }

    #[test]
    fn the_legacy_file_is_looked_for_under_both_names() {
        let directory = std::env::temp_dir().join("hmcl-legacy-config-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        assert!(!has_legacy_config(&directory));

        let path = directory.join(LEGACY_CONFIG_FILENAME_LINUX);
        std::fs::write(&path, r#"{"_version": 2}"#).expect("the file should be written");
        assert_eq!(locate_legacy_config(&directory), Some(path.clone()));
        assert_eq!(
            migrate_legacy_config(&path)
                .expect("the settings should be migrated")
                .path,
            path.to_string_lossy()
        );

        // The name the launcher of today writes wins over the other one.
        let current = directory.join(LEGACY_CONFIG_FILENAME);
        std::fs::write(&current, r#"{"_version": 2}"#).expect("the file should be written");
        assert_eq!(locate_legacy_config(&directory), Some(current));

        std::fs::remove_dir_all(&directory).expect("the directory should be removed");
    }

    #[test]
    fn the_migration_writes_the_files_once() {
        let directory = std::env::temp_dir().join("hmcl-legacy-settings-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        std::fs::write(
            directory.join(LEGACY_CONFIG_FILENAME),
            r#"{
                "_version": 2,
                "localization": "zh_CN",
                "width": 1216,
                "height": 716,
                "configurations": {"Default": {"gameDir": "/games/.minecraft"}}
            }"#,
        )
        .expect("the file should be written");

        let migration = migrate_legacy_settings(&directory, &directory)
            .expect("the migration should run")
            .expect("the file should be migrated");
        assert_eq!(migration.settings.language.as_deref(), Some("zh-Hans"));
        assert_eq!(migration.detached.game_directories.len(), 1);

        // The files of the launcher of today are written.
        let settings = crate::setting::SettingsFile::<LauncherSettings>::load_with_schema(
            crate::setting::launcher_settings_path(&directory),
            &crate::setting::Schema::new(
                "launcher-settings",
                crate::setting::SchemaVersion::new(1, 0, 0),
            ),
        )
        .expect("the settings should be read");
        assert_eq!(settings.settings().language.as_deref(), Some("zh-Hans"));

        let state = LauncherState::load(&directory).expect("the state should be read");
        assert_eq!(state.settings().width(), 1200.0);
        assert!(crate::setting::authlib_injector_servers_path(&directory).exists());
        assert!(legacy_migration_receipt_path(&directory).exists());

        // The receipt keeps the launcher from reading the same file again.
        assert!(migrate_legacy_settings(&directory, &directory)
            .expect("the migration should not fail")
            .is_none());

        std::fs::remove_dir_all(&directory).expect("the directory should be removed");
    }

    #[test]
    fn the_migration_keeps_the_settings_of_the_launcher_of_today() {
        let directory = std::env::temp_dir().join("hmcl-legacy-settings-keep-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        // A player who already uses the launcher of today keeps the language
        // they selected.
        let settings = LauncherSettings {
            language: Some("en".to_string()),
            ..LauncherSettings::default()
        };
        crate::setting::SettingsFile::new(
            crate::setting::launcher_settings_path(&directory),
            settings,
        )
        .save()
        .expect("the settings should be written");

        std::fs::write(
            directory.join(LEGACY_CONFIG_FILENAME),
            r#"{"_version": 2, "localization": "zh_CN"}"#,
        )
        .expect("the file should be written");

        migrate_legacy_settings(&directory, &directory)
            .expect("the migration should run")
            .expect("the file should be migrated");

        let read = crate::setting::SettingsFile::<LauncherSettings>::load(
            crate::setting::launcher_settings_path(&directory),
        )
        .expect("the settings should be read");
        assert_eq!(read.settings().language.as_deref(), Some("en"));

        std::fs::remove_dir_all(&directory).expect("the directory should be removed");
    }

    #[test]
    fn the_accounts_of_every_workspace_are_migrated() {
        let directory = std::env::temp_dir().join("hmcl-legacy-user-accounts-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        // The older launcher kept the accounts every workspace sees in a list
        // of its own.
        std::fs::write(
            legacy_user_accounts_path(&directory),
            r#"[
                {"type": "offline", "username": "Notch"},
                {"type": "microsoft", "uuid": "b50ad385-829d-3141-a216-7e7d7539ba7f",
                 "displayName": "Notch", "accessToken": "access"}
            ]"#,
        )
        .expect("the file should be written");

        let migration = migrate_legacy_user_accounts(&directory)
            .expect("the migration should run")
            .expect("the file should be migrated");

        assert_eq!(migration.metadata.accounts.len(), 2);
        // The account of the shared file is named apart from an account of a
        // single workspace, even though the record is the same.
        assert_eq!(
            member_string(&migration.metadata.accounts[0], "accountID").as_deref(),
            Some("account-6ae53a64-855f-5bb7-b03e-20d0c5c0ee8b")
        );
        // The credentials of the account of Microsoft are kept apart from its
        // public data.
        assert_eq!(migration.private_data.len(), 1);

        // The files of the launcher of today are written.
        let read = crate::setting::AccountMetadataStore::load_from(
            &crate::setting::user_accounts_path(&directory),
        )
        .expect("the accounts should be read");
        assert_eq!(read.settings().accounts.len(), 2);
        assert!(crate::setting::user_account_private_data_path(&directory).exists());
        assert!(legacy_user_accounts_receipt_path(&directory).exists());

        // The receipt keeps the launcher from reading the same file again.
        assert!(migrate_legacy_user_accounts(&directory)
            .expect("the migration should not fail")
            .is_none());

        std::fs::remove_dir_all(&directory).expect("the directory should be removed");
    }

    #[test]
    fn the_accounts_of_the_launcher_of_today_are_not_replaced() {
        let directory = std::env::temp_dir().join("hmcl-legacy-user-accounts-keep-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        let existing = AccountMetadataStore::from_records(vec![serde_json::json!({
            "accountID": "account-0199a3b4-1111-7222-8333-444444444444",
            "type": "offline",
            "profileName": "Steve"
        })
        .as_object()
        .expect("an object")
        .clone()]);
        AccountMetadataStore::save_to(&crate::setting::user_accounts_path(&directory), &existing)
            .expect("the accounts should be written");

        std::fs::write(
            legacy_user_accounts_path(&directory),
            r#"[{"type": "offline", "username": "Notch"}]"#,
        )
        .expect("the file should be written");

        migrate_legacy_user_accounts(&directory)
            .expect("the migration should run")
            .expect("the file should be migrated");

        let read = AccountMetadataStore::load_from(&crate::setting::user_accounts_path(&directory))
            .expect("the accounts should be read");
        assert_eq!(read.settings().accounts.len(), 1);
        assert_eq!(
            member_string(&read.settings().accounts[0], "profileName").as_deref(),
            Some("Steve")
        );

        std::fs::remove_dir_all(&directory).expect("the directory should be removed");
    }

    #[test]
    fn a_shared_account_file_that_holds_no_list_names_no_account() {
        let directory = std::env::temp_dir().join("hmcl-legacy-user-accounts-broken-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        std::fs::write(legacy_user_accounts_path(&directory), r#"{"accounts": []}"#)
            .expect("the file should be written");

        // The file is left for a launcher that can read it rather than being
        // taken as a file without accounts.
        assert!(migrate_legacy_user_accounts(&directory)
            .expect("the migration should not fail")
            .is_none());
        assert!(!crate::setting::user_accounts_path(&directory).exists());

        std::fs::remove_dir_all(&directory).expect("the directory should be removed");
    }
}
