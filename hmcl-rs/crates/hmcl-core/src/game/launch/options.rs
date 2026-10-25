//! The options a version of the game is started with.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::game::graphics::{GraphicsApi, Renderer};
use crate::game::process_priority::ProcessPriority;
use crate::game::quick_play::QuickPlayOption;
use crate::java::Platform;
use crate::util::operating_system::{current_arch, current_os, Arch, OsType};

/// The options of a command that starts a version of the game.
///
/// The options hold what the player set in the launcher and what the launcher
/// knows about the machine; the options of the version itself are read from
/// the manifest of the version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchOptions {
    /// The path of the program that runs the game.
    pub java: PathBuf,
    /// The arguments of the virtual machine the player set.
    pub jvm_arguments: Vec<String>,
    /// The arguments of the game the player set.
    pub game_arguments: Vec<String>,
    /// The largest amount of the memory the game may use, in mebibytes.
    pub max_memory: Option<u32>,
    /// The smallest amount of the memory the game keeps, in mebibytes.
    pub min_memory: Option<u32>,
    /// The amount of the memory the metaspace of the game may use, in
    /// mebibytes.
    pub metaspace: Option<u32>,
    /// The major version of Java the game runs with.
    ///
    /// The version tells how the memory of the metaspace and the encoding of
    /// the output of the game are set.
    pub java_major_version: u32,
    /// The width of the window of the game, in pixels.
    pub width: u32,
    /// The height of the window of the game, in pixels.
    pub height: u32,
    /// Whether the game starts in the full screen.
    pub fullscreen: bool,
    /// Whether the player plays the demo of the game.
    pub demo: bool,
    /// The name of the version, if it differs from the identifier of the
    /// version.
    pub version_name: Option<String>,
    /// The name of the profile of the launcher.
    pub profile_name: Option<String>,
    /// The kind of the version, if it differs from the kind of the manifest.
    pub version_type: Option<String>,
    /// The directory the game runs in, if it differs from the game directory.
    pub run_directory: Option<PathBuf>,
    /// The environment variables of the game.
    pub environment_variables: BTreeMap<String, String>,
    /// The program that starts the game, if the user chose one.
    pub wrapper: Option<String>,
    /// The command the launcher runs before it starts the game.
    pub pre_launch_command: Option<String>,
    /// The command the launcher runs after the game closes.
    pub post_exit_command: Option<String>,
    /// The priority of the process of the game.
    pub process_priority: ProcessPriority,
    /// The graphics interface the game draws with.
    pub graphics_api: GraphicsApi,
    /// The driver the game draws with.
    pub renderer: Renderer,
    /// Whether the launcher leaves the native libraries to the user.
    pub use_custom_natives: bool,
    /// The directory that holds the native libraries the user chose.
    pub natives_directory: Option<PathBuf>,
    /// Whether the launcher does not patch the native libraries of the game.
    pub not_patch_natives: bool,
    /// Whether the game writes the log of the debug level.
    pub enable_debug_log_output: bool,
    /// Whether the launcher may inject its agent into the game.
    pub allow_auto_agent: bool,
    /// The agents the virtual machine is started with.
    ///
    /// The launcher adds `-javaagent:` for every entry, in the order the
    /// entries are given. An account of a server of the authlib-injector names
    /// its agent here, which is what makes the game talk to that server.
    pub java_agents: Vec<String>,
    /// Whether the launcher does not write the options of the game by itself.
    pub disable_auto_game_options: bool,
    /// Whether the game uses the native GLFW or SDL library.
    pub use_native_glfw_or_sdl: bool,
    /// Whether the game uses the native OpenAL library.
    pub use_native_openal: bool,
    /// Whether the game asks for the high performance graphics card.
    pub high_performance: bool,
    /// Whether the launcher stays alive while the game runs.
    pub daemon: bool,
    /// The place the game opens when it starts.
    pub quick_play: Option<QuickPlayOption>,
    /// Whether the launcher does not add its own arguments of the virtual
    /// machine.
    pub no_generated_jvm_arguments: bool,
    /// Whether the launcher does not add the arguments that make the virtual
    /// machine faster.
    pub no_generated_optimizing_jvm_arguments: bool,
    /// Whether the launcher does not add its own arguments of the game.
    pub no_generated_game_arguments: bool,
    /// The amount of the bits the addresses of the Java runtime have.
    ///
    /// The launcher adds the arguments that make a 64-bit runtime faster and
    /// the arguments that keep a 32-bit runtime from running out of stack. The
    /// amount is not known when the architecture of the runtime is not known.
    pub java_bits: Option<u32>,
    /// The operating system and the architecture of the Java runtime the game
    /// runs on.
    ///
    /// The launcher replaces the native libraries of the version with the ones
    /// the players built for the platform of the runtime, so a runtime of
    /// another architecture than the machine reads the libraries of its own
    /// architecture.
    pub java_platform: Platform,
    /// The name of the launcher, written into the arguments of the game.
    pub launcher_name: String,
    /// The version of the launcher, written into the arguments of the game.
    pub launcher_version: String,
    /// The language of the launcher, written into the arguments of the game.
    pub language: String,
    /// The operating system the game is started on.
    pub os: OsType,
    /// The version of the operating system, if it is known.
    pub os_version: Option<String>,
    /// The architecture of the machine the game is started on.
    pub arch: Arch,
    /// Whether the files of the version on the machine of the player are
    /// checked before the game is started.
    ///
    /// The launcher reads the digests of the libraries and opens the archives,
    /// so a library the player broke is downloaded again. The check costs the
    /// time of reading the files, so the launcher makes it only after the game
    /// stopped in a way that may have broken them.
    pub integrity_check: bool,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            java: PathBuf::from("java"),
            jvm_arguments: Vec::new(),
            game_arguments: Vec::new(),
            max_memory: Some(2048),
            min_memory: None,
            metaspace: None,
            java_major_version: 17,
            width: 854,
            height: 480,
            fullscreen: false,
            demo: false,
            version_name: None,
            profile_name: None,
            version_type: None,
            run_directory: None,
            environment_variables: BTreeMap::new(),
            wrapper: None,
            pre_launch_command: None,
            post_exit_command: None,
            process_priority: ProcessPriority::Normal,
            graphics_api: GraphicsApi::Default,
            renderer: Renderer::DEFAULT,
            use_custom_natives: false,
            natives_directory: None,
            not_patch_natives: false,
            enable_debug_log_output: false,
            allow_auto_agent: false,
            java_agents: Vec::new(),
            disable_auto_game_options: false,
            use_native_glfw_or_sdl: false,
            use_native_openal: false,
            high_performance: false,
            daemon: false,
            quick_play: None,
            no_generated_jvm_arguments: false,
            no_generated_optimizing_jvm_arguments: false,
            no_generated_game_arguments: false,
            java_bits: Some(64),
            java_platform: Platform::current(),
            launcher_name: "HMCL".to_string(),
            launcher_version: env!("CARGO_PKG_VERSION").to_string(),
            language: "en-US".to_string(),
            os: current_os(),
            os_version: None,
            arch: current_arch(),
            integrity_check: false,
        }
    }
}

impl LaunchOptions {
    /// Returns the features the rules of the version are read with.
    ///
    /// The game is started with the resolution of the window when the player
    /// set one, and as the demo when the player plays the demo.
    pub fn features(&self) -> BTreeMap<String, bool> {
        let mut features = BTreeMap::new();
        features.insert(
            "has_custom_resolution".to_string(),
            self.width > 0 && self.height > 0,
        );
        features.insert("is_demo_user".to_string(), self.demo);
        features
    }

    /// Returns the directory the game runs in.
    pub fn working_directory<'a>(&'a self, paths: &'a LaunchPaths) -> &'a Path {
        match &self.run_directory {
            Some(directory) => directory,
            None => &paths.game_directory,
        }
    }
}

/// The files and the directories the command of a version is built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPaths {
    /// The directory the game stores its worlds and its settings in.
    pub game_directory: PathBuf,
    /// The directory the assets of the game are stored in.
    pub assets_directory: PathBuf,
    /// The identifier of the index of the assets the version reads.
    pub asset_index: String,
    /// The directory the native libraries of the game are extracted to.
    pub natives_directory: PathBuf,
    /// The directory the libraries of the game are stored in.
    pub libraries_directory: PathBuf,
    /// The file the version of the game itself is stored in.
    pub client_jar: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_options_start_a_released_version() {
        let options = LaunchOptions::default();

        assert_eq!(options.max_memory, Some(2048));
        assert_eq!(options.launcher_name, "HMCL");
        assert!(!options.fullscreen);
        assert!(!options.demo);
    }

    #[test]
    fn the_features_tell_the_resolution_and_the_demo() {
        let options = LaunchOptions::default();
        let features = options.features();

        assert_eq!(features.get("has_custom_resolution"), Some(&true));
        assert_eq!(features.get("is_demo_user"), Some(&false));

        let options = LaunchOptions {
            width: 0,
            height: 0,
            demo: true,
            ..LaunchOptions::default()
        };
        let features = options.features();

        assert_eq!(features.get("has_custom_resolution"), Some(&false));
        assert_eq!(features.get("is_demo_user"), Some(&true));
    }

    #[test]
    fn the_game_runs_in_the_run_directory() {
        let options = LaunchOptions::default();
        let paths = LaunchPaths {
            game_directory: PathBuf::from("game"),
            assets_directory: PathBuf::from("assets"),
            asset_index: "1.20".to_string(),
            natives_directory: PathBuf::from("natives"),
            libraries_directory: PathBuf::from("libraries"),
            client_jar: PathBuf::from("client.jar"),
        };

        assert_eq!(options.working_directory(&paths), Path::new("game"));

        let options = LaunchOptions {
            run_directory: Some(PathBuf::from("run")),
            ..LaunchOptions::default()
        };
        assert_eq!(options.working_directory(&paths), Path::new("run"));
    }
}
