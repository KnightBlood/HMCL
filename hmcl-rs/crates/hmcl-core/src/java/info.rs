//! The information a Java runtime tells about itself.
//!
//! A Java home keeps a `release` file with the version, the vendor and the
//! platform the runtime was built for; a runtime the launcher cannot read the
//! file of is asked for the same information by running it.

use std::collections::HashMap;
use std::fmt;

use crate::util::operating_system::OsType;
use crate::util::versioning::VersionNumber;

use super::{parse_arch_name, parse_os_name, Platform};

/// The error of reading the information about a Java runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaInfoError {
    /// The file does not tell the version of Java.
    MissingVersion,
    /// The file names an operating system the launcher does not know, or names
    /// no operating system at all.
    UnknownOperatingSystem(String),
    /// The file names an architecture the launcher does not know, or names no
    /// architecture at all.
    UnknownArchitecture(String),
}

impl fmt::Display for JavaInfoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingVersion => {
                formatter.write_str("The Java runtime does not tell its version")
            }
            Self::UnknownOperatingSystem(name) => {
                write!(
                    formatter,
                    "The Java runtime names an unknown operating system: {name}"
                )
            }
            Self::UnknownArchitecture(name) => {
                write!(
                    formatter,
                    "The Java runtime names an unknown architecture: {name}"
                )
            }
        }
    }
}

impl std::error::Error for JavaInfoError {}

/// The information about a Java runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaInfo {
    /// The operating system and the architecture the runtime was built for.
    platform: Platform,
    /// The version of Java, as the runtime writes it.
    version: String,
    /// The organization that built the runtime, if it is known.
    vendor: Option<String>,
    /// The major version of Java, or `-1` when it is not known.
    parsed_version: i32,
    /// The version of Java as a number the launcher can compare.
    version_number: VersionNumber,
}

impl JavaInfo {
    /// Creates the information about a Java runtime.
    pub fn new(platform: Platform, version: String, vendor: Option<String>) -> Self {
        let parsed_version = Self::parse_version(&version);

        Self {
            platform,
            version_number: VersionNumber::parse(&version),
            version,
            vendor,
            parsed_version,
        }
    }

    /// Reads the version of Java, such as `8` for `1.8.0_462` and `17` for
    /// `17.0.9`.
    ///
    /// The version is the first number after the `1.` prefix of the old
    /// versions of Java; the version the launcher does not know is `-1`.
    pub fn parse_version(version: &str) -> i32 {
        let version = version.strip_prefix("1.").unwrap_or(version);
        let digits: String = version
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();

        if digits.is_empty() {
            -1
        } else {
            digits.parse().unwrap_or(-1)
        }
    }

    /// Reads the information from the `release` file of a Java home.
    ///
    /// The file holds lines of the form `KEY="value"`; the launcher reads the
    /// operating system, the architecture, the version and the vendor of the
    /// runtime. A runtime that names an operating system or an architecture
    /// the launcher does not know cannot be used, so reading the file is an
    /// error.
    pub fn from_release_file(contents: &str) -> Result<Self, JavaInfoError> {
        let properties = release_properties(contents);

        let os_name = properties.get("OS_NAME").map(String::as_str).unwrap_or("");
        let os_arch = properties.get("OS_ARCH").map(String::as_str).unwrap_or("");
        let vendor = properties
            .get("IMPLEMENTOR")
            .map(String::as_str)
            .filter(|vendor| !vendor.is_empty())
            .map(str::to_string);
        let version = properties
            .get("JAVA_VERSION")
            .map(String::as_str)
            .unwrap_or("");

        // The BSD port of OpenJDK names no operating system; the version of
        // the launcher written in Java reads it as FreeBSD, which this launcher
        // does not support.
        let os = parse_os_name(os_name);
        let arch = parse_arch_name(os_arch);

        if os == OsType::Unknown {
            return Err(JavaInfoError::UnknownOperatingSystem(os_name.to_string()));
        }
        if arch == crate::util::operating_system::Arch::Unknown {
            return Err(JavaInfoError::UnknownArchitecture(os_arch.to_string()));
        }
        if version.is_empty() {
            return Err(JavaInfoError::MissingVersion);
        }

        Ok(Self::new(
            Platform::new(os, arch),
            version.to_string(),
            vendor,
        ))
    }

    /// Reads the information from the settings a Java runtime writes when it
    /// is started with `-XshowSettings:properties -version`.
    ///
    /// The settings are written as lines of the form `java.version = 17.0.9`.
    /// The operating system of the runtime is the operating system of the
    /// machine, because the runtime tells the architecture it runs as rather
    /// than the operating system it was built for.
    pub fn from_properties(contents: &str) -> Result<Self, JavaInfoError> {
        let properties = settings_properties(contents);

        let version = properties
            .get("java.version")
            .map(String::as_str)
            .unwrap_or("");
        let vendor = properties
            .get("java.vendor")
            .map(String::as_str)
            .filter(|vendor| !vendor.is_empty())
            .map(str::to_string);
        let arch_name = properties.get("os.arch").map(String::as_str).unwrap_or("");
        let arch = parse_arch_name(arch_name);

        if arch == crate::util::operating_system::Arch::Unknown {
            return Err(JavaInfoError::UnknownArchitecture(arch_name.to_string()));
        }
        if version.is_empty() {
            return Err(JavaInfoError::MissingVersion);
        }

        Ok(Self::new(
            Platform::new(crate::util::operating_system::current_os(), arch),
            version.to_string(),
            vendor,
        ))
    }

    /// Returns the operating system and the architecture the runtime was built
    /// for.
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Returns the version of Java, as the runtime writes it.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the organization that built the runtime, if it is known.
    pub fn vendor(&self) -> Option<&str> {
        self.vendor.as_deref()
    }

    /// Returns the name of the vendor the launcher shows the player.
    ///
    /// The vendors write their names differently; the launcher writes the
    /// names of the vendors it knows in one way, so the player can tell the
    /// runtimes apart. A runtime that names no vendor returns nothing.
    pub fn normalized_vendor(&self) -> Option<String> {
        Self::normalize_vendor(self.vendor.as_deref())
    }

    /// Returns the major version of Java, or `-1` when it is not known.
    pub fn parsed_version(&self) -> i32 {
        self.parsed_version
    }

    /// Returns the version of Java as a number the launcher can compare.
    pub fn version_number(&self) -> &VersionNumber {
        &self.version_number
    }

    /// Writes the name of a vendor in the way the launcher shows it.
    ///
    /// The vendor of the `release` file that names no vendor is `N/A`, which
    /// is no vendor; the names the launcher knows are written in one way, so
    /// the player can tell the runtimes apart.
    pub fn normalize_vendor(vendor: Option<&str>) -> Option<String> {
        let vendor = vendor?;

        Some(
            match vendor {
                "N/A" => return None,
                "Oracle Corporation" => "Oracle",
                "Azul Systems, Inc." => "Azul",
                "IBM Corporation"
                | "International Business Machines Corporation"
                | "Eclipse OpenJ9" => "IBM",
                "Eclipse Adoptium" => "Adoptium",
                "Amazon.com Inc." => "Amazon",
                name => name,
            }
            .to_string(),
        )
    }
}

/// Reads the properties of the `release` file of a Java home.
///
/// The values of the file are quoted; the launcher reads the text between the
/// quotes. A line that is not a property is skipped.
fn release_properties(contents: &str) -> HashMap<String, String> {
    let mut properties = HashMap::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value)
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");

        properties.insert(key.to_string(), value);
    }

    properties
}

/// Reads the properties a Java runtime writes to its error output when it is
/// started with `-XshowSettings:properties -version`.
///
/// The properties are written with a few spaces of the indentation and are
/// separated from the value by ` = `. The lines that are not properties, such
/// as the name of the section, are skipped.
fn settings_properties(contents: &str) -> HashMap<String, String> {
    let mut properties = HashMap::new();

    for line in contents.lines() {
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        properties.insert(key.to_string(), value.trim().to_string());
    }

    properties
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::operating_system::Arch;

    /// Returns the release file of a runtime of the current platform.
    fn release(version: &str, vendor: &str) -> String {
        let current = Platform::current();
        format!(
            "IMPLEMENTOR=\"{vendor}\"\nJAVA_VERSION=\"{version}\"\nOS_ARCH=\"{}\"\nOS_NAME=\"{}\"\n",
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

    #[test]
    fn the_version_of_java_is_read() {
        assert_eq!(JavaInfo::parse_version("1.8.0_462"), 8);
        assert_eq!(JavaInfo::parse_version("1.8"), 8);
        assert_eq!(JavaInfo::parse_version("17.0.9"), 17);
        assert_eq!(JavaInfo::parse_version("21"), 21);
        assert_eq!(JavaInfo::parse_version("25.0.4.1"), 25);
        assert_eq!(JavaInfo::parse_version("17-ea"), 17);
        assert_eq!(JavaInfo::parse_version("unknown"), -1);
        assert_eq!(JavaInfo::parse_version(""), -1);
    }

    #[test]
    fn the_release_file_is_read() {
        let info = JavaInfo::from_release_file(
            "IMPLEMENTOR=\"Eclipse Adoptium\"\nJAVA_VERSION=\"17.0.9\"\nOS_ARCH=\"x86_64\"\nOS_NAME=\"Windows\"\n",
        )
        .expect("the release file should be read");

        assert_eq!(info.version(), "17.0.9");
        assert_eq!(info.parsed_version(), 17);
        assert_eq!(info.vendor(), Some("Eclipse Adoptium"));
        assert_eq!(info.platform().os, OsType::Windows);
        assert_eq!(info.platform().arch, Arch::X86_64);
    }

    #[test]
    fn the_vendor_of_the_release_file_is_not_required() {
        let info = JavaInfo::from_release_file(&release("17.0.9", "N/A"))
            .expect("the release file should be read");

        assert_eq!(info.vendor(), Some("N/A"));
        assert_eq!(info.normalized_vendor(), None);
    }

    #[test]
    fn a_release_file_of_an_unknown_platform_is_rejected() {
        assert_eq!(
            JavaInfo::from_release_file(
                "JAVA_VERSION=\"17\"\nOS_ARCH=\"riscv64\"\nOS_NAME=\"Windows\"\n",
            ),
            Err(JavaInfoError::UnknownArchitecture("riscv64".to_string()))
        );
        assert_eq!(
            JavaInfo::from_release_file(
                "JAVA_VERSION=\"17\"\nOS_ARCH=\"x86_64\"\nOS_NAME=\"FreeBSD\"\n",
            ),
            Err(JavaInfoError::UnknownOperatingSystem("FreeBSD".to_string()))
        );
        assert_eq!(
            JavaInfo::from_release_file("OS_ARCH=\"x86_64\"\nOS_NAME=\"Windows\"\n"),
            Err(JavaInfoError::MissingVersion)
        );
    }

    #[test]
    fn the_settings_of_a_runtime_are_read() {
        let info = JavaInfo::from_properties(
            "Property settings:\n    java.version = 17.0.9\n    java.vendor = Eclipse Adoptium\n    os.arch = amd64\n    os.name = Windows 11\n",
        )
        .expect("the settings should be read");

        assert_eq!(info.version(), "17.0.9");
        assert_eq!(info.vendor(), Some("Eclipse Adoptium"));
        assert_eq!(info.platform().arch, Arch::X86_64);
    }

    #[test]
    fn the_vendors_are_written_in_one_way() {
        assert_eq!(
            JavaInfo::normalize_vendor(Some("Oracle Corporation")),
            Some("Oracle".to_string())
        );
        assert_eq!(
            JavaInfo::normalize_vendor(Some("Azul Systems, Inc.")),
            Some("Azul".to_string())
        );
        assert_eq!(
            JavaInfo::normalize_vendor(Some("Eclipse Adoptium")),
            Some("Adoptium".to_string())
        );
        assert_eq!(
            JavaInfo::normalize_vendor(Some("Amazon.com Inc.")),
            Some("Amazon".to_string())
        );
        assert_eq!(
            JavaInfo::normalize_vendor(Some("Eclipse OpenJ9")),
            Some("IBM".to_string())
        );
        assert_eq!(
            JavaInfo::normalize_vendor(Some("BellSoft")),
            Some("BellSoft".to_string())
        );
        assert_eq!(JavaInfo::normalize_vendor(Some("N/A")), None);
        assert_eq!(JavaInfo::normalize_vendor(None), None);
    }

    #[test]
    fn the_version_numbers_of_the_runtimes_are_compared() {
        let newer = JavaInfo::new(Platform::current(), "21.0.1".to_string(), None);
        let older = JavaInfo::new(Platform::current(), "17.0.9".to_string(), None);

        assert!(newer.version_number() > older.version_number());
        assert_eq!(newer.parsed_version(), 21);
        assert_eq!(older.parsed_version(), 17);
    }
}
