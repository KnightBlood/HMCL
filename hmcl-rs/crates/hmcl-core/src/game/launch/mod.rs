//! The command a version of the game is started with.
//!
//! The versions of the game tell the launcher the arguments the game is
//! started with; the launcher adds the arguments of the player, the amount of
//! the memory the game may use and the files the game reads its classes from,
//! and replaces the variables of the arguments with the values of the player
//! and of the machine.
//!
//! The launcher also reads the files the version needs before it is started.
//! The plan of a start tells the launcher which files it downloads and which
//! archives of the native libraries it extracts.

pub mod arguments;
pub mod assets;
pub mod classpath;
pub mod command;
pub mod libraries;
pub mod library;
pub mod native_patch;
pub mod natives;
pub mod normalize;
pub mod options;
pub mod plan;
pub mod process;
pub mod repair;
pub mod required;
pub mod script;
pub mod service;
pub mod settings;
pub mod variables;

pub use arguments::{
    default_game_arguments, default_jvm_arguments, resolve_arguments, split_arguments, tokenize,
};
pub use assets::{virtual_assets, VirtualAsset, VirtualAssets};
pub use classpath::{
    classpath, classpath_separator, is_native, join_classpath, library_paths, native_classifier,
};
pub use command::{arch_name, build_command, os_name, rule_context, LaunchCommand, LaunchError};
pub use libraries::unique_libraries;
pub use library::{library_file, LibraryFile, DEFAULT_LIBRARY_URL};
pub use native_patch::{current_context, patch_natives, platform_name, NativePatchOptions};
pub use natives::{
    extract_natives, extract_natives_with, native_archives, should_extract_file, NativeArchive,
    NativeOptions,
};
pub use normalize::{repair_for_launch, NormalizeError};
pub use options::{LaunchOptions, LaunchPaths};
pub use plan::{plan_launch, LaunchPlan};
pub use process::{
    classify_exit, pump_lines, ExitType, GameExit, GameLine, GameProcess, ProcessError,
};
pub use repair::{repair_libraries, LibraryRepair, RepairError};
pub use required::{
    asset_files, asset_index_file, client_file, is_present, libraries_to_download, library_files,
    missing_files, required_files, verify_file, RequiredFile, RequiredFileKind,
};
pub use script::{write_script, ScriptError, ScriptKind, MAX_BATCH_COMMAND_LENGTH};
pub use service::{start_command, DownloadError, FileDownloader, LaunchService, PrepareError};
pub use settings::{apply_settings, auto_allocated_memory, find_launch_java};
pub use variables::LaunchVariables;
