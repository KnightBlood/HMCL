//! The files a version of the game needs before it is started.

use std::path::PathBuf;

use crate::download::game::{rules_allow, AssetIndexFile, InstanceManifest, RuleContext};
use crate::game::verification::library_needs_download;
use crate::util::digest::Algorithm;

use super::libraries::unique_libraries;
use super::library::library_file;
use super::options::LaunchPaths;

/// The kind of a file a version of the game needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequiredFileKind {
    /// The file of the game itself.
    Client,
    /// A library, or the file of the native libraries of a library.
    Library,
    /// The index of the assets.
    AssetIndex,
    /// A file of the assets.
    Asset,
}

/// A file a version of the game needs before it is started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredFile {
    /// The place of the file.
    pub path: PathBuf,
    /// The address the file is downloaded from.
    ///
    /// The files that are kept on the machine of the player, such as the
    /// libraries the player installed, are not downloaded.
    pub url: Option<String>,
    /// The digest of the file, in the hexadecimal of SHA-1.
    pub sha1: Option<String>,
    /// The size of the file, in bytes.
    pub size: Option<u64>,
    /// The kind of the file.
    pub kind: RequiredFileKind,
}

/// Returns the file of the game itself.
pub fn client_file(manifest: &InstanceManifest, paths: &LaunchPaths) -> Option<RequiredFile> {
    let download = manifest.downloads.as_ref()?.get("client")?;

    Some(RequiredFile {
        path: paths.client_jar.clone(),
        url: Some(download.url.clone()).filter(|url| !url.is_empty()),
        sha1: download.sha1().map(str::to_string),
        size: download.size,
        kind: RequiredFileKind::Client,
    })
}

/// Returns the file of the index of the assets.
pub fn asset_index_file(manifest: &InstanceManifest, paths: &LaunchPaths) -> Option<RequiredFile> {
    let index = manifest.asset_index.as_ref()?;
    let id = index.id.as_deref()?;

    Some(RequiredFile {
        path: paths
            .assets_directory
            .join("indexes")
            .join(format!("{id}.json")),
        url: index.url.clone().filter(|url| !url.is_empty()),
        sha1: index.sha1.clone().filter(|sha1| !sha1.is_empty()),
        size: index.size,
        kind: RequiredFileKind::AssetIndex,
    })
}

/// Returns the files of the assets of a version.
///
/// The index of the assets tells the files the game reads; the launcher reads
/// the files from the directory of the assets, named after their digests.
pub fn asset_files(index: &AssetIndexFile, paths: &LaunchPaths) -> Vec<RequiredFile> {
    index
        .objects
        .values()
        .map(|object| RequiredFile {
            path: paths
                .assets_directory
                .join("objects")
                .join(object.location()),
            url: Some(object.url()),
            sha1: Some(object.hash.clone()),
            size: Some(object.size),
            kind: RequiredFileKind::Asset,
        })
        .collect()
}

/// Returns the files of the libraries of a version that are used on the system
/// of the player.
pub fn library_files(
    manifest: &InstanceManifest,
    paths: &LaunchPaths,
    context: &RuleContext<'_>,
) -> Vec<RequiredFile> {
    unique_libraries(&manifest.libraries)
        .iter()
        .filter(|library| rules_allow(library.rules.as_deref(), context))
        .filter_map(|library| library_file(library, context))
        .map(|file| RequiredFile {
            path: paths.libraries_directory.join(file.path),
            url: file.url,
            sha1: file.sha1,
            size: file.size,
            kind: RequiredFileKind::Library,
        })
        .collect()
}

/// Returns the files a version of the game needs before it is started.
///
/// The index of the assets is read separately, because the launcher downloads
/// it before it knows the files of the assets; a version the launcher does not
/// know the assets of holds no files of the assets.
pub fn required_files(
    manifest: &InstanceManifest,
    paths: &LaunchPaths,
    context: &RuleContext<'_>,
    index: Option<&AssetIndexFile>,
) -> Vec<RequiredFile> {
    let mut files = Vec::new();

    if let Some(client) = client_file(manifest, paths) {
        files.push(client);
    }
    if let Some(index) = asset_index_file(manifest, paths) {
        files.push(index);
    }

    files.extend(library_files(manifest, paths, context));

    if let Some(index) = index {
        files.extend(asset_files(index, paths));
    }

    // The index of the assets may name the same file twice, under the names the
    // game reads the file by; the file is downloaded once.
    let mut seen = std::collections::BTreeSet::new();
    files.retain(|file| seen.insert(file.path.clone()));

    files
}

/// Returns the files that are not in the directories they belong in.
///
/// A file is missing when it is not there, or when it is not the size the
/// version tells. The digests are not read, because the launcher reads the
/// files of the game every time it starts the game.
pub fn missing_files(files: &[RequiredFile]) -> Vec<RequiredFile> {
    files
        .iter()
        .filter(|file| !is_present(file))
        .cloned()
        .collect()
}

/// Returns the libraries of a version the launcher has to download again.
///
/// A library is downloaded again when it is not there, or when it is not the
/// file the version tells: the digest, the checksums of the files of the
/// library and the archive itself are checked. The libraries of the player are
/// checked as well, because a library that the game cannot read stops the game.
///
/// The files are checked only when `integrity_check` is on; a library that is
/// not there is downloaded whatever the setting is.
pub fn libraries_to_download(
    manifest: &InstanceManifest,
    paths: &LaunchPaths,
    context: &RuleContext<'_>,
    integrity_check: bool,
) -> Vec<RequiredFile> {
    unique_libraries(&manifest.libraries)
        .iter()
        .filter(|library| rules_allow(library.rules.as_deref(), context))
        .filter_map(|library| {
            let file = library_file(library, context)?;
            let path = paths.libraries_directory.join(&file.path);

            if !library_needs_download(&path, library, integrity_check) {
                return None;
            }

            Some(RequiredFile {
                path,
                url: file.url,
                sha1: file.sha1,
                size: file.size,
                kind: RequiredFileKind::Library,
            })
        })
        .collect()
}

/// Returns whether the file is there and is the size the version tells.
pub fn is_present(file: &RequiredFile) -> bool {
    let Ok(metadata) = std::fs::metadata(&file.path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    match file.size {
        Some(size) => metadata.len() == size,
        None => true,
    }
}

/// Returns whether the file is there and is the file the version tells.
///
/// The digest is read, so the launcher may check a file another launcher
/// downloaded or a file the player changed.
pub fn verify_file(file: &RequiredFile) -> bool {
    if !is_present(file) {
        return false;
    }

    match &file.sha1 {
        Some(expected) => Algorithm::Sha1
            .digest_file_to_string(&file.path)
            .map(|actual| actual.eq_ignore_ascii_case(expected))
            .unwrap_or(false),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{AssetObject, DownloadInfo, Library, LibraryDownloads};
    use std::collections::BTreeMap;

    fn paths(directory: &std::path::Path) -> LaunchPaths {
        LaunchPaths {
            game_directory: directory.join("game"),
            assets_directory: directory.join("assets"),
            asset_index: "1.20".to_string(),
            natives_directory: directory.join("natives"),
            libraries_directory: directory.join("libraries"),
            client_jar: directory.join("versions/1.20.1/1.20.1.jar"),
        }
    }

    fn manifest() -> InstanceManifest {
        InstanceManifest {
            downloads: Some(BTreeMap::from([(
                "client".to_string(),
                DownloadInfo {
                    url: "https://example.com/client.jar".to_string(),
                    path: None,
                    sha1: Some("111".to_string()),
                    size: Some(100),
                },
            )])),
            asset_index: Some(crate::download::game::AssetIndex {
                id: Some("1.20".to_string()),
                sha1: Some("222".to_string()),
                size: Some(200),
                total_size: None,
                url: Some("https://example.com/1.20.json".to_string()),
            }),
            libraries: vec![Library {
                name: "com.example:library:1.0".to_string(),
                downloads: Some(LibraryDownloads {
                    artifact: Some(DownloadInfo {
                        url: "https://example.com/library.jar".to_string(),
                        path: Some("com/example/library/1.0/library-1.0.jar".to_string()),
                        sha1: Some("333".to_string()),
                        size: Some(300),
                    }),
                    classifiers: None,
                }),
                ..Library::default()
            }],
            ..InstanceManifest::default()
        }
    }

    fn index() -> AssetIndexFile {
        AssetIndexFile {
            virtual_assets: false,
            map_to_resources: false,
            objects: BTreeMap::from([(
                "minecraft/sounds/click.ogg".to_string(),
                AssetObject {
                    hash: "abcdef0123456789".to_string(),
                    size: 42,
                },
            )]),
        }
    }

    #[test]
    fn the_files_of_a_version_are_listed() {
        let directory = std::path::Path::new("game");
        let paths = paths(directory);
        let index = index();
        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0", "x86_64", &features);
        let files = required_files(&manifest(), &paths, &context, Some(&index));

        assert_eq!(files.len(), 4);
        assert_eq!(files[0].kind, RequiredFileKind::Client);
        assert_eq!(files[0].path, paths.client_jar);
        assert_eq!(files[1].kind, RequiredFileKind::AssetIndex);
        assert_eq!(
            files[1].path,
            paths.assets_directory.join("indexes/1.20.json")
        );
        assert_eq!(files[2].kind, RequiredFileKind::Library);
        assert_eq!(
            files[2].path,
            paths
                .libraries_directory
                .join("com/example/library/1.0/library-1.0.jar")
        );
        assert_eq!(files[3].kind, RequiredFileKind::Asset);
        assert_eq!(
            files[3].path,
            paths.assets_directory.join("objects/ab/abcdef0123456789")
        );
        assert_eq!(
            files[3].url.as_deref(),
            Some("https://resources.download.minecraft.net/ab/abcdef0123456789")
        );
    }

    #[test]
    fn the_files_that_are_named_twice_are_downloaded_once() {
        let directory = std::path::Path::new("game");
        let paths = paths(directory);
        let index = AssetIndexFile {
            virtual_assets: false,
            map_to_resources: false,
            objects: BTreeMap::from([
                (
                    "minecraft/first.ogg".to_string(),
                    AssetObject {
                        hash: "abcdef0123456789".to_string(),
                        size: 42,
                    },
                ),
                (
                    "minecraft/second.ogg".to_string(),
                    AssetObject {
                        hash: "abcdef0123456789".to_string(),
                        size: 42,
                    },
                ),
            ]),
        };
        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0", "x86_64", &features);
        let files = required_files(&manifest(), &paths, &context, Some(&index));

        assert_eq!(
            files
                .iter()
                .filter(|file| file.kind == RequiredFileKind::Asset)
                .count(),
            1
        );
    }

    #[test]
    fn a_version_without_assets_holds_no_files_of_the_assets() {
        let directory = std::path::Path::new("game");
        let paths = paths(directory);
        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0", "x86_64", &features);

        let files = required_files(&manifest(), &paths, &context, None);
        assert_eq!(files.len(), 3);
        assert!(files
            .iter()
            .all(|file| file.kind != RequiredFileKind::Asset));
    }

    #[test]
    fn the_missing_files_are_found() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-required-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(directory.join("libraries")).expect("the directory should be made");
        std::fs::write(directory.join("libraries/library.jar"), b"12345")
            .expect("the file should be written");

        let present = RequiredFile {
            path: directory.join("libraries/library.jar"),
            url: None,
            sha1: None,
            size: Some(5),
            kind: RequiredFileKind::Library,
        };
        let wrong_size = RequiredFile {
            size: Some(6),
            ..present.clone()
        };
        let absent = RequiredFile {
            path: directory.join("libraries/absent.jar"),
            ..present.clone()
        };

        assert!(is_present(&present));
        assert!(!is_present(&wrong_size));
        assert!(!is_present(&absent));
        assert_eq!(
            missing_files(&[present.clone(), wrong_size.clone(), absent.clone()]),
            vec![wrong_size, absent]
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_digest_of_a_file_is_read() {
        let directory = std::env::temp_dir().join(format!("hmcl-rs-verify-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        std::fs::write(directory.join("file"), b"hello").expect("the file should be written");

        let file = RequiredFile {
            path: directory.join("file"),
            url: None,
            sha1: Some("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d".to_string()),
            size: Some(5),
            kind: RequiredFileKind::Library,
        };
        assert!(verify_file(&file));

        let wrong = RequiredFile {
            sha1: Some("0000000000000000000000000000000000000000".to_string()),
            ..file
        };
        assert!(!verify_file(&wrong));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_libraries_of_a_version_are_checked() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-libraries-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let paths = paths(&directory);
        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0", "x86_64", &features);

        // The library is not there, so it is downloaded whatever the setting is.
        let library = crate::game::launch::library_file(&manifest().libraries[0], &context)
            .expect("the library should have a file");
        let path = paths.libraries_directory.join(&library.path);
        assert_eq!(
            libraries_to_download(&manifest(), &paths, &context, true),
            vec![RequiredFile {
                path: path.clone(),
                url: library.url.clone(),
                sha1: library.sha1.clone(),
                size: library.size,
                kind: RequiredFileKind::Library,
            }]
        );
        assert_eq!(
            libraries_to_download(&manifest(), &paths, &context, false).len(),
            1
        );

        // A file of the size the version tells is accepted; the digests are the
        // ones of the archive of a library the launcher checks.
        std::fs::create_dir_all(path.parent().expect("the file should have a parent"))
            .expect("the directory should be made");
        let mut writer =
            crate::util::zip::ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("net/example/Main.class", b"classes")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");

        let digest = Algorithm::Sha1
            .digest_file_to_string(&path)
            .expect("the digest should be read");
        let mut checked = manifest();
        checked.libraries[0]
            .downloads
            .as_mut()
            .expect("the library should name its file")
            .artifact
            .as_mut()
            .expect("the library should name its file")
            .sha1 = Some(digest.clone());
        checked.libraries[0]
            .downloads
            .as_mut()
            .expect("the library should name its file")
            .artifact
            .as_mut()
            .expect("the library should name its file")
            .size = None;

        assert!(libraries_to_download(&checked, &paths, &context, true).is_empty());

        // A library whose digest is another digest is downloaded again.
        checked.libraries[0]
            .downloads
            .as_mut()
            .expect("the library should name its file")
            .artifact
            .as_mut()
            .expect("the library should name its file")
            .sha1 = Some("0000000000000000000000000000000000000000".to_string());
        assert_eq!(
            libraries_to_download(&checked, &paths, &context, true).len(),
            1
        );
        // The check is skipped when the launcher is told not to make it.
        assert!(libraries_to_download(&checked, &paths, &context, false).is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }
}
