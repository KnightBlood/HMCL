//! The Java runtimes the game runs on.
//!
//! The game is written in Java, so the launcher needs a Java runtime to start
//! it. The launcher finds the runtimes the machine has, reads the version and
//! the architecture of each of them, and lets the player choose the runtime the
//! game runs on.
//!
//! The launcher does not run the game on the Java runtime the launcher itself
//! runs on: the game needs the Java version the version of the game was made
//! for.

pub mod discovery;
pub mod info;
pub mod runtime;

use std::fmt;

use crate::util::operating_system::{current_arch, current_os, Arch, OsType};

pub use discovery::{default_java_runtime, default_search, JavaSearch};
pub use info::{JavaInfo, JavaInfoError};
pub use runtime::{JavaRuntime, JavaRuntimeError};

/// The operating system and the architecture a Java runtime was built for.
///
/// A runtime built for another operating system cannot run the game on this
/// machine; a runtime built for another architecture may run the game when the
/// system translates the programs of that architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    /// The operating system the runtime was built for.
    pub os: OsType,
    /// The architecture the runtime was built for.
    pub arch: Arch,
}

impl Platform {
    /// Creates a platform from the operating system and the architecture.
    pub const fn new(os: OsType, arch: Arch) -> Self {
        Self { os, arch }
    }

    /// Returns the platform the launcher runs on.
    pub const fn current() -> Self {
        Self::new(current_os(), current_arch())
    }

    /// Creates a platform from the names the release file of a Java runtime
    /// uses.
    ///
    /// A name the launcher does not know becomes [`OsType::Unknown`] or
    /// [`Arch::Unknown`], so the runtime is not used.
    pub fn parse(os_name: &str, arch_name: &str) -> Self {
        Self::new(parse_os_name(os_name), parse_arch_name(arch_name))
    }

    /// Returns whether the runtime can run on this machine.
    ///
    /// The runtime has to be built for the operating system of the machine; a
    /// 32-bit runtime runs on a 64-bit Windows or Linux, and the systems that
    /// translate the programs of another architecture run the runtimes of that
    /// architecture as well.
    pub fn is_compatible_with_system(self) -> bool {
        if self.os != current_os() {
            return false;
        }

        let system_arch = current_arch();
        if self.arch == system_arch {
            return true;
        }

        match (self.os, system_arch) {
            // Windows and Linux of the x86-64 architecture run 32-bit programs.
            (OsType::Windows | OsType::Linux, Arch::X86_64) => self.arch == Arch::X86,
            // Windows of the ARM architecture translates the x86-64 and the
            // x86 programs.
            (OsType::Windows, Arch::Aarch64) => matches!(self.arch, Arch::X86_64 | Arch::X86),
            // macOS of the ARM architecture translates the x86-64 programs.
            (OsType::MacOs, Arch::Aarch64) => self.arch == Arch::X86_64,
            _ => false,
        }
    }

    /// Returns the amount of the bits the addresses of the runtime have, or
    /// nothing when the architecture is not known.
    pub const fn bits(self) -> Option<u32> {
        match self.arch {
            Arch::X86_64 | Arch::Aarch64 => Some(64),
            Arch::X86 | Arch::Arm => Some(32),
            Arch::Unknown => None,
        }
    }
}

impl fmt::Display for Platform {
    /// Writes the platform the way the directories of the Java runtimes the
    /// launcher manages are named, such as `windows-x86-64`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}-{}",
            checked_os_name(self.os),
            checked_arch_name(self.arch)
        )
    }
}

/// Returns the name of an operating system the directories of the launcher use.
const fn checked_os_name(os: OsType) -> &'static str {
    match os {
        OsType::Windows => "windows",
        OsType::Linux => "linux",
        OsType::MacOs => "macos",
        OsType::Unknown => "unknown",
    }
}

/// Returns the name of an architecture the directories of the launcher use.
const fn checked_arch_name(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "x86-64",
        Arch::X86 => "x86",
        Arch::Aarch64 => "arm64",
        Arch::Arm => "arm32",
        Arch::Unknown => "unknown",
    }
}

/// Returns the name of the executable of a Java runtime.
///
/// The name is the one the release file of a runtime names the executable by,
/// which is the name the launcher looks for in the `bin` directory of a Java
/// home.
pub const fn java_executable_name() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

/// Returns the name of the compiler of a Java runtime.
///
/// A Java runtime that has a compiler is a JDK; the launcher only needs the
/// compiler to show the player whether the runtime is a JDK or a JRE.
pub const fn javac_executable_name() -> &'static str {
    if cfg!(windows) {
        "javac.exe"
    } else {
        "javac"
    }
}

/// Returns the operating system a name of the release file of a Java runtime
/// names.
///
/// The names are read the way the version of the launcher written in Java reads
/// them: a name that holds `mac` or `darwin` is macOS, a name that holds `win`
/// is Windows, and a name that holds `linux` is Linux. A name the launcher does
/// not know becomes [`OsType::Unknown`].
pub fn parse_os_name(name: &str) -> OsType {
    let name = name.trim().to_ascii_lowercase();

    if name.contains("mac") || name.contains("darwin") || name.contains("osx") {
        OsType::MacOs
    } else if name.contains("win") {
        OsType::Windows
    } else if name.contains("solaris")
        || name.contains("linux")
        || name.contains("unix")
        || name.contains("sunos")
    {
        OsType::Linux
    } else {
        OsType::Unknown
    }
}

/// Returns the architecture a name of the release file of a Java runtime names.
///
/// The release files name the architecture of a runtime the way the
/// architecture calls itself, so the launcher reads the names of the systems
/// that build Java. A name the launcher does not know becomes
/// [`Arch::Unknown`].
pub fn parse_arch_name(name: &str) -> Arch {
    match name.trim().to_ascii_lowercase().as_str() {
        "x8664" | "x86-64" | "x86_64" | "amd64" | "ia32e" | "em64t" | "x64" | "intel64" => {
            Arch::X86_64
        }
        "x8632" | "x86-32" | "x86_32" | "x86" | "i86pc" | "i386" | "i486" | "i586" | "i686"
        | "ia32" | "x32" => Arch::X86,
        "arm64" | "aarch64" => Arch::Aarch64,
        "arm" | "arm32" => Arch::Arm,
        _ => Arch::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_of_the_systems_are_read() {
        assert_eq!(parse_os_name("Windows"), OsType::Windows);
        assert_eq!(parse_os_name("Windows 11"), OsType::Windows);
        assert_eq!(parse_os_name("Linux"), OsType::Linux);
        assert_eq!(parse_os_name("Darwin"), OsType::MacOs);
        assert_eq!(parse_os_name("Mac OS X"), OsType::MacOs);
        assert_eq!(parse_os_name("FreeBSD"), OsType::Unknown);
        assert_eq!(parse_os_name(""), OsType::Unknown);
    }

    #[test]
    fn the_names_of_the_architectures_are_read() {
        assert_eq!(parse_arch_name("x86_64"), Arch::X86_64);
        assert_eq!(parse_arch_name("amd64"), Arch::X86_64);
        assert_eq!(parse_arch_name("AMD64"), Arch::X86_64);
        assert_eq!(parse_arch_name("x86"), Arch::X86);
        assert_eq!(parse_arch_name("i386"), Arch::X86);
        assert_eq!(parse_arch_name("aarch64"), Arch::Aarch64);
        assert_eq!(parse_arch_name("arm64"), Arch::Aarch64);
        assert_eq!(parse_arch_name("arm"), Arch::Arm);
        assert_eq!(parse_arch_name("riscv64"), Arch::Unknown);
    }

    #[test]
    fn a_runtime_of_the_current_platform_runs() {
        assert!(Platform::current().is_compatible_with_system());
    }

    #[test]
    fn a_runtime_of_another_operating_system_does_not_run() {
        let other = match current_os() {
            OsType::Windows => OsType::Linux,
            _ => OsType::Windows,
        };
        assert!(!Platform::new(other, current_arch()).is_compatible_with_system());
    }

    #[test]
    fn the_platform_is_written_as_the_names_of_the_platforms_are() {
        assert_eq!(
            Platform::new(OsType::Windows, Arch::X86_64).to_string(),
            "windows-x86-64"
        );
        assert_eq!(
            Platform::new(OsType::Linux, Arch::X86).to_string(),
            "linux-x86"
        );
        assert_eq!(
            Platform::new(OsType::MacOs, Arch::Aarch64).to_string(),
            "macos-arm64"
        );
    }

    #[test]
    fn the_amount_of_the_bits_is_read() {
        assert_eq!(
            Platform::new(OsType::Windows, Arch::X86_64).bits(),
            Some(64)
        );
        assert_eq!(Platform::new(OsType::Linux, Arch::X86).bits(), Some(32));
        assert_eq!(Platform::new(OsType::Linux, Arch::Unknown).bits(), None);
    }
}
