//! Finding the Java runtimes the machine has.
//!
//! The players install Java in the places their systems install programs and
//! the launchers of the game install the runtimes they need; the launcher looks
//! in those places and adds the runtimes the player chose.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::util::operating_system::{current_arch, current_os, Arch, OsType};

use super::runtime::canonicalize;
use super::{java_executable_name, JavaRuntime};

/// The directories Windows keeps the Java runtimes of a vendor in.
const WINDOWS_VENDOR_DIRECTORIES: &[&str] = &[
    "Java",
    "BellSoft",
    "AdoptOpenJDK",
    "Zulu",
    "Microsoft",
    "Eclipse Foundation",
    "Eclipse Adoptium",
    "Amazon Corretto",
    "Semeru",
    "JetBrains",
    "SapMachine",
];

/// The directories a launcher of the game installs the Java runtimes in.
const OFFICIAL_RUNTIME_DIRECTORIES: &[&str] = &[
    ".minecraft/runtime",
    "Library/Application Support/minecraft/runtime",
];

/// The places the launcher looks for Java runtimes.
///
/// The launcher does not search the whole file system: it looks in the places
/// Java is installed in and in the places the player tells it. A runtime that
/// cannot be read by one of the places is skipped, so a broken runtime does not
/// keep the launcher from showing the other runtimes.
#[derive(Debug, Clone, Default)]
pub struct JavaSearch {
    /// The directories whose children are Java homes.
    homes: Vec<PathBuf>,
    /// The Java homes themselves.
    known_homes: Vec<PathBuf>,
    /// The executables of Java.
    executables: Vec<PathBuf>,
    /// The roots of the Java runtimes the launcher manages.
    managed_roots: Vec<PathBuf>,
    /// The roots of the Java runtimes a launcher of the game installed.
    official_roots: Vec<PathBuf>,
    /// The executables the player disabled.
    disabled: Vec<PathBuf>,
}

impl JavaSearch {
    /// Creates a search that finds nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a directory whose children are Java homes, such as
    /// `/usr/lib/jvm`.
    pub fn with_homes_in(mut self, directory: impl Into<PathBuf>) -> Self {
        self.homes.push(directory.into());
        self
    }

    /// Adds a Java home.
    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.known_homes.push(home.into());
        self
    }

    /// Adds the executable of a Java runtime.
    pub fn with_executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.executables.push(executable.into());
        self
    }

    /// Adds a directory of the runtimes the launcher manages, which keeps a
    /// runtime in `<root>/<platform>/<name>` and its manifest next to it.
    pub fn with_managed_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.managed_roots.push(root.into());
        self
    }

    /// Adds a directory of the runtimes a launcher of the game installed,
    /// which keeps a runtime in `<root>/<component>/<platform>/<component>`.
    pub fn with_official_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.official_roots.push(root.into());
        self
    }

    /// Disables a runtime, so the player does not see it.
    ///
    /// The runtime is named by its executable; a runtime the launcher reads
    /// through another path is still the same runtime, so the path is resolved
    /// before it is compared.
    pub fn with_disabled(mut self, executable: impl Into<PathBuf>) -> Self {
        self.disabled.push(executable.into());
        self
    }

    /// Returns the runtimes the search found, ordered the way the launcher
    /// shows them.
    ///
    /// A runtime that cannot be read is skipped; the runtimes the player
    /// disabled are not returned.
    pub fn search(&self) -> Vec<JavaRuntime> {
        let disabled: HashSet<PathBuf> = self
            .disabled
            .iter()
            .filter_map(|executable| canonicalize(executable).ok())
            .collect();

        let mut candidates: Vec<(PathBuf, bool)> = Vec::new();
        for root in &self.managed_roots {
            candidates.extend(managed_executables(root));
        }
        candidates.extend(
            self.executables
                .iter()
                .map(|executable| (executable.clone(), false)),
        );
        for home in &self.known_homes {
            if let Some(executable) = executable_in_home(home) {
                candidates.push((executable, false));
            }
        }
        for directory in &self.homes {
            for home in child_directories(directory) {
                if let Some(executable) = executable_in_home(&home) {
                    candidates.push((executable, false));
                }
            }
        }
        for root in &self.official_roots {
            for official in official_executables(root) {
                candidates.push((official, false));
            }
        }

        let mut seen = HashSet::new();
        let mut runtimes = Vec::new();
        for (executable, managed) in candidates {
            let Ok(runtime) = JavaRuntime::detect(&executable, managed) else {
                continue;
            };
            if disabled.contains(runtime.binary()) {
                continue;
            }
            if seen.insert(runtime.binary().to_path_buf()) {
                runtimes.push(runtime);
            }
        }

        runtimes.sort();
        runtimes
    }
}

/// Returns the search the launcher starts with.
///
/// The search looks in the places the operating system installs Java in, in
/// the runtimes a launcher of the game installed, in the runtimes the launcher
/// manages under `managed_root`, and in the variables the player sets:
/// `JAVA_HOME` names a Java home and `HMCL_JRES` names the homes separated by
/// the separator of the system.
///
/// The path of the launcher itself is not a Java home; the game needs another
/// Java runtime than the one the launcher is written in.
pub fn default_search(managed_root: Option<&Path>) -> JavaSearch {
    let mut search = JavaSearch::new();

    if let Some(root) = managed_root {
        search = search.with_managed_root(root);
    }
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        search = search.with_home(home);
    }
    if let Some(homes) = std::env::var_os("HMCL_JRES") {
        for home in std::env::split_paths(&homes) {
            search = search.with_home(home);
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            // The `javapath` directory of the Oracle installer does not point
            // at a Java home, and the launcher lists the runtime itself.
            if current_os() == OsType::Windows && is_oracle_javapath(&directory) {
                continue;
            }

            let executable = directory.join(java_executable_name());
            if executable.is_file() {
                search = search.with_executable(executable);
            }
        }
    }

    match current_os() {
        OsType::Windows => {
            let program_files = [
                std::env::var_os("ProgramFiles"),
                std::env::var_os("ProgramFiles(x86)"),
                Some("C:\\Program Files".into()),
                Some("C:\\Program Files (x86)".into()),
            ];
            for program_files in program_files.into_iter().flatten() {
                for vendor in WINDOWS_VENDOR_DIRECTORIES {
                    search = search.with_homes_in(PathBuf::from(&program_files).join(vendor));
                }
            }

            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                search = search.with_official_root(
                    PathBuf::from(local)
                        .join("Packages")
                        .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
                        .join("LocalCache")
                        .join("Local")
                        .join("runtime"),
                );
            }
            if let Some(program_files) = std::env::var_os("ProgramFiles(x86)") {
                search = search.with_official_root(
                    PathBuf::from(program_files)
                        .join("Minecraft Launcher")
                        .join("runtime"),
                );
            }
        }
        OsType::Linux => {
            for directory in [
                "/usr/java",
                "/usr/lib/jvm",
                "/usr/lib32/jvm",
                "/usr/lib64/jvm",
            ] {
                search = search.with_homes_in(directory);
            }
            if let Some(home) = home_directory() {
                search = search.with_homes_in(home.join(".sdkman").join("candidates").join("java"));
                if current_arch() == Arch::X86_64 {
                    for directory in OFFICIAL_RUNTIME_DIRECTORIES {
                        search = search.with_official_root(home.join(directory));
                    }
                }
            }
        }
        OsType::MacOs => {
            search = search.with_homes_in("/Library/Java/JavaVirtualMachines");
            search = search
                .with_home("/Library/Internet Plug-Ins/JavaAppletPlugin.plugin/Contents/Home");
            search = search.with_home("/Applications/Xcode.app/Contents/Applications/Application Loader.app/Contents/MacOS/itms/java");
            search = search.with_home("/opt/homebrew/opt/java");
            if let Some(home) = home_directory() {
                search = search.with_homes_in(
                    home.join("Library")
                        .join("Java")
                        .join("JavaVirtualMachines"),
                );
                for directory in OFFICIAL_RUNTIME_DIRECTORIES {
                    search = search.with_official_root(home.join(directory));
                }
            }
        }
        OsType::Unknown => {}
    }

    if let Some(home) = home_directory() {
        search = search.with_homes_in(home.join(".jdks"));
    }

    search
}

/// Returns the Java executable of the running user, if the path of the user
/// names one.
pub fn find_java_in_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(java_executable_name()))
        .find(|executable| executable.is_file())
}

/// Reads the Java runtime the path of the user names.
///
/// The launcher uses the runtime when the settings cannot choose one and the
/// player turned the check of the runtime off. The version of the launcher
/// written in Java uses the runtime it runs on itself; the launcher written in
/// Rust runs on no Java runtime, so the runtime the player starts the other Java
/// programs with stands in for it.
///
/// Returns nothing when the path names no Java runtime, or when the runtime the
/// path names cannot be read.
pub fn default_java_runtime() -> Option<JavaRuntime> {
    let executable = find_java_in_path()?;
    JavaRuntime::detect(&executable, false).ok()
}

/// Returns whether a path is the `javapath` directory of the Oracle
/// installation, which does not point at a Java home.
fn is_oracle_javapath(path: &Path) -> bool {
    path.to_string_lossy()
        .to_ascii_lowercase()
        .contains("\\common files\\oracle\\java\\")
}

/// Returns the Java home of the running user, if the system tells it.
fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Returns the directories in a directory.
fn child_directories(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

/// Returns the executable of a Java home.
///
/// A macOS bundle keeps the home in `Contents/Home`, so the executable is
/// looked for there as well.
fn executable_in_home(home: &Path) -> Option<PathBuf> {
    let executable = home.join("bin").join(java_executable_name());
    if executable.is_file() {
        return Some(executable);
    }

    let executable = home
        .join("Contents")
        .join("Home")
        .join("bin")
        .join(java_executable_name());
    executable.is_file().then_some(executable)
}

/// Returns the executables of the runtimes the launcher manages.
///
/// The launcher keeps a runtime in `<root>/<platform>/<name>` and writes the
/// manifest of the runtime to `<root>/<platform>/<name>.json`; the profile of a
/// runtime without a manifest is not a runtime the launcher manages.
fn managed_executables(root: &Path) -> Vec<(PathBuf, bool)> {
    let mut executables = Vec::new();

    for platform in child_directories(root) {
        let Ok(entries) = std::fs::read_dir(&platform) else {
            continue;
        };

        for entry in entries.flatten() {
            let manifest = entry.path();
            if manifest
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("json")
            {
                continue;
            }
            let Some(name) = manifest.file_stem() else {
                continue;
            };
            if let Some(executable) = executable_in_home(&platform.join(name)) {
                executables.push((executable, true));
            }
        }
    }

    executables
}

/// Returns the executables of the runtimes a launcher of the game installed.
///
/// The launcher of the game keeps a runtime in
/// `<root>/<component>/<platform>/<component>`; the executable is the `java`
/// program of that directory, or the one of the macOS bundle in it.
fn official_executables(root: &Path) -> Vec<PathBuf> {
    let mut executables = Vec::new();

    for component in child_directories(root) {
        let Some(name) = component.file_name() else {
            continue;
        };
        for platform in child_directories(&component) {
            if let Some(executable) = executable_in_home(&platform.join(name)) {
                executables.push(executable);
            }
        }
    }

    executables
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::java::Platform;
    use crate::util::operating_system::{current_os, OsType};

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
    fn java_home(home: &Path, version: &str) {
        std::fs::create_dir_all(home.join("bin")).expect("the bin directory should be made");
        std::fs::write(home.join("release"), release(version))
            .expect("the release should be written");
        std::fs::write(
            home.join("bin").join(java_executable_name()),
            b"not a program",
        )
        .expect("the executable should be written");
    }

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("hmcl-rs-java-search-{name}-{}", std::process::id()))
    }

    #[test]
    fn the_homes_are_searched() {
        let directory = directory("homes");
        let _ = std::fs::remove_dir_all(&directory);
        java_home(&directory.join("jdk-17"), "17.0.9");
        java_home(&directory.join("jdk-8"), "1.8.0_462");
        // A directory that is not a Java home is skipped.
        std::fs::create_dir_all(directory.join("not-a-jdk")).expect("the directory should be made");

        let runtimes = JavaSearch::new().with_homes_in(&directory).search();

        assert_eq!(
            runtimes
                .iter()
                .map(|runtime| runtime.version())
                .collect::<Vec<_>>(),
            ["1.8.0_462", "17.0.9"]
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_runtime_the_player_disabled_is_skipped() {
        let directory = directory("disabled");
        let _ = std::fs::remove_dir_all(&directory);
        let home = directory.join("jdk-17");
        java_home(&home, "17.0.9");

        let executable = home.join("bin").join(java_executable_name());
        let runtimes = JavaSearch::new()
            .with_home(&home)
            .with_disabled(&executable)
            .search();

        assert!(runtimes.is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_runtimes_the_launcher_manages_are_found() {
        let directory = directory("managed");
        let _ = std::fs::remove_dir_all(&directory);
        let platform = Platform::current().to_string();
        let home = directory.join(&platform).join("mojang-java-runtime-gamma");
        java_home(&home, "17.0.9");
        std::fs::write(
            directory
                .join(&platform)
                .join("mojang-java-runtime-gamma.json"),
            b"{}",
        )
        .expect("the manifest should be written");

        // A runtime without a manifest is not found.
        java_home(&directory.join(&platform).join("not-managed"), "21.0.1");

        let runtimes = JavaSearch::new().with_managed_root(&directory).search();

        assert_eq!(runtimes.len(), 1);
        assert!(runtimes[0].is_managed());
        assert_eq!(runtimes[0].version(), "17.0.9");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_runtimes_a_launcher_of_the_game_installed_are_found() {
        let directory = directory("official");
        let _ = std::fs::remove_dir_all(&directory);
        let home = directory
            .join("java-runtime-gamma")
            .join("windows-x64")
            .join("java-runtime-gamma");
        java_home(&home, "17.0.8");

        let runtimes = JavaSearch::new().with_official_root(&directory).search();

        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0].version(), "17.0.8");
        assert!(!runtimes[0].is_managed());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_java_of_the_path_is_found() {
        let Some(executable) = find_java_in_path() else {
            return;
        };

        assert!(executable.is_file());
        assert_eq!(current_os(), Platform::current().os);
    }

    #[test]
    fn the_default_runtime_is_the_java_of_the_path() {
        let Some(executable) = find_java_in_path() else {
            return;
        };
        let Ok(expected) = JavaRuntime::detect(&executable, false) else {
            // The runtime the path names may have been built for another
            // machine, so the launcher cannot read it.
            return;
        };

        let runtime = default_java_runtime().expect("the runtime of the path should be read");

        assert_eq!(runtime, expected);
        assert!(!runtime.is_managed());
    }
}
