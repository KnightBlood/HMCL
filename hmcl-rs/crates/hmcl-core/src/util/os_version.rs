//! The version of the operating system the launcher runs on.
//!
//! The launcher reads the version of the operating system to tell the player
//! about the systems it was not tested on, and to refuse to start the game on
//! a system that is too old.

use std::cmp::Ordering;
use std::fmt;

use crate::util::operating_system::OsType;
use crate::util::versioning::VersionNumber;

/// The version of an operating system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsVersion {
    /// The version of Microsoft Windows, told apart by the numbers of the
    /// version.
    Windows(WindowsVersion),
    /// The version of another operating system, told apart by a version
    /// number.
    Generic(GenericVersion),
}

impl OsVersion {
    /// Windows 2000.
    pub const WINDOWS_2000: Self = Self::Windows(WindowsVersion::WINDOWS_2000);
    /// Windows XP.
    pub const WINDOWS_XP: Self = Self::Windows(WindowsVersion::WINDOWS_XP);
    /// Windows Vista.
    pub const WINDOWS_VISTA: Self = Self::Windows(WindowsVersion::WINDOWS_VISTA);
    /// Windows 7.
    pub const WINDOWS_7: Self = Self::Windows(WindowsVersion::WINDOWS_7);
    /// Windows 8.
    pub const WINDOWS_8: Self = Self::Windows(WindowsVersion::WINDOWS_8);
    /// Windows 8.1.
    pub const WINDOWS_8_1: Self = Self::Windows(WindowsVersion::WINDOWS_8_1);
    /// Windows 10.
    pub const WINDOWS_10: Self = Self::Windows(WindowsVersion::WINDOWS_10);
    /// Windows 11.
    pub const WINDOWS_11: Self = Self::Windows(WindowsVersion::WINDOWS_11);

    /// Reads the version of an operating system.
    ///
    /// Windows is told apart by the numbers of its version, so a version of
    /// Windows is read with [`WindowsVersion::parse`]; every other operating
    /// system is read as a version number.
    pub fn of(os: OsType, version: &str) -> Self {
        if os == OsType::Windows {
            Self::Windows(WindowsVersion::parse(version))
        } else {
            Self::Generic(GenericVersion::new(os, VersionNumber::parse(version)))
        }
    }

    /// Returns the operating system the version belongs to.
    pub fn operating_system(&self) -> OsType {
        match self {
            Self::Windows(_) => OsType::Windows,
            Self::Generic(generic) => generic.os,
        }
    }

    /// Returns the text of the version.
    ///
    /// The text of a version of Windows is the text it was read from, so the
    /// parts of the text the launcher does not read are kept; the text of a
    /// version of another operating system is the text of its version number.
    pub fn version(&self) -> String {
        match self {
            Self::Windows(windows) => windows.version(),
            Self::Generic(generic) => generic.version.value().to_string(),
        }
    }

    /// Returns whether the operating system is the same as the one of the
    /// other version and its version is not lower.
    ///
    /// The versions of different operating systems are never told apart, so
    /// the version of Linux is never at least the version of Windows.
    pub fn is_at_least(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Windows(windows), Self::Windows(other_windows)) => windows >= other_windows,
            (Self::Generic(generic), Self::Generic(other_generic)) => {
                generic.os == other_generic.os && generic.version >= other_generic.version
            }
            _ => false,
        }
    }
}

impl fmt::Display for OsVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.version())
    }
}

/// The version of Microsoft Windows, told apart by the numbers of the
/// version.
///
/// The numbers of the version are compared, so Windows 10 and Windows 11,
/// which share the numbers of the major and the minor version, are told apart
/// by the number of the build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsVersion {
    /// The number of the major version.
    pub major: u32,
    /// The number of the minor version.
    pub minor: u32,
    /// The number of the build.
    pub build: u32,
    /// The number of the revision.
    pub revision: u32,
    /// The text the version was read from.
    version: String,
}

impl WindowsVersion {
    /// Windows 2000.
    pub const WINDOWS_2000: Self = Self::new(5, 0, 0, 0);
    /// Windows XP.
    pub const WINDOWS_XP: Self = Self::new(5, 1, 0, 0);
    /// Windows Vista.
    pub const WINDOWS_VISTA: Self = Self::new(6, 0, 0, 0);
    /// Windows 7.
    pub const WINDOWS_7: Self = Self::new(6, 1, 0, 0);
    /// Windows 8.
    pub const WINDOWS_8: Self = Self::new(6, 2, 0, 0);
    /// Windows 8.1.
    pub const WINDOWS_8_1: Self = Self::new(6, 3, 0, 0);
    /// Windows 10.
    pub const WINDOWS_10: Self = Self::new(10, 0, 0, 0);
    /// Windows 11, told apart from Windows 10 by the number of the build.
    pub const WINDOWS_11: Self = Self::new(10, 0, 22_000, 0);

    /// Creates a version from the numbers of the version.
    ///
    /// The text of the version is written by [`WindowsVersion::version`] from
    /// the numbers, because the text cannot be written where a constant is
    /// declared.
    const fn new(major: u32, minor: u32, build: u32, revision: u32) -> Self {
        Self {
            major,
            minor,
            build,
            revision,
            version: String::new(),
        }
    }

    /// Returns a version whose text is written from the numbers.
    ///
    /// The numbers that are zero are left out, so the version `10.0` is not
    /// written as `10.0.0.0`.
    pub fn of_numbers(major: u32, minor: u32, build: u32, revision: u32) -> Self {
        let mut version = format!("{major}.{minor}");
        if build > 0 || revision > 0 {
            version.push_str(&format!(".{build}"));
            if revision > 0 {
                version.push_str(&format!(".{revision}"));
            }
        }

        Self {
            major,
            minor,
            build,
            revision,
            version,
        }
    }

    /// Returns the text the version was read from.
    ///
    /// The text of a version that was created from its numbers, such as
    /// [`WindowsVersion::WINDOWS_7`], is written from the numbers; the numbers
    /// that are zero are left out, so `10.0` is not written as `10.0.0.0`.
    pub fn version(&self) -> String {
        if self.version.is_empty() {
            Self::of_numbers(self.major, self.minor, self.build, self.revision).version
        } else {
            self.version.clone()
        }
    }

    /// Reads a version of Windows.
    ///
    /// The numbers are read from the start of the text, so the text the
    /// launcher does not read, such as the name of the edition, is kept; a
    /// text without numbers of the version is read as the version `0.0.0.0`.
    pub fn parse(version: &str) -> Self {
        let mut numbers = [0u32; 4];
        let mut count = 0usize;

        // The text is split at the dots; the first four parts that consist of
        // numbers are read, the rest of the text is ignored.
        for part in version.split('.') {
            if count == 4 {
                break;
            }

            // A part such as `26120-unknown` holds the number of the build
            // and the text the launcher does not read.
            let digits: String = part
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect();

            let Ok(number) = digits.parse::<u32>() else {
                break;
            };

            numbers[count] = number;
            count += 1;
        }

        if count == 0 {
            // The text holds no numbers of the version.
            return Self {
                major: 0,
                minor: 0,
                build: 0,
                revision: 0,
                version: version.to_string(),
            };
        }

        Self {
            major: numbers[0],
            minor: numbers[1],
            build: numbers[2],
            revision: numbers[3],
            version: version.to_string(),
        }
    }
}

impl PartialOrd for WindowsVersion {
    /// Compares the numbers of the version, not the text it was read from.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WindowsVersion {
    /// Compares the numbers of the version, not the text it was read from.
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.build.cmp(&other.build))
            .then_with(|| self.revision.cmp(&other.revision))
    }
}

impl fmt::Display for WindowsVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.version)
    }
}

/// The version of an operating system that is not Windows.
///
/// The version is compared as a version number, so `1.0` and `1` are the same
/// version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericVersion {
    /// The operating system the version belongs to.
    pub os: OsType,
    /// The version number.
    pub version: VersionNumber,
}

impl GenericVersion {
    /// Creates a version of an operating system.
    ///
    /// # Panics
    ///
    /// Panics when the operating system is Windows; the version of Windows is
    /// read with [`WindowsVersion::parse`], because the numbers of a version
    /// of Windows are compared, not the version number.
    pub fn new(os: OsType, version: VersionNumber) -> Self {
        assert!(
            os != OsType::Windows,
            "the version of Windows is not a generic version"
        );

        Self { os, version }
    }

    /// Returns the text of the version.
    pub fn version(&self) -> String {
        self.version.value().to_string()
    }
}

impl fmt::Display for GenericVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_of_windows_is_read() {
        // The numbers of the version are compared, so the text the version was
        // read from is not told apart.
        let windows_7 = OsVersion::of(OsType::Windows, "6.1");
        assert_eq!(windows_7.operating_system(), OsType::Windows);
        assert!(windows_7.is_at_least(&OsVersion::WINDOWS_7));
        assert_eq!(windows_7.version(), "6.1");

        let windows_10 = OsVersion::of(OsType::Windows, "10.0");
        assert!(windows_10.is_at_least(&OsVersion::WINDOWS_10));
        assert_eq!(windows_10.version(), "10.0");

        assert!(OsVersion::of(OsType::Windows, "10.0.26120")
            .is_at_least(&OsVersion::of(OsType::Windows, "10.0.26120-unknown")));
        assert!(OsVersion::of(OsType::Windows, "10.0.26120.3964")
            .is_at_least(&OsVersion::of(OsType::Windows, "10.0.26120.3964-unknown")));
    }

    #[test]
    fn a_version_of_windows_without_numbers_is_the_oldest_one() {
        let version = WindowsVersion::parse("unknown");
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 0);
        assert_eq!(version.build, 0);
        assert_eq!(version.revision, 0);
        assert_eq!(version.version(), "unknown");
    }

    #[test]
    fn the_numbers_of_a_version_are_written() {
        assert_eq!(WindowsVersion::of_numbers(6, 1, 0, 0).version(), "6.1");
        assert_eq!(
            WindowsVersion::of_numbers(10, 0, 22_000, 0).version(),
            "10.0.22000"
        );
        assert_eq!(
            WindowsVersion::of_numbers(10, 0, 26_120, 3_964).version(),
            "10.0.26120.3964"
        );
    }

    #[test]
    fn the_versions_of_windows_are_compared() {
        assert!(OsVersion::WINDOWS_10.is_at_least(&OsVersion::WINDOWS_7));
        assert!(OsVersion::WINDOWS_10.is_at_least(&OsVersion::WINDOWS_10));
        assert!(!OsVersion::WINDOWS_10.is_at_least(&OsVersion::WINDOWS_11));
        assert!(OsVersion::WINDOWS_11.is_at_least(&OsVersion::WINDOWS_10));
    }

    #[test]
    fn the_versions_of_other_systems_are_compared() {
        let linux_4 = OsVersion::of(OsType::Linux, "4.0");
        let linux_3 = OsVersion::of(OsType::Linux, "3.0");
        let linux_5 = OsVersion::of(OsType::Linux, "5.0");

        assert!(linux_4.is_at_least(&linux_4));
        assert!(linux_4.is_at_least(&linux_3));
        assert!(!linux_4.is_at_least(&linux_5));
    }

    #[test]
    fn the_versions_of_different_systems_are_not_compared() {
        let linux = OsVersion::of(OsType::Linux, "4.0");
        let windows = OsVersion::of(OsType::Windows, "4.0");
        let macos = OsVersion::of(OsType::MacOs, "4.0");

        assert!(!windows.is_at_least(&linux));
        assert!(!linux.is_at_least(&windows));
        assert!(!macos.is_at_least(&linux));
    }

    #[test]
    fn the_version_of_windows_is_not_a_generic_version() {
        assert!(std::panic::catch_unwind(|| {
            GenericVersion::new(OsType::Windows, VersionNumber::parse("4.0"))
        })
        .is_err());
    }

    #[test]
    fn the_version_tells_its_operating_system() {
        assert_eq!(OsVersion::WINDOWS_7.operating_system(), OsType::Windows);
        assert_eq!(
            OsVersion::of(OsType::Linux, "4.0").operating_system(),
            OsType::Linux
        );
        assert_eq!(
            OsVersion::of(OsType::MacOs, "14.0").operating_system(),
            OsType::MacOs
        );
    }

    #[test]
    fn the_text_of_a_generic_version_is_its_own_text() {
        assert_eq!(OsVersion::of(OsType::Linux, "4.0").version(), "4.0");
        assert_eq!(OsVersion::of(OsType::Linux, "4").version(), "4");
        assert_eq!(OsVersion::of(OsType::Linux, "4.0.0").version(), "4.0.0");
    }

    #[test]
    fn the_text_of_a_version_is_written() {
        assert_eq!(OsVersion::WINDOWS_7.to_string(), "6.1");
        assert_eq!(OsVersion::of(OsType::Linux, "4.0").to_string(), "4.0");
    }
}
