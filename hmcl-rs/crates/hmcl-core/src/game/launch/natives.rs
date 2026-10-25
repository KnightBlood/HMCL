//! The native libraries of a version of the game.
//!
//! The game reads the native libraries, such as the library that draws the
//! window or the one that reads the sounds, from a directory of files; the
//! files are kept in the archives of the libraries, which the launcher
//! extracts before it starts the game.

use std::path::{Path, PathBuf};

use crate::download::game::{rules_allow, InstanceManifest, RuleContext};
use crate::util::zip::{self, ZipEntry, ZipError};

use super::classpath::is_native;
use super::libraries::unique_libraries;
use super::library::library_file;
use super::options::LaunchPaths;

/// The options of extracting the native libraries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NativeOptions {
    /// Whether the game uses the GLFW and the SDL of the machine.
    ///
    /// The versions of the game may bring their own GLFW and SDL; the player
    /// may tell the launcher to use the libraries of the machine instead.
    pub use_system_glfw_sdl: bool,
    /// Whether the game uses the OpenAL of the machine.
    pub use_system_openal: bool,
}

/// The archive of the native libraries of a library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeArchive {
    /// The place of the archive.
    pub path: PathBuf,
    /// The address the archive is downloaded from.
    pub url: Option<String>,
    /// The digest of the archive, in the hexadecimal of SHA-1.
    pub sha1: Option<String>,
    /// The size of the archive, in bytes.
    pub size: Option<u64>,
    /// The classifier of the archive.
    pub classifier: Option<String>,
    /// The beginnings of the paths of the files that are not extracted.
    pub exclude: Vec<String>,
}

impl NativeArchive {
    /// Returns whether the file of the archive is extracted.
    ///
    /// The versions tell which files of the archive the game does not read,
    /// such as the metadata of the archive.
    pub fn should_extract(&self, name: &str) -> bool {
        self.exclude
            .iter()
            .all(|excluded| !name.starts_with(excluded))
    }
}

/// Returns the archives of the native libraries of a version.
///
/// The archives of the libraries that are about other systems are skipped, as
/// are the libraries that are kept on the machine of the player.
pub fn native_archives(
    manifest: &InstanceManifest,
    paths: &LaunchPaths,
    context: &RuleContext<'_>,
) -> Vec<NativeArchive> {
    unique_libraries(&manifest.libraries)
        .iter()
        .filter(|library| rules_allow(library.rules.as_deref(), context) && is_native(library))
        .filter_map(|library| {
            let file = library_file(library, context)?;
            Some(NativeArchive {
                path: paths.libraries_directory.join(file.path),
                url: file.url,
                sha1: file.sha1,
                size: file.size,
                classifier: file.classifier,
                exclude: library
                    .extract
                    .as_ref()
                    .map(|extract| extract.exclude.clone())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Extracts the files of the archives of the native libraries into a
/// directory.
///
/// The files of the directory that the archives do not hold are kept, so the
/// launcher removes the directory before it extracts the archives of a
/// different version of the game.
pub fn extract_natives(
    archives: &[NativeArchive],
    destination: &Path,
) -> Result<Vec<PathBuf>, ZipError> {
    extract_natives_with(archives, destination, NativeOptions::default())
}

/// Extracts the files of the archives of the native libraries into a
/// directory, reading the options of the player.
///
/// The files the game reads from the machine, the metadata of the archives and
/// the links of the archives are not extracted; a file that is already there
/// with the same size is kept.
pub fn extract_natives_with(
    archives: &[NativeArchive],
    destination: &Path,
    options: NativeOptions,
) -> Result<Vec<PathBuf>, ZipError> {
    std::fs::create_dir_all(destination).map_err(ZipError::Io)?;

    let mut extracted = Vec::new();
    for archive in archives {
        extracted.extend(zip::extract(
            &archive.path,
            destination,
            |entry, target| {
                archive.should_extract(&entry.name) && should_extract_file(entry, target, options)
            },
        )?);
    }

    Ok(extracted)
}

/// Returns whether a file of an archive is extracted into a directory.
///
/// The launcher does not extract the metadata of the archive, the links of the
/// archive or the libraries the game reads from the machine; a file that is
/// already there with the same size is kept, so the launcher may extract the
/// archives of a version again without writing the same files.
pub fn should_extract_file(entry: &ZipEntry, target: &Path, options: NativeOptions) -> bool {
    if entry.is_symlink() {
        return false;
    }

    if let Some(extension) = target.extension().and_then(|extension| extension.to_str()) {
        if extension.eq_ignore_ascii_case("sha1") || extension.eq_ignore_ascii_case("git") {
            return false;
        }
    }

    let name = entry
        .name
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if options.use_system_glfw_sdl && (name.contains("glfw") || name.contains("sdl")) {
        return false;
    }
    if options.use_system_openal && name.contains("openal") {
        return false;
    }

    !std::fs::metadata(target)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == entry.size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{DownloadInfo, ExtractRules, Library, LibraryDownloads};
    use std::collections::BTreeMap;

    fn lwjgl() -> Library {
        Library {
            name: "org.lwjgl:lwjgl:3.3.1".to_string(),
            natives: Some(BTreeMap::from([(
                "windows".to_string(),
                "natives-windows".to_string(),
            )])),
            extract: Some(ExtractRules {
                exclude: vec!["META-INF/".to_string()],
            }),
            downloads: Some(LibraryDownloads {
                artifact: None,
                classifiers: Some(BTreeMap::from([(
                    "natives-windows".to_string(),
                    DownloadInfo {
                        url: "https://example.com/lwjgl-natives.jar".to_string(),
                        path: Some(
                            "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar".to_string(),
                        ),
                        sha1: Some("abc".to_string()),
                        size: Some(42),
                    },
                )])),
            }),
            ..Library::default()
        }
    }

    fn paths() -> LaunchPaths {
        LaunchPaths {
            game_directory: PathBuf::from("game"),
            assets_directory: PathBuf::from("assets"),
            asset_index: "1.20".to_string(),
            natives_directory: PathBuf::from("natives"),
            libraries_directory: PathBuf::from("libraries"),
            client_jar: PathBuf::from("client.jar"),
        }
    }

    #[test]
    fn the_archives_of_the_native_libraries_are_listed() {
        let manifest = InstanceManifest {
            libraries: vec![lwjgl()],
            ..InstanceManifest::default()
        };
        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0", "x86_64", &features);
        let archives = native_archives(&manifest, &paths(), &context);

        assert_eq!(archives.len(), 1);
        assert_eq!(
            archives[0].path,
            PathBuf::from("libraries/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar")
        );
        assert_eq!(archives[0].classifier.as_deref(), Some("natives-windows"));
        assert_eq!(archives[0].exclude, vec!["META-INF/".to_string()]);
    }

    #[test]
    fn the_files_of_the_library_itself_are_not_extracted() {
        let mut manifest = InstanceManifest {
            libraries: vec![lwjgl()],
            ..InstanceManifest::default()
        };
        manifest.libraries[0].natives = None;
        manifest.libraries[0].downloads = Some(LibraryDownloads {
            artifact: Some(DownloadInfo {
                url: "https://example.com/lwjgl.jar".to_string(),
                ..DownloadInfo::default()
            }),
            classifiers: None,
        });

        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0", "x86_64", &features);

        assert!(native_archives(&manifest, &paths(), &context).is_empty());
    }

    #[test]
    fn the_files_the_version_excludes_are_not_extracted() {
        let archives = native_archives(
            &InstanceManifest {
                libraries: vec![lwjgl()],
                ..InstanceManifest::default()
            },
            &paths(),
            &RuleContext::new("windows", "10.0", "x86_64", &BTreeMap::new()),
        );

        assert!(!archives[0].should_extract("META-INF/MANIFEST.MF"));
        assert!(archives[0].should_extract("lwjgl.dll"));
    }

    #[test]
    fn the_archive_of_the_repository_is_extracted() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../minecraft/libraries/HMCLTransformerDiscoveryService/lib/modlauncher-4.1.0.jar",
        );
        if !path.exists() {
            return;
        }

        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-natives-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let archive = NativeArchive {
            path,
            url: None,
            sha1: None,
            size: None,
            classifier: None,
            exclude: vec!["META-INF/".to_string()],
        };
        let extracted = extract_natives(&[archive], &directory)
            .expect("the archive of the repository should be extracted");

        assert!(!extracted.is_empty());
        assert!(extracted
            .iter()
            .all(|file| !file.to_string_lossy().contains("META-INF")));
        assert!(extracted.iter().all(|file| file.exists()));

        let _ = std::fs::remove_dir_all(&directory);
    }

    fn entry(name: &str, size: u64, unix_mode: u32) -> ZipEntry {
        ZipEntry {
            name: name.to_string(),
            directory: false,
            size,
            unix_mode,
        }
    }

    #[test]
    fn the_metadata_and_the_links_of_an_archive_are_not_extracted() {
        let options = NativeOptions::default();

        assert!(!should_extract_file(
            &entry("library.dll.sha1", 3, 0),
            Path::new("natives/library.dll.sha1"),
            options
        ));
        assert!(!should_extract_file(
            &entry("link", 3, 0o120777),
            Path::new("natives/link"),
            options
        ));
        assert!(should_extract_file(
            &entry("library.dll", 3, 0o100644),
            Path::new("natives/library.dll"),
            options
        ));
    }

    #[test]
    fn the_libraries_of_the_machine_are_not_extracted() {
        let options = NativeOptions {
            use_system_glfw_sdl: true,
            use_system_openal: true,
        };

        assert!(!should_extract_file(
            &entry("glfw.dll", 3, 0),
            Path::new("natives/glfw.dll"),
            options
        ));
        assert!(!should_extract_file(
            &entry("libSDL2.so", 3, 0),
            Path::new("natives/libSDL2.so"),
            options
        ));
        assert!(!should_extract_file(
            &entry("OpenAL.dll", 3, 0),
            Path::new("natives/OpenAL.dll"),
            options
        ));

        assert!(should_extract_file(
            &entry("glfw.dll", 3, 0),
            Path::new("natives/glfw.dll"),
            NativeOptions::default()
        ));
    }

    #[test]
    fn the_file_that_is_already_there_is_not_extracted() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-natives-size-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");

        let target = directory.join("library.dll");
        std::fs::write(&target, b"abc").expect("the file should be written");

        assert!(!should_extract_file(
            &entry("library.dll", 3, 0),
            &target,
            NativeOptions::default()
        ));
        assert!(should_extract_file(
            &entry("library.dll", 4, 0),
            &target,
            NativeOptions::default()
        ));

        let _ = std::fs::remove_dir_all(&directory);
    }
}
