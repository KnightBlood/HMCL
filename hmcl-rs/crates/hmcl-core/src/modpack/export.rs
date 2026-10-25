//! Writing an instance of the player as a modpack.
//!
//! The wizard of the launcher collects the settings of an export, and the
//! writer of every format turns them into the archive the players download. The
//! formats differ in the manifest they carry: MCBBS writes `mcbbs.packmeta` and
//! keeps the files in `overrides`, Modrinth writes `modrinth.index.json` and
//! names the files it can find in the repository of the add-on, and the format
//! of the server of HMCL writes `server-manifest.json`.
//!
//! The files of the instance are the same for every format: the writer walks
//! the run directory of the instance, leaves out the files the launcher hides
//! ([`super::adviser`]) and writes the rest under the directory of the format.
//! A file the player selected is written; a file the wizard did not select is
//! left out.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::game::GameComponentType;
use crate::util::digest::Algorithm;
use crate::util::string::is_blank;
use crate::util::zip::{self, ZipWriter};

use super::adviser::accept_file;
use super::curse::{CurseManifest, CurseMinecraft, CurseModLoader, OVERRIDES as CURSE_OVERRIDES};
use super::mcbbs::{
    Addon as McbbsAddon, LaunchInfo, McbbsFile, McbbsModpackManifest, Origin, Settings,
    MANIFEST_JSON, MANIFEST_TYPE, PACK_META,
};
use super::modrinth::{
    ModrinthFile, ModrinthManifest, CLIENT_OVERRIDES, FORMAT_VERSION as MODRINTH_FORMAT_VERSION,
    GAME as MODRINTH_GAME, MANIFEST as MODRINTH_MANIFEST,
};
use super::server::{Addon as ServerAddon, ServerModpackManifest, MANIFEST as SERVER_MANIFEST};

/// The name of the file the launcher writes the settings of a bundled instance
/// in.
pub const BUNDLED_DIRECTORY: &str = ".hmcl";

/// The formats an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// The format of MCBBS, which an instance is completed from.
    Mcbbs,
    /// The format of the server of HMCL.
    Server,
    /// The format of Modrinth.
    Modrinth,
}

impl ExportFormat {
    /// Returns the format the wizard of an export names, or `None` when the name
    /// is not one the launcher knows.
    ///
    /// The wizard stores the type of a modpack as the short names `"mcbbs"`,
    /// `"server"` and `"modrinth"`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "mcbbs" => Some(Self::Mcbbs),
            "server" => Some(Self::Server),
            "modrinth" => Some(Self::Modrinth),
            _ => None,
        }
    }

    /// Returns the extension the archive of the format has.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Mcbbs | Self::Server => "zip",
            Self::Modrinth => "mrpack",
        }
    }

    /// Returns whether the archive of the format needs an address the instance
    /// is completed from.
    ///
    /// A modpack of MCBBS names the server the manifest of its newest version
    /// is read from, so an export of the format has to name one; a modpack of
    /// Modrinth and of the server needs no server.
    pub const fn requires_file_api(self) -> bool {
        matches!(self, Self::Mcbbs)
    }

    /// Returns the options the wizard of the format asks for.
    pub const fn options(self) -> ExportOptions {
        match self {
            Self::Mcbbs => ExportOptions::MCBBS,
            Self::Server => ExportOptions::SERVER,
            Self::Modrinth => ExportOptions::MODRINTH,
        }
    }
}

/// The settings the wizard of an export asks for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModpackExportInfo {
    /// The files the player selected, relative to the run directory.
    ///
    /// An empty list keeps every file the black list does not hide.
    pub whitelist: Vec<String>,
    /// The name of the modpack.
    pub name: String,
    /// The author of the modpack.
    pub author: String,
    /// The version of the modpack.
    pub version: String,
    /// The description of the modpack, which may hold HTML.
    pub description: String,
    /// The page of the modpack.
    pub url: String,
    /// The address the manifest of the newest version is read from.
    pub file_api: Option<String>,
    /// The smallest amount of the memory the game needs, in mebibytes.
    pub min_memory: i32,
    /// The versions of Java the modpack supports, if the author names them.
    pub supported_java_versions: Option<Vec<i32>>,
    /// The arguments of the game the modpack adds, written as one line.
    pub launch_arguments: String,
    /// The arguments of the virtual machine the modpack adds, written as one
    /// line.
    pub java_arguments: String,
    /// The server of the account the modpack is played on, if it names one.
    pub authlib_injector_server: Option<String>,
    /// The communities the modpack belongs to.
    pub origins: Vec<Origin>,
    /// Whether the launcher updates the instance even when the player asked it
    /// not to.
    pub force_update: bool,
    /// Whether the archive holds the launcher itself.
    pub pack_with_launcher: bool,
    /// Whether an export of Modrinth keeps every file in the archive instead of
    /// naming the files of the repository.
    pub no_create_remote_files: bool,
    /// Whether an export of Modrinth skips the files of CurseForge.
    pub skip_curse_forge_remote_files: bool,
}

impl ModpackExportInfo {
    /// Refuses an export the format cannot hold.
    ///
    /// The wizard asks only for the settings the format holds, so the fields a
    /// format does not ask for are left as they are instead of being refused.
    pub fn validate(&self, options: ExportOptions) -> Result<(), ExportError> {
        if is_blank(&self.name) {
            return Err(ExportError::Missing("name"));
        }
        if is_blank(&self.version) {
            return Err(ExportError::Missing("version"));
        }
        if options.require_author && is_blank(&self.author) {
            return Err(ExportError::Missing("author"));
        }
        if options.require_url && is_blank(&self.url) {
            return Err(ExportError::Missing("url"));
        }
        if options.require_file_api {
            let api = self.file_api.as_deref().unwrap_or_default();
            if is_blank(api) {
                // An export of MCBBS may skip the server, but the archive of a
                // modpack that names none cannot be completed.
                if options.validate_file_api {
                    return Err(ExportError::Missing("file_api"));
                }
            } else if !is_url(api) {
                return Err(ExportError::Malformed("file_api".to_string()));
            }
        }
        if options.require_min_memory && self.min_memory < 0 {
            return Err(ExportError::Malformed("min_memory".to_string()));
        }
        if options.require_origins && self.origins.is_empty() {
            return Err(ExportError::Missing("origins"));
        }
        Ok(())
    }

    /// Returns the address the manifest of the newest version is read from.
    ///
    /// The launcher appends the names of the files to the address, so the
    /// address has no slash at its end.
    pub fn file_api(&self) -> Option<&str> {
        self.file_api
            .as_deref()
            .map(|api| api.trim_end_matches('/'))
            .filter(|api| !api.is_empty())
    }
}

/// Returns whether the text is an address the launcher reads a file from.
fn is_url(text: &str) -> bool {
    text.starts_with("https://") || text.starts_with("http://")
}

/// The settings the wizard of a format asks for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportOptions {
    /// Whether the format needs the author of the modpack.
    pub require_author: bool,
    /// Whether the format needs the page of the modpack.
    pub require_url: bool,
    /// Whether the format needs the flag of the forced update.
    pub require_force_update: bool,
    /// Whether the format names the server the instance is completed from.
    pub require_file_api: bool,
    /// Whether the server is required instead of being allowed to be empty.
    pub validate_file_api: bool,
    /// Whether the format names the smallest amount of the memory.
    pub require_min_memory: bool,
    /// Whether the format names the server of the account.
    pub require_authlib_injector_server: bool,
    /// Whether the format names the arguments of the game.
    pub require_launch_arguments: bool,
    /// Whether the format names the arguments of the virtual machine.
    pub require_java_arguments: bool,
    /// Whether the format names the communities of the modpack.
    pub require_origins: bool,
    /// Whether the format asks for the files of Modrinth to be kept.
    pub require_no_create_remote_files: bool,
    /// Whether the format asks for the files of CurseForge to be skipped.
    pub require_skip_curse_forge_remote_files: bool,
}

impl ExportOptions {
    /// The settings of an export of MCBBS.
    pub const MCBBS: Self = Self {
        require_author: true,
        require_url: true,
        require_force_update: true,
        require_file_api: true,
        validate_file_api: true,
        require_min_memory: true,
        require_authlib_injector_server: true,
        require_launch_arguments: true,
        require_java_arguments: true,
        require_origins: true,
        require_no_create_remote_files: false,
        require_skip_curse_forge_remote_files: false,
    };

    /// The settings of an export of the server of HMCL.
    pub const SERVER: Self = Self {
        require_author: true,
        require_url: false,
        require_force_update: false,
        require_file_api: true,
        validate_file_api: false,
        require_min_memory: false,
        require_authlib_injector_server: false,
        require_launch_arguments: false,
        require_java_arguments: false,
        require_origins: false,
        require_no_create_remote_files: false,
        require_skip_curse_forge_remote_files: false,
    };

    /// The settings of an export of Modrinth.
    pub const MODRINTH: Self = Self {
        require_author: false,
        require_url: false,
        require_force_update: false,
        require_file_api: false,
        validate_file_api: false,
        require_min_memory: false,
        require_authlib_injector_server: false,
        require_launch_arguments: false,
        require_java_arguments: false,
        require_origins: false,
        require_no_create_remote_files: true,
        require_skip_curse_forge_remote_files: true,
    };

    /// Returns the options with the server of the instance required.
    pub const fn require_file_api(mut self) -> Self {
        self.require_file_api = true;
        self
    }

    /// Returns the options with the author of the instance required.
    pub const fn require_author(mut self) -> Self {
        self.require_author = true;
        self
    }
}

/// A file of an instance an export writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFile {
    /// The path of the file in the instance, with `/` as the separator.
    pub path: String,
    /// The SHA-1 digest of the file.
    pub sha1: String,
}

/// What an export wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportOutcome {
    /// The path of the archive.
    pub archive: PathBuf,
    /// The files the archive holds.
    pub files: Vec<ExportFile>,
    /// The files the export could not read.
    pub warnings: Vec<String>,
}

/// Returns the files of an instance an export writes.
///
/// The files are read in the order of their names, so the same instance writes
/// the same archive every time. The files the launcher hides are left out, and
/// when the player selected files only the selected files are written.
pub fn collect_files(
    run_directory: &Path,
    black_list: &[&str],
    white_list: &[String],
) -> Result<Vec<ExportFile>, ExportError> {
    let mut files = Vec::new();
    collect_directory(run_directory, "", black_list, white_list, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

/// Walks a directory and adds the files an export writes.
fn collect_directory(
    directory: &Path,
    prefix: &str,
    black_list: &[&str],
    white_list: &[String],
    files: &mut Vec<ExportFile>,
) -> Result<(), ExportError> {
    let mut entries = std::fs::read_dir(directory)
        .map_err(|error| ExportError::io(directory, error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ExportError::io(directory, error))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        // The Finder of macOS writes a file of every directory, which is not
        // something the player put there.
        if name == ".DS_Store" {
            continue;
        }

        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let entry_path = entry.path();

        if entry_path.is_dir() {
            if !accept_file(&path, black_list, white_list) {
                continue;
            }
            collect_directory(&entry_path, &path, black_list, white_list, files)?;
        } else if entry_path.is_file() {
            if !accept_file(&path, black_list, white_list) {
                continue;
            }
            let sha1 = Algorithm::Sha1
                .digest_file_to_string(&entry_path)
                .map_err(|error| ExportError::Io {
                    path: entry_path.clone(),
                    message: error.to_string(),
                })?;
            files.push(ExportFile { path, sha1 });
        }
    }

    Ok(())
}

/// Writes the files of an instance under a directory of an archive.
fn write_overrides(
    archive: &mut ZipWriter,
    run_directory: &Path,
    directory: &str,
    files: &[ExportFile],
) -> Result<(), ExportError> {
    let prefix = directory.trim_matches('/');
    for file in files {
        let name = if prefix.is_empty() {
            file.path.clone()
        } else {
            format!("{prefix}/{}", file.path)
        };
        archive
            .add_path(&name, &run_directory.join(&file.path))
            .map_err(ExportError::Zip)?;
    }
    Ok(())
}

/// Returns the components an export names, the game first.
///
/// A modpack names the version of the game and the mod loaders the instance
/// runs, because the launcher of the player installs the instance from them. A
/// component the launcher does not know is not named, because the launcher of
/// the player would not know what to install.
pub fn export_addons<'a>(
    game_version: &str,
    components: impl IntoIterator<Item = &'a crate::game::Mark>,
) -> Vec<(String, String)> {
    let mut addons = vec![(
        GameComponentType::Game.patch_id().to_string(),
        game_version.to_string(),
    )];

    for mark in components {
        if mark.component.is_mod_loader() || mark.component == GameComponentType::OptiFine {
            if mark.version.is_empty() {
                continue;
            }
            addons.push((mark.component.patch_id().to_string(), mark.version.clone()));
        }
    }

    addons
}

/// Returns the components an export names, with the game first.
///
/// The game is the component every modpack names, so a caller that builds the
/// list of the components itself does not have to remember it.
fn with_game(game_version: &str, components: &[(String, String)]) -> Vec<(String, String)> {
    let mut addons = vec![(
        GameComponentType::Game.patch_id().to_string(),
        game_version.to_string(),
    )];
    addons.extend(
        components
            .iter()
            .filter(|(id, _)| id != GameComponentType::Game.patch_id())
            .cloned(),
    );
    addons
}

/// Returns the manifest of a modpack of MCBBS.
///
/// The manifest names the files of the archive as the files the launcher
/// installs from the archive, so an update of the modpack replaces a file only
/// when the player did not change it.
pub fn mcbbs_manifest(
    info: &ModpackExportInfo,
    game_version: &str,
    components: &[(String, String)],
    files: &[ExportFile],
) -> McbbsModpackManifest {
    let components = with_game(game_version, components);
    McbbsModpackManifest {
        manifest_type: Some(MANIFEST_TYPE.to_string()),
        manifest_version: 2,
        name: info.name.clone(),
        version: info.version.clone(),
        author: info.author.clone(),
        description: info.description.clone(),
        file_api: info.file_api().map(str::to_string),
        url: info.url.clone(),
        force_update: info.force_update,
        origins: info.origins.clone(),
        addons: Some(
            components
                .iter()
                .map(|(id, version)| McbbsAddon {
                    id: id.clone(),
                    version: version.clone(),
                })
                .collect(),
        ),
        libraries: Vec::new(),
        files: Some(
            files
                .iter()
                .map(|file| McbbsFile::Addon {
                    // The files of an export belong to the modpack, so the
                    // launcher replaces them even when the player changed them.
                    force: true,
                    path: file.path.clone(),
                    hash: file.sha1.clone(),
                })
                .collect(),
        ),
        settings: Settings::default(),
        launch_info: LaunchInfo {
            min_memory: info.min_memory,
            support_java: info.supported_java_versions.clone(),
            launch_arguments: crate::game::launch::arguments::tokenize(&info.launch_arguments),
            java_arguments: crate::game::launch::arguments::tokenize(&info.java_arguments),
        },
    }
}

/// Returns the manifest of CurseForge the archive of an export of MCBBS holds.
///
/// The manifest of CurseForge is read by the launchers that do not know the
/// format of MCBBS, so the archive carries both. The mod loaders are named the
/// way CurseForge names them; OptiFine and LiteLoader have no name in the
/// format and are left out.
pub fn curse_manifest(
    info: &ModpackExportInfo,
    game_version: &str,
    components: &[(String, String)],
) -> CurseManifest {
    let mod_loaders = components
        .iter()
        .filter_map(|(id, version)| {
            let name = match id.as_str() {
                "forge" => format!("forge-{version}"),
                "neoforge" => format!("neoforge-{version}"),
                "fabric" => format!("fabric-{version}"),
                _ => return None,
            };
            Some(CurseModLoader {
                id: name,
                primary: true,
            })
        })
        .collect();

    CurseManifest {
        manifest_type: MANIFEST_TYPE.to_string(),
        manifest_version: 1,
        name: info.name.clone(),
        version: info.version.clone(),
        author: info.author.clone(),
        overrides: CURSE_OVERRIDES.to_string(),
        minecraft: CurseMinecraft {
            version: game_version.to_string(),
            mod_loaders,
        },
        files: Vec::new(),
    }
}

/// Returns the manifest of a modpack of the server of HMCL.
///
/// The manifest names the version of the game with the components, because the
/// launcher of the player installs the instance from them.
pub fn server_manifest(
    info: &ModpackExportInfo,
    game_version: &str,
    components: &[(String, String)],
    files: &[ExportFile],
) -> ServerModpackManifest {
    let components = with_game(game_version, components);
    ServerModpackManifest {
        name: info.name.clone(),
        author: info.author.clone(),
        version: info.version.clone(),
        description: info.description.clone(),
        file_api: info.file_api().map(str::to_string),
        files: Some(
            files
                .iter()
                .map(|file| {
                    crate::modpack::FileInformation::new(file.path.clone(), file.sha1.clone())
                })
                .collect(),
        ),
        addons: Some(
            components
                .iter()
                .map(|(id, version)| ServerAddon {
                    id: id.clone(),
                    version: version.clone(),
                })
                .collect(),
        ),
    }
}

/// Returns the dependencies of a modpack of Modrinth.
///
/// Modrinth names the loaders with the keys of its own, which differ from the
/// identifiers of the launcher: the loader of Fabric is `fabric-loader` and the
/// loader of Quilt is `quilt-loader`.
pub fn modrinth_dependencies(
    game_version: &str,
    components: &[(String, String)],
) -> std::collections::BTreeMap<String, String> {
    let mut dependencies = std::collections::BTreeMap::new();
    dependencies.insert("minecraft".to_string(), game_version.to_string());

    for (id, version) in components {
        let key = match id.as_str() {
            "forge" => "forge",
            "neoforge" => "neoforge",
            "fabric" => "fabric-loader",
            "quilt" => "quilt-loader",
            _ => continue,
        };
        dependencies.insert(key.to_string(), version.clone());
    }

    dependencies
}

/// Returns the manifest of a modpack of Modrinth.
///
/// The files are the ones the launcher found in the repository of the add-on:
/// a file it could not find is kept in the archive of the export, so the
/// manifest does not name it.
pub fn modrinth_manifest(
    info: &ModpackExportInfo,
    game_version: &str,
    components: &[(String, String)],
    files: Vec<ModrinthFile>,
) -> ModrinthManifest {
    ModrinthManifest {
        game: MODRINTH_GAME.to_string(),
        format_version: MODRINTH_FORMAT_VERSION,
        version_id: info.version.clone(),
        name: info.name.clone(),
        summary: Some(info.description.clone()).filter(|text| !text.is_empty()),
        files,
        dependencies: modrinth_dependencies(game_version, components),
    }
}

/// Writes an instance as a modpack of MCBBS.
///
/// The archive holds the files of the instance under `overrides`, the manifest
/// of MCBBS in `mcbbs.packmeta` and the manifest of CurseForge in
/// `manifest.json`, which is what the launcher of Java writes.
pub fn write_mcbbs(
    run_directory: &Path,
    archive: &Path,
    info: &ModpackExportInfo,
    components: &[(String, String)],
    game_version: &str,
) -> Result<ExportOutcome, ExportError> {
    info.validate(ExportFormat::Mcbbs.options())?;

    let files = collect_files(run_directory, &black_list(), &info.whitelist)?;
    let manifest = mcbbs_manifest(info, game_version, components, &files);

    let mut writer = ZipWriter::create(archive).map_err(ExportError::Zip)?;
    write_overrides(&mut writer, run_directory, CURSE_OVERRIDES, &files)?;

    let text = serde_json::to_string(&manifest)
        .map_err(|error| ExportError::Malformed(error.to_string()))?;
    writer
        .add_file(PACK_META, text.as_bytes())
        .map_err(ExportError::Zip)?;

    let curse = curse_manifest(info, game_version, components);
    let text =
        serde_json::to_string(&curse).map_err(|error| ExportError::Malformed(error.to_string()))?;
    writer
        .add_file(MANIFEST_JSON, text.as_bytes())
        .map_err(ExportError::Zip)?;

    writer.finish().map_err(ExportError::Zip)?;

    Ok(ExportOutcome {
        archive: archive.to_path_buf(),
        files,
        warnings: Vec::new(),
    })
}

/// Writes an instance as a modpack of the server of HMCL.
pub fn write_server(
    run_directory: &Path,
    archive: &Path,
    info: &ModpackExportInfo,
    components: &[(String, String)],
    game_version: &str,
) -> Result<ExportOutcome, ExportError> {
    info.validate(ExportFormat::Server.options())?;

    let files = collect_files(run_directory, &black_list(), &info.whitelist)?;
    let manifest = server_manifest(info, game_version, components, &files);

    let mut writer = ZipWriter::create(archive).map_err(ExportError::Zip)?;
    write_overrides(&mut writer, run_directory, CURSE_OVERRIDES, &files)?;

    let text = serde_json::to_string(&manifest)
        .map_err(|error| ExportError::Malformed(error.to_string()))?;
    writer
        .add_file(SERVER_MANIFEST, text.as_bytes())
        .map_err(ExportError::Zip)?;
    writer.finish().map_err(ExportError::Zip)?;

    Ok(ExportOutcome {
        archive: archive.to_path_buf(),
        files,
        warnings: Vec::new(),
    })
}

/// Writes an instance as a modpack of Modrinth.
///
/// The files the launcher can find in a repository are named in the manifest,
/// and the other files are written into the archive. The caller tells which
/// files a repository knows, because the lookup needs the network; a file the
/// lookup does not know is written into the archive instead of being lost.
pub fn write_modrinth(
    run_directory: &Path,
    archive: &Path,
    info: &ModpackExportInfo,
    components: &[(String, String)],
    game_version: &str,
    remote: impl Fn(&Path, &str) -> Option<ModrinthFile>,
) -> Result<ExportOutcome, ExportError> {
    info.validate(ExportFormat::Modrinth.options())?;

    let files = collect_files(run_directory, &black_list(), &info.whitelist)?;

    let mut named = Vec::new();
    let mut kept = Vec::new();
    for file in &files {
        match remote(&run_directory.join(&file.path), &file.path) {
            Some(entry) => named.push(entry),
            None => kept.push(file.clone()),
        }
    }

    let manifest = modrinth_manifest(info, game_version, components, named);

    let mut writer = ZipWriter::create(archive).map_err(ExportError::Zip)?;
    write_overrides(&mut writer, run_directory, CLIENT_OVERRIDES, &kept)?;

    let text = serde_json::to_string(&manifest)
        .map_err(|error| ExportError::Malformed(error.to_string()))?;
    writer
        .add_file(MODRINTH_MANIFEST, text.as_bytes())
        .map_err(ExportError::Zip)?;
    writer.finish().map_err(ExportError::Zip)?;

    Ok(ExportOutcome {
        archive: archive.to_path_buf(),
        files,
        warnings: Vec::new(),
    })
}

/// Returns the names of the files an export leaves out.
fn black_list() -> Vec<&'static str> {
    super::adviser::MODPACK_BLACK_LIST.to_vec()
}

/// Why an export cannot be written.
#[derive(Debug)]
pub enum ExportError {
    /// The settings of the export are incomplete.
    Missing(&'static str),
    /// A setting of the export is not a value the format holds.
    Malformed(String),
    /// A file of the instance cannot be read.
    Io {
        /// The file the export worked with.
        path: PathBuf,
        /// The reason the file cannot be read.
        message: String,
    },
    /// The archive cannot be written.
    Zip(zip::ZipError),
}

impl ExportError {
    /// Returns the failure of a file the export cannot read.
    fn io(path: &Path, error: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(field) => write!(formatter, "the export names no {field}"),
            Self::Malformed(field) => write!(formatter, "the export names an invalid {field}"),
            Self::Io { path, message } => {
                write!(formatter, "cannot read {}: {message}", path.display())
            }
            Self::Zip(error) => write!(formatter, "the archive cannot be written: {error}"),
        }
    }
}

impl std::error::Error for ExportError {}

/// Bundles a modpack that was already written into a launcher-bundled archive.
///
/// The bundle carries the launcher settings, the list of the servers of the
/// authlib-injector, the modpack under `.hmcl/modpack/`, the fonts of the
/// player and the launcher jar itself, so the archive can install the modpack
/// and the launcher in one step.
pub fn bundle_with_launcher(
    archive: &Path,
    modpack: &Path,
    modpack_type: &str,
    launcher_jar: Option<&Path>,
    launcher_settings: &[u8],
    authlib_injector_servers: &[u8],
    fonts: &[(String, Vec<u8>)],
) -> Result<(), ExportError> {
    let package_name = if modpack_type == "modrinth" {
        "modpack.mrpack"
    } else {
        "modpack.zip"
    };

    let mut writer = ZipWriter::create(archive).map_err(ExportError::Zip)?;

    writer
        .add_file(".hmcl/config/launcher-settings.json", launcher_settings)
        .map_err(ExportError::Zip)?;
    writer
        .add_file(
            ".hmcl/config/authlib-injector-servers.json",
            authlib_injector_servers,
        )
        .map_err(ExportError::Zip)?;
    writer
        .add_path(&format!(".hmcl/modpack/{package_name}"), modpack)
        .map_err(ExportError::Zip)?;

    for (extension, bytes) in fonts {
        writer
            .add_file(&format!(".hmcl/font.{extension}"), bytes)
            .map_err(ExportError::Zip)?;
    }

    if let Some(jar) = launcher_jar {
        let name = jar
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("hmcl.jar");
        writer.add_path(name, jar).map_err(ExportError::Zip)?;
    }

    writer.finish().map_err(ExportError::Zip)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Mark;

    /// Returns a directory of the tests that the other tests do not use.
    fn directory(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("hmcl-rs-export-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Writes a file of the instance.
    fn write(base: &Path, path: &str, text: &str) {
        let file = base.join(path);
        std::fs::create_dir_all(file.parent().expect("the file has a parent"))
            .expect("the directory should be created");
        std::fs::write(file, text).expect("the file should be written");
    }

    /// Returns the settings of a complete export.
    fn info() -> ModpackExportInfo {
        ModpackExportInfo {
            name: "World".to_string(),
            author: "player".to_string(),
            version: "1.0".to_string(),
            description: "A pack".to_string(),
            url: "https://example.com/world".to_string(),
            file_api: Some("https://example.com/pack/".to_string()),
            min_memory: 1024,
            launch_arguments: "-DEMO -width 800".to_string(),
            java_arguments: "-XX:+UseG1GC".to_string(),
            origins: vec![Origin {
                kind: "mcbbs".to_string(),
                id: 42,
            }],
            force_update: true,
            ..ModpackExportInfo::default()
        }
    }

    /// Returns the components of a modded instance.
    fn components() -> Vec<(String, String)> {
        let marks = [
            Mark {
                component: GameComponentType::Forge,
                version: "47.2.0".to_string(),
                from_patch: false,
            },
            Mark {
                component: GameComponentType::OptiFine,
                version: "H9".to_string(),
                from_patch: false,
            },
        ];
        export_addons("1.20.1", marks.iter())
    }

    #[test]
    fn an_export_writes_the_files_the_launcher_keeps() {
        let base = directory("files");
        write(&base, "mods/sodium.jar", "sodium");
        write(&base, "config/a.toml", "a");
        write(&base, "logs/latest.log", "a log");
        write(&base, ".DS_Store", "finder");
        write(&base, "saves/world/level.dat", "level");

        let files = collect_files(&base, &black_list(), &[]).expect("the instance should be read");
        let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["config/a.toml", "mods/sodium.jar", "saves/world/level.dat"]
        );
        assert_eq!(files[0].sha1, Algorithm::Sha1.digest_to_string(b"a"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_files_the_player_selected_are_the_only_ones_written() {
        let base = directory("whitelist");
        write(&base, "mods/sodium.jar", "sodium");
        write(&base, "config/a.toml", "a");

        // The wizard lists the directory and every file it holds, so the walk
        // of the export reaches the file.
        let files = collect_files(
            &base,
            &black_list(),
            &["config".to_string(), "config/a.toml".to_string()],
        )
        .expect("the instance should be read");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "config/a.toml");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_components_name_the_game_and_the_loaders() {
        let components = components();
        assert_eq!(
            components,
            vec![
                ("game".to_string(), "1.20.1".to_string()),
                ("forge".to_string(), "47.2.0".to_string()),
                ("optifine".to_string(), "H9".to_string()),
            ]
        );
    }

    #[test]
    fn the_manifest_of_mcbbs_names_the_files_of_the_archive() {
        let files = vec![ExportFile {
            path: "mods/sodium.jar".to_string(),
            sha1: "abc".to_string(),
        }];
        let manifest = mcbbs_manifest(&info(), "1.20.1", &components(), &files);

        assert_eq!(manifest.manifest_type.as_deref(), Some(MANIFEST_TYPE));
        assert_eq!(manifest.manifest_version, 2);
        assert_eq!(
            manifest.file_api.as_deref(),
            Some("https://example.com/pack")
        );
        assert_eq!(
            manifest.game_version().expect("the game should be named"),
            "1.20.1"
        );
        assert_eq!(
            manifest.files(),
            &[McbbsFile::Addon {
                force: true,
                path: "mods/sodium.jar".to_string(),
                hash: "abc".to_string(),
            }]
        );
        assert_eq!(manifest.launch_info.min_memory, 1024);
        assert_eq!(manifest.launch_arguments(), ["-DEMO", "-width", "800"]);
        assert_eq!(manifest.java_arguments(), ["-XX:+UseG1GC"]);

        let curse = curse_manifest(&info(), "1.20.1", &components());
        assert_eq!(curse.minecraft.version, "1.20.1");
        assert_eq!(curse.minecraft.mod_loaders.len(), 1);
        assert_eq!(curse.minecraft.mod_loaders[0].id, "forge-47.2.0");
        assert!(curse.minecraft.mod_loaders[0].primary);
    }

    #[test]
    fn an_export_of_mcbbs_writes_the_manifests_and_the_files() {
        let base = directory("mcbbs");
        write(&base, "mods/sodium.jar", "sodium");
        let archive = base.join("pack.zip");

        let outcome = write_mcbbs(&base, &archive, &info(), &components(), "1.20.1")
            .expect("the export should be written");
        assert_eq!(outcome.files.len(), 1);

        let names: Vec<String> = zip::entries(&archive)
            .expect("the archive should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert!(names.contains(&"overrides/mods/sodium.jar".to_string()));
        assert!(names.contains(&PACK_META.to_string()));
        assert!(names.contains(&MANIFEST_JSON.to_string()));

        let manifest =
            McbbsModpackManifest::read_from_archive(&archive).expect("the manifest should be read");
        assert_eq!(manifest.name, "World");
        assert_eq!(manifest.files().len(), 1);

        let curse = CurseManifest::read_from_archive(&archive)
            .expect("the manifest of CurseForge should be read");
        assert_eq!(curse.game_version(), "1.20.1");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_export_without_the_server_of_mcbbs_is_refused() {
        let base = directory("no-api");
        let mut info = info();
        info.file_api = None;
        let result = write_mcbbs(
            &base,
            &base.join("pack.zip"),
            &info,
            &components(),
            "1.20.1",
        );
        assert!(matches!(result, Err(ExportError::Missing("file_api"))));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_export_of_a_server_writes_its_manifest() {
        let base = directory("server");
        write(&base, "config/a.toml", "a");
        let archive = base.join("pack.zip");

        write_server(&base, &archive, &info(), &components(), "1.20.1")
            .expect("the export should be written");

        let manifest = ServerModpackManifest::read_from_archive(&archive)
            .expect("the manifest should be read");
        assert_eq!(manifest.name, "World");
        assert_eq!(manifest.files().len(), 1);
        assert_eq!(
            manifest.game_version().expect("the game should be named"),
            "1.20.1"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_export_of_modrinth_names_the_files_the_repository_knows() {
        let base = directory("modrinth");
        write(&base, "mods/sodium.jar", "sodium");
        write(&base, "config/a.toml", "a");
        let archive = base.join("pack.mrpack");

        let remote = |path: &Path, relative: &str| {
            if !relative.starts_with("mods/") {
                return None;
            }
            Some(ModrinthFile {
                path: relative.to_string(),
                hashes: [(
                    "sha512".to_string(),
                    Algorithm::Sha512
                        .digest_file_to_string(path)
                        .expect("the file should be read"),
                )]
                .into_iter()
                .collect(),
                env: None,
                downloads: vec!["https://example.com/sodium.jar".to_string()],
                file_size: 6,
            })
        };

        write_modrinth(&base, &archive, &info(), &components(), "1.20.1", remote)
            .expect("the export should be written");

        let manifest =
            ModrinthManifest::read_from_archive(&archive).expect("the manifest should be read");
        assert_eq!(manifest.name, "World");
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].path, "mods/sodium.jar");
        assert_eq!(manifest.loader_version("forge"), Some("47.2.0"));

        // The file the repository does not know is kept in the archive.
        let names: Vec<String> = zip::entries(&archive)
            .expect("the archive should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert!(names.contains(&"client-overrides/config/a.toml".to_string()));
        assert!(names.contains(&MODRINTH_MANIFEST.to_string()));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_wizard_names_the_formats_with_short_names() {
        assert_eq!(ExportFormat::from_name("mcbbs"), Some(ExportFormat::Mcbbs));
        assert_eq!(
            ExportFormat::from_name("server"),
            Some(ExportFormat::Server)
        );
        assert_eq!(
            ExportFormat::from_name("modrinth"),
            Some(ExportFormat::Modrinth)
        );
        assert_eq!(ExportFormat::from_name("curseforge"), None);
    }

    #[test]
    fn a_bundle_with_the_launcher_carries_the_settings_and_the_modpack() {
        let base = directory("bundle");
        write(&base, "mods/sodium.jar", "sodium");
        let inner = base.join("pack.zip");
        let jar = base.join("hmcl.jar");
        std::fs::write(&jar, "a launcher").expect("the jar should be written");

        write_mcbbs(&base, &inner, &info(), &components(), "1.20.1")
            .expect("the export should be written");

        let archive = base.join("bundle.zip");
        bundle_with_launcher(
            &archive,
            &inner,
            "mcbbs",
            Some(&jar),
            b"settings",
            b"servers",
            &[("ttf".to_string(), b"a font".to_vec())],
        )
        .expect("the bundle should be written");

        let names: Vec<String> = zip::entries(&archive)
            .expect("the archive should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert!(names.contains(&".hmcl/config/launcher-settings.json".to_string()));
        assert!(names.contains(&".hmcl/config/authlib-injector-servers.json".to_string()));
        assert!(names.contains(&".hmcl/modpack/modpack.zip".to_string()));
        assert!(names.contains(&".hmcl/font.ttf".to_string()));
        assert!(names.contains(&"hmcl.jar".to_string()));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_bundle_of_modrinth_names_the_package_after_its_format() {
        let base = directory("bundle-modrinth");
        let inner = base.join("pack.mrpack");
        std::fs::write(&inner, "a pack").expect("the pack should be written");

        let archive = base.join("bundle.zip");
        bundle_with_launcher(&archive, &inner, "modrinth", None, b"{}", b"{}", &[])
            .expect("the bundle should be written");

        let names: Vec<String> = zip::entries(&archive)
            .expect("the archive should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert!(names.contains(&".hmcl/modpack/modpack.mrpack".to_string()));
        // The launcher was not packed, so no jar was written.
        assert_eq!(names.len(), 3);

        let _ = std::fs::remove_dir_all(&base);
    }
}
