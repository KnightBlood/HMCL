//! The instances of the game.

pub mod component;
pub mod crash_report;
pub mod game_version;
pub mod graphics;
pub mod instance_icon;
pub mod java_version;
pub mod launch;
pub mod process_priority;
pub mod quick_play;
pub mod repository;
pub mod repository_draft;
pub mod repository_layout;
pub mod repository_snapshot;
pub mod resolver;
pub mod verification;
pub mod world;
pub mod world_backup;

pub use component::{
    AnalyzeError, ComponentAnalyzer, GameComponentType, Mark, FORGE_OPTIFINE_MAIN,
};
pub use crash_report::{
    analyze, extract_crash_report, find_crash_report, find_keywords_from_crash_report,
    java_version_from_major_version, CrashRuleMatch, Rule,
};
pub use game_version::minecraft_version;
pub use graphics::{GraphicsApi, Renderer};
pub use instance_icon::{icon_of_mod_loader, instance_icon, InstanceIcon};
pub use java_version::{find_suitable_java, GameJavaVersion, JavaVersionConstraint};
pub use launch::{
    build_command, classify_exit, classpath, default_game_arguments, default_jvm_arguments,
    extract_natives, extract_natives_with, libraries_to_download, library_file, missing_files,
    native_archives, plan_launch, pump_lines, repair_for_launch, repair_libraries, required_files,
    resolve_arguments, should_extract_file, start_command, unique_libraries, verify_file,
    virtual_assets, DownloadError, ExitType, FileDownloader, GameExit, GameLine, GameProcess,
    LaunchCommand, LaunchError, LaunchOptions, LaunchPaths, LaunchPlan, LaunchService,
    LaunchVariables, LibraryFile, LibraryRepair, NativeArchive, NativeOptions, NormalizeError,
    PrepareError, ProcessError, RepairError, RequiredFile, RequiredFileKind, VirtualAsset,
    VirtualAssets,
};
pub use process_priority::ProcessPriority;
pub use quick_play::{QuickPlayOption, QuickPlayType};
pub use repository::{
    is_valid_instance_id, GameRepository, RepositoryError, ABNORMAL_MARKER, IMAGE_EXTENSIONS,
};
pub use repository_draft::{DraftError, DraftState, GameRepositoryDraft, DRAFT_DIRECTORY};
pub use repository_layout::{OfficialLayout, RepositoryLayout};
pub use repository_snapshot::RepositorySnapshot;
pub use resolver::{InstanceResolver, ResolveError};
pub use verification::{
    bundled_library_resource, fix_optifine_of_forge, fml_libraries, forge_optifine_broken_range,
    library_needs_download, remove_legacy_forge_signatures, FmlLibrary, FORGE_SIGNATURE_FILES,
    FORGE_SIGNATURE_VERSION,
};
pub use world::{
    get_worlds, Difficulty, Dimension, GameMode, World, WorldError, WorldLocation, WorldPosition,
    ICON_SIZE,
};
pub use world_backup::{backup, list_backups, Backup, BackupError, BACKUPS_DIRECTORY};
