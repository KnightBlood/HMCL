//! The settings a version of the game is started with.
//!
//! The launcher keeps the settings in presets, and every instance may override
//! a part of the settings of its parent preset. An instance only inherits the
//! settings it does not override, so changing a preset changes every instance
//! that does not override the changed setting.
//!
//! The settings of a preset are stored in `config/game-settings.json`; the
//! settings of an instance are stored in
//! `.hmcl/config/instance-game-settings.json` inside the instance.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::auth::Uuid;
use crate::download::game::InstanceManifest;
use crate::game::component::GameComponentType;
use crate::game::graphics::{GraphicsApi, Renderer};
use crate::game::java_version::find_suitable_java;
use crate::game::process_priority::ProcessPriority;
use crate::game::quick_play::{QuickPlayOption, QuickPlayType};
use crate::i18n::LocalizedText;
use crate::java::JavaRuntime;
use crate::setting::schema::{Schema, SchemaSettings, SchemaVersion};
use crate::setting::storage::SettingsFile;
use crate::util::digest::{to_hex, Sha256};
use crate::util::operating_system::{current_os, total_physical_memory, OsType};
use crate::util::versioning::GameVersionNumber;

/// The prefix of the serialized identifiers of the presets.
pub const GAME_SETTINGS_PRESET_ID_PREFIX: &str = "game-settings-preset";

/// The error of reading the identifier of a preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSettingsPresetIdError {
    /// The text that could not be read.
    text: String,
}

impl GameSettingsPresetIdError {
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

impl fmt::Display for GameSettingsPresetIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "The identifier of the game settings preset is malformed: {}",
            self.text
        )
    }
}

impl std::error::Error for GameSettingsPresetIdError {}

/// The identifier of a saved set of game settings.
///
/// The identifier is written as `game-settings-preset:` followed by a UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameSettingsPresetId {
    /// The UUID payload of the identifier.
    uuid: Uuid,
}

impl GameSettingsPresetId {
    /// The identifier that stands for no preset.
    pub const NIL: Self = Self { uuid: Uuid::NIL };

    /// Reads an identifier from its text.
    ///
    /// The text must start with `game-settings-preset:` and hold a UUID.
    pub fn parse(text: &str) -> Result<Self, GameSettingsPresetIdError> {
        let prefix = format!("{GAME_SETTINGS_PRESET_ID_PREFIX}:");
        let payload = text
            .strip_prefix(&prefix)
            .ok_or_else(|| GameSettingsPresetIdError::of(text))?;

        Uuid::parse(payload)
            .map(|uuid| Self { uuid })
            .map_err(|_| GameSettingsPresetIdError::of(text))
    }

    /// Creates an identifier from its UUID payload.
    ///
    /// The payload of an identifier made from the name of a legacy profile is
    /// an identifier of version 5, so the same profile is named the same way
    /// every time the migration runs.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self { uuid }
    }

    /// Creates an identifier that sorts after the identifiers created before
    /// it.
    pub fn generate() -> Self {
        Self {
            uuid: Uuid::generate_v7(),
        }
    }

    /// Returns the UUID payload of the identifier.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Returns whether the identifier stands for no preset.
    pub fn is_nil(&self) -> bool {
        self.uuid.is_nil()
    }
}

impl fmt::Display for GameSettingsPresetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{GAME_SETTINGS_PRESET_ID_PREFIX}:{}", self.uuid)
    }
}

impl FromStr for GameSettingsPresetId {
    type Err = GameSettingsPresetIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl Serialize for GameSettingsPresetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for GameSettingsPresetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// The way the launcher chooses the Java runtime of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JavaVersionType {
    /// The launcher chooses a runtime that fits the version of the game.
    #[default]
    Auto,
    /// The launcher uses a runtime of the version the user entered.
    Version,
    /// The launcher uses a runtime the launcher detected and the user chose.
    Detected,
    /// The launcher uses the executable the user entered.
    Custom,
}

/// The way the window of the game is shown when the game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GameWindowType {
    /// The game starts in a window.
    #[default]
    Windowed,
    /// The game starts in the full screen.
    Fullscreen,
    /// The game starts in a maximized window.
    Maximized,
}

/// What the launcher does when the game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LauncherVisibility {
    /// The launcher closes as soon as the game process is created.
    Close,
    /// The launcher hides its window and shows it again when the game
    /// closes.
    #[default]
    Hide,
    /// The launcher keeps its window visible.
    Keep,
    /// The launcher hides its window while the game runs and opens it again
    /// when the game closes.
    HideAndReopen,
}

impl LauncherVisibility {
    /// Returns whether the launcher stays alive while the game runs.
    ///
    /// A launcher that stays alive can show the log of the game and open its
    /// window again when the game closes.
    pub fn is_daemon(self) -> bool {
        self != Self::Close
    }
}

/// The isolation of the files of an instance when a new instance is
/// installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DefaultIsolationType {
    /// Every new instance keeps its files in its own directory.
    Always,
    /// A new instance keeps its files in its own directory when it has a mod
    /// loader.
    #[default]
    Modded,
    /// New instances share the files of the game directory.
    Never,
}

/// The picture the launcher shows for an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GameInstanceIconType {
    /// The picture the launcher chooses.
    #[default]
    Default,
    /// A block of grass.
    Grass,
    /// A chest.
    Chest,
    /// A chicken.
    Chicken,
    /// A command block.
    Command,
    /// The logo of OptiFine.
    Optifine,
    /// A crafting table.
    CraftTable,
    /// The logo of Fabric.
    Fabric,
    /// The logo of Forge.
    Forge,
    /// The logo of NeoForge.
    NeoForge,
    /// A furnace.
    Furnace,
    /// The logo of Quilt.
    Quilt,
    /// The picture of the April Fools update.
    AprilFools,
    /// The logo of Cleanroom.
    Cleanroom,
    /// The logo of Legacy Fabric.
    LegacyFabric,
}

impl GameInstanceIconType {
    /// Returns the path of the picture in the resources of the launcher.
    pub fn resource_url(self) -> &'static str {
        match self {
            Self::Default | Self::Grass => "/assets/img/grass.png",
            Self::Chest => "/assets/img/chest.png",
            Self::Chicken => "/assets/img/chicken.png",
            Self::Command => "/assets/img/command.png",
            Self::Optifine => "/assets/img/optifine.png",
            Self::CraftTable => "/assets/img/craft_table.png",
            Self::Fabric => "/assets/img/fabric.png",
            Self::Forge => "/assets/img/forge.png",
            Self::NeoForge => "/assets/img/neoforge.png",
            Self::Furnace => "/assets/img/furnace.png",
            Self::Quilt => "/assets/img/quilt.png",
            Self::AprilFools => "/assets/img/april_fools.png",
            Self::Cleanroom => "/assets/img/cleanroom.png",
            Self::LegacyFabric => "/assets/img/legacyfabric.png",
        }
    }

    /// Returns the picture that fits a component of an instance, if the
    /// launcher has one for it.
    pub fn from_component_type(component: GameComponentType) -> Option<Self> {
        match component {
            GameComponentType::Game => Some(Self::Grass),
            GameComponentType::Fabric | GameComponentType::FabricApi => Some(Self::Fabric),
            GameComponentType::LegacyFabric | GameComponentType::LegacyFabricApi => {
                Some(Self::LegacyFabric)
            }
            GameComponentType::Forge => Some(Self::Forge),
            GameComponentType::Cleanroom => Some(Self::Cleanroom),
            GameComponentType::LiteLoader => Some(Self::Chicken),
            GameComponentType::OptiFine => Some(Self::Optifine),
            GameComponentType::Quilt | GameComponentType::QuiltApi => Some(Self::Quilt),
            GameComponentType::NeoForge => Some(Self::NeoForge),
        }
    }
}

/// A reference to a Java runtime from the list the launcher detected.
///
/// The path of the runtime is stored as a digest, so the reference still
/// points to the same runtime when the launcher is moved to another machine.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DetectedJava {
    /// The version the runtime reported.
    pub version: String,
    /// The digest of the normalized path of the executable, or an empty string
    /// when the path is not known.
    pub path_hash: String,
}

impl DetectedJava {
    /// The reference that points to no runtime.
    pub const EMPTY: Self = Self {
        version: String::new(),
        path_hash: String::new(),
    };

    /// Creates a reference for a runtime the launcher detected.
    pub fn of(runtime: &crate::java::JavaRuntime) -> Self {
        Self {
            version: runtime.version().to_string(),
            path_hash: Self::hash_existing_path(runtime.binary()),
        }
    }

    /// Creates a reference from the fields the older versions of the launcher
    /// stored.
    pub fn of_legacy_path(version: &str, binary: &str) -> Self {
        let path_hash = if binary.trim().is_empty() {
            String::new()
        } else {
            let existing = Self::hash_existing_path(Path::new(binary));
            if existing.is_empty() {
                Self::hash_path_text(binary)
            } else {
                existing
            }
        };

        Self {
            version: version.to_string(),
            path_hash,
        }
    }

    /// Returns the digest of the path of an executable that exists.
    ///
    /// A path that does not exist yields an empty string.
    pub fn hash_existing_path(binary: &Path) -> String {
        match crate::java::runtime::canonicalize(binary) {
            Ok(path) => Self::hash_path_text(&path.to_string_lossy()),
            Err(_) => String::new(),
        }
    }

    /// Returns the digest of the path of an executable.
    ///
    /// The path is written with `/` as the separator, and in lower case on
    /// Windows, so the digest does not depend on the way the path was written.
    pub fn hash_path_text(binary: &str) -> String {
        let mut normalized = binary.replace('\\', "/");
        if current_os() == OsType::Windows {
            normalized.make_ascii_lowercase();
        }

        let mut digest = Sha256::new();
        digest.update(normalized.as_bytes());
        to_hex(&digest.finish())
    }

    /// Returns whether the reference points to no runtime.
    pub fn is_empty(&self) -> bool {
        self.version.trim().is_empty() && self.path_hash.trim().is_empty()
    }
}

/// Returns the amount of memory the launcher suggests for the game.
///
/// The launcher gives a quarter of the memory of the machine, rounded to
/// multiples of 128 MiB, but at least 256 MiB. A machine with 32 GiB or more
/// gets 8 GiB.
pub fn suggested_memory() -> u32 {
    let total_megabytes = total_physical_memory()
        .map(|bytes| bytes as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);

    if total_megabytes >= 32768.0 {
        8192
    } else {
        let suggested = (total_megabytes / 4.0 / 128.0).round() * 128.0;
        suggested.max(256.0) as u32
    }
}

/// The settings a preset and an instance can hold.
///
/// A setting that is not set is `None`; the launcher uses the value of the
/// preset, or the default value of the setting, in its place.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GameSettingsValues {
    /// The way the launcher chooses the Java runtime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_type: Option<JavaVersionType>,
    /// The major version of the Java runtime the user chose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_java_version: Option<String>,
    /// The path of the Java executable the user chose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_java_path: Option<String>,
    /// The runtime the launcher detected and the user chose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_java: Option<DetectedJava>,
    /// Whether the launcher starts the game without the window of the
    /// console.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_use_javaw: Option<bool>,
    /// The arguments of the virtual machine the user set.
    #[serde(rename = "jvmOptions", skip_serializing_if = "Option::is_none")]
    pub jvm_options: Option<String>,
    /// Whether the launcher does not add its own arguments of the virtual
    /// machine.
    #[serde(rename = "noJVMOptions", skip_serializing_if = "Option::is_none")]
    pub no_jvm_options: Option<bool>,
    /// Whether the launcher does not add its own optimizing arguments of the
    /// virtual machine.
    #[serde(
        rename = "noOptimizingJVMOptions",
        skip_serializing_if = "Option::is_none"
    )]
    pub no_optimizing_jvm_options: Option<bool>,
    /// Whether the launcher starts the game even when the Java runtime does
    /// not match the version of the game.
    #[serde(rename = "notCheckJVM", skip_serializing_if = "Option::is_none")]
    pub not_check_jvm: Option<bool>,
    /// Whether the launcher starts the game even when the files of the game
    /// are not complete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_check_game: Option<bool>,
    /// Whether the launcher chooses the amount of memory by itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_memory: Option<bool>,
    /// The smallest amount of memory the game keeps, in MiB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_memory: Option<u32>,
    /// The largest amount of memory the game may use, in MiB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory: Option<u32>,
    /// The amount of memory the metaspace of the game may use, in MiB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perm_size: Option<String>,
    /// The way the window of the game is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_type: Option<GameWindowType>,
    /// The width of the window of the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// The height of the window of the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// The directory the game runs in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_directory: Option<String>,
    /// The priority of the process of the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_priority: Option<ProcessPriority>,
    /// What the launcher does when the game starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher_visibility: Option<LauncherVisibility>,
    /// Whether the launcher may inject its agent into the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_auto_agent: Option<bool>,
    /// Whether the launcher does not write the options of the game by itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_auto_game_options: Option<bool>,
    /// The arguments of the game the user set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_arguments: Option<String>,
    /// The graphics interface the game draws with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphics_backend: Option<GraphicsApi>,
    /// The OpenGL driver the game draws with.
    #[serde(rename = "openGLRenderer", skip_serializing_if = "Option::is_none")]
    pub open_gl_renderer: Option<Renderer>,
    /// The Vulkan driver the game draws with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vulkan_renderer: Option<Renderer>,
    /// Whether the game asks for the high performance graphics card.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_performance: Option<bool>,
    /// The environment variables of the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_variables: Option<String>,
    /// The program that starts the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_wrapper: Option<String>,
    /// The command the launcher runs before it starts the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_launch_command: Option<String>,
    /// The command the launcher runs after the game closes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_exit_command: Option<String>,
    /// The kind of place the game opens when it starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_play: Option<QuickPlayType>,
    /// The address of the server the game joins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_play_multiplayer: Option<String>,
    /// The folder of the world the game opens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_play_singleplayer: Option<String>,
    /// The identifier of the realm the game joins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_play_realms: Option<String>,
    /// Whether the launcher shows the log of the game after it starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_logs: Option<bool>,
    /// Whether the game writes the log of the debug level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_debug_log_output: Option<bool>,
    /// Whether the launcher does not patch the native libraries of the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_patch_natives: Option<bool>,
    /// Whether the launcher leaves the native libraries to the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_custom_natives: Option<bool>,
    /// The directory that holds the native libraries of the game.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub natives_directory: Option<String>,
    /// Whether the game uses the native GLFW or SDL library.
    #[serde(rename = "useNativeGLFW", skip_serializing_if = "Option::is_none")]
    pub use_native_glfw_or_sdl: Option<bool>,
    /// Whether the game uses the native OpenAL library.
    #[serde(rename = "useNativeOpenAL", skip_serializing_if = "Option::is_none")]
    pub use_native_openal: Option<bool>,
    /// The settings the launcher does not know yet.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

/// A saved set of game settings that instances can inherit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    /// The identifier of the preset.
    #[serde(deserialize_with = "deserialize_preset_id")]
    pub id: GameSettingsPresetId,
    /// The name the user gave the preset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<LocalizedText>,
    /// The number the launcher gave the preset when the user did not name
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_name_number: Option<u32>,
    /// The isolation of the files of a new instance that inherits the
    /// preset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_isolation_type: Option<DefaultIsolationType>,
    /// The settings of the preset.
    #[serde(default, flatten)]
    pub settings: GameSettingsValues,
}

impl Preset {
    /// Creates a preset that holds only its identifier.
    pub fn new(id: GameSettingsPresetId) -> Self {
        Self {
            id,
            name: None,
            auto_name_number: None,
            default_isolation_type: None,
            settings: GameSettingsValues::default(),
        }
    }

    /// Returns the isolation of the files of a new instance that inherits the
    /// preset.
    pub fn default_isolation_type(&self) -> DefaultIsolationType {
        self.default_isolation_type
            .unwrap_or(DefaultIsolationType::Modded)
    }

    /// Resolves the settings of the preset and the settings of an instance
    /// that may override a part of them.
    pub fn resolve<'a>(&'a self, instance: Option<&'a InstanceSettings>) -> Effective<'a> {
        Effective::new(self, instance)
    }
}

/// Reads the identifier of a preset and rejects the identifier of no preset.
fn deserialize_preset_id<'de, D>(deserializer: D) -> Result<GameSettingsPresetId, D::Error>
where
    D: Deserializer<'de>,
{
    let id = GameSettingsPresetId::deserialize(deserializer)?;
    if id.is_nil() {
        return Err(serde::de::Error::custom(
            "the identifier of a preset cannot be nil",
        ));
    }
    Ok(id)
}

/// The address of the schema of the settings of an instance.
pub const INSTANCE_GAME_SETTINGS_SCHEMA: &str =
    "https://schemas.glavo.site/hmcl/instance-game-settings/1.0.1";

/// The name of the file that holds the settings of an instance.
pub const INSTANCE_GAME_SETTINGS_FILE: &str = "instance-game-settings.json";

/// Returns the schema of the settings of an instance.
pub fn instance_game_settings_schema() -> Schema {
    Schema::new("instance-game-settings", SchemaVersion::new(1, 0, 1))
}

/// Returns the file that holds the settings of an instance.
///
/// The file is kept in the metadata directory of the instance, so the files
/// of the game itself are not touched.
pub fn instance_game_settings_path(instance_root: &Path) -> PathBuf {
    instance_root
        .join(".hmcl")
        .join("config")
        .join(INSTANCE_GAME_SETTINGS_FILE)
}

/// The settings of one instance.
///
/// An instance inherits the settings of its parent preset. A setting the
/// instance overrides is read from the instance; the other settings are read
/// from the preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceSettings {
    /// The address of the schema of the file.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// The identifier of the preset the instance inherits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<GameSettingsPresetId>,
    /// The picture the launcher shows for the instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<GameInstanceIconType>,
    /// The names of the settings the instance overrides.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub override_properties: BTreeSet<String>,
    /// The settings the instance sets itself.
    #[serde(flatten)]
    pub settings: GameSettingsValues,
}

impl Default for InstanceSettings {
    fn default() -> Self {
        Self {
            schema: INSTANCE_GAME_SETTINGS_SCHEMA.to_string(),
            parent: None,
            icon: None,
            override_properties: BTreeSet::new(),
            settings: GameSettingsValues::default(),
        }
    }
}

impl InstanceSettings {
    /// Creates settings that override nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the picture the launcher shows for the instance.
    pub fn icon(&self) -> GameInstanceIconType {
        self.icon.unwrap_or(GameInstanceIconType::Default)
    }

    /// Returns whether the instance overrides a setting.
    ///
    /// The name is the one the setting has in the settings file, such as
    /// `maxMemory` or `javaType`.
    pub fn is_overridden(&self, property: &str) -> bool {
        self.override_properties.contains(property)
    }

    /// Sets whether the instance overrides a setting.
    pub fn set_overridden(&mut self, property: &str, overridden: bool) {
        if overridden {
            self.override_properties.insert(property.to_string());
        } else {
            self.override_properties.remove(property);
        }
    }

    /// Resolves the settings of the instance and its parent preset.
    pub fn resolve<'a>(&'a self, preset: &'a Preset) -> Effective<'a> {
        Effective::new(preset, Some(self))
    }

    /// Reads the settings of an instance.
    ///
    /// A missing file yields the default settings, which inherit every
    /// setting from the parent preset.
    pub fn load(instance_root: &Path) -> Result<SettingsFile<Self>, crate::error::Error> {
        SettingsFile::load_with_schema(
            instance_game_settings_path(instance_root),
            &instance_game_settings_schema(),
        )
    }

    /// Writes the settings of an instance.
    pub fn save(instance_root: &Path, settings: &Self) -> Result<(), crate::error::Error> {
        let file = SettingsFile::new(instance_game_settings_path(instance_root), settings.clone());
        file.save()
    }
}

impl SchemaSettings for InstanceSettings {
    fn schema(&self) -> &str {
        &self.schema
    }

    fn set_schema(&mut self, schema: &str) {
        self.schema = schema.to_string();
    }
}

/// The settings of a preset and an instance, resolved into the values the
/// launcher uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effective<'a> {
    /// The preset the instance inherits from.
    preset: &'a Preset,
    /// The instance that may override a part of the settings.
    instance: Option<&'a InstanceSettings>,
}

impl<'a> Effective<'a> {
    /// Resolves the settings of a preset and an instance.
    pub fn new(preset: &'a Preset, instance: Option<&'a InstanceSettings>) -> Self {
        Self { preset, instance }
    }

    /// Returns the preset the settings are resolved from.
    pub fn preset(&self) -> &'a Preset {
        self.preset
    }

    /// Returns the instance whose settings are resolved, if there is one.
    pub fn instance(&self) -> Option<&'a InstanceSettings> {
        self.instance
    }
}

/// Generates the getters that resolve one setting of an instance and its
/// preset.
macro_rules! inherited_getters {
    ($( $(#[$doc:meta])* $method:ident => $field:ident, $property:literal, $type:ty, $default:expr; )*) => {
        impl<'a> Effective<'a> {
            $(
                $(#[$doc])*
                pub fn $method(&self) -> $type {
                    let value = match self.instance {
                        Some(instance) if instance.is_overridden($property) => {
                            instance.settings.$field.clone()
                        }
                        _ => self.preset.settings.$field.clone(),
                    };

                    value.unwrap_or_else(|| $default)
                }
            )*
        }
    };
}

inherited_getters! {
    /// Returns the way the launcher chooses the Java runtime.
    java_type => java_type, "javaType", JavaVersionType, JavaVersionType::Auto;
    /// Returns the major version of Java the user chose.
    custom_java_version => custom_java_version, "customJavaVersion", String, String::new();
    /// Returns the path of the Java executable the user chose.
    custom_java_path => custom_java_path, "customJavaPath", String, String::new();
    /// Returns the runtime the launcher detected and the user chose.
    detected_java => detected_java, "detectedJava", DetectedJava, DetectedJava::EMPTY;
    /// Returns whether the launcher starts the game without the window of the
    /// console.
    not_use_javaw => not_use_javaw, "notUseJavaw", bool, false;
    /// Returns the arguments of the virtual machine the user set.
    jvm_options => jvm_options, "jvmOptions", String, String::new();
    /// Returns whether the launcher does not add its own arguments of the
    /// virtual machine.
    no_jvm_options => no_jvm_options, "noJVMOptions", bool, false;
    /// Returns whether the launcher does not add its own optimizing arguments
    /// of the virtual machine.
    no_optimizing_jvm_options => no_optimizing_jvm_options, "noOptimizingJVMOptions", bool, false;
    /// Returns whether the launcher starts the game even when the Java runtime
    /// does not match the version of the game.
    not_check_jvm => not_check_jvm, "notCheckJVM", bool, false;
    /// Returns whether the launcher starts the game even when the files of the
    /// game are not complete.
    not_check_game => not_check_game, "notCheckGame", bool, false;
    /// Returns whether the launcher chooses the amount of memory by itself.
    auto_memory => auto_memory, "autoMemory", bool, true;
    /// Returns the amount of memory the game uses, in MiB.
    max_memory_setting => max_memory, "maxMemory", u32, suggested_memory();
    /// Returns the amount of memory the metaspace of the game may use, in MiB.
    perm_size => perm_size, "permSize", String, String::new();
    /// Returns the way the window of the game is shown.
    window_type => window_type, "windowType", GameWindowType, GameWindowType::Windowed;
    /// Returns the directory the game runs in.
    running_directory => running_directory, "runningDirectory", String, String::new();
    /// Returns the priority of the process of the game.
    process_priority => process_priority, "processPriority", ProcessPriority, ProcessPriority::Normal;
    /// Returns what the launcher does when the game starts.
    launcher_visibility => launcher_visibility, "launcherVisibility", LauncherVisibility, LauncherVisibility::Hide;
    /// Returns whether the launcher may inject its agent into the game.
    allow_auto_agent => allow_auto_agent, "allowAutoAgent", bool, false;
    /// Returns whether the launcher does not write the options of the game by
    /// itself.
    disable_auto_game_options => disable_auto_game_options, "disableAutoGameOptions", bool, false;
    /// Returns the arguments of the game the user set.
    game_arguments => game_arguments, "gameArguments", String, String::new();
    /// Returns the graphics interface the game draws with.
    graphics_backend => graphics_backend, "graphicsBackend", GraphicsApi, GraphicsApi::Default;
    /// Returns the OpenGL driver the game draws with.
    open_gl_renderer => open_gl_renderer, "openGLRenderer", Renderer, Renderer::DEFAULT;
    /// Returns the Vulkan driver the game draws with.
    vulkan_renderer => vulkan_renderer, "vulkanRenderer", Renderer, Renderer::DEFAULT;
    /// Returns whether the game asks for the high performance graphics card.
    high_performance => high_performance, "highPerformance", bool, false;
    /// Returns the environment variables of the game.
    environment_variables => environment_variables, "environmentVariables", String, String::new();
    /// Returns the program that starts the game.
    command_wrapper => command_wrapper, "commandWrapper", String, String::new();
    /// Returns the command the launcher runs before it starts the game.
    pre_launch_command => pre_launch_command, "preLaunchCommand", String, String::new();
    /// Returns the command the launcher runs after the game closes.
    post_exit_command => post_exit_command, "postExitCommand", String, String::new();
    /// Returns the kind of place the game opens when it starts.
    quick_play => quick_play, "quickPlay", QuickPlayType, QuickPlayType::None;
    /// Returns the address of the server the game joins.
    quick_play_multiplayer => quick_play_multiplayer, "quickPlayMultiplayer", String, String::new();
    /// Returns the folder of the world the game opens.
    quick_play_singleplayer => quick_play_singleplayer, "quickPlaySingleplayer", String, String::new();
    /// Returns the identifier of the realm the game joins.
    quick_play_realms => quick_play_realms, "quickPlayRealms", String, String::new();
    /// Returns whether the launcher shows the log of the game after it starts.
    show_logs => show_logs, "showLogs", bool, false;
    /// Returns whether the game writes the log of the debug level.
    enable_debug_log_output => enable_debug_log_output, "enableDebugLogOutput", bool, false;
    /// Returns whether the launcher does not patch the native libraries of the
    /// game.
    not_patch_natives => not_patch_natives, "notPatchNatives", bool, false;
    /// Returns whether the launcher leaves the native libraries to the user.
    use_custom_natives => use_custom_natives, "useCustomNatives", bool, false;
    /// Returns the directory that holds the native libraries of the game.
    natives_directory => natives_directory, "nativesDirectory", String, String::new();
    /// Returns whether the game uses the native GLFW or SDL library.
    use_native_glfw_or_sdl => use_native_glfw_or_sdl, "useNativeGLFW", bool, false;
    /// Returns whether the game uses the native OpenAL library.
    use_native_openal => use_native_openal, "useNativeOpenAL", bool, false;
}

impl Effective<'_> {
    /// Returns the smallest amount of memory the game keeps, in MiB.
    ///
    /// The value is absent unless the user chose one.
    pub fn min_memory(&self) -> Option<u32> {
        match self.instance {
            Some(instance) if instance.is_overridden("minMemory") => instance.settings.min_memory,
            _ => self.preset.settings.min_memory,
        }
    }

    /// Returns the amount of memory the game may use, in MiB.
    ///
    /// A value that is not positive yields the amount the launcher suggests.
    pub fn max_memory(&self) -> u32 {
        let value = self.max_memory_setting();
        if value > 0 {
            value
        } else {
            suggested_memory()
        }
    }

    /// Returns the width of the window of the game, in pixels.
    pub fn width(&self) -> u32 {
        let width = match self.instance {
            Some(instance) if instance.is_overridden("width") => {
                instance.settings.width.unwrap_or(854.0)
            }
            _ => self.preset.settings.width.unwrap_or(854.0),
        };

        if width.is_finite() {
            width.round().max(0.0) as u32
        } else {
            0
        }
    }

    /// Returns the height of the window of the game, in pixels.
    pub fn height(&self) -> u32 {
        let height = match self.instance {
            Some(instance) if instance.is_overridden("height") => {
                instance.settings.height.unwrap_or(480.0)
            }
            _ => self.preset.settings.height.unwrap_or(480.0),
        };

        if height.is_finite() {
            height.round().max(0.0) as u32
        } else {
            0
        }
    }

    /// Returns the driver the game draws with for a version of the game.
    ///
    /// A driver that belongs to the other graphics interface than the one the
    /// launcher asks for is replaced by the default driver.
    pub fn renderer(&self, version: &crate::util::versioning::GameVersionNumber) -> Renderer {
        let api = match self.graphics_backend() {
            GraphicsApi::Default => GraphicsApi::default_for_version(version),
            api => api,
        };

        let renderer = match api {
            GraphicsApi::Default | GraphicsApi::OpenGl => self.open_gl_renderer(),
            GraphicsApi::Vulkan => self.vulkan_renderer(),
        };

        match renderer.api() {
            Some(renderer_api) if renderer_api != api => Renderer::DEFAULT,
            _ => renderer,
        }
    }

    /// Returns the place the game opens when it starts, if the user chose one.
    ///
    /// A quick play option without a target is ignored.
    pub fn quick_play_option(&self) -> Option<QuickPlayOption> {
        match self.quick_play() {
            QuickPlayType::None => None,
            QuickPlayType::Multiplayer => {
                let server = self.quick_play_multiplayer();
                (!server.trim().is_empty()).then_some(QuickPlayOption::MultiPlayer(server))
            }
            QuickPlayType::Singleplayer => {
                let world = self.quick_play_singleplayer();
                (!world.trim().is_empty()).then_some(QuickPlayOption::SinglePlayer(world))
            }
            QuickPlayType::Realms => {
                let realm = self.quick_play_realms();
                (!realm.trim().is_empty()).then_some(QuickPlayOption::Realm(realm))
            }
        }
    }

    /// Returns the Java runtime the settings choose.
    ///
    /// The runtime is chosen from the runtimes the launcher found. An
    /// automatically chosen runtime fits the version of the game; a runtime
    /// chosen by version must have that version; a runtime chosen from the
    /// detected ones must have the version and, when the settings know the
    /// path, the same executable; a custom runtime is read from the path the
    /// user entered.
    pub fn find_java(
        &self,
        runtimes: &[JavaRuntime],
        game_version: &GameVersionNumber,
        manifest: Option<&InstanceManifest>,
    ) -> Option<JavaRuntime> {
        match self.java_type() {
            JavaVersionType::Auto => find_suitable_java(runtimes, game_version, manifest),
            JavaVersionType::Custom => {
                let path = self.custom_java_path();
                if path.trim().is_empty() {
                    return None;
                }
                JavaRuntime::detect(std::path::Path::new(&path), false).ok()
            }
            JavaVersionType::Version => {
                let version = self.custom_java_version();
                if version.trim().is_empty() {
                    return find_suitable_java(runtimes, game_version, manifest);
                }

                let major: i32 = match version.trim().parse() {
                    Ok(major) if major >= 0 => major,
                    _ => return None,
                };

                let matching: Vec<JavaRuntime> = runtimes
                    .iter()
                    .filter(|runtime| runtime.parsed_version() == major)
                    .cloned()
                    .collect();

                find_suitable_java(&matching, game_version, manifest)
            }
            JavaVersionType::Detected => {
                let detected = self.detected_java();
                if detected.version.trim().is_empty() {
                    return find_suitable_java(runtimes, game_version, manifest);
                }

                if !detected.path_hash.trim().is_empty() {
                    if let Some(runtime) = runtimes.iter().find(|runtime| {
                        runtime.version() == detected.version
                            && DetectedJava::hash_existing_path(runtime.binary())
                                == detected.path_hash
                    }) {
                        return Some(runtime.clone());
                    }
                }

                runtimes
                    .iter()
                    .find(|runtime| runtime.version() == detected.version)
                    .cloned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::quick_play::QuickPlayType;

    /// Returns a preset with the identifier used by the tests.
    fn preset() -> Preset {
        Preset::new(
            GameSettingsPresetId::parse(
                "game-settings-preset:123e4567-e89b-12d3-a456-426614174000",
            )
            .expect("the identifier should be read"),
        )
    }

    #[test]
    fn the_identifier_of_a_preset_is_read_and_written() {
        let text = "game-settings-preset:123e4567-e89b-12d3-a456-426614174000";
        let id = GameSettingsPresetId::parse(text).expect("the identifier should be read");

        assert_eq!(id.to_string(), text);
        assert_eq!(
            id.uuid().to_string(),
            "123e4567-e89b-12d3-a456-426614174000"
        );
        assert!(!id.is_nil());
        assert!(GameSettingsPresetId::parse("123e4567-e89b-12d3-a456-426614174000").is_err());
        assert!(GameSettingsPresetId::parse("game-settings-preset:not-a-uuid").is_err());
    }

    #[test]
    fn a_generated_preset_identifier_is_a_version_7_identifier() {
        let first = GameSettingsPresetId::generate();
        let second = GameSettingsPresetId::generate();

        assert!(!first.is_nil());
        assert_eq!(first.uuid().version(), 7);
        assert_ne!(first, second);
    }

    #[test]
    fn a_preset_must_have_a_usable_identifier() {
        assert!(serde_json::from_str::<Preset>("{}").is_err());
        assert!(serde_json::from_str::<Preset>(
            r#"{"id":"game-settings-preset:00000000-0000-0000-0000-000000000000"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Preset>(
            r#"{"id":"game-settings-preset:123e4567-e89b-12d3-a456-426614174000"}"#
        )
        .is_ok());
    }

    #[test]
    fn an_instance_inherits_the_settings_it_does_not_override() {
        let mut parent = preset();
        parent.settings.max_memory = Some(4096);
        parent.settings.window_type = Some(GameWindowType::Fullscreen);

        let mut instance = InstanceSettings::new();
        instance.settings.max_memory = Some(1024);
        instance.set_overridden("maxMemory", true);

        let effective = parent.resolve(Some(&instance));
        assert_eq!(effective.max_memory(), 1024);
        assert_eq!(effective.window_type(), GameWindowType::Fullscreen);
        assert_eq!(effective.width(), 854);
        assert_eq!(effective.height(), 480);
    }

    #[test]
    fn an_instance_value_that_is_not_overridden_is_ignored() {
        let mut parent = preset();
        parent.settings.max_memory = Some(4096);

        let mut instance = InstanceSettings::new();
        instance.settings.max_memory = Some(1024);
        instance.settings.min_memory = Some(512);

        let effective = parent.resolve(Some(&instance));
        assert_eq!(effective.max_memory(), 4096);
        assert_eq!(effective.min_memory(), None);
    }

    #[test]
    fn the_java_payload_of_an_instance_inherits_independently() {
        let mut parent = preset();
        parent.settings.java_type = Some(JavaVersionType::Version);
        parent.settings.custom_java_version = Some("17".to_string());
        parent.settings.custom_java_path = Some("/parent/java".to_string());
        parent.settings.detected_java = Some(DetectedJava {
            version: "17.0.11+9".to_string(),
            path_hash: "parent-hash".to_string(),
        });

        let mut instance = InstanceSettings::new();
        instance.settings.java_type = Some(JavaVersionType::Custom);
        instance.settings.custom_java_version = Some("21".to_string());
        instance.settings.custom_java_path = Some("/instance/java".to_string());
        instance.settings.detected_java = Some(DetectedJava {
            version: "21.0.1+12".to_string(),
            path_hash: "instance-hash".to_string(),
        });
        instance.set_overridden("javaType", true);
        instance.set_overridden("customJavaPath", true);

        let effective = parent.resolve(Some(&instance));
        assert_eq!(effective.java_type(), JavaVersionType::Custom);
        assert_eq!(effective.custom_java_version(), "17");
        assert_eq!(effective.custom_java_path(), "/instance/java");
        assert_eq!(effective.detected_java().version, "17.0.11+9");
    }

    #[test]
    fn an_instance_writes_the_names_of_the_settings_of_the_java_launcher() {
        let mut instance = InstanceSettings::new();
        instance.schema = INSTANCE_GAME_SETTINGS_SCHEMA.to_string();
        instance.settings.running_directory = Some("run".to_string());
        instance.settings.use_custom_natives = Some(true);
        instance.settings.natives_directory = Some("natives".to_string());

        let json = serde_json::to_value(&instance).expect("the settings should be written");
        assert_eq!(json["runningDirectory"], "run");
        assert_eq!(json["useCustomNatives"], true);
        assert_eq!(json["nativesDirectory"], "natives");
        assert!(json.get("runningDir").is_none());
        assert!(json.get("nativesDirType").is_none());
        assert!(json.get("nativesDir").is_none());
    }

    #[test]
    fn the_renderers_are_written_by_name() {
        let mut instance = InstanceSettings::new();
        instance.settings.open_gl_renderer = Some(Renderer::Llvmpipe);
        instance.settings.vulkan_renderer = Some(Renderer::Lavapipe);

        let json = serde_json::to_value(&instance).expect("the settings should be written");
        assert_eq!(json["openGLRenderer"], "LLVMPIPE");
        assert_eq!(json["vulkanRenderer"], "LAVAPIPE");

        let read: InstanceSettings =
            serde_json::from_value(json).expect("the settings should be read");
        assert_eq!(read.settings.open_gl_renderer, Some(Renderer::Llvmpipe));
        assert_eq!(read.settings.vulkan_renderer, Some(Renderer::Lavapipe));
    }

    #[test]
    fn a_non_positive_maximum_memory_yields_the_suggested_memory() {
        let mut parent = preset();
        parent.settings.max_memory = Some(0);

        assert_eq!(parent.resolve(None).max_memory(), suggested_memory());
    }

    #[test]
    fn the_window_is_measured_in_pixels() {
        let mut parent = preset();
        parent.settings.width = Some(1280.4);
        parent.settings.height = Some(720.6);

        let effective = parent.resolve(None);
        assert_eq!(effective.width(), 1280);
        assert_eq!(effective.height(), 721);

        parent.settings.width = Some(-10.0);
        assert_eq!(parent.resolve(None).width(), 0);
    }

    #[test]
    fn the_renderer_belongs_to_the_chosen_graphics_interface() {
        let mut parent = preset();
        parent.settings.graphics_backend = Some(GraphicsApi::Vulkan);
        parent.settings.open_gl_renderer = Some(Renderer::Llvmpipe);
        parent.settings.vulkan_renderer = Some(Renderer::Lavapipe);

        let version =
            crate::util::versioning::GameVersionNumber::as_game_version("26.2-snapshot-5");
        assert_eq!(parent.resolve(None).renderer(&version), Renderer::Lavapipe);

        parent.settings.graphics_backend = Some(GraphicsApi::OpenGl);
        assert_eq!(parent.resolve(None).renderer(&version), Renderer::Llvmpipe);

        // A Vulkan driver is not used when OpenGL is asked for.
        parent.settings.open_gl_renderer = Some(Renderer::Lavapipe);
        assert_eq!(parent.resolve(None).renderer(&version), Renderer::DEFAULT);
    }

    #[test]
    fn the_quick_play_option_needs_a_target() {
        let mut parent = preset();
        parent.settings.quick_play = Some(QuickPlayType::Multiplayer);
        assert_eq!(parent.resolve(None).quick_play_option(), None);

        parent.settings.quick_play_multiplayer = Some("example.org".to_string());
        assert_eq!(
            parent.resolve(None).quick_play_option(),
            Some(QuickPlayOption::MultiPlayer("example.org".to_string()))
        );
    }

    #[test]
    fn the_detected_java_path_is_hashed_without_the_windows_prefix() {
        let path = Path::new("C:\\Program Files\\Java\\bin\\java.exe");
        let hash = DetectedJava::hash_path_text(&path.to_string_lossy());
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|character| character.is_ascii_hexdigit()));

        let other = DetectedJava::hash_path_text("C:/Program Files/Java/bin/java.exe");
        if current_os() == OsType::Windows {
            assert_eq!(hash, other);
        }
    }

    #[test]
    fn an_empty_detected_java_is_empty() {
        assert!(DetectedJava::EMPTY.is_empty());
        assert!(DetectedJava::of_legacy_path("", "").is_empty());
        assert!(!DetectedJava::of_legacy_path("17", "").is_empty());
    }
}
