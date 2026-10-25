//! The file of the authlib-injector the game is started with.
//!
//! The authlib-injector is an agent of the virtual machine that makes the game
//! talk to the server of the player instead of the service of Mojang. The
//! launcher ships the file and writes it into the directory of the
//! dependencies the first time the game is started with an account of such a
//! server.
//!
//! The launcher reads the version of the file out of its manifest, so a file
//! the player replaced is reported rather than silently used.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::metadata::current_directory;
use crate::util::logging;
use crate::util::zip;

/// The version of the authlib-injector the launcher ships.
pub const AUTHLIB_INJECTOR_VERSION: &str = "1.2.8";

/// The bytes of the authlib-injector the launcher ships.
///
/// The file is compiled into the program, which is what the launcher of Java
/// does with the file it keeps in the class path: the launcher needs no network
/// to write the agent out.
static AUTHLIB_INJECTOR_BYTES: &[u8] =
    include_bytes!("../../../resources/assets/authlib-injector-1.2.8.jar");

/// The name of the title a manifest of the authlib-injector names.
const IMPLEMENTATION_TITLE: &str = "authlib-injector";

/// The name of the entry that holds the manifest of an archive.
const MANIFEST_ENTRY: &str = "META-INF/MANIFEST.MF";

/// What the launcher knows about a file of the authlib-injector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthlibInjectorArtifactInfo {
    /// The number of the build of the file.
    build_number: i32,
    /// The version of the file, such as `1.2.8`.
    version: String,
    /// The absolute path of the file.
    location: PathBuf,
}

impl AuthlibInjectorArtifactInfo {
    /// Reads the information of a file of the authlib-injector.
    ///
    /// The information is read out of the manifest of the archive, which is
    /// what tells the agent from another archive and what names its version. A
    /// file that is not an archive, or an archive whose manifest does not name
    /// the agent, is refused with the message the launcher of Java uses.
    pub fn from_path(location: &Path) -> io::Result<Self> {
        let data = zip::read_entry(location, MANIFEST_ENTRY)
            .map_err(|error| invalid_data(error.to_string()))?
            .ok_or_else(|| invalid_data("Missing Implementation-Title"))?;

        let manifest = String::from_utf8_lossy(&data);
        let attribute = |name: &str| manifest_attribute(&manifest, name);

        match attribute("Implementation-Title") {
            None => return Err(invalid_data("Missing Implementation-Title")),
            Some(title) if title != IMPLEMENTATION_TITLE => {
                return Err(invalid_data("Bad Implementation-Title"));
            }
            Some(_) => {}
        }

        let version = attribute("Implementation-Version")
            .ok_or_else(|| invalid_data("Missing Implementation-Version"))?;

        let build_number = match attribute("Build-Number") {
            None => return Err(invalid_data("Missing Build-Number")),
            Some(number) => number
                .parse::<i32>()
                .map_err(|error| invalid_data_with("Bad Build-Number", &error))?,
        };

        Ok(Self {
            build_number,
            version,
            location: absolute(location),
        })
    }

    /// Returns the number of the build of the file.
    pub const fn build_number(&self) -> i32 {
        self.build_number
    }

    /// Returns the version of the file.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the absolute path of the file.
    pub fn location(&self) -> &Path {
        &self.location
    }
}

impl fmt::Display for AuthlibInjectorArtifactInfo {
    /// Writes the version of the file and the number of its build.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "authlib-injector [buildNumber={}, version={}]",
            self.build_number, self.version
        )
    }
}

/// Provides the file of the authlib-injector the game is started with.
///
/// The launcher may be told where the file is by the player, in which case it
/// is used as it is; otherwise the launcher writes the file it ships into the
/// directory of the dependencies.
pub trait AuthlibInjectorArtifactProvider {
    /// Returns the file, writing it out when the launcher ships it.
    ///
    /// A failure is reported rather than replaced by a file the launcher does
    /// not know, because the game cannot be started without the agent.
    fn get_artifact_info(&self) -> io::Result<AuthlibInjectorArtifactInfo>;

    /// Returns the file when the launcher already has it, without writing
    /// anything.
    ///
    /// The launcher asks this before it starts the game for a player that did
    /// not log in: a file that is not there yet is not written, because the
    /// player is not going to need it.
    fn get_artifact_info_immediately(&self) -> Option<AuthlibInjectorArtifactInfo>;
}

/// Provides the file of the authlib-injector the player named.
///
/// The launcher of Java uses this provider when the player names the file in
/// the settings of the launcher; the file is then read as it is, without a
/// copy in the directory of the dependencies.
#[derive(Debug, Clone)]
pub struct SimpleAuthlibInjectorArtifactProvider {
    /// The file the player named.
    location: PathBuf,
}

impl SimpleAuthlibInjectorArtifactProvider {
    /// Creates a provider of the file the player named.
    pub fn new(location: impl Into<PathBuf>) -> Self {
        Self {
            location: location.into(),
        }
    }

    /// Returns the file the player named.
    pub fn location(&self) -> &Path {
        &self.location
    }
}

impl AuthlibInjectorArtifactProvider for SimpleAuthlibInjectorArtifactProvider {
    /// Reads the file the player named.
    fn get_artifact_info(&self) -> io::Result<AuthlibInjectorArtifactInfo> {
        AuthlibInjectorArtifactInfo::from_path(&self.location)
    }

    /// Reads the file the player named, reporting a file the launcher cannot
    /// use as a file that is not there.
    fn get_artifact_info_immediately(&self) -> Option<AuthlibInjectorArtifactInfo> {
        match AuthlibInjectorArtifactInfo::from_path(&self.location) {
            Ok(info) => Some(info),
            Err(error) => {
                logging::warn(&format!("Bad authlib-injector artifact: {error}"));
                None
            }
        }
    }
}

/// Writes the authlib-injector the launcher ships into the directory of the
/// dependencies.
///
/// The file is written once: a file that is there and reads as an
/// authlib-injector is used as it is, so the launcher writes the agent only
/// when the file is missing or was replaced by something the launcher cannot
/// use.
#[derive(Debug)]
pub struct AuthlibInjectorExtractor {
    /// The bytes the launcher writes out.
    source: &'static [u8],
    /// The file the launcher writes.
    artifact_location: PathBuf,
    /// The lock that keeps two threads from writing the file at once.
    lock: Mutex<()>,
}

impl AuthlibInjectorExtractor {
    /// Creates an extractor of the file the launcher ships.
    ///
    /// The source is the file the launcher compiled into the program, and the
    /// destination is the file the game is started with.
    pub fn new(source: &'static [u8], artifact_location: impl Into<PathBuf>) -> Self {
        Self {
            source,
            artifact_location: artifact_location.into(),
            lock: Mutex::new(()),
        }
    }

    /// Creates an extractor of the authlib-injector the launcher ships, kept in
    /// the directory of the dependencies.
    pub fn shipped(artifact_location: impl Into<PathBuf>) -> Self {
        Self::new(AUTHLIB_INJECTOR_BYTES, artifact_location)
    }

    /// Returns the file the launcher writes.
    pub fn artifact_location(&self) -> &Path {
        &self.artifact_location
    }
}

impl AuthlibInjectorArtifactProvider for AuthlibInjectorExtractor {
    /// Returns the file, writing the one the launcher ships when it is not
    /// there.
    fn get_artifact_info(&self) -> io::Result<AuthlibInjectorArtifactInfo> {
        if let Some(info) = self.get_artifact_info_immediately() {
            return Ok(info);
        }

        // The lock is held for the whole extraction, so the second thread that
        // waits reads the file the first one wrote instead of writing it again.
        let _guard = self.lock.lock().unwrap_or_else(|error| error.into_inner());

        if let Some(info) = self.get_artifact_info_immediately() {
            return Ok(info);
        }

        logging::info("No local authlib-injector found, extracting");

        if let Some(parent) = self.artifact_location.parent() {
            fs::create_dir_all(parent)?;
        }
        write_safely(&self.artifact_location, self.source)?;

        self.get_artifact_info_immediately()
            .ok_or_else(|| invalid_data("Failed to extract authlib-injector artifact"))
    }

    /// Returns the file when it is there, without writing it.
    fn get_artifact_info_immediately(&self) -> Option<AuthlibInjectorArtifactInfo> {
        if !self.artifact_location.is_file() {
            return None;
        }

        match AuthlibInjectorArtifactInfo::from_path(&self.artifact_location) {
            Ok(info) => Some(info),
            Err(error) => {
                logging::warn(&format!("Bad authlib-injector artifact: {error}"));
                None
            }
        }
    }
}

/// Returns the value of an attribute of a manifest.
///
/// A manifest writes a line that is too long as several lines, where every line
/// after the first starts with a space; the lines are joined before the
/// attributes are read. The attributes of the archive are the ones before the
/// first empty line, because the lines after it name the entries of the
/// archive. The names of the attributes are compared without their case, which
/// is the way Java reads a manifest.
fn manifest_attribute(manifest: &str, name: &str) -> Option<String> {
    let mut attributes: Vec<String> = Vec::new();
    for line in manifest.lines() {
        if line.is_empty() {
            break;
        }

        match line.strip_prefix(' ') {
            Some(continuation) => {
                if let Some(last) = attributes.last_mut() {
                    last.push_str(continuation);
                }
            }
            None => attributes.push(line.trim_end_matches('\r').to_string()),
        }
    }

    attributes.iter().find_map(|attribute| {
        let (key, value) = attribute.split_once(':')?;
        if key.eq_ignore_ascii_case(name) {
            Some(value.trim_start().to_string())
        } else {
            None
        }
    })
}

/// Returns the path as an absolute path.
///
/// The launcher writes the path of the agent into the command of the game,
/// which the virtual machine reads from the directory the game runs in, so a
/// relative path is resolved against the directory the launcher was started
/// from.
fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_directory().join(path)
    }
}

/// Writes a file through a temporary file next to it.
///
/// The file is written under another name and then moved over the destination,
/// so a launcher that is stopped while it writes leaves the file that was there
/// rather than half of the new one.
fn write_safely(path: &Path, data: &[u8]) -> io::Result<()> {
    let temporary = temporary_path(path);
    fs::write(&temporary, data)?;

    // A move that fails leaves the temporary file behind, which the next run
    // overwrites; the destination is not damaged either way.
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

/// Returns the name of the temporary file a write of a file goes through.
///
/// The file is written next to the destination, so the move over the
/// destination stays within one directory and replaces it in one step.
fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    path.with_file_name(format!(".{file_name}.tmp"))
}

/// Returns a failure that reports a file the launcher cannot use.
fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// Returns a failure that reports a file the launcher cannot use, with the
/// reason the value was refused.
fn invalid_data_with(message: &str, error: &dyn fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{message}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Returns a directory of the tests that no other test writes into.
    fn temporary_directory(name: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let index = COUNTER.fetch_add(1, Ordering::Relaxed);

        let directory = std::env::temp_dir()
            .join("hmcl-authlib-injector")
            .join(format!("{name}-{}-{index}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("the directory should be created");
        directory
    }

    /// Writes an archive whose manifest is the text, and returns its path.
    fn archive(name: &str, manifest: &str) -> PathBuf {
        let path = temporary_directory(name).join("authlib-injector.jar");
        let mut writer = zip::ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file(MANIFEST_ENTRY, manifest.as_bytes())
            .expect("the manifest should be written");
        writer.finish().expect("the archive should be finished");
        path
    }

    /// Returns a manifest of the authlib-injector with the fields the tests
    /// vary.
    fn manifest(title: &str, version: Option<&str>, build: Option<&str>) -> String {
        let mut text = format!(
            "Manifest-Version: 1.0\r\nImplementation-Title: {title}\r\nPremain-Class: moe.yushi.authlibinjector.Premain\r\n"
        );
        if let Some(version) = version {
            text.push_str(&format!("Implementation-Version: {version}\r\n"));
        }
        if let Some(build) = build {
            text.push_str(&format!("Build-Number: {build}\r\n"));
        }
        text.push_str("\r\n");
        text
    }

    #[test]
    fn the_file_the_launcher_ships_is_an_authlib_injector() {
        let directory = temporary_directory("shipped");
        let extractor = AuthlibInjectorExtractor::shipped(directory.join("authlib-injector.jar"));

        // The file is not there yet, so the launcher writes the one it ships.
        assert!(extractor.get_artifact_info_immediately().is_none());

        let info = extractor
            .get_artifact_info()
            .expect("the file should be written");

        assert_eq!(info.version(), AUTHLIB_INJECTOR_VERSION);
        assert!(info.build_number() > 0);
        assert_eq!(
            info.to_string(),
            format!(
                "authlib-injector [buildNumber={}, version={AUTHLIB_INJECTOR_VERSION}]",
                info.build_number()
            )
        );
        assert!(info.location().is_absolute());
        assert!(extractor.get_artifact_info_immediately().is_some());
    }

    #[test]
    fn the_file_that_is_there_is_not_written_again() {
        let directory = temporary_directory("written-once");
        let path = directory.join("authlib-injector.jar");
        let extractor = AuthlibInjectorExtractor::shipped(&path);

        extractor
            .get_artifact_info()
            .expect("the file should be written");
        let written = fs::read(&path).expect("the file should be read");

        // A file that reads as an authlib-injector is used as it is, even when
        // it is not the one the launcher ships.
        fs::write(&path, &written).expect("the file should be written");
        extractor
            .get_artifact_info()
            .expect("the file should be read");

        assert_eq!(
            fs::read(&path).expect("the file should be read"),
            written,
            "the file should be left as it is"
        );
    }

    #[test]
    fn a_file_the_launcher_cannot_use_is_written_again() {
        let directory = temporary_directory("replaced");
        let path = directory.join("authlib-injector.jar");
        fs::write(&path, "not an archive").expect("the file should be written");

        let extractor = AuthlibInjectorExtractor::shipped(&path);
        assert!(extractor.get_artifact_info_immediately().is_none());

        let info = extractor
            .get_artifact_info()
            .expect("the file should be written again");
        assert_eq!(info.version(), AUTHLIB_INJECTOR_VERSION);
    }

    #[test]
    fn the_manifest_of_a_file_names_its_version() {
        let path = archive(
            "manifest",
            &manifest("authlib-injector", Some("1.2.8"), Some("56")),
        );
        let info = AuthlibInjectorArtifactInfo::from_path(&path).expect("the file should be read");

        assert_eq!(info.build_number(), 56);
        assert_eq!(info.version(), "1.2.8");
    }

    #[test]
    fn the_attributes_of_a_manifest_are_read_without_their_case() {
        let path = archive(
            "case",
            "Manifest-Version: 1.0\r\nimplementation-title: authlib-injector\r\nimplementation-version: 1.2.8\r\nbuild-number: 56\r\n\r\n",
        );
        let info = AuthlibInjectorArtifactInfo::from_path(&path).expect("the file should be read");

        assert_eq!(info.version(), "1.2.8");
        assert_eq!(info.build_number(), 56);
    }

    #[test]
    fn a_manifest_writes_a_long_line_as_several_lines() {
        // The version is written over two lines, which Java joins before it
        // reads the value.
        let path = archive(
            "continuation",
            "Manifest-Version: 1.0\r\nImplementation-Title: authlib-injector\r\nImplementation-Version: 1.\r\n 2.8\r\nBuild-Number: 56\r\n\r\n",
        );
        let info = AuthlibInjectorArtifactInfo::from_path(&path).expect("the file should be read");

        assert_eq!(info.version(), "1.2.8");
    }

    #[test]
    fn a_file_without_a_manifest_is_refused() {
        let path = temporary_directory("no-manifest").join("authlib-injector.jar");
        let mut writer = zip::ZipWriter::create(&path).expect("the archive should be created");
        writer
            .add_file("other.txt", b"nothing")
            .expect("the file should be written");
        writer.finish().expect("the archive should be finished");

        let error =
            AuthlibInjectorArtifactInfo::from_path(&path).expect_err("the file should be refused");
        assert_eq!(error.to_string(), "Missing Implementation-Title");
    }

    #[test]
    fn a_file_that_is_not_an_archive_is_refused() {
        let path = temporary_directory("not-an-archive").join("authlib-injector.jar");
        fs::write(&path, "not an archive").expect("the file should be written");

        assert!(AuthlibInjectorArtifactInfo::from_path(&path).is_err());
    }

    #[test]
    fn a_manifest_that_names_another_title_is_refused() {
        let path = archive(
            "bad-title",
            &manifest("another-agent", Some("1.2.8"), Some("56")),
        );
        let error =
            AuthlibInjectorArtifactInfo::from_path(&path).expect_err("the file should be refused");

        assert_eq!(error.to_string(), "Bad Implementation-Title");
    }

    #[test]
    fn a_manifest_that_names_no_version_is_refused() {
        let path = archive(
            "no-version",
            &manifest("authlib-injector", None, Some("56")),
        );
        let error =
            AuthlibInjectorArtifactInfo::from_path(&path).expect_err("the file should be refused");

        assert_eq!(error.to_string(), "Missing Implementation-Version");
    }

    #[test]
    fn a_manifest_that_names_no_build_is_refused() {
        let path = archive(
            "no-build",
            &manifest("authlib-injector", Some("1.2.8"), None),
        );
        let error =
            AuthlibInjectorArtifactInfo::from_path(&path).expect_err("the file should be refused");

        assert_eq!(error.to_string(), "Missing Build-Number");
    }

    #[test]
    fn a_manifest_whose_build_is_not_a_number_is_refused() {
        let path = archive(
            "bad-build",
            &manifest("authlib-injector", Some("1.2.8"), Some("fifty-six")),
        );
        let error =
            AuthlibInjectorArtifactInfo::from_path(&path).expect_err("the file should be refused");

        assert!(error.to_string().starts_with("Bad Build-Number"), "{error}");
    }

    #[test]
    fn the_file_the_player_named_is_read_as_it_is() {
        let path = archive(
            "simple",
            &manifest("authlib-injector", Some("1.2.8"), Some("56")),
        );
        let provider = SimpleAuthlibInjectorArtifactProvider::new(&path);

        let info = provider
            .get_artifact_info()
            .expect("the file should be read");
        assert_eq!(info.version(), "1.2.8");
        assert_eq!(provider.location(), path.as_path());
    }

    #[test]
    fn the_file_the_player_named_that_cannot_be_used_is_not_there() {
        let path = temporary_directory("simple-bad").join("authlib-injector.jar");
        fs::write(&path, "not an archive").expect("the file should be written");
        let provider = SimpleAuthlibInjectorArtifactProvider::new(&path);

        // The file that cannot be read is reported as a file that is not
        // there, but the launcher does not hide the failure when it needs the
        // file.
        assert!(provider.get_artifact_info_immediately().is_none());
        assert!(provider.get_artifact_info().is_err());
    }
}
