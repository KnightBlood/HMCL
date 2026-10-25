//! The native libraries of the versions of the game.
//!
//! The versions of the game bring the native libraries of the systems Mojang
//! builds them for; a machine of another architecture, such as an ARM one, has
//! no native libraries of its own. The launcher keeps a list of the native
//! libraries the players of those machines built, and replaces the native
//! libraries of the version with the ones of the list.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::download::game::{rules_allow, InstanceManifest, Library, RuleContext};
use crate::java::Platform;
use crate::util::operating_system::{current_arch, current_os, Arch, OsType};
use crate::util::versioning::GameVersionNumber;

use super::classpath::{is_native, native_classifier};

/// The libraries a platform replaces the libraries of a version with.
///
/// A library of the list that is written as `null` is one the launcher drops
/// rather than replaces, so the value is optional rather than the entry being
/// absent.
type ReplacementList = BTreeMap<String, Option<Library>>;

/// The list of the native libraries the players of the machines without
/// official native libraries built, keyed by the name of the platform the
/// libraries were built for.
const NATIVES: &str = include_str!("../../../resources/game/natives.json");

/// The native libraries the players built, keyed by the platform they were
/// built for.
static NATIVES_OF_PLATFORM: OnceLock<BTreeMap<String, ReplacementList>> = OnceLock::new();

/// The settings of a start that tell the launcher how to read the native
/// libraries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePatchOptions {
    /// Whether the player brings the native libraries of their own.
    pub use_custom_natives: bool,
    /// Whether the game uses the GLFW and the SDL of the machine.
    pub use_native_glfw_or_sdl: bool,
    /// Whether the game uses the OpenAL of the machine.
    pub use_native_openal: bool,
    /// Whether the launcher leaves the native libraries of the version alone.
    pub not_patch_natives: bool,
}

/// Returns a copy of the manifest that holds the given libraries.
///
/// The manifest of an instance is built once and then read from everywhere, so
/// the launcher copies it when it changes the libraries of a start.
fn with_libraries(manifest: &InstanceManifest, libraries: Vec<Library>) -> InstanceManifest {
    InstanceManifest {
        libraries,
        ..manifest.clone()
    }
}

/// Returns the native libraries the players built for a platform.
fn natives_of(platform: Platform) -> &'static ReplacementList {
    NATIVES_OF_PLATFORM
        .get_or_init(|| {
            serde_json::from_str::<BTreeMap<String, ReplacementList>>(NATIVES).unwrap_or_default()
        })
        .get(&platform_name(platform))
        .unwrap_or_else(|| empty_natives())
}

/// Returns an empty list of native libraries.
fn empty_natives() -> &'static ReplacementList {
    static EMPTY: OnceLock<ReplacementList> = OnceLock::new();
    EMPTY.get_or_init(ReplacementList::new)
}

/// Returns the name of the platform the way the list of the native libraries
/// names it.
///
/// The list names the platforms the way the release files of the Java runtimes
/// do, such as `windows-x86_64`, rather than the way the directories of the
/// launcher are named, such as `windows-x86-64`.
pub fn platform_name(platform: Platform) -> String {
    let os = match platform.os {
        OsType::Windows => "windows",
        OsType::Linux => "linux",
        OsType::MacOs => "macos",
        OsType::Unknown => "universal",
    };
    let arch = match platform.arch {
        Arch::X86_64 => "x86_64",
        Arch::X86 => "x86",
        Arch::Aarch64 => "arm64",
        Arch::Arm => "arm32",
        Arch::Unknown => "unknown",
    };
    format!("{os}-{arch}")
}

/// Returns whether the operating system is a Linux one or a BSD one.
fn is_linux_or_bsd(os: OsType) -> bool {
    // The launcher reads the systems of the BSD family as Linux ones, the way
    // the versions of Java read them.
    matches!(os, OsType::Linux)
}

/// Replaces the native libraries of a version with the ones the players of the
/// machine built.
///
/// The player may bring the native libraries of their own, in which case the
/// launcher drops the native libraries of LWJGL altogether; the player may also
/// tell the launcher to leave the native libraries alone.
///
/// `java_arguments` holds the arguments of the virtual machine the player set.
/// When the version of LWJGL changes, the game of some versions of Sodium
/// crashes; the launcher adds an argument that keeps it from crashing.
pub fn patch_natives(
    manifest: &InstanceManifest,
    game_version: &GameVersionNumber,
    platform: Platform,
    options: NativePatchOptions,
    context: &RuleContext<'_>,
    java_arguments: &mut Vec<String>,
) -> InstanceManifest {
    // The player brings the native libraries of their own, so the ones of the
    // version are dropped rather than replaced.
    if options.use_custom_natives {
        if *game_version < GameVersionNumber::as_game_version("1.19") {
            return manifest.clone();
        }

        return with_libraries(
            manifest,
            manifest
                .libraries
                .iter()
                .filter(|library| {
                    rules_allow(library.rules.as_deref(), context)
                        && !is_dropped_by_custom_natives(library, context)
                })
                .cloned()
                .collect(),
        );
    }

    // The player may use the GLFW, the SDL and the OpenAL of the machine; the
    // native libraries of the version are then dropped, so that the ones of
    // the machine are read.
    if is_linux_or_bsd(platform.os)
        && (options.use_native_glfw_or_sdl || options.use_native_openal)
        && *game_version >= GameVersionNumber::as_game_version("1.19")
    {
        return with_libraries(
            manifest,
            manifest
                .libraries
                .iter()
                .filter(|library| {
                    !is_native_dropped_for_system_libraries(
                        library,
                        context,
                        options.use_native_glfw_or_sdl,
                        options.use_native_openal,
                    )
                })
                .cloned()
                .collect(),
        );
    }

    if options.not_patch_natives {
        return manifest.clone();
    }

    // The x86 runtimes and the ones of the machines the Mojang builds the
    // native libraries for have the native libraries of the version already.
    let arch = platform.arch;
    if arch.is_x86() && matches!(platform.os, OsType::Windows | OsType::Linux | OsType::MacOs) {
        return manifest.clone();
    }

    if arch == Arch::Aarch64
        && matches!(platform.os, OsType::MacOs | OsType::Windows)
        && *game_version >= GameVersionNumber::as_game_version("1.19")
    {
        return manifest.clone();
    }

    let replacements = natives_of(platform);
    if replacements.is_empty() {
        return manifest.clone();
    }

    let mut lwjgl_version_changed = false;
    let mut libraries = Vec::new();
    for library in &manifest.libraries {
        if !rules_allow(library.rules.as_deref(), context) {
            continue;
        }

        // The native libraries of a version are looked up with the `natives`
        // classifier, and the library itself with its own name.
        let native = is_native(library);
        let key = if native {
            format!("{}:natives", library.name)
        } else {
            library.name.clone()
        };

        match replacements.get(&key) {
            // The library has no replacement, so the version keeps its own.
            None => libraries.push(library.clone()),
            // The library is dropped rather than replaced.
            Some(None) => {}
            Some(Some(replacement)) => {
                libraries.push(replacement.clone());

                // The game of some versions of Sodium crashes when the version
                // of LWJGL itself changes.
                //
                // The Java launcher compares the whole name of the library with
                // `org.lwjgl:lwjgl`, which the name of a library never equals,
                // so this branch never runs there; the launcher reads the group
                // and the name of the library instead.
                if !native
                    && library.artifact().is_ok_and(|artifact| {
                        artifact.group == "org.lwjgl"
                            && artifact.name == "lwjgl"
                            && artifact.classifier.is_none()
                    })
                    && library_version(library) != library_version(replacement)
                {
                    lwjgl_version_changed = true;
                }
            }
        }
    }

    if lwjgl_version_changed {
        // https://github.com/CaffeineMC/sodium/issues/2561
        java_arguments.push("-Dsodium.checks.issue2561=false".to_string());
    }

    with_libraries(manifest, libraries)
}

/// Returns whether the library is one the launcher drops when the player brings
/// the native libraries of their own.
///
/// The classifier of a native library is either written into the name of the
/// library or named by the `natives` map of the version, so the classifier is
/// read the way the version reads the files of the library.
fn is_dropped_by_custom_natives(library: &Library, context: &RuleContext<'_>) -> bool {
    let Ok(artifact) = library.artifact() else {
        return false;
    };

    if !artifact.name.starts_with("lwjgl") {
        return false;
    }

    native_classifier(library, context).is_some_and(|classifier| classifier.starts_with("natives"))
}

/// Returns whether the native library is one the launcher drops when the game
/// uses the libraries of the machine.
fn is_native_dropped_for_system_libraries(
    library: &Library,
    context: &RuleContext<'_>,
    use_native_glfw_or_sdl: bool,
    use_native_openal: bool,
) -> bool {
    let Ok(artifact) = library.artifact() else {
        return false;
    };

    if artifact.group != "org.lwjgl" {
        return false;
    }

    // The library is only one of the native libraries when the classifier of
    // its native files is read from its name or from its `natives` map.
    if !native_classifier(library, context)
        .is_some_and(|classifier| classifier.starts_with("natives"))
    {
        return false;
    }

    (use_native_glfw_or_sdl && matches!(artifact.name.as_str(), "lwjgl-glfw" | "lwjgl-sdl"))
        || (use_native_openal && artifact.name == "lwjgl-openal")
}

/// Returns the version of a library, or nothing when its name cannot be read.
fn library_version(library: &Library) -> Option<String> {
    library
        .artifact()
        .ok()
        .map(|artifact| artifact.version.clone())
}

/// Returns the context the rules of the libraries are read in for the machine
/// of the player.
pub fn current_context<'a>(features: &'a BTreeMap<String, bool>) -> RuleContext<'a> {
    RuleContext::new(
        super::os_name(current_os()),
        "",
        current_arch().rule_name(),
        features,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::CompatibilityRule;

    fn library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            ..Library::default()
        }
    }

    fn native(name: &str) -> Library {
        let mut library = library(name);
        library.natives = Some(BTreeMap::from([(
            "linux-arm64".to_string(),
            "natives-linux-arm64".to_string(),
        )]));
        library
    }

    fn context<'a>(features: &'a BTreeMap<String, bool>) -> RuleContext<'a> {
        RuleContext::new("linux", "", "arm64", features)
    }

    fn manifest(libraries: Vec<Library>) -> InstanceManifest {
        InstanceManifest {
            libraries,
            ..InstanceManifest::default()
        }
    }

    fn options() -> NativePatchOptions {
        NativePatchOptions {
            use_custom_natives: false,
            use_native_glfw_or_sdl: false,
            use_native_openal: false,
            not_patch_natives: false,
        }
    }

    #[test]
    fn the_platforms_of_the_list_are_named_the_way_the_java_runtimes_are() {
        assert_eq!(
            platform_name(Platform::new(OsType::Windows, Arch::X86_64)),
            "windows-x86_64"
        );
        assert_eq!(
            platform_name(Platform::new(OsType::Linux, Arch::Aarch64)),
            "linux-arm64"
        );
        assert_eq!(
            platform_name(Platform::new(OsType::MacOs, Arch::Aarch64)),
            "macos-arm64"
        );
    }

    #[test]
    fn an_x86_runtime_keeps_the_native_libraries_of_the_version() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![native("org.lwjgl:lwjgl:3.3.2")]),
            &GameVersionNumber::as_game_version("1.20.1"),
            Platform::new(OsType::Linux, Arch::X86_64),
            options(),
            &context(&features),
            &mut java_arguments,
        );

        assert_eq!(result.libraries.len(), 1);
        assert_eq!(result.libraries[0].name, "org.lwjgl:lwjgl:3.3.2");
    }

    #[test]
    fn the_player_may_tell_the_launcher_to_leave_the_natives_alone() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![native("org.lwjgl:lwjgl:3.3.2")]),
            &GameVersionNumber::as_game_version("1.20.1"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            NativePatchOptions {
                not_patch_natives: true,
                ..options()
            },
            &context(&features),
            &mut java_arguments,
        );

        assert_eq!(result.libraries.len(), 1);
        assert_eq!(result.libraries[0].name, "org.lwjgl:lwjgl:3.3.2");
    }

    #[test]
    fn the_native_libraries_of_the_version_are_replaced() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![native("org.lwjgl:lwjgl:3.1.6")]),
            &GameVersionNumber::as_game_version("1.16.5"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            options(),
            &context(&features),
            &mut java_arguments,
        );

        assert_eq!(result.libraries.len(), 1);
        // The list replaces the native library of LWJGL 3.1.6 with the one of
        // LWJGL 3.3.2 built for the machines of the ARM architecture.
        assert_eq!(
            result.libraries[0].name,
            "org.lwjgl:lwjgl:3.3.2:natives-linux-arm64"
        );
        assert_eq!(
            result.libraries[0]
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
                .map(|artifact| artifact.url.as_str()),
            Some(
                "https://repo1.maven.org/maven2/org/lwjgl/lwjgl/3.3.2/lwjgl-3.3.2-natives-linux-arm64.jar"
            )
        );
        // The native library replaces the one of the version, so the version of
        // LWJGL itself is left alone.
        assert_eq!(java_arguments, Vec::<String>::new());
    }

    #[test]
    fn the_library_of_lwjgl_itself_changes_its_version() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![library("org.lwjgl:lwjgl:3.1.6")]),
            &GameVersionNumber::as_game_version("1.16.5"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            options(),
            &context(&features),
            &mut java_arguments,
        );

        // The library itself is not a native one, so the list replaces it with
        // the library of the same name rather than with the native files.
        assert_eq!(result.libraries.len(), 1);
        assert_eq!(result.libraries[0].name, "org.lwjgl:lwjgl:3.3.2");
        // The version of LWJGL changed, so the argument that keeps Sodium from
        // crashing is added.
        assert_eq!(java_arguments, vec!["-Dsodium.checks.issue2561=false"]);
    }

    #[test]
    fn a_library_the_list_holds_no_replacement_for_is_dropped() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let mut words = native("com.mojang:text2speech:1.10.3");
        words.natives = Some(BTreeMap::from([(
            "linux-arm64".to_string(),
            "natives-linux".to_string(),
        )]));

        let result = patch_natives(
            &manifest(vec![words]),
            &GameVersionNumber::as_game_version("1.16.5"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            options(),
            &context(&features),
            &mut java_arguments,
        );

        // The list writes the library as `null`, which tells the launcher to
        // drop it rather than replace it.
        assert!(result.libraries.is_empty());
    }

    #[test]
    fn the_libraries_of_other_systems_are_dropped() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let mut macos = library("com.example:macos:1.0");
        macos.rules = Some(vec![CompatibilityRule {
            action: crate::download::game::RuleAction::Allow,
            os: Some(crate::download::game::OsRule {
                name: Some("osx".to_string()),
                version: None,
                arch: None,
            }),
            features: None,
        }]);

        let result = patch_natives(
            &manifest(vec![macos, library("com.example:linux:1.0")]),
            &GameVersionNumber::as_game_version("1.16.5"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            options(),
            &context(&features),
            &mut java_arguments,
        );

        assert_eq!(result.libraries.len(), 1);
        assert_eq!(result.libraries[0].name, "com.example:linux:1.0");
    }

    #[test]
    fn the_custom_natives_drop_the_native_libraries_of_lwjgl() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![
                native("org.lwjgl:lwjgl-glfw:3.3.2"),
                library("com.example:library:1.0"),
            ]),
            &GameVersionNumber::as_game_version("1.20.1"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            NativePatchOptions {
                use_custom_natives: true,
                ..options()
            },
            &context(&features),
            &mut java_arguments,
        );

        assert_eq!(result.libraries.len(), 1);
        assert_eq!(result.libraries[0].name, "com.example:library:1.0");
    }

    #[test]
    fn the_versions_before_1_19_keep_their_natives_when_the_player_brings_their_own() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![native("org.lwjgl:lwjgl-glfw:3.3.2")]),
            &GameVersionNumber::as_game_version("1.12.2"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            NativePatchOptions {
                use_custom_natives: true,
                ..options()
            },
            &context(&features),
            &mut java_arguments,
        );

        assert_eq!(result.libraries.len(), 1);
        assert_eq!(result.libraries[0].name, "org.lwjgl:lwjgl-glfw:3.3.2");
    }

    #[test]
    fn the_glfw_of_the_machine_replaces_the_one_of_the_version() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![native("org.lwjgl:lwjgl-glfw:3.3.2")]),
            &GameVersionNumber::as_game_version("1.20.1"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            NativePatchOptions {
                use_native_glfw_or_sdl: true,
                ..options()
            },
            &context(&features),
            &mut java_arguments,
        );

        assert!(result.libraries.is_empty());
    }

    #[test]
    fn the_openal_of_the_machine_replaces_the_one_of_the_version() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![native("org.lwjgl:lwjgl-openal:3.3.2")]),
            &GameVersionNumber::as_game_version("1.20.1"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            NativePatchOptions {
                use_native_openal: true,
                ..options()
            },
            &context(&features),
            &mut java_arguments,
        );

        assert!(result.libraries.is_empty());
    }

    #[test]
    fn the_library_of_another_name_is_kept_when_the_glfw_of_the_machine_is_used() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![native("org.lwjgl:lwjgl-jemalloc:3.3.2")]),
            &GameVersionNumber::as_game_version("1.20.1"),
            Platform::new(OsType::Linux, Arch::Aarch64),
            NativePatchOptions {
                use_native_glfw_or_sdl: true,
                ..options()
            },
            &context(&features),
            &mut java_arguments,
        );

        assert_eq!(result.libraries.len(), 1);
        assert_eq!(result.libraries[0].name, "org.lwjgl:lwjgl-jemalloc:3.3.2");
    }

    #[test]
    fn a_platform_without_a_list_keeps_the_native_libraries_of_the_version() {
        let features = BTreeMap::new();
        let mut java_arguments = Vec::new();
        let result = patch_natives(
            &manifest(vec![native("org.lwjgl:lwjgl:3.3.2")]),
            &GameVersionNumber::as_game_version("1.16.5"),
            Platform::new(OsType::MacOs, Arch::X86),
            options(),
            &context(&features),
            &mut java_arguments,
        );

        assert_eq!(result.libraries.len(), 1);
        assert_eq!(result.libraries[0].name, "org.lwjgl:lwjgl:3.3.2");
    }
}
