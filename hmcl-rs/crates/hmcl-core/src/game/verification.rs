//! The fixes the launcher makes to the files of a version it installed.
//!
//! Some versions of the game were installed by the launchers of the past in a
//! way the launcher of today has to correct: the old versions of Forge signed
//! the file of the game with their own signatures, and the game refuses to run
//! a file whose signatures do not match. The signature files are removed, so
//! the game reads the file of the game as it is.

use std::path::Path;

use crate::download::game::Library;
use crate::download::library_cache::checksum_valid;
use crate::util::digest::Algorithm;
use crate::util::versioning::{GameVersionNumber, VersionNumber, VersionRange};
use crate::util::zip;

/// The version of the game before which the signatures of Forge are removed.
pub const FORGE_SIGNATURE_VERSION: &str = "1.6";

/// The signature files the old versions of Forge wrote into the file of the
/// game.
pub const FORGE_SIGNATURE_FILES: [&str; 2] = ["META-INF/MOJANG_C.DSA", "META-INF/MOJANG_C.SF"];

/// Removes the signatures of a version of Forge that the game refuses.
///
/// The signatures are removed only for a version of the game before
/// [`FORGE_SIGNATURE_VERSION`] that has Forge installed: the newer versions of
/// Forge do not sign the file of the game, and a version without Forge keeps
/// the signatures Mojang wrote.
///
/// Returns whether the file of the game was changed. A file that is not there
/// is not an error, because the version may not have downloaded its file yet.
pub fn remove_legacy_forge_signatures(
    jar: &Path,
    game_version: &GameVersionNumber,
    has_forge: bool,
) -> Result<bool, zip::ZipError> {
    if !has_forge || !jar.is_file() {
        return Ok(false);
    }

    let first_signed = GameVersionNumber::as_game_version(FORGE_SIGNATURE_VERSION);
    if game_version >= &first_signed {
        return Ok(false);
    }

    zip::remove_entries(jar, &FORGE_SIGNATURE_FILES)
}

/// Returns whether a file of a library is not the file the version tells.
///
/// The file is checked against the digest the version tells, then against the
/// checksums of the files of the library (the libraries of Forge name the
/// digests of their files instead of the digest of the archive), and finally
/// the archive is opened, because the game cannot read a file that is not an
/// archive.
///
/// A file the launcher cannot read is reported as a file to download again, so
/// a library the player broke is repaired instead of stopping the game. The
/// checks are skipped when `integrity_check` is off, but a library that is not
/// there is downloaded whatever the setting is.
pub fn library_needs_download(path: &Path, library: &Library, integrity_check: bool) -> bool {
    if !path.is_file() {
        return true;
    }

    if !integrity_check {
        return false;
    }

    // The digest the version tells is the digest of the library itself; a
    // library without a digest is a library the version does not describe.
    if let Some(expected) = library_digest(library) {
        match Algorithm::Sha1.digest_file_to_string(path) {
            Ok(actual) if actual.eq_ignore_ascii_case(expected) => {}
            Ok(_) => return true,
            Err(_) => return true,
        }
    }

    if let Some(checksums) = library.checksums.as_deref() {
        if !checksums.is_empty() && !checksum_valid(path, Some(checksums)) {
            return true;
        }
    }

    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
        && zip::entries(path).is_err()
    {
        return true;
    }

    false
}

/// Returns the digest of a library the version tells.
fn library_digest(library: &Library) -> Option<&str> {
    library.downloads.as_ref()?.artifact.as_ref()?.sha1()
}

/// The range of the versions of Forge whose `mods.toml` keeps OptiFine from
/// starting the game.
///
/// OptiFine of 1.20.4 ships a `META-INF/mods.toml` that the versions of Forge
/// between 48.0.0 and 49.0.50 read as a file of a mod, and the game stops. The
/// file is removed from the archive of OptiFine.
pub fn forge_optifine_broken_range() -> VersionRange<VersionNumber> {
    VersionRange::between(
        VersionNumber::parse("48.0.0"),
        VersionNumber::parse("49.0.50"),
    )
}

/// Removes the `mods.toml` of OptiFine that the versions of Forge in
/// [`forge_optifine_broken_range`] refuse.
///
/// The file is removed only from the archive of OptiFine of 1.20.4 and only
/// when the version of Forge of the instance is in the broken range. Returns
/// whether the archive was changed.
pub fn fix_optifine_of_forge(
    optifine_jar: &Path,
    optifine_version: &str,
    forge_version: Option<&str>,
) -> Result<bool, zip::ZipError> {
    if !optifine_version.starts_with("1.20.4_") {
        return Ok(false);
    }
    let Some(forge_version) = forge_version else {
        return Ok(false);
    };
    if !forge_optifine_broken_range().contains(&VersionNumber::parse(forge_version)) {
        return Ok(false);
    }
    if !optifine_jar.is_file() {
        return Ok(false);
    }

    zip::remove_entries(optifine_jar, &["META-INF/mods.toml"])
}

/// A file a version of Forge of the first generations needs but does not tell.
///
/// The version of Forge of Minecraft 1.5.2 was published without the files it
/// reads, so the launcher downloads them from the repository of the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FmlLibrary {
    /// The name of the file, which is also the name of the file in `lib`.
    pub name: String,
    /// The digest of the file, in the hexadecimal of SHA-1.
    pub sha1: String,
    /// The address the file is downloaded from.
    pub url: String,
}

/// Returns the files a version of Forge of the first generations needs.
///
/// Only the version of Forge of Minecraft 1.5.2 is known to miss its files; a
/// version the launcher does not know needs no other file, so the list is
/// empty.
pub fn fml_libraries(forge_version: Option<&str>) -> Vec<FmlLibrary> {
    let Some(forge_version) = forge_version else {
        return Vec::new();
    };
    if !forge_version.starts_with("7.8.1.") {
        return Vec::new();
    }

    // The files are kept on the site of the launcher, because the versions of
    // Forge of that generation do not name an address for them.
    const SITE: &str = "https://hmcl.glavo.site/metadata/fmllibs/";
    const FILES: [(&str, &str); 6] = [
        (
            "argo-small-3.2.jar",
            "58912ea2858d168c50781f956fa5b59f0f7c6b51",
        ),
        (
            "guava-14.0-rc3.jar",
            "931ae21fa8014c3ce686aaa621eae565fefb1a6a",
        ),
        (
            "asm-all-4.1.jar",
            "054986e962b88d8660ae4566475658469595ef58",
        ),
        (
            "bcprov-jdk15on-148.jar",
            "960dea7c9181ba0b17e8bab0c06a43f0a5f04e65",
        ),
        (
            "deobfuscation_data_1.5.2.zip",
            "446e55cd986582c70fcf12cb27bc00114c5adfd9",
        ),
        (
            "scala-library.jar",
            "458d046151ad179c85429ed7420ffb1eaf6ddf85",
        ),
    ];

    FILES
        .into_iter()
        .map(|(name, sha1)| FmlLibrary {
            name: name.to_string(),
            sha1: sha1.to_string(),
            url: format!("{SITE}{name}"),
        })
        .collect()
}

/// Returns the name of the archive of the launcher a library of the launcher is
/// copied from.
///
/// The launcher adds a bootstrap and a service of the transformer to the
/// versions of Forge and of NeoForge; the archives are shipped inside the
/// launcher, so the library is copied instead of downloaded.
pub fn bundled_library_resource(library: &Library) -> Option<&'static str> {
    let artifact = library.artifact().ok()?;

    match (artifact.group.as_str(), artifact.name.as_str()) {
        ("org.jackhuang.hmcl", "mmc-bootstrap") => {
            Some("/assets/game/HMCLMultiMCBootstrap-1.0.jar")
        }
        ("org.jackhuang.hmcl", "transformer-discovery-service") => {
            Some("/assets/game/HMCLTransformerDiscoveryService-1.0.jar")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::zip::{entries, read_entry, ZipWriter};

    /// Returns a directory that is removed when the test ends.
    fn directory(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir()
            .join("hmcl-core-verification")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Writes a file of a version with the signatures of Forge.
    fn signed_jar(base: &Path) -> std::path::PathBuf {
        let path = base.join("client.jar");
        let mut writer = ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("META-INF/MOJANG_C.DSA", b"signature")
            .expect("the file should be added");
        writer
            .add_file("META-INF/MOJANG_C.SF", b"signature file")
            .expect("the file should be added");
        writer
            .add_file("net/minecraft/client/Minecraft.class", b"classes")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");
        path
    }

    /// Returns the names of the files of an archive.
    fn names(path: &Path) -> Vec<String> {
        let mut names = entries(path)
            .expect("the archive should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[test]
    fn the_signatures_of_an_old_forge_are_removed() {
        let base = directory("old-forge");
        let jar = signed_jar(&base);
        let version = GameVersionNumber::as_game_version("1.5.2");

        assert!(remove_legacy_forge_signatures(&jar, &version, true)
            .expect("the signatures should be removed"));
        assert_eq!(names(&jar), vec!["net/minecraft/client/Minecraft.class"]);
        // The file of the game is kept.
        assert!(read_entry(&jar, "net/minecraft/client/Minecraft.class")
            .expect("the file should be read")
            .is_some());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_signatures_of_a_version_without_forge_are_kept() {
        let base = directory("without-forge");
        let jar = signed_jar(&base);
        let version = GameVersionNumber::as_game_version("1.5.2");

        assert!(!remove_legacy_forge_signatures(&jar, &version, false)
            .expect("the archive should be read"));
        assert_eq!(names(&jar).len(), 3);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_signatures_of_a_new_version_of_forge_are_kept() {
        let base = directory("new-forge");
        let jar = signed_jar(&base);
        let version = GameVersionNumber::as_game_version("1.6");

        assert!(!remove_legacy_forge_signatures(&jar, &version, true)
            .expect("the archive should be read"));
        assert_eq!(names(&jar).len(), 3);

        let version = GameVersionNumber::as_game_version("1.12.2");
        assert!(!remove_legacy_forge_signatures(&jar, &version, true)
            .expect("the archive should be read"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_version_without_a_file_of_the_game_is_not_an_error() {
        let base = directory("missing");
        let version = GameVersionNumber::as_game_version("1.5.2");

        assert!(
            !remove_legacy_forge_signatures(&base.join("client.jar"), &version, true)
                .expect("the missing file should be accepted")
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Returns a library whose file has the given content.
    fn library_of(data: &[u8], checksums: Option<Vec<String>>) -> Library {
        use crate::download::game::{DownloadInfo, LibraryDownloads};

        Library {
            name: "com.example:library:1.0".to_string(),
            checksums,
            downloads: Some(LibraryDownloads {
                artifact: Some(DownloadInfo {
                    url: "https://example.com/library.jar".to_string(),
                    path: None,
                    sha1: Some(Algorithm::Sha1.digest_to_string(data)),
                    size: Some(data.len() as u64),
                }),
                classifiers: None,
            }),
            ..Library::default()
        }
    }

    /// Returns a library that tells the digest of a file.
    fn library_with_digest(digest: &str, checksums: Option<Vec<String>>) -> Library {
        use crate::download::game::{DownloadInfo, LibraryDownloads};

        Library {
            name: "com.example:library:1.0".to_string(),
            checksums,
            downloads: Some(LibraryDownloads {
                artifact: Some(DownloadInfo {
                    url: "https://example.com/library.jar".to_string(),
                    path: None,
                    sha1: Some(digest.to_string()),
                    size: None,
                }),
                classifiers: None,
            }),
            ..Library::default()
        }
    }

    /// Writes an archive with one file.
    fn library_jar(base: &Path, data: &[u8]) -> std::path::PathBuf {
        let path = base.join("library.jar");
        let mut writer = ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("net/example/Main.class", data)
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");
        path
    }

    #[test]
    fn a_library_that_is_not_there_is_downloaded() {
        let base = directory("library-missing");
        let path = base.join("library.jar");

        assert!(library_needs_download(
            &path,
            &library_of(b"the library", None),
            true
        ));
        // The setting only spares the check of a file that is there.
        assert!(library_needs_download(
            &path,
            &library_of(b"the library", None),
            false
        ));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_library_of_another_digest_is_downloaded() {
        let base = directory("library-digest");
        let path = library_jar(&base, b"the library");
        let digest = Algorithm::Sha1
            .digest_file_to_string(&path)
            .expect("the digest should be read");

        assert!(!library_needs_download(
            &path,
            &library_with_digest(&digest, None),
            true
        ));
        assert!(library_needs_download(
            &path,
            &library_with_digest(&Algorithm::Sha1.digest_to_string(b"another library"), None),
            true
        ));

        // The checks are skipped when the launcher is told not to make them.
        assert!(!library_needs_download(
            &path,
            &library_with_digest(&Algorithm::Sha1.digest_to_string(b"another library"), None),
            false
        ));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_library_that_is_not_an_archive_is_downloaded() {
        let base = directory("library-broken");
        let path = base.join("library.jar");
        std::fs::write(&path, b"this is not an archive").expect("the file should be written");

        // The library tells no digest, so only the archive check can tell that
        // the file is broken.
        let library = library_of(b"", None);
        let library = Library {
            downloads: Some(crate::download::game::LibraryDownloads {
                artifact: Some(crate::download::game::DownloadInfo::default()),
                classifiers: None,
            }),
            ..library
        };

        assert!(library_needs_download(&path, &library, true));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_library_whose_checksums_do_not_match_is_downloaded() {
        let base = directory("library-checksums");
        let path = library_jar(&base, b"the library");
        let digest = Algorithm::Sha1
            .digest_file_to_string(&path)
            .expect("the digest should be read");

        // The version lists the digest of the archive itself.
        let library = library_with_digest(&digest, Some(vec![digest.clone()]));
        assert!(!library_needs_download(&path, &library, true));

        let library = library_with_digest(
            &digest,
            Some(vec![Algorithm::Sha1.digest_to_string(b"another file")]),
        );
        assert!(library_needs_download(&path, &library, true));

        // An empty list of checksums is a library the version gives no digest
        // of, so the archive check is the one that matters.
        let library = library_with_digest(&digest, Some(Vec::new()));
        assert!(!library_needs_download(&path, &library, true));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Writes an archive of OptiFine with the file the versions of Forge
    /// refuse.
    fn optifine_jar(base: &Path) -> std::path::PathBuf {
        let path = base.join("OptiFine.jar");
        let mut writer = ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("META-INF/mods.toml", b"modLoader = \"javafml\"")
            .expect("the file should be added");
        writer
            .add_file("net/optifine/Config.class", b"classes")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");
        path
    }

    #[test]
    fn the_mods_file_of_optifine_is_removed_for_a_broken_forge() {
        let base = directory("optifine");
        let jar = optifine_jar(&base);

        assert!(
            fix_optifine_of_forge(&jar, "1.20.4_HD_U_I6", Some("48.0.0"))
                .expect("the file should be removed")
        );
        assert_eq!(names(&jar), vec!["net/optifine/Config.class"]);
        // The file is already gone, so the archive is not written again.
        assert!(
            !fix_optifine_of_forge(&jar, "1.20.4_HD_U_I6", Some("48.0.0"))
                .expect("the archive should be read")
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_mods_file_of_optifine_is_kept_for_another_version() {
        let base = directory("optifine-kept");
        let jar = optifine_jar(&base);

        // A version of Forge the file does not stop.
        assert!(
            !fix_optifine_of_forge(&jar, "1.20.4_HD_U_I6", Some("47.4.0"))
                .expect("the archive should be read")
        );
        // A version of the game OptiFine is not installed for.
        assert!(
            !fix_optifine_of_forge(&jar, "1.20.1_HD_U_I6", Some("48.0.0"))
                .expect("the archive should be read")
        );
        // No Forge at all.
        assert!(!fix_optifine_of_forge(&jar, "1.20.4_HD_U_I6", None)
            .expect("the archive should be read"));
        assert_eq!(names(&jar).len(), 2);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_files_of_forge_of_minecraft_1_5_2_are_listed() {
        let files = fml_libraries(Some("7.8.1.738"));
        assert_eq!(files.len(), 6);
        assert_eq!(files[0].name, "argo-small-3.2.jar");
        assert_eq!(files[0].sha1, "58912ea2858d168c50781f956fa5b59f0f7c6b51");
        assert_eq!(
            files[0].url,
            "https://hmcl.glavo.site/metadata/fmllibs/argo-small-3.2.jar"
        );

        // The versions of Forge the launcher does not know need no other file.
        assert!(fml_libraries(Some("1.12.2-14.23.5.2859")).is_empty());
        assert!(fml_libraries(None).is_empty());
    }

    #[test]
    fn the_libraries_of_the_launcher_are_found() {
        use crate::download::game::Library;

        let library = Library {
            name: "org.jackhuang.hmcl:mmc-bootstrap:1.0".to_string(),
            ..Library::default()
        };
        assert_eq!(
            bundled_library_resource(&library),
            Some("/assets/game/HMCLMultiMCBootstrap-1.0.jar")
        );

        let library = Library {
            name: "org.jackhuang.hmcl:transformer-discovery-service:1.0".to_string(),
            ..Library::default()
        };
        assert_eq!(
            bundled_library_resource(&library),
            Some("/assets/game/HMCLTransformerDiscoveryService-1.0.jar")
        );

        let library = Library {
            name: "com.example:library:1.0".to_string(),
            ..Library::default()
        };
        assert_eq!(bundled_library_resource(&library), None);
    }
}
