//! The downloads of the files of the game.

pub mod cache;
pub mod downloader;
pub mod forge;
pub mod game;
pub mod library_cache;
pub mod loader;
pub mod provider;
pub mod task;

pub use cache::{drop_query, CacheError, CacheRepository, ETagIndex, ETagItem};
pub use downloader::TaskDownloader;
pub use forge::{
    client_mappings_digest, copy_installer_libraries, data_tokens, detect_format, digest_of,
    installer_game_version, is_mojang_mappings_task, is_neoforge_installer, legacy_library_name,
    legacy_patch, legacy_version_info, modify_neoforge_new_version, modify_neoforge_old_version,
    modify_version, mojang_mappings_target, neoforge_patch_id, parse_literal, processor_patch,
    profile_libraries, profile_name, read_processor_profile, read_profile_keys, replace_tokens,
    resolve_processor, Datum, InstallProfileKeys, InstallerFormat, LegacyInstall, Processor,
    ProcessorInstallProfile, ProcessorOptions, ResolvedProcessor, FORGE_PATCH_PRIORITY,
};
pub use library_cache::{
    checksum_valid, library_path_of, DefaultCacheRepository, LibraryIndex, LibraryIndexFile,
    LibraryKind,
};
pub use loader::{
    loader_patch, resolved_libraries, LaunchWrapper, LauncherMeta, Libraries, LoaderLibrary,
    LoaderMeta, LoaderVersion, MainClass, ModLoader, Tweakers, LOADER_PATCH_PRIORITY,
};
pub use provider::{
    is_china_mainland_machine, DownloadProvider, FabricVersionListCandidates, MirrorRule,
    SourcePreference, ASSET_URL, DEFAULT_BMCLAPI_ROOT, VERSION_MANIFEST_URL,
};
pub use task::{
    DownloadCandidate, DownloadCandidates, DownloadError, DownloadSource, DownloadedFile, Fetcher,
    FileDownloadTask, IntegrityCheck, ResponseInfo,
};
