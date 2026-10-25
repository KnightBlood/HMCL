//! Applying the game settings to the options of a start.
//!
//! The settings of a preset and an instance are resolved into concrete values.
//! These values are turned into the options a version of the game is started
//! with: the arguments of the virtual machine, the amount of memory, the size
//! of the window, the graphics interface and more.

use std::path::PathBuf;

use crate::java::JavaRuntime;
use crate::setting::{Effective, GameWindowType};
use crate::util::operating_system::{available_physical_memory, current_arch};
use crate::util::versioning::GameVersionNumber;

use super::arguments::tokenize;
use super::options::LaunchOptions;

/// The memory the launcher keeps for itself and for off-heap memory, in bytes.
const RESERVED_MEMORY: u64 = 512 * 1024 * 1024;

/// The largest amount of memory a 32-bit Java runtime may use, in bytes.
const MAX_32_BIT_MEMORY: u64 = 768 * 1024 * 1024;

/// Returns the amount of memory to give the game when the launcher chooses it.
///
/// The launcher keeps `512 MiB` for itself and for off-heap memory. When the
/// machine has at most `8 GiB` of memory, the game gets `80 %` of the rest;
/// otherwise it gets `6.4 GiB` plus `20 %` of the rest, capped at `16 GiB`. A
/// 32-bit Java runtime gets at most `768 MiB`.
pub fn auto_allocated_memory(available: u64, is_64_bit: bool) -> u64 {
    let usable = available.saturating_sub(RESERVED_MEMORY);

    let suggested = if usable == 0 {
        available
    } else if usable <= 8 * 1024 * 1024 * 1024 {
        (usable as f64 * 0.8) as u64
    } else {
        let extra = ((usable - 8 * 1024 * 1024 * 1024) as f64) * 0.2;
        ((8_u64 * 1024 * 1024 * 1024) as f64 * 0.8 + extra)
            .min((16_u64 * 1024 * 1024 * 1024) as f64) as u64
    };

    if is_64_bit {
        suggested
    } else {
        suggested.min(MAX_32_BIT_MEMORY)
    }
}

/// Applies the settings of a preset and an instance to the options of a start.
///
/// `java` is the runtime the settings chose, if there is one. When it is
/// `None`, the options keep their own Java executable, major version and
/// platform.
pub fn apply_settings(
    effective: &Effective<'_>,
    game_version: &GameVersionNumber,
    java: Option<&JavaRuntime>,
    options: &mut LaunchOptions,
) {
    options.jvm_arguments = tokenize(&effective.jvm_options());
    options.game_arguments = tokenize(&effective.game_arguments());
    options.no_generated_jvm_arguments = effective.no_jvm_options();
    options.no_generated_optimizing_jvm_arguments = effective.no_optimizing_jvm_options();
    options.no_generated_game_arguments = effective.disable_auto_game_options();

    if effective.auto_memory() {
        let available = available_physical_memory().unwrap_or(0);
        options.max_memory = Some(
            (auto_allocated_memory(available, current_arch().is_64bit()) / (1024 * 1024)) as u32,
        );
    } else {
        options.max_memory = Some(effective.max_memory());
    }
    options.min_memory = effective.min_memory();
    options.metaspace = effective.perm_size().trim().parse::<u32>().ok();

    options.width = effective.width();
    options.height = effective.height();
    options.fullscreen = matches!(effective.window_type(), GameWindowType::Fullscreen);

    if !effective.command_wrapper().trim().is_empty() {
        options.wrapper = Some(effective.command_wrapper());
    }
    if !effective.pre_launch_command().trim().is_empty() {
        options.pre_launch_command = Some(effective.pre_launch_command());
    }
    if !effective.post_exit_command().trim().is_empty() {
        options.post_exit_command = Some(effective.post_exit_command());
    }

    options.process_priority = effective.process_priority();
    options.graphics_api = effective.graphics_backend();
    options.renderer = effective.renderer(game_version);

    options.use_custom_natives = effective.use_custom_natives();
    options.natives_directory = (!effective.natives_directory().trim().is_empty())
        .then_some(PathBuf::from(effective.natives_directory()));
    options.not_patch_natives = effective.not_patch_natives();

    options.enable_debug_log_output = effective.enable_debug_log_output();
    options.allow_auto_agent = effective.allow_auto_agent();
    options.use_native_glfw_or_sdl = effective.use_native_glfw_or_sdl();
    options.use_native_openal = effective.use_native_openal();
    options.high_performance = effective.high_performance();
    options.daemon = effective.launcher_visibility().is_daemon();

    let mut environment = std::collections::BTreeMap::new();
    for entry in tokenize(&effective.environment_variables()) {
        if let Some((key, value)) = entry.split_once('=') {
            environment.insert(key.to_string(), value.to_string());
        }
    }
    options.environment_variables = environment;

    if let Some(java) = java {
        options.java = java.binary().to_path_buf();
        options.java_major_version = java.parsed_version() as u32;
        // The launcher reads the amount of the bits of the addresses of the
        // runtime from the platform it was built for; a runtime the launcher
        // cannot read the platform of gets no optimizing arguments.
        options.java_bits = java.platform().bits();
        options.java_platform = java.platform();
    }

    options.quick_play = effective.quick_play_option();
}

/// Returns the Java runtime the settings choose for a start.
///
/// The runtime the settings choose is the one the launcher checks against the
/// version of the game. When the player turned the check off, the launcher
/// needs a runtime whether or not it fits the version of the game, so the
/// runtime the player starts the other Java programs with stands in for the
/// runtime the settings could not choose. A machine without such a runtime
/// leaves the options as they are.
pub fn find_launch_java(
    effective: &Effective<'_>,
    runtimes: &[JavaRuntime],
    game_version: &GameVersionNumber,
    manifest: Option<&crate::download::game::InstanceManifest>,
) -> Option<JavaRuntime> {
    let chosen = effective.find_java(runtimes, game_version, manifest);
    if chosen.is_some() || !effective.not_check_jvm() {
        return chosen;
    }

    crate::java::default_java_runtime()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::graphics::{GraphicsApi, Renderer};
    use crate::game::process_priority::ProcessPriority;
    use crate::game::quick_play::{QuickPlayOption, QuickPlayType};
    use crate::setting::{GameSettingsPresetId, InstanceSettings, LauncherVisibility, Preset};
    use crate::util::versioning::GameVersionNumber;

    fn preset() -> Preset {
        Preset::new(
            GameSettingsPresetId::parse(
                "game-settings-preset:123e4567-e89b-12d3-a456-426614174000",
            )
            .expect("the identifier should be read"),
        )
    }

    fn runtime(version: &str) -> JavaRuntime {
        let info =
            crate::java::JavaInfo::new(crate::java::Platform::current(), version.to_string(), None);
        JavaRuntime::from_binary(PathBuf::from(format!("java-{version}")), info, false)
    }

    fn apply(preset: &Preset, instance: Option<&InstanceSettings>) -> LaunchOptions {
        let effective = preset.resolve(instance);
        let mut options = LaunchOptions::default();
        apply_settings(
            &effective,
            &GameVersionNumber::as_game_version("1.20.1"),
            None,
            &mut options,
        );
        options
    }

    #[test]
    fn the_default_settings_keep_the_default_options() {
        let preset = preset();
        let options = apply(&preset, None);

        let available = crate::util::operating_system::available_physical_memory().unwrap_or(0);
        let expected =
            (auto_allocated_memory(available, current_arch().is_64bit()) / (1024 * 1024)) as u32;

        assert_eq!(options.max_memory, Some(expected));
        assert_eq!(options.min_memory, None);
        assert_eq!(options.width, 854);
        assert_eq!(options.height, 480);
        assert!(!options.fullscreen);
        assert!(options.jvm_arguments.is_empty());
        assert!(options.game_arguments.is_empty());
        assert!(options.wrapper.is_none());
        assert!(options.daemon);
    }

    #[test]
    fn the_memory_is_taken_from_the_settings() {
        let mut preset = preset();
        preset.settings.auto_memory = Some(false);
        preset.settings.max_memory = Some(4096);
        preset.settings.min_memory = Some(1024);
        preset.settings.perm_size = Some("256".to_string());

        let options = apply(&preset, None);

        assert_eq!(options.max_memory, Some(4096));
        assert_eq!(options.min_memory, Some(1024));
        assert_eq!(options.metaspace, Some(256));
    }

    #[test]
    fn a_non_numeric_metaspace_is_ignored() {
        let mut preset = preset();
        preset.settings.perm_size = Some("abc".to_string());

        assert_eq!(apply(&preset, None).metaspace, None);
    }

    #[test]
    fn auto_memory_uses_the_auto_allocated_memory() {
        let mut preset = preset();
        preset.settings.auto_memory = Some(true);

        let available = crate::util::operating_system::available_physical_memory().unwrap_or(0);
        let expected = (auto_allocated_memory(available, true) / (1024 * 1024)) as u32;

        assert_eq!(apply(&preset, None).max_memory, Some(expected));
    }

    #[test]
    fn the_window_is_set_from_the_settings() {
        let mut preset = preset();
        preset.settings.width = Some(1920.0);
        preset.settings.height = Some(1080.0);
        preset.settings.window_type = Some(GameWindowType::Fullscreen);

        let options = apply(&preset, None);

        assert_eq!(options.width, 1920);
        assert_eq!(options.height, 1080);
        assert!(options.fullscreen);
    }

    #[test]
    fn the_arguments_are_tokenized() {
        let mut preset = preset();
        preset.settings.jvm_options = Some("-Xmx2G -XX:+UseG1GC".to_string());
        preset.settings.game_arguments = Some("--demo".to_string());

        let options = apply(&preset, None);

        assert_eq!(
            options.jvm_arguments,
            vec!["-Xmx2G".to_string(), "-XX:+UseG1GC".to_string()]
        );
        assert_eq!(options.game_arguments, vec!["--demo".to_string()]);
    }

    #[test]
    fn the_environment_variables_are_split_on_equals() {
        let mut preset = preset();
        preset.settings.environment_variables = Some("FOO=bar\nBAZ=qux".to_string());

        let options = apply(&preset, None);

        assert_eq!(
            options.environment_variables,
            [
                ("BAZ".to_string(), "qux".to_string()),
                ("FOO".to_string(), "bar".to_string()),
            ]
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>()
        );
    }

    #[test]
    fn the_wrapper_and_the_commands_are_copied() {
        let mut preset = preset();
        preset.settings.command_wrapper = Some("wrapper".to_string());
        preset.settings.pre_launch_command = Some("before".to_string());
        preset.settings.post_exit_command = Some("after".to_string());

        let options = apply(&preset, None);

        assert_eq!(options.wrapper.as_deref(), Some("wrapper"));
        assert_eq!(options.pre_launch_command.as_deref(), Some("before"));
        assert_eq!(options.post_exit_command.as_deref(), Some("after"));
    }

    #[test]
    fn blank_wrapper_and_commands_are_left_empty() {
        let mut preset = preset();
        preset.settings.command_wrapper = Some("   ".to_string());

        assert!(apply(&preset, None).wrapper.is_none());
    }

    #[test]
    fn the_priority_and_the_graphics_are_copied() {
        let mut preset = preset();
        preset.settings.process_priority = Some(ProcessPriority::High);
        preset.settings.graphics_backend = Some(GraphicsApi::Vulkan);
        preset.settings.open_gl_renderer = Some(Renderer::Llvmpipe);
        preset.settings.vulkan_renderer = Some(Renderer::Lavapipe);
        preset.settings.use_custom_natives = Some(true);
        preset.settings.natives_directory = Some("natives".to_string());
        preset.settings.not_patch_natives = Some(true);

        let version = GameVersionNumber::as_game_version("26.2-snapshot-5");
        let effective = preset.resolve(None);
        let mut options = LaunchOptions::default();
        apply_settings(&effective, &version, None, &mut options);

        assert_eq!(options.process_priority, ProcessPriority::High);
        assert_eq!(options.graphics_api, GraphicsApi::Vulkan);
        assert_eq!(options.renderer, Renderer::Lavapipe);
        assert!(options.use_custom_natives);
        assert_eq!(
            options.natives_directory.as_deref(),
            Some(Path::new("natives"))
        );
        assert!(options.not_patch_natives);
    }

    #[test]
    fn the_renderer_falls_back_when_the_interface_changes() {
        let mut preset = preset();
        preset.settings.graphics_backend = Some(GraphicsApi::OpenGl);
        preset.settings.vulkan_renderer = Some(Renderer::Lavapipe);

        let version = GameVersionNumber::as_game_version("26.2-snapshot-5");
        let effective = preset.resolve(None);
        let mut options = LaunchOptions::default();
        apply_settings(&effective, &version, None, &mut options);

        assert_eq!(options.renderer, Renderer::DEFAULT);
    }

    #[test]
    fn the_optimizing_arguments_may_be_turned_off() {
        let mut preset = preset();
        preset.settings.no_jvm_options = Some(true);
        preset.settings.no_optimizing_jvm_options = Some(true);

        let options = apply(&preset, None);

        assert!(options.no_generated_jvm_arguments);
        assert!(options.no_generated_optimizing_jvm_arguments);
    }

    #[test]
    fn the_optimizing_arguments_are_kept_by_default() {
        let options = apply(&preset(), None);

        assert!(!options.no_generated_optimizing_jvm_arguments);
    }

    #[test]
    fn the_bits_of_the_runtime_are_read_from_its_platform() {
        let mut preset = preset();
        preset.settings.no_jvm_options = Some(false);

        let effective = preset.resolve(None);
        let mut options = LaunchOptions::default();
        let java = JavaRuntime::from_binary(
            PathBuf::from("java"),
            crate::java::JavaInfo::new(crate::java::Platform::current(), "21".to_string(), None),
            false,
        );
        apply_settings(
            &effective,
            &GameVersionNumber::as_game_version("1.20.1"),
            Some(&java),
            &mut options,
        );

        assert_eq!(options.java_bits, crate::java::Platform::current().bits());
        assert_eq!(options.java_platform, crate::java::Platform::current());
        assert_eq!(options.java_major_version, 21);
    }

    #[test]
    fn the_platform_of_the_runtime_is_read_from_the_runtime() {
        // A runtime of another architecture than the machine runs the game of
        // the architecture it was built for, so the native libraries are the
        // ones of that architecture.
        let platform = crate::java::Platform::new(
            crate::util::operating_system::OsType::Linux,
            crate::util::operating_system::Arch::Aarch64,
        );
        let preset = preset();
        let effective = preset.resolve(None);
        let mut options = LaunchOptions::default();
        let java = JavaRuntime::from_binary(
            PathBuf::from("java"),
            crate::java::JavaInfo::new(platform, "21".to_string(), None),
            false,
        );

        apply_settings(
            &effective,
            &GameVersionNumber::as_game_version("1.20.1"),
            Some(&java),
            &mut options,
        );

        assert_eq!(options.java_platform, platform);
        assert_eq!(options.java_bits, Some(64));
    }

    #[test]
    fn without_a_runtime_the_platform_of_the_machine_is_kept() {
        let options = apply(&preset(), None);

        assert_eq!(options.java_platform, crate::java::Platform::current());
    }

    #[test]
    fn the_other_options_are_copied() {
        let mut preset = preset();
        preset.settings.enable_debug_log_output = Some(true);
        preset.settings.allow_auto_agent = Some(true);
        preset.settings.use_native_glfw_or_sdl = Some(true);
        preset.settings.use_native_openal = Some(true);
        preset.settings.high_performance = Some(true);
        preset.settings.launcher_visibility = Some(LauncherVisibility::HideAndReopen);

        let options = apply(&preset, None);

        assert!(options.enable_debug_log_output);
        assert!(options.allow_auto_agent);
        assert!(options.use_native_glfw_or_sdl);
        assert!(options.use_native_openal);
        assert!(options.high_performance);
        assert!(options.daemon);
    }

    #[test]
    fn the_quick_play_option_is_copied() {
        let mut preset = preset();
        preset.settings.quick_play = Some(QuickPlayType::Multiplayer);
        preset.settings.quick_play_multiplayer = Some("example.org".to_string());

        let options = apply(&preset, None);

        assert_eq!(
            options.quick_play,
            Some(QuickPlayOption::MultiPlayer("example.org".to_string()))
        );
    }

    #[test]
    fn an_instance_can_override_the_settings() {
        let mut preset = preset();
        preset.settings.auto_memory = Some(false);
        preset.settings.max_memory = Some(4096);
        preset.settings.jvm_options = Some("-Xmx1G".to_string());

        let mut instance = InstanceSettings::new();
        instance.settings.auto_memory = Some(false);
        instance.settings.max_memory = Some(2048);
        instance.set_overridden("maxMemory", true);
        instance.set_overridden("autoMemory", true);

        let options = apply(&preset, Some(&instance));

        assert_eq!(options.max_memory, Some(2048));
        assert_eq!(options.jvm_arguments, vec!["-Xmx1G"]);
    }

    #[test]
    fn the_java_runtime_sets_the_executable_and_the_version() {
        let preset = preset();
        let java = runtime("21");

        let effective = preset.resolve(None);
        let mut options = LaunchOptions::default();
        apply_settings(
            &effective,
            &GameVersionNumber::as_game_version("1.20.1"),
            Some(&java),
            &mut options,
        );

        assert_eq!(options.java, PathBuf::from("java-21"));
        assert_eq!(options.java_major_version, 21);
    }

    #[test]
    fn the_auto_allocated_memory_caps_the_32_bit_memory() {
        assert!(auto_allocated_memory(4 * 1024 * 1024 * 1024, false) <= MAX_32_BIT_MEMORY);
        assert!(auto_allocated_memory(4 * 1024 * 1024 * 1024, true) > MAX_32_BIT_MEMORY);
    }

    #[test]
    fn the_auto_allocated_memory_keeps_the_available_memory_when_it_is_small() {
        assert_eq!(auto_allocated_memory(0, true), 0);
    }

    #[test]
    fn without_the_check_the_runtime_of_the_machine_is_used_when_none_fits() {
        let mut preset = preset();
        preset.settings.not_check_jvm = Some(true);
        let effective = preset.resolve(None);

        let java = find_launch_java(
            &effective,
            &[],
            &GameVersionNumber::as_game_version("1.20.5"),
            None,
        );

        assert_eq!(java, crate::java::default_java_runtime());
    }

    #[test]
    fn without_the_check_a_runtime_that_fits_is_still_preferred() {
        let mut preset = preset();
        preset.settings.not_check_jvm = Some(true);
        let effective = preset.resolve(None);
        let java = runtime("21");

        let chosen = find_launch_java(
            &effective,
            std::slice::from_ref(&java),
            &GameVersionNumber::as_game_version("1.20.1"),
            None,
        );

        assert_eq!(chosen, Some(java));
    }

    #[test]
    fn with_the_check_no_runtime_is_chosen_when_none_fits() {
        let preset = preset();
        let effective = preset.resolve(None);

        let java = find_launch_java(
            &effective,
            &[],
            &GameVersionNumber::as_game_version("1.20.5"),
            None,
        );

        assert_eq!(java, None);
    }

    use std::path::Path;
}
