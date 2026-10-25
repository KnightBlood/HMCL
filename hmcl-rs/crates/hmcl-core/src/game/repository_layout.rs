//! The layout of the directories the game uses.
//!
//! The launcher keeps the files of the instances, the libraries and the
//! assets in a directory the user chooses; the layout tells where every kind
//! of file goes. The layout of the official launcher is supported; other
//! launchers use other layouts.

use std::path::{Path, PathBuf};

use crate::download::game::{Artifact, Library};

/// The layout of the directories of a game directory.
///
/// The layout tells where the launcher writes the files of the instances, the
/// libraries, the assets and the logs. The instances of the game are named by
/// their identifiers.
pub trait RepositoryLayout {
    /// Returns the directory the game directory keeps its files in.
    fn base_directory(&self) -> &Path;

    /// Returns the directory that holds the files of an instance.
    fn instance_root(&self, id: &str) -> PathBuf;

    /// Returns the directory that holds the libraries of every instance.
    fn libraries_directory(&self) -> PathBuf;

    /// Returns the file of a library in the shared directory of the
    /// libraries.
    fn artifact_file(&self, artifact: &Artifact) -> PathBuf {
        self.libraries_directory().join(artifact.path())
    }

    /// Returns the file of a library an instance uses.
    ///
    /// A library the instance carries itself, such as the installer of a mod
    /// loader, is stored with the instance; the other libraries are shared.
    fn library_file(&self, owner: &str, library: &Library) -> PathBuf {
        match library.artifact() {
            Ok(artifact) if library.hint.as_deref() == Some("local") => self
                .instance_root(owner)
                .join("libraries")
                .join(artifact.path()),
            Ok(artifact) => self.artifact_file(&artifact),
            Err(_) => self.libraries_directory().join(&library.name),
        }
    }

    /// Returns the directory that holds the assets of every instance.
    fn asset_directory(&self) -> PathBuf;

    /// Returns the file that holds an index of the assets.
    fn asset_index_file(&self, asset_id: &str) -> PathBuf;

    /// Returns the file of an asset.
    ///
    /// The assets are named by their digests; the launcher keeps them in
    /// directories named by the first two digits of the digest.
    fn asset_object(&self, hash: &str) -> PathBuf;

    /// Returns the file that holds the settings of the log of an instance.
    fn logging_object(&self, asset_id: &str, file_id: &str) -> PathBuf;
}

/// The layout of the official launcher.
///
/// The instances are kept in `versions/<id>`, the libraries in `libraries`
/// and the assets in `assets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficialLayout {
    /// The directory the game directory keeps its files in.
    base: PathBuf,
}

impl OfficialLayout {
    /// Creates the layout of a game directory.
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    /// Returns the directory that holds the files of an instance.
    pub fn version_directory(&self, id: &str) -> PathBuf {
        self.base.join("versions").join(id)
    }

    /// Returns the file of the details of an instance.
    pub fn version_json(&self, id: &str) -> PathBuf {
        self.version_directory(id).join(format!("{id}.json"))
    }

    /// Returns the file of the game of an instance.
    pub fn version_jar(&self, id: &str) -> PathBuf {
        self.version_directory(id).join(format!("{id}.jar"))
    }

    /// Returns the directory the native libraries of an instance are
    /// extracted into.
    pub fn natives_directory(&self, id: &str) -> PathBuf {
        self.version_directory(id).join("natives")
    }

    /// Returns the directory the game of an instance is run in.
    ///
    /// The instances share the directory of the game directory, so the worlds
    /// and the options of the game are shared too.
    pub fn run_directory(&self) -> PathBuf {
        self.base.clone()
    }

    /// Returns the directory the mods of the game are kept in.
    pub fn mods_directory(&self) -> PathBuf {
        self.base.join("mods")
    }

    /// Returns the directory the resource packs of the game are kept in.
    pub fn resource_packs_directory(&self) -> PathBuf {
        self.base.join("resourcepacks")
    }

    /// Returns the directory the shader packs of the game are kept in.
    pub fn shader_packs_directory(&self) -> PathBuf {
        self.base.join("shaderpacks")
    }

    /// Returns the directory the worlds of the game are kept in.
    pub fn saves_directory(&self) -> PathBuf {
        self.base.join("saves")
    }

    /// Returns the directory the logs of the game are kept in.
    pub fn logs_directory(&self) -> PathBuf {
        self.base.join("logs")
    }

    /// Returns the directory the reports of the crashes are kept in.
    pub fn crash_reports_directory(&self) -> PathBuf {
        self.base.join("crash-reports")
    }

    /// Returns the directory the screenshots of the game are kept in.
    pub fn screenshots_directory(&self) -> PathBuf {
        self.base.join("screenshots")
    }

    /// Returns the file that holds the settings of a modpack an instance was
    /// installed from.
    pub fn modpack_configuration_file(&self, id: &str) -> PathBuf {
        self.version_directory(id).join("modpack.json")
    }
}

impl RepositoryLayout for OfficialLayout {
    fn base_directory(&self) -> &Path {
        &self.base
    }

    fn instance_root(&self, id: &str) -> PathBuf {
        self.version_directory(id)
    }

    fn libraries_directory(&self) -> PathBuf {
        self.base.join("libraries")
    }

    fn asset_directory(&self) -> PathBuf {
        self.base.join("assets")
    }

    fn asset_index_file(&self, asset_id: &str) -> PathBuf {
        self.asset_directory()
            .join("indexes")
            .join(format!("{asset_id}.json"))
    }

    fn asset_object(&self, hash: &str) -> PathBuf {
        let directory = hash.get(..2).unwrap_or(hash);
        self.asset_directory()
            .join("objects")
            .join(directory)
            .join(hash)
    }

    fn logging_object(&self, asset_id: &str, file_id: &str) -> PathBuf {
        let _ = asset_id;
        self.asset_directory().join("log_configs").join(file_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> OfficialLayout {
        OfficialLayout::new("/games/.minecraft")
    }

    #[test]
    fn the_files_of_the_instances_are_placed() {
        let layout = layout();

        assert_eq!(
            layout.instance_root("1.20.1"),
            PathBuf::from("/games/.minecraft/versions/1.20.1")
        );
        assert_eq!(
            layout.version_json("1.20.1"),
            PathBuf::from("/games/.minecraft/versions/1.20.1/1.20.1.json")
        );
        assert_eq!(
            layout.version_jar("1.20.1"),
            PathBuf::from("/games/.minecraft/versions/1.20.1/1.20.1.jar")
        );
        assert_eq!(
            layout.modpack_configuration_file("1.20.1"),
            PathBuf::from("/games/.minecraft/versions/1.20.1/modpack.json")
        );
    }

    #[test]
    fn the_libraries_are_shared() {
        let layout = layout();
        let artifact = Artifact::new("com.example", "library", "1.0", None, None);

        assert_eq!(
            layout.artifact_file(&artifact),
            PathBuf::from("/games/.minecraft/libraries/com/example/library/1.0/library-1.0.jar")
        );
        assert_eq!(layout.base_directory(), Path::new("/games/.minecraft"));
    }

    #[test]
    fn the_libraries_of_the_instances_are_kept_with_the_instance() {
        let layout = layout();
        let library = Library {
            name: "com.example:library:1.0".to_string(),
            hint: Some("local".to_string()),
            ..Library::default()
        };

        assert_eq!(
            layout.library_file("1.20.1", &library),
            PathBuf::from(
                "/games/.minecraft/versions/1.20.1/libraries/com/example/library/1.0/library-1.0.jar"
            )
        );

        let shared = Library {
            name: "com.example:library:1.0".to_string(),
            ..Library::default()
        };
        assert_eq!(
            layout.library_file("1.20.1", &shared),
            PathBuf::from("/games/.minecraft/libraries/com/example/library/1.0/library-1.0.jar")
        );
    }

    #[test]
    fn the_assets_are_placed() {
        let layout = layout();
        let hash = "0123456789abcdef0123456789abcdef01234567";

        assert_eq!(
            layout.asset_index_file("1.20"),
            PathBuf::from("/games/.minecraft/assets/indexes/1.20.json")
        );
        assert_eq!(
            layout.asset_object(hash),
            PathBuf::from(format!("/games/.minecraft/assets/objects/01/{hash}"))
        );
        assert_eq!(
            layout.logging_object("1.20", "client-1.12.xml"),
            PathBuf::from("/games/.minecraft/assets/log_configs/client-1.12.xml")
        );
    }

    #[test]
    fn the_directories_of_the_game_are_placed() {
        let layout = layout();

        assert_eq!(layout.run_directory(), PathBuf::from("/games/.minecraft"));
        assert_eq!(
            layout.mods_directory(),
            PathBuf::from("/games/.minecraft/mods")
        );
        assert_eq!(
            layout.resource_packs_directory(),
            PathBuf::from("/games/.minecraft/resourcepacks")
        );
        assert_eq!(
            layout.shader_packs_directory(),
            PathBuf::from("/games/.minecraft/shaderpacks")
        );
        assert_eq!(
            layout.saves_directory(),
            PathBuf::from("/games/.minecraft/saves")
        );
        assert_eq!(
            layout.logs_directory(),
            PathBuf::from("/games/.minecraft/logs")
        );
        assert_eq!(
            layout.crash_reports_directory(),
            PathBuf::from("/games/.minecraft/crash-reports")
        );
        assert_eq!(
            layout.screenshots_directory(),
            PathBuf::from("/games/.minecraft/screenshots")
        );
    }
}
