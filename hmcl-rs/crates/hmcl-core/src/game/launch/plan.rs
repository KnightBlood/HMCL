//! The plan of a start of a version of the game.
//!
//! The launcher may not start the game at once: the version needs files the
//! machine of the player does not have yet, and the native libraries need to
//! be extracted. The plan tells the launcher which files are missing, so the
//! launcher downloads them before it starts the game.

use std::io;
use std::path::{Path, PathBuf};

use crate::auth::AuthInfo;
use crate::download::game::{AssetIndexFile, InstanceManifest};
use crate::game::game_version::minecraft_version;
use crate::util::versioning::GameVersionNumber;

use super::assets::{virtual_assets, VirtualAssets};
use super::command::{build_command, rule_context, LaunchCommand, LaunchError};
use super::native_patch::{patch_natives, NativePatchOptions};
use super::natives::{native_archives, NativeArchive};
use super::options::{LaunchOptions, LaunchPaths};
use super::required::{
    libraries_to_download, missing_files, required_files, RequiredFile, RequiredFileKind,
};

/// The version the launcher plans a version of the game whose file it cannot
/// read as.
///
/// The launcher names the version `0.0` when it cannot read the version of the
/// file of the game, which is older than every version the patches of the
/// newest versions apply to.
const UNKNOWN_GAME_VERSION: &str = "0.0";

/// The plan of a start of a version of the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    /// The command the game is started with.
    pub command: LaunchCommand,
    /// The files the version needs before it is started.
    pub required_files: Vec<RequiredFile>,
    /// The files of the version that are not on the machine of the player.
    pub missing_files: Vec<RequiredFile>,
    /// The archives of the native libraries of the version.
    pub native_archives: Vec<NativeArchive>,
    /// The directories the game reads its files from.
    pub directories: Vec<PathBuf>,
    /// The directory the game reads its assets from.
    ///
    /// The old versions read their assets from a directory that maps the names
    /// of the files to the files; the launcher copies the files of the assets
    /// into that directory before it starts the game.
    pub assets_directory: PathBuf,
    /// The files of the assets to copy when the game reads the assets by their
    /// names.
    pub virtual_assets: Option<VirtualAssets>,
    /// The directory the game runs in.
    pub working_directory: PathBuf,
    /// The directory the native libraries are extracted to.
    pub natives_directory: PathBuf,
}

impl LaunchPlan {
    /// Returns whether the launcher has every file the version needs.
    pub fn is_ready(&self) -> bool {
        self.missing_files.is_empty()
    }

    /// Returns the file of the game of the version.
    ///
    /// The file is the one of the required files the version downloads as the
    /// client; a version whose metadata does not name its client file keeps the
    /// file next to its manifest.
    pub fn client_jar(&self) -> Option<&Path> {
        self.required_files
            .iter()
            .find(|file| file.kind == RequiredFileKind::Client)
            .map(|file| file.path.as_path())
    }

    /// Creates the directories of the game and empties the directory of the
    /// native libraries.
    ///
    /// The native libraries of the earlier runs are removed, because the
    /// libraries of another version of the game may have the same names; the
    /// launcher extracts the libraries of the version after it prepares the
    /// plan.
    pub fn prepare(&self) -> io::Result<()> {
        for directory in &self.directories {
            std::fs::create_dir_all(directory)?;
        }

        if self.natives_directory.exists() {
            std::fs::remove_dir_all(&self.natives_directory)?;
        }
        std::fs::create_dir_all(&self.natives_directory)?;

        Ok(())
    }

    /// Copies the files of the assets the game reads by their names.
    ///
    /// The files of the assets are downloaded after the launcher creates the
    /// directories of the game, so the launcher copies them after it downloads
    /// them and before it starts the game.
    pub fn prepare_assets(&self) -> io::Result<()> {
        match &self.virtual_assets {
            Some(assets) => assets.copy(),
            None => Ok(()),
        }
    }
}

/// Builds the plan of a start of a version of the game.
///
/// The version must be resolved: the changes of the versions it inherits from
/// and the patches of the repository must be applied. The index of the assets
/// is read from the file of the index, which the launcher downloads when the
/// file is not on the machine of the player.
///
/// The arguments and the libraries of the version are repaired for the start:
/// the tweak classes of the mod loaders are put back and the service of the
/// transformer is added, so the command holds the arguments the game needs. A
/// version that is not resolved is planned as it is, because the launcher
/// cannot tell which component the patches install.
///
/// The native libraries are patched after the repairs: the versions of the
/// game Mojang built no native libraries for read the ones the players built,
/// which changes the libraries the plan downloads and extracts.
pub fn plan_launch(
    manifest: &InstanceManifest,
    auth: &AuthInfo,
    options: &LaunchOptions,
    paths: &LaunchPaths,
    index: Option<&AssetIndexFile>,
) -> Result<LaunchPlan, LaunchError> {
    let features = options.features();
    let context = rule_context(options, &features);

    let repaired = super::normalize::repair_for_launch(manifest).ok();
    let manifest = repaired.as_ref().unwrap_or(manifest);

    // The version of the game is read from the file of the game; a file the
    // launcher cannot read names no version, which the launcher reads as the
    // oldest one, so the patches of the newest versions do not apply.
    let game_version = minecraft_version(&paths.client_jar)
        .ok()
        .flatten()
        .map_or_else(
            || GameVersionNumber::as_game_version(UNKNOWN_GAME_VERSION),
            |version| GameVersionNumber::as_game_version(&version),
        );

    let mut native_arguments = Vec::new();
    let patched = patch_natives(
        manifest,
        &game_version,
        options.java_platform,
        NativePatchOptions {
            use_custom_natives: options.use_custom_natives,
            use_native_glfw_or_sdl: options.use_native_glfw_or_sdl,
            use_native_openal: options.use_native_openal,
            not_patch_natives: options.not_patch_natives,
        },
        &context,
        &mut native_arguments,
    );
    let manifest = &patched;

    // The arguments the patch adds are the ones of the player, so they are
    // written where the player writes theirs.
    let mut start_options = options.clone();
    start_options.jvm_arguments.extend(native_arguments);

    let working_directory = options.working_directory(paths).to_path_buf();
    let assets = index.and_then(|index| {
        virtual_assets(
            index,
            &paths.assets_directory,
            &paths.asset_index,
            &working_directory,
        )
    });

    // The old versions read their assets from the directory the launcher
    // builds out of the files of the assets, so the command names that
    // directory instead of the directory the files are kept in.
    let command = {
        let mut command_paths = paths.clone();
        if let Some(assets) = &assets {
            command_paths.assets_directory = assets.directory.clone();
        }
        build_command(
            manifest,
            auth,
            &start_options,
            &game_version,
            &command_paths,
        )?
    };

    let files = required_files(manifest, paths, &context, index);
    let mut missing = missing_files(&files);

    // A library that is there but is not the file the version tells is
    // downloaded again; the launcher makes the check only when the player asks
    // it to, because reading the files of the version costs the time of the
    // start of the game.
    if options.integrity_check {
        for file in libraries_to_download(manifest, paths, &context, true) {
            if !missing.iter().any(|known| known.path == file.path) {
                missing.push(file);
            }
        }
    }
    let archives = native_archives(manifest, paths, &context);

    let mut directories = Vec::new();
    for directory in [
        paths.game_directory.clone(),
        paths.assets_directory.clone(),
        paths.libraries_directory.clone(),
        paths.natives_directory.clone(),
        working_directory.clone(),
    ] {
        if !directories.contains(&directory) {
            directories.push(directory);
        }
    }

    Ok(LaunchPlan {
        command,
        required_files: files,
        missing_files: missing,
        native_archives: archives,
        directories,
        assets_directory: assets
            .as_ref()
            .map(|assets| assets.directory.clone())
            .unwrap_or_else(|| paths.assets_directory.clone()),
        virtual_assets: assets,
        working_directory,
        natives_directory: paths.natives_directory.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{DownloadInfo, Library, LibraryDownloads};

    fn manifest() -> InstanceManifest {
        InstanceManifest {
            id: Some("1.20.1".to_string()),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            libraries: vec![Library {
                name: "com.example:library:1.0".to_string(),
                downloads: Some(LibraryDownloads {
                    artifact: Some(DownloadInfo {
                        url: "https://example.com/library.jar".to_string(),
                        path: Some("com/example/library/1.0/library-1.0.jar".to_string()),
                        sha1: Some("abc".to_string()),
                        size: Some(42),
                    }),
                    classifiers: None,
                }),
                ..Library::default()
            }],
            ..InstanceManifest::default()
        }
    }

    fn paths(directory: &std::path::Path) -> LaunchPaths {
        LaunchPaths {
            game_directory: directory.join("game"),
            assets_directory: directory.join("assets"),
            asset_index: "1.20".to_string(),
            natives_directory: directory.join("natives"),
            libraries_directory: directory.join("libraries"),
            client_jar: directory.join("client.jar"),
        }
    }

    #[test]
    fn the_plan_lists_the_files_of_the_version() {
        let directory = std::env::temp_dir().join(format!("hmcl-rs-plan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            None,
        )
        .expect("the plan should be built");

        assert_eq!(plan.command.main_class, "net.minecraft.client.main.Main");
        assert_eq!(plan.required_files.len(), 1);
        assert_eq!(plan.missing_files.len(), 1);
        assert!(!plan.is_ready());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_old_versions_read_the_assets_from_the_directory_of_the_names() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-plan-assets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let manifest = InstanceManifest {
            minecraft_arguments: Some("--assetsDir ${game_assets}".to_string()),
            ..manifest()
        };
        let index = AssetIndexFile {
            virtual_assets: true,
            map_to_resources: false,
            objects: std::collections::BTreeMap::new(),
        };

        let plan = plan_launch(
            &manifest,
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            Some(&index),
        )
        .expect("the plan should be built");

        let assets = directory.join("assets/virtual/1.20");
        assert_eq!(plan.assets_directory, assets);
        assert!(plan.virtual_assets.is_some());
        assert!(plan
            .command
            .game_arguments
            .windows(2)
            .any(|pair| pair[0] == "--assetsDir" && std::path::Path::new(&pair[1]) == assets));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_newer_versions_read_the_assets_from_the_directory_of_the_digests() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-plan-assets-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let index = AssetIndexFile {
            virtual_assets: false,
            map_to_resources: false,
            objects: std::collections::BTreeMap::new(),
        };

        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            Some(&index),
        )
        .expect("the plan should be built");

        assert_eq!(plan.assets_directory, directory.join("assets"));
        assert_eq!(plan.virtual_assets, None);

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_preparation_copies_the_assets_of_the_old_versions() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-plan-assets-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let objects = directory.join("assets/objects/ab");
        std::fs::create_dir_all(&objects).expect("the directory should be made");
        std::fs::write(objects.join("abcdef0123456789"), b"click")
            .expect("the file should be written");

        let index = AssetIndexFile {
            virtual_assets: true,
            map_to_resources: false,
            objects: std::collections::BTreeMap::from([(
                "minecraft/click.ogg".to_string(),
                crate::download::game::AssetObject {
                    hash: "abcdef0123456789".to_string(),
                    size: 5,
                },
            )]),
        };

        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            Some(&index),
        )
        .expect("the plan should be built");

        plan.prepare().expect("the plan should be prepared");
        plan.prepare_assets().expect("the assets should be copied");

        assert_eq!(
            std::fs::read(directory.join("assets/virtual/1.20/minecraft/click.ogg"))
                .expect("the file should be read"),
            b"click"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_library_of_another_digest_is_planned_for_download() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-plan-integrity-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let paths = paths(&directory);
        let library = manifest()
            .libraries
            .into_iter()
            .next()
            .expect("the version should hold a library");
        let default_options = LaunchOptions::default();
        let features = default_options.features();
        let context = rule_context(&default_options, &features);
        let file = crate::game::launch::library_file(&library, &context)
            .expect("the library should have a file");
        let path = paths.libraries_directory.join(&file.path);
        std::fs::create_dir_all(path.parent().expect("the file should have a parent"))
            .expect("the directory should be made");
        // The size the version tells is the size of the file, so only the
        // digest can tell that the player wrote another file.
        std::fs::write(&path, vec![b'x'; 42]).expect("the file should be written");

        // The library is there, so the plan of a start that does not check the
        // files of the version does not download it.
        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths,
            None,
        )
        .expect("the plan should be built");
        assert!(plan.missing_files.is_empty());

        let options = LaunchOptions {
            integrity_check: true,
            ..LaunchOptions::default()
        };
        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &options,
            &paths,
            None,
        )
        .expect("the plan should be built");
        assert_eq!(plan.missing_files.len(), 1);
        assert_eq!(plan.missing_files[0].path, path);
        assert_eq!(
            plan.missing_files[0].kind,
            crate::game::launch::RequiredFileKind::Library
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_native_libraries_of_the_version_are_patched_for_the_runtime() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-plan-natives-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        // The version of the game is read from the file of the game, so the
        // file has to be an archive that tells the version.
        let paths = paths(&directory);
        let mut writer = crate::util::zip::ZipWriter::create(&paths.client_jar)
            .expect("the archive should be created");
        writer
            .add_file("version.json", br#"{"id": "1.16.5"}"#)
            .expect("the version should be added");
        writer.finish().expect("the archive should be written");

        let manifest = InstanceManifest {
            libraries: vec![
                // The version names the library itself and the files of the
                // system it runs on; the list replaces both.
                Library {
                    name: "org.lwjgl:lwjgl:3.1.6".to_string(),
                    ..Library::default()
                },
                Library {
                    name: "org.lwjgl:lwjgl:3.1.6".to_string(),
                    natives: Some(std::collections::BTreeMap::from([(
                        "linux".to_string(),
                        "natives-linux".to_string(),
                    )])),
                    ..Library::default()
                },
            ],
            ..manifest()
        };
        let options = LaunchOptions {
            os: crate::util::operating_system::OsType::Linux,
            arch: crate::util::operating_system::Arch::Aarch64,
            java_platform: crate::java::Platform::new(
                crate::util::operating_system::OsType::Linux,
                crate::util::operating_system::Arch::Aarch64,
            ),
            ..LaunchOptions::default()
        };

        let plan = plan_launch(
            &manifest,
            &AuthInfo::offline("Steve"),
            &options,
            &paths,
            None,
        )
        .expect("the plan should be built");

        // The machines of the ARM architecture read the native libraries the
        // players built, so the plan holds the library of the list rather than
        // the one of the version.
        let patched = directory
            .join("libraries")
            .join("org/lwjgl/lwjgl/3.3.2/lwjgl-3.3.2-natives-linux-arm64.jar");
        let kept = directory
            .join("libraries")
            .join("org/lwjgl/lwjgl/3.1.6/lwjgl-3.1.6-natives-linux.jar");

        assert!(plan.required_files.iter().any(|file| file.path == patched));
        assert!(plan.required_files.iter().all(|file| file.path != kept));

        // The version of LWJGL changes, so the game of the versions of Sodium
        // reads the argument that keeps it from crashing.
        assert!(plan
            .command
            .jvm_arguments
            .iter()
            .any(|argument| argument == "-Dsodium.checks.issue2561=false"));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_version_without_a_main_class_has_no_plan() {
        let manifest = InstanceManifest {
            id: Some("broken".to_string()),
            ..InstanceManifest::default()
        };
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-plan-broken-{}", std::process::id()));

        assert_eq!(
            plan_launch(
                &manifest,
                &AuthInfo::offline("Steve"),
                &LaunchOptions::default(),
                &paths(&directory),
                None
            ),
            Err(LaunchError::MissingMainClass)
        );
    }

    #[test]
    fn the_preparation_creates_the_directories_and_empties_the_natives() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-plan-prepare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            None,
        )
        .expect("the plan should be built");

        std::fs::create_dir_all(&plan.natives_directory).expect("the directory should be made");
        std::fs::write(plan.natives_directory.join("stale.dll"), b"stale")
            .expect("the file should be written");

        plan.prepare().expect("the plan should be prepared");

        assert!(plan.natives_directory.is_dir());
        assert!(!plan.natives_directory.join("stale.dll").exists());
        assert!(plan.directories.iter().all(|directory| directory.is_dir()));

        let _ = std::fs::remove_dir_all(&directory);
    }
}
