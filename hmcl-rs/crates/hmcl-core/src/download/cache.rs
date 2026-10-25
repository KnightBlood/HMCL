//! The cache of the files the launcher downloads.
//!
//! The launcher downloads the same files many times: every version of the game
//! uses the same libraries and the same assets, and the launcher installs many
//! versions. The files are kept in a directory, named after their digests, so a
//! file is downloaded once and copied from the cache afterwards.
//!
//! The files of the servers are also kept, with the validators the servers send,
//! so a file the launcher already has is not downloaded again when the server
//! tells the launcher the file did not change.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::util::datetime::{parse_http_date, unix_millis};
use crate::util::digest::{is_sha1_digest, Algorithm};

/// The name of the algorithm the files of the cache are named after.
pub const SHA1: &str = "SHA-1";

/// The name of the directory the files of the cache are kept in.
pub const CACHE_DIRECTORY: &str = "cache";

/// The name of the file that holds the validators of the files of the servers.
pub const ETAG_INDEX_FILE: &str = "etag.json";

/// The name of the file that holds the libraries the launcher cached.
pub const LIBRARY_INDEX_FILE: &str = "index.json";

/// The entry of the cache that describes a file of a server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ETagItem {
    /// The address of the file, without its query and its fragment.
    pub url: String,
    /// The validator the server sent for the file.
    #[serde(default, rename = "eTag")]
    pub e_tag: Option<String>,
    /// The digest of the file, in the hexadecimal of SHA-1.
    pub hash: String,
    /// The moment the file was written, in milliseconds since the first of
    /// January 1970.
    #[serde(default, rename = "local")]
    pub local_last_modified: i64,
    /// The date the server last changed the file, as the header tells it.
    #[serde(default, rename = "remote")]
    pub remote_last_modified: Option<String>,
    /// The moment the file has to be asked for again, in milliseconds since
    /// the first of January 1970.
    #[serde(default)]
    pub expires: i64,
}

impl ETagItem {
    /// Returns the moment the server last changed the file.
    ///
    /// The date is written by the server; a date the launcher cannot read is
    /// reported as the first moment, so the entry is replaced by a newer one.
    pub fn remote_moment(&self) -> i64 {
        self.remote_last_modified
            .as_deref()
            .and_then(parse_http_date)
            .unwrap_or(i64::MIN)
    }

    /// Returns whether the entry is newer than another entry of the same file.
    ///
    /// The entries without a date the launcher can read come last, so the
    /// entry of a server that tells the date replaces the entry of a server
    /// that does not.
    pub fn is_newer_than(&self, other: &Self) -> bool {
        self.remote_moment() > other.remote_moment()
    }
}

/// The file that holds the entries of the cache.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ETagIndex {
    /// The entries of the files of the servers.
    #[serde(rename = "eTag", default)]
    pub entries: Vec<ETagItem>,
}

/// The cache of the files the launcher downloads.
///
/// The cache holds two kinds of files: the files named after their digest,
/// which are the files of the game, and the files of the servers, which are
/// described by [`ETagItem`]. The digest of a file of the cache is checked
/// before the file is used, so a file the player changed is downloaded again.
#[derive(Debug, Clone)]
pub struct CacheRepository {
    /// The directory the files of the game are kept in.
    common_directory: PathBuf,
    /// The directory the cache is kept in.
    cache_directory: PathBuf,
    /// The file that holds the entries of the files of the servers.
    index_file: PathBuf,
    /// The entries of the files of the servers, by their address.
    index: BTreeMap<String, ETagItem>,
}

impl CacheRepository {
    /// Creates a cache in a directory.
    ///
    /// The entries of the files of the servers are read at once; a file that
    /// cannot be read is ignored, so a broken index does not stop the launcher.
    pub fn new(common_directory: impl Into<PathBuf>) -> Self {
        let common_directory = common_directory.into();
        let cache_directory = common_directory.join(CACHE_DIRECTORY);
        let index_file = cache_directory.join(ETAG_INDEX_FILE);
        let index = read_index(&index_file);

        Self {
            common_directory,
            cache_directory,
            index_file,
            index,
        }
    }

    /// Returns the directory the files of the game are kept in.
    pub fn common_directory(&self) -> &Path {
        &self.common_directory
    }

    /// Returns the directory the cache is kept in.
    pub fn cache_directory(&self) -> &Path {
        &self.cache_directory
    }

    /// Returns the file that holds the entries of the files of the servers.
    pub fn index_file(&self) -> &Path {
        &self.index_file
    }

    /// Returns the file of a digest in the cache.
    ///
    /// The file is kept under the name of the algorithm of the digest, in a
    /// directory named after the first two characters of the digest, so a
    /// directory of the cache does not hold too many files.
    pub fn file_of(&self, algorithm: &str, hash: &str) -> PathBuf {
        let hash = hash.to_ascii_lowercase();
        let prefix = if hash.len() >= 2 {
            &hash[..2]
        } else {
            &hash[..]
        };
        self.cache_directory
            .join(algorithm)
            .join(prefix)
            .join(&hash)
    }

    /// Returns whether the cache holds the file of a digest.
    ///
    /// The digest of the file is read again, so a file that was changed or
    /// that was written by an interrupted download is not used.
    pub fn file_exists(&self, algorithm: Algorithm, hash: &str) -> bool {
        if !is_sha1_digest(hash) {
            return false;
        }

        let path = self.file_of(algorithm.name(), hash);
        if !path.is_file() {
            return false;
        }

        algorithm
            .digest_file_to_string(&path)
            .map(|digest| digest.eq_ignore_ascii_case(hash))
            .unwrap_or(false)
    }

    /// Copies a file of the cache to a place of the game directory.
    ///
    /// The failure of the copy is reported, so the caller downloads the file
    /// instead of using the cache.
    pub fn copy_from_cache(
        &self,
        algorithm: Algorithm,
        hash: &str,
        target: &Path,
    ) -> Result<(), Error> {
        let source = self.file_of(algorithm.name(), hash);
        copy_file(&source, target)
    }

    /// Writes a file to the cache.
    ///
    /// The file is written next to the file of the cache and moved into place,
    /// so a file that is being written is never used.
    pub fn cache_file(
        &self,
        path: &Path,
        algorithm: Algorithm,
        hash: &str,
    ) -> Result<PathBuf, Error> {
        check_hash(hash)?;

        let cache = self.file_of(algorithm.name(), hash);
        copy_file(path, &cache)?;
        Ok(cache)
    }

    /// Writes a file to the cache, unless the cache already holds it.
    ///
    /// The file of the cache that already exists is kept; the caller checked
    /// the files it caches, so the file of the cache is the same file.
    pub fn try_cache_file(
        &self,
        path: &Path,
        algorithm: Algorithm,
        hash: &str,
    ) -> Result<(), Error> {
        check_hash(hash)?;

        let cache = self.file_of(algorithm.name(), hash);
        if cache.is_file() {
            return Ok(());
        }

        copy_file(path, &cache)
    }

    /// Returns the file of a digest the launcher already has.
    ///
    /// The file of the cache is used when it is there and holds the digest.
    /// The file the caller names is used when the cache does not hold the file
    /// and the file the caller names holds the digest; the file is written to
    /// the cache as well. A file without a digest is used as it is, because
    /// the launcher has no way to tell whether it is the file it wants.
    pub fn check_existent_file(
        &self,
        original: Option<&Path>,
        algorithm: Algorithm,
        hash: Option<&str>,
    ) -> Option<PathBuf> {
        if let Some(hash) = hash {
            if self.file_exists(algorithm, hash) {
                return Some(self.file_of(algorithm.name(), hash));
            }
        }

        let original = original?;
        if !original.is_file() {
            return None;
        }

        match hash {
            Some(hash) => {
                let digest = algorithm.digest_file_to_string(original).ok()?;
                if !digest.eq_ignore_ascii_case(hash) {
                    return None;
                }

                let _ = self.cache_file(original, algorithm, hash);
                Some(self.file_of(algorithm.name(), hash))
            }
            // A file without a digest is used as it is, because the launcher
            // cannot tell whether the file of the cache is the same file.
            None => Some(original.to_path_buf()),
        }
    }

    /// Returns the entry of a file of a server.
    pub fn remote_entry(&self, url: &str) -> Option<&ETagItem> {
        self.index.get(&drop_query(url))
    }

    /// Returns the file of a server the cache holds, if the file is there and
    /// is not expired.
    ///
    /// The digest of the file is checked against the entry, so a file that was
    /// changed is refused.
    pub fn get_cached_remote_file(
        &self,
        url: &str,
        check_expires: bool,
    ) -> Result<PathBuf, CacheError> {
        let key = drop_query(url);
        let entry = self
            .index
            .get(&key)
            .ok_or_else(|| CacheError::Missing(key.clone()))?;

        if !self.file_exists(Algorithm::Sha1, &entry.hash) {
            return Err(CacheError::MissingFile(key));
        }
        if check_expires && unix_millis() > entry.expires {
            return Err(CacheError::Expired(entry.expires));
        }

        let file = self.file_of(SHA1, &entry.hash);
        let modified = file_modified_millis(&file);
        if modified != entry.local_last_modified {
            // The moment of the file changed, so the file may have been
            // replaced; the digest tells whether it is the file of the cache.
            let digest = Algorithm::Sha1
                .digest_file_to_string(&file)
                .map_err(|error| CacheError::Io(error.to_string()))?;
            if !digest.eq_ignore_ascii_case(&entry.hash) {
                return Err(CacheError::Modified(key));
            }
        }

        Ok(file)
    }

    /// Removes the entry of a file of a server, keeping the file of the cache.
    pub fn remove_remote_entry(&mut self, url: &str) {
        self.index.remove(&drop_query(url));
    }

    /// Returns the headers that ask the server whether the file changed.
    ///
    /// The server answers with a short answer instead of the file when the
    /// file did not change since the launcher read it.
    pub fn conditional_headers(&self, url: &str) -> BTreeMap<String, String> {
        let mut headers = BTreeMap::new();
        if let Some(entry) = self.index.get(&drop_query(url)) {
            if let Some(e_tag) = &entry.e_tag {
                headers.insert("If-None-Match".to_string(), e_tag.clone());
            }
        }
        headers
    }

    /// Writes a file of a server to the cache.
    ///
    /// The entry of the file is written only when the server sent a validator;
    /// the file of the cache is written under its digest, so the same file of
    /// the two servers is kept once.
    pub fn cache_remote_file(
        &mut self,
        url: &str,
        headers: &BTreeMap<String, String>,
        downloaded: &Path,
    ) -> Result<PathBuf, Error> {
        let hash = Algorithm::Sha1.digest_file_to_string(downloaded)?;
        let cached = self.cache_file(downloaded, Algorithm::Sha1, &hash)?;

        self.cache_entry(url, headers, &hash, &cached);
        Ok(cached)
    }

    /// Writes a text of a server to the cache.
    pub fn cache_text(
        &mut self,
        url: &str,
        headers: &BTreeMap<String, String>,
        text: &str,
    ) -> Result<PathBuf, Error> {
        self.cache_bytes(url, headers, text.as_bytes())
    }

    /// Writes the data of a server to the cache.
    pub fn cache_bytes(
        &mut self,
        url: &str,
        headers: &BTreeMap<String, String>,
        data: &[u8],
    ) -> Result<PathBuf, Error> {
        let hash = Algorithm::Sha1.digest_to_string(data);
        let cached = self.file_of(SHA1, &hash);

        if let Some(parent) = cached.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
        }
        let temporary = temporary_path(&cached);
        fs::write(&temporary, data).map_err(|error| Error::io(&temporary, error))?;
        fs::rename(&temporary, &cached).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            Error::io(&cached, error)
        })?;

        self.cache_entry(url, headers, &hash, &cached);
        Ok(cached)
    }

    /// Writes the entry of a file of a server, when the server sent a
    /// validator.
    fn cache_entry(
        &mut self,
        url: &str,
        headers: &BTreeMap<String, String>,
        hash: &str,
        cached: &Path,
    ) {
        let Some(e_tag) = header(headers, "etag").filter(|e_tag| !e_tag.trim().is_empty()) else {
            return;
        };

        let expires = expires_of(headers);
        // The server told the launcher not to keep the file, so no entry is
        // written and the file is downloaded again.
        let Some(expires) = expires else {
            return;
        };

        let key = drop_query(url);
        let entry = ETagItem {
            url: key.clone(),
            e_tag: Some(e_tag.to_string()),
            hash: hash.to_string(),
            local_last_modified: file_modified_millis(cached),
            remote_last_modified: header(headers, "last-modified").map(str::to_string),
            expires,
        };

        // The file of the entry that is replaced is removed when the new file
        // is another file, so the cache does not grow with the same file.
        if let Some(old) = self.index.get(&key) {
            if !old.hash.eq_ignore_ascii_case(hash) {
                let _ = fs::remove_file(self.file_of(SHA1, &old.hash));
            }
        }

        self.index.insert(key, entry);
        let _ = self.save_index();
    }

    /// Writes the entries of the files of the servers.
    ///
    /// The file is written next to the index and moved into place, so an
    /// interrupted write does not destroy the index.
    pub fn save_index(&self) -> Result<(), Error> {
        let parent = self.index_file.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;

        let index = ETagIndex {
            entries: self.index.values().cloned().collect(),
        };
        let content = serde_json::to_string(&index)
            .map_err(|error| Error::invalid_format(&self.index_file, error.to_string()))?;

        let temporary = temporary_path(&self.index_file);
        fs::write(&temporary, content.as_bytes()).map_err(|error| Error::io(&temporary, error))?;
        fs::rename(&temporary, &self.index_file).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            Error::io(&self.index_file, error)
        })
    }

    /// Returns the entries of the files of the servers.
    pub fn entries(&self) -> impl Iterator<Item = &ETagItem> {
        self.index.values()
    }
}

/// The failure of reading a file of the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// The cache holds no entry of the file.
    Missing(String),
    /// The cache holds the entry of the file, but not the file itself.
    MissingFile(String),
    /// The moment of the file of the cache passed.
    Expired(i64),
    /// The file of the cache is not the file the entry describes.
    Modified(String),
    /// The cache cannot be read or written.
    Io(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(url) => write!(formatter, "the cache holds no entry of {url}"),
            Self::MissingFile(url) => write!(formatter, "the cache holds no file of {url}"),
            Self::Expired(expires) => {
                write!(formatter, "the file of the cache expired at {expires}")
            }
            Self::Modified(url) => write!(formatter, "the file of the cache of {url} was changed"),
            Self::Io(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for CacheError {}

/// Returns the address without its query and its fragment.
///
/// The files of the servers are the same file whatever query the launcher
/// sends, so the entries of the cache are kept by the address of the file.
pub fn drop_query(url: &str) -> String {
    let without_fragment = match url.find('#') {
        Some(index) => &url[..index],
        None => url,
    };
    match without_fragment.find('?') {
        Some(index) => without_fragment[..index].to_string(),
        None => without_fragment.to_string(),
    }
}

/// Reads the entries of the files of the servers.
///
/// A file that cannot be read yields no entries, so a broken index does not
/// stop the launcher; the files of the cache are still there.
fn read_index(path: &Path) -> BTreeMap<String, ETagItem> {
    let Ok(content) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(index) = serde_json::from_str::<ETagIndex>(&content) else {
        return BTreeMap::new();
    };

    let mut entries: BTreeMap<String, ETagItem> = BTreeMap::new();
    for entry in index.entries {
        // The entries of the same file are kept once, and the newer of the two
        // is kept, like the launcher reads the index the other servers wrote.
        match entries.get(&entry.url) {
            Some(current) if !entry.is_newer_than(current) => {}
            _ => {
                entries.insert(entry.url.clone(), entry);
            }
        }
    }
    entries
}

/// Returns the moment a file was written, in milliseconds since the first of
/// January 1970.
fn file_modified_millis(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|moment| moment.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

/// Returns the moment the file of a server has to be asked for again.
///
/// The headers of the server tell the moment in two ways: the `Cache-Control`
/// header tells how many seconds the file may be kept, and the `Expires`
/// header tells the moment itself. A server that tells neither keeps its file
/// until the file is asked for again.
fn expires_of(headers: &BTreeMap<String, String>) -> Option<i64> {
    if let Some(cache_control) = header(headers, "cache-control") {
        if cache_control.to_ascii_lowercase().contains("no-store") {
            return None;
        }

        if let Some(seconds) = max_age(cache_control) {
            return Some(unix_millis() + seconds.saturating_mul(1_000));
        }
    }

    if let Some(expires) = header(headers, "expires") {
        if let Some(moment) = parse_http_date(expires) {
            return Some(moment.saturating_mul(1_000));
        }
    }

    // The server tells no moment, so the file of the cache is kept until the
    // launcher asks the server whether the file changed.
    Some(i64::MAX)
}

/// Returns the number of the seconds of the `max-age` part of a
/// `Cache-Control` header.
fn max_age(cache_control: &str) -> Option<i64> {
    let lower = cache_control.to_ascii_lowercase();
    let mut rest = lower.as_str();

    while let Some(index) = rest.find("max-age=") {
        // The `s-maxage` part names the same number for the servers of the
        // proxies; the `max-age` part is the one of the file.
        let after = &rest[index + "max-age=".len()..];
        let digits: String = after
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        if let Ok(seconds) = digits.parse::<i64>() {
            return Some(seconds);
        }
        rest = after;
    }

    None
}

/// Returns the value of a header, without regard to the case of its name.
fn header<'a>(headers: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// Returns the path of the file that is written before the target replaces
/// it.
fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.tmp"))
}

/// Copies a file, creating the directories of the target.
pub fn copy_file(source: &Path, target: &Path) -> Result<(), Error> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
    }

    let temporary = temporary_path(target);
    fs::copy(source, &temporary).map_err(|error| Error::io(&temporary, error))?;
    fs::rename(&temporary, target).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        Error::io(target, error)
    })
}

/// Returns whether the digest of the text is the digest of the launcher.
fn check_hash(hash: &str) -> Result<(), Error> {
    if is_sha1_digest(hash) {
        Ok(())
    } else {
        Err(Error::invalid_format(
            Path::new(""),
            format!("the digest '{hash}' is not a digest of SHA-1"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Returns a directory that is removed when the test ends.
    fn directory(name: &str) -> PathBuf {
        let path = std::env::temp_dir()
            .join("hmcl-core-cache")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Returns the headers a server sends with a file that may be kept.
    fn headers(e_tag: &str, last_modified: &str) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("ETag".to_string(), e_tag.to_string()),
            ("Last-Modified".to_string(), last_modified.to_string()),
            (
                "Cache-Control".to_string(),
                "public, max-age=3600".to_string(),
            ),
        ])
    }

    /// The digest of the data the tests write.
    fn digest(data: &[u8]) -> String {
        Algorithm::Sha1.digest_to_string(data)
    }

    #[test]
    fn the_files_of_the_cache_are_named_after_their_digests() {
        let base = directory("naming");
        let cache = CacheRepository::new(&base);

        let hash = digest(b"data");
        let path = cache.file_of(SHA1, &hash);
        assert_eq!(path, base.join("cache/SHA-1").join(&hash[..2]).join(&hash));
        // The digest is read without regard to its case.
        assert_eq!(cache.file_of(SHA1, &hash.to_uppercase()), path);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_is_cached_and_copied_again() {
        let base = directory("copy");
        let cache = CacheRepository::new(&base);
        let hash = digest(b"the file of the game");

        let source = base.join("downloaded.jar");
        fs::write(&source, b"the file of the game").expect("the file should be written");

        let cached = cache
            .cache_file(&source, Algorithm::Sha1, &hash)
            .expect("the file should be cached");
        assert!(cached.is_file());
        assert!(cache.file_exists(Algorithm::Sha1, &hash));

        let target = base.join("versions/1.20/1.20.jar");
        cache
            .copy_from_cache(Algorithm::Sha1, &hash, &target)
            .expect("the file should be copied");
        assert_eq!(
            fs::read(&target).expect("the file should be read"),
            b"the file of the game"
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_that_was_changed_is_not_used() {
        let base = directory("changed");
        let cache = CacheRepository::new(&base);
        let hash = digest(b"the file of the game");

        let file = cache.file_of(SHA1, &hash);
        fs::create_dir_all(file.parent().expect("the file should have a parent"))
            .expect("the directory should be created");
        fs::write(&file, b"another file").expect("the file should be written");

        assert!(!cache.file_exists(Algorithm::Sha1, &hash));
        assert_eq!(
            cache.check_existent_file(None, Algorithm::Sha1, Some(&hash)),
            None
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_file_the_launcher_has_is_cached() {
        let base = directory("existent");
        let cache = CacheRepository::new(&base);
        let hash = digest(b"the file of the game");

        let original = base.join("libraries/library.jar");
        fs::create_dir_all(original.parent().expect("the file should have a parent"))
            .expect("the directory should be created");
        fs::write(&original, b"the file of the game").expect("the file should be written");

        let found = cache
            .check_existent_file(Some(&original), Algorithm::Sha1, Some(&hash))
            .expect("the file should be found");
        assert_eq!(found, cache.file_of(SHA1, &hash));
        assert!(cache.file_exists(Algorithm::Sha1, &hash));

        // A file of another digest is refused.
        let wrong = digest(b"another file");
        assert_eq!(
            cache.check_existent_file(Some(&original), Algorithm::Sha1, Some(&wrong)),
            None
        );

        // A file without a digest is used as it is.
        assert_eq!(
            cache.check_existent_file(Some(&original), Algorithm::Sha1, None),
            Some(original)
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_entries_of_the_files_of_the_servers_are_written_and_read_again() {
        let base = directory("entries");
        let mut cache = CacheRepository::new(&base);

        let downloaded = base.join("version_manifest.json");
        fs::write(&downloaded, b"{\"versions\":[]}").expect("the file should be written");

        let url = "https://piston-meta.mojang.com/mc/game/version_manifest.json?token=1";
        let cached = cache
            .cache_remote_file(
                url,
                &headers("\"abc\"", "Wed, 21 Oct 2015 07:28:00 GMT"),
                &downloaded,
            )
            .expect("the file should be cached");
        assert!(cached.is_file());

        // The entry is kept by the address without its query.
        let entry = cache
            .remote_entry(url)
            .expect("the entry should be there")
            .clone();
        assert_eq!(
            entry.url,
            "https://piston-meta.mojang.com/mc/game/version_manifest.json"
        );
        assert_eq!(entry.e_tag.as_deref(), Some("\"abc\""));
        assert_eq!(
            entry.remote_last_modified.as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
        assert_eq!(
            entry.remote_moment(),
            parse_http_date("Wed, 21 Oct 2015 07:28:00 GMT").expect("the date should be read")
        );

        // The file of the server is read again.
        let found = cache
            .get_cached_remote_file(url, false)
            .expect("the file should be found");
        assert_eq!(found, cached);

        // The entries are written to the index, which is read by a new cache.
        let index = fs::read_to_string(cache.index_file()).expect("the index should be read");
        assert!(index.contains("piston-meta.mojang.com"));
        let other = CacheRepository::new(&base);
        assert_eq!(
            other.remote_entry(url).map(|entry| &entry.hash),
            Some(&entry.hash)
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_without_a_validator_is_not_kept() {
        let base = directory("no-validator");
        let mut cache = CacheRepository::new(&base);

        let downloaded = base.join("file.json");
        fs::write(&downloaded, b"{}").expect("the file should be written");

        // Without an ETag the entry is not written, because the launcher has
        // no way to ask the server whether the file changed.
        cache
            .cache_remote_file(
                "https://example.com/file.json",
                &BTreeMap::new(),
                &downloaded,
            )
            .expect("the file should be cached");
        assert_eq!(
            cache.get_cached_remote_file("https://example.com/file.json", false),
            Err(CacheError::Missing(
                "https://example.com/file.json".to_string()
            ))
        );

        // The file of the cache is there, though.
        assert!(cache.file_exists(Algorithm::Sha1, &digest(b"{}")));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_server_forbids_to_keep_is_not_kept() {
        let base = directory("no-store");
        let mut cache = CacheRepository::new(&base);

        let downloaded = base.join("file.json");
        fs::write(&downloaded, b"{}").expect("the file should be written");

        let mut headers = headers("\"abc\"", "Wed, 21 Oct 2015 07:28:00 GMT");
        headers.insert("Cache-Control".to_string(), "no-store".to_string());
        cache
            .cache_remote_file("https://example.com/file.json", &headers, &downloaded)
            .expect("the file should be cached");

        assert!(cache
            .remote_entry("https://example.com/file.json")
            .is_none());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_moment_of_the_entries_of_the_servers_is_read() {
        let base = directory("expires");
        let mut cache = CacheRepository::new(&base);

        let downloaded = base.join("file.json");
        fs::write(&downloaded, b"{}").expect("the file should be written");

        let headers = headers("\"abc\"", "Wed, 21 Oct 2015 07:28:00 GMT");
        cache
            .cache_remote_file("https://example.com/file.json", &headers, &downloaded)
            .expect("the file should be cached");

        let entry = cache
            .remote_entry("https://example.com/file.json")
            .expect("the entry should be there");
        // The file may be kept for an hour, so the moment is in the future.
        assert!(entry.expires > unix_millis());

        // The file that is expired is refused but still there.
        assert!(cache
            .get_cached_remote_file("https://example.com/file.json", false)
            .is_ok());

        // The headers ask the server whether the file changed.
        let conditional = cache.conditional_headers("https://example.com/file.json");
        assert_eq!(
            conditional.get("If-None-Match").map(String::as_str),
            Some("\"abc\"")
        );
        assert!(cache
            .conditional_headers("https://example.com/other.json")
            .is_empty());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_entry_of_a_changed_file_is_replaced() {
        let base = directory("replaced");
        let mut cache = CacheRepository::new(&base);

        let first = base.join("first.json");
        fs::write(&first, b"first").expect("the file should be written");
        cache
            .cache_remote_file(
                "https://example.com/file.json",
                &headers("\"one\"", "Wed, 21 Oct 2015 07:28:00 GMT"),
                &first,
            )
            .expect("the file should be cached");
        let first_hash = digest(b"first");
        assert!(cache.file_exists(Algorithm::Sha1, &first_hash));

        let second = base.join("second.json");
        fs::write(&second, b"second").expect("the file should be written");
        cache
            .cache_remote_file(
                "https://example.com/file.json",
                &headers("\"two\"", "Thu, 22 Oct 2015 07:28:00 GMT"),
                &second,
            )
            .expect("the file should be cached");

        // The file of the entry that was replaced is removed from the cache.
        assert!(!cache.file_exists(Algorithm::Sha1, &first_hash));
        assert!(cache.file_exists(Algorithm::Sha1, &digest(b"second")));
        assert_eq!(
            cache
                .remote_entry("https://example.com/file.json")
                .and_then(|entry| entry.e_tag.as_deref()),
            Some("\"two\"")
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_data_of_a_server_is_cached() {
        let base = directory("bytes");
        let mut cache = CacheRepository::new(&base);

        let cached = cache
            .cache_text(
                "https://example.com/index.json",
                &headers("\"abc\"", "Wed, 21 Oct 2015 07:28:00 GMT"),
                "{\"objects\":{}}",
            )
            .expect("the text should be cached");
        assert!(cached.is_file());
        assert_eq!(
            fs::read_to_string(&cached).expect("the file should be read"),
            "{\"objects\":{}}"
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_of_another_digest_is_refused() {
        let base = directory("digest");
        let cache = CacheRepository::new(&base);
        let source = base.join("file");
        fs::write(&source, b"data").expect("the file should be written");

        assert!(cache.cache_file(&source, Algorithm::Sha1, "abc").is_err());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_query_is_dropped_from_the_address() {
        assert_eq!(
            drop_query("https://example.com/a.json"),
            "https://example.com/a.json"
        );
        assert_eq!(
            drop_query("https://example.com/a.json?token=1&x=2"),
            "https://example.com/a.json"
        );
        assert_eq!(
            drop_query("https://example.com/a.json#part"),
            "https://example.com/a.json"
        );
        assert_eq!(
            drop_query("https://example.com/a.json?token=1#part"),
            "https://example.com/a.json"
        );
    }

    #[test]
    fn the_seconds_of_the_cache_control_are_read() {
        assert_eq!(max_age("public, max-age=3600"), Some(3_600));
        assert_eq!(max_age("max-age=60, public"), Some(60));
        assert_eq!(max_age("s-maxage=30, max-age=90"), Some(90));
        assert_eq!(max_age("no-cache"), None);
        assert_eq!(max_age("max-age="), None);

        let mut headers = BTreeMap::new();
        headers.insert("cache-control".to_string(), "max-age=10".to_string());
        let expires = expires_of(&headers).expect("the moment should be read");
        assert!(expires > unix_millis());
        assert!(expires < unix_millis() + 60_000);

        let mut headers = BTreeMap::new();
        headers.insert(
            "Expires".to_string(),
            "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
        );
        assert_eq!(
            expires_of(&headers),
            parse_http_date("Wed, 21 Oct 2015 07:28:00 GMT").map(|seconds| seconds * 1_000)
        );

        // A server that tells no moment keeps its file.
        assert_eq!(expires_of(&BTreeMap::new()), Some(i64::MAX));
    }
}
