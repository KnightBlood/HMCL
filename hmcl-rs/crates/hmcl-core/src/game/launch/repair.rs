//! Fixing the libraries of a version before the game is started.
//!
//! The libraries of a version are checked, and the ones the launcher has to
//! correct are corrected: the archive of OptiFine that a version of Forge
//! refuses loses the file that stops the game, and the archives the launcher
//! ships are copied instead of downloaded. The libraries that are not the files
//! the version tells are told to the launcher, so it downloads them.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::download::game::{rules_allow, InstanceManifest, Library, RuleContext};
use crate::game::component::{ComponentAnalyzer, GameComponentType};
use crate::game::verification::{
    bundled_library_resource, fix_optifine_of_forge, library_needs_download,
};
use crate::util::zip::ZipError;

use super::libraries::unique_libraries;
use super::library::library_file;
use super::options::LaunchPaths;
use super::required::{RequiredFile, RequiredFileKind};

/// The error of repairing the libraries of a version.
#[derive(Debug)]
pub enum RepairError {
    /// A file that is copied or read cannot be written.
    Io {
        /// The file that cannot be written.
        path: PathBuf,
        /// The reason the file cannot be written.
        error: io::Error,
    },
    /// The archive of a library cannot be changed.
    Zip {
        /// The archive that cannot be changed.
        path: PathBuf,
        /// The reason the archive cannot be changed.
        error: ZipError,
    },
    /// The launcher does not ship the archive of a library.
    MissingResource {
        /// The name of the archive the launcher ships.
        name: &'static str,
    },
}

impl fmt::Display for RepairError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => {
                write!(
                    formatter,
                    "The file {} cannot be written: {error}",
                    path.display()
                )
            }
            Self::Zip { path, error } => write!(
                formatter,
                "The archive {} cannot be changed: {error}",
                path.display()
            ),
            Self::MissingResource { name } => {
                write!(formatter, "The launcher does not ship the archive {name}")
            }
        }
    }
}

impl std::error::Error for RepairError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            Self::Zip { error, .. } => Some(error),
            Self::MissingResource { .. } => None,
        }
    }
}

/// What the launcher changed in the libraries of a version.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryRepair {
    /// The libraries the launcher has to download.
    pub downloads: Vec<RequiredFile>,
    /// The archives the launcher changed.
    pub fixed: Vec<PathBuf>,
    /// The archives the launcher copied from its own files.
    pub installed: Vec<PathBuf>,
}

impl LibraryRepair {
    /// Returns whether the launcher may start the game without downloading.
    pub fn is_ready(&self) -> bool {
        self.downloads.is_empty()
    }
}

/// Checks the libraries of a version and corrects the ones the launcher knows
/// are broken.
///
/// The archives of OptiFine that a version of Forge refuses lose the file that
/// stops the game, and the archives the launcher ships are copied into the
/// directory of the libraries. The libraries that are not the files the version
/// tells are returned, so the launcher downloads them.
///
/// `read_resource` reads an archive the launcher ships by the name
/// [`bundled_library_resource`] returns.
pub fn repair_libraries<F>(
    manifest: &InstanceManifest,
    paths: &LaunchPaths,
    context: &RuleContext<'_>,
    integrity_check: bool,
    components: Option<&ComponentAnalyzer>,
    mut read_resource: F,
) -> Result<LibraryRepair, RepairError>
where
    F: FnMut(&str) -> Option<Vec<u8>>,
{
    let optifine_version = optifine_version_of_1_20_4(manifest);
    let forge_version = components
        .and_then(|components| components.version(GameComponentType::Forge))
        .map(str::to_string);

    let mut repair = LibraryRepair::default();

    for library in unique_libraries(&manifest.libraries) {
        let library = &library;
        if !rules_allow(library.rules.as_deref(), context) {
            continue;
        }
        let Some(file) = library_file(library, context) else {
            continue;
        };
        let path = paths.libraries_directory.join(&file.path);
        let group = library.artifact().map(|artifact| artifact.group).ok();

        if let (Some(version), Some(forge_version)) = (&optifine_version, &forge_version) {
            if group.as_deref() == Some("optifine") {
                let changed = fix_optifine_of_forge(&path, version, Some(forge_version)).map_err(
                    |error| RepairError::Zip {
                        path: path.clone(),
                        error,
                    },
                )?;
                if changed {
                    repair.fixed.push(path.clone());
                }
            }
        }

        if let Some(resource) = bundled_library_resource(library) {
            if install_bundled(&path, resource, &mut read_resource)? {
                repair.installed.push(path.clone());
            }
        }

        if !library_needs_download(&path, library, integrity_check) {
            continue;
        }

        // A library of OptiFine that names no address is not downloaded: the
        // launcher of OptiFine keeps the file in the directory of the
        // libraries, and the version names neither a repository nor a file.
        if file.url.is_none() && group.as_deref() == Some("optifine") {
            continue;
        }

        repair.downloads.push(RequiredFile {
            path,
            url: file.url,
            sha1: file.sha1,
            size: file.size,
            kind: RequiredFileKind::Library,
        });
    }

    Ok(repair)
}

/// Returns the version of OptiFine the version of the game needs the fix for.
///
/// The version of OptiFine has to be the one of Minecraft 1.20.4; the versions
/// of the game before and after it do not have the file that stops the game.
fn optifine_version_of_1_20_4(manifest: &InstanceManifest) -> Option<String> {
    manifest
        .libraries
        .iter()
        .filter_map(|library: &Library| library.artifact().ok())
        .find(|artifact| {
            artifact.group == "optifine"
                && artifact.name == "OptiFine"
                && artifact.version.starts_with("1.20.4_")
        })
        .map(|artifact| artifact.version)
}

/// Copies an archive the launcher ships into the directory of the libraries.
///
/// Returns whether the archive was copied. The archive of the bootstrap is
/// copied only when it is not there, because the launcher of the game may keep
/// its own; the archive of the transformer is copied every time, because it is
/// a part of the launcher.
fn install_bundled<F>(
    path: &Path,
    resource: &'static str,
    read_resource: &mut F,
) -> Result<bool, RepairError>
where
    F: FnMut(&str) -> Option<Vec<u8>>,
{
    let keep = resource.ends_with("HMCLMultiMCBootstrap-1.0.jar") && path.is_file();
    if keep {
        return Ok(false);
    }

    let Some(contents) = read_resource(resource) else {
        return Err(RepairError::MissingResource { name: resource });
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| RepairError::Io {
            path: parent.to_path_buf(),
            error,
        })?;
    }
    std::fs::write(path, contents).map_err(|error| RepairError::Io {
        path: path.to_path_buf(),
        error,
    })?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{DownloadInfo, LibraryDownloads};
    use crate::util::digest::Algorithm;
    use crate::util::zip::ZipWriter;
    use std::collections::BTreeMap;

    /// Returns a directory the test works in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-repair-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    /// Returns the paths of a version in the directory.
    fn paths(directory: &Path) -> LaunchPaths {
        LaunchPaths {
            game_directory: directory.join("game"),
            assets_directory: directory.join("assets"),
            asset_index: "1.20".to_string(),
            natives_directory: directory.join("natives"),
            libraries_directory: directory.join("libraries"),
            client_jar: directory.join("client.jar"),
        }
    }

    /// Returns the rules the libraries are read with.
    fn context<'a>(features: &'a BTreeMap<String, bool>) -> crate::download::game::RuleContext<'a> {
        crate::download::game::RuleContext::new("windows", "10.0", "x86_64", features)
    }

    /// Returns a library that names its file.
    fn library(name: &str, path: &str) -> Library {
        Library {
            name: name.to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(DownloadInfo {
                    url: format!("https://example.com/{path}"),
                    path: Some(path.to_string()),
                    sha1: None,
                    size: None,
                }),
                classifiers: None,
            }),
            ..Library::default()
        }
    }

    /// Writes an archive of OptiFine with the file the versions of Forge
    /// refuse.
    fn optifine_jar(path: &Path) {
        std::fs::create_dir_all(path.parent().expect("the file should have a parent"))
            .expect("the directory should be made");
        let mut writer = ZipWriter::create(path).expect("the archive should be created");
        writer
            .add_file("META-INF/mods.toml", b"modLoader = \"javafml\"")
            .expect("the file should be added");
        writer
            .add_file("net/optifine/Config.class", b"classes")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");
    }

    #[test]
    fn the_mods_file_of_optifine_is_removed() {
        let directory = directory("optifine");
        let paths = paths(&directory);
        let library = library(
            "optifine:OptiFine:1.20.4_HD_U_I6",
            "optifine/OptiFine/1.20.4/OptiFine.jar",
        );
        let jar = paths
            .libraries_directory
            .join("optifine/OptiFine/1.20.4/OptiFine.jar");
        optifine_jar(&jar);

        let manifest = InstanceManifest {
            libraries: vec![library],
            patches: vec![crate::download::game::InstancePatch {
                id: Some("forge".to_string()),
                version: Some("48.1.0".to_string()),
                ..crate::download::game::InstancePatch::default()
            }],
            ..InstanceManifest::default()
        };
        let components = ComponentAnalyzer::analyze(&manifest, Some("1.20.4"))
            .expect("the components should be read");

        let features = BTreeMap::new();
        let context = context(&features);
        let repair = repair_libraries(&manifest, &paths, &context, true, Some(&components), |_| {
            None
        })
        .expect("the libraries should be repaired");
        assert_eq!(repair.fixed, vec![jar.clone()]);

        let names = crate::util::zip::entries(&jar)
            .expect("the archive should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["net/optifine/Config.class"]);

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn an_archive_of_the_launcher_is_copied() {
        let directory = directory("bundled");
        let paths = paths(&directory);
        let manifest = InstanceManifest {
            libraries: vec![library(
                "org.jackhuang.hmcl:mmc-bootstrap:1.0",
                "org/jackhuang/hmcl/mmc-bootstrap/1.0/mmc-bootstrap-1.0.jar",
            )],
            ..InstanceManifest::default()
        };

        let features = BTreeMap::new();
        let context = context(&features);
        let repair = repair_libraries(&manifest, &paths, &context, false, None, |name| {
            (name == "/assets/game/HMCLMultiMCBootstrap-1.0.jar").then(|| b"bootstrap".to_vec())
        })
        .expect("the libraries should be repaired");

        let jar = paths
            .libraries_directory
            .join("org/jackhuang/hmcl/mmc-bootstrap/1.0/mmc-bootstrap-1.0.jar");
        assert_eq!(repair.installed, vec![jar.clone()]);
        assert_eq!(
            std::fs::read(&jar).expect("the file should be read"),
            b"bootstrap"
        );
        // The version names an address, so the archive of the launcher that is
        // there is not downloaded again.
        assert!(repair.downloads.is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_library_the_launcher_does_not_ship_is_a_download() {
        let directory = directory("download");
        let paths = paths(&directory);
        let manifest = InstanceManifest {
            libraries: vec![library(
                "com.example:library:1.0",
                "com/example/library/1.0/library-1.0.jar",
            )],
            ..InstanceManifest::default()
        };

        let features = BTreeMap::new();
        let context = context(&features);
        let repair = repair_libraries(&manifest, &paths, &context, false, None, |_| None)
            .expect("the libraries should be repaired");
        assert_eq!(repair.downloads.len(), 1);
        assert_eq!(repair.downloads[0].kind, RequiredFileKind::Library);
        assert!(!repair.is_ready());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn an_archive_the_launcher_does_not_ship_is_an_error() {
        let directory = directory("missing-resource");
        let paths = paths(&directory);
        let manifest = InstanceManifest {
            libraries: vec![library(
                "org.jackhuang.hmcl:transformer-discovery-service:1.0",
                "org/jackhuang/hmcl/transformer-discovery-service/1.0/transformer-discovery-service-1.0.jar",
            )],
            ..InstanceManifest::default()
        };

        let features = BTreeMap::new();
        let context = context(&features);
        let error = repair_libraries(&manifest, &paths, &context, false, None, |_| None)
            .expect_err("the repair should fail");
        assert!(matches!(error, RepairError::MissingResource { .. }));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_library_of_another_digest_is_a_download() {
        let directory = directory("digest");
        let paths = paths(&directory);
        let path = paths
            .libraries_directory
            .join("com/example/library/1.0/library-1.0.jar");
        std::fs::create_dir_all(path.parent().expect("the file should have a parent"))
            .expect("the directory should be made");
        std::fs::write(&path, b"the file the player wrote").expect("the file should be written");

        let mut library = library(
            "com.example:library:1.0",
            "com/example/library/1.0/library-1.0.jar",
        );
        library
            .downloads
            .as_mut()
            .and_then(|downloads| downloads.artifact.as_mut())
            .expect("the library should name its file")
            .sha1 = Some(Algorithm::Sha1.digest_to_string(b"the file of the version"));

        let manifest = InstanceManifest {
            libraries: vec![library],
            ..InstanceManifest::default()
        };

        let features = BTreeMap::new();
        let context = context(&features);
        // The library is there, so a start that does not check the files of the
        // version keeps it.
        let repair = repair_libraries(&manifest, &paths, &context, false, None, |_| None)
            .expect("the libraries should be repaired");
        assert!(repair.is_ready());

        let repair = repair_libraries(&manifest, &paths, &context, true, None, |_| None)
            .expect("the libraries should be repaired");
        assert_eq!(repair.downloads.len(), 1);
        assert_eq!(repair.downloads[0].path, path);

        let _ = std::fs::remove_dir_all(&directory);
    }
}
