//! The Java runtimes the launcher can start the game with.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::util::versioning::VersionNumber;

use super::{java_executable_name, javac_executable_name, JavaInfo, JavaInfoError, Platform};

/// The error of reading a Java runtime.
#[derive(Debug)]
pub enum JavaRuntimeError {
    /// The executable of the runtime cannot be read.
    Io(io::Error),
    /// The executable does not tell the information about itself.
    Info(JavaInfoError),
    /// The runtime was built for a platform this machine cannot run.
    Incompatible(Platform),
}

impl fmt::Display for JavaRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "The Java runtime cannot be read: {error}"),
            Self::Info(error) => write!(formatter, "The Java runtime cannot be told: {error}"),
            Self::Incompatible(platform) => write!(
                formatter,
                "The Java runtime was built for {platform}, which this machine cannot run"
            ),
        }
    }
}

impl std::error::Error for JavaRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Info(error) => Some(error),
            Self::Incompatible(_) => None,
        }
    }
}

/// A Java runtime the launcher can start the game with.
///
/// A runtime is the executable of Java, the information the runtime tells
/// about itself, and whether the launcher manages the runtime, which the
/// launcher shows first.
#[derive(Debug, Clone)]
pub struct JavaRuntime {
    /// The executable of Java.
    binary: PathBuf,
    /// The information the runtime tells about itself.
    info: JavaInfo,
    /// Whether the launcher installed the runtime itself.
    managed: bool,
    /// Whether the runtime has a compiler, which makes it a JDK rather than a
    /// JRE.
    jdk: bool,
}

impl JavaRuntime {
    /// Creates a runtime from the executable and the information about it.
    ///
    /// The runtime is a JDK when the compiler is next to the executable.
    pub fn from_binary(binary: PathBuf, info: JavaInfo, managed: bool) -> Self {
        let jdk = binary.with_file_name(javac_executable_name()).is_file();

        Self {
            binary,
            info,
            managed,
            jdk,
        }
    }

    /// Reads the runtime of a Java executable.
    ///
    /// The information about the runtime is read from the `release` file of the
    /// Java home the executable is in; a runtime without a readable release
    /// file is asked by running it. The runtime is rejected when it was built
    /// for a platform this machine cannot run.
    pub fn detect(binary: &Path, managed: bool) -> Result<Self, JavaRuntimeError> {
        let binary = canonicalize(binary).map_err(JavaRuntimeError::Io)?;
        if !binary.is_file() {
            return Err(JavaRuntimeError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} is not a file", binary.display()),
            )));
        }

        let info = match read_release_file(&binary) {
            Some(info) => info,
            None => read_runtime_properties(&binary)
                .map_err(JavaRuntimeError::Io)
                .and_then(|contents| {
                    JavaInfo::from_properties(&contents).map_err(JavaRuntimeError::Info)
                })?,
        };

        if !info.platform().is_compatible_with_system() {
            return Err(JavaRuntimeError::Incompatible(info.platform()));
        }

        Ok(Self::from_binary(binary, info, managed))
    }

    /// Reads the runtime of a Java home.
    ///
    /// The executable is the `java` program in the `bin` directory of the
    /// home.
    pub fn from_java_home(home: &Path, managed: bool) -> Result<Self, JavaRuntimeError> {
        Self::detect(&home.join("bin").join(java_executable_name()), managed)
    }

    /// Returns the executable of Java.
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// Returns the information the runtime tells about itself.
    pub fn info(&self) -> &JavaInfo {
        &self.info
    }

    /// Returns the version of Java, as the runtime writes it.
    pub fn version(&self) -> &str {
        self.info.version()
    }

    /// Returns the major version of Java, or `-1` when it is not known.
    pub fn parsed_version(&self) -> i32 {
        self.info.parsed_version()
    }

    /// Returns the version of Java as a number the launcher can compare.
    pub fn version_number(&self) -> &VersionNumber {
        self.info.version_number()
    }

    /// Returns the organization that built the runtime, if it is known.
    pub fn vendor(&self) -> Option<&str> {
        self.info.vendor()
    }

    /// Returns the operating system and the architecture the runtime was built
    /// for.
    pub fn platform(&self) -> Platform {
        self.info.platform()
    }

    /// Returns whether the launcher installed the runtime itself.
    pub fn is_managed(&self) -> bool {
        self.managed
    }

    /// Returns whether the runtime has a compiler, which makes it a JDK rather
    /// than a JRE.
    pub fn is_jdk(&self) -> bool {
        self.jdk
    }

    /// Returns whether the runtime can run on this machine.
    pub fn is_compatible_with_system(&self) -> bool {
        self.info.platform().is_compatible_with_system()
    }
}

impl PartialEq for JavaRuntime {
    fn eq(&self, other: &Self) -> bool {
        self.binary == other.binary
    }
}

impl Eq for JavaRuntime {}

impl Hash for JavaRuntime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.binary.hash(state);
    }
}

impl PartialOrd for JavaRuntime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JavaRuntime {
    /// Orders the runtimes the way the version of the launcher written in Java
    /// orders them: the runtimes the launcher manages come first, then the
    /// newer versions of Java, then the architectures, and then the
    /// executables.
    fn cmp(&self, other: &Self) -> Ordering {
        self.managed
            .cmp(&other.managed)
            .reverse()
            .then_with(|| self.parsed_version().cmp(&other.parsed_version()))
            .then_with(|| self.version_number().cmp(other.version_number()))
            .then_with(|| arch_order(self.platform().arch).cmp(&arch_order(other.platform().arch)))
            .then_with(|| self.binary.cmp(&other.binary))
    }
}

/// Returns the order of an architecture, from the oldest to the newest.
fn arch_order(arch: crate::util::operating_system::Arch) -> u8 {
    use crate::util::operating_system::Arch;

    match arch {
        Arch::X86 => 0,
        Arch::X86_64 => 1,
        Arch::Arm => 2,
        Arch::Aarch64 => 3,
        Arch::Unknown => 4,
    }
}

/// Returns the path without the prefix Windows uses for its long paths.
///
/// The long paths of Windows name the same files as the paths the player
/// writes, but the launcher shows the paths it reads the files with, so the
/// prefix is removed.
pub(crate) fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    let path = std::fs::canonicalize(path)?;
    let text = path.to_string_lossy();

    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        Ok(PathBuf::from(format!(r"\\{rest}")))
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        Ok(PathBuf::from(rest))
    } else {
        Ok(path)
    }
}

/// Reads the information from the release file next to the executable, if the
/// file is there and tells the information.
fn read_release_file(binary: &Path) -> Option<JavaInfo> {
    let home = binary.parent()?.parent()?;
    let contents = std::fs::read_to_string(home.join("release")).ok()?;

    JavaInfo::from_release_file(&contents).ok()
}

/// Asks a Java runtime for the settings that tell the information about it.
///
/// The runtime writes the settings to the error output; the launcher reads
/// both outputs, because the runtime writes the version to the error output and
/// other settings to the standard output on some versions. The runtime is
/// stopped when it does not answer in ten seconds, so a broken runtime does not
/// keep the launcher from finding the other runtimes.
fn read_runtime_properties(binary: &Path) -> io::Result<String> {
    let mut child = Command::new(binary)
        .args(["-XshowSettings:properties", "-version"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .map(read_all)
        .ok_or_else(|| io::Error::other("the output of the Java runtime is not piped"))?;
    let stderr = child
        .stderr
        .take()
        .map(read_all)
        .ok_or_else(|| io::Error::other("the error output of the Java runtime is not piped"))?;

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout.join();
            let _ = stderr.join();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("{} did not answer in ten seconds", binary.display()),
            ));
        }
        thread::sleep(Duration::from_millis(25));
    }

    let mut contents = stdout.join().unwrap_or_default();
    contents.push_str(&stderr.join().unwrap_or_default());
    Ok(contents)
}

/// Reads all bytes of a stream as text.
fn read_all(mut stream: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::java::discovery::find_java_in_path;
    use crate::util::operating_system::{current_os, Arch, OsType};

    /// Returns the release file of a runtime of the current platform.
    fn release(version: &str) -> String {
        let current = Platform::current();
        format!(
            "IMPLEMENTOR=\"Eclipse Adoptium\"\nJAVA_VERSION=\"{version}\"\nOS_ARCH=\"{}\"\nOS_NAME=\"{}\"\n",
            match current.arch {
                Arch::X86_64 => "x86_64",
                Arch::X86 => "x86",
                Arch::Aarch64 => "aarch64",
                Arch::Arm => "arm",
                Arch::Unknown => "unknown",
            },
            match current.os {
                OsType::Windows => "Windows",
                OsType::Linux => "Linux",
                OsType::MacOs => "Darwin",
                OsType::Unknown => "unknown",
            }
        )
    }

    /// Creates a Java home with a release file and an empty executable.
    fn java_home(directory: &Path, name: &str, version: &str) -> PathBuf {
        let home = directory.join(name);
        std::fs::create_dir_all(home.join("bin")).expect("the bin directory should be made");
        std::fs::write(home.join("release"), release(version))
            .expect("the release should be written");
        std::fs::write(
            home.join("bin").join(java_executable_name()),
            b"not a program",
        )
        .expect("the executable should be written");
        home
    }

    #[test]
    fn the_runtime_of_a_java_home_is_read() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-java-runtime-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let home = java_home(&directory, "jdk-17", "17.0.9");

        let runtime =
            JavaRuntime::from_java_home(&home, false).expect("the runtime should be read");

        assert_eq!(runtime.version(), "17.0.9");
        assert_eq!(runtime.parsed_version(), 17);
        assert_eq!(runtime.vendor(), Some("Eclipse Adoptium"));
        assert!(!runtime.is_jdk());
        assert!(runtime.is_compatible_with_system());
        assert!(runtime
            .binary()
            .ends_with(Path::new("jdk-17").join("bin").join(java_executable_name())));

        // The compiler makes the runtime a JDK.
        std::fs::write(
            home.join("bin").join(javac_executable_name()),
            b"not a program",
        )
        .expect("the compiler should be written");
        let runtime = JavaRuntime::from_java_home(&home, true).expect("the runtime should be read");
        assert!(runtime.is_jdk());
        assert!(runtime.is_managed());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_java_home_without_an_executable_is_rejected() {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-rs-java-runtime-missing-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");

        assert!(matches!(
            JavaRuntime::from_java_home(&directory, false),
            Err(JavaRuntimeError::Io(_))
        ));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_runtime_of_another_platform_is_rejected() {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-rs-java-runtime-platform-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let home = directory.join("jdk");
        std::fs::create_dir_all(home.join("bin")).expect("the bin directory should be made");
        std::fs::write(
            home.join("release"),
            "IMPLEMENTOR=\"Test\"\nJAVA_VERSION=\"17\"\nOS_ARCH=\"x86_64\"\nOS_NAME=\"Plan9\"\n",
        )
        .expect("the release should be written");
        std::fs::write(
            home.join("bin").join(java_executable_name()),
            b"not a program",
        )
        .expect("the executable should be written");

        // The release file of the runtime names an operating system the
        // launcher does not know, so the runtime is asked by running it; the
        // file is not a program, so reading the runtime fails.
        assert!(matches!(
            JavaRuntime::from_java_home(&home, false),
            Err(JavaRuntimeError::Io(_)) | Err(JavaRuntimeError::Info(_))
        ));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_runtimes_are_ordered_by_their_versions() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-java-runtime-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        let make = |name: &str, version: &str, managed: bool| {
            let binary = directory
                .join(name)
                .join("bin")
                .join(java_executable_name());
            JavaRuntime::from_binary(
                binary,
                JavaInfo::new(Platform::current(), version.to_string(), None),
                managed,
            )
        };

        let mut runtimes = [
            make("jdk-21", "21.0.1", false),
            make("jdk-8", "1.8.0_462", false),
            make("managed-17", "17.0.9", true),
        ];
        runtimes.sort();

        assert_eq!(runtimes[0].version(), "17.0.9");
        assert!(runtimes[0].is_managed());
        assert_eq!(runtimes[1].version(), "1.8.0_462");
        assert_eq!(runtimes[2].version(), "21.0.1");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_java_of_the_machine_is_read() {
        let Some(binary) = find_java_in_path() else {
            // The machine the tests run on may not have Java.
            return;
        };

        let runtime = JavaRuntime::detect(&binary, false).expect("the Java should be read");

        assert!(runtime.parsed_version() > 0);
        assert!(!runtime.version().is_empty());
        assert!(runtime.is_compatible_with_system());
        assert_eq!(runtime.platform().os, current_os());
    }
}
