//! The modpacks the launchers of the community write.
//!
//! A modpack is an archive that holds the files of an instance and a manifest
//! that names them. Every launcher writes its own manifest, so the module has
//! a reader for every format the launcher of Java reads: the format of
//! MultiMC, the format of MCBBS, the format of the server of HMCL, the format
//! of CurseForge and the format of Modrinth. The manifests of CurseForge and
//! Modrinth name the add-ons instead of the files, so the launcher asks the
//! repository of the add-on for the name and the address of a file.
//!
//! The formats share the way the launcher updates an instance: the manifest of
//! the server of the modpack is compared with the files of the instance, and
//! the result is a plan of the files to download and to remove, which is
//! described by [`plan::UpdatePlan`] and kept by [`configuration`].
//!
//! The module also holds what an export of an instance needs: the names of the
//! files an export leaves out ([`adviser`]) and the way the launcher reads the
//! files of an instance under the directory of the archive ([`install`]).

pub mod adviser;
pub mod completion;
pub mod configuration;
pub mod curse;
pub mod export;
pub mod install;
pub mod mcbbs;
pub mod modrinth;
pub mod multimc;
pub mod optional;
pub mod plan;
pub mod server;

pub use adviser::{
    accept_file, suggest_mod, ModAdviser, ModSuggestion, MODPACK_BLACK_LIST,
    MODPACK_SUGGESTED_BLACK_LIST,
};
pub use completion::{
    has_simple_mod, read_excluded, report_of as report_of_mcbbs, report_of_curse,
    report_of_modrinth, unresolved_curse_files, unresolved_files as unresolved_mcbbs_files,
    write_excluded, CompletionDownload, CompletionError, CompletionPlan, CompletionReport,
    UnresolvedFile, UnresolvedKind, DOWNLOAD_STAGE, EXCLUDED_FILE, UNPACK_STAGE,
};
pub use configuration::{ConfigurationError, FileInformation, ModpackConfiguration};
pub use curse::{
    plan_completion as plan_curse_completion,
    plan_completion_with_updates as plan_curse_completion_with_updates,
    removed_mods as removed_curse_mods, CurseError, CurseFile, CurseKind, CurseManifest,
    CurseMinecraft, CurseModLoader, MANIFEST as CURSE_MANIFEST,
    MANIFEST_TYPE as CURSE_MANIFEST_TYPE, MOD_LIST, PROVIDER_NAME as CURSE_PROVIDER_NAME,
};
pub use export::{
    collect_files, curse_manifest, export_addons, mcbbs_manifest, modrinth_dependencies,
    modrinth_manifest, server_manifest, write_mcbbs, write_modrinth, write_server, ExportError,
    ExportFile, ExportFormat, ExportOptions, ExportOutcome, ModpackExportInfo, BUNDLED_DIRECTORY,
};
pub use install::{
    archive_overrides, install, update_with_backup, InstallError, InstallOutcome, UpdateError,
    OVERRIDES,
};
pub use mcbbs::{
    plan_update as plan_mcbbs_update, LaunchInfo, McbbsError, McbbsFile, McbbsModpackManifest,
    McbbsUpdate, Origin, Settings, MANIFEST_JSON, MANIFEST_TYPE, PACK_META, PATCH_NAME,
};
pub use modrinth::{
    plan_completion as plan_modrinth_completion,
    plan_completion_with_updates as plan_modrinth_completion_with_updates,
    removed_files as removed_modrinth_files, ModrinthError, ModrinthFile, ModrinthManifest, CLIENT,
    CLIENT_OVERRIDES, FORMAT_VERSION as MODRINTH_FORMAT_VERSION, GAME as MODRINTH_GAME,
    MANIFEST as MODRINTH_MANIFEST, MINECRAFT, OPTIONAL, OVERRIDES as MODRINTH_OVERRIDES,
    PROVIDER_NAME as MODRINTH_PROVIDER_NAME, UNSUPPORTED,
};
pub use multimc::{
    bootstrap_library, bootstrap_profile_url, component_of_uid, installer_profile, meta_url,
    resolve_patches, root_directory, MultiMcComponent, MultiMcConfig, MultiMcError,
    MultiMcInstancePatch, MultiMcManifest, MultiMcRequirement, ResolvedInstance, BOOTSTRAP_GROUP,
    BOOTSTRAP_MAIN, BOOTSTRAP_NAME, BOOTSTRAP_VERSION, DEFAULT_MAIN_CLASS, META_ROOT,
};
pub use optional::{ModpackFile, OptionalFiles};
pub use plan::{relative_path, sha1_of, DownloadRequest, UpdatePlan};
pub use server::{
    is_disabled_mod, plan_update as plan_server_update, Addon, ServerModpackError,
    ServerModpackManifest, ServerUpdate,
};
