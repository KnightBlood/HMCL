//! The migration of the game settings of an older launcher.
//!
//! The older launcher kept the settings of the game in the profile of a game
//! directory and, for an instance that had settings of its own, in a file of
//! the instance. The launcher of today keeps the settings of the game in
//! presets that the instances inherit from, so the settings of a profile
//! become a preset and the settings of an instance become the settings the
//! instance overrides.
//!
//! The conversion is pure: it takes the object the older launcher wrote and
//! returns the preset or the settings of the instance. The caller decides
//! whether the files are written, which keeps the conversion testable and lets
//! the launcher of today apply it before it reads any setting.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::game_settings::{
    suggested_memory, DefaultIsolationType, DetectedJava, GameInstanceIconType,
    GameSettingsPresetId, GameSettingsValues, GameWindowType, InstanceSettings, JavaVersionType,
    LauncherVisibility, Preset,
};
use super::game_settings_presets::GameSettingsPresets;
use super::legacy_migration::{
    legacy_source_digest, member_bool, member_number, member_ordinal, member_string,
    migration_receipt_matches,
};
use crate::error::Error;
use crate::game::graphics::{GraphicsApi, Renderer};
use crate::game::process_priority::ProcessPriority;
use crate::game::quick_play::QuickPlayType;

/// The name of the file the older launcher kept the settings of an instance
/// in.
pub const LEGACY_INSTANCE_SETTINGS_FILE: &str = "hmclversion.cfg";

/// The name of the receipt that records the migration of the settings of an
/// instance.
pub const LEGACY_INSTANCE_SETTINGS_RECEIPT_FILE: &str =
    "instance-game-settings.migration-receipt.json";

/// The directory of the metadata of an instance.
pub const INSTANCE_METADATA_DIRECTORY: &str = ".hmcl";

/// The directory of the configuration files of an instance.
pub const INSTANCE_CONFIG_DIRECTORY: &str = "config";

/// The directory of the state files of an instance.
pub const INSTANCE_STATE_DIRECTORY: &str = "state";

/// The names of the icons the older launcher numbered.
///
/// The older launcher wrote the icon as a number; the names are the ones the
/// launcher of Java reads the numbers as, in the order it reads them.
const LEGACY_VERSION_ICON_TYPES: [GameInstanceIconType; 15] = [
    GameInstanceIconType::Default,
    GameInstanceIconType::Grass,
    GameInstanceIconType::Chest,
    GameInstanceIconType::Chicken,
    GameInstanceIconType::Command,
    GameInstanceIconType::Optifine,
    GameInstanceIconType::CraftTable,
    GameInstanceIconType::Fabric,
    GameInstanceIconType::Forge,
    GameInstanceIconType::NeoForge,
    GameInstanceIconType::Furnace,
    GameInstanceIconType::Quilt,
    GameInstanceIconType::AprilFools,
    GameInstanceIconType::Cleanroom,
    GameInstanceIconType::LegacyFabric,
];

/// The kinds of the runtime the older launcher numbered.
///
/// The numbers zero and one were the automatic choice and the choice of the
/// launcher itself, which are the same choice by now, so both are read as the
/// automatic one.
const LEGACY_JAVA_VERSION_TYPES: [&str; 5] = ["AUTO", "AUTO", "VERSION", "DETECTED", "CUSTOM"];

/// The kinds of the directory the game runs in that the older launcher
/// numbered.
const LEGACY_NATIVES_DIRECTORY_TYPES: [&str; 2] = ["VERSION_FOLDER", "CUSTOM"];

/// The kinds of the directory the game runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyGameDirectoryType {
    /// The directory of the game directory itself.
    RootFolder,
    /// The directory of the version of the game.
    VersionFolder,
    /// The directory the player chose.
    Custom,
}

impl LegacyGameDirectoryType {
    /// Returns the kind with a number, in the order the older launcher wrote
    /// them.
    const fn from_ordinal(ordinal: usize) -> Option<Self> {
        match ordinal {
            0 => Some(Self::RootFolder),
            1 => Some(Self::VersionFolder),
            2 => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Returns the string of a member, or the default when the member is missing
/// or is not a string.
fn member_string_or(object: &Map<String, Value>, name: &str, default: &str) -> String {
    member_string(object, name).unwrap_or_else(|| default.to_string())
}

/// Returns the number of a member as an integer, or the default when the
/// member is missing or is not a whole number.
fn member_int(object: &Map<String, Value>, name: &str, default: i64) -> i64 {
    match member_number(object, name) {
        Some(number) if number.is_finite() => number as i64,
        _ => default,
    }
}

/// Returns the preset a profile of the older launcher becomes.
///
/// The settings the older launcher kept for every profile are read into the
/// preset, so the instances of the directory inherit them. A profile that kept
/// the files of every instance apart becomes a preset that asks for the same.
pub fn legacy_preset(
    id: GameSettingsPresetId,
    auto_name_number: u32,
    source: Option<&Map<String, Value>>,
) -> Preset {
    let mut preset = Preset::new(id);
    preset.auto_name_number = Some(auto_name_number);

    if legacy_game_directory_type(source) == LegacyGameDirectoryType::VersionFolder {
        preset.default_isolation_type = Some(DefaultIsolationType::Always);
    }

    if let Some(source) = source {
        copy_common_settings(source, &mut preset.settings);
    }

    preset
}

/// Returns the settings an instance of the older launcher becomes.
///
/// An instance that inherited the settings of its profile keeps inheriting
/// them, so the settings the player changed in the older launcher are read
/// from the preset of the profile. An instance that kept its own copy of the
/// settings overrides every setting the older launcher knew, which is what the
/// launcher of Java does, so the preset does not change the settings of the
/// instance later.
pub fn legacy_instance_settings(
    parent: Option<GameSettingsPresetId>,
    parent_preset: Option<&Preset>,
    source: Option<&Map<String, Value>>,
    inherits_legacy_parent: bool,
) -> InstanceSettings {
    let mut instance = InstanceSettings::new();
    instance.parent = parent;
    instance.icon = Some(legacy_icon_type(source));

    // The instance of a profile that keeps the files of every instance apart
    // runs in a directory of its own unless the preset names one.
    if inherits_legacy_parent
        && parent_preset.is_some_and(|preset| {
            preset.default_isolation_type() == DefaultIsolationType::Always
                && preset
                    .settings
                    .running_directory
                    .as_deref()
                    .is_none_or(str::is_empty)
        })
    {
        instance.set_overridden(RUNNING_DIRECTORY, true);
    }

    if let Some(source) = source {
        if !inherits_legacy_parent {
            copy_common_settings(source, &mut instance.settings);
            if legacy_game_directory_type(Some(source)) != LegacyGameDirectoryType::RootFolder {
                instance.set_overridden(RUNNING_DIRECTORY, true);
            }
            for name in OVERRIDDEN_SETTINGS {
                instance.set_overridden(name, true);
            }
        }
    }

    instance
}

/// The name of the setting of the directory the game runs in.
const RUNNING_DIRECTORY: &str = "runningDirectory";

/// The settings an instance of the older launcher kept a copy of.
///
/// The older launcher wrote every setting into the file of an instance, so an
/// instance that did not inherit the settings of its profile overrides all of
/// them; a setting the launcher of today adds later is read from the preset.
const OVERRIDDEN_SETTINGS: [&str; 38] = [
    "javaType",
    "customJavaVersion",
    "customJavaPath",
    "detectedJava",
    "jvmOptions",
    "noJVMOptions",
    "noOptimizingJVMOptions",
    "notCheckJVM",
    "notCheckGame",
    "autoMemory",
    "minMemory",
    "maxMemory",
    "permSize",
    "windowType",
    "width",
    "height",
    "processPriority",
    "launcherVisibility",
    "gameArguments",
    "graphicsBackend",
    "openGLRenderer",
    "vulkanRenderer",
    "environmentVariables",
    "commandWrapper",
    "preLaunchCommand",
    "postExitCommand",
    "quickPlay",
    "quickPlayMultiplayer",
    "quickPlaySingleplayer",
    "quickPlayRealms",
    "showLogs",
    "enableDebugLogOutput",
    "notPatchNatives",
    "useCustomNatives",
    "nativesDirectory",
    "useNativeGLFW",
    "useNativeOpenAL",
    "runningDirectory",
];

/// Copies the settings the older launcher kept for a profile or an instance.
fn copy_common_settings(source: &Map<String, Value>, target: &mut GameSettingsValues) {
    let java_type = legacy_java_version_type(source);
    let legacy_java_version = legacy_java_version(source);
    target.java_type = Some(java_type);
    match java_type {
        JavaVersionType::Version => {
            target.custom_java_version = Some(legacy_java_version);
        }
        JavaVersionType::Detected if !legacy_java_version.trim().is_empty() => {
            target.detected_java = Some(DetectedJava::of_legacy_path(
                &legacy_java_version,
                &member_string_or(source, "defaultJavaPath", ""),
            ));
        }
        _ => {}
    }

    target.custom_java_path = Some(member_string_or(source, "javaDir", ""));
    target.jvm_options = Some(member_string_or(source, "javaArgs", ""));
    target.no_jvm_options = Some(member_bool(source, "noJVMArgs", false));
    target.no_optimizing_jvm_options = Some(member_bool(source, "noOptimizingJVMArgs", false));
    target.not_check_jvm = Some(member_bool(source, "notCheckJVM", false));
    target.not_check_game = Some(member_bool(source, "notCheckGame", false));

    target.auto_memory = Some(member_bool(source, "autoMemory", true));
    target.min_memory = member_number(source, "minMemory")
        .map(|memory| memory.max(0.0) as u32)
        .filter(|memory| *memory > 0);
    let max_memory = member_int(source, "maxMemory", suggested_memory() as i64);
    target.max_memory = Some(if max_memory > 0 {
        max_memory as u32
    } else {
        suggested_memory()
    });
    target.perm_size = Some(member_string_or(source, "permSize", ""));

    target.window_type = Some(if member_bool(source, "fullscreen", false) {
        GameWindowType::Fullscreen
    } else {
        GameWindowType::Windowed
    });
    target.width = Some(member_int(source, "width", 0).max(0) as f64);
    target.height = Some(member_int(source, "height", 0).max(0) as f64);
    target.running_directory = Some(
        if legacy_game_directory_type(Some(source)) == LegacyGameDirectoryType::Custom {
            member_string_or(source, "gameDir", "")
        } else {
            String::new()
        },
    );

    target.process_priority = Some(legacy_process_priority(source));
    target.launcher_visibility = Some(legacy_launcher_visibility(source));
    target.game_arguments = Some(member_string_or(source, "minecraftArgs", ""));

    let renderer = legacy_renderer(source);
    let graphics_backend = legacy_graphics_backend(source, &renderer);
    target.graphics_backend = Some(graphics_backend);
    match graphics_backend {
        GraphicsApi::OpenGl => target.open_gl_renderer = Some(renderer.clone()),
        GraphicsApi::Vulkan => target.vulkan_renderer = Some(renderer.clone()),
        GraphicsApi::Default => {
            target.open_gl_renderer = Some(renderer.clone());
            target.vulkan_renderer = Some(renderer.clone());
        }
    }

    target.environment_variables = Some(member_string_or(source, "environmentVariables", ""));
    target.command_wrapper = Some(member_string_or(source, "wrapper", ""));
    target.pre_launch_command = Some(member_string_or(source, "precalledCommand", ""));
    target.post_exit_command = Some(member_string_or(source, "postExitCommand", ""));

    let server_ip = member_string_or(source, "serverIp", "");
    if server_ip.trim().is_empty() {
        target.quick_play = Some(QuickPlayType::None);
    } else {
        target.quick_play = Some(QuickPlayType::Multiplayer);
        target.quick_play_multiplayer = Some(server_ip);
    }

    target.show_logs = Some(member_bool(source, "showLogs", false));
    target.enable_debug_log_output = Some(member_bool(source, "enableDebugLogOutput", false));
    target.not_patch_natives = Some(member_bool(source, "notPatchNatives", false));

    let custom_natives = legacy_custom_natives_directory(source);
    target.use_custom_natives = Some(custom_natives);
    target.natives_directory = Some(if custom_natives {
        member_string_or(source, "nativesDir", "")
    } else {
        String::new()
    });

    target.use_native_glfw_or_sdl = Some(member_bool(source, "useNativeGLFW", false));
    target.use_native_openal = Some(member_bool(source, "useNativeOpenAL", false));
}

/// Returns the kind of the directory the game runs in.
fn legacy_game_directory_type(source: Option<&Map<String, Value>>) -> LegacyGameDirectoryType {
    let Some(source) = source else {
        return LegacyGameDirectoryType::RootFolder;
    };

    match source.get("gameDirType") {
        Some(Value::Number(number)) => number
            .as_u64()
            .and_then(|ordinal| LegacyGameDirectoryType::from_ordinal(ordinal as usize))
            .unwrap_or(LegacyGameDirectoryType::RootFolder),
        Some(Value::String(name)) => match name.to_ascii_uppercase().as_str() {
            "VERSION_FOLDER" => LegacyGameDirectoryType::VersionFolder,
            "CUSTOM" => LegacyGameDirectoryType::Custom,
            _ => LegacyGameDirectoryType::RootFolder,
        },
        _ => LegacyGameDirectoryType::RootFolder,
    }
}

/// Returns the way the older launcher chose the runtime of the game.
fn legacy_java_version_type(source: &Map<String, Value>) -> JavaVersionType {
    use JavaVersionType;

    if let Some(value) = source.get("javaVersionType") {
        if let Some(ordinal) = member_ordinal(source, "javaVersionType") {
            return match LEGACY_JAVA_VERSION_TYPES.get(ordinal) {
                Some(&"VERSION") => JavaVersionType::Version,
                Some(&"DETECTED") => JavaVersionType::Detected,
                Some(&"CUSTOM") => JavaVersionType::Custom,
                _ => JavaVersionType::Auto,
            };
        }

        if let Some(name) = value.as_str() {
            return match name.to_ascii_uppercase().as_str() {
                "VERSION" => JavaVersionType::Version,
                "DETECTED" => JavaVersionType::Detected,
                "CUSTOM" => JavaVersionType::Custom,
                _ => JavaVersionType::Auto,
            };
        }
    }

    match member_string_or(source, "java", "").as_str() {
        "Custom" => JavaVersionType::Custom,
        _ => JavaVersionType::Auto,
    }
}

/// Returns the version of the runtime the older launcher wrote.
fn legacy_java_version(source: &Map<String, Value>) -> String {
    if source.contains_key("javaVersionType") {
        return match legacy_java_version_type(source) {
            JavaVersionType::Version | JavaVersionType::Detected => {
                member_string_or(source, "java", "")
            }
            _ => String::new(),
        };
    }

    match member_string_or(source, "java", "").as_str() {
        "Default" | "Auto" | "Custom" => String::new(),
        java => java.to_string(),
    }
}

/// Returns the driver the older launcher chose.
fn legacy_renderer(source: &Map<String, Value>) -> Renderer {
    if let Some(renderer) = source.get("renderer") {
        let name = match renderer {
            Value::String(name) => Some(name.as_str()),
            Value::Object(object) => object.get("name").and_then(Value::as_str),
            _ => None,
        };
        if let Some(name) = name {
            return Renderer::parse(name);
        }
    }

    if member_bool(source, "useSoftwareRenderer", false) {
        Renderer::Llvmpipe
    } else {
        Renderer::DEFAULT
    }
}

/// Returns the interface the game draws with.
///
/// The older launcher did not name the interface in most of its versions, so
/// the interface of the driver it chose is used; a driver the launcher of
/// today does not know leaves the interface to the launcher.
fn legacy_graphics_backend(source: &Map<String, Value>, renderer: &Renderer) -> GraphicsApi {
    if let Some(name) = member_string(source, "graphicsBackend") {
        match name.to_ascii_uppercase().as_str() {
            "DEFAULT" => return GraphicsApi::Default,
            "OPENGL" => return GraphicsApi::OpenGl,
            "VULKAN" => return GraphicsApi::Vulkan,
            _ => {}
        }
    }

    renderer.api().unwrap_or(GraphicsApi::Default)
}

/// Returns whether the older launcher selected the native libraries of the
/// player.
fn legacy_custom_natives_directory(source: &Map<String, Value>) -> bool {
    match source.get("nativesDirType") {
        Some(Value::Number(_)) => member_ordinal(source, "nativesDirType")
            .and_then(|ordinal| LEGACY_NATIVES_DIRECTORY_TYPES.get(ordinal))
            .is_some_and(|name| *name == "CUSTOM"),
        Some(Value::String(name)) => name.eq_ignore_ascii_case("CUSTOM"),
        _ => false,
    }
}

/// Returns the icon of an instance.
fn legacy_icon_type(source: Option<&Map<String, Value>>) -> GameInstanceIconType {
    let Some(source) = source else {
        return GameInstanceIconType::Default;
    };

    if let Some(ordinal) = member_ordinal(source, "versionIcon") {
        return LEGACY_VERSION_ICON_TYPES
            .get(ordinal)
            .copied()
            .unwrap_or(GameInstanceIconType::Default);
    }

    let Some(name) = member_string(source, "versionIcon") else {
        return GameInstanceIconType::Default;
    };
    LEGACY_VERSION_ICON_TYPES
        .iter()
        .copied()
        .find(|icon| icon.name().eq_ignore_ascii_case(&name))
        .unwrap_or(GameInstanceIconType::Default)
}

/// Returns the priority of the process of the game.
fn legacy_process_priority(source: &Map<String, Value>) -> ProcessPriority {
    const NAMES: [ProcessPriority; 5] = [
        ProcessPriority::Low,
        ProcessPriority::BelowNormal,
        ProcessPriority::Normal,
        ProcessPriority::AboveNormal,
        ProcessPriority::High,
    ];

    parse_legacy_enum(source, "processPriority", &NAMES, ProcessPriority::Normal)
}

/// Returns what the launcher does when the game starts.
fn legacy_launcher_visibility(source: &Map<String, Value>) -> LauncherVisibility {
    const NAMES: [LauncherVisibility; 4] = [
        LauncherVisibility::Close,
        LauncherVisibility::Hide,
        LauncherVisibility::Keep,
        LauncherVisibility::HideAndReopen,
    ];

    parse_legacy_enum(
        source,
        "launcherVisibility",
        &NAMES,
        LauncherVisibility::Hide,
    )
}

/// Reads an item of a list of items the older launcher wrote either by its
/// number or by its name.
fn parse_legacy_enum<T: Copy + NamedItem>(
    source: &Map<String, Value>,
    name: &str,
    items: &[T],
    default: T,
) -> T {
    if let Some(ordinal) = member_ordinal(source, name) {
        return items.get(ordinal).copied().unwrap_or(default);
    }

    let Some(name) = member_string(source, name) else {
        return default;
    };
    items
        .iter()
        .copied()
        .find(|item| item.name().eq_ignore_ascii_case(&name))
        .unwrap_or(default)
}

/// An item the older launcher wrote by its name.
trait NamedItem {
    /// Returns the name the older launcher wrote.
    fn name(&self) -> &'static str;
}

impl NamedItem for ProcessPriority {
    fn name(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::BelowNormal => "BELOW_NORMAL",
            Self::Normal => "NORMAL",
            Self::AboveNormal => "ABOVE_NORMAL",
            Self::High => "HIGH",
        }
    }
}

impl NamedItem for LauncherVisibility {
    fn name(&self) -> &'static str {
        match self {
            Self::Close => "CLOSE",
            Self::Hide => "HIDE",
            Self::Keep => "KEEP",
            Self::HideAndReopen => "HIDE_AND_REOPEN",
        }
    }
}

impl NamedItem for GameInstanceIconType {
    fn name(&self) -> &'static str {
        match self {
            Self::Default => "DEFAULT",
            Self::Grass => "GRASS",
            Self::Chest => "CHEST",
            Self::Chicken => "CHICKEN",
            Self::Command => "COMMAND",
            Self::Optifine => "OPTIFINE",
            Self::CraftTable => "CRAFT_TABLE",
            Self::Fabric => "FABRIC",
            Self::Forge => "FORGE",
            Self::NeoForge => "NEO_FORGE",
            Self::Furnace => "FURNACE",
            Self::Quilt => "QUILT",
            Self::AprilFools => "APRIL_FOOLS",
            Self::Cleanroom => "CLEANROOM",
            Self::LegacyFabric => "LEGACY_FABRIC",
        }
    }
}

/// Returns the path of the file the older launcher kept the settings of an
/// instance in.
pub fn legacy_instance_settings_path(instance_root: &Path) -> PathBuf {
    instance_root.join(LEGACY_INSTANCE_SETTINGS_FILE)
}

/// Returns the path of the receipt that records the migration of the settings
/// of an instance.
pub fn legacy_instance_settings_receipt_path(instance_root: &Path) -> PathBuf {
    instance_root
        .join(INSTANCE_METADATA_DIRECTORY)
        .join(INSTANCE_STATE_DIRECTORY)
        .join(LEGACY_INSTANCE_SETTINGS_RECEIPT_FILE)
}

/// The settings of an instance that were migrated out of the file of the older
/// launcher.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyInstanceSettingsMigration {
    /// The path of the file the settings were read from.
    pub path: PathBuf,
    /// The path of the receipt that records the migration.
    pub receipt_path: PathBuf,
    /// The settings of the instance.
    pub settings: InstanceSettings,
}

/// Reads the settings an older launcher kept for one instance.
///
/// The settings are `None` when the instance has no file of the older
/// launcher, when the file was migrated before, or when the instance already
/// has settings of its own, so the settings of the player are never replaced.
pub fn migrate_legacy_instance_settings(
    instance_root: &Path,
    parent: Option<GameSettingsPresetId>,
    parent_preset: Option<&Preset>,
    base_directory: Option<&Path>,
) -> Result<Option<LegacyInstanceSettingsMigration>, Error> {
    let path = legacy_instance_settings_path(instance_root);
    if !path.is_file() {
        return Ok(None);
    }

    // The instance of the launcher of today has its settings in a file of its
    // own; a file the player wrote is not replaced.
    if super::game_settings::instance_game_settings_path(instance_root).exists() {
        return Ok(None);
    }

    let text = std::fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
    let receipt_path = legacy_instance_settings_receipt_path(instance_root);
    if let Ok(receipt) = std::fs::read_to_string(&receipt_path) {
        if migration_receipt_matches(&receipt, &text) {
            return Ok(None);
        }
    }

    let Some(source) = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|value| value.as_object().cloned())
    else {
        return Ok(None);
    };

    let inherits_legacy_parent = member_bool(&source, "usesGlobal", false);
    let mut settings =
        legacy_instance_settings(parent, parent_preset, Some(&source), inherits_legacy_parent);

    // An instance that kept its own settings but ran in the directory of the
    // game directory keeps doing so, even when the preset names a directory
    // for the instances that inherit it.
    if !inherits_legacy_parent
        && legacy_game_directory_type(Some(&source)) == LegacyGameDirectoryType::RootFolder
    {
        if let (Some(preset), Some(base_directory)) = (parent_preset, base_directory) {
            if preset
                .settings
                .running_directory
                .as_deref()
                .is_some_and(|directory| !directory.trim().is_empty())
            {
                settings.settings.running_directory =
                    Some(base_directory.to_string_lossy().into_owned());
                settings.set_overridden(RUNNING_DIRECTORY, true);
            }
        }
    }

    Ok(Some(LegacyInstanceSettingsMigration {
        path,
        receipt_path,
        settings,
    }))
}

/// Returns the text of the receipt that records the migration of the settings
/// of an instance.
///
/// The receipt records the fingerprint of the file, so the launcher does not
/// read the same file again, while a file the player changed in the older
/// launcher is read again.
pub fn legacy_instance_settings_receipt(text: &str, migrated_at: &str) -> String {
    let receipt = serde_json::json!({
        "sourceSize": text.len(),
        "sourceSHA256": legacy_source_digest(text),
        "migratedAt": migrated_at,
    });

    serde_json::to_string_pretty(&receipt).unwrap_or_default()
}

/// Reads the settings an older launcher kept for every instance of a game
/// directory.
///
/// An instance is the directory under `versions` that holds the file of the
/// version of the game, which is named after the directory. The settings of
/// the instances are returned in the order the directories are read; an
/// instance that has no file of the older launcher, that was migrated before
/// or that already has settings of its own is not returned.
///
/// The preset of the instance is the preset the profile of the game directory
/// became, so the settings the player changed for the profile are inherited.
pub fn migrate_legacy_instances(
    game_directory: &Path,
    presets: &GameSettingsPresets,
    parent: Option<GameSettingsPresetId>,
) -> Result<Vec<LegacyInstanceSettingsMigration>, Error> {
    let versions = game_directory.join("versions");
    let entries = match std::fs::read_dir(&versions) {
        Ok(entries) => entries,
        // A directory without versions has nothing to migrate, which is not a
        // failure: the player may have removed every instance.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(Error::io(&versions, error)),
    };

    let parent_preset = parent.and_then(|id| presets.get_preset(id));
    let mut migrations = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(&versions, error))?;
        let instance_root = entry.path();
        if !instance_root.is_dir() {
            continue;
        }

        // An instance is named after the file of the version of the game, so a
        // directory without one is not an instance.
        let Some(name) = instance_root.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !instance_root.join(format!("{name}.json")).is_file() {
            continue;
        }

        if let Some(migration) = migrate_legacy_instance_settings(
            &instance_root,
            parent,
            parent_preset,
            Some(game_directory),
        )? {
            migrations.push(migration);
        }
    }

    Ok(migrations)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the object of a legacy file.
    fn source(text: &str) -> Map<String, Value> {
        serde_json::from_str::<Value>(text)
            .expect("the file should be read")
            .as_object()
            .expect("the file should hold an object")
            .clone()
    }

    /// Returns the identifier of a preset.
    fn preset_id() -> GameSettingsPresetId {
        GameSettingsPresetId::parse("game-settings-preset:123e4567-e89b-12d3-a456-426614174000")
            .expect("the identifier should be read")
    }

    #[test]
    fn the_settings_of_a_profile_become_a_preset() {
        let source = source(
            r#"{
                "javaVersionType": 2,
                "java": "17",
                "javaDir": "/java/17/bin/java",
                "javaArgs": "-Xmx4G",
                "noJVMArgs": true,
                "autoMemory": false,
                "maxMemory": 4096,
                "fullscreen": true,
                "width": 1280,
                "height": 720,
                "processPriority": 4,
                "launcherVisibility": 0,
                "minecraftArgs": "--demo",
                "wrapper": "mangohud",
                "precalledCommand": "echo hi",
                "postExitCommand": "echo bye",
                "serverIp": "mc.example.com",
                "showLogs": true,
                "useNativeGLFW": true
            }"#,
        );

        let preset = legacy_preset(preset_id(), 3, Some(&source));
        assert_eq!(preset.id, preset_id());
        assert_eq!(preset.auto_name_number, Some(3));

        let settings = &preset.settings;
        assert_eq!(settings.java_type, Some(JavaVersionType::Version));
        assert_eq!(settings.custom_java_version.as_deref(), Some("17"));
        assert_eq!(
            settings.custom_java_path.as_deref(),
            Some("/java/17/bin/java")
        );
        assert_eq!(settings.jvm_options.as_deref(), Some("-Xmx4G"));
        assert_eq!(settings.no_jvm_options, Some(true));
        assert_eq!(settings.auto_memory, Some(false));
        assert_eq!(settings.max_memory, Some(4096));
        assert_eq!(settings.window_type, Some(GameWindowType::Fullscreen));
        assert_eq!(settings.width, Some(1280.0));
        assert_eq!(settings.height, Some(720.0));
        assert_eq!(settings.process_priority, Some(ProcessPriority::High));
        assert_eq!(
            settings.launcher_visibility,
            Some(LauncherVisibility::Close)
        );
        assert_eq!(settings.game_arguments.as_deref(), Some("--demo"));
        assert_eq!(settings.command_wrapper.as_deref(), Some("mangohud"));
        assert_eq!(settings.pre_launch_command.as_deref(), Some("echo hi"));
        assert_eq!(settings.post_exit_command.as_deref(), Some("echo bye"));
        assert_eq!(settings.quick_play, Some(QuickPlayType::Multiplayer));
        assert_eq!(
            settings.quick_play_multiplayer.as_deref(),
            Some("mc.example.com")
        );
        assert_eq!(settings.show_logs, Some(true));
        assert_eq!(settings.use_native_glfw_or_sdl, Some(true));
    }

    #[test]
    fn a_profile_without_settings_keeps_the_defaults() {
        let preset = legacy_preset(preset_id(), 1, None);
        assert_eq!(preset.auto_name_number, Some(1));
        assert_eq!(preset.default_isolation_type, None);
        assert_eq!(preset.settings, GameSettingsValues::default());
    }

    #[test]
    fn a_profile_that_keeps_the_files_apart_asks_for_isolation() {
        let preset = legacy_preset(preset_id(), 1, Some(&source(r#"{"gameDirType": 1}"#)));
        assert_eq!(
            preset.default_isolation_type,
            Some(DefaultIsolationType::Always)
        );

        let preset = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"gameDirType": "VERSION_FOLDER"}"#)),
        );
        assert_eq!(
            preset.default_isolation_type,
            Some(DefaultIsolationType::Always)
        );

        let preset = legacy_preset(preset_id(), 1, Some(&source(r#"{"gameDirType": 0}"#)));
        assert_eq!(preset.default_isolation_type, None);
    }

    #[test]
    fn the_directory_the_game_runs_in_is_read() {
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"gameDirType": 2, "gameDir": "/games/shared"}"#)),
        )
        .settings;
        assert_eq!(settings.running_directory.as_deref(), Some("/games/shared"));

        // The root folder and the folder of the version are chosen by the
        // launcher, so the setting names no directory.
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"gameDirType": 0, "gameDir": "/games/shared"}"#)),
        )
        .settings;
        assert_eq!(settings.running_directory.as_deref(), Some(""));
    }

    #[test]
    fn the_way_the_runtime_is_chosen_is_read() {
        // The older launcher numbered the choice: the first two numbers are
        // the automatic choice.
        for (number, expected) in [
            (0, JavaVersionType::Auto),
            (1, JavaVersionType::Auto),
            (2, JavaVersionType::Version),
            (3, JavaVersionType::Detected),
            (4, JavaVersionType::Custom),
            (9, JavaVersionType::Auto),
        ] {
            let settings = legacy_preset(
                preset_id(),
                1,
                Some(&source(&format!(r#"{{"javaVersionType": {number}}}"#))),
            )
            .settings;
            assert_eq!(settings.java_type, Some(expected), "number {number}");
        }

        // The choice may be written by its name.
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"javaVersionType": "DETECTED"}"#)),
        )
        .settings;
        assert_eq!(settings.java_type, Some(JavaVersionType::Detected));

        // The launcher before the choice existed named it with the version.
        let settings =
            legacy_preset(preset_id(), 1, Some(&source(r#"{"java": "Custom"}"#))).settings;
        assert_eq!(settings.java_type, Some(JavaVersionType::Custom));

        let settings = legacy_preset(preset_id(), 1, Some(&source(r#"{"java": "17"}"#))).settings;
        assert_eq!(settings.java_type, Some(JavaVersionType::Auto));
        assert_eq!(settings.custom_java_version, None);
    }

    #[test]
    fn the_version_of_the_runtime_is_kept_for_the_choice_that_needs_it() {
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"javaVersionType": 2, "java": "17"}"#)),
        )
        .settings;
        assert_eq!(settings.custom_java_version.as_deref(), Some("17"));

        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(
                r#"{"javaVersionType": 3, "java": "17", "defaultJavaPath": "/java/17/bin/java"}"#,
            )),
        )
        .settings;
        let detected = settings.detected_java.expect("the runtime should be read");
        assert_eq!(detected.version, "17");
        assert!(!detected.path_hash.is_empty());

        // The other choices name no version of a runtime.
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"javaVersionType": 0, "java": "17"}"#)),
        )
        .settings;
        assert_eq!(settings.custom_java_version, None);
        assert_eq!(settings.detected_java, None);
    }

    #[test]
    fn the_renderer_is_read() {
        // The driver may be written by its name.
        let settings =
            legacy_preset(preset_id(), 1, Some(&source(r#"{"renderer": "LLVMPIPE"}"#))).settings;
        assert_eq!(settings.open_gl_renderer, Some(Renderer::Llvmpipe));
        // The driver draws with OpenGL, so the interface is chosen for it and
        // the driver of the other interface is left to the launcher.
        assert_eq!(settings.vulkan_renderer, None);
        assert_eq!(settings.graphics_backend, Some(GraphicsApi::OpenGl));

        // The older launcher wrote the driver of a newer version as an object.
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"renderer": {"name": "LAVAPIPE"}}"#)),
        )
        .settings;
        assert_eq!(settings.vulkan_renderer, Some(Renderer::Lavapipe));
        assert_eq!(settings.graphics_backend, Some(GraphicsApi::Vulkan));

        // The software renderer was a flag before the driver had a name.
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"useSoftwareRenderer": true}"#)),
        )
        .settings;
        assert_eq!(settings.open_gl_renderer, Some(Renderer::Llvmpipe));

        // A driver of the interface the player chose is written to that
        // interface only.
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(
                r#"{"renderer": "LAVAPIPE", "graphicsBackend": "OPENGL"}"#,
            )),
        )
        .settings;
        assert_eq!(settings.graphics_backend, Some(GraphicsApi::OpenGl));
        assert_eq!(settings.open_gl_renderer, Some(Renderer::Lavapipe));
        assert_eq!(settings.vulkan_renderer, None);
    }

    #[test]
    fn the_native_libraries_of_the_player_are_read() {
        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(
                r#"{"nativesDirType": 1, "nativesDir": "/natives"}"#,
            )),
        )
        .settings;
        assert_eq!(settings.use_custom_natives, Some(true));
        assert_eq!(settings.natives_directory.as_deref(), Some("/natives"));

        let settings = legacy_preset(
            preset_id(),
            1,
            Some(&source(r#"{"nativesDirType": "VERSION_FOLDER"}"#)),
        )
        .settings;
        assert_eq!(settings.use_custom_natives, Some(false));
        assert_eq!(settings.natives_directory.as_deref(), Some(""));
    }

    #[test]
    fn the_amount_of_memory_is_read() {
        // A file that names no amount of memory gets the amount the launcher
        // suggests.
        let settings = legacy_preset(preset_id(), 1, Some(&source("{}"))).settings;
        assert_eq!(settings.max_memory, Some(suggested_memory()));

        // An amount of zero is the amount the launcher suggests.
        let settings = legacy_preset(preset_id(), 1, Some(&source(r#"{"maxMemory": 0}"#))).settings;
        assert_eq!(settings.max_memory, Some(suggested_memory()));

        let settings =
            legacy_preset(preset_id(), 1, Some(&source(r#"{"minMemory": 512}"#))).settings;
        assert_eq!(settings.min_memory, Some(512));

        // A file that names no smallest amount of memory lets the game choose.
        let settings = legacy_preset(preset_id(), 1, Some(&source("{}"))).settings;
        assert_eq!(settings.min_memory, None);
    }

    #[test]
    fn the_instance_that_inherits_its_profile_keeps_inheriting_it() {
        let parent = preset_id();
        let mut preset = Preset::new(parent);
        preset.default_isolation_type = Some(DefaultIsolationType::Always);
        preset.settings.running_directory = Some(String::new());

        let instance = legacy_instance_settings(
            Some(parent),
            Some(&preset),
            Some(&source(r#"{"usesGlobal": true, "versionIcon": 7}"#)),
            true,
        );

        assert_eq!(instance.parent, Some(parent));
        assert_eq!(instance.icon, Some(GameInstanceIconType::Fabric));
        // The preset keeps the files of every instance apart and names no
        // directory, so the instance runs in a directory of its own.
        assert!(instance.is_overridden(RUNNING_DIRECTORY));
        // The settings of the instance are read from the preset.
        assert_eq!(instance.settings, GameSettingsValues::default());
    }

    #[test]
    fn the_instance_that_kept_its_own_settings_overrides_them() {
        let parent = preset_id();
        let instance = legacy_instance_settings(
            Some(parent),
            None,
            Some(&source(
                r#"{"usesGlobal": false, "maxMemory": 2048, "versionIcon": "FORGE"}"#,
            )),
            false,
        );

        assert_eq!(instance.icon, Some(GameInstanceIconType::Forge));
        assert_eq!(instance.settings.max_memory, Some(2048));
        for name in OVERRIDDEN_SETTINGS {
            assert!(instance.is_overridden(name), "{name} should be overridden");
        }
    }

    #[test]
    fn an_instance_without_a_file_of_the_older_launcher_is_not_migrated() {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-core-legacy-instance-empty-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        assert!(
            migrate_legacy_instance_settings(&directory, None, None, None)
                .expect("the migration should not fail")
                .is_none()
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_settings_of_an_instance_are_migrated_once() {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-core-legacy-instance-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        let text = r#"{"usesGlobal": true, "versionIcon": 4}"#;
        std::fs::write(legacy_instance_settings_path(&directory), text)
            .expect("the file should be written");

        let parent = preset_id();
        let migration = migrate_legacy_instance_settings(
            &directory,
            Some(parent),
            Some(&Preset::new(parent)),
            None,
        )
        .expect("the migration should run")
        .expect("the file should be migrated");

        assert_eq!(migration.settings.parent, Some(parent));
        assert_eq!(migration.settings.icon, Some(GameInstanceIconType::Command));
        assert_eq!(
            migration.receipt_path,
            legacy_instance_settings_receipt_path(&directory)
        );

        // The receipt keeps the launcher from reading the same file again.
        let receipt = legacy_instance_settings_receipt(text, "2026-10-24T12:00:00Z");
        std::fs::create_dir_all(
            migration
                .receipt_path
                .parent()
                .expect("the receipt has a parent"),
        )
        .expect("the directory should be created");
        std::fs::write(&migration.receipt_path, &receipt).expect("the receipt should be written");
        assert!(migration_receipt_matches(&receipt, text));
        assert!(migrate_legacy_instance_settings(
            &directory,
            Some(parent),
            Some(&Preset::new(parent)),
            None,
        )
        .expect("the migration should not fail")
        .is_none());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_instances_of_a_game_directory_are_read() {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-core-legacy-instances-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        // An instance is a directory under `versions` that holds the file of
        // the version of the game.
        let versions = directory.join("versions");
        let instance = versions.join("1.20.1");
        std::fs::create_dir_all(&instance).expect("the instance should be created");
        std::fs::write(instance.join("1.20.1.json"), "{}").expect("the file should be written");
        std::fs::write(
            legacy_instance_settings_path(&instance),
            r#"{"usesGlobal": true}"#,
        )
        .expect("the settings should be written");

        // A directory that holds no file of a version is not an instance.
        let other = versions.join("not-an-instance");
        std::fs::create_dir_all(&other).expect("the directory should be created");
        std::fs::write(
            legacy_instance_settings_path(&other),
            r#"{"usesGlobal": true}"#,
        )
        .expect("the settings should be written");

        let parent = preset_id();
        let mut presets = GameSettingsPresets::new();
        presets.presets.push(Preset::new(parent));

        let migrations = migrate_legacy_instances(&directory, &presets, Some(parent))
            .expect("the migration runs");
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0].path, legacy_instance_settings_path(&instance));
        assert_eq!(migrations[0].settings.parent, Some(parent));

        // A directory without versions has nothing to migrate.
        let empty = directory.join("empty");
        std::fs::create_dir_all(&empty).expect("the directory should be created");
        assert!(migrate_legacy_instances(&empty, &presets, Some(parent))
            .expect("the migration runs")
            .is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_settings_of_the_launcher_of_today_are_not_replaced() {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-core-legacy-instance-keep-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        std::fs::write(
            legacy_instance_settings_path(&directory),
            r#"{"usesGlobal": false}"#,
        )
        .expect("the file should be written");

        // The instance already has settings of its own, so the file of the
        // older launcher is left alone.
        let mut settings = InstanceSettings::new();
        settings.settings.max_memory = Some(1024);
        InstanceSettings::save(&directory, &settings).expect("the settings should be written");

        assert!(
            migrate_legacy_instance_settings(&directory, None, None, None)
                .expect("the migration should not fail")
                .is_none()
        );

        let read = InstanceSettings::load(&directory).expect("the settings should be read");
        assert_eq!(read.settings().settings.max_memory, Some(1024));

        let _ = std::fs::remove_dir_all(&directory);
    }
}
