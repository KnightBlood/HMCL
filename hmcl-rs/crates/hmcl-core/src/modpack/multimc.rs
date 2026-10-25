//! The modpacks MultiMC writes.
//!
//! A modpack of MultiMC is a directory that holds the settings of the instance
//! in `instance.cfg`, the components of the instance in `mmc-pack.json` and a
//! patch for every component in `patches/<uid>.json`. A patch is the version
//! file the launcher of MultiMC reads: it names the classes of the component,
//! the arguments it needs and the libraries it brings.
//!
//! The launcher of today does not read the patches itself when the game is
//! started. The patches are therefore turned into one manifest the launcher
//! reads: the patches are applied in the order the components are listed, and
//! the result is the manifest of the instance.
//!
//! MultiMC starts a component by calling the class the patch names; the loader
//! of HMCL can not, because the component may install a class loader of its
//! own. The manifest therefore names the bootstrap of HMCL as its main class
//! and tells the bootstrap the class of the game in a property, so the
//! bootstrap starts the class the patches name.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::download::game::{
    Argument, ArgumentDetails, ArgumentValue, Arguments, AssetIndex, CompatibilityRule,
    DownloadInfo, InstanceManifest, JavaVersion, Library, OsRule, RuleAction,
};
use crate::game::launch::{default_jvm_arguments, tokenize};
use crate::game::GameComponentType;
use crate::setting::game_settings::{GameSettingsValues, GameWindowType, JavaVersionType};

/// The root the metadata of the components of MultiMC is published at.
pub const META_ROOT: &str = "https://meta.multimc.org/v1";

/// The group of the bootstrap of HMCL.
pub const BOOTSTRAP_GROUP: &str = "org.jackhuang.hmcl";

/// The name of the bootstrap of HMCL.
pub const BOOTSTRAP_NAME: &str = "mmc-bootstrap";

/// The version of the bootstrap of HMCL.
pub const BOOTSTRAP_VERSION: &str = "1.0";

/// The class the instances of MultiMC are started with.
pub const BOOTSTRAP_MAIN: &str = "org.jackhuang.hmcl.HMCLMultiMCBootstrap";

/// The class the patches of MultiMC name when the patch does not name one.
pub const DEFAULT_MAIN_CLASS: &str = "net.minecraft.client.Minecraft";

/// The identity of the components of a modpack of MultiMC.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct MultiMcComponent {
    /// The name of the component the launcher of MultiMC shows.
    #[serde(default, rename = "cachedName")]
    pub cached_name: Option<String>,
    /// The components the component needs.
    #[serde(default, rename = "cachedRequires")]
    pub cached_requires: Vec<MultiMcRequirement>,
    /// The version of the component the launcher of MultiMC cached.
    #[serde(default, rename = "cachedVersion")]
    pub cached_version: Option<String>,
    /// Whether the component is important to the player.
    #[serde(default)]
    pub important: bool,
    /// Whether the component is only installed because another component needs
    /// it.
    #[serde(default, rename = "dependencyOnly")]
    pub dependency_only: bool,
    /// The identity of the component, such as `net.minecraftforge`.
    #[serde(default, rename = "uid")]
    pub uid: String,
    /// The version of the component.
    #[serde(default)]
    pub version: Option<String>,
}

/// A component another component needs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct MultiMcRequirement {
    /// The version the component must have, if the component names one.
    #[serde(default, rename = "equals")]
    pub equals: Option<String>,
    /// The identity of the component.
    #[serde(default, rename = "uid")]
    pub uid: String,
    /// The version the component suggests when the exact version is not known.
    #[serde(default)]
    pub suggests: Option<String>,
}

/// The components of a modpack of MultiMC, as `mmc-pack.json` names them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct MultiMcManifest {
    /// The version of the format of the components.
    #[serde(default, rename = "formatVersion")]
    pub format_version: i32,
    /// The components of the instance.
    #[serde(default)]
    pub components: Vec<MultiMcComponent>,
}

impl MultiMcManifest {
    /// Reads `mmc-pack.json`.
    ///
    /// Returns [`MultiMcError::Patch`] when the file is not valid JSON; a file
    /// that names no component is refused by [`MultiMcManifest::game_version`]
    /// and by [`resolve_patches`].
    pub fn read(text: &str) -> Result<Self, MultiMcError> {
        serde_json::from_str(text).map_err(|error| MultiMcError::Patch {
            uid: "mmc-pack.json".to_string(),
            message: error.to_string(),
        })
    }

    /// Returns the version of the game the components name.
    ///
    /// The game is the component whose identity the launcher reads as the game
    /// itself; the modpack must name one, because the other components are
    /// patches of that version.
    pub fn game_version(&self) -> Option<&str> {
        self.components
            .iter()
            .find(|component| component_of_uid(&component.uid) == Some(GameComponentType::Game))
            .and_then(|component| component.version.as_deref())
    }

    /// Returns the version of the game the components name.
    ///
    /// The patch of every component is published for a version of the game; a
    /// modpack that names no version cannot be installed, so the launcher
    /// refuses it instead of reading its patches against no version.
    pub fn require_game_version(&self) -> Result<&str, MultiMcError> {
        self.game_version().ok_or(MultiMcError::NoGame)
    }
}

/// A patch of a component, as `patches/<uid>.json` writes it.
///
/// The fields the patches of MultiMC add to the version files of Mojang begin
/// with `+`; the launcher reads both spellings of the libraries, because the
/// patches of the older launchers write the list without the `+`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct MultiMcInstancePatch {
    /// The version of the format of the patch, which must be `1`.
    #[serde(default, rename = "formatVersion")]
    pub format_version: i32,
    /// The identity of the component the patch belongs to.
    #[serde(default, rename = "uid")]
    pub uid: String,
    /// The version of the component.
    #[serde(default)]
    pub version: Option<String>,
    /// The index of the assets of the component.
    #[serde(default, rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    /// The arguments of the game in the old format, in one text.
    #[serde(default, rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    /// The arguments of the virtual machine the component adds.
    #[serde(default, rename = "+jvmArgs")]
    pub jvm_args: Vec<String>,
    /// The class the component starts.
    #[serde(default, rename = "mainClass")]
    pub main_class: Option<String>,
    /// The versions of Java the component runs on.
    #[serde(default, rename = "compatibleJavaMajors")]
    pub java_majors: Option<Vec<i32>>,
    /// The file of the game the component replaces.
    #[serde(default, rename = "mainJar")]
    pub main_jar: Option<Library>,
    /// The traits of the component, which tell how the launcher starts it.
    #[serde(default, rename = "+traits")]
    pub traits: Vec<String>,
    /// The tweakers the component adds to the class the game is started with.
    #[serde(default, rename = "+tweakers")]
    pub tweakers: Vec<String>,
    /// The libraries the component adds, as the newer patches write them.
    #[serde(default, rename = "+libraries")]
    pub libraries: Vec<Library>,
    /// The libraries the component adds, as the older patches write them.
    #[serde(default, rename = "libraries")]
    pub libraries_legacy: Vec<Library>,
    /// The libraries of the component that are only needed while the game is
    /// installed.
    #[serde(default, rename = "mavenFiles")]
    pub maven_files: Vec<Library>,
    /// The archives the component adds to the file of the game.
    #[serde(default, rename = "jarMods")]
    pub jar_mods: Vec<Library>,
    /// The components the patch needs.
    #[serde(default)]
    pub requires: Vec<MultiMcRequirement>,
}

impl MultiMcInstancePatch {
    /// Reads a patch of a component.
    pub fn read(uid: &str, text: &str) -> Result<Self, MultiMcError> {
        serde_json::from_str(text).map_err(|error| MultiMcError::Patch {
            uid: uid.to_string(),
            message: error.to_string(),
        })
    }

    /// Returns the libraries the patch adds.
    ///
    /// The newer patches write the list as `+libraries` and the older ones as
    /// `libraries`; the launcher reads both and keeps the `+` list first.
    pub fn libraries(&self) -> Vec<Library> {
        let mut libraries = self.libraries.clone();
        libraries.extend(self.libraries_legacy.iter().cloned());
        libraries
    }
}

/// The settings of an instance of MultiMC, as `instance.cfg` writes them.
///
/// The file is a properties file: every line holds a key and a value, and the
/// lines that begin with `#` or `!` are comments. The launcher of MultiMC
/// writes the values the player set, so a key that is not there has the
/// default value of the launcher.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultiMcConfig {
    /// The keys and the values of the file.
    pub values: BTreeMap<String, String>,
}

impl MultiMcConfig {
    /// Reads the settings of an instance.
    ///
    /// The settings are read the way the launcher of Java reads a properties
    /// file, with the workaround the launcher of today makes for the values
    /// that begin with a quote and end with a colon: some versions of the
    /// launcher of MultiMC wrote a quote in front of the value and a colon
    /// after it, and the colon is not part of the value.
    pub fn parse(text: &str) -> Self {
        let mut values = BTreeMap::new();

        let mut line = String::new();
        let mut characters = text.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '\\' => {
                    // A backslash at the end of a line continues the value on
                    // the next line.
                    match characters.next() {
                        Some('\n') => {}
                        Some('\r') => {
                            if characters.peek() == Some(&'\n') {
                                characters.next();
                            }
                        }
                        Some(escaped) => {
                            line.push('\\');
                            line.push(escaped);
                        }
                        None => line.push('\\'),
                    }
                }
                '\n' | '\r' => {
                    if character == '\r' && characters.peek() == Some(&'\n') {
                        characters.next();
                    }
                    read_line(&mut line, &mut values);
                }
                character => line.push(character),
            }
        }
        read_line(&mut line, &mut values);

        Self { values }
    }

    /// Returns the value of a key.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Returns the value of a key as a number, if it holds one.
    pub fn number(&self, key: &str) -> Option<i32> {
        self.value(key).and_then(|value| value.trim().parse().ok())
    }

    /// Returns the value of a key as a flag.
    ///
    /// A key that is not there is off, which is the default of the launcher of
    /// MultiMC.
    pub fn flag(&self, key: &str) -> bool {
        self.value(key)
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
    }

    /// Returns whether the player let the instance set the amount of the memory
    /// it uses.
    pub fn overrides_memory(&self) -> bool {
        self.flag("OverrideMemory")
    }

    /// Returns whether the player let the instance set the version of Java it
    /// runs on.
    pub fn overrides_java_location(&self) -> bool {
        self.flag("OverrideJavaLocation")
    }

    /// Returns whether the player let the instance set the arguments of the
    /// virtual machine.
    pub fn overrides_java_args(&self) -> bool {
        self.flag("OverrideJavaArgs")
    }

    /// Returns whether the player let the instance set the commands it runs
    /// before the game and after it.
    pub fn overrides_commands(&self) -> bool {
        self.flag("OverrideCommands")
    }

    /// Returns whether the player let the instance set the size of the window
    /// of the game.
    pub fn overrides_window(&self) -> bool {
        self.flag("OverrideWindow")
    }

    /// Returns whether the player let the instance set the window of the
    /// console of the game.
    pub fn overrides_console(&self) -> bool {
        self.flag("OverrideConsole")
    }

    /// Returns whether the game is started in a full window.
    pub fn fullscreen(&self) -> bool {
        self.flag("LaunchMaximized")
    }

    /// Returns the version of the game the instance was created for.
    ///
    /// The components name the version of the game; the settings only keep the
    /// version of the instances that had no components, which are the
    /// instances of the first format of the launcher of MultiMC.
    pub fn game_version<'a>(&'a self, manifest: Option<&'a MultiMcManifest>) -> Option<&'a str> {
        match manifest {
            Some(manifest) => manifest.game_version(),
            None => self.value("IntendedVersion"),
        }
    }

    /// Returns the name of the icon of the instance in the modpack.
    pub fn icon_key(&self) -> Option<&str> {
        self.value("iconKey")
    }
}

/// Reads a line of a properties file into the settings.
fn read_line(line: &mut String, values: &mut BTreeMap<String, String>) {
    let text = std::mem::take(line);
    let trimmed = text.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
        return;
    }

    let (key, value) = match split_line(trimmed) {
        Some(pair) => pair,
        None => return,
    };

    let key = unescape(key.trim_end());
    if key.is_empty() {
        return;
    }

    // The colon the launcher of MultiMC wrote after a value that begins with a
    // quote is not part of the value.
    let value = unescape(value.trim_start());
    let value = if value.len() >= 2 && value.starts_with('"') && value.ends_with(':') {
        value[..value.len() - 1].to_string()
    } else {
        value
    };

    values.insert(key, value);
}

/// Splits a line of a properties file into its key and its value.
fn split_line(text: &str) -> Option<(&str, &str)> {
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '=' | ':' => return Some((&text[..index], &text[index + 1..])),
            character if character.is_whitespace() => {
                // The white space separates the key from the value when the
                // value is not written after `=` or `:`.
                let rest = &text[index..];
                let rest = rest.trim_start();
                let rest = rest
                    .strip_prefix('=')
                    .or_else(|| rest.strip_prefix(':'))
                    .unwrap_or(rest);
                return Some((&text[..index], rest));
            }
            _ => {}
        }
    }

    Some((text, ""))
}

/// Reads the escapes of a properties file.
fn unescape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }

        match characters.next() {
            Some('t') => result.push('\t'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('f') => result.push('\u{0c}'),
            Some('u') => {
                let digits: String = characters.by_ref().take(4).collect();
                match u32::from_str_radix(&digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                {
                    Some(decoded) => result.push(decoded),
                    // A broken escape is kept as it is, so the value of the
                    // player is not silently changed.
                    None => {
                        result.push_str("\\u");
                        result.push_str(&digits);
                    }
                }
            }
            Some(escaped) => result.push(escaped),
            None => result.push('\\'),
        }
    }
    result
}

/// Returns the component a patch of MultiMC belongs to.
///
/// The identities are the names the launcher of MultiMC publishes the patches
/// under; a component the launcher has no name for is read as no component of
/// the game, so its patch is kept as it is.
pub fn component_of_uid(uid: &str) -> Option<GameComponentType> {
    match uid {
        "net.minecraft" => Some(GameComponentType::Game),
        "net.minecraftforge" => Some(GameComponentType::Forge),
        "net.neoforged" => Some(GameComponentType::NeoForge),
        "com.mumfrey.liteloader" => Some(GameComponentType::LiteLoader),
        "net.fabricmc.fabric-loader" => Some(GameComponentType::Fabric),
        "org.quiltmc.quilt-loader" => Some(GameComponentType::Quilt),
        _ => None,
    }
}

/// Returns the address the patch of a component is published at.
///
/// The components that do not name a version are read as the version the
/// launcher of MultiMC reads them as: the versions of LWJGL keep the versions
/// the game was built with, and the components the loaders generate follow the
/// version of the game.
///
/// Returns `None` when the version of the component is not known and the
/// component is not one the launcher can fill in.
pub fn meta_url(component: &str, version: Option<&str>, mc_version: &str) -> Option<String> {
    let version = match version {
        Some(version) => version,
        None => match component {
            "org.lwjgl" => "2.9.1",
            "org.lwjgl3" => "3.1.2",
            "net.fabricmc.intermediary" | "org.quiltmc.hashed" => mc_version,
            _ => return None,
        },
    };

    Some(format!("{META_ROOT}/{component}/{version}.json"))
}

/// Returns the directory of an archive that holds the instance of a modpack of
/// MultiMC.
///
/// A modpack of the launcher of MultiMC writes the files of the instance in the
/// root of the archive, and a modpack exported by the launcher of today writes
/// them in one directory. The directory is the one that holds `instance.cfg`
/// directly, and it is returned with the slash at its end; the root of the
/// archive is returned as the empty text.
pub fn root_directory<S: AsRef<str>>(
    names: impl IntoIterator<Item = S>,
) -> Result<String, MultiMcError> {
    let mut found: Option<String> = None;

    for name in names {
        let name = name.as_ref();
        if name == "instance.cfg" {
            return Ok(String::new());
        }
        if found.is_some() {
            continue;
        }

        // The file must be directly inside the first directory of the archive.
        if let Some((directory, file)) = name.split_once('/') {
            if file == "instance.cfg" {
                found = Some(format!("{directory}/"));
            }
        }
    }

    found.ok_or(MultiMcError::NotAModpack)
}

/// The instance a modpack of MultiMC is installed as.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedInstance {
    /// The manifest the launcher reads the instance from.
    pub manifest: InstanceManifest,
    /// The version of the game the components name.
    pub game_version: Option<String>,
    /// The file of the game the components replace.
    pub main_jar: Option<Library>,
    /// The names of the archives the components add to the file of the game.
    pub jar_mods: Vec<String>,
    /// The libraries that are only needed while the instance is installed.
    pub maven_files: Vec<Library>,
    /// What the launcher could not read of the components.
    ///
    /// The features of MultiMC the launcher has no idea about are reported
    /// instead of being silently dropped, so the player knows the game may
    /// behave differently than under the launcher of MultiMC.
    pub warnings: Vec<String>,
}

/// Returns the bootstrap the manifest of an instance of MultiMC needs.
///
/// The file of the bootstrap is not downloaded: the launcher keeps it, and
/// writes it where the manifest tells the library to be.
pub fn bootstrap_library() -> Library {
    Library {
        name: format!("{BOOTSTRAP_GROUP}:{BOOTSTRAP_NAME}:{BOOTSTRAP_VERSION}"),
        ..Library::default()
    }
}

/// Returns the address the bootstrap reads the class of the game from.
///
/// The address is written as a query: the bootstrap reads the class of the
/// game and the text the launcher writes about the installation it made, so a
/// bug the player reports names the launcher that installed the game.
pub fn bootstrap_profile_url(main_class: &str, installer: &str) -> String {
    format!(
        "hmcl:///bootstrap_profile_v1/?main_class={}&installer={}",
        form_encode(main_class),
        form_encode(installer)
    )
}

/// Returns the text the launcher writes about the way it reads the patches of
/// MultiMC.
pub fn installer_profile() -> String {
    format!(
        "Patches: recursive install, fabric & quilt intermediary\nImplementation: {}",
        installer_name()
    )
}

/// Returns the name the launcher writes as the implementation of the patches.
fn installer_name() -> String {
    format!(
        "{BOOTSTRAP_GROUP} (Rust port, {})",
        env!("CARGO_PKG_VERSION")
    )
}

/// Encodes the text the way a query of an address is written.
///
/// The spaces are written as `+` and the characters that are not letters,
/// numbers or `.-*_` are written with their bytes in hexadecimal, which is the
/// way the launcher of Java writes a query.
fn form_encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'*' | b'_' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            byte => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// Turns the patches of the components of a modpack into the manifest of an
/// instance.
///
/// The patches are applied the way the launcher of MultiMC applies them: the
/// patch of the last component names every part of the version, and the
/// patches of the components before it fill in the parts the later patches do
/// not name. The libraries of every patch are kept, the ones of the component
/// that comes first in the modpack first.
pub fn resolve_patches(
    patches: &[MultiMcInstancePatch],
    instance_id: &str,
) -> Result<ResolvedInstance, MultiMcError> {
    if patches.is_empty() {
        return Err(MultiMcError::NoComponents);
    }

    for patch in patches {
        if patch.format_version != 1 {
            return Err(MultiMcError::UnsupportedFormat {
                uid: patch.uid.clone(),
                version: patch.format_version,
            });
        }
    }

    let mut warnings = Vec::new();

    let last = &patches[patches.len() - 1];
    let mut minecraft_arguments: Option<Vec<String>> =
        last.minecraft_arguments.as_deref().map(tokenize);
    // Only the components before the last one contribute the arguments of the
    // virtual machine, which is the way the launcher of MultiMC reads a pack.
    let mut jvm_arguments: Vec<Argument> = default_jvm_arguments();
    let mut main_class = last.main_class.clone();
    let mut asset_index = last.asset_index.clone();
    let mut java_majors = last.java_majors.clone();
    let mut main_jar = last.main_jar.clone();
    let mut traits = last.traits.clone();
    let mut tweakers = last.tweakers.clone();
    let mut libraries = last.libraries();
    let mut maven_files = last.maven_files.clone();
    let mut jar_mods: Vec<String> = last
        .jar_mods
        .iter()
        .filter_map(|library| library.filename.clone())
        .collect();

    for patch in patches[..patches.len() - 1].iter().rev() {
        if minecraft_arguments.is_none() {
            minecraft_arguments = patch.minecraft_arguments.as_deref().map(tokenize);
        }

        // The arguments of the components are appended in the reverse order of
        // the components, which is the way the launcher of MultiMC reads them.
        for argument in &patch.jvm_args {
            jvm_arguments.push(Argument::Text(argument.clone()));
        }

        if main_class.is_none() {
            main_class = patch.main_class.clone();
        }
        if asset_index.is_none() {
            asset_index = patch.asset_index.clone();
        }
        if java_majors.is_none() {
            java_majors = patch.java_majors.clone();
        }
        if main_jar.is_none() {
            main_jar = patch.main_jar.clone();
        }

        traits = merge(&patch.traits, &traits);
        tweakers = merge(&patch.tweakers, &tweakers);
        libraries = merge(&patch.libraries(), &libraries);
        maven_files = merge(&patch.maven_files, &maven_files);
        jar_mods = merge(
            &patch
                .jar_mods
                .iter()
                .filter_map(|library| library.filename.clone())
                .collect::<Vec<String>>(),
            &jar_mods,
        );
    }

    let mut main_class = main_class.unwrap_or_else(|| DEFAULT_MAIN_CLASS.to_string());
    let mut game_arguments = minecraft_arguments.unwrap_or_default();

    // The launcher of MultiMC writes the tweakers of the components into the
    // arguments of the game; a tweaker is a class the launcher of the game
    // reads before the game itself, and it must not be the last argument.
    if game_arguments.len() > 1 {
        let mut index = (game_arguments.len() - 2) as isize;
        while index >= 0 {
            let position = index as usize;
            if game_arguments[position] == "--tweakClass" && position + 1 < game_arguments.len() {
                tweakers.push(game_arguments[position + 1].clone());
                game_arguments.remove(position);
                game_arguments.remove(position);
            }
            index -= 1;
        }
    }

    dedupe(&mut traits);
    dedupe(&mut tweakers);
    dedupe(&mut jar_mods);

    for tweaker in &tweakers {
        game_arguments.push("--tweakClass".to_string());
        game_arguments.push(tweaker.clone());
    }

    for trait_name in &traits {
        match trait_name.as_str() {
            "FirstThreadOnMacOS" => jvm_arguments.push(Argument::Detailed(ArgumentDetails {
                rules: Some(vec![CompatibilityRule {
                    action: RuleAction::Allow,
                    os: Some(OsRule {
                        name: Some("osx".to_string()),
                        version: None,
                        arch: None,
                    }),
                    features: None,
                }]),
                value: Some(ArgumentValue::Text("-XstartOnFirstThread".to_string())),
            })),
            // The chat report the newer versions of the game write and the
            // texture packs the older versions read need no argument of the
            // launcher.
            "XR:Initial" | "texturepacks" | "no-texturepacks" => {}
            other => warnings.push(format!("trait: {other}")),
        }
    }

    // ForgeWrapper starts the installer of Forge itself; it reads where the
    // libraries of the instance are and which file the installer writes.
    if libraries.iter().any(is_forge_wrapper) {
        jvm_arguments.push(Argument::Text(
            "-Dforgewrapper.librariesDir=${library_directory}".to_string(),
        ));
        jvm_arguments.push(Argument::Text(
            "-Dforgewrapper.minecraft=${primary_jar}".to_string(),
        ));

        for library in &libraries {
            if let Some(path) = installer_path(library) {
                jvm_arguments.push(Argument::Text(format!(
                    "-Dforgewrapper.installer=${{library_directory}}/{path}"
                )));
            }
        }
    }

    // The bootstrap starts the class the patches name; the launcher of HMCL
    // can not start a component that installs a class loader of its own.
    libraries.insert(0, bootstrap_library());
    jvm_arguments.push(Argument::Text(format!(
        "-Dhmcl.mmc.bootstrap={}",
        bootstrap_profile_url(&main_class, &installer_profile())
    )));
    main_class = BOOTSTRAP_MAIN.to_string();

    let mut manifest = InstanceManifest {
        id: Some(instance_id.to_string()),
        main_class: Some(main_class),
        arguments: Some(Arguments {
            game: Some(
                game_arguments
                    .into_iter()
                    .map(Argument::Text)
                    .collect::<Vec<Argument>>(),
            ),
            jvm: Some(jvm_arguments),
        }),
        asset_index,
        libraries,
        root: Some(true),
        hidden: Some(false),
        ..InstanceManifest::default()
    };

    if let Some(main_jar) = &main_jar {
        let client = main_jar
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artifact.clone())
            .or_else(|| {
                main_jar.url.as_deref().map(|url| DownloadInfo {
                    url: url.to_string(),
                    ..DownloadInfo::default()
                })
            });
        if let Some(client) = client {
            let mut downloads = BTreeMap::new();
            downloads.insert("client".to_string(), client);
            manifest.downloads = Some(downloads);
        }
    }

    // The manifest of Mojang names one version of Java; the patches name every
    // version the component runs on. The newest version the launcher knows is
    // written, and the older versions are reported, because the game may need
    // one of them.
    if let Some(majors) = java_majors {
        let mut majors = majors;
        majors.sort_unstable();
        let selected = majors
            .iter()
            .rev()
            .find_map(|major| crate::game::GameJavaVersion::of(*major));
        match selected {
            Some(version) => {
                manifest.java_version = Some(JavaVersion {
                    component: Some(version.component.to_string()),
                    major_version: Some(version.major_version),
                });
            }
            None => warnings.push(format!("java version range: {majors:?}")),
        }
    }

    let game_version = patches
        .iter()
        .find(|patch| component_of_uid(&patch.uid) == Some(GameComponentType::Game))
        .and_then(|patch| patch.version.clone());

    Ok(ResolvedInstance {
        manifest,
        game_version,
        main_jar,
        jar_mods,
        maven_files,
        warnings,
    })
}

/// Joins two lists, the first list first.
fn merge<T: Clone>(first: &[T], second: &[T]) -> Vec<T> {
    let mut merged = first.to_vec();
    merged.extend(second.iter().cloned());
    merged
}

/// Removes the repeated values of a list, keeping the first ones.
fn dedupe<T: PartialEq + Clone>(values: &mut Vec<T>) {
    let mut kept: Vec<T> = Vec::with_capacity(values.len());
    for value in values.iter() {
        if !kept.contains(value) {
            kept.push(value.clone());
        }
    }
    *values = kept;
}

/// Returns whether the library is ForgeWrapper.
fn is_forge_wrapper(library: &Library) -> bool {
    library.artifact().is_ok_and(|artifact| {
        artifact.group == "io.github.zekerzhayard" && artifact.name == "ForgeWrapper"
    })
}

/// Returns the path of the installer of Forge or NeoForge in a library.
fn installer_path(library: &Library) -> Option<String> {
    let artifact = library.artifact().ok()?;
    if artifact.classifier.as_deref() != Some("installer") {
        return None;
    }

    match (artifact.group.as_str(), artifact.name.as_str()) {
        ("net.neoforged", "neoforge") | ("net.minecraftforge", "forge") => Some(artifact.path()),
        _ => None,
    }
}

/// Applies the settings of an instance of MultiMC to the settings of HMCL.
///
/// The settings of MultiMC are read from `instance.cfg`, which the player of
/// MultiMC filled in; the launcher of Java applies them to the instance it
/// installs when the player let the instance set them, which is what the
/// `Override*` keys say. A setting the player did not let the instance set is
/// left as it is, so the settings of HMCL the player made are kept.
///
/// The memory is written as `Option::is_some` so the launcher of HMCL knows the
/// instance asks for an amount of its own; the window is written the way the
/// launcher of HMCL names it: a full window is `Fullscreen`, a maximized one is
/// `Maximized` and a window of a size is `Windowed`.
pub fn apply_to_settings(config: &MultiMcConfig, settings: &mut GameSettingsValues) {
    if config.overrides_memory() {
        if let Some(max_memory) = config.number("MaxMemAlloc") {
            if max_memory > 0 {
                settings.max_memory = Some(max_memory as u32);
            }
        }
        if let Some(min_memory) = config.number("MinMemAlloc") {
            if min_memory > 0 {
                settings.min_memory = Some(min_memory as u32);
            }
        }
        if let Some(perm_gen) = config.number("PermGen") {
            if perm_gen > 0 {
                settings.perm_size = Some(perm_gen.to_string());
            }
        }
        settings.auto_memory = Some(false);
    }

    if config.overrides_java_location() {
        if let Some(java_path) = config
            .value("JavaPath")
            .filter(|path| !path.trim().is_empty())
        {
            settings.java_type = Some(JavaVersionType::Custom);
            settings.custom_java_path = Some(java_path.trim().to_string());
        }
    }

    if config.overrides_java_args() {
        if let Some(jvm_args) = config
            .value("JvmArgs")
            .filter(|args| !args.trim().is_empty())
        {
            settings.jvm_options = Some(jvm_args.trim().to_string());
            // The launcher of MultiMC replaces its own arguments with the ones
            // the player wrote, so the launcher of HMCL adds none of its own.
            settings.no_optimizing_jvm_options = Some(true);
        }
    }

    if config.overrides_commands() {
        if let Some(command) = config
            .value("WrapperCommand")
            .filter(|command| !command.trim().is_empty())
        {
            settings.command_wrapper = Some(command.trim().to_string());
        }
        if let Some(command) = config
            .value("PreLaunchCommand")
            .filter(|command| !command.trim().is_empty())
        {
            settings.pre_launch_command = Some(command.trim().to_string());
        }
        if let Some(command) = config
            .value("PostExitCommand")
            .filter(|command| !command.trim().is_empty())
        {
            settings.post_exit_command = Some(command.trim().to_string());
        }
    }

    if config.overrides_window() {
        let height = config
            .number("MinecraftWinHeight")
            .filter(|value| *value > 0);
        let width = config
            .number("MinecraftWinWidth")
            .filter(|value| *value > 0);

        if config.fullscreen() {
            settings.window_type = Some(GameWindowType::Fullscreen);
        } else if height.is_none() && width.is_none() {
            // The launcher of MultiMC starts a maximized window when the player
            // set no size.
            settings.window_type = Some(GameWindowType::Maximized);
        } else {
            settings.window_type = Some(GameWindowType::Windowed);
            if let Some(width) = width {
                settings.width = Some(width as f64);
            }
            if let Some(height) = height {
                settings.height = Some(height as f64);
            }
        }
    }

    if config.overrides_console() {
        // The launcher of HMCL shows the window of the console when the game
        // runs; the launcher of MultiMC shows it when the player asked for it
        // or when the game failed.
        let show = config.flag("ShowConsole") || config.flag("ShowConsoleOnError");
        settings.not_use_javaw = Some(show || !config.flag("AutoCloseConsole"));
        settings.show_logs = Some(show);
    }
}

/// A modpack of MultiMC the launcher cannot read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiMcError {
    /// The modpack names no component.
    NoComponents,
    /// The archive is not a modpack of MultiMC.
    NotAModpack,
    /// A patch uses a format the launcher does not know.
    UnsupportedFormat {
        /// The identity of the component the patch belongs to.
        uid: String,
        /// The version of the format.
        version: i32,
    },
    /// A patch is not a valid patch.
    Patch {
        /// The identity of the component the patch belongs to.
        uid: String,
        /// The reason the patch was refused.
        message: String,
    },
    /// The modpack names no version of the game.
    NoGame,
}

impl fmt::Display for MultiMcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoComponents => formatter.write_str("the modpack names no component"),
            Self::NotAModpack => formatter.write_str(
                "the archive holds no `instance.cfg`, so it is not a modpack of MultiMC",
            ),
            Self::UnsupportedFormat { uid, version } => write!(
                formatter,
                "the patch of {uid} uses the format {version}, which is not supported"
            ),
            Self::Patch { uid, message } => {
                write!(formatter, "the patch of {uid} is malformed: {message}")
            }
            Self::NoGame => formatter.write_str("the modpack names no version of Minecraft"),
        }
    }
}

impl std::error::Error for MultiMcError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::LibraryDownloads;

    /// Returns a patch of a component.
    fn patch(uid: &str, version: &str) -> MultiMcInstancePatch {
        MultiMcInstancePatch {
            format_version: 1,
            uid: uid.to_string(),
            version: Some(version.to_string()),
            ..MultiMcInstancePatch::default()
        }
    }

    /// Returns a library.
    fn library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            ..Library::default()
        }
    }

    /// Returns the text of the arguments of the game of a manifest.
    fn game_arguments(manifest: &InstanceManifest) -> Vec<String> {
        manifest
            .arguments
            .as_ref()
            .and_then(|arguments| arguments.game.as_ref())
            .map(|arguments| {
                arguments
                    .iter()
                    .filter_map(|argument| argument.value().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns the text of the arguments of the virtual machine of a manifest.
    fn jvm_arguments(manifest: &InstanceManifest) -> Vec<String> {
        manifest
            .arguments
            .as_ref()
            .and_then(|arguments| arguments.jvm.as_ref())
            .map(|arguments| {
                arguments
                    .iter()
                    .filter_map(|argument| argument.value().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn the_last_patch_names_the_version_and_the_others_fill_it_in() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.main_class = Some("net.minecraft.client.main.Main".to_string());
        game.minecraft_arguments = Some("--username ${auth_player_name}".to_string());
        game.asset_index = Some(AssetIndex {
            id: Some("1.20".to_string()),
            url: Some("https://example.com/1.20.json".to_string()),
            ..AssetIndex::default()
        });
        game.libraries = vec![library("com.mojang:brigadier:1.0.18")];

        let mut forge = patch("net.minecraftforge", "1.20.1-47.1.0");
        forge.libraries = vec![library("net.minecraftforge:forge:1.20.1-47.1.0")];
        forge.main_class = Some("cpw.mods.modlauncher.Launcher".to_string());

        let resolved =
            resolve_patches(&[game, forge], "mmc").expect("the patches should be resolved");
        let manifest = &resolved.manifest;

        // The patch of the last component names the class and the arguments.
        assert_eq!(
            manifest.main_class.as_deref(),
            Some(BOOTSTRAP_MAIN),
            "the bootstrap starts the game"
        );
        assert_eq!(
            game_arguments(manifest),
            vec!["--username", "${auth_player_name}"]
        );
        assert_eq!(
            manifest
                .asset_index
                .as_ref()
                .and_then(|index| index.id.as_deref()),
            Some("1.20")
        );
        assert_eq!(resolved.game_version.as_deref(), Some("1.20.1"));

        let names: Vec<&str> = manifest
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "org.jackhuang.hmcl:mmc-bootstrap:1.0",
                "com.mojang:brigadier:1.0.18",
                "net.minecraftforge:forge:1.20.1-47.1.0",
            ],
            "the libraries of the earlier component come first, the way the launcher of MultiMC merges them"
        );
    }

    #[test]
    fn the_arguments_of_the_components_before_the_last_are_kept() {
        let mut game = patch("net.minecraft", "1.7.10");
        game.minecraft_arguments = Some("--username ${auth_player_name}".to_string());
        game.jvm_args = vec!["-Dgame=true".to_string()];

        let mut forge = patch("net.minecraftforge", "1.7.10-10.13.4.1614");
        forge.jvm_args = vec!["-Dforge=true".to_string()];

        let resolved =
            resolve_patches(&[game, forge], "mmc").expect("the patches should be resolved");
        let jvm = jvm_arguments(&resolved.manifest);

        assert!(
            jvm.contains(&"-Dgame=true".to_string()),
            "the arguments of the first component are kept: {jvm:?}"
        );
        // The launcher of MultiMC reads the arguments of the components before
        // the last one only; the patch of the last component describes the
        // version itself and its arguments of the virtual machine are not read.
        assert!(
            !jvm.contains(&"-Dforge=true".to_string()),
            "the arguments of the last component are not read: {jvm:?}"
        );
    }

    #[test]
    fn the_tweakers_are_read_from_the_arguments_and_the_patches() {
        let mut game = patch("net.minecraft", "1.7.10");
        game.minecraft_arguments = Some(
            "--username ${auth_player_name} --tweakClass optifine.OptiFineTweaker".to_string(),
        );
        game.tweakers = vec!["com.mumfrey.liteloader.launch.LiteLoaderTweaker".to_string()];

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        let arguments = game_arguments(&resolved.manifest);

        assert_eq!(
            arguments,
            vec![
                "--username",
                "${auth_player_name}",
                "--tweakClass",
                "com.mumfrey.liteloader.launch.LiteLoaderTweaker",
                "--tweakClass",
                "optifine.OptiFineTweaker",
            ],
            "the tweaker of the patch is written before the tweaker of the arguments"
        );
    }

    #[test]
    fn the_tweakers_are_written_once() {
        let mut game = patch("net.minecraft", "1.12.2");
        game.minecraft_arguments = Some(
            "--tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker --tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker"
                .to_string(),
        );

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        let arguments = game_arguments(&resolved.manifest);

        assert_eq!(
            arguments,
            vec![
                "--tweakClass",
                "net.minecraftforge.fml.common.launcher.FMLTweaker"
            ]
        );
    }

    #[test]
    fn the_bootstrap_is_told_the_class_of_the_game() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.main_class = Some("net.minecraft.client.main.Main".to_string());

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        let jvm = jvm_arguments(&resolved.manifest);

        let bootstrap = jvm
            .iter()
            .find(|argument| argument.starts_with("-Dhmcl.mmc.bootstrap="))
            .expect("the bootstrap should be told the class of the game");
        assert!(
            bootstrap.contains("main_class=net.minecraft.client.main.Main"),
            "the address of the bootstrap names the class: {bootstrap}"
        );
        assert!(
            bootstrap.starts_with("-Dhmcl.mmc.bootstrap=hmcl:///bootstrap_profile_v1/?main_class="),
            "the address of the bootstrap is written as a query: {bootstrap}"
        );
    }

    #[test]
    fn a_patch_without_a_class_starts_the_default_class() {
        let resolved = resolve_patches(&[patch("net.minecraft", "1.7.10")], "mmc")
            .expect("the patches should be resolved");

        let jvm = jvm_arguments(&resolved.manifest);
        assert!(
            jvm.iter().any(|argument| argument
                == "-Dhmcl.mmc.bootstrap=hmcl:///bootstrap_profile_v1/?main_class=net.minecraft.client.Minecraft&installer=Patches%3A+recursive+install%2C+fabric+%26+quilt+intermediary%0AImplementation%3A+org.jackhuang.hmcl+%28Rust+port%2C+0.1.0%29"),
            "the default class is named: {jvm:?}"
        );
    }

    #[test]
    fn the_first_thread_trait_adds_the_argument_of_the_mac() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.traits = vec!["FirstThreadOnMacOS".to_string()];

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        let jvm = resolved
            .manifest
            .arguments
            .as_ref()
            .and_then(|arguments| arguments.jvm.as_ref())
            .expect("the arguments of the virtual machine should be kept");

        let argument = jvm
            .iter()
            .find(|argument| argument.values().contains(&"-XstartOnFirstThread"))
            .expect("the argument of the mac should be added");
        let rules = argument.rules().expect("the argument should be ruled");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].action, RuleAction::Allow);
        assert_eq!(
            rules[0].os.as_ref().and_then(|os| os.name.as_deref()),
            Some("osx")
        );
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn the_traits_the_launcher_does_not_know_are_reported() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.traits = vec![
            "texturepacks".to_string(),
            "FeatureThatDoesNotExist".to_string(),
        ];

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        assert_eq!(
            resolved.warnings,
            vec!["trait: FeatureThatDoesNotExist".to_string()]
        );
    }

    #[test]
    fn the_wrapper_of_forge_is_told_where_the_libraries_are() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.libraries = vec![
            library("io.github.zekerzhayard:ForgeWrapper:1.5.6"),
            library("net.minecraftforge:forge:1.20.1-47.1.0:installer"),
        ];

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        let jvm = jvm_arguments(&resolved.manifest);

        assert!(jvm.contains(&"-Dforgewrapper.librariesDir=${library_directory}".to_string()));
        assert!(jvm.contains(&"-Dforgewrapper.minecraft=${primary_jar}".to_string()));
        assert!(jvm.contains(
            &"-Dforgewrapper.installer=${library_directory}/net/minecraftforge/forge/1.20.1-47.1.0/forge-1.20.1-47.1.0-installer.jar"
                .to_string()
        ));
    }

    #[test]
    fn the_newest_java_the_launcher_knows_is_written() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.java_majors = Some(vec![21, 17]);

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        let version = resolved
            .manifest
            .java_version
            .as_ref()
            .expect("the version of Java should be written");
        assert_eq!(version.major_version, Some(21));
        assert_eq!(version.component.as_deref(), Some("java-runtime-delta"));
    }

    #[test]
    fn a_version_of_java_the_launcher_does_not_know_is_reported() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.java_majors = Some(vec![30]);

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        assert_eq!(resolved.manifest.java_version, None);
        assert_eq!(
            resolved.warnings,
            vec!["java version range: [30]".to_string()]
        );
    }

    #[test]
    fn the_file_of_the_game_is_the_main_jar_of_the_patches() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.main_jar = Some(Library {
            name: "com.mojang:minecraft:1.20.1:client".to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(DownloadInfo {
                    url: "https://example.com/client.jar".to_string(),
                    sha1: Some("abc".to_string()),
                    size: Some(100),
                    path: None,
                }),
                classifiers: None,
            }),
            ..Library::default()
        });

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        let client = resolved
            .manifest
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.get("client"))
            .expect("the file of the game should be named");
        assert_eq!(client.url, "https://example.com/client.jar");
        assert_eq!(client.sha1.as_deref(), Some("abc"));
    }

    #[test]
    fn the_archives_of_the_components_are_kept() {
        let mut game = patch("net.minecraft", "1.7.10");
        game.jar_mods = vec![
            Library {
                name: "local:archive:1.0".to_string(),
                filename: Some("archive.jar".to_string()),
                ..Library::default()
            },
            Library {
                name: "local:archive:1.0".to_string(),
                filename: Some("archive.jar".to_string()),
                ..Library::default()
            },
        ];

        let resolved = resolve_patches(&[game], "mmc").expect("the patches should be resolved");
        assert_eq!(resolved.jar_mods, vec!["archive.jar".to_string()]);
    }

    #[test]
    fn a_patch_of_a_format_the_launcher_does_not_know_is_refused() {
        let mut game = patch("net.minecraft", "1.20.1");
        game.format_version = 2;

        assert_eq!(
            resolve_patches(&[game], "mmc"),
            Err(MultiMcError::UnsupportedFormat {
                uid: "net.minecraft".to_string(),
                version: 2,
            })
        );
        assert_eq!(resolve_patches(&[], "mmc"), Err(MultiMcError::NoComponents));
    }

    #[test]
    fn the_patches_are_read_from_the_json_of_the_modpack() {
        let patch = MultiMcInstancePatch::read(
            "net.minecraftforge",
            r#"{
                "formatVersion": 1,
                "uid": "net.minecraftforge",
                "version": "1.20.1-47.1.0",
                "mainClass": "cpw.mods.modlauncher.Launcher",
                "+jvmArgs": ["-Dforge=true"],
                "+libraries": [
                    { "name": "net.minecraftforge:forge:1.20.1-47.1.0", "MMC-hint": "local" }
                ],
                "libraries": [{ "name": "com.example:old:1.0" }],
                "requires": [{ "uid": "net.minecraft", "equals": "1.20.1" }]
            }"#,
        )
        .expect("the patch should be read");

        assert_eq!(patch.uid, "net.minecraftforge");
        assert_eq!(
            patch.main_class.as_deref(),
            Some("cpw.mods.modlauncher.Launcher")
        );
        assert_eq!(patch.jvm_args, vec!["-Dforge=true".to_string()]);
        assert_eq!(patch.libraries().len(), 2);
        assert_eq!(patch.libraries()[0].hint.as_deref(), Some("local"));
        assert_eq!(patch.requires[0].equals.as_deref(), Some("1.20.1"));
    }

    #[test]
    fn a_patch_that_is_not_json_is_refused() {
        let error = MultiMcInstancePatch::read("net.minecraft", "not json")
            .expect_err("the patch should be refused");
        assert!(matches!(error, MultiMcError::Patch { .. }));
    }

    #[test]
    fn the_settings_of_an_instance_are_read() {
        let config = MultiMcConfig::parse(
            "# a comment\r\ninstanceType=OneSix\r\nname=My Instance\r\nMaxMemAlloc=2048\r\n\
             OverrideMemory=true\r\nLaunchMaximized=false\r\nnotes=The notes: of the player\r\n\
             IntendedVersion:1.20.1\r\n",
        );

        assert_eq!(config.value("instanceType"), Some("OneSix"));
        assert_eq!(config.value("name"), Some("My Instance"));
        assert_eq!(config.number("MaxMemAlloc"), Some(2048));
        assert!(config.overrides_memory());
        assert!(!config.fullscreen());
        assert_eq!(config.value("IntendedVersion"), Some("1.20.1"));
        assert!(config.value("Missing").is_none());
    }

    #[test]
    fn a_value_the_launcher_of_multimc_quoted_is_read() {
        // Some versions of the launcher of MultiMC wrote a quote in front of
        // the value and a colon after it, and the colon is not part of the
        // value.
        let config = MultiMcConfig::parse("name=\"My Instance:\n");
        assert_eq!(config.value("name"), Some("\"My Instance"));
    }

    #[test]
    fn the_value_of_a_key_keeps_the_escapes() {
        let config = MultiMcConfig::parse("PreLaunchCommand=echo\\ a\\\\ b\\u0041\n");
        assert_eq!(
            config.value("PreLaunchCommand"),
            Some("echo a\\ bA"),
            "the escaped space is kept and the escaped backslash is read"
        );
    }

    /// The settings of an instance the tests apply the settings of MultiMC to.
    fn settings() -> GameSettingsValues {
        GameSettingsValues::default()
    }

    #[test]
    fn a_setting_the_instance_does_not_override_is_kept() {
        let config = MultiMcConfig::parse("MaxMemAlloc=4096\nJvmArgs=-Xmx4G\n");
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.max_memory, None, "the instance may not set it");
        assert_eq!(values.jvm_options, None, "the instance may not set it");
    }

    #[test]
    fn the_memory_the_instance_names_is_applied() {
        let config = MultiMcConfig::parse(
            "OverrideMemory=true\nMaxMemAlloc=4096\nMinMemAlloc=512\nPermGen=256\n",
        );
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.max_memory, Some(4096));
        assert_eq!(values.min_memory, Some(512));
        assert_eq!(values.perm_size.as_deref(), Some("256"));
        // The instance asks for an amount of its own.
        assert_eq!(values.auto_memory, Some(false));
    }

    #[test]
    fn the_java_the_instance_names_is_applied() {
        let config =
            MultiMcConfig::parse("OverrideJavaLocation=true\nJavaPath=C:/java/bin/java.exe\n");
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.java_type, Some(JavaVersionType::Custom));
        assert_eq!(
            values.custom_java_path.as_deref(),
            Some("C:/java/bin/java.exe")
        );
    }

    #[test]
    fn the_arguments_the_instance_names_replace_the_ones_of_the_launcher() {
        let config = MultiMcConfig::parse("OverrideJavaArgs=true\nJvmArgs=-Xmx4G\n");
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.jvm_options.as_deref(), Some("-Xmx4G"));
        assert_eq!(values.no_optimizing_jvm_options, Some(true));
    }

    #[test]
    fn the_commands_the_instance_names_are_applied() {
        let config = MultiMcConfig::parse(
            "OverrideCommands=true\nWrapperCommand=gamemoderun\nPreLaunchCommand=echo before\n\
             PostExitCommand=echo after\n",
        );
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.command_wrapper.as_deref(), Some("gamemoderun"));
        assert_eq!(values.pre_launch_command.as_deref(), Some("echo before"));
        assert_eq!(values.post_exit_command.as_deref(), Some("echo after"));
    }

    #[test]
    fn a_window_of_a_size_is_windowed() {
        let config = MultiMcConfig::parse(
            "OverrideWindow=true\nMinecraftWinWidth=854\nMinecraftWinHeight=480\n",
        );
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.window_type, Some(GameWindowType::Windowed));
        assert_eq!(values.width, Some(854.0));
        assert_eq!(values.height, Some(480.0));
    }

    #[test]
    fn a_window_the_instance_asks_to_maximize_is_maximized() {
        // The launcher of MultiMC starts a maximized window when the player set
        // no size at all.
        let config = MultiMcConfig::parse("OverrideWindow=true\n");
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.window_type, Some(GameWindowType::Maximized));
        assert_eq!(values.width, None);
    }

    #[test]
    fn a_full_window_is_written_as_fullscreen() {
        let config = MultiMcConfig::parse(
            "OverrideWindow=true\nLaunchMaximized=true\nMinecraftWinWidth=854\n",
        );
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.window_type, Some(GameWindowType::Fullscreen));
    }

    #[test]
    fn the_console_the_instance_names_is_applied() {
        let config = MultiMcConfig::parse("OverrideConsole=true\nShowConsole=true\n");
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.show_logs, Some(true));
        assert_eq!(values.not_use_javaw, Some(true));
    }

    #[test]
    fn a_console_the_instance_does_not_show_is_hidden() {
        let config = MultiMcConfig::parse(
            "OverrideConsole=true\nShowConsole=false\nShowConsoleOnError=false\nAutoCloseConsole=true\n",
        );
        let mut values = settings();

        apply_to_settings(&config, &mut values);

        assert_eq!(values.show_logs, Some(false));
        assert_eq!(values.not_use_javaw, Some(false));
    }

    #[test]
    fn the_version_of_the_game_is_named_by_the_components() {
        let manifest = MultiMcManifest {
            format_version: 1,
            components: vec![
                MultiMcComponent {
                    uid: "net.minecraft".to_string(),
                    version: Some("1.20.1".to_string()),
                    important: true,
                    ..MultiMcComponent::default()
                },
                MultiMcComponent {
                    uid: "net.minecraftforge".to_string(),
                    version: Some("1.20.1-47.1.0".to_string()),
                    ..MultiMcComponent::default()
                },
            ],
        };

        assert_eq!(manifest.game_version(), Some("1.20.1"));

        let config = MultiMcConfig::parse("IntendedVersion=1.7.10");
        assert_eq!(config.game_version(Some(&manifest)), Some("1.20.1"));
        assert_eq!(config.game_version(None), Some("1.7.10"));
    }

    #[test]
    fn the_components_are_named_the_way_multimc_names_them() {
        assert_eq!(
            component_of_uid("net.minecraft"),
            Some(GameComponentType::Game)
        );
        assert_eq!(
            component_of_uid("net.fabricmc.fabric-loader"),
            Some(GameComponentType::Fabric)
        );
        assert_eq!(
            component_of_uid("org.quiltmc.quilt-loader"),
            Some(GameComponentType::Quilt)
        );
        assert_eq!(component_of_uid("org.lwjgl3"), None);
    }

    #[test]
    fn the_address_of_a_patch_falls_back_the_way_multimc_does() {
        assert_eq!(
            meta_url("net.minecraft", Some("1.20.1"), "1.20.1").as_deref(),
            Some("https://meta.multimc.org/v1/net.minecraft/1.20.1.json")
        );
        assert_eq!(
            meta_url("org.lwjgl", None, "1.20.1").as_deref(),
            Some("https://meta.multimc.org/v1/org.lwjgl/2.9.1.json")
        );
        assert_eq!(
            meta_url("org.lwjgl3", None, "1.20.1").as_deref(),
            Some("https://meta.multimc.org/v1/org.lwjgl3/3.1.2.json")
        );
        assert_eq!(
            meta_url("net.fabricmc.intermediary", None, "1.20.1").as_deref(),
            Some("https://meta.multimc.org/v1/net.fabricmc.intermediary/1.20.1.json")
        );
        assert_eq!(meta_url("com.example.component", None, "1.20.1"), None);
    }

    #[test]
    fn the_address_of_the_bootstrap_holds_the_class_of_the_game() {
        assert_eq!(
            bootstrap_profile_url("net.minecraft.client.main.Main", "Patches: a & b"),
            "hmcl:///bootstrap_profile_v1/?main_class=net.minecraft.client.main.Main&installer=Patches%3A+a+%26+b"
        );
        assert_eq!(
            bootstrap_profile_url("a.b.C", "x_y-z.1*2"),
            "hmcl:///bootstrap_profile_v1/?main_class=a.b.C&installer=x_y-z.1*2"
        );
    }

    #[test]
    fn the_root_of_the_instance_is_found() {
        assert_eq!(
            root_directory(["instance.cfg", "mmc-pack.json"]),
            Ok(String::new())
        );
        assert_eq!(
            root_directory(["My Pack/instance.cfg", "My Pack/mmc-pack.json"]),
            Ok("My Pack/".to_string())
        );
        assert_eq!(
            root_directory(["instance.cfg.bak", "mmc-pack.json"]),
            Err(MultiMcError::NotAModpack)
        );
        assert_eq!(
            root_directory(["My Pack/nested/instance.cfg"]),
            Err(MultiMcError::NotAModpack),
            "the file is not directly inside the directory"
        );
    }

    #[test]
    fn a_modpack_without_the_game_is_refused() {
        let manifest = MultiMcManifest {
            format_version: 1,
            components: vec![MultiMcComponent {
                uid: "net.minecraftforge".to_string(),
                version: Some("1.20.1-47.1.0".to_string()),
                ..MultiMcComponent::default()
            }],
        };

        assert_eq!(manifest.require_game_version(), Err(MultiMcError::NoGame));

        let manifest = MultiMcManifest {
            format_version: 1,
            components: vec![MultiMcComponent {
                uid: "net.minecraft".to_string(),
                version: Some("1.20.1".to_string()),
                ..MultiMcComponent::default()
            }],
        };
        assert_eq!(manifest.require_game_version(), Ok("1.20.1"));
    }

    #[test]
    fn the_installer_profile_names_the_implementation() {
        let profile = installer_profile();
        assert!(profile.starts_with("Patches: recursive install, fabric & quilt intermediary\n"));
        assert!(profile.contains("Implementation: org.jackhuang.hmcl"));
    }
}
