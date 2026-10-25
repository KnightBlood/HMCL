//! The installation of the mod loaders that ship an installer.
//!
//! Forge and NeoForge do not publish the changes they make to a version of the
//! game as metadata the way the loaders of the Fabric family do; they ship an
//! installer that carries the files of the loader and a list of the tasks that
//! patch the game with them. The launcher reads that installer, runs the tasks
//! it tells and keeps the version of the game the installer describes as a
//! patch of the instance.
//!
//! Two shapes of the installer exist. The installers of the old shape carry the
//! file of the loader itself, and the launcher only unpacks it; the installers
//! of the new shape carry a list of the tasks the installer of the loader asks
//! the launcher to run, and the tasks patch the game with a program of the
//! loader. Which shape an installer has is told by the keys of the
//! `install_profile.json` it carries.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::game::{Artifact, InstanceManifest, InstancePatch, Library, VersionMetadata};
use super::library_cache::library_path_of;
use crate::error::Error;
use crate::game::component::{GameComponentType, FORGE_OPTIFINE_MAIN};
use crate::util::digest::Algorithm;
use crate::util::string::{remove_prefix, remove_suffix};
use crate::util::versioning::GameVersionNumber;
use crate::util::zip;

/// The separator of the paths of the system the launcher runs on.
const MAIN_SEPARATOR: &str = if cfg!(windows) { "\\" } else { "/" };

/// The priority the patches of the loaders that ship an installer are written
/// with.
pub const FORGE_PATCH_PRIORITY: i32 = 30000;

/// The shape an installer of a mod loader is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerFormat {
    /// The installer carries the tasks that patch the game.
    Processor,
    /// The installer carries the file of the loader itself.
    Legacy,
}

/// The keys of the `install_profile.json` an installer carries.
///
/// The installers of the old shape hold the file of the loader and the version
/// of the game the installer makes; the installers of the new shape hold the
/// number of the shape of the installer and the version of the game the
/// installer supports.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallProfileKeys {
    /// The number of the shape of the installer, if the installer has one.
    pub spec: Option<i64>,
    /// The version of the game the installer supports, if the installer names
    /// it at its root.
    pub minecraft: Option<String>,
    /// The file of the installer the old shape carries, if there is one.
    pub install: Option<LegacyInstall>,
    /// The version of the game the old shape makes, if there is one.
    #[serde(rename = "versionInfo")]
    pub version_info: Option<InstanceManifest>,
    /// The name of the profile the installer carries, if it names one.
    #[serde(default)]
    pub profile: Option<String>,
}

/// The file of the loader an installer of the old shape carries.
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyInstall {
    /// The name of the library the file is kept as.
    pub path: String,
    /// The path of the file in the installer.
    #[serde(rename = "filePath")]
    pub file_path: String,
    /// The version of the game the file belongs to.
    pub minecraft: String,
}

/// The `install_profile.json` an installer of the new shape carries.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessorInstallProfile {
    /// The version of the game the installer supports.
    pub minecraft: String,
    /// The path of the version of the game the installer makes, in the
    /// installer.
    pub json: String,
    /// The version of the loader.
    #[serde(default)]
    pub version: Option<String>,
    /// The name of the library the file of the loader is kept as.
    #[serde(default)]
    pub path: Option<String>,
    /// The libraries the tasks of the installer need.
    #[serde(default)]
    pub libraries: Vec<Library>,
    /// The tasks the installer asks the launcher to run.
    #[serde(default)]
    pub processors: Vec<Processor>,
    /// The data the tasks are given, by name.
    #[serde(default)]
    pub data: BTreeMap<String, Datum>,
}

/// A task an installer of the new shape asks the launcher to run.
#[derive(Debug, Clone, Deserialize)]
pub struct Processor {
    /// The sides of the game the task runs on; the task runs on every side when
    /// the installer names none.
    #[serde(default)]
    pub sides: Option<Vec<String>>,
    /// The name of the library that holds the program of the task.
    pub jar: String,
    /// The names of the libraries the program of the task needs.
    #[serde(default)]
    pub classpath: Vec<String>,
    /// The arguments the program of the task is given.
    #[serde(default)]
    pub args: Vec<String>,
    /// The files the task writes, with the digest of every file.
    #[serde(default)]
    pub outputs: BTreeMap<String, String>,
}

/// The data a task of an installer is given, by the side of the game.
#[derive(Debug, Clone, Deserialize)]
pub struct Datum {
    /// The value the client is given.
    #[serde(default)]
    pub client: Option<String>,
}

/// A task of an installer, with its arguments read.
#[derive(Debug, Clone)]
pub struct ResolvedProcessor {
    /// The file of the program of the task.
    pub jar: PathBuf,
    /// The files the program of the task needs, the file of the program last.
    pub classpath: Vec<PathBuf>,
    /// The arguments the program is given.
    pub args: Vec<String>,
    /// The files the task writes, with the digest of every file.
    pub outputs: BTreeMap<String, String>,
}

/// The words a task of an installer is asked for.
#[derive(Debug, Clone, Default)]
pub struct ProcessorOptions {
    /// The words, by the name the `--` names.
    pub words: BTreeMap<String, String>,
}

impl ProcessorOptions {
    /// Reads the words of the arguments of a task.
    ///
    /// A word that follows another word without a name is not read, because the
    /// launcher only tells the words the task names.
    pub fn parse(args: &[String], tokens: &BTreeMap<String, String>) -> Result<Self, Error> {
        let mut words = BTreeMap::new();
        let mut name = None;

        for arg in args {
            if let Some(rest) = arg.strip_prefix("--") {
                if let Some(name) = name.take() {
                    words.insert(name, String::new());
                }
                name = Some(rest.to_string());
            } else if let Some(name) = name.take() {
                words.insert(name, replace_tokens(tokens, arg)?);
            }
        }
        if let Some(name) = name {
            words.insert(name, String::new());
        }

        Ok(Self { words })
    }

    /// Returns the word a name holds.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.words.get(name).map(String::as_str)
    }
}

/// Reads the keys of the `install_profile.json` an installer carries.
///
/// The launcher reads the keys before it reads the profile itself, because the
/// shape of the installer is told by which keys it holds.
pub fn read_profile_keys(installer: &Path) -> Result<InstallProfileKeys, Error> {
    let text = read_installer_text(installer, "install_profile.json")?;
    serde_json::from_str(&text)
        .map_err(|error| Error::invalid_format(installer, format!("install_profile.json: {error}")))
}

/// Reads the version of the game an installer of the new shape makes.
pub fn read_processor_profile(installer: &Path) -> Result<ProcessorInstallProfile, Error> {
    let text = read_installer_text(installer, "install_profile.json")?;
    serde_json::from_str(&text)
        .map_err(|error| Error::invalid_format(installer, format!("install_profile.json: {error}")))
}

/// Returns the shape an installer is written in.
///
/// An installer that names the number of its shape is written in the new
/// shape; an installer that names the file of the loader and the version of
/// the game it makes is written in the old shape. An installer that names
/// neither is refused.
pub fn detect_format(installer: &Path) -> Result<InstallerFormat, Error> {
    let keys = read_profile_keys(installer)?;

    if keys.spec.is_some() {
        Ok(InstallerFormat::Processor)
    } else if keys.install.is_some() && keys.version_info.is_some() {
        Ok(InstallerFormat::Legacy)
    } else {
        Err(Error::invalid_format(
            installer,
            "the installer names neither the tasks of the loader nor the file of the loader",
        ))
    }
}

/// Returns the version of the game an installer is made for.
///
/// The version is read from the place the shape of the installer names it.
pub fn installer_game_version(installer: &Path) -> Result<String, Error> {
    let keys = read_profile_keys(installer)?;
    match (keys.minecraft, keys.install) {
        (Some(version), _) => Ok(version),
        (None, Some(install)) => Ok(install.minecraft),
        _ => Err(Error::invalid_format(
            installer,
            "the installer names no version of the game",
        )),
    }
}

/// Returns the name of the loader an installer of the old shape carries.
pub fn legacy_library_name(installer: &Path) -> Result<String, Error> {
    let keys = read_profile_keys(installer)?;
    keys.install.map(|install| install.path).ok_or_else(|| {
        Error::invalid_format(installer, "the installer carries no file of the loader")
    })
}

/// Returns the version of the game an installer of the old shape makes.
pub fn legacy_version_info(installer: &Path) -> Result<InstanceManifest, Error> {
    let keys = read_profile_keys(installer)?;
    keys.version_info.ok_or_else(|| {
        Error::invalid_format(installer, "the installer carries no version of the game")
    })
}

/// Replaces the words of the arguments of a task with the values they name.
///
/// A word is a value, the name of a value the data of the installer holds or
/// the path of a file; the words are read the way the installers of the loaders
/// read them, so the tasks of an installer are given the values the installer
/// expected.
pub fn replace_tokens(tokens: &BTreeMap<String, String>, value: &str) -> Result<String, Error> {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        match character {
            '\\' => {
                let escaped = chars.next().map(|(_, escaped)| escaped).ok_or_else(|| {
                    Error::invalid_format("the pattern", "the escape is not closed")
                })?;
                result.push(escaped);
            }
            opening @ ('{' | '\'') => {
                let closing = if opening == '{' { '}' } else { '\'' };
                let mut key = String::new();
                let mut closed = false;

                while let Some((_, character)) = chars.next() {
                    match character {
                        '\\' => {
                            let escaped =
                                chars.next().map(|(_, escaped)| escaped).ok_or_else(|| {
                                    Error::invalid_format("the pattern", "the escape is not closed")
                                })?;
                            key.push(escaped);
                        }
                        character if character == closing => {
                            closed = true;
                            break;
                        }
                        character => key.push(character),
                    }
                }

                // The pattern the word names is not closed, so the word the
                // rest of the value holds is not known.
                if !closed {
                    return Err(Error::invalid_format(
                        "the pattern",
                        format!("the {opening} of the value is not closed"),
                    ));
                }

                if opening == '\'' {
                    result.push_str(&key);
                } else {
                    let value = tokens.get(&key).ok_or_else(|| {
                        Error::invalid_format("the pattern", format!("the key {key} is not known"))
                    })?;
                    result.push_str(value);
                }
            }
            _ => {
                let _ = index;
                result.push(character);
            }
        }
    }

    Ok(result)
}

/// Reads a value the arguments of a task name.
///
/// The three shapes of a value are read: the name of a value the data of the
/// installer holds, a text and the path of a file. Any other value is read as
/// the text of the words it holds.
pub fn parse_literal(
    literal: &str,
    tokens: &BTreeMap<String, String>,
    layout: &impl crate::game::RepositoryLayout,
) -> Result<String, Error> {
    let literal = literal.trim();

    if let Some(key) = literal
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    {
        return tokens.get(key).cloned().ok_or_else(|| {
            Error::invalid_format("the pattern", format!("the key {key} is not known"))
        });
    }

    if let Some(text) = literal
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
    {
        return Ok(text.to_string());
    }

    if let Some(descriptor) = literal
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        let artifact = Artifact::from_descriptor(descriptor)
            .map_err(|reason| Error::invalid_format(descriptor, reason))?;
        return Ok(layout
            .artifact_file(&artifact)
            .to_string_lossy()
            .into_owned());
    }

    replace_tokens(tokens, literal)
}

/// Returns the values the tasks of an installer are given.
///
/// The installer names the files it carries that the tasks are given, and the
/// launcher writes those files next to the tasks instead of telling the tasks
/// where the installer is kept.
pub fn data_tokens(
    profile: &ProcessorInstallProfile,
    installer: &Path,
    temporary: &Path,
) -> Result<BTreeMap<String, String>, Error> {
    let mut tokens = BTreeMap::new();

    for (name, datum) in &profile.data {
        let Some(client) = &datum.client else {
            continue;
        };

        let value = parse_literal(client, &BTreeMap::new(), &InstallerLayout)?;
        let path = temporary.join(name.replace(['/', '\\'], "_"));
        if let Some(bytes) = zip::read_entry(installer, &value)? {
            if let Some(directory) = path.parent() {
                std::fs::create_dir_all(directory).map_err(|error| Error::io(directory, error))?;
            }
            std::fs::write(&path, bytes).map_err(|error| Error::io(&path, error))?;
        }
        tokens.insert(name.clone(), path.to_string_lossy().into_owned());
    }

    Ok(tokens)
}

/// Returns the words of the tasks of an installer that download the mappings
/// of the game.
///
/// The installers of the loaders ask the launcher to download the mappings of
/// the game with a task of their own, but the launcher already knows where the
/// mappings of a version of the game are kept, so the task is replaced by a
/// download the launcher does itself.
pub fn is_mojang_mappings_task(options: &ProcessorOptions) -> bool {
    options.get("task") == Some("DOWNLOAD_MOJMAPS") && options.get("side") == Some("client")
}

/// Returns the version of the game and the file the mappings of a task of an
/// installer are written to.
pub fn mojang_mappings_target(options: &ProcessorOptions) -> Option<(String, PathBuf)> {
    let version = options.get("version")?;
    let output = options.get("output")?;
    Some((version.to_string(), PathBuf::from(output)))
}

/// Returns the digest of the mappings of a version of the game.
pub fn client_mappings_digest(manifest: &VersionMetadata) -> Option<String> {
    manifest
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.client_mappings.as_ref())
        .and_then(|mappings| mappings.sha1())
        .map(str::to_string)
}

/// Returns the files the tasks of an installer need.
///
/// The tasks name the files they are given as the names of the libraries they
/// are kept as, so the launcher reads the names the way the repositories of
/// Java write them.
pub fn resolve_processor(
    processor: &Processor,
    profile: &ProcessorInstallProfile,
    tokens: &BTreeMap<String, String>,
    libraries_directory: &Path,
    installer: &Path,
) -> Result<ResolvedProcessor, Error> {
    let jar = Artifact::from_descriptor(&processor.jar)
        .map_err(|reason| Error::invalid_format(&processor.jar, reason))?;
    let jar = installer_artifact_file(&jar, profile, libraries_directory, installer)?;

    let mut classpath = Vec::with_capacity(processor.classpath.len() + 1);
    for descriptor in &processor.classpath {
        let artifact = Artifact::from_descriptor(descriptor)
            .map_err(|reason| Error::invalid_format(descriptor, reason))?;
        let file = installer_artifact_file(&artifact, profile, libraries_directory, installer)?;
        if !file.is_file() {
            return Err(Error::io(&file, std::io::ErrorKind::NotFound.into()));
        }
        classpath.push(file);
    }
    classpath.push(jar.clone());

    let mut args = Vec::with_capacity(processor.args.len());
    for arg in &processor.args {
        args.push(parse_literal(arg, tokens, &InstallerLayout)?);
    }

    let mut outputs = BTreeMap::new();
    for (key, value) in &processor.outputs {
        let key = parse_literal(key, tokens, &InstallerLayout)?;
        let value = parse_literal(value, tokens, &InstallerLayout)?;
        outputs.insert(key, value);
    }

    Ok(ResolvedProcessor {
        jar,
        classpath,
        args,
        outputs,
    })
}

/// Returns the files the installer carries that the libraries are kept as.
///
/// The libraries the tasks of an installer need are carried by the installer
/// itself, so the launcher writes them to the directory of the libraries before
/// it runs the tasks.
pub fn copy_installer_libraries(
    profile: &ProcessorInstallProfile,
    installer: &Path,
    libraries_directory: &Path,
) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::with_capacity(profile.libraries.len());

    for library in &profile.libraries {
        let source = format!("maven/{}", library_path_of(library));
        if zip::read_entry(installer, &source)?.is_none() {
            continue;
        }

        let path = libraries_directory.join(library_path_of(library));
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory).map_err(|error| Error::io(directory, error))?;
        }
        if let Some(bytes) = zip::read_entry(installer, &source)? {
            std::fs::write(&path, bytes).map_err(|error| Error::io(&path, error))?;
        }
        files.push(path);
    }

    if let Some(path) = &profile.path {
        let artifact = Artifact::from_descriptor(path)
            .map_err(|reason| Error::invalid_format(path, reason))?;
        let source = format!("maven/{}", artifact.path());
        if let Some(bytes) = zip::read_entry(installer, &source)? {
            let destination = libraries_directory.join(artifact.path());
            if let Some(directory) = destination.parent() {
                std::fs::create_dir_all(directory).map_err(|error| Error::io(directory, error))?;
            }
            std::fs::write(&destination, bytes).map_err(|error| Error::io(&destination, error))?;
            files.push(destination);
        }
    }

    Ok(files)
}

/// Returns the libraries the tasks of an installer need.
///
/// The libraries of the installer are read as the libraries of a version of the
/// game, so the launcher downloads the ones the installer does not carry.
pub fn profile_libraries(profile: &ProcessorInstallProfile) -> Vec<Library> {
    profile.libraries.clone()
}

/// Returns the patch an installer of the new shape makes.
pub fn processor_patch(
    manifest: &InstanceManifest,
    self_version: &str,
    component: GameComponentType,
) -> InstancePatch {
    InstancePatch {
        id: Some(component.patch_id().to_string()),
        version: Some(self_version.to_string()),
        priority: FORGE_PATCH_PRIORITY,
        ..InstancePatch::from_manifest(manifest)
    }
}

/// Returns the patch an installer of the old shape makes.
pub fn legacy_patch(
    manifest: &InstanceManifest,
    self_version: &str,
    component: GameComponentType,
) -> InstancePatch {
    InstancePatch {
        id: Some(component.patch_id().to_string()),
        version: Some(self_version.to_string()),
        priority: FORGE_PATCH_PRIORITY,
        ..InstancePatch::from_manifest(manifest)
    }
}

/// Returns the name the version of the loader is written with.
///
/// The installers name their version with the version of the game they are made
/// for; the launcher writes the name of the loader without it.
pub fn modify_version(game_version: &str, version: &str) -> String {
    let version = version.replace(game_version, "");
    let version = version.trim();
    let version = remove_suffix(version, "-");
    let version = remove_prefix(version, "-");
    let version = remove_suffix(version, "_");
    let version = remove_prefix(version, "_");
    remove_prefix(version, "forge-").to_string()
}

/// Returns the name the version of NeoForge is written with, for the
/// installers of the old shape.
pub fn modify_neoforge_old_version(game_version: &str, version: &str) -> String {
    let version = version.replace(game_version, "");
    let version = version.trim();
    let version = remove_suffix(version, "-");
    let version = remove_prefix(version, "-");
    let version = remove_suffix(version, "_");
    remove_prefix(version, "_").to_string()
}

/// Returns the name the version of NeoForge is written with, for the
/// installers of the new shape.
pub fn modify_neoforge_new_version(version: &str) -> String {
    remove_prefix(&version.replace("neoforge", ""), "-").to_string()
}

/// Returns the name of the patch of NeoForge the launcher writes.
///
/// The installers of the old shape of NeoForge name their patch after Forge,
/// because NeoForge began as a part of Forge; the launcher writes the patch
/// after NeoForge.
pub fn neoforge_patch_id(patch: InstancePatch) -> Result<InstancePatch, Error> {
    let Some(version) = patch.version.clone() else {
        return Err(Error::invalid_format(
            "the patch",
            "the version of the loader is not known",
        ));
    };

    if patch.id.as_deref() != Some(GameComponentType::Forge.patch_id()) {
        return Err(Error::invalid_format(
            "the patch",
            "the patch is not the patch of Forge",
        ));
    }

    Ok(InstancePatch {
        id: Some(GameComponentType::NeoForge.patch_id().to_string()),
        version: Some(remove_prefix(&version.replace("forge", ""), "-").to_string()),
        ..patch
    })
}

/// Returns whether the installer is the installer of NeoForge.
///
/// The installers of the old shape of NeoForge name their profile after Forge
/// and carry the file that tells that they are the installers of NeoForge; the
/// installers of the new shape name their profile after NeoForge.
pub fn is_neoforge_installer(installer: &Path, profile_name: Option<&str>) -> bool {
    if profile_name == Some(GameComponentType::NeoForge.patch_id()) {
        return true;
    }

    profile_name == Some(GameComponentType::Forge.patch_id())
        && (zip::read_entry(installer, "META-INF/NEOFORGE.RSA")
            .ok()
            .flatten()
            .is_some()
            || read_installer_text(installer, "install_profile.json")
                .map(|text| text.contains("neoforge"))
                .unwrap_or(false))
}

/// Returns the name of the profile an installer names.
pub fn profile_name(installer: &Path) -> Option<String> {
    read_profile_keys(installer)
        .ok()
        .and_then(|keys| keys.profile)
}

/// Returns whether the version of the game is started the way Forge expects.
///
/// The versions of the game 1.13 and after are not started through the wrapper
/// of the game, so the launcher checks the version of the game before it
/// installs Forge into it.
pub fn check_launch_wrapper(main_class: Option<&str>, game_version: &str) -> Result<(), Error> {
    // The versions of the game before 1.13 are started through the wrapper of
    // the game anyway, so the launcher does not check them.
    if GameVersionNumber::compare("1.13", game_version) == std::cmp::Ordering::Greater {
        return Ok(());
    }

    let supported = main_class
        .map(|main_class| FORGE_OPTIFINE_MAIN.contains(&main_class))
        .unwrap_or(false);
    if supported {
        Ok(())
    } else {
        Err(Error::unsupported(
            "the version of the game is not started the way Forge expects",
        ))
    }
}

/// Returns the digest of a file.
pub fn digest_of(path: &Path) -> Result<String, Error> {
    Algorithm::Sha1.digest_file_to_string(path)
}

/// Returns the file of a library an installer carries.
///
/// A file the installer does not carry is kept in the directory of the
/// libraries under the name the installer gives it.
fn installer_artifact_file(
    artifact: &Artifact,
    profile: &ProcessorInstallProfile,
    libraries_directory: &Path,
    installer: &Path,
) -> Result<PathBuf, Error> {
    let path = libraries_directory.join(artifact.path());
    if path.is_file() {
        return Ok(path);
    }

    let source = format!("maven/{}", artifact.path());
    if let Some(bytes) = zip::read_entry(installer, &source)? {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory).map_err(|error| Error::io(directory, error))?;
        }
        std::fs::write(&path, bytes).map_err(|error| Error::io(&path, error))?;
        return Ok(path);
    }

    let _ = profile;
    Ok(path)
}

/// Reads a file of an installer as text.
fn read_installer_text(installer: &Path, name: &str) -> Result<String, Error> {
    match zip::read_entry(installer, name)? {
        Some(bytes) => String::from_utf8(bytes)
            .map_err(|_| Error::invalid_format(installer, format!("{name} is not text"))),
        None => Err(Error::invalid_format(
            installer,
            format!("{name} does not exist"),
        )),
    }
}

/// The layout the tasks of an installer read the paths of the files with.
///
/// The tasks of an installer name the files they are given as the names of the
/// libraries they are kept as, so the launcher reads the names the way the
/// repositories of Java write them; the paths are the paths the directory of
/// the libraries names, written with the separators of the system the launcher
/// runs on.
struct InstallerLayout;

impl crate::game::RepositoryLayout for InstallerLayout {
    fn base_directory(&self) -> &Path {
        Path::new(".")
    }

    fn instance_root(&self, id: &str) -> PathBuf {
        Path::new(".").join(id)
    }

    fn libraries_directory(&self) -> PathBuf {
        PathBuf::from("libraries")
    }

    fn artifact_file(&self, artifact: &Artifact) -> PathBuf {
        let path = self.libraries_directory().join(artifact.path());
        let text = path.to_string_lossy().replace('/', MAIN_SEPARATOR);
        PathBuf::from(text)
    }

    fn asset_directory(&self) -> PathBuf {
        PathBuf::from("assets")
    }

    fn asset_index_file(&self, asset_id: &str) -> PathBuf {
        PathBuf::from("assets/indexes").join(format!("{asset_id}.json"))
    }

    fn asset_object(&self, hash: &str) -> PathBuf {
        PathBuf::from("assets/objects").join(hash)
    }

    fn logging_object(&self, asset_id: &str, file_id: &str) -> PathBuf {
        PathBuf::from("assets/log_configs")
            .join(asset_id)
            .join(file_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::zip::ZipWriter;

    /// The directory the tests write their files in.
    fn directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir()
            .join("hmcl-core-forge")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");
        directory
    }

    /// Writes an installer that carries the files the tests name.
    fn installer(directory: &Path, files: &[(&str, &[u8])]) -> PathBuf {
        let path = directory.join("installer.jar");
        let mut writer = ZipWriter::create(&path).expect("the installer should be created");
        for (name, data) in files {
            writer
                .add_file(name, data)
                .expect("the file should be written");
        }
        writer.finish().expect("the installer should be written");
        path
    }

    #[test]
    fn the_installers_of_the_new_shape_are_told_apart() {
        let directory = directory("new");
        let installer = installer(
            &directory,
            &[(
                "install_profile.json",
                br#"{"spec": 1, "minecraft": "1.20.1", "json": "version.json"}"#,
            )],
        );

        assert_eq!(
            detect_format(&installer).expect("the shape should be read"),
            InstallerFormat::Processor
        );
        assert_eq!(
            installer_game_version(&installer).expect("the version should be read"),
            "1.20.1"
        );
    }

    #[test]
    fn the_installers_of_the_old_shape_are_told_apart() {
        let directory = directory("old");
        let installer = installer(
            &directory,
            &[(
                "install_profile.json",
                br#"{"install": {"path": "net.minecraftforge:forge:1.20.1-47.1.0", "filePath": "forge.jar", "minecraft": "1.20.1"}, "versionInfo": {"id": "forge"}}"#,
            )],
        );

        assert_eq!(
            detect_format(&installer).expect("the shape should be read"),
            InstallerFormat::Legacy
        );
        assert_eq!(
            installer_game_version(&installer).expect("the version should be read"),
            "1.20.1"
        );
        assert_eq!(
            legacy_library_name(&installer).expect("the name should be read"),
            "net.minecraftforge:forge:1.20.1-47.1.0"
        );
    }

    #[test]
    fn the_installers_of_no_shape_are_refused() {
        let directory = directory("none");
        // The number of the shape of the installer is enough for the launcher
        // to read the installer as an installer of the new shape.
        let named = installer(&directory, &[("install_profile.json", br#"{"spec": 0}"#)]);
        assert_eq!(
            detect_format(&named).expect("the shape should be read"),
            InstallerFormat::Processor
        );

        let empty = installer(&directory, &[("install_profile.json", br#"{}"#)]);
        assert!(detect_format(&empty).is_err());
    }

    #[test]
    fn the_words_of_a_task_are_read() {
        let tokens = BTreeMap::from([
            (
                "MINECRAFT_JAR".to_string(),
                "/tmp/minecraft.jar".to_string(),
            ),
            ("SIDE".to_string(), "client".to_string()),
        ]);
        let options = ProcessorOptions::parse(
            &[
                "--task".to_string(),
                "{MINECRAFT_JAR}".to_string(),
                "--side".to_string(),
                "client".to_string(),
                "--flag".to_string(),
                "ignored".to_string(),
            ],
            &tokens,
        )
        .expect("the words should be read");

        assert_eq!(options.get("task"), Some("/tmp/minecraft.jar"));
        assert_eq!(options.get("side"), Some("client"));
        // The word that follows the name is the value of the name, whatever the
        // name is.
        assert_eq!(options.get("flag"), Some("ignored"));
    }

    #[test]
    fn the_task_that_downloads_the_mappings_is_told_apart() {
        let tokens = BTreeMap::new();
        let options = ProcessorOptions::parse(
            &[
                "--task".to_string(),
                "DOWNLOAD_MOJMAPS".to_string(),
                "--side".to_string(),
                "client".to_string(),
                "--version".to_string(),
                "1.20.1".to_string(),
                "--output".to_string(),
                "/tmp/mappings.txt".to_string(),
            ],
            &tokens,
        )
        .expect("the words should be read");

        assert!(is_mojang_mappings_task(&options));
        assert_eq!(
            mojang_mappings_target(&options),
            Some(("1.20.1".to_string(), PathBuf::from("/tmp/mappings.txt")))
        );
    }

    #[test]
    fn the_tasks_of_the_other_sides_are_not_read() {
        let tokens = BTreeMap::new();
        let options = ProcessorOptions::parse(
            &[
                "--task".to_string(),
                "DOWNLOAD_MOJMAPS".to_string(),
                "--side".to_string(),
                "server".to_string(),
            ],
            &tokens,
        )
        .expect("the words should be read");

        assert!(!is_mojang_mappings_task(&options));
    }

    #[test]
    fn the_words_that_name_nothing_are_not_read() {
        let tokens = BTreeMap::new();
        let options =
            ProcessorOptions::parse(&["value".to_string(), "--name".to_string()], &tokens)
                .expect("the words should be read");

        assert_eq!(options.get("name"), Some(""));
        assert_eq!(options.words.len(), 1);
    }

    #[test]
    fn the_values_of_a_task_are_read() {
        let tokens = BTreeMap::from([("MC_VERSION".to_string(), "1.20.1".to_string())]);

        assert_eq!(
            parse_literal("{MC_VERSION}", &tokens, &InstallerLayout)
                .expect("the value should be read"),
            "1.20.1"
        );
        assert_eq!(
            parse_literal("'literal'", &tokens, &InstallerLayout)
                .expect("the value should be read"),
            "literal"
        );
        assert_eq!(
            parse_literal(
                "[com.example:library:1.0]",
                &tokens,
                &InstallerLayout
            )
            .expect("the value should be read"),
            // The tasks of an installer name the files of the libraries the way
            // the repositories of Java write them, so the path is written with
            // the separators of the system the launcher runs on.
            format!("libraries{MAIN_SEPARATOR}com{MAIN_SEPARATOR}example{MAIN_SEPARATOR}library{MAIN_SEPARATOR}1.0{MAIN_SEPARATOR}library-1.0.jar")
        );
        assert_eq!(
            parse_literal("the version is {MC_VERSION}", &tokens, &InstallerLayout)
                .expect("the value should be read"),
            "the version is 1.20.1"
        );
    }

    #[test]
    fn the_values_that_name_no_key_are_refused() {
        let tokens = BTreeMap::new();

        assert!(parse_literal("{UNKNOWN}", &tokens, &InstallerLayout).is_err());
        assert!(replace_tokens(&tokens, "the {UNKNOWN} key").is_err());
    }

    #[test]
    fn the_escapes_of_a_pattern_are_read() {
        let tokens = BTreeMap::from([("KEY".to_string(), "value".to_string())]);

        assert_eq!(
            replace_tokens(&tokens, "\\{KEY\\}").expect("the text should be read"),
            "{KEY}"
        );
        assert_eq!(
            replace_tokens(&tokens, "a\\'b").expect("the text should be read"),
            "a'b"
        );
    }

    #[test]
    fn the_patterns_that_are_not_closed_are_refused() {
        let tokens = BTreeMap::new();

        assert!(replace_tokens(&tokens, "{").is_err());
        assert!(replace_tokens(&tokens, "'").is_err());
        assert!(replace_tokens(&tokens, "abc\\").is_err());
    }

    #[test]
    fn the_libraries_an_installer_carries_are_written() {
        let directory = directory("libraries");
        let installer = installer(
            &directory,
            &[
                (
                    "install_profile.json",
                    br#"{"spec": 1, "minecraft": "1.20.1", "json": "version.json", "libraries": [{"name": "com.example:library:1.0"}], "path": "com.example:main:2.0"}"#,
                ),
                ("maven/com/example/library/1.0/library-1.0.jar", b"library"),
                ("maven/com/example/main/2.0/main-2.0.jar", b"main"),
            ],
        );

        let profile = read_processor_profile(&installer).expect("the profile should be read");
        let libraries = directory.join("libraries");
        let files = copy_installer_libraries(&profile, &installer, &libraries)
            .expect("the libraries should be written");

        assert_eq!(
            files,
            vec![
                libraries.join("com/example/library/1.0/library-1.0.jar"),
                libraries.join("com/example/main/2.0/main-2.0.jar"),
            ]
        );
        assert_eq!(
            std::fs::read(libraries.join("com/example/main/2.0/main-2.0.jar"))
                .expect("the file should be read"),
            b"main"
        );
    }

    #[test]
    fn the_files_a_task_needs_are_written() {
        let directory = directory("data");
        let installer = installer(
            &directory,
            &[
                (
                    "install_profile.json",
                    br#"{"spec": 1, "minecraft": "1.20.1", "json": "version.json", "data": {"BINPATCH": {"client": "data/binpatch.lzma"}}}"#,
                ),
                ("data/binpatch.lzma", b"patched"),
            ],
        );

        let profile = read_processor_profile(&installer).expect("the profile should be read");
        let temporary = directory.join("temporary");
        let tokens =
            data_tokens(&profile, &installer, &temporary).expect("the data should be written");

        let path = temporary.join("BINPATCH");
        assert_eq!(
            std::fs::read(&path).expect("the file should be read"),
            b"patched"
        );
        assert_eq!(tokens.get("BINPATCH").map(PathBuf::from), Some(path));
    }

    #[test]
    fn the_name_of_the_version_of_the_loader_is_written() {
        assert_eq!(modify_version("1.20.1", "1.20.1-47.1.0"), "47.1.0");
        // The installers of the old shape name the version of the loader with
        // the word Forge, and the launcher takes that word away before it
        // writes the name.
        assert_eq!(
            modify_version("1.20.1", &"1.20.1-47.1.0-forge".replace("forge", "")),
            "47.1.0"
        );
        assert_eq!(
            modify_neoforge_old_version("1.20.1", "1.20.1-47.1.0"),
            "47.1.0"
        );
        assert_eq!(modify_neoforge_new_version("neoforge-20.4.100"), "20.4.100");
    }

    #[test]
    fn the_patch_of_neoforge_is_named_after_neoforge() {
        let patch = InstancePatch {
            id: Some("forge".to_string()),
            version: Some("20.4.100".to_string()),
            ..InstancePatch::default()
        };

        let patch = neoforge_patch_id(patch).expect("the patch should be renamed");

        assert_eq!(patch.id.as_deref(), Some("neoforge"));
        assert_eq!(patch.version.as_deref(), Some("20.4.100"));
    }

    #[test]
    fn the_patch_that_is_not_the_patch_of_forge_is_refused() {
        let patch = InstancePatch {
            id: Some("neoforge".to_string()),
            version: Some("20.4.100".to_string()),
            ..InstancePatch::default()
        };

        assert!(neoforge_patch_id(patch).is_err());
    }

    #[test]
    fn the_installer_of_neoforge_is_told_apart() {
        let new = installer(
            &directory("neoforge-new"),
            &[("install_profile.json", br#"{"profile": "neoforge"}"#)],
        );
        let old = installer(
            &directory("neoforge-old"),
            &[
                ("install_profile.json", br#"{"profile": "forge"}"#),
                ("META-INF/NEOFORGE.RSA", b"signature"),
            ],
        );
        let forge = installer(
            &directory("neoforge-forge"),
            &[("install_profile.json", br#"{"profile": "forge"}"#)],
        );

        assert!(is_neoforge_installer(&new, profile_name(&new).as_deref()));
        assert!(is_neoforge_installer(&old, profile_name(&old).as_deref()));
        assert!(!is_neoforge_installer(
            &forge,
            profile_name(&forge).as_deref()
        ));
    }

    #[test]
    fn the_versions_of_the_game_that_are_not_started_the_way_forge_expects_are_refused() {
        // The versions of the game before 1.13 are not checked, because they
        // are started through the wrapper of the game anyway.
        assert!(check_launch_wrapper(Some("com.example.Main"), "1.12.2").is_ok());
        assert!(check_launch_wrapper(None, "1.12.2").is_ok());

        // The versions of the game 1.13 and after are checked, because they
        // are started through the launcher of the loader.
        assert!(check_launch_wrapper(Some(VANILLA_MAIN), "1.13").is_ok());
        assert!(check_launch_wrapper(Some(FORGE_BOOTSTRAP_MAIN), "1.13").is_ok());
        assert!(check_launch_wrapper(Some("com.example.Main"), "1.13").is_err());
        assert!(check_launch_wrapper(None, "1.13").is_err());
    }

    use crate::game::component::{FORGE_BOOTSTRAP_MAIN, VANILLA_MAIN};
}
