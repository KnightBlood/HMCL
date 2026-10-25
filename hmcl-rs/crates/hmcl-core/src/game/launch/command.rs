//! The command a version of the game is started with.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use crate::auth::AuthInfo;
use crate::download::game::{InstanceManifest, RuleContext};
use crate::game::graphics::GraphicsApi;
use crate::util::operating_system::{total_physical_memory, OsType};
use crate::util::versioning::GameVersionNumber;

use super::arguments::{
    default_game_arguments, default_jvm_arguments, resolve_arguments, split_arguments,
};
use super::classpath::classpath;
use super::options::{LaunchOptions, LaunchPaths};
use super::variables::LaunchVariables;

/// The parts of the command a version of the game is started with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchCommand {
    /// The files the game reads its classes from.
    pub classpath: Vec<PathBuf>,
    /// The class the game is started with.
    pub main_class: String,
    /// The arguments of the virtual machine.
    pub jvm_arguments: Vec<String>,
    /// The arguments of the game.
    pub game_arguments: Vec<String>,
}

impl LaunchCommand {
    /// Returns the arguments of the process, without the path of the program
    /// that runs the game.
    pub fn arguments(&self) -> Vec<String> {
        let mut arguments = self.jvm_arguments.clone();
        arguments.push(self.main_class.clone());
        arguments.extend(self.game_arguments.iter().cloned());
        arguments
    }
}

/// The error of building the command of a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    /// The version does not tell the class the game is started with.
    MissingMainClass,
}

impl fmt::Display for LaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMainClass => {
                formatter.write_str("The version does not tell the class the game is started with")
            }
        }
    }
}

impl std::error::Error for LaunchError {}

/// Returns the name of the system the rules use.
pub const fn os_name(os: OsType) -> &'static str {
    match os {
        OsType::Windows => "windows",
        OsType::Linux => "linux",
        OsType::MacOs => "osx",
        OsType::Unknown => "",
    }
}

/// Returns the name of the architecture the rules use.
///
/// The architectures of the machines the launcher does not know have no name;
/// the rules about the architectures do not apply to them.
pub const fn arch_name(arch: crate::util::operating_system::Arch) -> &'static str {
    arch.rule_name()
}

/// Returns the environment the rules of a version are read in.
pub fn rule_context<'a>(
    options: &'a LaunchOptions,
    features: &'a BTreeMap<String, bool>,
) -> RuleContext<'a> {
    RuleContext::new(
        os_name(options.os),
        options.os_version.as_deref().unwrap_or(""),
        arch_name(options.arch),
        features,
    )
}

/// The name of the file of the agent that gives the game back the access to
/// the memory it uses by itself.
///
/// The agent is the one of [LWJGL] `3.4.1`, which changed the way it uses the
/// memory the game is given.
///
/// [LWJGL]: https://github.com/LWJGL/lwjgl3/issues/1111
const LWJGL_UNSAFE_AGENT_FILE: &str = "lwjgl-unsafe-agent.jar";

/// The first version of the game that reads the interface it draws with from
/// `--graphicsBackend`.
///
/// The versions before it draw with OpenGL only, so the launcher does not pass
/// the argument to them.
const GRAPHICS_BACKEND_VERSION: &str = "26.2-snapshot-2";

/// Returns whether the version needs the agent that gives it back the access
/// to the memory it uses by itself.
///
/// LWJGL `3.4.1` writes the memory of the game through the memory of the
/// virtual machine, which Java `25` and `26` no longer allow without a
/// warning; the agent gives the library back the access it had before.
pub fn need_patch_memory_util(manifest: &InstanceManifest, java_version: u32) -> bool {
    if !(25..=26).contains(&java_version) {
        return false;
    }

    manifest.libraries.iter().any(|library| {
        library.artifact().is_ok_and(|artifact| {
            artifact.group == "org.lwjgl"
                && artifact.name == "lwjgl"
                && artifact.version == "3.4.1"
                && artifact.classifier.is_none()
        })
    })
}

/// Returns the name of the encoding the virtual machine reports its output in.
fn encoding_name() -> &'static str {
    // The launcher writes the text of the game in the encoding of the system,
    // which the launcher does not read yet; the name is the one Java uses for
    // the encoding of the system of the player.
    "UTF-8"
}

/// Adds an argument of the virtual machine unless the player set one that
/// begins the same way.
///
/// The player may have written `-Xmx4G` instead of the amount of memory the
/// launcher chose, or a value of one of the properties the launcher writes;
/// the argument of the player wins.
fn add_default(arguments: &mut Vec<String>, option: &str, value: &str) {
    if arguments
        .iter()
        .any(|argument| argument.starts_with(option))
    {
        return;
    }

    arguments.push(format!("{option}{value}"));
}

/// Adds an argument of the virtual machine that holds a boolean value, unless
/// the player set one that writes the same option.
fn add_unstable_default(arguments: &mut Vec<String>, option: &str, value: bool) {
    let sign = if value { "+" } else { "-" };
    let key = format!("-XX:{sign}{option}");

    if arguments.iter().any(|argument| argument == &key) {
        return;
    }

    arguments.push(key);
}

/// Adds an argument of the virtual machine that holds a text value, unless the
/// player set one that writes the same option.
fn add_unstable_text_default(arguments: &mut Vec<String>, option: &str, value: &str) {
    let key = format!("-XX:{option}=");

    if arguments.iter().any(|argument| argument.starts_with(&key)) {
        return;
    }

    arguments.push(format!("{key}{value}"));
}

/// Builds the command a version of the game is started with.
///
/// The version must be resolved: the changes of the versions it inherits from
/// and the patches of the repository must be applied. The command holds the
/// arguments the launcher adds and the arguments the version reads, with the
/// variables of the version replaced.
///
/// The version number of the game decides which arguments the game understands:
/// the interface the game draws with and the arguments of the virtual machine
/// the game no longer accepts depend on it.
pub fn build_command(
    manifest: &InstanceManifest,
    auth: &AuthInfo,
    options: &LaunchOptions,
    game_version: &GameVersionNumber,
    paths: &LaunchPaths,
) -> Result<LaunchCommand, LaunchError> {
    let features = options.features();
    let context = rule_context(options, &features);

    let classpath = classpath(
        manifest,
        &paths.libraries_directory,
        &paths.client_jar,
        &context,
    );
    let variables = LaunchVariables::for_instance(manifest, auth, options, paths, &classpath);

    let mut jvm_arguments = Vec::new();

    if let Some(max) = options.max_memory.filter(|max| *max > 0) {
        add_default(&mut jvm_arguments, "-Xmx", &format!("{max}m"));
    }
    if let Some(min) = options
        .min_memory
        .filter(|min| *min > 0 && options.max_memory.is_none_or(|max| *min <= max))
    {
        add_default(&mut jvm_arguments, "-Xms", &format!("{min}m"));
    }
    if let Some(metaspace) = options.metaspace.filter(|metaspace| *metaspace > 0) {
        let flag = if options.java_major_version < 8 {
            "-XX:PermSize="
        } else {
            "-XX:MetaspaceSize="
        };
        add_default(&mut jvm_arguments, flag, &format!("{metaspace}m"));
    }

    jvm_arguments.extend(
        options
            .jvm_arguments
            .iter()
            .map(|argument| variables.substitute(argument)),
    );

    if !options.no_generated_jvm_arguments {
        let encoding = encoding_name();
        add_default(&mut jvm_arguments, "-Dfile.encoding=", encoding);

        if options.java_major_version < 19 {
            add_default(&mut jvm_arguments, "-Dsun.stdout.encoding=", encoding);
            add_default(&mut jvm_arguments, "-Dsun.stderr.encoding=", encoding);
        } else {
            add_default(&mut jvm_arguments, "-Dstdout.encoding=", encoding);
            add_default(&mut jvm_arguments, "-Dstderr.encoding=", encoding);
        }

        add_default(
            &mut jvm_arguments,
            "-Djava.rmi.server.useCodebaseOnly=",
            "true",
        );
        add_default(
            &mut jvm_arguments,
            "-Dcom.sun.jndi.rmi.object.trustURLCodebase=",
            "false",
        );
        add_default(
            &mut jvm_arguments,
            "-Dcom.sun.jndi.cosnaming.object.trustURLCodebase=",
            "false",
        );
        add_default(&mut jvm_arguments, "-Dlog4j2.formatMsgNoLookups=", "true");

        // The agent that gives the game of the newer versions back the access
        // to the memory it uses by itself. The launcher of Java adds it from
        // `appendJvmArgs`, which runs before the arguments of the version.
        if options.allow_auto_agent
            && !options.no_generated_optimizing_jvm_arguments
            && need_patch_memory_util(manifest, options.java_major_version)
        {
            jvm_arguments.push(format!("-javaagent:{}", LWJGL_UNSAFE_AGENT_FILE));
        }

        // The arguments of the virtual machine that make the game faster; the
        // player may turn them off when the game does not start with them.
        if !options.no_generated_optimizing_jvm_arguments {
            let java_version = options.java_major_version;
            let is_64_bit = options.java_bits.unwrap_or(0) == 64;

            add_unstable_default(&mut jvm_arguments, "UnlockExperimentalVMOptions", true);
            add_unstable_default(&mut jvm_arguments, "UnlockDiagnosticVMOptions", true);

            // The game runs with the settings of G1 unless the player chose
            // another garbage collector.
            if java_version >= 8
                && !jvm_arguments.iter().any(|argument| {
                    argument == "-XX:-UseG1GC"
                        || (argument.starts_with("-XX:+Use") && argument.ends_with("GC"))
                })
            {
                add_unstable_default(&mut jvm_arguments, "UseG1GC", true);
                add_unstable_text_default(&mut jvm_arguments, "G1MixedGCCountTarget", "5");
                add_unstable_text_default(&mut jvm_arguments, "G1NewSizePercent", "20");
                add_unstable_text_default(&mut jvm_arguments, "G1ReservePercent", "20");
                add_unstable_text_default(&mut jvm_arguments, "MaxGCPauseMillis", "50");
                add_unstable_text_default(&mut jvm_arguments, "G1HeapRegionSize", "32m");
            }

            add_unstable_default(&mut jvm_arguments, "OmitStackTraceInFastThrow", false);

            if java_version <= 8 {
                add_unstable_text_default(&mut jvm_arguments, "MaxInlineLevel", "15");
            }

            // The arguments of the compiler, which the launcher adds when the
            // machine has the memory to use them.
            if is_64_bit && total_physical_memory().unwrap_or(0) > 4 * 1024 * 1024 * 1024 {
                add_unstable_default(&mut jvm_arguments, "DontCompileHugeMethods", false);
                add_unstable_text_default(&mut jvm_arguments, "MaxNodeLimit", "240000");
                add_unstable_text_default(&mut jvm_arguments, "NodeLimitFudgeFactor", "8000");
                add_unstable_text_default(&mut jvm_arguments, "TieredCompileTaskTimeout", "10000");
                add_unstable_text_default(&mut jvm_arguments, "ReservedCodeCacheSize", "400M");

                if java_version >= 9 {
                    add_unstable_text_default(&mut jvm_arguments, "NonNMethodCodeHeapSize", "12M");
                    add_unstable_text_default(&mut jvm_arguments, "ProfiledCodeHeapSize", "194M");
                }

                if java_version >= 8 {
                    add_unstable_text_default(&mut jvm_arguments, "NmethodSweepActivity", "1");
                }
            }

            if is_64_bit && (25..=26).contains(&java_version) {
                add_unstable_default(&mut jvm_arguments, "UseCompactObjectHeaders", true);
            }

            // A 32-bit runtime gives the stack 320 KiB rather than 1 MiB, and
            // the game of the older versions overflows it.
            if !is_64_bit {
                add_default(&mut jvm_arguments, "-Xss", "1m");
            }
        }

        if options.java_major_version == 16 {
            add_default(&mut jvm_arguments, "--illegal-access=", "permit");
        }

        if matches!(options.java_major_version, 24 | 25) {
            add_default(
                &mut jvm_arguments,
                "--sun-misc-unsafe-memory-access=",
                "allow",
            );
        }

        add_default(
            &mut jvm_arguments,
            "-Dfml.ignoreInvalidMinecraftCertificates=",
            "true",
        );
        add_default(
            &mut jvm_arguments,
            "-Dfml.ignorePatchDiscrepancies=",
            "true",
        );
    }

    match manifest
        .arguments
        .as_ref()
        .and_then(|arguments| arguments.jvm.as_deref())
    {
        Some(arguments) => {
            jvm_arguments.extend(resolve_arguments(arguments, &variables, &context));
        }
        None if !options.no_generated_jvm_arguments => {
            jvm_arguments.extend(resolve_arguments(
                &default_jvm_arguments(),
                &variables,
                &context,
            ));
        }
        None => {}
    }

    // The arguments of the account, such as the agent of the authlib-injector,
    // which the game needs to talk to the server of the player. The launcher of
    // Java adds them after the arguments of the version and before the agents
    // of the player.
    if let Some(arguments) = &auth.launch_arguments {
        jvm_arguments.extend(
            arguments
                .iter()
                .map(|argument| variables.substitute(argument)),
        );
    }

    for agent in &options.java_agents {
        jvm_arguments.push(format!("-javaagent:{agent}"));
    }

    let main_class = manifest
        .main_class
        .clone()
        .filter(|main_class| !main_class.is_empty())
        .ok_or(LaunchError::MissingMainClass)?;

    let mut game_arguments = Vec::new();
    let legacy_arguments = manifest
        .minecraft_arguments
        .as_deref()
        .filter(|arguments| !arguments.is_empty());

    if let Some(arguments) = legacy_arguments {
        game_arguments.extend(split_arguments(arguments, &variables));
    }
    if let Some(arguments) = manifest
        .arguments
        .as_ref()
        .and_then(|arguments| arguments.game.as_deref())
    {
        game_arguments.extend(resolve_arguments(arguments, &variables, &context));
    }
    if legacy_arguments.is_some() && !options.no_generated_game_arguments {
        game_arguments.extend(resolve_arguments(
            &default_game_arguments(),
            &variables,
            &context,
        ));
    }
    if let Some(quick_play) = &options.quick_play {
        game_arguments.push(quick_play.argument());
    }

    if options.fullscreen {
        game_arguments.push("--fullscreen".to_string());
    }

    // The game reads the interface it draws with from `--graphicsBackend` since
    // `26.2-snapshot-2`; the versions before it draw with OpenGL only.
    if options.graphics_api != GraphicsApi::Default
        && game_version >= &GameVersionNumber::as_game_version(GRAPHICS_BACKEND_VERSION)
    {
        game_arguments.push("--graphicsBackend".to_string());
        game_arguments.push(options.graphics_api.minecraft_argument().to_string());
    }

    game_arguments.extend(
        options
            .game_arguments
            .iter()
            .map(|argument| variables.substitute(argument)),
    );

    // The virtual machine of Java 9 and later refuses `-Xincgc`, which the
    // players of the older versions wrote into their arguments. The launcher of
    // Java removes it from the whole command, wherever the player wrote it.
    if options.java_major_version >= 9 {
        jvm_arguments.retain(|argument| argument != "-Xincgc");
        game_arguments.retain(|argument| argument != "-Xincgc");
    }

    Ok(LaunchCommand {
        classpath,
        main_class,
        jvm_arguments,
        game_arguments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::Arguments;
    use crate::util::operating_system::Arch;

    fn paths() -> LaunchPaths {
        LaunchPaths {
            game_directory: PathBuf::from("game"),
            assets_directory: PathBuf::from("assets"),
            asset_index: "5".to_string(),
            natives_directory: PathBuf::from("natives"),
            libraries_directory: PathBuf::from("libraries"),
            client_jar: PathBuf::from("client.jar"),
        }
    }

    fn options() -> LaunchOptions {
        LaunchOptions {
            os: OsType::Windows,
            arch: Arch::X86_64,
            max_memory: Some(4096),
            min_memory: Some(512),
            ..LaunchOptions::default()
        }
    }

    /// Returns a version of the game that understands every argument.
    fn game_version() -> GameVersionNumber {
        GameVersionNumber::as_game_version("1.20.1")
    }

    fn legacy_manifest() -> InstanceManifest {
        InstanceManifest {
            id: Some("1.7.10".to_string()),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            minecraft_arguments: Some(
                "--username ${auth_player_name} --version ${version_name}".to_string(),
            ),
            ..InstanceManifest::default()
        }
    }

    #[test]
    fn the_command_of_an_old_version_holds_its_arguments() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &options(),
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert_eq!(command.main_class, "net.minecraft.client.main.Main");
        assert!(command.jvm_arguments.contains(&"-Xmx4096m".to_string()));
        assert!(command.jvm_arguments.contains(&"-Xms512m".to_string()));
        assert!(command
            .jvm_arguments
            .contains(&"-Djava.library.path=natives".to_string()));
        assert!(command.jvm_arguments.contains(&"-cp".to_string()));
        assert!(command.classpath.contains(&PathBuf::from("client.jar")));

        assert!(command
            .game_arguments
            .windows(2)
            .any(|pair| pair == ["--username", "Steve"]));
        assert!(command
            .game_arguments
            .windows(2)
            .any(|pair| pair == ["--version", "1.7.10"]));
        assert!(command.game_arguments.contains(&"--width".to_string()));

        let arguments = command.arguments();
        assert!(arguments.contains(&"net.minecraft.client.main.Main".to_string()));
    }

    #[test]
    fn the_command_of_a_modern_version_holds_its_arguments() {
        let manifest = InstanceManifest {
            id: Some("1.20.1".to_string()),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            arguments: Some(Arguments {
                game: Some(vec![
                    crate::download::game::Argument::Text("--username".to_string()),
                    crate::download::game::Argument::Text("${auth_player_name}".to_string()),
                    crate::download::game::Argument::Text("--demo".to_string()),
                ]),
                jvm: Some(vec![crate::download::game::Argument::Text(
                    "-cp".to_string(),
                )]),
            }),
            ..InstanceManifest::default()
        };

        let command = build_command(
            &manifest,
            &AuthInfo::offline("Steve"),
            &options(),
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert_eq!(
            command.game_arguments,
            vec!["--username", "Steve", "--demo"]
        );

        // The launcher adds the arguments of the virtual machine that make the
        // game faster before the arguments the version reads.
        assert!(command
            .jvm_arguments
            .starts_with(&["-Xmx4096m".to_string(), "-Xms512m".to_string()]));
        assert!(command.jvm_arguments.contains(&"-cp".to_string()));
    }

    #[test]
    fn the_optimizing_arguments_are_added_by_default() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &options(),
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command
            .jvm_arguments
            .contains(&"-XX:+UnlockExperimentalVMOptions".to_string()));
        assert!(command
            .jvm_arguments
            .contains(&"-XX:+UnlockDiagnosticVMOptions".to_string()));
        assert!(command.jvm_arguments.contains(&"-XX:+UseG1GC".to_string()));
        assert!(command
            .jvm_arguments
            .contains(&"-XX:-OmitStackTraceInFastThrow".to_string()));
        assert!(command
            .jvm_arguments
            .contains(&"-Dfml.ignoreInvalidMinecraftCertificates=true".to_string()));
    }

    #[test]
    fn the_player_may_turn_off_the_optimizing_arguments() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                no_generated_optimizing_jvm_arguments: true,
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command.jvm_arguments.contains(&"-cp".to_string()));
        assert!(!command.jvm_arguments.contains(&"-XX:+UseG1GC".to_string()));
        assert!(!command
            .jvm_arguments
            .contains(&"-XX:+UnlockExperimentalVMOptions".to_string()));
        assert!(!command
            .jvm_arguments
            .contains(&"-XX:-OmitStackTraceInFastThrow".to_string()));
        assert!(command
            .jvm_arguments
            .contains(&"-Dfile.encoding=UTF-8".to_string()));
    }

    #[test]
    fn a_player_garbage_collector_keeps_the_launcher_from_adding_g1() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                jvm_arguments: vec!["-XX:+UseZGC".to_string()],
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command.jvm_arguments.contains(&"-XX:+UseZGC".to_string()));
        assert!(!command.jvm_arguments.contains(&"-XX:+UseG1GC".to_string()));
    }

    #[test]
    fn a_thirty_two_bit_runtime_gets_a_larger_stack() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                java_bits: Some(32),
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command.jvm_arguments.contains(&"-Xss1m".to_string()));
        // The arguments of the compiler need the address space of a 64-bit
        // runtime.
        assert!(!command
            .jvm_arguments
            .contains(&"-XX:ReservedCodeCacheSize=400M".to_string()));
    }

    #[test]
    fn the_agent_is_added_when_the_version_needs_it() {
        let manifest = InstanceManifest {
            id: Some("1.21.11".to_string()),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            libraries: vec![crate::download::game::Library {
                name: "org.lwjgl:lwjgl:3.4.1".to_string(),
                ..Default::default()
            }],
            ..InstanceManifest::default()
        };

        let options = LaunchOptions {
            java_major_version: 25,
            allow_auto_agent: true,
            ..options()
        };

        let command = build_command(
            &manifest,
            &AuthInfo::offline("Steve"),
            &options,
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command
            .jvm_arguments
            .contains(&"-javaagent:lwjgl-unsafe-agent.jar".to_string()));
    }

    #[test]
    fn the_agent_is_not_added_when_the_version_does_not_need_it() {
        let manifest = InstanceManifest {
            id: Some("1.20.1".to_string()),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            libraries: vec![crate::download::game::Library {
                name: "org.lwjgl:lwjgl:3.3.2".to_string(),
                ..Default::default()
            }],
            ..InstanceManifest::default()
        };

        let command = build_command(
            &manifest,
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                java_major_version: 25,
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(!command
            .jvm_arguments
            .iter()
            .any(|argument| argument.starts_with("-javaagent:")));
    }

    #[test]
    fn the_agent_is_not_added_when_the_optimizing_arguments_are_off() {
        let manifest = InstanceManifest {
            id: Some("1.21.11".to_string()),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            libraries: vec![crate::download::game::Library {
                name: "org.lwjgl:lwjgl:3.4.1".to_string(),
                ..Default::default()
            }],
            ..InstanceManifest::default()
        };

        let command = build_command(
            &manifest,
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                java_major_version: 25,
                no_generated_optimizing_jvm_arguments: true,
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(!command
            .jvm_arguments
            .iter()
            .any(|argument| argument.starts_with("-javaagent:")));
    }

    #[test]
    fn the_player_may_turn_off_the_arguments_of_the_launcher() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                no_generated_jvm_arguments: true,
                no_generated_game_arguments: true,
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(!command.jvm_arguments.contains(&"-cp".to_string()));
        assert!(command.classpath.contains(&PathBuf::from("client.jar")));
        assert!(!command.game_arguments.contains(&"--width".to_string()));
    }

    #[test]
    fn the_player_may_add_arguments() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                jvm_arguments: vec!["-Duser.name=${auth_player_name}".to_string()],
                game_arguments: vec!["--fullscreen".to_string()],
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command
            .jvm_arguments
            .contains(&"-Duser.name=Steve".to_string()));
        assert!(command.game_arguments.contains(&"--fullscreen".to_string()));
    }

    #[test]
    fn a_version_without_a_main_class_is_refused() {
        let manifest = InstanceManifest {
            id: Some("broken".to_string()),
            ..InstanceManifest::default()
        };

        assert_eq!(
            build_command(
                &manifest,
                &AuthInfo::offline("Steve"),
                &options(),
                &game_version(),
                &paths()
            ),
            Err(LaunchError::MissingMainClass)
        );
    }

    #[test]
    fn the_fullscreen_argument_is_added() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                fullscreen: true,
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command.game_arguments.contains(&"--fullscreen".to_string()));
    }

    #[test]
    fn the_quick_play_option_is_added_to_the_game_arguments() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                quick_play: Some(crate::game::quick_play::QuickPlayOption::MultiPlayer(
                    "example.org".to_string(),
                )),
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command
            .game_arguments
            .contains(&"quickPlayMultiplayer=example.org".to_string()));
    }

    #[test]
    fn the_graphics_backend_is_added_when_the_game_reads_it() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                graphics_api: GraphicsApi::Vulkan,
                ..options()
            },
            &GameVersionNumber::as_game_version("26.2-snapshot-2"),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command
            .game_arguments
            .windows(2)
            .any(|pair| pair == ["--graphicsBackend", "vulkan"]));
    }

    #[test]
    fn the_graphics_backend_is_omitted_for_a_game_that_does_not_read_it() {
        // The versions before `26.2-snapshot-2` draw with OpenGL only, so the
        // argument would be unknown to them.
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                graphics_api: GraphicsApi::Vulkan,
                ..options()
            },
            &GameVersionNumber::as_game_version("26.2-snapshot-1"),
            &paths(),
        )
        .expect("the command should be built");

        assert!(!command
            .game_arguments
            .contains(&"--graphicsBackend".to_string()));
    }

    #[test]
    fn the_graphics_backend_is_omitted_when_it_is_the_default() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &options(),
            &GameVersionNumber::as_game_version("26.2"),
            &paths(),
        )
        .expect("the command should be built");

        assert!(!command
            .game_arguments
            .contains(&"--graphicsBackend".to_string()));
    }

    #[test]
    fn the_arguments_of_the_account_are_added_after_the_arguments_of_the_version() {
        let manifest = InstanceManifest {
            id: Some("1.20.1".to_string()),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            arguments: Some(Arguments {
                game: None,
                jvm: Some(vec![crate::download::game::Argument::Text(
                    "-Dfrom.manifest=true".to_string(),
                )]),
            }),
            ..InstanceManifest::default()
        };

        let auth = AuthInfo::offline("Steve").with_launch_arguments(vec![
            "-javaagent:/dependencies/authlib-injector.jar=https://example.org/".to_string(),
            "-Dauthlibinjector.side=client".to_string(),
        ]);

        let command = build_command(&manifest, &auth, &options(), &game_version(), &paths())
            .expect("the command should be built");

        let manifest_position = command
            .jvm_arguments
            .iter()
            .position(|argument| argument == "-Dfrom.manifest=true")
            .expect("the argument of the version should be there");
        let agent_position = command
            .jvm_arguments
            .iter()
            .position(|argument| argument.starts_with("-javaagent:"))
            .expect("the agent of the account should be there");

        assert!(manifest_position < agent_position);
        assert!(command
            .jvm_arguments
            .contains(&"-Dauthlibinjector.side=client".to_string()));
    }

    #[test]
    fn the_agents_of_the_options_are_added_before_the_main_class() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                java_agents: vec!["/dependencies/agent.jar".to_string()],
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command
            .jvm_arguments
            .contains(&"-javaagent:/dependencies/agent.jar".to_string()));

        // The agents are arguments of the virtual machine, so they are written
        // before the class the game is started with.
        let arguments = command.arguments();
        let agent = arguments
            .iter()
            .position(|argument| argument == "-javaagent:/dependencies/agent.jar")
            .expect("the agent should be there");
        let main_class = arguments
            .iter()
            .position(|argument| argument == "net.minecraft.client.main.Main")
            .expect("the main class should be there");
        assert!(agent < main_class);
    }

    #[test]
    fn the_garbage_collector_of_java_8_is_removed_for_a_newer_runtime() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                game_arguments: vec!["-Xincgc".to_string()],
                java_major_version: 17,
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(!command.game_arguments.contains(&"-Xincgc".to_string()));
    }

    #[test]
    fn the_garbage_collector_of_java_8_is_kept_for_java_8() {
        let command = build_command(
            &legacy_manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions {
                game_arguments: vec!["-Xincgc".to_string()],
                java_major_version: 8,
                ..options()
            },
            &game_version(),
            &paths(),
        )
        .expect("the command should be built");

        assert!(command.game_arguments.contains(&"-Xincgc".to_string()));
    }
}
