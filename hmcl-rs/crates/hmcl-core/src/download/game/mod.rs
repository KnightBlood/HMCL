//! The models of the versions of the game.
//!
//! The launcher reads the list of the versions of the game and the details
//! of every version; the models of the files are the ones the servers of the
//! game use.

pub mod artifact;
pub mod asset_index;
pub mod instance_manifest;
pub mod instance_patch;
pub mod release_type;
pub mod version_manifest;
pub mod version_metadata;

pub use artifact::Artifact;
pub use asset_index::{AssetIndexFile, AssetObject, ASSET_URL};
pub use instance_manifest::InstanceManifest;
pub use instance_patch::InstancePatch;
pub use release_type::ReleaseType;
pub use version_manifest::{LatestVersions, VersionManifest, VersionManifestEntry};
pub use version_metadata::{
    rules_allow, Argument, ArgumentDetails, ArgumentValue, Arguments, AssetIndex,
    CompatibilityRule, DownloadInfo, Downloads, ExtractRules, JavaVersion, Library,
    LibraryDownloads, Logging, LoggingArgument, OsRule, RuleAction, RuleContext, VersionMetadata,
};
