//! The version of Java a version of the game runs on.
//!
//! The versions of the game are written for a version of Java: the newer
//! versions need a newer Java, and the mod loaders of some versions only work
//! on the Java version they were written for. The launcher reads the version
//! the game records, the version the loaders need, and the version of each
//! runtime the machine has, and chooses the runtime that fits best.

use crate::download::game::InstanceManifest;
use crate::java::{JavaRuntime, Platform};
use crate::util::operating_system::{current_arch, current_os, Arch, OsType};
use crate::util::versioning::{GameVersionNumber, VersionNumber, VersionRange};

use super::component::{ComponentAnalyzer, GameComponentType, LAUNCH_WRAPPER_MAIN};

/// A version of Java the launcher downloads for the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameJavaVersion {
    /// The name of the component the runtime is downloaded as.
    pub component: &'static str,
    /// The major version of Java.
    pub major_version: i32,
}

impl GameJavaVersion {
    /// The Java 25 runtime of Mojang.
    pub const JAVA_25: Self = Self {
        component: "java-runtime-epsilon",
        major_version: 25,
    };

    /// The Java 21 runtime of Mojang.
    pub const JAVA_21: Self = Self {
        component: "java-runtime-delta",
        major_version: 21,
    };

    /// The Java 17 runtime of Mojang.
    pub const JAVA_17: Self = Self {
        component: "java-runtime-beta",
        major_version: 17,
    };

    /// The Java 16 runtime of Mojang.
    pub const JAVA_16: Self = Self {
        component: "java-runtime-alpha",
        major_version: 16,
    };

    /// The Java 8 runtime of Mojang.
    pub const JAVA_8: Self = Self {
        component: "jre-legacy",
        major_version: 8,
    };

    /// The newest Java runtime the launcher knows.
    pub const LATEST: Self = Self::JAVA_25;

    /// Returns the version of Java a version of the game needs.
    ///
    /// The versions of the game before the 1.13 run on any Java the launcher
    /// knows, so no version is needed. The versions the launcher does not know
    /// are read as the versions of the game they are named after.
    pub fn minimum_for(game_version: &GameVersionNumber) -> Option<Self> {
        if game_version >= &GameVersionNumber::as_game_version("26.1") {
            Some(Self::JAVA_25)
        } else if game_version >= &GameVersionNumber::as_game_version("1.20.5") {
            Some(Self::JAVA_21)
        } else if game_version >= &GameVersionNumber::as_game_version("1.18") {
            Some(Self::JAVA_17)
        } else if game_version >= &GameVersionNumber::as_game_version("1.17") {
            Some(Self::JAVA_16)
        } else if game_version >= &GameVersionNumber::as_game_version("1.13") {
            Some(Self::JAVA_8)
        } else {
            None
        }
    }

    /// Returns the version of Java a version of Cleanroom needs.
    ///
    /// Cleanroom 0.5 needs Java 25; the versions before it need Java 21.
    pub fn for_cleanroom(cleanroom_version: &str) -> Self {
        let version = cleanroom_version
            .strip_suffix("-alpha")
            .unwrap_or(cleanroom_version);

        if VersionNumber::parse(version) >= VersionNumber::parse("0.5.0") {
            Self::JAVA_25
        } else {
            Self::JAVA_21
        }
    }

    /// Returns the version of Java of a major version, if the launcher knows
    /// one.
    pub fn of(major_version: i32) -> Option<Self> {
        match major_version {
            8 => Some(Self::JAVA_8),
            16 => Some(Self::JAVA_16),
            17 => Some(Self::JAVA_17),
            21 => Some(Self::JAVA_21),
            25 => Some(Self::JAVA_25),
            _ => None,
        }
    }

    /// Returns the versions of Java the launcher downloads for a platform.
    ///
    /// The platforms the launcher does not download Java for have no versions;
    /// a platform that runs the programs of another architecture may still run
    /// the runtimes the launcher downloaded for that architecture.
    pub fn supported_versions(platform: Platform) -> Vec<Self> {
        match (platform.os, platform.arch) {
            (OsType::Windows, Arch::X86) => vec![Self::JAVA_8, Self::JAVA_16, Self::JAVA_17],
            (OsType::Linux, Arch::X86) => vec![Self::JAVA_8],
            (OsType::Windows | OsType::Linux | OsType::MacOs, Arch::X86_64) => {
                vec![
                    Self::JAVA_8,
                    Self::JAVA_16,
                    Self::JAVA_17,
                    Self::JAVA_21,
                    Self::JAVA_25,
                ]
            }
            (OsType::Windows | OsType::MacOs, Arch::Aarch64) => {
                vec![Self::JAVA_17, Self::JAVA_21, Self::JAVA_25]
            }
            _ => Vec::new(),
        }
    }
}

/// A rule about the version of Java a version of the game runs on.
///
/// A rule applies to the versions of the game and to the runtimes in a range of
/// versions; a runtime that does not fit a rule the launcher requires is not
/// used, and a runtime that does not fit a rule the launcher only suggests is
/// used only when no runtime fits every rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaVersionConstraint {
    /// The game runs on the version of Java Mojang records for it.
    Vanilla,
    /// The version records the version of Java it needs.
    GameJson,
    /// The versions of Forge before the 1.7.10 only work on Java 7.
    ModdedJava7,
    /// The versions of Forge between the 1.7.10 and the 1.16 only work on
    /// Java 8.
    ModdedJava8,
    /// The versions of Forge of the 1.17 only work on Java 16.
    ModdedJava16,
    /// The versions of Forge between the 1.18 and the 1.20.4 only work on
    /// Java 17.
    ModdedJava17,
    /// The versions of Forge since the 1.20.5 only work on Java 21.
    ModdedJava21,
    /// Cleanroom needs the version of Java it records.
    Cleanroom,
    /// The launch wrapper of the versions before the 1.13 only works on Java 8.
    LaunchWrapper,
    /// The versions since the 1.13 crash on the Java 8 before 1.8.0_51.
    VanillaJava8_51,
    /// The versions before the 1.13 cannot load their native libraries on the
    /// 64-bit Java of Linux.
    VanillaLinuxJava8,
    /// The versions before the 1.6 do not run on the ARM64 architecture of
    /// Windows and macOS.
    VanillaX86,
    /// The ModLauncher of the Forge versions of the 1.16 and the 1.17 crashes
    /// on the Java versions with the error JDK-8273826.
    Modlauncher8,
}

impl JavaVersionConstraint {
    /// The rules the launcher reads, in the order the version of the launcher
    /// written in Java reads them.
    pub const ALL: &'static [Self] = &[
        Self::Vanilla,
        Self::GameJson,
        Self::ModdedJava7,
        Self::ModdedJava8,
        Self::ModdedJava16,
        Self::ModdedJava17,
        Self::ModdedJava21,
        Self::Cleanroom,
        Self::LaunchWrapper,
        Self::VanillaJava8_51,
        Self::VanillaLinuxJava8,
        Self::VanillaX86,
        Self::Modlauncher8,
    ];

    /// Returns whether a runtime that does not fit the rule may not be used.
    pub fn is_mandatory(self) -> bool {
        matches!(
            self,
            Self::Vanilla
                | Self::GameJson
                | Self::Cleanroom
                | Self::LaunchWrapper
                | Self::VanillaLinuxJava8
        )
    }

    /// Returns whether the rule is about the version of the game.
    pub fn applies_to(
        self,
        game_version: &GameVersionNumber,
        manifest: Option<&InstanceManifest>,
        java: Option<&JavaRuntime>,
        analyzer: Option<&ComponentAnalyzer>,
    ) -> bool {
        self.game_version_range().contains(game_version)
            && self.applies_to_impl(game_version, manifest, java, analyzer)
    }

    /// Returns whether a Java runtime fits the rule.
    pub fn check_java(
        self,
        game_version: &GameVersionNumber,
        manifest: Option<&InstanceManifest>,
        java: &JavaRuntime,
        analyzer: Option<&ComponentAnalyzer>,
    ) -> bool {
        match self {
            Self::Vanilla => GameJavaVersion::minimum_for(game_version)
                .is_none_or(|minimum| java.parsed_version() >= minimum.major_version),
            Self::VanillaLinuxJava8 => {
                java.platform().arch != Arch::X86_64
                    || self
                        .java_version_range(manifest, analyzer)
                        .contains(java.version_number())
            }
            Self::VanillaX86 => matches!(java.platform().arch, Arch::X86 | Arch::X86_64),
            Self::Modlauncher8 => match java.parsed_version() {
                8 => java.version_number() < &VersionNumber::parse("1.8.0_321"),
                11 => java.version_number() < &VersionNumber::parse("11.0.14"),
                15 => java.version_number() < &VersionNumber::parse("15.0.6"),
                17 => java.version_number() < &VersionNumber::parse("17.0.2"),
                version if version > 17 => false,
                _ => true,
            },
            _ => self
                .java_version_range(manifest, analyzer)
                .contains(java.version_number()),
        }
    }

    /// Returns the versions of the game the rule is about.
    fn game_version_range(self) -> VersionRange<GameVersionNumber> {
        match self {
            Self::ModdedJava7 => game_at_most("1.7.2"),
            Self::ModdedJava8 => game_between("1.7.10", "1.16.999"),
            Self::ModdedJava16 => game_between("1.17", "1.17.999"),
            Self::ModdedJava17 => game_between("1.18", "1.20.4"),
            Self::ModdedJava21 => game_at_least("1.20.5"),
            Self::Cleanroom => game_between("1.12.2", "1.12.999"),
            Self::LaunchWrapper => game_at_most("1.12.999"),
            Self::VanillaLinuxJava8 => game_at_most("1.12.999"),
            Self::Modlauncher8 => game_between("1.16.3", "1.17.1"),
            _ => VersionRange::all(),
        }
    }

    /// Returns the versions of Java the rule is about.
    fn java_version_range(
        self,
        manifest: Option<&InstanceManifest>,
        analyzer: Option<&ComponentAnalyzer>,
    ) -> VersionRange<VersionNumber> {
        match self {
            Self::Vanilla | Self::VanillaX86 | Self::Modlauncher8 => VersionRange::all(),
            Self::GameJson => {
                let Some(major_version) = manifest
                    .and_then(|manifest| manifest.java_version.as_ref())
                    .and_then(|java_version| java_version.major_version)
                else {
                    return VersionRange::all();
                };

                if major_version >= 9 {
                    java_at_least(&major_version.to_string())
                } else {
                    java_at_least(&format!("1.{major_version}"))
                }
            }
            Self::ModdedJava7 => java_at_most("1.7.999"),
            Self::ModdedJava8 => java_between("1.8", "1.8.999"),
            Self::ModdedJava16 => java_between("16", "16.999"),
            Self::ModdedJava17 => java_between("17", "17.999"),
            Self::ModdedJava21 => java_between("21", "21.999"),
            Self::Cleanroom => {
                let Some(version) =
                    analyzer.and_then(|analyzer| analyzer.version(GameComponentType::Cleanroom))
                else {
                    return VersionRange::all();
                };

                java_at_least(
                    &GameJavaVersion::for_cleanroom(version)
                        .major_version
                        .to_string(),
                )
            }
            Self::LaunchWrapper | Self::VanillaLinuxJava8 => java_at_most("1.8.999"),
            Self::VanillaJava8_51 => java_at_least("1.8.0_51"),
        }
    }

    /// Returns whether the rule is about the version, with the rules of the
    /// version itself.
    fn applies_to_impl(
        self,
        game_version: &GameVersionNumber,
        manifest: Option<&InstanceManifest>,
        java: Option<&JavaRuntime>,
        analyzer: Option<&ComponentAnalyzer>,
    ) -> bool {
        match self {
            Self::Vanilla => manifest
                .and_then(|manifest| manifest.java_version.as_ref())
                .is_none(),
            Self::GameJson => {
                game_version >= &GameVersionNumber::as_game_version("1.7.10")
                    && manifest
                        .and_then(|manifest| manifest.java_version.as_ref())
                        .and_then(|java_version| java_version.major_version)
                        .is_some()
            }
            Self::ModdedJava7
            | Self::ModdedJava8
            | Self::ModdedJava16
            | Self::ModdedJava17
            | Self::ModdedJava21 => {
                analyzer.is_some_and(|analyzer| analyzer.has(GameComponentType::Forge))
                    && self.applies_with_manifest_requirement(manifest, analyzer)
            }
            Self::Cleanroom => {
                analyzer.is_some_and(|analyzer| analyzer.has(GameComponentType::Cleanroom))
                    && self.applies_with_manifest_requirement(manifest, analyzer)
            }
            Self::LaunchWrapper => {
                let Some(manifest) = manifest else {
                    return false;
                };

                self.applies_with_manifest_requirement(Some(manifest), analyzer)
                    && manifest.main_class.as_deref() == Some(LAUNCH_WRAPPER_MAIN)
                    && manifest.libraries.iter().any(|library| {
                        library.artifact().is_ok_and(|artifact| {
                            artifact.name == "launchwrapper"
                                && VersionNumber::parse(&artifact.version)
                                    < VersionNumber::parse("1.13")
                        })
                    })
            }
            Self::VanillaJava8_51 => true,
            Self::VanillaLinuxJava8 => {
                current_os() == OsType::Linux
                    && current_arch() == Arch::X86_64
                    && java.is_none_or(|java| java.platform().arch == Arch::X86_64)
                    && !analyzer.is_some_and(|analyzer| analyzer.has(GameComponentType::Cleanroom))
            }
            Self::VanillaX86 => {
                let Some(java) = java else {
                    return false;
                };

                java.platform().arch == Arch::Aarch64
                    && matches!(current_os(), OsType::Windows | OsType::MacOs)
                    && game_version < &GameVersionNumber::as_game_version("1.6")
            }
            Self::Modlauncher8 => {
                let (Some(manifest), Some(analyzer)) = (manifest, analyzer) else {
                    return false;
                };
                if java.is_none()
                    || !self.applies_with_manifest_requirement(Some(manifest), Some(analyzer))
                {
                    return false;
                }
                let Some(forge_version) = analyzer.version(GameComponentType::Forge) else {
                    return false;
                };
                let forge_version = VersionNumber::parse(forge_version);

                match game_version.value() {
                    "1.16.3" => forge_version >= VersionNumber::parse("34.1.27"),
                    "1.16.4" => true,
                    "1.16.5" => forge_version <= VersionNumber::parse("36.2.23"),
                    "1.17.1" => VersionRange::between(
                        VersionNumber::parse("37.0.60"),
                        VersionNumber::parse("37.0.75"),
                    )
                    .contains(&forge_version),
                    _ => false,
                }
            }
        }
    }

    /// Returns whether the version of the game may record a version of Java
    /// that the rule is about.
    ///
    /// A version that records a version of Java older than the rule does not
    /// give the rule to the runtime; the launcher then leaves the runtime to
    /// the version of the game.
    fn applies_with_manifest_requirement(
        self,
        manifest: Option<&InstanceManifest>,
        analyzer: Option<&ComponentAnalyzer>,
    ) -> bool {
        let Some(major_version) = manifest
            .and_then(|manifest| manifest.java_version.as_ref())
            .and_then(|java_version| java_version.major_version)
        else {
            return true;
        };

        let required = if major_version >= 9 {
            major_version.to_string()
        } else {
            format!("1.{major_version}")
        };
        let range = self.java_version_range(manifest, analyzer);

        range
            .maximum()
            .is_none_or(|maximum| maximum >= &VersionNumber::parse(&required))
    }
}

/// Returns a range of the versions of the game between two versions.
fn game_between(minimum: &str, maximum: &str) -> VersionRange<GameVersionNumber> {
    VersionRange::between(
        GameVersionNumber::as_game_version(minimum),
        GameVersionNumber::as_game_version(maximum),
    )
}

/// Returns a range of the versions of the game up to a version.
fn game_at_most(maximum: &str) -> VersionRange<GameVersionNumber> {
    VersionRange::at_most(GameVersionNumber::as_game_version(maximum))
}

/// Returns a range of the versions of the game since a version.
fn game_at_least(minimum: &str) -> VersionRange<GameVersionNumber> {
    VersionRange::at_least(GameVersionNumber::as_game_version(minimum))
}

/// Returns a range of the versions of Java between two versions.
fn java_between(minimum: &str, maximum: &str) -> VersionRange<VersionNumber> {
    VersionRange::between(VersionNumber::parse(minimum), VersionNumber::parse(maximum))
}

/// Returns a range of the versions of Java up to a version.
fn java_at_most(maximum: &str) -> VersionRange<VersionNumber> {
    VersionRange::at_most(VersionNumber::parse(maximum))
}

/// Returns a range of the versions of Java since a version.
fn java_at_least(minimum: &str) -> VersionRange<VersionNumber> {
    VersionRange::at_least(VersionNumber::parse(minimum))
}

/// Returns the Java runtime the version of the game fits best.
///
/// A runtime that does not fit every rule the launcher requires is not used; a
/// runtime that does not fit a rule the launcher only suggests is used only
/// when no runtime fits every rule. The runtimes the launcher manages are
/// preferred, and among the runtimes of the same version the newer build is
/// preferred.
///
/// The runtime has to be built for the architecture of the machine, except on
/// the ARM64 architecture of Windows and macOS, where the versions of the game
/// before the 1.6 only run on the x86 architecture.
pub fn find_suitable_java(
    runtimes: &[JavaRuntime],
    game_version: &GameVersionNumber,
    manifest: Option<&InstanceManifest>,
) -> Option<JavaRuntime> {
    let analyzer = manifest
        .and_then(|manifest| ComponentAnalyzer::analyze(manifest, Some(game_version.value())).ok());

    let force_x86 = current_arch() == Arch::Aarch64
        && matches!(current_os(), OsType::Windows | OsType::MacOs)
        && game_version < &GameVersionNumber::as_game_version("1.6");

    let mut mandatory: Option<JavaRuntime> = None;
    let mut suggested: Option<JavaRuntime> = None;

    for runtime in runtimes {
        if force_x86 {
            if !matches!(runtime.platform().arch, Arch::X86 | Arch::X86_64) {
                continue;
            }
        } else if runtime.platform().arch != current_arch() {
            continue;
        }

        let mut violation_mandatory = false;
        let mut violation_suggested = false;

        for constraint in JavaVersionConstraint::ALL {
            if constraint.applies_to(game_version, manifest, Some(runtime), analyzer.as_ref())
                && !constraint.check_java(game_version, manifest, runtime, analyzer.as_ref())
            {
                if constraint.is_mandatory() {
                    violation_mandatory = true;
                } else {
                    violation_suggested = true;
                }
            }
        }

        if !violation_mandatory {
            mandatory = choose_java(mandatory, runtime);

            if !violation_suggested {
                suggested = choose_java(suggested, runtime);
            }
        }
    }

    suggested.or(mandatory)
}

/// Returns the runtime the launcher prefers of two runtimes.
///
/// The older version of Java is preferred, because it is the version the game
/// is closer to; the newer build of the same version is preferred.
fn choose_java(current: Option<JavaRuntime>, candidate: &JavaRuntime) -> Option<JavaRuntime> {
    let Some(current) = current else {
        return Some(candidate.clone());
    };

    if current.parsed_version() != candidate.parsed_version() {
        if current.parsed_version() < candidate.parsed_version() {
            Some(current)
        } else {
            Some(candidate.clone())
        }
    } else if current.version_number() >= candidate.version_number() {
        Some(current)
    } else {
        Some(candidate.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{JavaVersion, Library};
    use crate::java::JavaInfo;
    use std::path::PathBuf;

    /// Returns a runtime of the current architecture with a version.
    fn runtime(version: &str, managed: bool) -> JavaRuntime {
        JavaRuntime::from_binary(
            PathBuf::from(format!("/java/{version}/bin/java")),
            JavaInfo::new(
                Platform::new(current_os(), current_arch()),
                version.to_string(),
                None,
            ),
            managed,
        )
    }

    /// Returns a version of the game the tests read.
    fn game(version: &str) -> GameVersionNumber {
        GameVersionNumber::as_game_version(version)
    }

    /// Returns a manifest that records the version of Java it needs.
    fn manifest_with_java(major_version: i32) -> InstanceManifest {
        InstanceManifest {
            java_version: Some(JavaVersion {
                component: None,
                major_version: Some(major_version),
            }),
            ..InstanceManifest::default()
        }
    }

    /// Returns a manifest of a version of Forge.
    fn forge_manifest(game_version: &str, forge_version: &str) -> InstanceManifest {
        InstanceManifest {
            libraries: vec![Library {
                name: format!("net.minecraftforge:forge:{game_version}-{forge_version}"),
                ..Library::default()
            }],
            ..InstanceManifest::default()
        }
    }

    #[test]
    fn the_version_of_java_of_a_version_of_the_game_is_read() {
        assert_eq!(GameJavaVersion::minimum_for(&game("1.12.2")), None);
        assert_eq!(
            GameJavaVersion::minimum_for(&game("1.13")),
            Some(GameJavaVersion::JAVA_8)
        );
        assert_eq!(
            GameJavaVersion::minimum_for(&game("1.17")),
            Some(GameJavaVersion::JAVA_16)
        );
        assert_eq!(
            GameJavaVersion::minimum_for(&game("1.20.4")),
            Some(GameJavaVersion::JAVA_17)
        );
        assert_eq!(
            GameJavaVersion::minimum_for(&game("1.20.5")),
            Some(GameJavaVersion::JAVA_21)
        );
        assert_eq!(
            GameJavaVersion::minimum_for(&game("1.21")),
            Some(GameJavaVersion::JAVA_21)
        );
        assert_eq!(
            GameJavaVersion::minimum_for(&game("26.1")),
            Some(GameJavaVersion::JAVA_25)
        );
    }

    #[test]
    fn the_version_of_java_of_cleanroom_is_read() {
        assert_eq!(
            GameJavaVersion::for_cleanroom("0.4.9"),
            GameJavaVersion::JAVA_21
        );
        assert_eq!(
            GameJavaVersion::for_cleanroom("0.4.9-alpha"),
            GameJavaVersion::JAVA_21
        );
        assert_eq!(
            GameJavaVersion::for_cleanroom("0.5.0"),
            GameJavaVersion::JAVA_25
        );
    }

    #[test]
    fn the_versions_of_java_of_a_platform_are_read() {
        assert_eq!(
            GameJavaVersion::supported_versions(Platform::new(OsType::Windows, Arch::X86_64)),
            [
                GameJavaVersion::JAVA_8,
                GameJavaVersion::JAVA_16,
                GameJavaVersion::JAVA_17,
                GameJavaVersion::JAVA_21,
                GameJavaVersion::JAVA_25
            ]
        );
        assert_eq!(
            GameJavaVersion::supported_versions(Platform::new(OsType::Linux, Arch::X86)),
            [GameJavaVersion::JAVA_8]
        );
        assert!(
            GameJavaVersion::supported_versions(Platform::new(OsType::Linux, Arch::Aarch64))
                .is_empty()
        );
        assert_eq!(GameJavaVersion::of(17), Some(GameJavaVersion::JAVA_17));
        assert_eq!(GameJavaVersion::of(19), None);
    }

    #[test]
    fn the_older_version_of_java_is_chosen_for_the_older_versions_of_the_game() {
        let runtimes = [runtime("1.8.0_462", false), runtime("17.0.9", false)];

        let java = find_suitable_java(&runtimes, &game("1.12.2"), None)
            .expect("a runtime should be chosen");
        assert_eq!(java.parsed_version(), 8);
    }

    #[test]
    fn the_version_the_game_needs_is_chosen() {
        let runtimes = [
            runtime("1.8.0_462", false),
            runtime("17.0.9", false),
            runtime("21.0.1", false),
        ];

        let java = find_suitable_java(&runtimes, &game("1.20.5"), None)
            .expect("a runtime should be chosen");
        assert_eq!(java.parsed_version(), 21);
    }

    #[test]
    fn the_version_the_game_records_is_chosen() {
        let manifest = manifest_with_java(17);
        let runtimes = [runtime("1.8.0_462", false), runtime("17.0.9", false)];

        let java = find_suitable_java(&runtimes, &game("1.20.1"), Some(&manifest))
            .expect("a runtime should be chosen");

        assert_eq!(java.parsed_version(), 17);
    }

    #[test]
    fn the_version_of_java_the_mod_loader_needs_is_suggested() {
        let manifest = forge_manifest("1.12.2", "14.23.5.2859");
        let runtimes = [runtime("1.8.0_462", false), runtime("17.0.9", false)];

        let java = find_suitable_java(&runtimes, &game("1.12.2"), Some(&manifest))
            .expect("a runtime should be chosen");

        // The Java 8 is only suggested for the mod loader, but the Java 8 is
        // also the older version, which the launcher prefers.
        assert_eq!(java.parsed_version(), 8);
    }

    #[test]
    fn the_mandatory_rule_of_the_launch_wrapper_is_read() {
        let manifest = InstanceManifest {
            main_class: Some(LAUNCH_WRAPPER_MAIN.to_string()),
            libraries: vec![Library {
                name: "net.minecraft:launchwrapper:1.12".to_string(),
                ..Library::default()
            }],
            ..InstanceManifest::default()
        };
        let analyzer = ComponentAnalyzer::analyze(&manifest, Some("1.12.2")).ok();
        let java_17 = runtime("17.0.9", false);
        let java_8 = runtime("1.8.0_462", false);

        assert!(JavaVersionConstraint::LaunchWrapper.applies_to(
            &game("1.12.2"),
            Some(&manifest),
            Some(&java_17),
            analyzer.as_ref()
        ));
        assert!(!JavaVersionConstraint::LaunchWrapper.check_java(
            &game("1.12.2"),
            Some(&manifest),
            &java_17,
            analyzer.as_ref()
        ));
        assert!(JavaVersionConstraint::LaunchWrapper.check_java(
            &game("1.12.2"),
            Some(&manifest),
            &java_8,
            analyzer.as_ref()
        ));
    }

    #[test]
    fn the_managed_runtime_is_preferred() {
        let runtimes = [runtime("17.0.9", true), runtime("17.0.9", false)];

        let java =
            find_suitable_java(&runtimes, &game("1.18"), None).expect("a runtime should be chosen");

        assert!(java.is_managed());
    }

    #[test]
    fn the_rule_of_the_modlauncher_is_read() {
        let manifest = forge_manifest("1.16.5", "36.2.20");
        let analyzer = ComponentAnalyzer::analyze(&manifest, Some("1.16.5")).ok();
        let java = runtime("17.0.2", false);

        assert!(JavaVersionConstraint::Modlauncher8.applies_to(
            &game("1.16.5"),
            Some(&manifest),
            Some(&java),
            analyzer.as_ref()
        ));
        assert!(!JavaVersionConstraint::Modlauncher8.check_java(
            &game("1.16.5"),
            Some(&manifest),
            &java,
            analyzer.as_ref()
        ));
    }

    #[test]
    fn a_machine_without_a_suitable_runtime_has_no_runtime() {
        // The 32-bit runtime does not run the 1.20.5 version on a 64-bit
        // machine, and the launcher does not choose a runtime of another
        // architecture.
        let mut java = runtime("21.0.1", false);
        java = JavaRuntime::from_binary(
            java.binary().to_path_buf(),
            JavaInfo::new(
                Platform::new(current_os(), Arch::X86),
                "21.0.1".to_string(),
                None,
            ),
            false,
        );

        if current_arch() != Arch::X86 {
            assert!(find_suitable_java(&[java], &game("1.20.5"), None).is_none());
        }
    }
}
