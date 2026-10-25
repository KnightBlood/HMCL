//! Preparing the files of a version and starting the game.
//!
//! The launcher does not download the files of the game itself; the way the
//! files are downloaded is the choice of the program the launcher runs in. The
//! launcher asks a [`FileDownloader`] for the files the plan tells it misses,
//! extracts the native libraries and starts the game.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use crate::game::component::{ComponentAnalyzer, GameComponentType};
use crate::game::verification::remove_legacy_forge_signatures;
use crate::util::zip::ZipError;

use super::natives::extract_natives;
use super::options::LaunchOptions;
use super::plan::LaunchPlan;
use super::process::{GameProcess, ProcessError};
use super::required::RequiredFile;

/// The way the launcher gets the files of the game.
pub trait FileDownloader {
    /// Downloads a file of the game to the place the version keeps it.
    ///
    /// The downloader checks the digest and the size of the file; the launcher
    /// starts the game with the files the downloader writes, so a file that is
    /// not the file the version tells may break the game.
    fn download(&self, file: &RequiredFile) -> Result<(), DownloadError>;
}

/// The error of downloading a file of the game.
///
/// The downloader of the launcher is [`crate::download::task::FileDownloadTask`],
/// which reports the failures of the servers, of the digests and of the file
/// system; a program that downloads the files another way reports its own
/// failures with [`DownloadError::new`].
pub use crate::download::task::DownloadError;

/// The error of preparing the files of a version.
#[derive(Debug)]
pub enum PrepareError {
    /// A directory or a file could not be written.
    Io(io::Error),
    /// A file of the version could not be downloaded.
    Download {
        /// The place of the file.
        file: PathBuf,
        /// The error the downloader wrote.
        error: DownloadError,
    },
    /// The archives of the native libraries could not be extracted.
    Native(ZipError),
    /// The file of the game could not be fixed.
    Verification(ZipError),
    /// A library of the version could not be repaired.
    Repair(super::repair::RepairError),
}

impl fmt::Display for PrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(
                formatter,
                "The files of the version cannot be prepared: {error}"
            ),
            Self::Download { file, error } => write!(
                formatter,
                "The file of the version cannot be downloaded: {}: {error}",
                file.display()
            ),
            Self::Native(error) => {
                write!(
                    formatter,
                    "The native libraries cannot be extracted: {error}"
                )
            }
            Self::Verification(error) => {
                write!(formatter, "The file of the game cannot be fixed: {error}")
            }
            Self::Repair(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for PrepareError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Download { error, .. } => Some(error),
            Self::Native(error) => Some(error),
            Self::Verification(error) => Some(error),
            Self::Repair(error) => Some(error),
        }
    }
}

/// The service that prepares the files of a version and starts the game.
pub struct LaunchService<'a> {
    /// The way the launcher gets the files of the game.
    downloader: &'a dyn FileDownloader,
}

impl<'a> LaunchService<'a> {
    /// Creates the service that downloads the files with the downloader.
    pub fn new(downloader: &'a dyn FileDownloader) -> Self {
        Self { downloader }
    }

    /// Prepares the files of a version and extracts the native libraries.
    ///
    /// The plan tells the files the launcher misses; the service downloads
    /// them, creates the directories of the game and extracts the archives of
    /// the native libraries. The files the plan does not miss are kept as they
    /// are.
    pub fn prepare(&self, plan: &LaunchPlan) -> Result<(), PrepareError> {
        plan.prepare().map_err(PrepareError::Io)?;

        for file in &plan.missing_files {
            self.downloader
                .download(file)
                .map_err(|error| PrepareError::Download {
                    file: file.path.clone(),
                    error,
                })?;
        }

        plan.prepare_assets().map_err(PrepareError::Io)?;

        extract_natives(&plan.native_archives, &plan.natives_directory)
            .map_err(PrepareError::Native)?;

        Ok(())
    }

    /// Prepares a version whose file of the game needs a fix before it runs.
    ///
    /// The old versions of Forge signed the file of the game with their own
    /// signatures, and the game refuses to run such a file. The signatures are
    /// removed after the files are downloaded and before the game is started.
    /// A version whose components are not known keeps the signatures, because
    /// the launcher cannot tell whether the file is a file of Forge.
    pub fn prepare_verified(
        &self,
        plan: &LaunchPlan,
        components: Option<&ComponentAnalyzer>,
    ) -> Result<(), PrepareError> {
        self.prepare(plan)?;

        let Some(components) = components else {
            return Ok(());
        };
        let Some(version) = components.version(GameComponentType::Game) else {
            return Ok(());
        };
        let Some(jar) = plan.client_jar() else {
            return Ok(());
        };

        remove_legacy_forge_signatures(
            jar,
            &crate::util::versioning::GameVersionNumber::as_game_version(version),
            components.has(GameComponentType::Forge),
        )
        .map_err(PrepareError::Verification)?;

        Ok(())
    }

    /// Prepares a version and repairs the libraries the launcher knows are
    /// broken.
    ///
    /// The libraries of OptiFine that a version of Forge refuses are fixed and
    /// the archives the launcher ships are copied before the plan is prepared,
    /// so the files the launcher downloads do not overwrite the fixed ones. The
    /// libraries that are not the files the version tells are downloaded.
    ///
    /// `read_resource` reads an archive the launcher ships by the name
    /// [`crate::game::verification::bundled_library_resource`] returns.
    pub fn prepare_repaired<F>(
        &self,
        manifest: &crate::download::game::InstanceManifest,
        paths: &super::options::LaunchPaths,
        options: &LaunchOptions,
        components: Option<&ComponentAnalyzer>,
        read_resource: F,
    ) -> Result<super::repair::LibraryRepair, PrepareError>
    where
        F: FnMut(&str) -> Option<Vec<u8>>,
    {
        let features = options.features();
        let context = super::command::rule_context(options, &features);

        let repair = super::repair::repair_libraries(
            manifest,
            paths,
            &context,
            options.integrity_check,
            components,
            read_resource,
        )
        .map_err(PrepareError::Repair)?;

        for file in &repair.downloads {
            self.downloader
                .download(file)
                .map_err(|error| PrepareError::Download {
                    file: file.path.clone(),
                    error,
                })?;
        }

        Ok(repair)
    }

    /// Starts the game of a version the service prepared.
    ///
    /// The output of the game is read by the launcher, so it may show the log
    /// of the game while the game runs; [`GameProcess::wait`] returns the way
    /// the game stopped.
    pub fn start(plan: &LaunchPlan, options: &LaunchOptions) -> Result<GameProcess, ProcessError> {
        GameProcess::spawn(start_command(plan, options))
    }
}

/// Returns the command the game of a version is started with.
///
/// The command runs in the directory the game reads its worlds from and holds
/// the environment variables the player set.
pub fn start_command(plan: &LaunchPlan, options: &LaunchOptions) -> Command {
    let mut command = if let Some(wrapper) = &options.wrapper {
        Command::new(wrapper)
    } else {
        Command::new(&options.java)
    };
    command
        .args(plan.command.arguments())
        .current_dir(&plan.working_directory);

    for (name, value) in &options.environment_variables {
        command.env(name, value);
    }

    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthInfo;
    use crate::download::game::{DownloadInfo, InstanceManifest, Library, LibraryDownloads};
    use crate::game::launch::{plan_launch, LaunchPaths};
    use std::cell::RefCell;

    struct FakeDownloader {
        downloaded: RefCell<Vec<PathBuf>>,
    }

    impl FakeDownloader {
        fn new() -> Self {
            Self {
                downloaded: RefCell::new(Vec::new()),
            }
        }
    }

    impl FileDownloader for FakeDownloader {
        fn download(&self, file: &RequiredFile) -> Result<(), DownloadError> {
            if let Some(parent) = file.path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| DownloadError::new(error.to_string()))?;
            }
            std::fs::write(&file.path, b"downloaded")
                .map_err(|error| DownloadError::new(error.to_string()))?;
            self.downloaded.borrow_mut().push(file.path.clone());
            Ok(())
        }
    }

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
                        size: Some(10),
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
    fn the_missing_files_are_downloaded() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-service-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            None,
        )
        .expect("the plan should be built");
        assert_eq!(plan.missing_files.len(), 1);

        let downloader = FakeDownloader::new();
        LaunchService::new(&downloader)
            .prepare(&plan)
            .expect("the version should be prepared");

        assert_eq!(downloader.downloaded.borrow().len(), 1);
        assert!(plan.missing_files[0].path.is_file());
        assert!(plan.natives_directory.is_dir());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_command_of_the_game_is_built() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-service-command-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let options = LaunchOptions {
            java: PathBuf::from("java"),
            ..LaunchOptions::default()
        };
        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &options,
            &paths(&directory),
            None,
        )
        .expect("the plan should be built");

        let command = start_command(&plan, &options);

        assert_eq!(command.get_program(), "java");
        assert_eq!(
            command.get_current_dir(),
            Some(plan.working_directory.as_path())
        );
        assert!(command
            .get_args()
            .any(|argument| argument == "net.minecraft.client.main.Main"));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_wrapper_is_used_as_the_program_of_the_command() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-service-wrapper-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let options = LaunchOptions {
            java: PathBuf::from("java"),
            wrapper: Some("wrapper".to_string()),
            ..LaunchOptions::default()
        };
        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &options,
            &paths(&directory),
            None,
        )
        .expect("the plan should be built");

        let command = start_command(&plan, &options);

        assert_eq!(command.get_program(), "wrapper");

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A downloader that writes a file of the game with the signatures of
    /// Forge.
    struct SignedDownloader;

    impl FileDownloader for SignedDownloader {
        fn download(&self, file: &RequiredFile) -> Result<(), DownloadError> {
            if let Some(parent) = file.path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| DownloadError::new(error.to_string()))?;
            }

            let mut writer = crate::util::zip::ZipWriter::create(&file.path)
                .map_err(|error| DownloadError::new(error.to_string()))?;
            for name in crate::game::verification::FORGE_SIGNATURE_FILES {
                writer
                    .add_file(name, b"signature")
                    .map_err(|error| DownloadError::new(error.to_string()))?;
            }
            writer
                .add_file("net/minecraft/client/Main.class", b"classes")
                .map_err(|error| DownloadError::new(error.to_string()))?;
            writer
                .finish()
                .map_err(|error| DownloadError::new(error.to_string()))
        }
    }

    #[test]
    fn the_signatures_of_an_old_forge_are_removed_after_the_files_are_downloaded() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-service-signatures-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let mut manifest = manifest();
        manifest.downloads = Some(std::collections::BTreeMap::from([(
            "client".to_string(),
            DownloadInfo {
                url: "https://example.com/client.jar".to_string(),
                path: None,
                sha1: None,
                size: None,
            },
        )]));
        manifest.patches = vec![crate::download::game::InstancePatch {
            id: Some("forge".to_string()),
            version: Some("1.5.2".to_string()),
            ..crate::download::game::InstancePatch::default()
        }];

        let plan = plan_launch(
            &manifest,
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            None,
        )
        .expect("the plan should be built");

        let components = ComponentAnalyzer::analyze(&manifest, Some("1.5.2"))
            .expect("the components should be read");

        LaunchService::new(&SignedDownloader)
            .prepare_verified(&plan, Some(&components))
            .expect("the version should be prepared");

        let jar = plan.client_jar().expect("the file should be there");
        let names = crate::util::zip::entries(jar)
            .expect("the file should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["net/minecraft/client/Main.class"]);

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_version_without_components_is_prepared_without_a_fix() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-service-nofix-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            None,
        )
        .expect("the plan should be built");

        LaunchService::new(&FakeDownloader::new())
            .prepare_verified(&plan, None)
            .expect("the version should be prepared");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_libraries_of_a_version_are_repaired_and_downloaded() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-service-repair-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let paths = paths(&directory);
        let manifest = manifest();
        let options = LaunchOptions::default();

        let downloader = FakeDownloader::new();
        let repair = LaunchService::new(&downloader)
            .prepare_repaired(&manifest, &paths, &options, None, |_| None)
            .expect("the libraries should be repaired");

        // The library is not there, so it is downloaded; nothing is fixed.
        assert_eq!(repair.downloads.len(), 1);
        assert!(repair.fixed.is_empty());
        assert_eq!(downloader.downloaded.borrow().len(), 1);
        assert!(repair.downloads[0].path.is_file());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_download_error_names_the_file() {
        struct Failing;

        impl FileDownloader for Failing {
            fn download(&self, _file: &RequiredFile) -> Result<(), DownloadError> {
                Err(DownloadError::new("the server does not answer"))
            }
        }

        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-service-failing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let plan = plan_launch(
            &manifest(),
            &AuthInfo::offline("Steve"),
            &LaunchOptions::default(),
            &paths(&directory),
            None,
        )
        .expect("the plan should be built");

        let error = LaunchService::new(&Failing)
            .prepare(&plan)
            .expect_err("the download should fail");
        assert!(matches!(error, PrepareError::Download { .. }));
        assert!(error.to_string().contains("the server does not answer"));

        let _ = std::fs::remove_dir_all(&directory);
    }
}
