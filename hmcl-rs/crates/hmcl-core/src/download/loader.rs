//! The mod loaders that publish the changes they make as metadata.
//!
//! Fabric, Quilt and Legacy Fabric all follow the same shape: a service
//! publishes, for every version of the loader, the main class the game is
//! started with, the libraries the loader adds and, for the loaders that wrap
//! the game, the tweak class the wrapper is called with. The launcher reads
//! that metadata, writes it into a patch of the instance and downloads the
//! libraries the patch adds.
//!
//! The three loaders differ in the repository their libraries are kept in and
//! in the extra fields their metadata holds, so the launcher asks the loader
//! for the repository of a library rather than guessing it.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::downloader::TaskDownloader;
use super::game::{Arguments, Artifact, DownloadInfo, InstancePatch, Library, LibraryDownloads};
use super::task::{DownloadCandidates, DownloadError, Fetcher, FileDownloadTask};
use super::DownloadProvider;
use crate::game::component::GameComponentType;

/// The priority a patch of a mod loader wins over the version of the game
/// with.
///
/// The libraries and the arguments of the patch are added to the ones of the
/// version, but the main class of the patch replaces the main class of the
/// version, so the game is started by the loader.
pub const LOADER_PATCH_PRIORITY: i32 = 30000;

/// The repository the libraries of Fabric and of the loaders that follow its
/// shape are kept in.
const FABRIC_MAVEN_URL: &str = "https://maven.fabricmc.net/";

/// The repository the libraries of Quilt are kept in.
const QUILT_MAVEN_URL: &str = "https://maven.quiltmc.org/repository/release/";

/// The repository the libraries of Legacy Fabric are kept in.
const LEGACY_FABRIC_MAVEN_URL: &str = "https://maven.legacyfabric.net/";

/// The mod loader whose metadata the launcher read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModLoader {
    /// The loader of Fabric.
    Fabric,
    /// The loader of Quilt.
    Quilt,
    /// The loader of Legacy Fabric.
    LegacyFabric,
}

impl ModLoader {
    /// Returns the mod loader the identifier of a patch names.
    pub fn of_patch_id(id: &str) -> Option<Self> {
        match GameComponentType::from_patch_id(id)? {
            GameComponentType::Fabric => Some(Self::Fabric),
            GameComponentType::Quilt => Some(Self::Quilt),
            GameComponentType::LegacyFabric => Some(Self::LegacyFabric),
            _ => None,
        }
    }

    /// Returns the component the patch of the loader belongs to.
    pub const fn component(self) -> GameComponentType {
        match self {
            Self::Fabric => GameComponentType::Fabric,
            Self::Quilt => GameComponentType::Quilt,
            Self::LegacyFabric => GameComponentType::LegacyFabric,
        }
    }

    /// Returns the repository the library of the loader is kept in.
    ///
    /// The metadata of Quilt names the intermediary with the group of Fabric,
    /// because Quilt is built on the intermediary of Fabric; the launcher
    /// therefore asks the group of the library rather than the loader whose
    /// metadata it reads.
    pub fn repository_of(self, maven: &str) -> String {
        let group = Artifact::from_descriptor(maven)
            .ok()
            .map(|artifact| artifact.group);

        match (self, group.as_deref()) {
            (Self::Quilt, Some("org.quiltmc")) => QUILT_MAVEN_URL,
            (Self::LegacyFabric, Some("net.legacyfabric")) => LEGACY_FABRIC_MAVEN_URL,
            _ => FABRIC_MAVEN_URL,
        }
        .to_string()
    }
}

/// The metadata a service of a mod loader publishes about one of its
/// versions.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LoaderMeta {
    /// The coordinates of the loader itself.
    pub loader: LoaderVersion,
    /// The coordinates of the intermediary the loader is built on, if the
    /// service names one.
    #[serde(default)]
    pub intermediary: Option<LoaderVersion>,
    /// The changes the loader makes to the version of the game.
    #[serde(rename = "launcherMeta")]
    pub launcher_meta: LauncherMeta,
}

/// A version of a library a service of a mod loader names.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LoaderVersion {
    /// The coordinates of the library, as the repositories of Java name it.
    pub maven: String,
}

/// The changes a mod loader makes to a version of the game.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LauncherMeta {
    /// The class the game is started with.
    #[serde(rename = "mainClass")]
    pub main_class: MainClass,
    /// The libraries the loader adds, by the side of the game they are used
    /// on.
    #[serde(default)]
    pub libraries: Libraries,
    /// The classes the game is wrapped with, if the loader wraps the game.
    #[serde(default)]
    pub launchwrapper: Option<LaunchWrapper>,
}

/// The class the game is started with.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum MainClass {
    /// The class the game is started with, whatever the side of the game is.
    Text(String),
    /// The class of every side of the game.
    Object {
        /// The class the client is started with.
        client: String,
    },
}

impl MainClass {
    /// Returns the class the client is started with.
    pub fn client(&self) -> &str {
        match self {
            Self::Text(text) => text,
            Self::Object { client } => client,
        }
    }
}

/// The libraries a mod loader adds, by the side of the game they are used
/// on.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Libraries {
    /// The libraries both sides of the game use.
    #[serde(default)]
    pub common: Vec<LoaderLibrary>,
    /// The libraries the server uses.
    ///
    /// The client runs the libraries of the server as well, which is why the
    /// launcher reads this list; the list of the client is not read at all,
    /// because the installer of the loader does not tell what it wrote there.
    #[serde(default)]
    pub server: Vec<LoaderLibrary>,
}

/// A library the metadata of a mod loader names.
///
/// The service of a loader writes the digest and the size of a library next to
/// its name, without the rest of the fields a version of the game writes about
/// its libraries, so the launcher reads them separately and writes them into
/// the library it keeps.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LoaderLibrary {
    /// The library, as the versions of the game name it.
    #[serde(flatten)]
    pub library: Library,
    /// The digest of the file of the library.
    #[serde(default)]
    pub sha1: Option<String>,
    /// The size of the file of the library.
    #[serde(default)]
    pub size: Option<u64>,
}

impl LoaderLibrary {
    /// Returns the library the metadata names, with the digest and the size
    /// of its file written into it.
    ///
    /// The digest of a library the service does not publish is not known, so
    /// the file is downloaded without being held to one.
    pub fn into_library(self) -> Library {
        Library {
            downloads: Some(LibraryDownloads {
                artifact: Some(DownloadInfo {
                    url: String::new(),
                    path: None,
                    sha1: self.sha1,
                    size: self.size,
                }),
                classifiers: None,
            }),
            ..self.library
        }
    }
}

/// The classes a mod loader wraps the game with.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct LaunchWrapper {
    /// The classes, by the side of the game.
    #[serde(default)]
    pub tweakers: Tweakers,
}

/// The classes the game is wrapped with, by the side of the game.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Tweakers {
    /// The classes the client is wrapped with, in the order they are applied.
    #[serde(default)]
    pub client: Vec<String>,
}

/// Reads the patch a mod loader publishes about one of its versions.
///
/// The metadata names the main class of the client, the class of the wrapper
/// the game is started through, if there is one, and the libraries both
/// sides of the game use. The loader and the intermediary are added to those
/// libraries, because the service names them outside of the list.
pub fn loader_patch(loader: ModLoader, version: &str, meta: &LoaderMeta) -> InstancePatch {
    let launcher_meta = &meta.launcher_meta;
    let component = loader.component();

    let mut libraries: Vec<Library> = launcher_meta
        .libraries
        .common
        .iter()
        .chain(launcher_meta.libraries.server.iter())
        .cloned()
        .map(LoaderLibrary::into_library)
        .collect();

    if let Some(intermediary) = &meta.intermediary {
        libraries.push(Library {
            name: intermediary.maven.clone(),
            url: Some(loader.repository_of(&intermediary.maven)),
            ..Library::default()
        });
    }

    libraries.push(Library {
        name: meta.loader.maven.clone(),
        url: Some(loader.repository_of(&meta.loader.maven)),
        ..Library::default()
    });

    let mut game = Vec::new();
    if let Some(tweaker) = client_tweaker(launcher_meta) {
        game.push(super::game::Argument::Text("--tweakClass".to_string()));
        game.push(super::game::Argument::Text(tweaker));
    }

    InstancePatch {
        id: Some(component.patch_id().to_string()),
        version: Some(version.to_string()),
        priority: LOADER_PATCH_PRIORITY,
        main_class: Some(launcher_meta.main_class.client().to_string()),
        arguments: Some(Arguments {
            game: Some(game),
            jvm: None,
        }),
        libraries,
        ..InstancePatch::default()
    }
}

/// Returns the class the game is wrapped with, if the loader wraps the game.
fn client_tweaker(launcher_meta: &LauncherMeta) -> Option<String> {
    launcher_meta
        .launchwrapper
        .as_ref()?
        .tweakers
        .client
        .first()
        .cloned()
}

/// Returns the path a library of a mod loader is kept at, under the
/// directory of the libraries.
///
/// The service of a loader names the repository of its libraries and the
/// digest and the size of every file, but not the path of the file under the
/// repository; the path is the one the coordinates of the library name.
pub fn library_path(library: &Library) -> String {
    library
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.artifact.as_ref())
        .and_then(|artifact| artifact.path.clone())
        .or_else(|| library.artifact().ok().map(|artifact| artifact.path()))
        .unwrap_or_else(|| library.name.replace(':', "/"))
}

/// Returns the address the file of a library of a mod loader is downloaded
/// from.
///
/// The metadata names the address of the repository of the library; a
/// library that names no address is kept on the machine of the player.
pub fn library_url(library: &Library) -> Option<String> {
    library
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.artifact.as_ref())
        .map(|artifact| artifact.url.clone())
        .filter(|url| !url.is_empty())
        .or_else(|| {
            let mut url = library.url.clone()?;
            if !url.ends_with('/') {
                url.push('/');
            }
            url.push_str(&library_path(library));
            Some(url)
        })
}

/// Returns the digest and the size of the file of a library of a mod
/// loader.
///
/// The digest of a file the service does not publish is not known, so the
/// file is downloaded without being held to one.
pub fn library_download(library: &Library) -> DownloadInfo {
    let artifact = library
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.artifact.clone());

    DownloadInfo {
        url: library_url(library).unwrap_or_default(),
        path: Some(library_path(library)),
        sha1: artifact
            .as_ref()
            .and_then(|artifact| artifact.sha1())
            .map(str::to_string),
        size: artifact.and_then(|artifact| artifact.size),
    }
}

/// Returns the libraries of a mod loader with the address, the digest and
/// the size of their files.
///
/// The libraries the launcher reads from the metadata of the loader are the
/// ones the installer wrote; the launcher writes the address of every file
/// into the library, so the file is downloaded without reading the metadata
/// again.
pub fn resolved_libraries(libraries: &[Library]) -> Vec<Library> {
    libraries
        .iter()
        .map(|library| Library {
            url: library.url.clone(),
            downloads: Some(LibraryDownloads {
                artifact: Some(library_download(library)),
                classifiers: None,
            }),
            ..library.clone()
        })
        .collect()
}

/// The metadata of a mod loader that was read from its service.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledLoader {
    /// The changes the loader makes to the version of the game.
    pub patch: InstancePatch,
    /// The files of the libraries the patch adds, in the order they were
    /// downloaded.
    pub files: Vec<PathBuf>,
}

/// Installs a mod loader that publishes its metadata.
///
/// The launcher reads the metadata of the loader from its service, writes the
/// changes of the loader into a patch of the instance and downloads the
/// libraries the patch adds, because the installer of the game reads them
/// before the instance is resolved.
///
/// The metadata is kept in the cache of the launcher, so an installation
/// that is repeated for another instance of the same version of the game does
/// not ask the service again.
pub fn install_loader(
    loader: ModLoader,
    game_version: &str,
    loader_version: &str,
    provider: &DownloadProvider,
    fetcher: &dyn Fetcher,
    downloader: &TaskDownloader<'_>,
    libraries_directory: &Path,
) -> Result<InstalledLoader, DownloadError> {
    let meta = read_loader_meta(
        loader,
        game_version,
        loader_version,
        provider,
        fetcher,
        downloader,
    )?;

    let patch = loader_patch(loader, loader_version, &meta);
    let libraries = resolved_libraries(&patch.libraries);
    let mut files = Vec::with_capacity(libraries.len());

    for library in &libraries {
        let path = libraries_directory.join(library_path(library));
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)
                .map_err(|error| DownloadError::Io(error.to_string()))?;
        }

        downloader.fetch_library(library, &path)?;
        files.push(path);
    }

    Ok(InstalledLoader {
        patch: InstancePatch { libraries, ..patch },
        files,
    })
}

/// Reads the metadata a mod loader publishes about one of its versions.
///
/// The metadata is a small file that is only read when a loader is
/// installed, so the launcher asks the service for it and keeps the answer in
/// the cache of the servers rather than in the directory of the libraries. The
/// server is asked whether the file changed, so an installation that is
/// repeated does not download the metadata again.
///
/// The metadata is written to a place of its own next to the other files of
/// the servers, because the answer of the service is not a file the game
/// reads; the file is removed when the metadata has been read.
fn read_loader_meta(
    loader: ModLoader,
    game_version: &str,
    loader_version: &str,
    provider: &DownloadProvider,
    fetcher: &dyn Fetcher,
    downloader: &TaskDownloader<'_>,
) -> Result<LoaderMeta, DownloadError> {
    let candidates = match loader {
        ModLoader::Fabric => provider.loader_meta_candidates(game_version, loader_version),
        ModLoader::Quilt => provider.quilt_meta_candidates(game_version, loader_version),
        ModLoader::LegacyFabric => {
            provider.legacy_fabric_meta_candidates(game_version, loader_version)
        }
    };
    let temporary = loader_meta_path(downloader, &candidates);

    let task = FileDownloadTask::new(candidates, temporary.clone()).with_remote_caching(true);
    let downloaded = task.download(fetcher, Some(&mut downloader.cache_mut().cache_mut()))?;

    let text = std::fs::read_to_string(&downloaded.path)
        .map_err(|error| DownloadError::Io(error.to_string()))?;

    // The file of the cache is kept, so the place the metadata was written to
    // is only a copy that is not needed once the metadata has been read.
    let _ = std::fs::remove_file(&downloaded.path);

    serde_json::from_str(&text).map_err(|error| {
        DownloadError::Failed(format!(
            "the metadata of {loader:?} {loader_version} for {game_version} is broken: {error}"
        ))
    })
}

/// Returns the place the metadata of a mod loader is written to.
///
/// The place names the version of the loader the address ends with, so the
/// metadata of two versions of the loader do not share a file. The file is
/// written inside the cache of the servers, where the temporary files of the
/// downloads are written as well.
fn loader_meta_path(downloader: &TaskDownloader<'_>, candidates: &DownloadCandidates) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let number = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let digest = candidates
        .primary()
        .url
        .rsplit('/')
        .next()
        .unwrap_or("loader")
        .to_string();

    downloader.cache().cache().cache_directory().join(format!(
        ".hmcl-loader-meta-{}-{number}-{digest}.json",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::library_cache::DefaultCacheRepository;
    use crate::download::provider::SourcePreference;
    use serde_json::json;

    /// Returns the metadata of a version of the loader of Fabric.
    fn meta() -> LoaderMeta {
        let text = json!({
            "loader": {"maven": "net.fabricmc:fabric-loader:0.15.11"},
            "intermediary": {"maven": "net.fabricmc:intermediary:1.20.1"},
            "launcherMeta": {
                "version": 2,
                "min_java_version": 8,
                "mainClass": {"client": "net.fabricmc.loader.impl.launch.knot.KnotClient"},
                "libraries": {
                    "client": [{"name": "net.fabricmc:sponge-mixin:0.15.0"}],
                    "common": [{
                        "name": "org.ow2.asm:asm:9.6",
                        "url": "https://maven.fabricmc.net/",
                        "sha1": "aa205cf0a06dbd8e04ece91c0b37c3f5d567546a",
                        "size": 123598
                    }],
                    "server": []
                }
            }
        })
        .to_string();

        serde_json::from_str(&text).expect("the metadata should be read")
    }

    /// Returns the addresses of the repositories of the libraries.
    fn repositories(patch: &InstancePatch) -> Vec<Option<&str>> {
        patch
            .libraries
            .iter()
            .map(|library| library.url.as_deref())
            .collect()
    }

    /// Returns the texts of the arguments of the game.
    fn game_arguments(patch: &InstancePatch) -> Vec<String> {
        patch
            .arguments
            .as_ref()
            .and_then(|arguments| arguments.game.as_ref())
            .map(|game| {
                game.iter()
                    .filter_map(|argument| argument.value().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn the_patch_names_the_loader_and_the_version() {
        let patch = loader_patch(ModLoader::Fabric, "0.15.11", &meta());
        assert_eq!(patch.id.as_deref(), Some("fabric"));
        assert_eq!(patch.version.as_deref(), Some("0.15.11"));
        assert_eq!(patch.priority, LOADER_PATCH_PRIORITY);
        assert_eq!(
            patch.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
    }

    #[test]
    fn the_libraries_of_both_sides_are_kept_but_not_the_ones_of_the_client() {
        let patch = loader_patch(ModLoader::Fabric, "0.15.11", &meta());
        let names: Vec<&str> = patch
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "org.ow2.asm:asm:9.6",
                "net.fabricmc:intermediary:1.20.1",
                "net.fabricmc:fabric-loader:0.15.11"
            ]
        );
    }

    #[test]
    fn the_intermediary_of_quilt_is_kept_on_the_repository_of_fabric() {
        let text = json!({
            "loader": {"maven": "org.quiltmc:quilt-loader:0.23.0"},
            "intermediary": {"maven": "net.fabricmc:intermediary:1.20.1"},
            "launcherMeta": {
                "mainClass": "org.quiltmc.loader.impl.launch.knot.KnotClient",
                "libraries": {"common": [], "server": []}
            }
        })
        .to_string();
        let meta: LoaderMeta = serde_json::from_str(&text).expect("the metadata should be read");

        let patch = loader_patch(ModLoader::Quilt, "0.23.0", &meta);
        assert_eq!(
            repositories(&patch),
            vec![Some(FABRIC_MAVEN_URL), Some(QUILT_MAVEN_URL)]
        );
        assert_eq!(
            patch.main_class.as_deref(),
            Some("org.quiltmc.loader.impl.launch.knot.KnotClient")
        );
    }

    #[test]
    fn the_intermediary_of_legacy_fabric_is_kept_on_its_own_repository() {
        let text = json!({
            "loader": {"maven": "net.fabricmc:fabric-loader:0.14.9"},
            "intermediary": {"maven": "net.legacyfabric:intermediary:1.8.9"},
            "launcherMeta": {
                "mainClass": {"client": "net.fabricmc.loader.impl.launch.knot.KnotClient"},
                "libraries": {"common": [], "server": []}
            }
        })
        .to_string();
        let meta: LoaderMeta = serde_json::from_str(&text).expect("the metadata should be read");

        let patch = loader_patch(ModLoader::LegacyFabric, "0.14.9", &meta);
        assert_eq!(
            repositories(&patch),
            vec![Some(LEGACY_FABRIC_MAVEN_URL), Some(FABRIC_MAVEN_URL)]
        );
    }

    #[test]
    fn the_tweak_class_of_a_wrapper_is_added_to_the_arguments() {
        let text = json!({
            "loader": {"maven": "net.fabricmc:fabric-loader:0.14.9"},
            "intermediary": {"maven": "net.legacyfabric:intermediary:1.8.9"},
            "launcherMeta": {
                "mainClass": {"client": "net.minecraft.launchwrapper.Launch"},
                "libraries": {"common": [], "server": []},
                "launchwrapper": {
                    "tweakers": {"client": ["optifine.OptiFineTweaker"]}
                }
            }
        })
        .to_string();
        let meta: LoaderMeta = serde_json::from_str(&text).expect("the metadata should be read");

        let patch = loader_patch(ModLoader::LegacyFabric, "0.14.9", &meta);
        assert_eq!(
            game_arguments(&patch),
            vec!["--tweakClass", "optifine.OptiFineTweaker"]
        );
    }

    #[test]
    fn a_loader_that_does_not_wrap_the_game_adds_no_arguments() {
        let patch = loader_patch(ModLoader::Fabric, "0.15.11", &meta());
        assert!(game_arguments(&patch).is_empty());
    }

    #[test]
    fn the_address_of_a_library_is_read_from_the_metadata() {
        let patch = loader_patch(ModLoader::Fabric, "0.15.11", &meta());
        let library = &patch.libraries[0];
        assert_eq!(library_path(library), "org/ow2/asm/asm/9.6/asm-9.6.jar");
        assert_eq!(
            library_url(library).as_deref(),
            Some("https://maven.fabricmc.net/org/ow2/asm/asm/9.6/asm-9.6.jar")
        );
    }

    #[test]
    fn the_digest_and_the_size_of_a_library_are_read_from_the_metadata() {
        let patch = loader_patch(ModLoader::Fabric, "0.15.11", &meta());
        let library = &patch.libraries[0];
        let download = library_download(library);
        assert_eq!(
            download.sha1.as_deref(),
            Some("aa205cf0a06dbd8e04ece91c0b37c3f5d567546a")
        );
        assert_eq!(download.size, Some(123598));
        assert_eq!(
            download.url,
            "https://maven.fabricmc.net/org/ow2/asm/asm/9.6/asm-9.6.jar"
        );
    }

    #[test]
    fn a_library_the_metadata_says_nothing_about_names_its_repository() {
        let patch = loader_patch(ModLoader::Fabric, "0.15.11", &meta());
        let library = &patch.libraries[2];
        assert_eq!(
            library_url(library).as_deref(),
            Some("https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar")
        );
        let download = library_download(library);
        assert_eq!(download.sha1, None);
        assert_eq!(download.size, None);
    }

    #[test]
    fn the_resolved_libraries_name_the_file_of_every_library() {
        let patch = loader_patch(ModLoader::Fabric, "0.15.11", &meta());
        let libraries = resolved_libraries(&patch.libraries);
        for (library, resolved) in patch.libraries.iter().zip(libraries.iter()) {
            let artifact = resolved
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.clone())
                .expect("the file of the library should be named");
            assert_eq!(
                artifact.path.as_deref(),
                Some(library_path(library).as_str())
            );
            assert_eq!(artifact.url, library_url(library).unwrap_or_default());
            assert_eq!(artifact.sha1, library_download(library).sha1);
            assert_eq!(artifact.size, library_download(library).size);
        }
    }

    #[test]
    fn the_loader_of_a_patch_is_found_by_the_identifier_of_the_patch() {
        assert_eq!(ModLoader::of_patch_id("fabric"), Some(ModLoader::Fabric));
        assert_eq!(ModLoader::of_patch_id("quilt"), Some(ModLoader::Quilt));
        assert_eq!(
            ModLoader::of_patch_id("legacyfabric"),
            Some(ModLoader::LegacyFabric)
        );
        assert_eq!(ModLoader::of_patch_id("forge"), None);
    }

    /// A server that answers with the file the tests name.
    struct Server {
        /// The bodies, by address.
        answers: std::collections::BTreeMap<String, Vec<u8>>,
    }

    impl Server {
        /// Creates a server that answers the given addresses.
        fn new(answers: &[(&str, &[u8])]) -> Self {
            Self {
                answers: answers
                    .iter()
                    .map(|(url, body)| ((*url).to_string(), body.to_vec()))
                    .collect(),
            }
        }
    }

    impl Fetcher for Server {
        fn fetch(
            &self,
            url: &str,
            _headers: &std::collections::BTreeMap<String, String>,
            sink: &mut dyn std::io::Write,
        ) -> Result<crate::download::task::ResponseInfo, String> {
            match self.answers.get(url) {
                Some(body) => {
                    sink.write_all(body).map_err(|error| error.to_string())?;
                    Ok(crate::download::task::ResponseInfo {
                        url: url.to_string(),
                        status: 200,
                        headers: std::collections::BTreeMap::new(),
                    })
                }
                None => Ok(crate::download::task::ResponseInfo {
                    url: url.to_string(),
                    status: 404,
                    headers: std::collections::BTreeMap::new(),
                }),
            }
        }
    }

    /// Returns a provider that asks the official servers, so the tests only
    /// have to answer the addresses the loaders themselves publish.
    fn provider() -> DownloadProvider {
        let mut provider = DownloadProvider::hmcl(false);
        provider.set_file_source(SourcePreference::Official);
        provider.set_version_list_source(SourcePreference::Official);
        provider
    }

    /// Returns a downloader that keeps its files in a directory of its own.
    fn downloader<'a>(
        server: &'a Server,
        name: &str,
    ) -> (TaskDownloader<'a>, DefaultCacheRepository, PathBuf) {
        let base =
            std::env::temp_dir().join(format!("hmcl-core-loader-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("the directory should be created");
        let cache = DefaultCacheRepository::new(&base);
        (
            TaskDownloader::new(server, cache.clone()),
            cache,
            base.join("libraries"),
        )
    }

    /// Writes a jar into a file and returns the bytes of the file.
    ///
    /// The files of the libraries are archives that the game opens, so a
    /// library the tests name must be an archive as well.
    fn jar(name: &str, text: &str) -> Vec<u8> {
        let path = std::env::temp_dir().join(format!("hmcl-core-loader-jar-{name}.zip"));
        let mut writer =
            crate::util::zip::ZipWriter::create(&path).expect("the archive should be written");
        writer
            .add_file("META-INF/MANIFEST.MF", text.as_bytes())
            .expect("the file should be written");
        writer.finish().expect("the archive should be written");
        std::fs::read(&path).expect("the archive should be read")
    }

    #[test]
    fn the_libraries_of_the_loader_are_downloaded() {
        let text = json!({
            "loader": {"maven": "net.fabricmc:fabric-loader:0.15.11"},
            "intermediary": {"maven": "net.fabricmc:intermediary:1.20.1"},
            "launcherMeta": {
                "mainClass": {"client": "net.fabricmc.loader.impl.launch.knot.KnotClient"},
                "libraries": {"common": [], "server": []}
            }
        })
        .to_string();

        let loader = jar("loader", "Manifest-Version: 1.0\n");
        let intermediary = jar("intermediary", "Manifest-Version: 1.0\n");

        let server = Server::new(&[
            (
                "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.15.11",
                text.as_bytes(),
            ),
            (
                "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar",
                &loader,
            ),
            (
                "https://maven.fabricmc.net/net/fabricmc/intermediary/1.20.1/intermediary-1.20.1.jar",
                &intermediary,
            ),
        ]);
        let (downloader, _cache, libraries) = downloader(&server, "install");

        let installed = install_loader(
            ModLoader::Fabric,
            "1.20.1",
            "0.15.11",
            &provider(),
            &server,
            &downloader,
            &libraries,
        )
        .expect("the loader should be installed");

        assert_eq!(
            installed.patch.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        assert_eq!(installed.files.len(), 2);
        // The intermediary comes before the loader itself, because the service
        // names the loader outside of the list of the libraries.
        assert_eq!(
            std::fs::read(&installed.files[0]).expect("the file should be read"),
            intermediary
        );
        assert_eq!(
            std::fs::read(&installed.files[1]).expect("the file should be read"),
            loader
        );
        assert_eq!(
            installed.files[1],
            libraries.join("net/fabricmc/fabric-loader/0.15.11/fabric-loader-0.15.11.jar")
        );
    }

    #[test]
    fn the_library_of_the_cache_is_not_downloaded_again() {
        let text = json!({
            "loader": {"maven": "net.fabricmc:fabric-loader:0.15.11"},
            "intermediary": {"maven": "net.fabricmc:intermediary:1.20.1"},
            "launcherMeta": {
                "mainClass": {"client": "net.fabricmc.loader.impl.launch.knot.KnotClient"},
                "libraries": {"common": [], "server": []}
            }
        })
        .to_string();

        let server = Server::new(&[(
            "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.15.11",
            text.as_bytes(),
        )]);
        let (downloader, mut cache, libraries) = downloader(&server, "cached");

        // Both libraries the service names outside of the list of the
        // libraries are kept in the cache, so neither of them is asked for
        // again; the loader itself names no digest, so the file of the cache is
        // kept as it is.
        for library in [
            Library {
                name: "net.fabricmc:intermediary:1.20.1".to_string(),
                url: Some(FABRIC_MAVEN_URL.to_string()),
                ..Library::default()
            },
            Library {
                name: "net.fabricmc:fabric-loader:0.15.11".to_string(),
                url: Some(FABRIC_MAVEN_URL.to_string()),
                ..Library::default()
            },
        ] {
            let file = libraries.join(crate::download::library_path_of(&library));
            std::fs::create_dir_all(file.parent().expect("the file should have a parent"))
                .expect("the directory should be created");
            std::fs::write(&file, jar("cached", "Manifest-Version: 1.0\n"))
                .expect("the file should be written");
            cache
                .try_cache_library(&library, &file)
                .expect("the library should be checked");
        }

        let installed = install_loader(
            ModLoader::Fabric,
            "1.20.1",
            "0.15.11",
            &provider(),
            &server,
            &downloader,
            &libraries,
        )
        .expect("the loader should be installed");

        assert_eq!(installed.files.len(), 2);
        assert!(installed.files[0].is_file());
        assert!(installed.files[1].is_file());
    }

    #[test]
    fn the_metadata_of_a_broken_answer_is_reported() {
        let server = Server::new(&[(
            "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.15.11",
            b"not json",
        )]);
        let (downloader, _cache, libraries) = downloader(&server, "broken");

        let error = install_loader(
            ModLoader::Fabric,
            "1.20.1",
            "0.15.11",
            &provider(),
            &server,
            &downloader,
            &libraries,
        )
        .expect_err("the broken metadata should be refused");

        assert!(matches!(error, DownloadError::Failed(_)), "{error:?}");
    }

    #[test]
    fn the_metadata_of_a_version_that_is_not_there_is_reported() {
        let server = Server::new(&[]);
        let (downloader, _cache, libraries) = downloader(&server, "missing");

        let error = install_loader(
            ModLoader::Quilt,
            "1.20.1",
            "0.23.0",
            &provider(),
            &server,
            &downloader,
            &libraries,
        )
        .expect_err("the version that is not there should be refused");

        assert!(matches!(error, DownloadError::NotFound { .. }), "{error:?}");
    }
}
