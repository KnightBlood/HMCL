//! The modpacks of the server of HMCL.
//!
//! The launcher can install an instance from the archive of a server: the
//! archive holds the files of the server and a manifest that names them, and
//! the manifest may name a server the files are downloaded from. The format
//! is the one of HMCL itself, so the launcher of today keeps reading the
//! archives of the launcher of the past.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::util::string::remove_suffix;

use super::configuration::{FileInformation, ModpackConfiguration};
use super::plan::{relative_path, sha1_of, DownloadRequest, UpdatePlan};

/// The name of the provider, as the launcher writes it in a configuration.
pub const PROVIDER_NAME: &str = "Server";

/// The file the manifest of a modpack of a server is written in.
pub const MANIFEST: &str = "server-manifest.json";

/// The manifest of a modpack of a server.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerModpackManifest {
    /// The name of the modpack.
    #[serde(default)]
    pub name: String,
    /// The author of the modpack.
    #[serde(default)]
    pub author: String,
    /// The version of the modpack.
    #[serde(default)]
    pub version: String,
    /// The description of the modpack.
    #[serde(default)]
    pub description: String,
    /// The address the files of the modpack are downloaded from.
    #[serde(default, rename = "fileApi")]
    pub file_api: Option<String>,
    /// The files the archive of the modpack holds.
    #[serde(default)]
    pub files: Option<Vec<FileInformation>>,
    /// The components of the instance.
    #[serde(default)]
    pub addons: Option<Vec<Addon>>,
}

impl ServerModpackManifest {
    /// Reads the manifest of a modpack.
    ///
    /// A manifest that names no server cannot be completed, but it can be
    /// installed from its archive, so only the files are required.
    pub fn read(text: &str) -> Result<Self, ServerModpackError> {
        let manifest: Self = serde_json::from_str(text)
            .map_err(|error| ServerModpackError::Malformed(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Reads the manifest of a modpack from the archive of the modpack.
    pub fn read_from_archive(archive: &Path) -> Result<Self, ServerModpackError> {
        let data = crate::util::zip::read_entry(archive, MANIFEST)
            .map_err(ServerModpackError::Zip)?
            .ok_or(ServerModpackError::MissingManifest)?;
        let text = String::from_utf8(data)
            .map_err(|error| ServerModpackError::Malformed(error.to_string()))?;
        Self::read(&text)
    }

    /// Refuses a manifest the launcher cannot install.
    pub fn validate(&self) -> Result<(), ServerModpackError> {
        if self.files.is_none() {
            return Err(ServerModpackError::MissingFiles);
        }
        Ok(())
    }

    /// Returns the files the archive holds.
    pub fn files(&self) -> &[FileInformation] {
        self.files.as_deref().unwrap_or_default()
    }

    /// Returns the components of the instance.
    pub fn addons(&self) -> &[Addon] {
        self.addons.as_deref().unwrap_or_default()
    }

    /// Returns the version of the game the modpack names.
    pub fn game_version(&self) -> Result<&str, ServerModpackError> {
        self.addons()
            .iter()
            .find(|addon| addon.id == crate::game::GameComponentType::Game.patch_id())
            .map(|addon| addon.version.as_str())
            .ok_or(ServerModpackError::NoGame)
    }

    /// Returns the address the files of the modpack are downloaded from.
    pub fn file_api(&self) -> Option<&str> {
        self.file_api
            .as_deref()
            .map(|api| remove_suffix(api, "/"))
            .filter(|api| !api.is_empty())
    }

    /// Returns the components the two versions of the modpack compare by.
    ///
    /// The launcher updates the components of the instance only when the
    /// version of one of them changed, so the components are read as a map.
    pub fn components(&self) -> std::collections::BTreeMap<&str, &str> {
        self.addons()
            .iter()
            .map(|addon| (addon.id.as_str(), addon.version.as_str()))
            .collect()
    }

    /// Returns whether the components of two versions of a modpack differ.
    pub fn components_changed(&self, other: &Self) -> bool {
        self.components() != other.components()
    }
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

/// The files an update of a modpack of a server downloads and removes.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerUpdate {
    /// The manifest of the new version of the modpack.
    pub manifest: ServerModpackManifest,
    /// The files the launcher downloads and removes.
    pub plan: UpdatePlan,
    /// Whether the components of the instance have to be installed again.
    pub components_changed: bool,
}

impl ServerUpdate {
    /// Returns the configuration of the instance after the update.
    pub fn configuration(&self) -> ModpackConfiguration<ServerModpackManifest> {
        ModpackConfiguration::new(
            self.manifest.clone(),
            PROVIDER_NAME,
            self.manifest.name.clone(),
            Some(self.manifest.version.clone()),
            self.manifest.files().to_vec(),
        )
    }
}

/// Returns whether a file of the mods of the instance is disabled.
///
/// A player disables a mod the modpack installed by renaming its file; the
/// launcher keeps the file as it is, because the player chose not to load the
/// mod, and downloading the file again would enable it.
pub fn is_disabled_mod(mods_directory: &Path, path: &Path) -> bool {
    if path.parent() != Some(mods_directory) {
        return false;
    }

    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    mods_directory
        .join(format!("{name}{}", crate::addon::DISABLED_EXTENSION))
        .exists()
        || mods_directory
            .join(format!("{name}{}", crate::addon::OLD_EXTENSION))
            .exists()
}

/// Compares the manifest of an instance with the manifest of the server of the
/// modpack.
///
/// A file is downloaded when the instance does not have it, when the player
/// deleted it, or when the server changed it and the player did not. A file the
/// server removed from the modpack is removed from the instance, and a mod the
/// player disabled is left alone.
pub fn plan_update(
    local: &ServerModpackManifest,
    remote: &ServerModpackManifest,
    run_directory: &Path,
) -> ServerUpdate {
    let mut plan = UpdatePlan::default();
    let remote_paths: std::collections::BTreeSet<&str> = remote
        .files()
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    let mods_directory = run_directory.join("mods");

    let Some(api) = remote.file_api() else {
        plan.warnings
            .push("the manifest names no server, so no file can be downloaded".to_string());
        return ServerUpdate {
            manifest: remote.clone(),
            plan,
            components_changed: local.components_changed(remote),
        };
    };

    for file in remote.files() {
        let Some(path) = relative_path(run_directory, &file.path) else {
            plan.warnings
                .push(format!("the file `{}` leaves the instance", file.path));
            continue;
        };

        if is_disabled_mod(&mods_directory, &path) {
            continue;
        }

        let download = match local.files().iter().find(|old| old.path == file.path) {
            // The instance does not have the file, so the server has one the
            // player deleted or never had.
            None => true,
            Some(_) if !path.is_file() => true,
            Some(old) => {
                let actual = sha1_of(&path);
                let old_hash = old.hash.as_deref().unwrap_or_default();
                let new_hash = file.hash.as_deref().unwrap_or_default();

                // The file is replaced only when the server changed it and the
                // player did not: a file the player changed is what the player
                // expects.
                old_hash != new_hash && old_hash.eq_ignore_ascii_case(&actual.unwrap_or_default())
            }
        };

        if download {
            plan.downloads.push(DownloadRequest {
                url: format!("{api}/overrides/{}", file.path),
                path,
                sha1: file.hash.clone(),
            });
        }
    }

    // The files the server removed from the modpack are removed from the
    // instance, unless the player disabled or changed them.
    for file in local.files() {
        if remote_paths.contains(file.path.as_str()) {
            continue;
        }
        let Some(path) = relative_path(run_directory, &file.path) else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        let unchanged = file.hash.as_deref().is_some_and(|hash| {
            sha1_of(&path).is_some_and(|actual| actual.eq_ignore_ascii_case(hash))
        });
        if unchanged {
            plan.removals.push(path);
        }
    }

    ServerUpdate {
        manifest: remote.clone(),
        plan,
        components_changed: local.components_changed(remote),
    }
}

/// The manifest of a modpack of a server the launcher cannot read.
#[derive(Debug)]
pub enum ServerModpackError {
    /// The manifest is not valid JSON.
    Malformed(String),
    /// The manifest names no file.
    MissingFiles,
    /// The manifest names no version of the game.
    NoGame,
    /// The archive holds no manifest of a server.
    MissingManifest,
    /// The archive cannot be read.
    Zip(crate::util::zip::ZipError),
}

impl std::fmt::Display for ServerModpackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(message) => {
                write!(
                    formatter,
                    "the manifest of the modpack is malformed: {message}"
                )
            }
            Self::MissingFiles => formatter.write_str("the manifest of the modpack names no file"),
            Self::NoGame => {
                formatter.write_str("the manifest of the modpack names no version of Minecraft")
            }
            Self::MissingManifest => write!(
                formatter,
                "the archive holds no `{MANIFEST}`, so it is not a modpack of a server"
            ),
            Self::Zip(error) => write!(formatter, "the archive cannot be read: {error}"),
        }
    }
}

impl std::error::Error for ServerModpackError {}

/// Returns a path under a directory of an instance.
///
/// The helper is used by the tests and by the launcher when it copies the files
/// of an archive; a path that leaves the directory is refused.
pub fn instance_path(run_directory: &Path, path: &str) -> Option<PathBuf> {
    relative_path(run_directory, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::digest::Algorithm;

    /// Returns a directory of the tests that the other tests do not use.
    fn directory(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("hmcl-rs-server-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Writes a file of the instance and returns its digest.
    fn write_file(base: &Path, path: &str, text: &str) -> String {
        let file = base.join(path);
        std::fs::create_dir_all(file.parent().expect("the file has a parent"))
            .expect("the directory should be created");
        std::fs::write(&file, text).expect("the file should be written");
        Algorithm::Sha1.digest_to_string(text.as_bytes())
    }

    /// Returns a file of a modpack with the digest of the text.
    fn file(path: &str, text: &str) -> FileInformation {
        FileInformation::new(path, Algorithm::Sha1.digest_to_string(text.as_bytes()))
    }

    /// Returns a manifest with the files the test names.
    fn manifest(files: Vec<FileInformation>) -> ServerModpackManifest {
        ServerModpackManifest {
            name: "World".to_string(),
            author: "Author".to_string(),
            version: "1.0".to_string(),
            file_api: Some("https://example.com/pack/".to_string()),
            files: Some(files),
            addons: Some(vec![
                Addon {
                    id: "game".to_string(),
                    version: "1.20.1".to_string(),
                },
                Addon {
                    id: "forge".to_string(),
                    version: "47.1.0".to_string(),
                },
            ]),
            ..ServerModpackManifest::default()
        }
    }

    #[test]
    fn the_manifest_of_a_server_modpack_is_read() {
        let text = r#"{
            "name": "World",
            "author": "Author",
            "version": "1.0",
            "description": "A modpack",
            "fileApi": "https://example.com/pack/",
            "files": [ { "path": "mods/a.jar", "hash": "abc" } ],
            "addons": [ { "id": "game", "version": "1.20.1" } ]
        }"#;

        let manifest = ServerModpackManifest::read(text).expect("the manifest should be read");
        assert_eq!(
            manifest.game_version().expect("the game should be named"),
            "1.20.1"
        );
        assert_eq!(manifest.file_api(), Some("https://example.com/pack"));
        assert_eq!(manifest.files().len(), 1);
        assert_eq!(manifest.components().len(), 1);
    }

    #[test]
    fn a_manifest_without_files_is_refused() {
        let text = r#"{ "name": "World", "addons": [] }"#;
        assert!(matches!(
            ServerModpackManifest::read(text),
            Err(ServerModpackError::MissingFiles)
        ));
    }

    #[test]
    fn a_modpack_without_a_game_is_refused() {
        let mut manifest = manifest(Vec::new());
        manifest.addons = Some(vec![Addon {
            id: "forge".to_string(),
            version: "47.1.0".to_string(),
        }]);
        assert!(matches!(
            manifest.game_version(),
            Err(ServerModpackError::NoGame)
        ));
    }

    #[test]
    fn the_manifest_of_an_archive_is_read() {
        let base = directory("archive");
        let text = serde_json::to_string(&manifest(vec![file("mods/a.jar", "a")]))
            .expect("the manifest should be written");

        let archive = base.join("pack.zip");
        let mut writer =
            crate::util::zip::ZipWriter::create(&archive).expect("the archive should be created");
        writer
            .add_file(MANIFEST, text.as_bytes())
            .expect("the manifest should be added");
        writer.finish().expect("the archive should be written");

        let read = ServerModpackManifest::read_from_archive(&archive)
            .expect("the manifest should be read");
        assert_eq!(read.name, "World");

        let empty = base.join("empty.zip");
        let mut writer =
            crate::util::zip::ZipWriter::create(&empty).expect("the archive should be created");
        writer
            .add_file("a.txt", b"a")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");
        assert!(matches!(
            ServerModpackManifest::read_from_archive(&empty),
            Err(ServerModpackError::MissingManifest)
        ));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_server_changed_is_downloaded_when_the_player_did_not_change_it() {
        let base = directory("changed");
        let old_hash = write_file(&base, "mods/a.jar", "old");

        let local = manifest(vec![FileInformation::new("mods/a.jar", old_hash)]);
        let remote = manifest(vec![file("mods/a.jar", "new")]);

        let update = plan_update(&local, &remote, &base);
        assert_eq!(update.plan.downloads.len(), 1);
        assert_eq!(
            update.plan.downloads[0].url,
            "https://example.com/pack/overrides/mods/a.jar"
        );
        assert_eq!(update.plan.downloads[0].path, base.join("mods/a.jar"));
        assert!(!update.components_changed);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_player_changed_is_kept() {
        let base = directory("changed-by-player");
        write_file(&base, "mods/a.jar", "changed by the player");

        let local = manifest(vec![FileInformation::new(
            "mods/a.jar",
            Algorithm::Sha1.digest_to_string(b"the original"),
        )]);
        let remote = manifest(vec![file("mods/a.jar", "new")]);

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.downloads.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mod_the_player_disabled_is_left_alone() {
        let base = directory("disabled");
        write_file(&base, "mods/a.jar", "the mod");
        std::fs::rename(base.join("mods/a.jar"), base.join("mods/a.jar.disabled"))
            .expect("the file should be renamed");

        let local = manifest(vec![file("mods/a.jar", "the mod")]);
        let remote = manifest(vec![file("mods/a.jar", "new")]);

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.downloads.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_server_removed_is_removed_when_the_player_did_not_change_it() {
        let base = directory("removed");
        write_file(&base, "mods/a.jar", "the file");
        write_file(&base, "mods/b.jar", "the other file");

        let local = manifest(vec![
            file("mods/a.jar", "the file"),
            file("mods/b.jar", "the other file"),
        ]);
        let remote = manifest(vec![file("mods/a.jar", "the file")]);

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.downloads.is_empty());
        assert_eq!(update.plan.removals, vec![base.join("mods/b.jar")]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_player_changed_is_not_removed() {
        let base = directory("removed-changed");
        write_file(&base, "mods/b.jar", "changed by the player");

        let local = manifest(vec![FileInformation::new(
            "mods/b.jar",
            Algorithm::Sha1.digest_to_string(b"the original"),
        )]);
        let remote = manifest(Vec::new());

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.removals.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_components_are_compared() {
        let local = manifest(Vec::new());
        let mut remote = local.clone();
        assert!(!local.components_changed(&remote));

        remote.addons = Some(vec![Addon {
            id: "game".to_string(),
            version: "1.21".to_string(),
        }]);
        assert!(local.components_changed(&remote));

        let update = plan_update(&local, &remote, &base_of("components"));
        assert!(update.components_changed);
    }

    /// Returns a directory of the tests.
    fn base_of(name: &str) -> PathBuf {
        let path = directory(name);
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn a_modpack_without_a_server_removes_and_downloads_nothing() {
        let base = directory("no-server");
        let local = manifest(Vec::new());
        let mut remote = manifest(vec![file("mods/a.jar", "the file")]);
        remote.file_api = None;

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.downloads.is_empty());
        assert!(update.plan.removals.is_empty());
        assert_eq!(update.plan.warnings.len(), 1);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn no_file_is_written_outside_the_instance() {
        let base = directory("outside");
        let local = manifest(Vec::new());
        let remote = manifest(vec![file("../outside.jar", "the file")]);

        let update = plan_update(&local, &remote, &base);
        assert!(update.plan.downloads.is_empty());
        assert_eq!(update.plan.warnings.len(), 1);
        assert_eq!(instance_path(&base, "../outside.jar"), None);

        let _ = std::fs::remove_dir_all(&base);
    }
}
