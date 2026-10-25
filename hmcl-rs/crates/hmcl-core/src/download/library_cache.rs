//! The cache of the libraries of the game.
//!
//! The libraries of a mod loader are not published in the repository of the
//! game: Forge and the other loaders ship their libraries with their installer,
//! and the same library is installed again for every version of the game the
//! player makes. The libraries are kept in the cache by name, so the launcher
//! copies them instead of asking the player for the installer again.
//!
//! The libraries are also kept by digest, like the other files of the game, so a
//! library whose digest is known is found without reading the index.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::download::cache::{CacheRepository, LIBRARY_INDEX_FILE};
use crate::download::game::Library;
use crate::error::Error;
use crate::util::digest::{is_sha1_digest, Algorithm};
use crate::util::zip;

/// The kind of a library of the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LibraryKind {
    /// The library is published by the game or by a repository of mods, and the
    /// digest the version tells is the digest of the library.
    Jar,
    /// The library is shipped inside the installer of a mod loader, so the
    /// digest the version tells may be the digest of the installer instead.
    Forge,
}

impl LibraryKind {
    /// Returns the name the index file writes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jar => "jar",
            Self::Forge => "forge",
        }
    }

    /// Reads the name the index file writes.
    ///
    /// A name the launcher does not know is read as [`Self::Jar`], because the
    /// library is still the file of the cache.
    pub fn parse(name: &str) -> Self {
        if name.eq_ignore_ascii_case("forge") {
            Self::Forge
        } else {
            Self::Jar
        }
    }
}

/// A library the cache holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryIndex {
    /// The name of the library, as the version of the game writes it.
    pub name: String,
    /// The digest of the file of the library, in the hexadecimal of SHA-1.
    pub hash: String,
    /// The kind of the library.
    #[serde(rename = "type", default = "default_kind")]
    pub kind: LibraryKind,
}

/// Returns the kind of a library the index does not name.
fn default_kind() -> LibraryKind {
    LibraryKind::Jar
}

/// The file that holds the libraries of the cache.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryIndexFile {
    /// The libraries the cache holds.
    #[serde(default)]
    pub libraries: Vec<LibraryIndex>,
}

/// The cache the launcher of this crate uses.
///
/// The cache holds the files of the game named after their digests and the
/// libraries of the mod loaders named after the name of the library. The index
/// of the libraries is read at once and written whenever a library is added, so
/// the launcher reads the libraries of the previous sessions as well.
#[derive(Debug, Clone)]
pub struct DefaultCacheRepository {
    /// The cache of the files named after their digests.
    cache: CacheRepository,
    /// The directory the libraries of the previous launchers are kept in.
    libraries_directory: PathBuf,
    /// The file that holds the libraries of the cache.
    index_file: PathBuf,
    /// The libraries of the cache, by the name of the library.
    libraries: BTreeMap<String, Vec<LibraryIndex>>,
}

impl DefaultCacheRepository {
    /// Creates a cache in a directory.
    ///
    /// A file of the index that cannot be read is ignored, so a broken index
    /// does not stop the launcher; the files of the cache are still there.
    pub fn new(common_directory: impl Into<PathBuf>) -> Self {
        let common_directory = common_directory.into();
        let cache = CacheRepository::new(&common_directory);
        let libraries_directory = common_directory.join("libraries");
        let index_file = cache.cache_directory().join(LIBRARY_INDEX_FILE);
        let libraries = read_index(&index_file);

        Self {
            cache,
            libraries_directory,
            index_file,
            libraries,
        }
    }

    /// Returns the cache of the files named after their digests.
    pub fn cache(&self) -> &CacheRepository {
        &self.cache
    }

    /// Returns the cache of the files named after their digests, for writing
    /// the files of the servers.
    pub fn cache_mut(&mut self) -> &mut CacheRepository {
        &mut self.cache
    }

    /// Returns the file that holds the libraries of the cache.
    pub fn index_file(&self) -> &Path {
        &self.index_file
    }

    /// Returns the libraries of the cache.
    pub fn entries(&self) -> impl Iterator<Item = &LibraryIndex> {
        self.libraries.values().flatten()
    }

    /// Returns the file of a library, if the cache holds it.
    ///
    /// The file of the cache is used when the digest the version tells is in
    /// the cache. The libraries of the mod loaders are found by their name, and
    /// the digest the version tells is checked against the checksums of the
    /// library, because the digest of a library of Forge is the digest of the
    /// installer that carries it.
    ///
    /// A library the previous launchers kept in the shared directory of the
    /// libraries is moved into the cache as well, so the library is found again
    /// when the launcher is started.
    pub fn library_file(&mut self, library: &Library) -> Option<PathBuf> {
        let sha1 = library_sha1(library);

        if let Some(sha1) = &sha1 {
            if self.cache.file_exists(Algorithm::Sha1, sha1) {
                return Some(self.cache.file_of(Algorithm::Sha1.name(), sha1));
            }
        }

        let name = library.name.as_str();
        let known = self.libraries.get(name).cloned().unwrap_or_default();
        for entry in known {
            if !self.cache.file_exists(Algorithm::Sha1, &entry.hash) {
                continue;
            }

            let file = self.cache.file_of(Algorithm::Sha1.name(), &entry.hash);
            if entry.kind == LibraryKind::Forge
                && !checksum_valid(&file, library.checksums.as_deref())
            {
                continue;
            }

            return Some(file);
        }

        // The libraries of the previous launchers were kept in the shared
        // directory of the libraries.
        let path = library_path_of(library);
        let jar = self.libraries_directory.join(path);
        if !jar.is_file() {
            return None;
        }

        let valid = match &sha1 {
            Some(sha1) => Algorithm::Sha1
                .digest_file_to_string(&jar)
                .map(|digest| digest.eq_ignore_ascii_case(sha1))
                .unwrap_or(false),
            None => match library.checksums.as_deref() {
                Some(checksums) if !checksums.is_empty() => checksum_valid(&jar, Some(checksums)),
                // A library without a digest is used as it is, because the
                // launcher cannot tell whether the file of the cache is the
                // same file.
                _ => true,
            },
        };
        if !valid {
            return None;
        }

        let forge = library.checksums.as_deref().is_some_and(|checksums| {
            sha1.as_deref()
                .is_none_or(|sha1| !checksums.iter().any(|sum| sum.eq_ignore_ascii_case(sha1)))
        });
        let cached = self.cache_library(library, &jar, forge).ok()?;
        Some(cached)
    }

    /// Writes a library to the cache.
    ///
    /// The library is written under its digest; the name of the library is
    /// written to the index as well, so a library whose digest the version does
    /// not tell is found again. The library must be checked by the caller.
    pub fn cache_library(
        &mut self,
        library: &Library,
        jar: &Path,
        forge: bool,
    ) -> Result<PathBuf, Error> {
        let hash = match library_sha1(library) {
            Some(hash) if is_sha1_digest(&hash) => hash.to_ascii_lowercase(),
            _ => Algorithm::Sha1.digest_file_to_string(jar)?,
        };

        let cached = self.cache.cache_file(jar, Algorithm::Sha1, &hash)?;

        let entry = LibraryIndex {
            name: library.name.clone(),
            hash,
            kind: if forge {
                LibraryKind::Forge
            } else {
                LibraryKind::Jar
            },
        };
        let entries = self.libraries.entry(library.name.clone()).or_default();
        if !entries.contains(&entry) {
            entries.push(entry);
        }
        let _ = self.save_index();

        Ok(cached)
    }

    /// Writes a library to the cache, if the file is the library the version
    /// tells.
    ///
    /// The library is not written when the launcher cannot tell whether the
    /// file is the library, so a file the player changed is not kept.
    pub fn try_cache_library(&mut self, library: &Library, jar: &Path) -> Result<bool, Error> {
        if self
            .libraries
            .get(&library.name)
            .is_some_and(|entries| !entries.is_empty())
        {
            return Ok(false);
        }

        let valid = match library_sha1(library) {
            Some(sha1) => Algorithm::Sha1
                .digest_file_to_string(jar)?
                .eq_ignore_ascii_case(&sha1),
            None => match library.checksums.as_deref() {
                Some(checksums) if !checksums.is_empty() => checksum_valid(jar, Some(checksums)),
                // A library that tells neither a digest nor the checksums of
                // its files is not kept, because the launcher cannot tell
                // whether the file is the library.
                _ => false,
            },
        };

        if !valid {
            return Ok(false);
        }

        // The digest of a library of Forge is the digest of the installer, so
        // the checksums of the files of the library are the ones that tell
        // whether the file is the library.
        let forge = library_sha1(library).is_none() && library.checksums.is_some();
        self.cache_library(library, jar, forge)?;
        Ok(true)
    }

    /// Writes the index of the libraries.
    ///
    /// The file is written next to the index and moved into place, so an
    /// interrupted write does not destroy the index.
    pub fn save_index(&self) -> Result<(), Error> {
        let parent = self.index_file.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;

        let index = LibraryIndexFile {
            libraries: self.entries().cloned().collect(),
        };
        let content = serde_json::to_string(&index)
            .map_err(|error| Error::invalid_format(&self.index_file, error.to_string()))?;

        let temporary = self.index_file.with_extension("json.part");
        fs::write(&temporary, content.as_bytes()).map_err(|error| Error::io(&temporary, error))?;
        fs::rename(&temporary, &self.index_file).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            Error::io(&self.index_file, error)
        })
    }
}

/// Reads the index of the libraries.
///
/// A file that cannot be read yields no libraries, so a broken index does not
/// stop the launcher; the files of the cache are still there.
fn read_index(path: &Path) -> BTreeMap<String, Vec<LibraryIndex>> {
    let Ok(content) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(index) = serde_json::from_str::<LibraryIndexFile>(&content) else {
        return BTreeMap::new();
    };

    let mut libraries: BTreeMap<String, Vec<LibraryIndex>> = BTreeMap::new();
    for entry in index.libraries {
        let entries = libraries.entry(entry.name.clone()).or_default();
        if !entries.contains(&entry) {
            entries.push(entry);
        }
    }
    libraries
}

/// Returns the path of a library in the directory of the libraries.
///
/// The versions of the game tell the path of the files of their libraries; a
/// library that tells none is kept where the repositories of Java keep it,
/// which is what its coordinates name.
pub fn library_path_of(library: &Library) -> String {
    if let Some(path) = library
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.artifact.as_ref())
        .and_then(|artifact| artifact.path.clone())
    {
        return path;
    }

    library
        .artifact()
        .ok()
        .map(|artifact| artifact.path())
        .unwrap_or_else(|| library.name.replace(':', "/"))
}

/// Returns the digest of a library the version tells.
fn library_sha1(library: &Library) -> Option<String> {
    library
        .downloads
        .as_ref()?
        .artifact
        .as_ref()?
        .sha1()
        .map(str::to_string)
}

/// Returns whether a file of a library is the library the versions of the game
/// tell.
///
/// The library of a mod loader is a jar that holds the files of the library and
/// a manifest of their digests; the digest of the jar the version tells is the
/// digest of the installer that carries the jar, so the file is checked file by
/// file.
pub fn checksum_valid(jar: &Path, checksums: Option<&[String]>) -> bool {
    let checksums = match checksums {
        Some(checksums) if !checksums.is_empty() => checksums,
        // A library without the checksums of its files is the library the
        // version tells.
        _ => return true,
    };

    let Ok(digest) = Algorithm::Sha1.digest_file_to_string(jar) else {
        return false;
    };
    if checksums
        .iter()
        .any(|sum| sum.eq_ignore_ascii_case(&digest))
    {
        return true;
    }

    // The files of the library are checked file by file when the archive holds
    // the manifest of their digests.
    if !jar.to_string_lossy().to_lowercase().ends_with(".jar") {
        return false;
    }

    manifest_is_valid(jar, checksums).unwrap_or(false)
}

/// Returns whether the files of an archive are the files the manifest tells.
fn manifest_is_valid(jar: &Path, checksums: &[String]) -> Result<bool, zip::ZipError> {
    let entries = zip::entries(jar)?;
    let mut digests: BTreeMap<String, String> = BTreeMap::new();
    let mut manifest: Option<Vec<String>> = None;

    for entry in &entries {
        if entry.directory {
            continue;
        }
        let Some(data) = zip::read_entry(jar, &entry.name)? else {
            continue;
        };
        if entry.name == "checksums.sha1" {
            manifest = Some(
                String::from_utf8_lossy(&data)
                    .split('\n')
                    .map(str::to_string)
                    .collect(),
            );
        }
        digests.insert(entry.name.clone(), Algorithm::Sha1.digest_to_string(&data));
    }

    let Some(manifest) = manifest else {
        return Ok(false);
    };
    let Some(manifest_digest) = digests.get("checksums.sha1") else {
        return Ok(false);
    };
    if !checksums
        .iter()
        .any(|sum| sum.eq_ignore_ascii_case(manifest_digest))
    {
        return Ok(false);
    }

    // Every line of the manifest names the digest of a file and the file.
    for line in manifest {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((expected, name)) = line.split_once(' ') else {
            continue;
        };
        match digests.get(name) {
            Some(actual) if actual.eq_ignore_ascii_case(expected) => {}
            _ => return Ok(false),
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{DownloadInfo, LibraryDownloads};
    use crate::util::zip::ZipWriter;

    /// Returns a directory that is removed when the test ends.
    fn directory(name: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let index = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let path = std::env::temp_dir()
            .join("hmcl-core-library-cache")
            .join(format!("{name}-{}-{index}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the directory should be created");
        path
    }

    /// Returns a library of the version, with the digest of its file.
    fn library(name: &str, path: &str, data: Option<&[u8]>) -> Library {
        Library {
            name: name.to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(DownloadInfo {
                    url: format!("https://example.com/{path}"),
                    path: Some(path.to_string()),
                    sha1: data.map(|data| Algorithm::Sha1.digest_to_string(data)),
                    size: data.map(|data| data.len() as u64),
                }),
                classifiers: None,
            }),
            ..Library::default()
        }
    }

    /// Writes the data to a file under the directory.
    fn file(base: &Path, name: &str, data: &[u8]) -> PathBuf {
        let path = base.join(name);
        fs::create_dir_all(path.parent().expect("the file should have a parent"))
            .expect("the directory should be created");
        fs::write(&path, data).expect("the file should be written");
        path
    }

    #[test]
    fn a_cached_library_is_found_by_its_digest() {
        let base = directory("digest");
        let data = b"the library";
        let mut cache = DefaultCacheRepository::new(&base);

        let library = library(
            "com.example:library:1.0",
            "com/example/library/1.0/library-1.0.jar",
            Some(data),
        );
        let jar = file(&base, "downloaded/library.jar", data);
        cache
            .cache_library(&library, &jar, false)
            .expect("the library should be cached");

        assert_eq!(
            cache
                .library_file(&library)
                .expect("the library should be found"),
            cache.cache().file_of(
                Algorithm::Sha1.name(),
                &Algorithm::Sha1.digest_to_string(data)
            )
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_index_is_read_again() {
        let base = directory("index");
        let data = b"the library";
        let mut cache = DefaultCacheRepository::new(&base);

        // A library of Forge: its digest is the digest of the installer, so the
        // library is found by its name.
        let library = Library {
            checksums: Some(vec![Algorithm::Sha1.digest_to_string(data)]),
            ..library(
                "net.minecraftforge:forge:1.12.2-14.23.5.2859",
                "net/minecraftforge/forge/1.12.2-14.23.5.2859/forge-1.12.2-14.23.5.2859-universal.jar",
                None,
            )
        };
        let jar = file(&base, "downloaded/forge.jar", data);
        cache
            .cache_library(&library, &jar, true)
            .expect("the library should be cached");

        let mut read = DefaultCacheRepository::new(&base);
        assert_eq!(
            read.library_file(&library)
                .expect("the library should be found"),
            cache.cache().file_of(
                Algorithm::Sha1.name(),
                &Algorithm::Sha1.digest_to_string(data)
            )
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_library_of_forge_is_checked_file_by_file() {
        let base = directory("forge-checksums");
        let path = base.join("forge.jar");

        // The manifest of the library tells the digest of its file.
        let manifest = format!(
            "{} net/example/Main.class\n",
            Algorithm::Sha1.digest_to_string(b"classes")
        );
        let mut writer = ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("net/example/Main.class", b"classes")
            .expect("the file should be added");
        writer
            .add_file("checksums.sha1", manifest.as_bytes())
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");

        // The digest of the archive is the digest the installer names.
        let digest = Algorithm::Sha1
            .digest_file_to_string(&path)
            .expect("the digest should be read");
        assert!(checksum_valid(&path, Some(&[digest])));

        // A digest of another archive is accepted when the digest of the
        // manifest of the archive is the checksum the version tells.
        let manifest_digest = Algorithm::Sha1.digest_to_string(manifest.as_bytes());
        assert!(checksum_valid(&path, Some(&[manifest_digest])));

        // A digest of a file that was changed is refused.
        let mut writer = ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("net/example/Main.class", b"other classes")
            .expect("the file should be added");
        writer
            .add_file("checksums.sha1", manifest.as_bytes())
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");
        assert!(!checksum_valid(
            &path,
            Some(&[Algorithm::Sha1.digest_to_string(manifest.as_bytes())])
        ));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_library_of_the_shared_directory_is_cached() {
        let base = directory("shared");
        let data = b"the library";
        let mut cache = DefaultCacheRepository::new(&base);

        let library = library(
            "com.example:library:1.0",
            "com/example/library/1.0/library-1.0.jar",
            Some(data),
        );
        file(
            &base,
            "libraries/com/example/library/1.0/library-1.0.jar",
            data,
        );

        let cached = cache
            .library_file(&library)
            .expect("the library of the shared directory should be found");
        assert_eq!(
            cached,
            cache.cache().file_of(
                Algorithm::Sha1.name(),
                &Algorithm::Sha1.digest_to_string(data)
            )
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_of_another_digest_is_not_cached() {
        let base = directory("changed");
        let mut cache = DefaultCacheRepository::new(&base);

        let library = library(
            "com.example:library:1.0",
            "com/example/library/1.0/library-1.0.jar",
            Some(b"the library"),
        );
        let jar = file(&base, "downloaded/library.jar", b"another file");
        assert!(!cache
            .try_cache_library(&library, &jar)
            .expect("the library should be checked"));
        assert!(cache.library_file(&library).is_none());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_library_without_a_digest_is_checked_by_the_checksums_of_its_files() {
        let base = directory("checksums-only");
        let data = b"the library";
        let mut cache = DefaultCacheRepository::new(&base);

        let library = Library {
            checksums: Some(vec![Algorithm::Sha1.digest_to_string(data)]),
            ..library(
                "com.example:library:1.0",
                "com/example/library/1.0/library-1.0.jar",
                None,
            )
        };
        let jar = file(&base, "downloaded/library.jar", data);

        assert!(cache
            .try_cache_library(&library, &jar)
            .expect("the library should be checked"));
        assert!(cache.library_file(&library).is_some());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn the_kinds_of_the_libraries_are_read() {
        assert_eq!(LibraryKind::parse("forge"), LibraryKind::Forge);
        assert_eq!(LibraryKind::parse("FORGE"), LibraryKind::Forge);
        assert_eq!(LibraryKind::parse("jar"), LibraryKind::Jar);
        assert_eq!(LibraryKind::parse("unknown"), LibraryKind::Jar);
        assert_eq!(LibraryKind::Forge.as_str(), "forge");
        assert_eq!(LibraryKind::Jar.as_str(), "jar");
    }
}
