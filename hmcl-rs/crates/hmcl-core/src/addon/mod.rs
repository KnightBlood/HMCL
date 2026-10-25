//! The addons the repositories of the launcher offer, and the addons the player
//! has installed.

pub mod import;
pub mod local_mod;
pub mod mod_loader;
pub mod mod_manager;
pub mod pack;
pub mod remote_addon;
pub mod shader_pack;
pub mod update;

pub use import::{
    import_data_packs, import_mods, import_resource_packs, is_importable_data_pack,
    is_importable_mod, is_importable_resource_pack, ImportError, ImportFailure, ImportReport,
    ImportedAddon,
};
pub use local_mod::{display_name, ModFile, DISABLED_EXTENSION, MOD_EXTENSIONS, OLD_EXTENSION};
pub use mod_loader::ModLoaderType;
pub use mod_manager::{loaders_of, ModError, ModManager};
pub use pack::{
    format_option_list, is_file_data_pack, is_file_resource_pack, pack_version_of,
    pack_version_of_jar, parse_option_list, read_options, supports_new_options_format,
    write_options, Pack, PackCompatibility, PackError, PackInfo, PackKind, PackManager,
    PackVersion, SupportedFormats, DISABLED_PACK_EXTENSION,
};
pub use remote_addon::{
    check_update_of_sources, check_updates, find_update, updates_of, updates_of_with_restrictions,
    AddonFile, AddonLoader, AddonSource, AddonType, AddonUpdate, AddonUpdates, AddonVersion,
    Dependency, DependencyType, RemoteAddon, UpdateRestriction, VersionType,
};
pub use shader_pack::{
    enabled_shader_pack, import_shader_pack, list_shader_packs, set_enabled_shader_pack,
    ShaderPack, SHADER_PACKS_DIRECTORY,
};
pub use update::{
    install_update, install_updates, AddonKind, AddonUpdateReport, AddonUpdateRequest,
    InstalledUpdate, UpdateError,
};
