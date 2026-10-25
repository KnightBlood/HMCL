//! Downloading the files a version of the game needs.
//!
//! The launcher knows the files a version needs from the metadata of the
//! version: the file of the game, the libraries, the index of the assets and
//! the files of the assets. This module joins the description of those files
//! with [`FileDownloadTask`], so the launcher downloads them through the cache
//! and checks every file against the digest the version tells.

use std::cell::RefCell;

use crate::download::cache::copy_file;
use crate::download::game::Library;
use crate::download::library_cache::DefaultCacheRepository;
use crate::download::task::{
    DownloadCandidates, DownloadError, DownloadSource, DownloadedFile, Fetcher, FileDownloadTask,
    IntegrityCheck,
};
use crate::game::launch::{FileDownloader, RequiredFile, RequiredFileKind};
use crate::util::digest::Algorithm;

/// The downloader the launcher of this crate uses.
///
/// The downloader takes the cache of the launcher and the way the program that
/// runs the launcher gets the body of an address. The files are kept in the
/// cache under their digests, so a file that is used by several versions of the
/// game is downloaded once.
pub struct TaskDownloader<'a> {
    /// The way the program gets the body of an address.
    fetcher: &'a dyn Fetcher,
    /// The cache of the files of the game.
    cache: RefCell<DefaultCacheRepository>,
}

impl<'a> TaskDownloader<'a> {
    /// Creates a downloader that keeps its files in a cache.
    pub fn new(fetcher: &'a dyn Fetcher, cache: DefaultCacheRepository) -> Self {
        Self {
            fetcher,
            cache: RefCell::new(cache),
        }
    }

    /// Returns the cache of the downloader.
    pub fn cache(&self) -> std::cell::Ref<'_, DefaultCacheRepository> {
        self.cache.borrow()
    }

    /// Returns the task that downloads a file of a version.
    ///
    /// The file of the game and the libraries are archives that the game
    /// opens, so a file that cannot be opened is refused before the game is
    /// started. The assets are not archives.
    pub fn task_of(&self, file: &RequiredFile) -> Option<FileDownloadTask> {
        let url = file.url.as_deref()?;
        let check = IntegrityCheck::of(Algorithm::Sha1, file.sha1.as_deref());

        Some(
            FileDownloadTask::new(DownloadCandidates::of(url), file.path.clone())
                .with_optional_integrity_check(check)
                .with_caching(true)
                .with_archive_check(matches!(
                    file.kind,
                    RequiredFileKind::Client | RequiredFileKind::Library
                )),
        )
    }

    /// Downloads a file and reports where the file came from.
    pub fn fetch(&self, file: &RequiredFile) -> Result<DownloadedFile, DownloadError> {
        let Some(url) = file.url.as_deref() else {
            // The file is kept on the machine of the player, so the launcher
            // only checks that the file is there.
            return if file.path.is_file() {
                Ok(DownloadedFile {
                    path: file.path.clone(),
                    source: DownloadSource::Cache,
                    url: None,
                })
            } else {
                Err(DownloadError::Failed(format!(
                    "{} is not on the machine",
                    file.path.display()
                )))
            };
        };

        let task = self
            .task_of(file)
            .ok_or_else(|| DownloadError::Failed(format!("{url} cannot be downloaded")))?;

        let mut cache = self.cache.borrow_mut();
        task.download(self.fetcher, Some(cache.cache_mut()))
    }

    /// Downloads a library the version of a mod loader installs.
    ///
    /// The libraries of the mod loaders are shipped inside their installer, so
    /// the digest of the installer is not the digest of the library. The cache
    /// of the libraries is asked first; a library the cache holds is copied to
    /// its place without asking a server.
    pub fn fetch_library(
        &self,
        library: &Library,
        path: &std::path::Path,
    ) -> Result<(), DownloadError> {
        {
            let mut cache = self.cache.borrow_mut();
            if let Some(cached) = cache.library_file(library) {
                return copy_file(&cached, path)
                    .map_err(|error| DownloadError::Io(error.to_string()));
            }
        }

        let file = RequiredFile {
            path: path.to_path_buf(),
            url: library
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
                .map(|artifact| artifact.url.clone())
                .filter(|url| !url.is_empty()),
            sha1: library
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
                .and_then(|artifact| artifact.sha1())
                .map(str::to_string),
            size: library
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
                .and_then(|artifact| artifact.size),
            kind: RequiredFileKind::Library,
        };

        self.fetch(&file)?;

        // The library is written to the cache as well, so the next version that
        // uses the library does not download it again.
        let _ = self.cache.borrow_mut().try_cache_library(library, path);

        Ok(())
    }
}

impl FileDownloader for TaskDownloader<'_> {
    /// Downloads a file of the game to the place the version keeps it.
    fn download(&self, file: &RequiredFile) -> Result<(), DownloadError> {
        self.fetch(file).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::zip::ZipWriter;
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// A server that answers with the file the tests name.
    struct FakeServer {
        /// The bodies, by address.
        answers: BTreeMap<String, Vec<u8>>,
        /// The addresses the launcher asked for.
        requests: Mutex<Vec<String>>,
    }

    impl FakeServer {
        fn new(answers: &[(&str, &[u8])]) -> Self {
            Self {
                answers: answers
                    .iter()
                    .map(|(url, body)| (url.to_string(), body.to_vec()))
                    .collect(),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn request_count(&self, url: &str) -> usize {
            self.requests
                .lock()
                .expect("the list should not be poisoned")
                .iter()
                .filter(|asked| asked.as_str() == url)
                .count()
        }
    }

    impl Fetcher for FakeServer {
        fn fetch(
            &self,
            url: &str,
            _headers: &BTreeMap<String, String>,
            sink: &mut dyn Write,
        ) -> Result<crate::download::task::ResponseInfo, String> {
            self.requests
                .lock()
                .expect("the list should not be poisoned")
                .push(url.to_string());

            match self.answers.get(url) {
                Some(body) => {
                    sink.write_all(body).map_err(|error| error.to_string())?;
                    Ok(crate::download::task::ResponseInfo {
                        url: url.to_string(),
                        status: 200,
                        headers: BTreeMap::new(),
                    })
                }
                None => Ok(crate::download::task::ResponseInfo {
                    url: url.to_string(),
                    status: 404,
                    headers: BTreeMap::new(),
                }),
            }
        }
    }

    /// Returns a directory that is removed when the test ends.
    fn directory(name: &str) -> PathBuf {
        let path = std::env::temp_dir()
            .join("hmcl-core-downloader")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Returns an archive that holds a manifest.
    fn archive() -> Vec<u8> {
        // The tests run in parallel, so every archive is written to a name of
        // its own.
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let index = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let path = std::env::temp_dir()
            .join("hmcl-core-downloader")
            .join(format!("archive-{}-{index}.zip", std::process::id()))
            .to_string_lossy()
            .into_owned();
        fs::create_dir_all(
            std::path::Path::new(&path)
                .parent()
                .expect("the archive should have a parent"),
        )
        .expect("the directory should be created");
        let _ = fs::remove_file(&path);
        let mut writer = ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n")
            .expect("the file should be added");
        writer.finish().expect("the archive should be finished");

        let bytes = fs::read(&path).expect("the archive should be read");
        let _ = fs::remove_file(&path);
        bytes
    }

    /// Returns a file of a version.
    fn required(
        path: PathBuf,
        url: Option<&str>,
        data: &[u8],
        kind: RequiredFileKind,
    ) -> RequiredFile {
        RequiredFile {
            path,
            url: url.map(str::to_string),
            sha1: Some(Algorithm::Sha1.digest_to_string(data)),
            size: Some(data.len() as u64),
            kind,
        }
    }

    #[test]
    fn a_file_of_a_version_is_downloaded_and_cached() {
        let base = directory("library");
        let data = archive();
        let server = FakeServer::new(&[("https://example.com/library.jar", &data)]);
        let downloader = TaskDownloader::new(&server, DefaultCacheRepository::new(&base));

        let file = required(
            base.join("libraries/library.jar"),
            Some("https://example.com/library.jar"),
            &data,
            RequiredFileKind::Library,
        );
        downloader
            .download(&file)
            .expect("the library should be downloaded");
        assert_eq!(fs::read(&file.path).expect("the file should be read"), data);
        assert!(downloader.cache().cache().file_exists(
            Algorithm::Sha1,
            file.sha1.as_deref().expect("the digest should be there")
        ));

        // The second download of the same file does not ask the server again.
        let other = required(
            base.join("libraries-2/library.jar"),
            Some("https://example.com/library.jar"),
            &data,
            RequiredFileKind::Library,
        );
        downloader
            .download(&other)
            .expect("the library should come from the cache");
        assert_eq!(server.request_count("https://example.com/library.jar"), 1);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_of_another_digest_is_refused() {
        let base = directory("digest");
        let data = archive();
        let server = FakeServer::new(&[("https://example.com/library.jar", &data)]);
        let downloader = TaskDownloader::new(&server, DefaultCacheRepository::new(&base));

        let mut file = required(
            base.join("libraries/library.jar"),
            Some("https://example.com/library.jar"),
            &data,
            RequiredFileKind::Library,
        );
        file.sha1 = Some(Algorithm::Sha1.digest_to_string(b"another file"));

        let error = downloader
            .download(&file)
            .expect_err("the file should be refused");
        assert!(error.to_string().contains("do not match"));
        assert!(!file.path.exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_that_is_kept_on_the_machine_is_only_checked() {
        let base = directory("local");
        let data = b"the library";
        let server = FakeServer::new(&[]);
        let downloader = TaskDownloader::new(&server, DefaultCacheRepository::new(&base));

        let path = base.join("libraries/local.jar");
        fs::create_dir_all(path.parent().expect("the file should have a parent"))
            .expect("the directory should be created");
        fs::write(&path, data).expect("the file should be written");

        let file = required(path.clone(), None, data, RequiredFileKind::Library);
        downloader
            .download(&file)
            .expect("the file should be accepted");
        assert!(path.is_file());

        // A file that is not there is refused.
        let missing = required(
            base.join("libraries/missing.jar"),
            None,
            data,
            RequiredFileKind::Library,
        );
        assert!(downloader.download(&missing).is_err());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_broken_library_is_refused() {
        let base = directory("broken");
        let data = b"this is not an archive";
        let server = FakeServer::new(&[("https://example.com/library.jar", data)]);
        let downloader = TaskDownloader::new(&server, DefaultCacheRepository::new(&base));

        let file = required(
            base.join("libraries/library.jar"),
            Some("https://example.com/library.jar"),
            data,
            RequiredFileKind::Library,
        );
        assert!(downloader.download(&file).is_err());
        assert!(!file.path.exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn an_asset_is_not_opened_as_an_archive() {
        let base = directory("asset");
        let data = b"the texture of the game";
        let server = FakeServer::new(&[("https://example.com/asset", data)]);
        let downloader = TaskDownloader::new(&server, DefaultCacheRepository::new(&base));

        let file = required(
            base.join("assets/objects/ab/abcdef"),
            Some("https://example.com/asset"),
            data,
            RequiredFileKind::Asset,
        );
        downloader
            .download(&file)
            .expect("the asset should be downloaded");

        let _ = fs::remove_dir_all(&base);
    }

    /// Returns a library of a mod loader.
    fn mod_library(name: &str, path: &str, data: &[u8]) -> crate::download::game::Library {
        use crate::download::game::{DownloadInfo, LibraryDownloads};

        crate::download::game::Library {
            name: name.to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(DownloadInfo {
                    url: format!("https://example.com/{path}"),
                    path: Some(path.to_string()),
                    sha1: Some(Algorithm::Sha1.digest_to_string(data)),
                    size: Some(data.len() as u64),
                }),
                classifiers: None,
            }),
            ..crate::download::game::Library::default()
        }
    }

    #[test]
    fn a_library_of_a_mod_loader_comes_from_the_cache() {
        let base = directory("mod-library");
        let data = archive();
        let path = "mod/library.jar";
        let server = FakeServer::new(&[("https://example.com/mod/library.jar", &data)]);
        let downloader = TaskDownloader::new(&server, DefaultCacheRepository::new(&base));

        let library = mod_library("com.example:library:1.0", path, &data);
        downloader
            .fetch_library(&library, &base.join("first/library.jar"))
            .expect("the library should be downloaded");
        // The library is written to the cache of the libraries.
        assert!(downloader.cache().entries().next().is_some());

        // The second version of the game that uses the library does not ask the
        // server again.
        downloader
            .fetch_library(&library, &base.join("second/library.jar"))
            .expect("the library should come from the cache");
        assert_eq!(
            server.request_count("https://example.com/mod/library.jar"),
            1
        );
        assert_eq!(
            fs::read(base.join("second/library.jar")).expect("the file should be read"),
            data
        );

        let _ = fs::remove_dir_all(&base);
    }
}
