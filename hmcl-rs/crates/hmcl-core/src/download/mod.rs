//! The downloads of the files of the game.

pub mod cache;
pub mod downloader;
pub mod game;
pub mod library_cache;
pub mod provider;
pub mod task;

pub use cache::{drop_query, CacheError, CacheRepository, ETagIndex, ETagItem};
pub use downloader::TaskDownloader;
pub use library_cache::{
    checksum_valid, DefaultCacheRepository, LibraryIndex, LibraryIndexFile, LibraryKind,
};
pub use provider::{
    is_china_mainland_machine, DownloadProvider, FabricVersionListCandidates, MirrorRule,
    SourcePreference, ASSET_URL, DEFAULT_BMCLAPI_ROOT, VERSION_MANIFEST_URL,
};
pub use task::{
    DownloadCandidate, DownloadCandidates, DownloadError, DownloadSource, DownloadedFile, Fetcher,
    FileDownloadTask, IntegrityCheck, ResponseInfo,
};
