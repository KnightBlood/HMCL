//! The modpacks of MCBBS.
//!
//! The launcher of MCBBS writes the manifest of a modpack in
//! `mcbbs.packmeta`; the older archives write it in `manifest.json`, the
//! manifest of CurseForge, because the format was built on it. The manifest
//! names the components of the instance, the libraries the modpack needs and
//! the files the archive holds, and it may name the server the manifest of the
//! newest version is read from.
//!
//! The files of the archive are unpacked like the files of every other
//! modpack; the manifest is what makes a modpack of MCBBS different: the
//! launcher can complete an instance from the server of the author, so the
//! player does not download the archive again when the author updates one
//! file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::download::game::Library;
use crate::util::string::remove_suffix;

use super::configuration::{FileInformation, ModpackConfiguration};
use super::plan::{relative_path, sha1_of, DownloadRequest, UpdatePlan};

/// The type the manifest of MCBBS names itself.
pub const MANIFEST_TYPE: &str = "minecraftModpack";

/// The name of the provider, as the launcher writes it in a configuration.
pub const PROVIDER_NAME: &str = "Mcbbs";

/// The file the manifest of a modpack of today is written in.
pub const PACK_META: &str = "mcbbs.packmeta";

/// The file the manifest of an older modpack is written in.
pub const MANIFEST_JSON: &str = "manifest.json";

/// The name of the patch the libraries of a modpack of MCBBS are kept in.
pub const PATCH_NAME: &str = "mcbbs";

/// The manifest of a modpack of MCBBS.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McbbsModpackManifest {
    /// The type of the manifest, which must be [`MANIFEST_TYPE`].
    #[serde(default, rename = "manifestType")]
    pub manifest_type: Option<String>,
    /// The version of the format of the manifest.
    #[serde(default, rename = "manifestVersion")]
    pub manifest_version: i32,
    /// The name of the modpack.
    #[serde(default)]
    pub name: String,
    /// The version of the modpack.
    #[serde(default)]
    pub version: String,
    /// The author of the modpack.
    #[serde(default)]
    pub author: String,
    /// The description of the modpack, which may hold HTML.
    #[serde(default)]
    pub description: String,
    /// The address the manifest of the newest version is read from.
    #[serde(default, rename = "fileApi")]
    pub file_api: Option<String>,
    /// The page of the modpack.
    #[serde(default)]
    pub url: String,
    /// Whether the launcher updates the instance even when the player asked it
    /// not to.
    #[serde(default, rename = "forceUpdate")]
    pub force_update: bool,
    /// The communities the modpack belongs to.
    ///
    /// The field is written as `origin`, which is the name the launcher of
    /// MCBBS uses for a single community.
    #[serde(default, rename = "origin")]
    pub origins: Vec<Origin>,
    /// The components of the instance.
    #[serde(default)]
    pub addons: Option<Vec<Addon>>,
    /// The libraries the modpack adds to the instance.
    #[serde(default)]
    pub libraries: Vec<Library>,
    /// The files the archive holds and the modpack adds to the instance.
    #[serde(default)]
    pub files: Option<Vec<McbbsFile>>,
    /// The settings that tell which kinds of the files the launcher installs.
    #[serde(default)]
    pub settings: Settings,
    /// The options the game is started with.
    #[serde(default, rename = "launchInfo")]
    pub launch_info: LaunchInfo,
}

impl McbbsModpackManifest {
    /// Reads the manifest of a modpack.
    ///
    /// The manifest is refused when it does not name itself as a modpack or
    /// when the files or the components are missing, because the launcher
    /// cannot tell what to install.
    pub fn read(text: &str) -> Result<Self, McbbsError> {
        let manifest: Self =
            serde_json::from_str(text).map_err(|error| McbbsError::Malformed(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Reads the manifest of a modpack from the archive of the modpack.
    ///
    /// The manifest of a modpack of today is `mcbbs.packmeta`; an archive that
    /// was written before the file existed keeps the manifest in
    /// `manifest.json`.
    pub fn read_from_archive(archive: &Path) -> Result<Self, McbbsError> {
        for name in [PACK_META, MANIFEST_JSON] {
            let Some(data) =
                crate::util::zip::read_entry(archive, name).map_err(McbbsError::Zip)?
            else {
                continue;
            };
            let text = String::from_utf8(data)
                .map_err(|error| McbbsError::Malformed(error.to_string()))?;
            return Self::read(&text);
        }

        Err(McbbsError::MissingManifest)
    }

    /// Refuses a manifest the launcher cannot install.
    pub fn validate(&self) -> Result<(), McbbsError> {
        match self.manifest_type.as_deref() {
            Some(MANIFEST_TYPE) => {}
            Some(other) => return Err(McbbsError::ManifestType(other.to_string())),
            None => return Err(McbbsError::ManifestType(String::new())),
        }

        if self.files.is_none() {
            return Err(McbbsError::MissingFiles);
        }
        if self.addons.is_none() {
            return Err(McbbsError::MissingAddons);
        }

        Ok(())
    }

    /// Returns the files the archive holds.
    pub fn files(&self) -> &[McbbsFile] {
        self.files.as_deref().unwrap_or_default()
    }

    /// Returns the components of the instance.
    pub fn addons(&self) -> &[Addon] {
        self.addons.as_deref().unwrap_or_default()
    }

    /// Returns the version of the game the modpack names.
    ///
    /// The game is one of the components; a modpack that names none cannot be
    /// installed, because the launcher does not know which version of the game
    /// the libraries of the modpack belong to.
    pub fn game_version(&self) -> Result<&str, McbbsError> {
        self.addons()
            .iter()
            .find(|addon| addon.id == crate::game::GameComponentType::Game.patch_id())
            .map(|addon| addon.version.as_str())
            .ok_or(McbbsError::NoGame)
    }

    /// Returns the arguments of the game the modpack adds.
    pub fn launch_arguments(&self) -> &[String] {
        &self.launch_info.launch_arguments
    }

    /// Returns the arguments of the virtual machine the modpack adds.
    pub fn java_arguments(&self) -> &[String] {
        &self.launch_info.java_arguments
    }

    /// Returns the address the files of the modpack are downloaded from.
    ///
    /// The address the manifest names does not end with a slash, so the paths
    /// of the files can be appended to it.
    pub fn file_api(&self) -> Option<&str> {
        self.file_api
            .as_deref()
            .map(|api| remove_suffix(api, "/"))
            .filter(|api| !api.is_empty())
    }

    /// Returns the manifest with other files.
    pub fn with_files(&self, files: Vec<McbbsFile>) -> Self {
        let mut manifest = self.clone();
        manifest.files = Some(files);
        manifest
    }

    /// Returns the file of the modpack in the instance.
    ///
    /// The files are unpacked in the root of the run directory of the instance.
    fn file_path(run_directory: &Path, file: &McbbsFile) -> Option<PathBuf> {
        match file {
            McbbsFile::Addon { path, .. } => relative_path(run_directory, path),
            McbbsFile::Curse { .. } => None,
        }
    }

    /// Returns the digest the manifest tells of a file.
    fn file_hash(file: &McbbsFile) -> Option<&str> {
        match file {
            McbbsFile::Addon { hash, .. } => Some(hash.as_str()).filter(|hash| !hash.is_empty()),
            McbbsFile::Curse { .. } => None,
        }
    }

    /// Returns the path of a file of the modpack in the instance.
    ///
    /// A mod of CurseForge is installed under its file name in the directory
    /// of the mods; the launcher knows the name only after the server of the
    /// mod told it, so a mod whose name is not known has no path yet.
    pub fn file_path_of(&self, run_directory: &Path, file: &McbbsFile) -> Option<PathBuf> {
        match file {
            McbbsFile::Addon { .. } => Self::file_path(run_directory, file),
            McbbsFile::Curse { file_name, .. } => file_name
                .as_deref()
                .filter(|name| !name.is_empty())
                .and_then(|name| relative_path(&run_directory.join("mods"), name)),
        }
    }

    /// Returns the files the launcher keeps of an instance.
    ///
    /// The launcher keeps the digest of the files the archive holds; the files
    /// of CurseForge are not kept, because the launcher reads their digests
    /// from the server of the mod when it completes the instance.
    pub fn overrides(&self) -> Vec<FileInformation> {
        self.files()
            .iter()
            .filter_map(|file| match file {
                McbbsFile::Addon { path, hash, .. } => {
                    Some(FileInformation::new(path.clone(), hash.clone()))
                }
                McbbsFile::Curse { .. } => None,
            })
            .collect()
    }
}

/// A community the modpack belongs to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    /// The kind of the community.
    #[serde(default, rename = "type")]
    pub kind: String,
    /// The identifier of the modpack in the community.
    #[serde(default)]
    pub id: i32,
}

/// A component of the instance the modpack installs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Addon {
    /// The identifier of the component, such as `game` or `forge`.
    #[serde(default)]
    pub id: String,
    /// The version of the component.
    #[serde(default)]
    pub version: String,
}

/// The settings that tell which kinds of the files the launcher installs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Whether the mods of the modpack are installed.
    #[serde(default = "default_true", rename = "install_mods")]
    pub install_mods: bool,
    /// Whether the resource packs of the modpack are installed.
    #[serde(default = "default_true", rename = "install_resourcepack")]
    pub install_resourcepack: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            install_mods: true,
            install_resourcepack: true,
        }
    }
}

/// Returns the default of a flag the manifest leaves out.
fn default_true() -> bool {
    true
}

/// A file the modpack installs.
///
/// The file either is in the archive of the modpack or is downloaded from
/// CurseForge; the manifest names which of the two it is in its `type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McbbsFile {
    /// A file the archive of the modpack holds.
    Addon {
        /// Whether the file may be replaced by the launcher even when the
        /// player changed it.
        #[serde(default)]
        force: bool,
        /// The path of the file in the modpack, relative to the root of the
        /// instance.
        path: String,
        /// The SHA-1 digest of the file.
        hash: String,
    },
    /// A mod of CurseForge.
    Curse {
        /// Whether the file may be replaced by the launcher even when the
        /// player changed it.
        #[serde(default)]
        force: bool,
        /// The identifier of the project of the mod.
        #[serde(default, rename = "projectID")]
        project_id: i32,
        /// The identifier of the file of the mod.
        #[serde(default, rename = "fileID")]
        file_id: i32,
        /// The name of the file, if the launcher knows it.
        #[serde(default, rename = "fileName")]
        file_name: Option<String>,
        /// The address the file is downloaded from, if the manifest names one.
        #[serde(default)]
        url: Option<String>,
    },
}

impl McbbsFile {
    /// Returns whether the launcher may replace the file even when the player
    /// changed it.
    pub fn force(&self) -> bool {
        match self {
            Self::Addon { force, .. } | Self::Curse { force, .. } => *force,
        }
    }

    /// Returns the key the two versions of a modpack are compared by.
    ///
    /// A file of the archive is the same file when it has the same path, and a
    /// mod of CurseForge is the same mod when it has the same project and file
    /// identifiers; the name of the file of a mod may change between the
    /// versions of the modpack.
    pub fn key(&self) -> String {
        match self {
            Self::Addon { path, .. } => format!("addon:{path}"),
            Self::Curse {
                project_id,
                file_id,
                ..
            } => format!("curse:{project_id}:{file_id}"),
        }
    }

    /// Returns the address a mod of CurseForge is downloaded from.
    ///
    /// The manifest of the older modpacks names the address of the mod; the
    /// manifest of today names only the identifiers, and the file is read from
    /// the archive of CurseForge, which the launcher builds from them.
    pub fn curse_url(&self) -> Option<String> {
        match self {
            Self::Curse { url: Some(url), .. } if !url.is_empty() => Some(url.clone()),
            Self::Curse {
                project_id,
                file_id,
                ..
            } => Some(format!(
                "https://www.curseforge.com/minecraft/mc-mods/{project_id}/download/{file_id}/file"
            )),
            Self::Addon { .. } => None,
        }
    }

    /// Returns the file with a name the server of the mod told the launcher.
    pub fn with_file_name(&self, name: impl Into<String>) -> Self {
        match self {
            Self::Curse {
                force,
                project_id,
                file_id,
                url,
                ..
            } => Self::Curse {
                force: *force,
                project_id: *project_id,
                file_id: *file_id,
                file_name: Some(name.into()),
                url: url.clone(),
            },
            other => other.clone(),
        }
    }

    /// Returns the file with the address the server of the mod told the
    /// launcher.
    pub fn with_url(&self, address: impl Into<String>) -> Self {
        match self {
            Self::Curse {
                force,
                project_id,
                file_id,
                file_name,
                ..
            } => Self::Curse {
                force: *force,
                project_id: *project_id,
                file_id: *file_id,
                file_name: file_name.clone(),
                url: Some(address.into()),
            },
            other => other.clone(),
        }
    }

    /// Returns the path of a file of the archive.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Addon { path, .. } => Some(path.as_str()),
            Self::Curse { .. } => None,
        }
    }
}

/// The options the game of the modpack is started with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchInfo {
    /// The smallest amount of the memory the game needs, in mebibytes.
    #[serde(default, rename = "minMemory")]
    pub min_memory: i32,
    /// The versions of Java the modpack supports, if it names them.
    #[serde(default, rename = "supportJava")]
    pub support_java: Option<Vec<i32>>,
    /// The arguments of the game the modpack adds.
    #[serde(default, rename = "launchArgument")]
    pub launch_arguments: Vec<String>,
    /// The arguments of the virtual machine the modpack adds.
    #[serde(default, rename = "javaArgument")]
    pub java_arguments: Vec<String>,
}

/// The files an update of a modpack of MCBBS downloads and removes.
#[derive(Debug, Clone, PartialEq)]
pub struct McbbsUpdate {
    /// The manifest of the modpack with the files the launcher keeps.
    pub manifest: McbbsModpackManifest,
    /// The files the launcher downloads and removes.
    pub plan: UpdatePlan,
}

impl McbbsUpdate {
    /// Returns the configuration of the instance after the update.
    pub fn configuration(&self) -> ModpackConfiguration<McbbsModpackManifest> {
        ModpackConfiguration::new(
            self.manifest.clone(),
            PROVIDER_NAME,
            self.manifest.name.clone(),
            Some(self.manifest.version.clone()),
            self.manifest.overrides(),
        )
    }
}

/// Compares the manifest of an instance with the manifest of the server of the
/// modpack.
///
/// A file is downloaded when the instance does not have it, when the player
/// deleted it, or when the new version of the modpack changed it and the player
/// did not. A file the player changed is kept, because the player expects the
/// change, unless the manifest of the modpack names it as forced. A file the
/// new version does not hold any more is removed.
///
/// A mod of CurseForge whose file name is not known is not downloaded here: the
/// launcher asks the server of the mod for the name first, which the caller
/// does before it downloads the plan.
pub fn plan_update(
    local: &McbbsModpackManifest,
    remote: &McbbsModpackManifest,
    run_directory: &Path,
) -> McbbsUpdate {
    let mut plan = UpdatePlan::default();
    let mut remaining: Vec<McbbsFile> = local.files().to_vec();
    let mut files = Vec::with_capacity(remote.files().len());

    let Some(api) = remote.file_api() else {
        plan.warnings
            .push("the manifest names no server, so no file can be downloaded".to_string());
        return McbbsUpdate {
            manifest: remote.with_files(remote.files().to_vec()),
            plan,
        };
    };

    for file in remote.files() {
        let old = remaining
            .iter()
            .position(|candidate| candidate.key() == file.key())
            .map(|index| remaining.remove(index));

        // The entry the launcher keeps is the one of the new version, but a mod
        // of CurseForge keeps the name the launcher already resolved, because
        // the new version does not name it and the launcher would have to ask
        // the server of the mod again.
        let merged = merge_file(old.as_ref(), file);
        let path = remote.file_path_of(run_directory, &merged);
        let download = match (&old, &path) {
            // The instance does not hold the file yet, and the launcher knows
            // where to write it.
            (None, Some(_)) => true,
            // The instance does not hold the file and the launcher does not
            // know its path; a mod of CurseForge has to be resolved first.
            (None, None) => false,
            (Some(old), Some(path)) => should_download(old, file, path),
            // The launcher cannot tell where the file of the new version is.
            (Some(_), None) => false,
        };

        if download {
            let url = match file {
                McbbsFile::Addon { path, .. } => format!("{api}/overrides/{path}"),
                McbbsFile::Curse { .. } => file.curse_url().unwrap_or_default(),
            };
            plan.downloads.push(DownloadRequest {
                url,
                path: path.expect("a file that is downloaded has a path"),
                sha1: McbbsModpackManifest::file_hash(&merged).map(str::to_string),
            });
        }

        files.push(merged);
    }

    // The files the new version of the modpack does not hold any more are
    // removed; they are the entries the previous version had and the loop above
    // did not take.
    for file in &remaining {
        let Some(path) = local.file_path_of(run_directory, file) else {
            continue;
        };
        if path.is_file() {
            plan.removals.push(path);
        }
    }

    McbbsUpdate {
        manifest: remote.with_files(files),
        plan,
    }
}

/// Returns whether a file the instance holds is downloaded again.
///
/// The digest of the file of the instance is compared with the digest the new
/// version names; a file the player changed is kept, unless the manifest names
/// the file as forced or the launcher does not know the digest the instance
/// had.
fn should_download(old: &McbbsFile, file: &McbbsFile, path: &Path) -> bool {
    if !path.is_file() {
        return true;
    }

    let Some(new_hash) = McbbsModpackManifest::file_hash(file) else {
        return false;
    };

    let Some(actual) = sha1_of(path) else {
        return true;
    };
    if actual.eq_ignore_ascii_case(new_hash) {
        return false;
    }

    let Some(old_hash) = McbbsModpackManifest::file_hash(old) else {
        return true;
    };

    file.force() || old_hash.eq_ignore_ascii_case(&actual)
}

/// Returns the entry the launcher keeps of a file of the new version.
///
/// The entry of a file of the archive is the one of the new version, so the
/// launcher keeps its digest; the entry of a mod of CurseForge is the one the
/// launcher already resolved, because the new version does not name the file
/// name, and the launcher would have to ask the server of the mod again.
fn merge_file(old: Option<&McbbsFile>, file: &McbbsFile) -> McbbsFile {
    match (old, file) {
        (Some(old @ McbbsFile::Curse { .. }), McbbsFile::Curse { .. }) => old.clone(),
        _ => file.clone(),
    }
}

/// The manifest of a modpack of MCBBS the launcher cannot read or install.
#[derive(Debug)]
pub enum McbbsError {
    /// The manifest is not valid JSON.
    Malformed(String),
    /// The manifest names another type, so the archive is not a modpack of
    /// MCBBS.
    ManifestType(String),
    /// The manifest names no file.
    MissingFiles,
    /// The manifest names no component.
    MissingAddons,
    /// The manifest names no version of the game.
    NoGame,
    /// The archive holds no manifest of MCBBS.
    MissingManifest,
    /// The archive cannot be read.
    Zip(crate::util::zip::ZipError),
}

impl std::fmt::Display for McbbsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(message) => {
                write!(formatter, "the manifest of the modpack is malformed: {message}")
            }
            Self::ManifestType(kind) => write!(
                formatter,
                "the manifest names the type `{kind}`, but a modpack of MCBBS is `{MANIFEST_TYPE}`"
            ),
            Self::MissingFiles => {
                formatter.write_str("the manifest of the modpack names no file")
            }
            Self::MissingAddons => {
                formatter.write_str("the manifest of the modpack names no component")
            }
            Self::NoGame => {
                formatter.write_str("the manifest of the modpack names no version of Minecraft")
            }
            Self::MissingManifest => write!(
                formatter,
                "the archive holds neither `{PACK_META}` nor `{MANIFEST_JSON}`, so it is not a modpack of MCBBS"
            ),
            Self::Zip(error) => write!(formatter, "the archive cannot be read: {error}"),
        }
    }
}

impl std::error::Error for McbbsError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::digest::Algorithm;

    /// Returns a directory of the tests that the other tests do not use.
    fn directory(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("hmcl-rs-mcbbs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Returns a manifest with one component and the files the test names.
    fn manifest(files: Vec<McbbsFile>) -> McbbsModpackManifest {
        McbbsModpackManifest {
            manifest_type: Some(MANIFEST_TYPE.to_string()),
            manifest_version: 2,
            name: "World".to_string(),
            version: "1.0".to_string(),
            author: "Author".to_string(),
            file_api: Some("https://example.com/pack".to_string()),
            addons: Some(vec![Addon {
                id: "game".to_string(),
                version: "1.20.1".to_string(),
            }]),
            files: Some(files),
            launch_info: LaunchInfo {
                min_memory: 2048,
                launch_arguments: vec!["--demo".to_string()],
                java_arguments: vec!["-Dtest=1".to_string()],
                ..LaunchInfo::default()
            },
            ..McbbsModpackManifest::default()
        }
    }

    /// Writes a file of the instance with the digest of the text.
    fn write_file(base: &Path, path: &str, text: &str) -> String {
        let file = base.join(path);
        std::fs::create_dir_all(file.parent().expect("the file has a parent"))
            .expect("the directory should be created");
        std::fs::write(&file, text).expect("the file should be written");
        Algorithm::Sha1.digest_to_string(text.as_bytes())
    }

    /// Returns a file of the archive with the digest of the text.
    fn addon(path: &str, text: &str) -> McbbsFile {
        McbbsFile::Addon {
            force: false,
            path: path.to_string(),
            hash: Algorithm::Sha1.digest_to_string(text.as_bytes()),
        }
    }

    #[test]
    fn the_manifest_of_a_modpack_is_read() {
        let text = r#"{
            "manifestType": "minecraftModpack",
            "manifestVersion": 2,
            "name": "World",
            "version": "1.0",
            "author": "Author",
            "description": "A modpack",
            "fileApi": "https://example.com/pack/",
            "url": "https://example.com/world",
            "forceUpdate": true,
            "origin": [ { "type": "curseforge", "id": 1234 } ],
            "addons": [
                { "id": "game", "version": "1.20.1" },
                { "id": "forge", "version": "47.1.0" }
            ],
            "libraries": [ { "name": "com.example:library:1.0" } ],
            "files": [
                { "type": "addon", "force": true, "path": "config/a.toml", "hash": "abc" },
                { "type": "curse", "projectID": 1, "fileID": 2, "fileName": "sodium.jar" }
            ],
            "settings": { "install_mods": false },
            "launchInfo": {
                "minMemory": 2048,
                "supportJava": [ 17, 21 ],
                "launchArgument": [ "--demo" ],
                "javaArgument": [ "-Dtest=1" ]
            }
        }"#;

        let manifest = McbbsModpackManifest::read(text).expect("the manifest should be read");
        assert_eq!(
            manifest.game_version().expect("the game should be named"),
            "1.20.1"
        );
        assert_eq!(manifest.file_api(), Some("https://example.com/pack"));
        assert!(manifest.force_update);
        assert_eq!(manifest.origins.len(), 1);
        assert_eq!(manifest.libraries.len(), 1);
        assert!(!manifest.settings.install_mods);
        assert!(manifest.settings.install_resourcepack);
        assert_eq!(manifest.launch_arguments(), ["--demo"]);
        assert_eq!(manifest.java_arguments(), ["-Dtest=1"]);

        let files = manifest.files();
        assert!(files[0].force());
        assert_eq!(files[0].path(), Some("config/a.toml"));
        assert_eq!(
            files[1].curse_url().as_deref(),
            Some("https://www.curseforge.com/minecraft/mc-mods/1/download/2/file")
        );
        // The launcher keeps the files of the archive only.
        assert_eq!(manifest.overrides().len(), 1);
    }

    #[test]
    fn a_manifest_of_another_format_is_refused() {
        let text = r#"{ "manifestType": "minecraftModpack", "files": [], "addons": [] }"#;
        assert!(McbbsModpackManifest::read(text).is_ok());

        let text = r#"{ "manifestType": "curseforge", "files": [], "addons": [] }"#;
        assert!(matches!(
            McbbsModpackManifest::read(text),
            Err(McbbsError::ManifestType(_))
        ));

        let text = r#"{ "manifestType": "minecraftModpack", "addons": [] }"#;
        assert!(matches!(
            McbbsModpackManifest::read(text),
            Err(McbbsError::MissingFiles)
        ));
    }

    #[test]
    fn a_modpack_that_names_no_game_is_refused() {
        let mut manifest = manifest(Vec::new());
        manifest.addons = Some(vec![Addon {
            id: "forge".to_string(),
            version: "47.1.0".to_string(),
        }]);
        assert!(matches!(manifest.game_version(), Err(McbbsError::NoGame)));
    }

    #[test]
    fn the_manifest_of_an_archive_is_read_from_either_file() {
        let base = directory("archive");

        let text = serde_json::to_string(&manifest(vec![addon("a.txt", "a")]))
            .expect("the manifest should be written");

        let meta_archive = base.join("meta.zip");
        let mut writer = crate::util::zip::ZipWriter::create(&meta_archive)
            .expect("the archive should be created");
        writer
            .add_file(PACK_META, text.as_bytes())
            .expect("the manifest should be added");
        writer.finish().expect("the archive should be written");

        let read = McbbsModpackManifest::read_from_archive(&meta_archive)
            .expect("the manifest should be read");
        assert_eq!(read.name, "World");

        let old_archive = base.join("old.zip");
        let mut writer = crate::util::zip::ZipWriter::create(&old_archive)
            .expect("the archive should be created");
        writer
            .add_file(MANIFEST_JSON, text.as_bytes())
            .expect("the manifest should be added");
        writer.finish().expect("the archive should be written");

        let read = McbbsModpackManifest::read_from_archive(&old_archive)
            .expect("the manifest should be read");
        assert_eq!(read.name, "World");

        let empty_archive = base.join("empty.zip");
        let mut writer = crate::util::zip::ZipWriter::create(&empty_archive)
            .expect("the archive should be created");
        writer
            .add_file("a.txt", b"a")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");
        assert!(matches!(
            McbbsModpackManifest::read_from_archive(&empty_archive),
            Err(McbbsError::MissingManifest)
        ));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_new_file_is_downloaded_and_an_unchanged_file_is_kept() {
        let base = directory("unchanged");
        let text = "the file";
        write_file(&base, "config/a.toml", text);

        let file = addon("config/a.toml", text);
        let local = manifest(vec![file.clone()]);
        let remote = manifest(vec![file.clone(), addon("config/b.toml", "other")]);

        let update = plan_update(&local, &remote, &base);

        assert_eq!(update.plan.downloads.len(), 1);
        assert_eq!(
            update.plan.downloads[0].url,
            "https://example.com/pack/overrides/config/b.toml"
        );
        assert_eq!(update.plan.downloads[0].path, base.join("config/b.toml"));
        assert!(update.plan.removals.is_empty());
        assert_eq!(update.manifest.files().len(), 2);
        assert_eq!(update.configuration().kind, "Mcbbs");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_new_version_changed_is_downloaded_when_the_player_did_not_change_it() {
        let base = directory("changed");
        let old_hash = write_file(&base, "config/a.toml", "old");
        let new = addon("config/a.toml", "new");

        let local = manifest(vec![McbbsFile::Addon {
            force: false,
            path: "config/a.toml".to_string(),
            hash: old_hash,
        }]);
        let remote = manifest(vec![new.clone()]);

        let update = plan_update(&local, &remote, &base);
        assert_eq!(update.plan.downloads.len(), 1);
        assert_eq!(update.plan.downloads[0].sha1.as_deref(), mcbb_hash(&new));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Returns the digest a file of the archive names.
    fn mcbb_hash(file: &McbbsFile) -> Option<&str> {
        McbbsModpackManifest::file_hash(file)
    }

    #[test]
    fn a_file_the_player_changed_is_kept_unless_it_is_forced() {
        let base = directory("changed-by-player");
        // The player changed the file, so the digest of the instance is not the
        // digest the old version of the modpack named.
        write_file(&base, "config/a.toml", "changed by the player");

        let local = manifest(vec![McbbsFile::Addon {
            force: false,
            path: "config/a.toml".to_string(),
            hash: Algorithm::Sha1.digest_to_string(b"the original"),
        }]);
        let remote = manifest(vec![addon("config/a.toml", "new")]);

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.downloads.is_empty());

        // A file the manifest names as forced is replaced even when the player
        // changed it.
        let remote = manifest(vec![McbbsFile::Addon {
            force: true,
            path: "config/a.toml".to_string(),
            hash: Algorithm::Sha1.digest_to_string(b"new"),
        }]);
        let update = plan_update(&local, &remote, &base);
        assert_eq!(update.plan.downloads.len(), 1);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_player_deleted_is_downloaded_again() {
        let base = directory("deleted");
        let file = addon("config/a.toml", "the file");

        let local = manifest(vec![file.clone()]);
        let remote = manifest(vec![file]);

        let update = plan_update(&local, &remote, &base);
        assert_eq!(update.plan.downloads.len(), 1);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_new_version_dropped_is_removed() {
        let base = directory("dropped");
        write_file(&base, "config/a.toml", "the file");
        write_file(&base, "config/b.toml", "the other file");

        let local = manifest(vec![
            addon("config/a.toml", "the file"),
            addon("config/b.toml", "the other file"),
        ]);
        let remote = manifest(vec![addon("config/a.toml", "the file")]);

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.downloads.is_empty());
        assert_eq!(update.plan.removals, vec![base.join("config/b.toml")]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mod_of_curseforge_keeps_the_name_the_launcher_resolved() {
        let base = directory("curse");
        let resolved = McbbsFile::Curse {
            force: false,
            project_id: 1,
            file_id: 2,
            file_name: Some("sodium.jar".to_string()),
            url: Some("https://example.com/sodium.jar".to_string()),
        };
        let unresolved = McbbsFile::Curse {
            force: false,
            project_id: 1,
            file_id: 2,
            file_name: None,
            url: None,
        };

        let local = manifest(vec![resolved.clone()]);
        let remote = manifest(vec![unresolved]);

        let update = plan_update(&local, &remote, &base);
        assert_eq!(update.manifest.files(), std::slice::from_ref(&resolved));
        // The launcher knows the name of the file, so it can tell whether the
        // file of the instance is the one the modpack wants.
        assert_eq!(update.plan.downloads.len(), 1);
        assert_eq!(update.plan.downloads[0].path, base.join("mods/sodium.jar"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_modpack_without_a_server_removes_and_downloads_nothing() {
        let base = directory("no-server");
        let local = manifest(vec![addon("config/a.toml", "the file")]);
        let mut remote = manifest(vec![addon("config/b.toml", "other")]);
        remote.file_api = None;

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.downloads.is_empty());
        assert!(update.plan.removals.is_empty());
        assert_eq!(update.plan.warnings.len(), 1);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_path_that_leaves_the_instance_is_not_written() {
        let base = directory("outside");
        let local = manifest(Vec::new());
        let remote = manifest(vec![addon("../outside.txt", "the file")]);

        let update = plan_update(&local, &remote, &base);
        // The file is not downloaded, because the launcher does not know where
        // to write it.
        assert!(update.plan.downloads.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }
}
