//! Downloading a file from the servers, with the cache and the retries.
//!
//! The launcher asks the servers of the game for the same files again and
//! again, and the mirrors of the servers answer differently: a file may be
//! missing from one mirror, a mirror may be down for a moment, and a download
//! may break halfway through. The task of this module tells the downloader
//! which addresses to try, checks the file it got, and writes the file only
//! when it is the file the launcher asked for.
//!
//! The launcher does not open the connections itself: the program the launcher
//! runs in names a [`Fetcher`], which writes the body of an answer to the
//! place it is given. So the task can be tested without a network, and the
//! program that runs the launcher chooses the library it downloads with.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::download::cache::{copy_file, CacheRepository};
use crate::util::digest::{is_sha1_digest, Algorithm};
use crate::util::zip;

/// The number of the attempts a download makes before it gives up.
///
/// The launcher tries the address once more than this number, so the file is
/// asked for six times before the mirror is given up on.
pub const DEFAULT_RETRY: u32 = 5;

/// The moment the launcher waits before it asks a mirror again.
pub const RETRY_DELAY: Duration = Duration::from_millis(200);

/// The address of a file, with the way it is asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadCandidate {
    /// The address of the file.
    pub url: String,
    /// The number of the attempts after the first one.
    ///
    /// A candidate without a number uses [`DEFAULT_RETRY`].
    pub retry: Option<u32>,
}

impl DownloadCandidate {
    /// Creates a candidate with the default number of the attempts.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            retry: None,
        }
    }

    /// Creates a candidate that is asked for the given number of times.
    pub fn with_retry(url: impl Into<String>, retry: u32) -> Self {
        Self {
            url: url.into(),
            retry: Some(retry),
        }
    }

    /// Returns the number of the attempts after the first one.
    pub fn attempts(&self) -> u32 {
        self.retry.unwrap_or(DEFAULT_RETRY)
    }
}

/// The addresses of a file, in the order they are tried.
///
/// The addresses are a list the task iterates, so a file that is kept in many
/// mirrors is found even when one of the mirrors is down. A list without an
/// address is refused, because the launcher would have nothing to try.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadCandidates {
    /// The addresses, in the order they are tried.
    candidates: Vec<DownloadCandidate>,
}

impl DownloadCandidates {
    /// Creates the addresses of a file.
    ///
    /// # Panics
    ///
    /// The list panics when it holds no address, which is a mistake of the
    /// program and not a failure of a download.
    pub fn new(candidates: Vec<DownloadCandidate>) -> Self {
        assert!(
            !candidates.is_empty(),
            "the addresses of a file must not be empty"
        );
        Self { candidates }
    }

    /// Creates the addresses of a file from the addresses.
    pub fn of_urls(urls: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::new(urls.into_iter().map(DownloadCandidate::new).collect())
    }

    /// Creates the addresses of a file that is kept in one place.
    pub fn of(url: impl Into<String>) -> Self {
        Self::new(vec![DownloadCandidate::new(url)])
    }

    /// Returns the address that is tried first.
    pub fn primary(&self) -> &DownloadCandidate {
        &self.candidates[0]
    }

    /// Returns the addresses, in the order they are tried.
    pub fn candidates(&self) -> &[DownloadCandidate] {
        &self.candidates
    }

    /// Returns the number of the addresses.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// Returns whether the list holds no address, which the constructor
    /// refuses.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

/// The check a downloaded file is held to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityCheck {
    /// The algorithm of the digest of the file.
    pub algorithm: Algorithm,
    /// The digest of the file, as the version of the game tells it.
    pub checksum: String,
}

impl IntegrityCheck {
    /// Creates a check of a digest of SHA-1.
    pub fn sha1(checksum: impl Into<String>) -> Self {
        Self {
            algorithm: Algorithm::Sha1,
            checksum: checksum.into(),
        }
    }

    /// Creates the check of a digest, if the server told one.
    pub fn of(algorithm: Algorithm, checksum: Option<&str>) -> Option<Self> {
        let checksum = checksum?;
        if checksum.trim().is_empty() {
            return None;
        }
        Some(Self {
            algorithm,
            checksum: checksum.to_string(),
        })
    }

    /// Checks a file against the digest.
    pub fn verify(&self, path: &Path) -> Result<(), DownloadError> {
        let actual = self
            .algorithm
            .digest_file_to_string(path)
            .map_err(|error| DownloadError::Io(error.to_string()))?;

        if actual.eq_ignore_ascii_case(&self.checksum) {
            Ok(())
        } else {
            Err(DownloadError::Checksum {
                algorithm: self.algorithm,
                expected: self.checksum.clone(),
                actual,
            })
        }
    }
}

/// An answer of a server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResponseInfo {
    /// The address the answer came from, after the redirects.
    pub url: String,
    /// The code of the answer, such as `200` or `304`.
    pub status: u16,
    /// The headers of the answer, with the names as the server writes them.
    pub headers: BTreeMap<String, String>,
}

impl ResponseInfo {
    /// Returns the value of a header, without regard to the case of its name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Returns whether the server sent the file.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Returns whether the server told the launcher the file did not change.
    pub fn is_not_modified(&self) -> bool {
        self.status == 304
    }
}

/// The way the launcher gets the body of an address.
///
/// The implementation writes the body of the answer to the writer and returns
/// the headers; an answer that is not a success may hold no body. A failure of
/// the connection is reported as an error, and the task tries the address
/// again.
pub trait Fetcher {
    /// Asks a server for a file.
    ///
    /// The headers of the request tell the server whether the launcher already
    /// has the file.
    fn fetch(
        &self,
        url: &str,
        headers: &BTreeMap<String, String>,
        sink: &mut dyn Write,
    ) -> Result<ResponseInfo, String>;
}

/// The failure of a download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadError {
    /// The connection broke or the server could not be reached.
    Network {
        /// The address the launcher asked.
        url: String,
        /// The message the downloader wrote.
        message: String,
    },
    /// The server does not hold the file.
    NotFound {
        /// The address the file was asked from.
        url: String,
    },
    /// The server answered with a code the launcher does not handle.
    Status {
        /// The address the file was asked from.
        url: String,
        /// The code of the answer.
        status: u16,
    },
    /// The file is not the file the launcher asked for.
    Checksum {
        /// The algorithm of the digest.
        algorithm: Algorithm,
        /// The digest the version tells.
        expected: String,
        /// The digest of the file the launcher got.
        actual: String,
    },
    /// The archive of the file is broken.
    Archive {
        /// The reason the archive was refused.
        message: String,
    },
    /// The file could not be written.
    Io(String),
    /// The task was asked to stop.
    Cancelled,
    /// A downloader reported a failure in its own words.
    Failed(String),
}

impl DownloadError {
    /// Creates an error with a message that is shown to the user.
    pub fn new(message: impl Into<String>) -> Self {
        Self::Failed(message.into())
    }

    /// Returns whether the download may be tried again.
    ///
    /// A server that does not hold the file and a file that is not the file
    /// the version tells are not tried again, and neither is a cancellation;
    /// the other failures may be temporary.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::NotFound { .. } | Self::Cancelled | Self::Failed(_) => false,
            Self::Network { .. } | Self::Status { .. } | Self::Io(_) | Self::Archive { .. } => true,
            // The digest of a file that arrived whole does not match, which
            // means the mirror holds another file; the task still tries the
            // file again, because a body that was cut short is reported by the
            // fetcher as a network failure and not as a digest.
            Self::Checksum { .. } => true,
        }
    }
}

impl fmt::Display for DownloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network { url, message } => {
                write!(formatter, "cannot download {url}: {message}")
            }
            Self::NotFound { url } => write!(formatter, "{url} does not exist"),
            Self::Status { url, status } => {
                write!(formatter, "{url} answered with the code {status}")
            }
            Self::Checksum {
                algorithm,
                expected,
                actual,
            } => write!(
                formatter,
                "the digests of the file do not match ({}), expected: {expected}, actual: {actual}",
                algorithm.name()
            ),
            Self::Archive { message } => write!(formatter, "the archive is broken: {message}"),
            Self::Io(message) => formatter.write_str(message),
            Self::Cancelled => formatter.write_str("the download was cancelled"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DownloadError {}

/// Where a downloaded file came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadSource {
    /// The file was copied from the cache of the launcher.
    Cache,
    /// The file was sent by a server.
    Server,
}

/// A file the launcher downloaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedFile {
    /// The place of the file.
    pub path: PathBuf,
    /// Where the file came from.
    pub source: DownloadSource,
    /// The address the file was sent from, if a server sent it.
    pub url: Option<String>,
}

/// Downloads a file of the game to the place the version keeps it.
///
/// The task writes the file next to the place and moves it there when the file
/// passed every check, so a download that breaks does not leave a half-written
/// file where the game reads it. A download that names an integrity check
/// takes the file from the cache of the launcher when the cache holds it, and
/// a download that is cached in the cache of the servers asks the server
/// whether the file changed instead of asking for the file again.
#[derive(Debug, Clone)]
pub struct FileDownloadTask {
    /// The addresses of the file, in the order they are tried.
    candidates: DownloadCandidates,
    /// The place the file is written to.
    path: PathBuf,
    /// The check the file is held to.
    integrity_check: Option<IntegrityCheck>,
    /// Whether the file is kept in the cache of the launcher.
    caching: bool,
    /// Whether the file is kept in the cache of the servers.
    caching_remote: bool,
    /// Whether the file is an archive that is opened before it is accepted.
    check_archive: bool,
    /// The moment the launcher waits before it asks a mirror again.
    retry_delay: Duration,
}

impl FileDownloadTask {
    /// Creates a task that downloads a file.
    pub fn new(candidates: DownloadCandidates, path: impl Into<PathBuf>) -> Self {
        Self {
            candidates,
            path: path.into(),
            integrity_check: None,
            caching: false,
            caching_remote: false,
            check_archive: false,
            retry_delay: RETRY_DELAY,
        }
    }

    /// Creates a task that downloads a file from one address.
    pub fn of(url: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::new(DownloadCandidates::of(url), path)
    }

    /// Holds the file to a digest.
    pub fn with_integrity_check(mut self, check: IntegrityCheck) -> Self {
        self.integrity_check = Some(check);
        self
    }

    /// Holds the file to a digest, if the version tells one.
    pub fn with_optional_integrity_check(mut self, check: Option<IntegrityCheck>) -> Self {
        self.integrity_check = check;
        self
    }

    /// Keeps the file in the cache of the launcher.
    pub fn with_caching(mut self, caching: bool) -> Self {
        self.caching = caching;
        self
    }

    /// Asks the server whether the file changed, and keeps the file of the
    /// server in the cache.
    pub fn with_remote_caching(mut self, caching: bool) -> Self {
        self.caching_remote = caching;
        self
    }

    /// Opens the file as an archive before the file is accepted.
    ///
    /// The archives of the libraries are opened by the game, so an archive
    /// that cannot be opened stops the game from starting; the task refuses
    /// such a file instead of writing it.
    pub fn with_archive_check(mut self, check: bool) -> Self {
        self.check_archive = check;
        self
    }

    /// Sets the moment the launcher waits before it asks a mirror again.
    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Returns the place the file is written to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the addresses of the file.
    pub fn candidates(&self) -> &DownloadCandidates {
        &self.candidates
    }

    /// Returns the check the file is held to.
    pub fn integrity_check(&self) -> Option<&IntegrityCheck> {
        self.integrity_check.as_ref()
    }

    /// Returns the place the file is written while it is downloaded.
    ///
    /// The file is written next to the place of the file, so the move that
    /// puts it in place does not cross a file system.
    fn temporary_path(&self) -> PathBuf {
        let name = self
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.path.with_file_name(format!(".{name}.part"))
    }

    /// Returns the file from the cache of the launcher, if the cache holds it.
    fn cached_file(&self, cache: &CacheRepository) -> Option<DownloadedFile> {
        if !self.caching {
            return None;
        }

        let check = self.integrity_check.as_ref()?;
        let found =
            cache.check_existent_file(Some(&self.path), check.algorithm, Some(&check.checksum))?;

        // The file of the cache is copied to the place the game reads it from,
        // so a file the player already had in place is kept.
        if found != self.path && copy_file(&found, &self.path).is_err() {
            return None;
        }

        Some(DownloadedFile {
            path: self.path.clone(),
            source: DownloadSource::Cache,
            url: None,
        })
    }

    /// Copies the file of the cache of the servers to the place of the file,
    /// when the server told the launcher the file did not change.
    ///
    /// The file is copied instead of being moved, because the file of the
    /// cache is kept for the other files that use the same content.
    fn remote_cached_file(
        &self,
        cache: &CacheRepository,
        url: &str,
        response: &ResponseInfo,
    ) -> Option<DownloadedFile> {
        let url = if response.url.is_empty() {
            url
        } else {
            &response.url
        };
        let cached = cache.get_cached_remote_file(url, false).ok()?;

        // The file of the cache is copied to the place of the file, so the game
        // reads the file from the place it knows.
        copy_file(&cached, &self.path).ok()?;

        Some(DownloadedFile {
            path: self.path.clone(),
            source: DownloadSource::Cache,
            url: Some(url.to_string()),
        })
    }

    /// Downloads the file.
    ///
    /// The task tries the addresses of the file in the order they are given,
    /// and every address as often as the candidate tells. The file is checked
    /// before it is moved to its place, and the file of a server that did not
    /// change is taken from the cache.
    pub fn download(
        &self,
        fetcher: &dyn Fetcher,
        cache: Option<&mut CacheRepository>,
    ) -> Result<DownloadedFile, DownloadError> {
        let mut cache = cache;

        if let Some(repository) = cache.as_deref() {
            if let Some(file) = self.cached_file(repository) {
                return Ok(file);
            }
        }

        let mut failures: Vec<DownloadError> = Vec::new();

        for candidate in self.candidates.candidates() {
            let attempts = candidate.attempts().saturating_add(1);
            let mut attempt = 0;
            let mut use_remote_cache = self.caching_remote && cache.is_some();

            while attempt < attempts {
                attempt += 1;

                let headers = if use_remote_cache {
                    cache
                        .as_deref()
                        .map(|repository| repository.conditional_headers(&candidate.url))
                        .unwrap_or_default()
                } else {
                    BTreeMap::new()
                };

                match self.attempt(fetcher, &mut cache, &candidate.url, &headers) {
                    Ok(file) => return Ok(file),
                    Err(error) => {
                        // A server that answers with a short answer told the
                        // launcher the file did not change, but the cache does
                        // not hold the file; the file is asked for again
                        // without the headers of the cache.
                        if matches!(error, DownloadError::Failed(_)) && use_remote_cache {
                            use_remote_cache = false;
                            attempt -= 1;
                            continue;
                        }

                        if !error.is_retryable() {
                            failures.push(error);
                            break;
                        }

                        failures.push(error);
                        if attempt < attempts && !self.retry_delay.is_zero() {
                            std::thread::sleep(self.retry_delay);
                        }
                    }
                }
            }
        }

        // The failure the user sees is the last one, because it is the one of
        // the address the launcher gave up on. When the launcher tried more
        // than one address, the failures of the other addresses are named as
        // the reasons they were given up on.
        let last = match failures.pop() {
            Some(last) => last,
            None => return Err(DownloadError::Cancelled),
        };
        if self.candidates.len() == 1 {
            Err(last)
        } else {
            let mut message = last.to_string();
            for failure in failures {
                message.push_str("; ");
                message.push_str(&failure.to_string());
            }

            Err(DownloadError::Network {
                url: self.candidates.primary().url.clone(),
                message,
            })
        }
    }

    /// Tries an address once.
    fn attempt(
        &self,
        fetcher: &dyn Fetcher,
        cache: &mut Option<&mut CacheRepository>,
        url: &str,
        headers: &BTreeMap<String, String>,
    ) -> Result<DownloadedFile, DownloadError> {
        let temporary = self.temporary_path();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| DownloadError::Io(error.to_string()))?;
        }

        let mut sink = fs::File::create(&temporary)
            .map_err(|error| DownloadError::Io(format!("{}: {error}", temporary.display())))?;

        let response = match fetcher.fetch(url, headers, &mut sink) {
            Ok(response) => response,
            Err(message) => {
                let _ = fs::remove_file(&temporary);
                return Err(DownloadError::Network {
                    url: url.to_string(),
                    message,
                });
            }
        };
        drop(sink);

        if response.is_not_modified() {
            let _ = fs::remove_file(&temporary);
            return match cache.as_deref() {
                Some(repository) => self
                    .remote_cached_file(repository, url, &response)
                    .ok_or_else(|| {
                        DownloadError::new("the server told the launcher the file did not change")
                    }),
                None => Err(DownloadError::new(
                    "the server told the launcher the file did not change",
                )),
            };
        }

        if response.status == 404 || response.status == 410 {
            let _ = fs::remove_file(&temporary);
            return Err(DownloadError::NotFound {
                url: url.to_string(),
            });
        }
        if !response.is_success() {
            let _ = fs::remove_file(&temporary);
            return Err(DownloadError::Status {
                url: url.to_string(),
                status: response.status,
            });
        }

        // The checks are made before the file is moved into place, so a file
        // that is not the file the version tells never replaces a file of the
        // game.
        let checked = self.check(&temporary);
        if let Err(error) = checked {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }

        fs::rename(&temporary, &self.path).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            DownloadError::Io(format!("{}: {error}", self.path.display()))
        })?;

        if let Some(repository) = cache.as_deref_mut() {
            if let Some(check) = &self.integrity_check {
                if self.caching {
                    let _ = repository.try_cache_file(&self.path, check.algorithm, &check.checksum);
                }
            }
            if self.caching_remote {
                let _ = repository.cache_remote_file(url, &response.headers, &self.path);
            }
        }

        Ok(DownloadedFile {
            path: self.path.clone(),
            source: DownloadSource::Server,
            url: Some(url.to_string()),
        })
    }

    /// Checks a file the launcher downloaded.
    fn check(&self, path: &Path) -> Result<(), DownloadError> {
        if let Some(check) = &self.integrity_check {
            check.verify(path)?;
        }
        if self.check_archive {
            check_archive(path)?;
        }
        Ok(())
    }
}

/// Opens an archive of the game, so a broken archive is found before the game
/// starts.
///
/// The archives of the libraries and of the game itself are read by the game;
/// the launcher only checks that the directory of the archive can be read.
pub fn check_archive(path: &Path) -> Result<(), DownloadError> {
    extract_entries_names(path)
        .map(|_| ())
        .map_err(|error| DownloadError::Archive {
            message: error.to_string(),
        })
}

/// Returns the names of the files of an archive, which reads the directory of
/// the archive.
fn extract_entries_names(path: &Path) -> Result<Vec<String>, zip::ZipError> {
    let entries = zip::entries(path)?;
    Ok(entries.into_iter().map(|entry| entry.name).collect())
}

/// Verifies a file against a digest, returning the failure of the check.
pub fn verify_checksum(
    path: &Path,
    algorithm: Algorithm,
    expected: &str,
) -> Result<(), DownloadError> {
    IntegrityCheck {
        algorithm,
        checksum: expected.to_string(),
    }
    .verify(path)
}

/// Returns whether the digest is a digest of SHA-1, which the cache keeps its
/// files under.
pub fn is_sha1(hash: &str) -> bool {
    is_sha1_digest(hash)
}

/// Copies a file, so a check of the cache and the place of the game are kept
/// apart.
pub fn copy_verified(source: &Path, target: &Path) -> Result<(), DownloadError> {
    copy_file(source, target).map_err(|error| DownloadError::Io(error.to_string()))
}

/// Returns the moment a download started, for the callers that report the
/// speed of a download.
pub fn started_now() -> Instant {
    Instant::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::cache::SHA1;
    use crate::util::zip::ZipWriter;
    use std::cell::RefCell;
    use std::io::Cursor;

    /// The body a fake server sends.
    #[derive(Clone)]
    enum Answer {
        /// A body of the given bytes.
        Body(Vec<u8>),
        /// The status of an answer without a body.
        Status(u16),
        /// A failure of the connection.
        Failure(String),
    }

    /// The answers of a fake server for one address.
    struct ServerAnswers {
        /// The address the answers are for.
        url: String,
        /// The answers, in the order they are sent.
        answers: Vec<Answer>,
        /// The number of the answers that were sent.
        asked: RefCell<usize>,
    }

    /// A server that answers from the memory.
    struct FakeServer {
        /// The answers, by address.
        answers: RefCell<Vec<ServerAnswers>>,
        /// The headers the launcher sent with every request.
        requests: RefCell<Vec<(String, BTreeMap<String, String>)>>,
    }

    impl FakeServer {
        fn new(answers: Vec<(&str, Vec<Answer>)>) -> Self {
            Self {
                answers: RefCell::new(
                    answers
                        .into_iter()
                        .map(|(url, answers)| ServerAnswers {
                            url: url.to_string(),
                            answers,
                            asked: RefCell::new(0),
                        })
                        .collect(),
                ),
                requests: RefCell::new(Vec::new()),
            }
        }

        fn request_count(&self, url: &str) -> usize {
            self.answers
                .borrow()
                .iter()
                .find(|answers| answers.url == url)
                .map(|answers| *answers.asked.borrow())
                .unwrap_or_default()
        }
    }

    impl Fetcher for FakeServer {
        fn fetch(
            &self,
            url: &str,
            headers: &BTreeMap<String, String>,
            sink: &mut dyn Write,
        ) -> Result<ResponseInfo, String> {
            self.requests
                .borrow_mut()
                .push((url.to_string(), headers.clone()));

            let answers = self.answers.borrow();
            let answers = answers
                .iter()
                .find(|answers| answers.url == url)
                .ok_or_else(|| format!("no such address: {url}"))?;

            let index = *answers.asked.borrow();
            *answers.asked.borrow_mut() = index + 1;
            // The last answer is repeated, so a test that expects a failure
            // keeps failing when the launcher asks again.
            let answer = answers
                .answers
                .get(index)
                .or_else(|| answers.answers.last())
                .cloned()
                .ok_or_else(|| format!("no answer for {url}"))?;

            match answer {
                Answer::Body(body) => {
                    sink.write_all(&body).map_err(|error| error.to_string())?;
                    Ok(ResponseInfo {
                        url: url.to_string(),
                        status: 200,
                        headers: BTreeMap::from([(
                            "Content-Length".to_string(),
                            body.len().to_string(),
                        )]),
                    })
                }
                Answer::Status(status) => Ok(ResponseInfo {
                    url: url.to_string(),
                    status,
                    headers: BTreeMap::new(),
                }),
                Answer::Failure(message) => Err(message),
            }
        }
    }

    /// Returns a directory that is removed when the test ends.
    fn directory(name: &str) -> PathBuf {
        let path = std::env::temp_dir()
            .join("hmcl-core-download")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Returns the digest of the data.
    fn digest(data: &[u8]) -> String {
        Algorithm::Sha1.digest_to_string(data)
    }

    /// Returns the bytes of an archive that holds a file.
    fn archive() -> Vec<u8> {
        let path = std::env::temp_dir()
            .join("hmcl-core-download-archive.zip")
            .to_string_lossy()
            .into_owned();
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

    #[test]
    fn a_downloaded_file_is_written_to_its_place() {
        let base = directory("simple");
        let task = FileDownloadTask::of(
            "https://example.com/library.jar",
            base.join("libraries/library.jar"),
        );
        let server = FakeServer::new(vec![(
            "https://example.com/library.jar",
            vec![Answer::Body(b"the library".to_vec())],
        )]);

        let file = task
            .download(&server, None)
            .expect("the file should be downloaded");
        assert_eq!(file.source, DownloadSource::Server);
        assert_eq!(file.url.as_deref(), Some("https://example.com/library.jar"));
        assert_eq!(
            fs::read(base.join("libraries/library.jar")).expect("the file should be read"),
            b"the library"
        );
        // The file that is being written is not left behind.
        assert!(!base.join("libraries/.library.jar.part").exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_of_another_digest_is_refused() {
        let base = directory("checksum");
        let task = FileDownloadTask::of("https://example.com/file.jar", base.join("file.jar"))
            .with_integrity_check(IntegrityCheck::sha1(digest(b"the file")));
        let server = FakeServer::new(vec![(
            "https://example.com/file.jar",
            vec![Answer::Body(b"another file".to_vec())],
        )]);

        let error = task
            .download(&server, None)
            .expect_err("the file should be refused");
        assert!(error.to_string().contains("do not match"));
        // The file of the game is not written, and the half-downloaded file is
        // removed.
        assert!(!base.join("file.jar").exists());
        assert!(!base.join(".file.jar.part").exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mirror_that_fails_is_tried_again() {
        let base = directory("retry");
        let task = FileDownloadTask::of("https://mirror.example.com/file", base.join("file"))
            .with_retry_delay(Duration::ZERO);
        let server = FakeServer::new(vec![(
            "https://mirror.example.com/file",
            vec![
                Answer::Failure("the connection broke".to_string()),
                Answer::Failure("the connection broke".to_string()),
                Answer::Body(b"the file".to_vec()),
            ],
        )]);

        let file = task
            .download(&server, None)
            .expect("the file should be downloaded");
        assert_eq!(file.source, DownloadSource::Server);
        assert_eq!(server.request_count("https://mirror.example.com/file"), 3);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_next_mirror_is_tried() {
        let base = directory("mirrors");
        let candidates = DownloadCandidates::new(vec![
            DownloadCandidate::with_retry("https://first.example.com/file", 0),
            DownloadCandidate::with_retry("https://second.example.com/file", 0),
        ]);
        let task =
            FileDownloadTask::new(candidates, base.join("file")).with_retry_delay(Duration::ZERO);
        let server = FakeServer::new(vec![
            ("https://first.example.com/file", vec![Answer::Status(500)]),
            (
                "https://second.example.com/file",
                vec![Answer::Body(b"the file".to_vec())],
            ),
        ]);

        task.download(&server, None)
            .expect("the second mirror should answer");
        assert_eq!(server.request_count("https://first.example.com/file"), 1);
        assert_eq!(server.request_count("https://second.example.com/file"), 1);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_that_is_not_there_is_not_asked_for_again() {
        let base = directory("missing");
        let candidates = DownloadCandidates::new(vec![
            DownloadCandidate::new("https://first.example.com/file"),
            DownloadCandidate::new("https://second.example.com/file"),
        ]);
        let task =
            FileDownloadTask::new(candidates, base.join("file")).with_retry_delay(Duration::ZERO);
        let server = FakeServer::new(vec![
            ("https://first.example.com/file", vec![Answer::Status(404)]),
            (
                "https://second.example.com/file",
                vec![Answer::Body(b"the file".to_vec())],
            ),
        ]);

        task.download(&server, None)
            .expect("the second mirror should answer");
        assert_eq!(server.request_count("https://first.example.com/file"), 1);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn every_attempt_is_used_before_the_mirror_is_given_up_on() {
        let base = directory("attempts");
        let candidates = DownloadCandidates::new(vec![DownloadCandidate::with_retry(
            "https://mirror.example.com/file",
            2,
        )]);
        let task =
            FileDownloadTask::new(candidates, base.join("file")).with_retry_delay(Duration::ZERO);
        let server = FakeServer::new(vec![(
            "https://mirror.example.com/file",
            vec![Answer::Status(503)],
        )]);

        let error = task
            .download(&server, None)
            .expect_err("the download should fail");
        assert!(error.to_string().contains("503"));
        assert_eq!(server.request_count("https://mirror.example.com/file"), 3);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_file_of_the_cache_is_used() {
        let base = directory("cache");
        let mut cache = CacheRepository::new(&base);
        let data = b"the library";
        let hash = digest(data);

        let source = base.join("downloaded.jar");
        fs::write(&source, data).expect("the file should be written");
        cache
            .cache_file(&source, Algorithm::Sha1, &hash)
            .expect("the file should be cached");

        let task = FileDownloadTask::of(
            "https://example.com/library.jar",
            base.join("libraries/library.jar"),
        )
        .with_integrity_check(IntegrityCheck::sha1(&hash))
        .with_caching(true);

        let server = FakeServer::new(vec![]);
        let file = task
            .download(&server, Some(&mut cache))
            .expect("the file should come from the cache");
        assert_eq!(file.source, DownloadSource::Cache);
        assert_eq!(
            fs::read(base.join("libraries/library.jar")).expect("the file should be read"),
            data
        );
        assert_eq!(server.request_count("https://example.com/library.jar"), 0);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_downloaded_file_is_kept_in_the_cache() {
        let base = directory("store");
        let mut cache = CacheRepository::new(&base);
        let data = b"the library";
        let hash = digest(data);

        let task =
            FileDownloadTask::of("https://example.com/library.jar", base.join("library.jar"))
                .with_integrity_check(IntegrityCheck::sha1(&hash))
                .with_caching(true);
        let server = FakeServer::new(vec![(
            "https://example.com/library.jar",
            vec![Answer::Body(data.to_vec())],
        )]);

        task.download(&server, Some(&mut cache))
            .expect("the file should be downloaded");
        assert!(cache.file_exists(Algorithm::Sha1, &hash));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_server_did_not_change_is_taken_from_the_cache() {
        let base = directory("not-modified");
        let mut cache = CacheRepository::new(&base);
        let data = b"{\"versions\":[]}";
        let hash = digest(data);

        // The cache holds the file the server sent earlier.
        cache
            .cache_bytes(
                "https://example.com/version_manifest.json",
                &BTreeMap::from([("ETag".to_string(), "\"abc\"".to_string())]),
                data,
            )
            .expect("the file should be cached");

        let task = FileDownloadTask::of(
            "https://example.com/version_manifest.json",
            base.join("version_manifest.json"),
        )
        .with_remote_caching(true);
        let server = FakeServer::new(vec![(
            "https://example.com/version_manifest.json",
            vec![Answer::Status(304)],
        )]);

        let file = task
            .download(&server, Some(&mut cache))
            .expect("the file should come from the cache");
        assert_eq!(file.source, DownloadSource::Cache);
        // The file of the cache is copied to the place of the game, so the game
        // reads the file from the place it knows.
        assert_eq!(file.path, base.join("version_manifest.json"));
        assert_eq!(
            fs::read(base.join("version_manifest.json")).expect("the file should be read"),
            data
        );
        assert!(cache.file_exists(Algorithm::Sha1, &hash));
        // The launcher asked whether the file changed.
        let requests = server.requests.borrow();
        assert_eq!(
            requests[0].1.get("If-None-Match").map(String::as_str),
            Some("\"abc\"")
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_file_is_asked_for_again_when_the_cache_does_not_hold_it() {
        let base = directory("not-modified-missing");
        let mut cache = CacheRepository::new(&base);
        let data = b"{\"versions\":[]}";

        cache
            .cache_bytes(
                "https://example.com/version_manifest.json",
                &BTreeMap::from([("ETag".to_string(), "\"abc\"".to_string())]),
                data,
            )
            .expect("the file should be cached");
        // The file of the cache is removed, so the answer of the server cannot
        // be used.
        fs::remove_file(cache.file_of(SHA1, &digest(data))).expect("the file should be removed");
        let task = FileDownloadTask::of(
            "https://example.com/version_manifest.json",
            base.join("version_manifest.json"),
        )
        .with_remote_caching(true)
        .with_retry_delay(Duration::ZERO);
        let server = FakeServer::new(vec![(
            "https://example.com/version_manifest.json",
            vec![Answer::Status(304), Answer::Body(data.to_vec())],
        )]);

        let file = task
            .download(&server, Some(&mut cache))
            .expect("the file should be downloaded");
        assert_eq!(file.source, DownloadSource::Server);
        assert_eq!(
            fs::read(base.join("version_manifest.json")).expect("the file should be read"),
            data
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_broken_archive_is_refused() {
        let base = directory("archive");
        let task =
            FileDownloadTask::of("https://example.com/library.jar", base.join("library.jar"))
                .with_archive_check(true)
                .with_retry_delay(Duration::ZERO);
        let server = FakeServer::new(vec![(
            "https://example.com/library.jar",
            vec![Answer::Body(b"this is not an archive".to_vec())],
        )]);

        let error = task
            .download(&server, None)
            .expect_err("the archive should be refused");
        assert!(matches!(error, DownloadError::Archive { .. }));
        assert!(!base.join("library.jar").exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_archive_of_a_library_is_accepted() {
        let base = directory("archive-ok");
        let bytes = archive();
        let task =
            FileDownloadTask::of("https://example.com/library.jar", base.join("library.jar"))
                .with_archive_check(true)
                .with_integrity_check(IntegrityCheck::sha1(digest(&bytes)));
        let server = FakeServer::new(vec![(
            "https://example.com/library.jar",
            vec![Answer::Body(bytes.clone())],
        )]);

        task.download(&server, None)
            .expect("the archive should be accepted");
        assert_eq!(
            fs::read(base.join("library.jar")).expect("the file should be read"),
            bytes
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_addresses_of_a_file_are_needed() {
        let candidates =
            DownloadCandidates::of_urls(["https://example.com/a", "https://mirror.example.com/a"]);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates.primary().url, "https://example.com/a");
        assert!(!candidates.is_empty());
        assert_eq!(candidates.primary().attempts(), DEFAULT_RETRY);
        assert_eq!(
            DownloadCandidate::with_retry("https://example.com/a", 0).attempts(),
            0
        );
    }

    #[test]
    fn the_check_of_a_digest_is_read() {
        assert_eq!(IntegrityCheck::of(Algorithm::Sha1, None), None);
        assert_eq!(IntegrityCheck::of(Algorithm::Sha1, Some("")), None);
        assert_eq!(
            IntegrityCheck::of(Algorithm::Sha1, Some("abc")),
            Some(IntegrityCheck::sha1("abc"))
        );
    }

    #[test]
    fn the_answers_of_the_servers_are_read() {
        let response = ResponseInfo {
            url: "https://example.com/file".to_string(),
            status: 200,
            headers: BTreeMap::from([("ETag".to_string(), "\"abc\"".to_string())]),
        };
        assert!(response.is_success());
        assert!(!response.is_not_modified());
        assert_eq!(response.header("etag"), Some("\"abc\""));
        assert_eq!(response.header("other"), None);

        let response = ResponseInfo {
            status: 304,
            ..response
        };
        assert!(response.is_not_modified());
        assert!(!response.is_success());
    }

    #[test]
    fn the_failures_tell_whether_they_are_temporary() {
        assert!(DownloadError::Network {
            url: "https://example.com".to_string(),
            message: "broken".to_string(),
        }
        .is_retryable());
        assert!(!DownloadError::NotFound {
            url: "https://example.com".to_string(),
        }
        .is_retryable());
        assert!(!DownloadError::Cancelled.is_retryable());
        assert!(!DownloadError::new("the downloader said no").is_retryable());
    }

    #[test]
    fn a_check_of_a_file_is_made() {
        let base = directory("verify");
        let path = base.join("file");
        fs::write(&path, b"data").expect("the file should be written");

        assert_eq!(
            verify_checksum(&path, Algorithm::Sha1, &digest(b"data")),
            Ok(())
        );
        assert!(verify_checksum(&path, Algorithm::Sha1, &digest(b"other")).is_err());
        assert!(is_sha1(&digest(b"data")));
        assert!(!is_sha1("abc"));

        let mut cursor = Cursor::new(Vec::new());
        let _ = cursor.write_all(b"data");

        let _ = fs::remove_dir_all(&base);
    }
}
