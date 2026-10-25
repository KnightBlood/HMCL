//! The modpacks of CurseForge.
//!
//! CurseForge keeps the files of a modpack in its own archive and the launcher
//! reads the manifest of the pack: the manifest names the version of the game,
//! the mod loaders the pack needs and the projects and files of the mods. The
//! launcher asks CurseForge for the name and the address of a file, because the
//! manifest of the pack names the identifiers of the file only.
//!
//! The manifest is also written by the launcher when it exports an instance for
//! the players of CurseForge, so the reader and the writer of the same format
//! live together.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::util::string::is_blank;

use super::completion::{
    has_simple_mod, CompletionDownload, CompletionPlan, UnresolvedFile, UnresolvedKind,
};
use super::plan::relative_path;

/// The type the manifest of a modpack of CurseForge names itself.
pub const MANIFEST_TYPE: &str = "minecraftModpack";

/// The name of the provider, as the launcher writes it in a configuration.
pub const PROVIDER_NAME: &str = "Curse";

/// The file the manifest of a modpack is written in.
pub const MANIFEST: &str = "manifest.json";

/// The file the description of a modpack is written in.
pub const MOD_LIST: &str = "modlist.html";

/// The directory the archive of a modpack keeps the files of the instance in.
pub const OVERRIDES: &str = "overrides";

/// The manifest of a modpack of CurseForge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurseManifest {
    /// The type of the manifest, which must be [`MANIFEST_TYPE`].
    #[serde(default, rename = "manifestType")]
    pub manifest_type: String,
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
    /// The directory the archive keeps the files of the instance in.
    #[serde(default)]
    pub overrides: String,
    /// The version of the game and the mod loaders the modpack needs.
    #[serde(default)]
    pub minecraft: CurseMinecraft,
    /// The mods of the modpack.
    #[serde(default)]
    pub files: Vec<CurseFile>,
}

impl CurseManifest {
    /// Reads the manifest of a modpack.
    pub fn read(text: &str) -> Result<Self, CurseError> {
        let manifest: Self =
            serde_json::from_str(text).map_err(|error| CurseError::Malformed(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Reads the manifest of a modpack from the archive of the modpack.
    pub fn read_from_archive(archive: &std::path::Path) -> Result<Self, CurseError> {
        let data = crate::util::zip::read_entry(archive, MANIFEST)
            .map_err(CurseError::Zip)?
            .ok_or(CurseError::MissingManifest)?;
        let text =
            String::from_utf8(data).map_err(|error| CurseError::Malformed(error.to_string()))?;
        Self::read(&text)
    }

    /// Reads the description of a modpack from the archive of the modpack.
    ///
    /// The description of a modpack of CurseForge is a page of HTML the author
    /// writes; the manifest names no description, so the launcher reads it from
    /// the archive. An archive without the page has no description.
    pub fn read_description(archive: &std::path::Path) -> Option<String> {
        let data = crate::util::zip::read_entry(archive, MOD_LIST).ok()??;
        String::from_utf8(data).ok()
    }

    /// Refuses a manifest the launcher cannot install.
    pub fn validate(&self) -> Result<(), CurseError> {
        if self.manifest_type != MANIFEST_TYPE {
            return Err(CurseError::ManifestType(self.manifest_type.clone()));
        }
        if is_blank(&self.minecraft.version) {
            return Err(CurseError::NoGame);
        }
        for file in &self.files {
            file.validate()?;
        }
        Ok(())
    }

    /// Returns the version of the game the modpack needs.
    pub fn game_version(&self) -> &str {
        &self.minecraft.version
    }

    /// Returns the directory the archive keeps the files of the instance in.
    ///
    /// A modpack of CurseForge names the directory, and the launcher of today
    /// writes `overrides`; an archive of an older launcher may name another
    /// directory or name none, which the manifest of the format does not allow
    /// but an old archive may still do.
    pub fn overrides(&self) -> &str {
        if self.overrides.is_empty() {
            OVERRIDES
        } else {
            self.overrides.as_str()
        }
    }

    /// Returns the manifest with other files.
    pub fn with_files(&self, files: Vec<CurseFile>) -> Self {
        let mut manifest = self.clone();
        manifest.files = files;
        manifest
    }
}

/// The version of the game and the mod loaders a modpack needs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurseMinecraft {
    /// The version of the game.
    #[serde(default)]
    pub version: String,
    /// The mod loaders of the modpack.
    #[serde(default, rename = "modLoaders")]
    pub mod_loaders: Vec<CurseModLoader>,
}

/// A mod loader of a modpack.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurseModLoader {
    /// The identifier of the loader, such as `forge-47.2.0`.
    #[serde(default)]
    pub id: String,
    /// Whether the loader is the one the modpack is built for.
    #[serde(default)]
    pub primary: bool,
}

impl CurseModLoader {
    /// Refuses a loader without an identifier.
    pub fn validate(&self) -> Result<(), CurseError> {
        if is_blank(&self.id) {
            return Err(CurseError::NoModLoader);
        }
        Ok(())
    }
}

/// A mod of a modpack.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurseFile {
    /// The identifier of the project of the mod.
    #[serde(default, rename = "projectID")]
    pub project_id: i32,
    /// The identifier of the file of the mod.
    #[serde(default, rename = "fileID")]
    pub file_id: i32,
    /// The name of the file, if the launcher knows it.
    #[serde(default, rename = "fileName")]
    pub file_name: Option<String>,
    /// The address the file is downloaded from, if the manifest names one.
    #[serde(default)]
    pub url: Option<String>,
    /// Whether the modpack needs the file.
    #[serde(default)]
    pub required: bool,
    /// The algorithms and the digests of the file, if the manifest names them.
    ///
    /// The manifest of a modpack of CurseForge names no digests; the launcher
    /// writes them when it resolves a file itself.
    #[serde(default)]
    pub hashes: Option<std::collections::BTreeMap<String, String>>,
}

impl CurseFile {
    /// Returns the key the optional files of a modpack are chosen by.
    pub fn key(&self) -> String {
        format!("curseforge:{}:{}", self.project_id, self.file_id)
    }

    /// Returns whether the modpack works without the file.
    pub fn optional(&self) -> bool {
        !self.required
    }

    /// Returns the path of the file in the instance.
    pub fn path(&self) -> Option<String> {
        self.file_name
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(|name| format!("mods/{name}"))
    }

    /// Returns the address the file is downloaded from.
    ///
    /// The manifest of an older modpack names the address; the manifest of
    /// today names the identifiers only, and the address is built from them the
    /// way the archive of CurseForge addresses its files.
    pub fn url(&self) -> Option<String> {
        match (&self.url, &self.file_name) {
            (Some(url), _) if !url.is_empty() => Some(url.clone()),
            (_, Some(name)) if !name.is_empty() => Some(format!(
                "https://edge.forgecdn.net/files/{}/{}/{}",
                self.file_id / 1000,
                self.file_id % 1000,
                name
            )),
            _ => None,
        }
    }

    /// Returns the algorithms and the digests of the file the launcher checks.
    ///
    /// The first digest of the manifest is used, in the order the launcher
    /// knows them, because a manifest may name several.
    pub fn integrity_check(&self) -> Option<(&'static str, &str)> {
        const ALGORITHMS: [(&str, &str); 4] = [
            ("sha1", "SHA-1"),
            ("sha256", "SHA-256"),
            ("sha512", "SHA-512"),
            ("md5", "MD5"),
        ];

        let hashes = self.hashes.as_ref()?;
        ALGORITHMS.iter().find_map(|(key, algorithm)| {
            hashes
                .get(*key)
                .filter(|hash| !hash.is_empty())
                .map(|hash| (*algorithm, hash.as_str()))
        })
    }

    /// Refuses a file without identifiers.
    pub fn validate(&self) -> Result<(), CurseError> {
        if self.project_id == 0 || self.file_id == 0 {
            return Err(CurseError::NoFileId);
        }
        Ok(())
    }

    /// Returns the file with the name the server of the mod told the launcher.
    pub fn with_file_name(&self, name: impl Into<String>) -> Self {
        let mut file = self.clone();
        file.file_name = Some(name.into());
        file
    }

    /// Returns the file with the address the server of the mod told the
    /// launcher.
    pub fn with_url(&self, address: impl Into<String>) -> Self {
        let mut file = self.clone();
        file.url = Some(address.into());
        file
    }

    /// Returns the file with the digests the server of the mod told the
    /// launcher.
    pub fn with_hashes(&self, hashes: std::collections::BTreeMap<String, String>) -> Self {
        let mut file = self.clone();
        file.hashes = Some(hashes);
        file
    }
}

/// The directory of an instance a file of CurseForge is installed in.
///
/// CurseForge keeps mods, resource packs and shader packs in the same list of
/// files and tells the kind of a project with its class; the launcher reads the
/// class from the repository and writes the file into the directory the game
/// reads it from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurseKind {
    /// A mod, which the game reads from `mods`.
    Mods,
    /// A resource pack, which the game reads from `resourcepacks`.
    ResourcePacks,
    /// A shader pack, which the game reads from `shaderpacks`.
    ShaderPacks,
}

impl CurseKind {
    /// Returns the kind of a project of the class CurseForge names.
    ///
    /// The class `12` is a resource pack, the class `6945` is a data pack, which
    /// the launcher writes into the directory of the resource packs like the
    /// launcher of Java does, and the class `6552` is a shader pack. Every other
    /// class is a mod.
    pub fn of_class(class: i32) -> Self {
        match class {
            12 | 6945 => Self::ResourcePacks,
            6552 => Self::ShaderPacks,
            _ => Self::Mods,
        }
    }

    /// Returns the directory of the instance the kind is installed in.
    pub fn directory(self) -> &'static str {
        match self {
            Self::Mods => "mods",
            Self::ResourcePacks => "resourcepacks",
            Self::ShaderPacks => "shaderpacks",
        }
    }
}

/// Plans the files a completion of a modpack of CurseForge downloads.
///
/// The manifest of a modpack names the projects and the files of its mods, and
/// the name of a file the launcher has not resolved yet is absent. The plan
/// reports such a file as [`UnresolvedKind::NotResolved`], so the caller can ask
/// the repository and plan again; a file that is already in the instance, that
/// the player left out or that the game already loads is skipped.
///
/// The kind of a project comes from the repository, which the launcher asks per
/// project; `kind_of` stands for that question. It returns `None` when the
/// repository could not be asked, which the launcher reports and skips, because
/// one project it cannot ask must not fail the whole install.
///
/// The mods the old manifest named and the new one does not are removed, which
/// is the way an update of a modpack replaces a file: the plan names them in
/// [`CompletionPlan::removals`].
pub fn plan_completion<K>(
    manifest: &CurseManifest,
    run_directory: &Path,
    excluded: Option<&BTreeSet<String>>,
    kind_of: K,
) -> CompletionPlan
where
    K: FnMut(i32) -> Option<CurseKind>,
{
    plan_completion_with_updates(
        manifest,
        run_directory,
        excluded,
        &[],
        &BTreeSet::new(),
        kind_of,
    )
}

/// Plans the files a completion of a modpack of CurseForge downloads, with the
/// mods of the version the instance had.
///
/// The mods of the old manifest that the new one no longer names are removed,
/// unless the new archive unpacks them itself from `overrides`.
pub fn plan_completion_with_updates<K>(
    manifest: &CurseManifest,
    run_directory: &Path,
    excluded: Option<&BTreeSet<String>>,
    old: &[CurseFile],
    override_mods: &BTreeSet<String>,
    mut kind_of: K,
) -> CompletionPlan
where
    K: FnMut(i32) -> Option<CurseKind>,
{
    let mut plan = CompletionPlan {
        total: manifest.files.len() as u64,
        ..CompletionPlan::default()
    };

    for file in &manifest.files {
        if excluded.is_some_and(|excluded| excluded.contains(&file.key())) {
            continue;
        }

        let Some(name) = file
            .file_name
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(str::to_string)
        else {
            plan.unresolved.push(UnresolvedFile::curse(
                file.project_id,
                file.file_id,
                UnresolvedKind::NotResolved,
            ));
            continue;
        };

        let Some(url) = file.url() else {
            plan.unresolved.push(UnresolvedFile::curse_named(
                file.project_id,
                file.file_id,
                name,
                UnresolvedKind::NotResolved,
            ));
            continue;
        };

        let Some(kind) = kind_of(file.project_id) else {
            plan.warnings.push(format!(
                "Could not query the repository for the project {}; the modpack is installed without its file {name}",
                file.project_id
            ));
            continue;
        };

        let Some(path) = relative_path(run_directory, &format!("{}/{name}", kind.directory()))
        else {
            plan.warnings.push(format!(
                "The file {name} of the project {} leaves the instance",
                file.project_id
            ));
            continue;
        };

        // A mod the player disabled or replaced is still the mod of the pack, so
        // the launcher keeps the file the player arranged.
        if kind == CurseKind::Mods && has_simple_mod(&run_directory.join("mods"), &name) {
            continue;
        }
        if path.exists() {
            continue;
        }

        plan.downloads.push(CompletionDownload {
            urls: vec![url],
            integrity: file
                .integrity_check()
                .map(|(algorithm, digest)| (algorithm.to_string(), digest.to_string())),
            path,
        });
    }

    plan.removals = removed_mods(old, manifest, run_directory, override_mods);

    plan
}

/// Returns the mods an update of a modpack of CurseForge removes.
///
/// The launcher removes a mod of the old manifest that the new manifest no
/// longer names, unless the new archive holds the mod itself under its
/// `overrides` directory: a file of the archive belongs to the modpack and is
/// unpacked by the install, so removing it would delete a file the pack wants.
///
/// The files of the manifest with the same project and file identifiers are the
/// same mod, whichever names the two manifests gave it.
pub fn removed_mods(
    old: &[CurseFile],
    new: &CurseManifest,
    run_directory: &Path,
    override_mods: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let mut removals = Vec::new();

    for file in old {
        let Some(name) = file.file_name.as_deref().filter(|name| !name.is_empty()) else {
            continue;
        };
        let relative = format!("mods/{name}");
        if override_mods.contains(&relative) {
            continue;
        }
        if new.files.iter().any(|candidate| {
            candidate.project_id == file.project_id && candidate.file_id == file.file_id
        }) {
            continue;
        }
        let Some(path) = relative_path(run_directory, &relative) else {
            continue;
        };
        if path.exists() {
            removals.push(path);
        }
    }

    removals
}

/// Why a manifest of CurseForge cannot be read.
#[derive(Debug)]
pub enum CurseError {
    /// The archive holds no manifest.
    MissingManifest,
    /// The manifest is not a document the launcher reads.
    Malformed(String),
    /// The manifest names another format of a modpack.
    ManifestType(String),
    /// The manifest names no version of the game.
    NoGame,
    /// The manifest names a mod loader without an identifier.
    NoModLoader,
    /// The manifest names a mod without its identifiers.
    NoFileId,
    /// The archive cannot be read.
    Zip(crate::util::zip::ZipError),
}

impl std::fmt::Display for CurseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingManifest => formatter.write_str("The archive holds no manifest.json"),
            Self::Malformed(reason) => write!(formatter, "The manifest is malformed: {reason}"),
            Self::ManifestType(kind) => {
                write!(formatter, "The manifest names the type {kind:?}")
            }
            Self::NoGame => formatter.write_str("The manifest names no version of the game"),
            Self::NoModLoader => formatter.write_str("A mod loader of the manifest has no id"),
            Self::NoFileId => formatter.write_str("A mod of the manifest has no identifiers"),
            Self::Zip(error) => write!(formatter, "The archive cannot be read: {error}"),
        }
    }
}

impl std::error::Error for CurseError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a manifest with the mods the test names.
    fn manifest() -> &'static str {
        r#"{
            "manifestType": "minecraftModpack",
            "manifestVersion": 1,
            "name": "World",
            "version": "1.0",
            "author": "player",
            "overrides": "overrides",
            "minecraft": {
                "version": "1.20.1",
                "modLoaders": [{"id": "forge-47.2.0", "primary": true}]
            },
            "files": [
                {"projectID": 12, "fileID": 34567, "required": true},
                {"projectID": 56, "fileID": 78, "fileName": "sodium.jar", "url": "https://example.com/sodium.jar"}
            ]
        }"#
    }

    #[test]
    fn the_manifest_of_a_modpack_is_read() {
        let manifest = CurseManifest::read(manifest()).expect("the manifest should be read");
        assert_eq!(manifest.name, "World");
        assert_eq!(manifest.game_version(), "1.20.1");
        assert_eq!(manifest.overrides(), "overrides");
        assert_eq!(manifest.minecraft.mod_loaders.len(), 1);
        assert_eq!(manifest.files.len(), 2);
        assert!(manifest.files[0].required);
        assert!(!manifest.files[0].optional());
    }

    #[test]
    fn a_file_without_a_name_is_addressed_by_its_identifiers() {
        let manifest = CurseManifest::read(manifest()).expect("the manifest should be read");
        let file = &manifest.files[0];
        // The address of a file of CurseForge holds the name of the file, so a
        // file whose name the launcher does not know has no address yet.
        assert_eq!(file.url(), None);
        assert_eq!(file.key(), "curseforge:12:34567");
        assert_eq!(file.path(), None);

        let named = file.with_file_name("sodium.jar");
        assert_eq!(named.path().as_deref(), Some("mods/sodium.jar"));
        assert_eq!(
            named.url().as_deref(),
            Some("https://edge.forgecdn.net/files/34/567/sodium.jar")
        );
    }

    #[test]
    fn a_file_names_the_address_of_the_manifest_first() {
        let manifest = CurseManifest::read(manifest()).expect("the manifest should be read");
        assert_eq!(
            manifest.files[1].url().as_deref(),
            Some("https://example.com/sodium.jar")
        );
    }

    #[test]
    fn the_digest_of_a_file_is_the_first_the_launcher_knows() {
        let manifest = CurseManifest::read(manifest()).expect("the manifest should be read");
        assert_eq!(manifest.files[0].integrity_check(), None);

        let hashes = [
            (
                "md5".to_string(),
                "d41d8cd98f00b204e9800998ecf8427e".to_string(),
            ),
            ("sha256".to_string(), "a".repeat(64)),
        ]
        .into_iter()
        .collect();
        let file = manifest.files[0].with_hashes(hashes);
        assert_eq!(
            file.integrity_check().map(|(algorithm, _)| algorithm),
            Some("SHA-256")
        );
    }

    #[test]
    fn a_manifest_of_another_format_is_refused() {
        let text = manifest().replace(MANIFEST_TYPE, "somethingElse");
        let error = CurseManifest::read(&text).expect_err("the manifest should be refused");
        assert!(matches!(error, CurseError::ManifestType(_)));

        let text = manifest().replace("\"version\": \"1.20.1\"", "\"version\": \"\"");
        let error = CurseManifest::read(&text).expect_err("the manifest should be refused");
        assert!(matches!(error, CurseError::NoGame));
    }

    #[test]
    fn a_mod_without_identifiers_is_refused() {
        let text = manifest().replace(
            "\"projectID\": 12, \"fileID\": 34567",
            "\"projectID\": 0, \"fileID\": 0",
        );
        let error = CurseManifest::read(&text).expect_err("the manifest should be refused");
        assert!(matches!(error, CurseError::NoFileId));
    }

    #[test]
    fn the_plan_downloads_a_file_the_class_of_its_project_names() {
        let manifest = CurseManifest::read(manifest()).expect("the manifest should be read");
        let run = run_directory("plan");
        let plan = plan_completion(&manifest, &run, None, |project| match project {
            56 => Some(CurseKind::ResourcePacks),
            _ => Some(CurseKind::Mods),
        });

        // The file without a name has to be resolved first.
        assert_eq!(plan.total, 2);
        assert_eq!(plan.unresolved.len(), 1);
        assert_eq!(plan.unresolved[0].project, Some(12));
        assert_eq!(plan.downloads.len(), 1);
        assert_eq!(
            plan.downloads[0].path,
            run.join("resourcepacks").join("sodium.jar")
        );
        assert_eq!(
            plan.downloads[0].urls,
            vec!["https://example.com/sodium.jar".to_string()]
        );

        let _ = std::fs::remove_dir_all(&run);
    }

    #[test]
    fn a_file_the_player_left_out_or_the_game_holds_is_not_downloaded() {
        let manifest = CurseManifest::read(manifest()).expect("the manifest should be read");
        let run = run_directory("skipped");
        std::fs::create_dir_all(run.join("mods")).expect("the mods should be made");
        std::fs::write(run.join("mods").join("sodium.jar"), b"a mod")
            .expect("the mod should be written");

        let excluded: std::collections::BTreeSet<String> =
            [manifest.files[1].key()].into_iter().collect();
        let plan = plan_completion(&manifest, &run, Some(&excluded), |_| Some(CurseKind::Mods));
        assert_eq!(plan.downloads.len(), 0);

        // A mod the player disabled is the mod of the pack.
        std::fs::write(
            run.join("mods").join("sodium.jar.disabled"),
            b"a disabled mod",
        )
        .expect("the mod should be written");
        let plan = plan_completion(&manifest, &run, None, |_| Some(CurseKind::Mods));
        assert_eq!(plan.downloads.len(), 0);

        let _ = std::fs::remove_dir_all(&run);
    }

    #[test]
    fn a_project_the_repository_cannot_be_asked_about_is_reported() {
        let manifest = CurseManifest::read(manifest()).expect("the manifest should be read");
        let run = run_directory("unasked");
        let plan = plan_completion(&manifest, &run, None, |_| None);

        assert_eq!(plan.downloads.len(), 0);
        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("56"));

        let _ = std::fs::remove_dir_all(&run);
    }

    #[test]
    fn an_update_of_a_modpack_removes_the_mods_of_the_version_it_had() {
        let new = CurseManifest::read(manifest()).expect("the manifest should be read");
        let old = vec![CurseFile {
            project_id: 12,
            file_id: 34,
            file_name: Some("old.jar".to_string()),
            ..CurseFile::default()
        }];
        let run = run_directory("plan-update");
        std::fs::create_dir_all(run.join("mods")).expect("the mods should be made");
        std::fs::write(run.join("mods").join("old.jar"), b"a mod")
            .expect("the mod should be written");

        let plan = plan_completion_with_updates(&new, &run, None, &old, &BTreeSet::new(), |_| {
            Some(CurseKind::Mods)
        });

        // The mod the new manifest no longer names is removed, and the mod it
        // names is downloaded.
        assert_eq!(plan.removals, vec![run.join("mods").join("old.jar")]);
        assert_eq!(plan.downloads.len(), 1);
        assert_eq!(plan.downloads[0].path, run.join("mods").join("sodium.jar"));

        let _ = std::fs::remove_dir_all(&run);
    }

    #[test]
    fn an_update_removes_the_mods_the_new_manifest_no_longer_names() {
        let old = vec![
            CurseFile {
                project_id: 12,
                file_id: 34,
                file_name: Some("sodium.jar".to_string()),
                ..CurseFile::default()
            },
            CurseFile {
                project_id: 56,
                file_id: 78,
                file_name: Some("kept.jar".to_string()),
                ..CurseFile::default()
            },
            CurseFile {
                project_id: 90,
                file_id: 12,
                file_name: Some("gone.jar".to_string()),
                ..CurseFile::default()
            },
        ];
        let new = CurseManifest::read(manifest()).expect("the manifest should be read");
        let run = run_directory("update");
        std::fs::create_dir_all(run.join("mods")).expect("the mods should be made");
        for name in ["sodium.jar", "kept.jar", "gone.jar"] {
            std::fs::write(run.join("mods").join(name), b"a mod")
                .expect("the mod should be written");
        }

        let overrides = ["mods/kept.jar".to_string()]
            .into_iter()
            .collect::<std::collections::BTreeSet<String>>();
        let removals = removed_mods(&old, &new, &run, &overrides);

        // The mod of the archive is unpacked by the install, so it is kept.
        assert_eq!(removals.len(), 2);
        assert!(removals.contains(&run.join("mods").join("sodium.jar")));
        assert!(removals.contains(&run.join("mods").join("gone.jar")));
        assert!(!removals.contains(&run.join("mods").join("kept.jar")));

        let _ = std::fs::remove_dir_all(&run);
    }

    /// Returns a directory of an instance no other test uses.
    fn run_directory(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("hmcl-rs-curse-{name}-{}", std::process::id()))
    }
}
