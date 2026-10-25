//! The modpacks of Modrinth.
//!
//! Modrinth publishes the manifest of a modpack as `modrinth.index.json`: the
//! manifest names the version of the game, the versions of the loaders and the
//! files of the pack. A file of Modrinth is named by its path in the instance
//! and by the digests of its content, which makes the pack installable from
//! whichever address answers first.
//!
//! The launcher also writes the format when it exports an instance, so a pack
//! it writes can be uploaded to Modrinth.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::util::digest::Algorithm;
use crate::util::string::is_blank;

use super::completion::{has_simple_mod, CompletionDownload, CompletionPlan};
use super::plan::relative_path;

/// The game the manifest names, which is the only game the launcher installs.
pub const GAME: &str = "minecraft";

/// The version of the format the launcher writes.
pub const FORMAT_VERSION: i32 = 1;

/// The name of the provider, as the launcher writes it in a configuration.
pub const PROVIDER_NAME: &str = "Modrinth";

/// The file the manifest of a modpack is written in.
pub const MANIFEST: &str = "modrinth.index.json";

/// The directory the archive keeps the files of the instance in.
pub const OVERRIDES: &str = "overrides";

/// The directory the archive of a modpack of Modrinth keeps the files of the
/// client in.
pub const CLIENT_OVERRIDES: &str = "client-overrides";

/// The key of the dependencies that names the version of the game.
pub const MINECRAFT: &str = "minecraft";

/// The key of the environment map that names the side of a file.
pub const CLIENT: &str = "client";

/// The value of the environment map that marks a file as optional.
pub const OPTIONAL: &str = "optional";

/// The value of the environment map that marks a file the client cannot use.
pub const UNSUPPORTED: &str = "unsupported";

/// The manifest of a modpack of Modrinth.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModrinthManifest {
    /// The game the modpack is for.
    #[serde(default)]
    pub game: String,
    /// The version of the format of the manifest.
    #[serde(default, rename = "formatVersion")]
    pub format_version: i32,
    /// The version of the modpack.
    #[serde(default, rename = "versionId")]
    pub version_id: String,
    /// The name of the modpack.
    #[serde(default)]
    pub name: String,
    /// The description of the modpack.
    #[serde(default)]
    pub summary: Option<String>,
    /// The files of the modpack.
    #[serde(default)]
    pub files: Vec<ModrinthFile>,
    /// The versions of the game and of the loaders the modpack needs.
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

impl ModrinthManifest {
    /// Reads the manifest of a modpack.
    pub fn read(text: &str) -> Result<Self, ModrinthError> {
        let manifest: Self = serde_json::from_str(text)
            .map_err(|error| ModrinthError::Malformed(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Reads the manifest of a modpack from the archive of the modpack.
    pub fn read_from_archive(archive: &Path) -> Result<Self, ModrinthError> {
        let data = crate::util::zip::read_entry(archive, MANIFEST)
            .map_err(ModrinthError::Zip)?
            .ok_or(ModrinthError::MissingManifest)?;
        let text =
            String::from_utf8(data).map_err(|error| ModrinthError::Malformed(error.to_string()))?;
        Self::read(&text)
    }

    /// Refuses a manifest the launcher cannot install.
    ///
    /// A modpack of Modrinth must name the version of the game, because the
    /// launcher cannot install the files of a pack into an instance it does not
    /// know the version of.
    pub fn validate(&self) -> Result<(), ModrinthError> {
        if !self.game.is_empty() && self.game != GAME {
            return Err(ModrinthError::Game(self.game.clone()));
        }
        if self
            .dependencies
            .get(MINECRAFT)
            .is_none_or(|version| is_blank(version))
        {
            return Err(ModrinthError::NoGame);
        }
        for file in &self.files {
            file.validate()?;
        }
        Ok(())
    }

    /// Returns the version of the game the modpack needs.
    pub fn game_version(&self) -> Option<&str> {
        self.dependencies.get(MINECRAFT).map(String::as_str)
    }

    /// Returns the description of the modpack, which may be absent.
    pub fn summary(&self) -> &str {
        self.summary.as_deref().unwrap_or_default()
    }

    /// Returns the version of a loader the modpack needs.
    ///
    /// The loaders are named by the keys Modrinth uses, which differ from the
    /// names the launcher uses for the same loader.
    pub fn loader_version(&self, loader: &str) -> Option<&str> {
        self.dependencies.get(loader).map(String::as_str)
    }

    /// Returns the manifest with other files.
    pub fn with_files(&self, files: Vec<ModrinthFile>) -> Self {
        let mut manifest = self.clone();
        manifest.files = files;
        manifest
    }

    /// Returns the files of the modpack the player chose to install.
    ///
    /// The files a pack marks as optional are installed only when the player
    /// selects them; a file the player did not select is left out.
    pub fn selected_files<'a>(
        &'a self,
        excluded: Option<&'a std::collections::BTreeSet<String>>,
    ) -> impl Iterator<Item = &'a ModrinthFile> {
        self.files.iter().filter(move |file| match excluded {
            Some(excluded) => !excluded.contains(&file.key()),
            None => true,
        })
    }
}

/// A file of a modpack of Modrinth.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModrinthFile {
    /// The path of the file in the instance.
    #[serde(default)]
    pub path: String,
    /// The algorithms and the digests of the file.
    #[serde(default)]
    pub hashes: BTreeMap<String, String>,
    /// The sides of the game the file belongs to.
    #[serde(default)]
    pub env: Option<BTreeMap<String, String>>,
    /// The addresses the file is downloaded from.
    #[serde(default)]
    pub downloads: Vec<String>,
    /// The size of the file in bytes.
    #[serde(default, rename = "fileSize")]
    pub file_size: i64,
}

impl ModrinthFile {
    /// Refuses a file the launcher cannot install.
    ///
    /// The path of a file is refused when it leaves the instance, because the
    /// manifest of a modpack is written by a stranger; the digest is required
    /// because the launcher cannot tell whether the file it downloaded is the
    /// file of the pack without it.
    pub fn validate(&self) -> Result<(), ModrinthError> {
        if is_blank(&self.path) {
            return Err(ModrinthError::NoPath);
        }
        if relative_path(Path::new("."), &self.path).is_none() {
            return Err(ModrinthError::EscapingPath(self.path.clone()));
        }
        if !self
            .hashes
            .get("sha512")
            .is_some_and(|digest| crate::util::digest::is_sha512_digest(digest))
        {
            return Err(ModrinthError::NoDigest(self.path.clone()));
        }
        if self
            .env
            .as_ref()
            .is_some_and(|env| !env.contains_key(CLIENT))
        {
            return Err(ModrinthError::NoEnvironment(self.path.clone()));
        }
        if self.downloads.is_empty() {
            return Err(ModrinthError::NoDownload(self.path.clone()));
        }
        Ok(())
    }

    /// Returns the key the optional files of a modpack are chosen by.
    pub fn key(&self) -> String {
        format!(
            "modrinth:{}:{}",
            self.hashes.get("sha512").map(String::as_str).unwrap_or(""),
            self.path
        )
    }

    /// Returns the name of the file, without the directories of its path.
    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(self.path.as_str())
    }

    /// Returns the side of the game the file belongs to.
    pub fn side(&self) -> Option<&str> {
        self.env.as_ref()?.get(CLIENT).map(String::as_str)
    }

    /// Returns whether the modpack works without the file.
    pub fn optional(&self) -> bool {
        self.side() == Some(OPTIONAL)
    }

    /// Returns the address the file is downloaded from, in the order the
    /// manifest names them.
    pub fn download(&self) -> Option<&str> {
        self.downloads.first().map(String::as_str)
    }

    /// Returns the algorithm and the digest the launcher checks the file with.
    ///
    /// Modrinth names the SHA-512 of every file; the launcher also reads the
    /// SHA-1, which is the digest the downloads of the launcher report.
    pub fn integrity_check(&self) -> Option<(&'static str, &str)> {
        for (key, algorithm) in [("sha512", Algorithm::Sha512), ("sha1", Algorithm::Sha1)] {
            if let Some(digest) = self.hashes.get(key).filter(|digest| !digest.is_empty()) {
                return Some((algorithm.name(), digest.as_str()));
            }
        }
        None
    }

    /// Returns the SHA-512 digest of the file, which the launcher knows.
    pub fn sha512(&self) -> Option<&str> {
        self.hashes.get("sha512").map(String::as_str)
    }
}

/// Plans the files a completion of a modpack of Modrinth downloads.
///
/// The manifest of a modpack of Modrinth names the path of a file in the
/// instance and the addresses the file is downloaded from, so a completion
/// needs no repository. It skips the files the player left out and the files the
/// game cannot use on the client, and it keeps the files the player arranged.
///
/// Returns [`ModrinthError::EscapingPath`] for a path that leaves the instance,
/// because such a file would be written outside of the instance; the launcher of
/// Java refuses the whole completion for it.
pub fn plan_completion(
    manifest: &ModrinthManifest,
    run_directory: &Path,
    excluded: Option<&BTreeSet<String>>,
) -> Result<CompletionPlan, ModrinthError> {
    plan_completion_with_updates(manifest, run_directory, excluded, &[])
}

/// Plans the files a completion of a modpack of Modrinth downloads, with the
/// files of the version the instance had.
///
/// The files of the old manifest the new one no longer names are removed, which
/// is the way an update of a modpack replaces a file.
pub fn plan_completion_with_updates(
    manifest: &ModrinthManifest,
    run_directory: &Path,
    excluded: Option<&BTreeSet<String>>,
    old: &[ModrinthFile],
) -> Result<CompletionPlan, ModrinthError> {
    let mut plan = CompletionPlan {
        total: manifest.files.len() as u64,
        ..CompletionPlan::default()
    };

    for file in &manifest.files {
        if file.side() == Some(UNSUPPORTED) {
            continue;
        }
        if excluded.is_some_and(|excluded| excluded.contains(&file.key())) {
            continue;
        }
        if file.downloads.is_empty() {
            plan.warnings
                .push(format!("The file {} names no address", file.path));
            continue;
        }

        let Some(path) = relative_path(run_directory, &file.path) else {
            return Err(ModrinthError::EscapingPath(file.path.clone()));
        };
        if path.exists() {
            continue;
        }

        // A mod the player disabled or replaced is still the mod of the pack.
        let mods_directory = run_directory.join("mods");
        if path.parent() == Some(mods_directory.as_path())
            && has_simple_mod(&mods_directory, file.file_name())
        {
            continue;
        }

        plan.downloads.push(CompletionDownload {
            urls: file.downloads.clone(),
            integrity: file
                .integrity_check()
                .map(|(algorithm, digest)| (algorithm.to_string(), digest.to_string())),
            path,
        });
    }

    plan.removals = removed_files(old, manifest, run_directory);

    Ok(plan)
}

/// Returns the files an update of a modpack of Modrinth removes.
///
/// The launcher removes a file of the old manifest that the new manifest no
/// longer names. The files are the same when the path, the digests, the
/// environment and the addresses are the same, which is the way Modrinth tells
/// two files apart. A file that is not in the instance is left out, because the
/// player may have removed it themselves.
pub fn removed_files(
    old: &[ModrinthFile],
    new: &ModrinthManifest,
    run_directory: &Path,
) -> Vec<PathBuf> {
    let mut removals = Vec::new();

    for file in old {
        if new.files.contains(file) {
            continue;
        }
        let Some(path) = relative_path(run_directory, &file.path) else {
            continue;
        };
        if path.exists() {
            removals.push(path);
        }
    }

    removals
}

/// Why a manifest of Modrinth cannot be read.
#[derive(Debug)]
pub enum ModrinthError {
    /// The archive holds no manifest.
    MissingManifest,
    /// The manifest is not a document the launcher reads.
    Malformed(String),
    /// The manifest is for another game.
    Game(String),
    /// The manifest names no version of the game.
    NoGame,
    /// A file of the manifest names no path.
    NoPath,
    /// A file of the manifest leaves the instance.
    EscapingPath(String),
    /// A file of the manifest names no digest.
    NoDigest(String),
    /// A file of the manifest names an environment without the client.
    NoEnvironment(String),
    /// A file of the manifest names no address.
    NoDownload(String),
    /// The archive cannot be read.
    Zip(crate::util::zip::ZipError),
}

impl std::fmt::Display for ModrinthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingManifest => {
                formatter.write_str("The archive holds no modrinth.index.json")
            }
            Self::Malformed(reason) => write!(formatter, "The manifest is malformed: {reason}"),
            Self::Game(game) => write!(formatter, "The manifest is for the game {game:?}"),
            Self::NoGame => formatter.write_str("The manifest names no version of the game"),
            Self::NoPath => formatter.write_str("A file of the manifest names no path"),
            Self::EscapingPath(path) => {
                write!(formatter, "The file {path:?} leaves the instance")
            }
            Self::NoDigest(path) => {
                write!(formatter, "The file {path:?} names no SHA-512 digest")
            }
            Self::NoEnvironment(path) => {
                write!(
                    formatter,
                    "The file {path:?} names an environment without a client"
                )
            }
            Self::NoDownload(path) => {
                write!(formatter, "The file {path:?} names no address")
            }
            Self::Zip(error) => write!(formatter, "The archive cannot be read: {error}"),
        }
    }
}

impl std::error::Error for ModrinthError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the digest of an empty file, which the tests use as a digest.
    fn sha512() -> String {
        Algorithm::Sha512.digest_to_string(b"")
    }

    /// Returns a manifest with the files the test names.
    fn manifest(path: &str, env: Option<&str>) -> String {
        let env = match env {
            Some(side) => format!(r#","env": {{"client": "{side}"}}"#),
            None => String::new(),
        };
        format!(
            r#"{{
                "game": "minecraft",
                "formatVersion": 1,
                "versionId": "1.0",
                "name": "World",
                "summary": "A pack",
                "files": [{{
                    "path": "{path}",
                    "hashes": {{"sha1": "{}", "sha512": "{}"}},
                    "downloads": ["https://example.com/file.jar"],
                    "fileSize": 3{env}
                }}],
                "dependencies": {{"minecraft": "1.20.1", "fabric-loader": "0.15.0"}}
            }}"#,
            Algorithm::Sha1.digest_to_string(b""),
            sha512()
        )
    }

    #[test]
    fn the_manifest_of_a_modpack_is_read() {
        let manifest = ModrinthManifest::read(&manifest("mods/sodium.jar", None))
            .expect("the manifest should be read");

        assert_eq!(manifest.name, "World");
        assert_eq!(manifest.summary(), "A pack");
        assert_eq!(manifest.game_version(), Some("1.20.1"));
        assert_eq!(manifest.loader_version("fabric-loader"), Some("0.15.0"));
        assert_eq!(manifest.loader_version("forge"), None);
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].file_name(), "sodium.jar");
        assert!(!manifest.files[0].optional());
        assert_eq!(
            manifest.files[0].download(),
            Some("https://example.com/file.jar")
        );
    }

    #[test]
    fn the_digest_of_a_file_is_the_one_modrinth_names_first() {
        let manifest = ModrinthManifest::read(&manifest("mods/a.jar", None))
            .expect("the manifest should be read");
        let (algorithm, digest) = manifest.files[0]
            .integrity_check()
            .expect("the file should name a digest");
        assert_eq!(algorithm, "SHA-512");
        assert_eq!(Some(digest), manifest.files[0].sha512());
    }

    #[test]
    fn a_file_the_player_does_not_install_is_left_out() {
        let text = manifest("mods/sodium.jar", Some(OPTIONAL));
        let manifest = ModrinthManifest::read(&text).expect("the manifest should be read");
        assert!(manifest.files[0].optional());
        assert_eq!(
            manifest.files[0].key(),
            format!("modrinth:{}:mods/sodium.jar", sha512())
        );

        let excluded: std::collections::BTreeSet<String> =
            [manifest.files[0].key()].into_iter().collect();
        assert_eq!(manifest.selected_files(Some(&excluded)).count(), 0);
        assert_eq!(manifest.selected_files(None).count(), 1);
    }

    #[test]
    fn a_path_that_leaves_the_instance_is_refused() {
        let text = manifest("../outside.jar", None);
        let error = ModrinthManifest::read(&text).expect_err("the manifest should be refused");
        assert!(matches!(error, ModrinthError::EscapingPath(_)));

        // A path with a directory is kept, because the launcher writes it.
        let text = manifest("config/a.toml", None);
        assert!(ModrinthManifest::read(&text).is_ok());
    }

    #[test]
    fn a_manifest_without_the_game_is_refused() {
        let text = manifest("mods/a.jar", None).replace("\"minecraft\": \"1.20.1\", ", "");
        let error = ModrinthManifest::read(&text).expect_err("the manifest should be refused");
        assert!(matches!(error, ModrinthError::NoGame));
    }

    #[test]
    fn a_file_without_a_digest_or_an_address_is_refused() {
        let text = manifest("mods/a.jar", None).replace(&sha512(), "not a digest");
        let error = ModrinthManifest::read(&text).expect_err("the manifest should be refused");
        assert!(matches!(error, ModrinthError::NoDigest(_)));

        let text = manifest("mods/a.jar", None).replace(
            "\"downloads\": [\"https://example.com/file.jar\"],",
            "\"downloads\": [],",
        );
        let error = ModrinthManifest::read(&text).expect_err("the manifest should be refused");
        assert!(matches!(error, ModrinthError::NoDownload(_)));
    }

    #[test]
    fn a_file_with_an_environment_without_a_client_is_refused() {
        let text = manifest("mods/a.jar", None).replace(
            "\"fileSize\": 3",
            "\"env\": {\"server\": \"required\"}, \"fileSize\": 3",
        );
        let error = ModrinthManifest::read(&text).expect_err("the manifest should be refused");
        assert!(matches!(error, ModrinthError::NoEnvironment(_)));
    }

    #[test]
    fn the_plan_downloads_the_files_the_instance_is_missing() {
        let text = manifest("mods/sodium.jar", None);
        let manifest = ModrinthManifest::read(&text).expect("the manifest should be read");
        let run = run_directory("plan");
        let plan = plan_completion(&manifest, &run, None).expect("the plan should be made");

        assert_eq!(plan.total, 1);
        assert_eq!(plan.downloads.len(), 1);
        assert_eq!(plan.downloads[0].path, run.join("mods").join("sodium.jar"));
        assert_eq!(
            plan.downloads[0]
                .integrity
                .as_ref()
                .map(|(algorithm, _)| algorithm.as_str()),
            Some("SHA-512")
        );

        let _ = std::fs::remove_dir_all(&run);
    }

    #[test]
    fn the_files_the_player_left_out_or_the_instance_holds_are_skipped() {
        let text = manifest("mods/sodium.jar", None);
        let pack = ModrinthManifest::read(&text).expect("the manifest should be read");
        let run = run_directory("skipped");
        std::fs::create_dir_all(run.join("mods")).expect("the mods should be made");
        std::fs::write(run.join("mods").join("sodium.jar"), b"a mod")
            .expect("the mod should be written");

        let plan = plan_completion(&pack, &run, None).expect("the plan should be made");
        assert_eq!(plan.downloads.len(), 0);

        // A file the client cannot use is not downloaded at all.
        std::fs::remove_file(run.join("mods").join("sodium.jar")).expect("the mod should go");
        let unsupported = ModrinthManifest::read(&manifest("mods/sodium.jar", Some(UNSUPPORTED)))
            .expect("the manifest should be read");
        let plan = plan_completion(&unsupported, &run, None).expect("the plan should be made");
        assert_eq!(plan.downloads.len(), 0);

        // A file the player did not select is left out, like a disabled mod.
        std::fs::write(
            run.join("mods").join("sodium.jar.disabled"),
            b"a disabled mod",
        )
        .expect("the mod should be written");
        let optional = ModrinthManifest::read(&manifest("mods/sodium.jar", Some(OPTIONAL)))
            .expect("the manifest should be read");
        let excluded: std::collections::BTreeSet<String> =
            [optional.files[0].key()].into_iter().collect();
        assert_eq!(
            plan_completion(&optional, &run, Some(&excluded))
                .expect("the plan should be made")
                .downloads
                .len(),
            0
        );
        assert_eq!(
            plan_completion(&optional, &run, None)
                .expect("the plan should be made")
                .downloads
                .len(),
            0
        );

        let _ = std::fs::remove_dir_all(&run);
    }

    #[test]
    fn an_update_of_a_modpack_removes_the_files_of_the_version_it_had() {
        let new = ModrinthManifest::read(&manifest("mods/kept.jar", None))
            .expect("the manifest should be read");
        let old_manifest = ModrinthManifest::read(&manifest("mods/gone.jar", None))
            .expect("the manifest should be read");
        let run = run_directory("plan-update");
        std::fs::create_dir_all(run.join("mods")).expect("the mods should be made");
        std::fs::write(run.join("mods").join("gone.jar"), b"a mod")
            .expect("the mod should be written");

        let plan = plan_completion_with_updates(&new, &run, None, &old_manifest.files)
            .expect("the plan should be made");

        assert_eq!(plan.removals, vec![run.join("mods").join("gone.jar")]);
        assert_eq!(plan.downloads.len(), 1);
        assert_eq!(plan.downloads[0].path, run.join("mods").join("kept.jar"));

        let _ = std::fs::remove_dir_all(&run);
    }

    #[test]
    fn an_update_removes_the_files_the_new_manifest_no_longer_names() {
        let old = ModrinthManifest::read(&manifest("mods/gone.jar", None))
            .expect("the manifest should be read");
        let new = ModrinthManifest::read(&manifest("mods/kept.jar", None))
            .expect("the manifest should be read");
        let run = run_directory("update");
        std::fs::create_dir_all(run.join("mods")).expect("the mods should be made");
        std::fs::write(run.join("mods").join("gone.jar"), b"a mod")
            .expect("the mod should be written");
        std::fs::write(run.join("mods").join("kept.jar"), b"a mod")
            .expect("the mod should be written");

        let removals = removed_files(&old.files, &new, &run);
        assert_eq!(removals, vec![run.join("mods").join("gone.jar")]);

        let _ = std::fs::remove_dir_all(&run);
    }

    /// Returns a directory of an instance no other test uses.
    fn run_directory(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("hmcl-rs-modrinth-{name}-{}", std::process::id()))
    }
}
