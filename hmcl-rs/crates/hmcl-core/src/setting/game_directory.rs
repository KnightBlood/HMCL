//! The game directories of the user.
//!
//! A game directory holds the instances of the game: the directory `versions`
//! with the files of the versions, and the libraries, the assets and the other
//! files the game reads. The user may have more than one, for example one for
//! the game and one for the servers.
//!
//! The directories are kept in two stores, like the launcher of Java keeps
//! them:
//!
//! * `config/game-directories.json` holds the directories of the workspace the
//!   launcher runs in,
//! * `config/user-game-directories.json` holds the directories every workspace
//!   shares.
//!
//! The launcher shows the two merged: a directory of the workspace hides the
//! directory of the user with the same identifier, so a player who moved a
//! shared directory into the workspace does not see it twice.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::auth::uuid::Uuid;
use crate::error::Error;
use crate::i18n::{I18n, LocalizedText};
use crate::util::portable_path::PortablePath;

use super::game_settings::GameSettingsPresetId;
use super::schema::{Schema, SchemaSettings, SchemaVersion};
use super::storage::SettingsFile;

/// The prefix of the identifiers of the game directories.
pub const GAME_DIRECTORY_ID_PREFIX: &str = "game-directory";

/// The name of the file that holds the directories of the workspace.
pub const GAME_DIRECTORIES_FILE: &str = "game-directories.json";

/// The name of the file that holds the directories every workspace shares.
pub const USER_GAME_DIRECTORIES_FILE: &str = "user-game-directories.json";

/// The path of the directory the launcher of Java uses for the game of the
/// workspace, which is relative to the directory the launcher runs in.
pub const CURRENT_GAME_DIRECTORY_PATH: &str = ".minecraft";

/// The identifier of the game directory of the workspace, which the launcher
/// creates for a workspace without a directory of its own.
pub const DEFAULT_GAME_DIRECTORY_ID: GameDirectoryId = GameDirectoryId::from_bytes([
    0x71, 0x05, 0xbc, 0x1f, 0x49, 0x0e, 0x5e, 0x8c, 0x87, 0x8c, 0xf5, 0x84, 0x4c, 0x3d, 0x4b, 0xc3,
]);

/// The identifier of the game directory of the home of the user, which the
/// launcher creates for a user without a directory of their own.
pub const HOME_GAME_DIRECTORY_ID: GameDirectoryId = GameDirectoryId::from_bytes([
    0xf3, 0xea, 0xfd, 0xe8, 0x50, 0x6e, 0x5a, 0x77, 0xbc, 0x88, 0xf2, 0x4b, 0x47, 0x28, 0xdf, 0xb2,
]);

/// The error of reading the identifier of a game directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDirectoryIdError {
    /// The text that could not be read.
    text: String,
}

impl GameDirectoryIdError {
    /// Creates an error for the text that could not be read.
    pub fn of(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    /// Returns the text that could not be read.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for GameDirectoryIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "The identifier of the game directory is malformed: {}",
            self.text
        )
    }
}

impl std::error::Error for GameDirectoryIdError {}

/// The identifier of a game directory.
///
/// The identifier never changes when the player renames the directory or moves
/// it to another path, so the instances the launcher selected stay selected.
/// The identifier is written as `game-directory:` followed by a UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameDirectoryId {
    /// The UUID payload of the identifier.
    uuid: Uuid,
}

impl GameDirectoryId {
    /// The identifier that stands for no directory.
    pub const NIL: Self = Self { uuid: Uuid::NIL };

    /// Creates an identifier from its UUID payload.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self {
            uuid: Uuid::from_bytes(bytes),
        }
    }

    /// Creates an identifier from its UUID payload.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self { uuid }
    }

    /// Creates an identifier for a new directory.
    pub fn generate() -> Self {
        Self {
            uuid: Uuid::generate_v7(),
        }
    }

    /// Returns the UUID payload of the identifier.
    pub const fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Returns whether the identifier stands for no directory.
    pub fn is_nil(&self) -> bool {
        self.uuid.is_nil()
    }

    /// Reads an identifier from its text.
    ///
    /// The text must start with `game-directory:` and hold a UUID.
    pub fn parse(text: &str) -> Result<Self, GameDirectoryIdError> {
        let prefix = format!("{GAME_DIRECTORY_ID_PREFIX}:");
        let payload = text
            .strip_prefix(&prefix)
            .ok_or_else(|| GameDirectoryIdError::of(text))?;

        Uuid::parse(payload)
            .map(|uuid| Self { uuid })
            .map_err(|_| GameDirectoryIdError::of(text))
    }
}

impl fmt::Display for GameDirectoryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{GAME_DIRECTORY_ID_PREFIX}:{}", self.uuid)
    }
}

impl Serialize for GameDirectoryId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for GameDirectoryId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// Reads the identifier of a directory and rejects the identifier of no
/// directory.
fn deserialize_directory_id<'de, D>(deserializer: D) -> Result<GameDirectoryId, D::Error>
where
    D: Deserializer<'de>,
{
    let id = GameDirectoryId::deserialize(deserializer)?;
    if id.is_nil() {
        return Err(serde::de::Error::custom(
            "the identifier of a game directory cannot be nil",
        ));
    }
    Ok(id)
}

/// A game directory of the user.
///
/// The path is stored together with whether it is absolute, so a relative path
/// stays relative when the launcher is moved to another machine; see
/// [`PortablePath`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDirectory {
    /// The identifier of the directory; it never changes.
    #[serde(deserialize_with = "deserialize_directory_id")]
    pub id: GameDirectoryId,
    /// The name the user gave the directory, if the user gave it one.
    ///
    /// The name may be translated, so the launcher shows it in the language of
    /// the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<LocalizedText>,
    /// The path of the directory.
    pub path: PortablePath,
    /// The identifier of the preset the instances of the directory inherited
    /// before the launcher kept the settings of an instance by itself.
    ///
    /// The launcher of Java writes the field while it migrates the settings of
    /// an older launcher, so the instances of the directory keep the settings
    /// they had; the field is kept so the two launchers read the same file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_game_settings: Option<GameSettingsPresetId>,
}

impl GameDirectory {
    /// Creates a directory that the user did not name.
    pub fn new(id: GameDirectoryId, path: PortablePath) -> Self {
        Self {
            id,
            name: None,
            path,
            legacy_game_settings: None,
        }
    }

    /// Returns the name the user gave the directory, in the language the
    /// launcher shows.
    ///
    /// The languages are the ones the launcher prefers, in the order it
    /// prefers them.
    pub fn custom_name(&self, languages: &[&str]) -> Option<&str> {
        self.name.as_ref().map(|name| name.text(languages))
    }

    /// Returns the absolute path of the directory.
    ///
    /// A relative path is resolved against the directory the launcher runs in,
    /// which is `base_directory`.
    pub fn absolute_path(&self, base_directory: &Path) -> PathBuf {
        self.path.resolve(base_directory)
    }

    /// Returns whether the directory is the directory of the workspace.
    ///
    /// The directory of the workspace is the one the launcher creates for a
    /// workspace that has no directory of its own, which is the directory
    /// `.minecraft` of the directory the launcher runs in.
    pub fn is_current_directory(&self) -> bool {
        !self.path.is_absolute() && self.path.path() == CURRENT_GAME_DIRECTORY_PATH
    }

    /// Returns whether the directory is the directory of the home of the user.
    pub fn is_home_directory(&self, home_directory: &PortablePath) -> bool {
        self.path.is_absolute() == home_directory.is_absolute()
            && self.path.path() == home_directory.path()
    }

    /// Returns the name the launcher shows for the directory.
    ///
    /// The name the user gave is shown when there is one; the two directories
    /// the launcher creates itself are shown with the names of their texts, and
    /// a directory the user added without a name is shown with its identifier,
    /// which is what the launcher of Java does.
    pub fn display_name(
        &self,
        languages: &[&str],
        i18n: &I18n,
        home_directory: &PortablePath,
    ) -> String {
        if let Some(name) = self.custom_name(languages) {
            return name.to_string();
        }

        if self.is_current_directory() {
            return i18n.i18n("game_directory.default");
        }
        if self.is_home_directory(home_directory) {
            return i18n.i18n("game_directory.home");
        }

        self.id.to_string()
    }
}

/// The schema of the file that holds the directories.
pub fn game_directories_schema() -> Schema {
    Schema::new("game-directories", SchemaVersion::new(1, 0, 0))
}

/// The directories of the game, as they are stored in one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GameDirectories {
    /// The address of the schema of the file.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// The directories, in the order the launcher shows them.
    pub directories: Vec<GameDirectory>,
}

impl Default for GameDirectories {
    fn default() -> Self {
        Self {
            schema: game_directories_schema().url().to_string(),
            directories: Vec::new(),
        }
    }
}

impl GameDirectories {
    /// Creates a store without directories.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a store from the directories.
    pub fn from_directories(directories: Vec<GameDirectory>) -> Self {
        Self {
            directories,
            ..Self::default()
        }
    }

    /// Returns whether the store holds no directory.
    pub fn is_empty(&self) -> bool {
        self.directories.is_empty()
    }

    /// Returns the directory with the given identifier.
    pub fn get(&self, id: GameDirectoryId) -> Option<&GameDirectory> {
        self.directories.iter().find(|directory| directory.id == id)
    }

    /// Returns the position of the directory with the given identifier.
    pub fn index_of(&self, id: GameDirectoryId) -> Option<usize> {
        self.directories
            .iter()
            .position(|directory| directory.id == id)
    }

    /// Adds a directory, or replaces the directory with the same identifier.
    ///
    /// A directory that is replaced keeps the place it had in the store, so the
    /// order the player sees does not change when a directory is renamed.
    pub fn insert(&mut self, directory: GameDirectory) {
        match self.index_of(directory.id) {
            Some(index) => self.directories[index] = directory,
            None => self.directories.push(directory),
        }
    }

    /// Removes the directory with the given identifier, if the store has it.
    pub fn remove(&mut self, id: GameDirectoryId) -> Option<GameDirectory> {
        let index = self.index_of(id)?;
        Some(self.directories.remove(index))
    }

    /// Returns the directories of the store, merged with the directories of
    /// another store.
    ///
    /// The directories of this store are shown first and hide the directories
    /// of the other store with the same identifier, which is the way the
    /// launcher of Java merges the directories of the workspace with the
    /// directories of the user.
    pub fn merged_with(&self, other: &Self) -> Vec<GameDirectory> {
        let mut merged: Vec<GameDirectory> = self.directories.clone();
        for directory in &other.directories {
            if !merged.iter().any(|known| known.id == directory.id) {
                merged.push(directory.clone());
            }
        }
        merged
    }

    /// Creates the built-in directories when the store has none.
    ///
    /// The directory of the workspace and the directory of the home of the user
    /// are the two the launcher of Java creates for a new user, so the launcher
    /// always has a directory to show. The directories are returned, so the
    /// caller can tell whether it has to show them again.
    pub fn ensure_built_in_directories(&mut self, home_directory: &PortablePath) -> bool {
        if !self.directories.is_empty() {
            return false;
        }

        self.directories.push(GameDirectory::new(
            DEFAULT_GAME_DIRECTORY_ID,
            PortablePath::of(CURRENT_GAME_DIRECTORY_PATH),
        ));
        self.directories.push(GameDirectory::new(
            HOME_GAME_DIRECTORY_ID,
            home_directory.clone(),
        ));
        true
    }

    /// Reads the directories from `config/game-directories.json` of the given
    /// home directory.
    pub fn load(home: &Path) -> Result<SettingsFile<Self>, Error> {
        Self::load_from(super::game_directories_path(home))
    }

    /// Writes the directories to `config/game-directories.json` of the given
    /// home directory.
    pub fn save(home: &Path, directories: &Self) -> Result<(), Error> {
        Self::save_to(super::game_directories_path(home), directories)
    }

    /// Reads the directories from a file.
    pub fn load_from(path: impl Into<PathBuf>) -> Result<SettingsFile<Self>, Error> {
        SettingsFile::load_with_schema(path, &game_directories_schema())
    }

    /// Writes the directories to a file.
    pub fn save_to(path: impl Into<PathBuf>, directories: &Self) -> Result<(), Error> {
        SettingsFile::new(path, directories.clone()).save()
    }
}

impl SchemaSettings for GameDirectories {
    fn schema(&self) -> &str {
        &self.schema
    }

    fn set_schema(&mut self, schema: &str) {
        self.schema = schema.to_string();
    }
}

/// Returns the path of the directory of the game of the user.
///
/// The directory is the one the launcher of Java uses on the system the
/// launcher runs on, which is where the launcher of Mojang keeps the game as
/// well.
pub fn default_home_game_directory() -> PortablePath {
    PortablePath::from_path(crate::metadata::minecraft_directory())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;

    /// Returns a directory with the identifier the tests use.
    fn directory(name: Option<&str>, path: &str) -> GameDirectory {
        let mut directory = GameDirectory::new(
            GameDirectoryId::parse("game-directory:123e4567-e89b-12d3-a456-426614174000")
                .expect("the identifier should be read"),
            PortablePath::of(path),
        );
        directory.name = name.map(LocalizedText::plain);
        directory
    }

    #[test]
    fn the_identifiers_are_read_and_written() {
        let id = GameDirectoryId::parse("game-directory:123e4567-e89b-12d3-a456-426614174000")
            .expect("the identifier should be read");
        assert_eq!(
            id.to_string(),
            "game-directory:123e4567-e89b-12d3-a456-426614174000"
        );
        assert_eq!(id.uuid().version(), 1);

        // The identifier without the prefix is not an identifier.
        assert!(GameDirectoryId::parse("123e4567-e89b-12d3-a456-426614174000").is_err());
        assert!(GameDirectoryId::parse("account:123e4567-e89b-12d3-a456-426614174000").is_err());
    }

    #[test]
    fn the_built_in_identifiers_are_the_ones_of_the_java_launcher() {
        assert_eq!(
            DEFAULT_GAME_DIRECTORY_ID.to_string(),
            "game-directory:7105bc1f-490e-5e8c-878c-f5844c3d4bc3"
        );
        assert_eq!(
            HOME_GAME_DIRECTORY_ID.to_string(),
            "game-directory:f3eafde8-506e-5a77-bc88-f24b4728dfb2"
        );
    }

    #[test]
    fn the_created_identifiers_are_unique() {
        let first = GameDirectoryId::generate();
        let second = GameDirectoryId::generate();

        assert_ne!(first, second);
        assert_eq!(first.uuid().version(), 7);
        assert!(!first.is_nil());
    }

    #[test]
    fn a_directory_is_written_the_way_the_java_launcher_writes_it() {
        let mut game_directory = directory(Some("Dev"), "versions\\Dev");
        game_directory.legacy_game_settings = Some(
            GameSettingsPresetId::parse(
                "game-settings-preset:123e4567-e89b-12d3-a456-426614174001",
            )
            .expect("the identifier should be read"),
        );

        let json = serde_json::to_value(&game_directory).expect("the directory should be written");
        assert_eq!(
            json["id"],
            "game-directory:123e4567-e89b-12d3-a456-426614174000"
        );
        assert_eq!(json["name"], "Dev");
        assert_eq!(json["path"], "versions/Dev");
        assert_eq!(
            json["legacyGameSettings"],
            "game-settings-preset:123e4567-e89b-12d3-a456-426614174001"
        );
        assert!(json.get("useRelativePath").is_none());

        let read: GameDirectory =
            serde_json::from_value(json).expect("the directory should be read");
        assert_eq!(read, game_directory);
        assert!(!read.path.is_absolute());
    }

    #[test]
    fn a_directory_without_a_name_or_a_legacy_preset_writes_only_its_path() {
        let json =
            serde_json::to_value(directory(None, "/games")).expect("the directory is written");

        assert!(json.get("name").is_none());
        assert!(json.get("legacyGameSettings").is_none());
        assert_eq!(json["path"], "/games");
    }

    #[test]
    fn a_directory_without_an_identifier_or_a_path_is_refused() {
        // The identifier and the path are required, as they are in Java.
        assert!(serde_json::from_str::<GameDirectory>(r#"{"path": "/games"}"#).is_err());
        assert!(serde_json::from_str::<GameDirectory>(
            r#"{"id": "game-directory:123e4567-e89b-12d3-a456-426614174000"}"#
        )
        .is_err());

        // The identifier of no directory is refused.
        assert!(serde_json::from_str::<GameDirectory>(
            r#"{"id": "game-directory:00000000-0000-0000-0000-000000000000", "path": "/games"}"#
        )
        .is_err());

        // An identifier of another kind of record is refused.
        assert!(serde_json::from_str::<GameDirectory>(
            r#"{"id": "account:123e4567-e89b-12d3-a456-426614174000", "path": "/games"}"#
        )
        .is_err());
    }

    #[test]
    fn the_name_of_a_directory_is_shown_in_the_language_of_the_user() {
        let i18n = I18n::new(Locale::English);
        let home = PortablePath::of("/home/user/.minecraft");

        let named = directory(Some("My games"), "games");
        assert_eq!(named.display_name(&["en"], &i18n, &home), "My games");

        // The directories the launcher creates are named by their texts.
        let current = GameDirectory::new(
            DEFAULT_GAME_DIRECTORY_ID,
            PortablePath::of(CURRENT_GAME_DIRECTORY_PATH),
        );
        assert_eq!(current.display_name(&["en"], &i18n, &home), "Current");

        let home_directory = GameDirectory::new(HOME_GAME_DIRECTORY_ID, home.clone());
        assert_eq!(
            home_directory.display_name(&["en"], &i18n, &home),
            "Minecraft Launcher"
        );

        // A directory the user added without a name is shown by its identifier.
        let unnamed = directory(None, "/games");
        assert_eq!(
            unnamed.display_name(&["en"], &i18n, &home),
            "game-directory:123e4567-e89b-12d3-a456-426614174000"
        );
    }

    #[test]
    fn the_paths_of_the_directories_are_resolved() {
        let relative = directory(None, "games");
        assert_eq!(
            relative.absolute_path(Path::new("/home/user/.hmcl")),
            PathBuf::from("/home/user/.hmcl/games")
        );

        let absolute = directory(None, "/games");
        assert_eq!(
            absolute.absolute_path(Path::new("/home/user/.hmcl")),
            PathBuf::from("/games")
        );
    }

    #[test]
    fn a_directory_that_is_added_again_keeps_its_place() {
        let mut store = GameDirectories::new();
        let first = directory(Some("First"), "first");
        let mut second = directory(Some("Second"), "second");
        second.id = GameDirectoryId::generate();
        store.insert(first.clone());
        store.insert(second.clone());

        let mut renamed = first.clone();
        renamed.name = Some(LocalizedText::plain("Renamed"));
        store.insert(renamed.clone());

        assert_eq!(store.directories, vec![renamed.clone(), second]);
        assert_eq!(store.get(first.id), Some(&renamed));
        assert_eq!(store.index_of(first.id), Some(0));
    }

    #[test]
    fn the_directories_of_the_workspace_hide_the_directories_of_the_user() {
        let mut local = GameDirectories::new();
        local.insert(directory(Some("Local"), "local/Dev"));

        let mut user = GameDirectories::new();
        // The same identifier, but the workspace wins.
        user.insert(directory(Some("User"), "user/Dev"));
        let mut other = directory(Some("Added"), "user/Added");
        other.id = GameDirectoryId::generate();
        user.insert(other.clone());

        let merged = local.merged_with(&user);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, Some(LocalizedText::plain("Local")));
        assert_eq!(merged[1], other);

        // The user store keeps its own copy of the hidden directory.
        assert_eq!(user.directories.len(), 2);
    }

    #[test]
    fn an_empty_store_gets_the_two_built_in_directories() {
        let mut store = GameDirectories::new();
        let home = PortablePath::of("/home/user/.minecraft");

        assert!(store.ensure_built_in_directories(&home));
        assert_eq!(store.directories.len(), 2);
        assert_eq!(store.directories[0].id, DEFAULT_GAME_DIRECTORY_ID);
        assert_eq!(
            store.directories[0].path.path(),
            CURRENT_GAME_DIRECTORY_PATH
        );
        assert_eq!(store.directories[1].id, HOME_GAME_DIRECTORY_ID);
        assert_eq!(store.directories[1].path, home);

        // A store that has a directory keeps it.
        assert!(!store.ensure_built_in_directories(&home));
    }

    #[test]
    fn the_store_uses_the_schema_of_the_java_launcher() {
        let store = GameDirectories::new();
        let json = serde_json::to_value(&store).expect("the store should be written");

        assert_eq!(
            json["$schema"],
            "https://schemas.glavo.site/hmcl/game-directories/1.0.0"
        );
        assert_eq!(json["directories"], serde_json::json!([]));

        let read: GameDirectories = serde_json::from_value(serde_json::json!({"directories": []}))
            .expect("the store should be read");
        assert_eq!(read.schema, game_directories_schema().url());
    }

    #[test]
    fn the_store_is_written_and_read_back() {
        let directory_path = std::env::temp_dir().join(format!(
            "hmcl-core-game-directories-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory_path);
        std::fs::create_dir_all(&directory_path).expect("the directory should be created");

        let mut store = GameDirectories::new();
        store.insert(directory(Some("Dev"), "versions/Dev"));

        GameDirectories::save(&directory_path, &store).expect("the directories should be saved");
        let read =
            GameDirectories::load(&directory_path).expect("the directories should be loaded");

        assert_eq!(read.settings(), &store);
        assert_eq!(
            read.path(),
            super::super::game_directories_path(&directory_path)
        );
        assert!(read.is_savable());

        let _ = std::fs::remove_dir_all(&directory_path);
    }

    #[test]
    fn the_file_of_a_newer_launcher_is_only_read() {
        let directory_path = std::env::temp_dir().join(format!(
            "hmcl-core-game-directories-newer-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory_path);
        std::fs::create_dir_all(directory_path.join("config")).expect("the directory is created");

        let path = super::super::game_directories_path(&directory_path);
        std::fs::write(
            &path,
            r#"{"$schema": "https://schemas.glavo.site/hmcl/game-directories/1.1.0", "directories": []}"#,
        )
        .expect("the file should be written");

        let read =
            GameDirectories::load(&directory_path).expect("the directories should be loaded");
        assert!(!read.is_savable());
        assert!(read.settings().is_empty());

        let _ = std::fs::remove_dir_all(&directory_path);
    }

    #[test]
    fn the_identifier_of_a_broken_file_is_reported() {
        let error = GameDirectoryId::parse("nonsense").expect_err("the identifier is broken");
        assert_eq!(error.text(), "nonsense");
        assert!(error.to_string().contains("nonsense"));

        // A UUID that is not an identifier is not read either.
        assert!(GameDirectoryId::parse("game-directory:not-a-uuid").is_err());
    }
}
