//! The version numbers of Minecraft.
//!
//! The launcher shows the versions of Minecraft in several places: the list of
//! the downloads, the list of the instances and the checks that decide
//! whether a version supports a feature. The numbers of the versions are
//! therefore compared with the rules of this module, which follow the Java
//! implementation of HMCL.
//!
//! The numbers come in four shapes:
//!
//! * the old versions, such as `b1.7.3` and `rd-132211`,
//! * the releases, such as `1.20.1` and `1.21-pre2`,
//! * the snapshots, such as `23w42a`,
//! * the special versions, such as `1.14_combat-212796`, which are the
//!   versions of the experiments and of the April Fools snapshots.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, OnceLock};

use super::version_number::VersionNumber;

/// The list of the versions of Minecraft, in the order they were released.
const VERSIONS_TXT: &str = include_str!("../../../resources/game/versions.txt");

/// The aliases of the special versions.
const VERSION_ALIAS_CSV: &str = include_str!("../../../resources/game/version-alias.csv");

/// The first major version that uses the year as the major version.
const MINIMUM_YEAR_MAJOR_VERSION: i32 = 26;

/// The groups of the versions, in the order they follow each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum VersionType {
    /// The versions before the first public release, such as `rd-132211`.
    PreClassic,
    /// The versions of the first public release, such as `c0.30`.
    Classic,
    /// The versions of the first development period, such as `in-0.30`.
    Indev,
    /// The versions of the second development period, such as `inf-0.31`.
    Infdev,
    /// The versions of the alpha period, such as `a1.2.6`.
    Alpha,
    /// The versions of the beta period, such as `b1.7.3`.
    Beta,
    /// The versions that are released today, such as `1.20.1` and `23w42a`.
    New,
}

/// The kind of a release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReleaseType {
    /// A release that is not known yet.
    Unknown,
    /// A snapshot of the upcoming release.
    Snapshot,
    /// A pre-release of the upcoming release.
    PreRelease,
    /// A release candidate of the upcoming release.
    ReleaseCandidate,
    /// A finished release.
    Ga,
}

impl ReleaseType {
    /// Returns the infix the versions of the first generation use.
    const fn legacy_infix(self) -> &'static str {
        match self {
            Self::Unknown | Self::Ga => "",
            Self::Snapshot => "-snapshot-",
            Self::PreRelease => "-pre",
            Self::ReleaseCandidate => "-rc",
        }
    }

    /// Returns the infix the versions of the second generation use.
    const fn new_infix(self) -> &'static str {
        match self {
            Self::Unknown | Self::Ga => "",
            Self::Snapshot => "-snapshot-",
            Self::PreRelease => "-pre-",
            Self::ReleaseCandidate => "-rc-",
        }
    }
}

/// The additional information a version carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Additional {
    /// The version is the ordinary one.
    None,
    /// The version is the unobfuscated one.
    Unobfuscated,
}

impl Additional {
    /// Returns the suffix the version carries.
    const fn suffix(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Unobfuscated => "_unobfuscated",
        }
    }
}

/// A version of the first generations, such as `b1.7.3`.
#[derive(Debug, Clone)]
pub struct OldVersion {
    /// The text of the version.
    value: String,
    /// The group the version belongs to.
    version_type: VersionType,
    /// The number that follows the prefix of the version.
    version_number: VersionNumber,
}

impl OldVersion {
    /// Parses a version of the first generations.
    fn parse(value: &str) -> Option<Self> {
        if value.is_empty() {
            return None;
        }

        let (version_type, prefix_length) = match value.as_bytes()[0] {
            b'r' => {
                if !value.starts_with("rd-") {
                    return None;
                }
                (VersionType::PreClassic, 3)
            }
            b'i' => {
                if value.starts_with("inf-") {
                    (VersionType::Infdev, 4)
                } else if value.starts_with("in-") {
                    (VersionType::Indev, 3)
                } else {
                    return None;
                }
            }
            b'a' => (VersionType::Alpha, 1),
            b'b' => (VersionType::Beta, 1),
            b'c' => (VersionType::Classic, 1),
            _ => return None,
        };

        let rest = &value[prefix_length..];
        let first = rest.chars().next()?;
        if !first.is_ascii_digit() {
            return None;
        }

        Some(Self {
            value: value.to_string(),
            version_type,
            version_number: VersionNumber::parse(rest),
        })
    }
}

/// A release of Minecraft, such as `1.20.1` or `1.21-pre2`.
#[derive(Debug, Clone)]
pub struct ReleaseVersion {
    /// The text of the version.
    value: String,
    /// The normalized text of the version.
    normalized: String,
    /// The major version.
    major: i32,
    /// The minor version.
    minor: i32,
    /// The patch version.
    patch: i32,
    /// The kind of the release.
    release_type: ReleaseType,
    /// The number of the early access version.
    ea_version: VersionNumber,
    /// The additional information the version carries.
    additional: Additional,
}

/// Reads the part of a release that follows its numbers.
///
/// The part tells whether the release is finished and whether it is a
/// preview of a release; the third value tells whether the text of the
/// version has to be brought into the normal form.
fn parse_suffix(
    suffix: &str,
    is_legacy_release: bool,
) -> Option<(ReleaseType, VersionNumber, bool)> {
    if suffix.is_empty() {
        return Some((ReleaseType::Ga, VersionNumber::parse("0"), false));
    }

    if let Some(rest) = suffix.strip_prefix("-snapshot-") {
        return Some((ReleaseType::Snapshot, VersionNumber::parse(rest), false));
    }
    if let Some(rest) = suffix.strip_prefix(" Snapshot ") {
        return Some((ReleaseType::Snapshot, VersionNumber::parse(rest), true));
    }
    if let Some(rest) = suffix.strip_prefix("-pre-") {
        return Some((
            ReleaseType::PreRelease,
            VersionNumber::parse(rest),
            is_legacy_release,
        ));
    }
    if let Some(rest) = suffix.strip_prefix("-pre") {
        return Some((
            ReleaseType::PreRelease,
            VersionNumber::parse(rest),
            !is_legacy_release,
        ));
    }
    if let Some(rest) = suffix.strip_prefix(" Pre-Release ") {
        return Some((ReleaseType::PreRelease, VersionNumber::parse(rest), true));
    }
    if let Some(rest) = suffix.strip_prefix(" Pre-release ") {
        return Some((ReleaseType::PreRelease, VersionNumber::parse(rest), true));
    }
    if let Some(rest) = suffix.strip_prefix("-rc-") {
        return Some((
            ReleaseType::ReleaseCandidate,
            VersionNumber::parse(rest),
            is_legacy_release,
        ));
    }
    if let Some(rest) = suffix.strip_prefix("-rc") {
        return Some((
            ReleaseType::ReleaseCandidate,
            VersionNumber::parse(rest),
            !is_legacy_release,
        ));
    }
    if let Some(rest) = suffix.strip_prefix(" Release Candidate ") {
        return Some((
            ReleaseType::ReleaseCandidate,
            VersionNumber::parse(rest),
            true,
        ));
    }

    None
}

impl ReleaseVersion {
    /// Creates the version that stands for an unknown version.
    fn zero() -> Self {
        Self {
            value: "0.0".to_string(),
            normalized: "0.0".to_string(),
            major: 0,
            minor: 0,
            patch: 0,
            release_type: ReleaseType::Unknown,
            ea_version: VersionNumber::parse("0"),
            additional: Additional::None,
        }
    }

    /// Parses a release.
    fn parse(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();

        let mut index = 0usize;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == 0 {
            return None;
        }

        let major_text = &value[..index];
        // The major version is either `1` or a number that does not start
        // with a zero.
        if major_text != "1" && (major_text.starts_with('0') || major_text.len() < 2) {
            return None;
        }

        let major: i32 = major_text.parse().ok()?;
        if major != 1 && major < MINIMUM_YEAR_MAJOR_VERSION {
            return None;
        }

        if index >= bytes.len() || bytes[index] != b'.' {
            return None;
        }
        index += 1;

        let minor_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == minor_start {
            return None;
        }
        let minor: i32 = value[minor_start..index].parse().ok()?;

        let mut patch = 0;
        if index < bytes.len() && bytes[index] == b'.' {
            let patch_start = index + 1;
            let mut patch_end = patch_start;
            while patch_end < bytes.len() && bytes[patch_end].is_ascii_digit() {
                patch_end += 1;
            }
            if patch_end > patch_start {
                patch = value[patch_start..patch_end].parse().ok()?;
                index = patch_end;
            }
        }

        let prefix = &value[..index];
        let mut suffix = &value[index..];

        let is_legacy_release = major == 1;
        let mut additional = Additional::None;
        let mut need_normalize = false;

        if let Some(stripped) = suffix.strip_suffix("_unobfuscated") {
            suffix = stripped;
            additional = Additional::Unobfuscated;
        } else if let Some(stripped) = suffix.strip_suffix(" Unobfuscated") {
            need_normalize = true;
            suffix = stripped;
            additional = Additional::Unobfuscated;
        }

        let (release_type, ea_version, parsed_normalize) = parse_suffix(suffix, is_legacy_release)?;
        need_normalize = need_normalize || parsed_normalize;

        let normalized = if need_normalize {
            let mut builder = String::with_capacity(value.len());
            builder.push_str(prefix);
            if release_type != ReleaseType::Ga {
                builder.push_str(if is_legacy_release {
                    release_type.legacy_infix()
                } else {
                    release_type.new_infix()
                });
                builder.push_str(ea_version.value());
            }
            builder.push_str(additional.suffix());
            builder
        } else {
            value.to_string()
        };

        Some(Self {
            value: value.to_string(),
            normalized,
            major,
            minor,
            patch,
            release_type,
            ea_version,
            additional,
        })
    }

    /// Parses a release of the simple format, such as `1.21.8`.
    ///
    /// The kind of the release is [`ReleaseType::Unknown`], so the version
    /// is smaller than the preview versions and than the finished release of
    /// the same number.
    pub fn parse_simple(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();

        let major_length = number_length(bytes, 0);
        if major_length == 0 || bytes.len() < major_length + 2 || bytes[major_length] != b'.' {
            return None;
        }

        let major_text = &value[..major_length];
        let major: i32 = major_text.parse().ok()?;
        if major != 1 && major < MINIMUM_YEAR_MAJOR_VERSION {
            return None;
        }

        let minor_offset = major_length + 1;
        let minor_length = number_length(bytes, minor_offset);
        if minor_length == 0 {
            return None;
        }
        let minor: i32 = value[minor_offset..minor_offset + minor_length]
            .parse()
            .ok()?;

        let mut patch = 0;
        if minor_offset + minor_length < bytes.len() {
            let patch_offset = minor_offset + minor_length + 1;
            if patch_offset >= bytes.len() || bytes[patch_offset - 1] != b'.' {
                return None;
            }
            patch = value[patch_offset..].parse().ok()?;
        }

        Some(Self {
            value: value.to_string(),
            normalized: value.to_string(),
            major,
            minor,
            patch,
            release_type: ReleaseType::Unknown,
            ea_version: VersionNumber::parse("0"),
            additional: Additional::None,
        })
    }

    /// Returns the major version.
    pub const fn major(&self) -> i32 {
        self.major
    }

    /// Returns the minor version.
    pub const fn minor(&self) -> i32 {
        self.minor
    }

    /// Returns the patch version.
    pub const fn patch(&self) -> i32 {
        self.patch
    }

    /// Returns the kind of the release.
    pub const fn release_type(&self) -> ReleaseType {
        self.release_type
    }

    /// Returns the number of the early access version.
    pub const fn ea_version(&self) -> &VersionNumber {
        &self.ea_version
    }

    /// Returns the additional information the version carries.
    pub const fn additional(&self) -> Additional {
        self.additional
    }

    /// Compares the release with another release.
    fn compare_to_release(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| self.release_type.cmp(&other.release_type))
            .then_with(|| self.ea_version.cmp(&other.ea_version))
            .then_with(|| self.additional.cmp(&other.additional))
    }

    /// Compares the release with a snapshot.
    ///
    /// A release is newer than the snapshots that came before it and older
    /// than the snapshots that came after it.
    fn compare_to_snapshot(&self, snapshot: &LegacySnapshotVersion) -> Ordering {
        if self.major == 0 {
            return Ordering::Less;
        }
        if self.major != 1 {
            return Ordering::Greater;
        }

        let tables = tables();
        let index = match tables.snapshot_ints.binary_search(&snapshot.int_value) {
            Ok(index) => index,
            Err(index) => {
                if index == tables.snapshot_ints.len() {
                    return Ordering::Less;
                }
                index
            }
        };

        match &tables.snapshot_prev[index] {
            Some(previous) => {
                if self.compare_to_release(previous) != Ordering::Greater {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            None => Ordering::Less,
        }
    }
}

/// A snapshot of Minecraft, such as `23w42a`.
#[derive(Debug, Clone)]
pub struct LegacySnapshotVersion {
    /// The text of the version.
    value: String,
    /// The normalized text of the version.
    normalized: String,
    /// The number that is used to compare the snapshots.
    int_value: i32,
}

impl LegacySnapshotVersion {
    /// Parses a snapshot.
    fn parse(value: &str) -> Option<Self> {
        if value.len() < 6 || value.as_bytes()[2] != b'w' {
            return None;
        }

        let (prefix_length, unobfuscated, normalized) =
            if let Some(stripped) = value.strip_suffix("_unobfuscated") {
                (stripped.len(), true, value.to_string())
            } else if let Some(stripped) = value.strip_suffix(" Unobfuscated") {
                (stripped.len(), true, format!("{stripped}_unobfuscated"))
            } else {
                (value.len(), false, value.to_string())
            };

        if prefix_length != 6 {
            return None;
        }

        let year: i32 = value[0..2].parse().ok()?;
        let week: i32 = value[3..5].parse().ok()?;
        if year >= MINIMUM_YEAR_MAJOR_VERSION {
            return None;
        }

        let suffix = value.as_bytes()[5] as char;
        if !suffix.is_ascii_lowercase() {
            return None;
        }

        Some(Self {
            value: value.to_string(),
            normalized,
            int_value: Self::to_int(year, week, suffix, unobfuscated),
        })
    }

    /// Returns the number that is used to compare the snapshots.
    const fn to_int(year: i32, week: i32, suffix: char, unobfuscated: bool) -> i32 {
        (year << 24) | (week << 16) | ((suffix as i32) << 8) | if unobfuscated { 1 } else { 0 }
    }

    /// Returns the year of the snapshot.
    pub const fn year(&self) -> i32 {
        (self.int_value >> 24) & 0xff
    }

    /// Returns the week of the snapshot.
    pub const fn week(&self) -> i32 {
        (self.int_value >> 16) & 0xff
    }

    /// Returns the letter of the snapshot.
    pub const fn suffix(&self) -> char {
        ((self.int_value >> 8) & 0xff) as u8 as char
    }

    /// Returns whether the snapshot is the unobfuscated one.
    pub const fn is_unobfuscated(&self) -> bool {
        self.int_value & 1 != 0
    }
}

/// A version that the launcher does not know, such as `1.14_combat-212796`.
///
/// The special versions are the versions of the experiments and of the April
/// Fools snapshots; they are ordered by the version that came before them.
#[derive(Debug, Clone)]
pub struct SpecialVersion {
    /// The text of the version.
    value: String,
    /// The text of the version the alias stands for.
    normalized: String,
    /// The version that came before this one.
    prev: Option<Arc<GameVersionNumber>>,
}

impl SpecialVersion {
    /// Returns whether the version is unknown.
    fn is_unknown(&self) -> bool {
        self.prev.is_none()
    }

    /// Returns the number of the normalized text of the version.
    fn as_version_number(&self) -> VersionNumber {
        VersionNumber::parse(&self.normalized)
    }

    /// Returns the version that came before this one and that is not a
    /// special version.
    fn prev_normal_version(&self) -> Option<&GameVersionNumber> {
        let mut current = self.prev.as_deref();
        while let Some(GameVersionNumber::Special(special)) = current {
            current = special.prev.as_deref();
        }
        current
    }

    /// Compares the version with a release or with a snapshot.
    fn compare_to_release_or_snapshot(&self, other: &GameVersionNumber) -> Ordering {
        if self.is_unknown() {
            return Ordering::Greater;
        }

        match self.prev_normal_version() {
            Some(previous) if previous.cmp(other) != Ordering::Less => Ordering::Greater,
            _ => Ordering::Less,
        }
    }

    /// Compares the version with another special version.
    fn compare_to_special(&self, other: &Self) -> Ordering {
        if self.is_unknown() {
            return if other.is_unknown() {
                self.as_version_number().cmp(&other.as_version_number())
            } else {
                Ordering::Greater
            };
        }
        if other.is_unknown() {
            return Ordering::Less;
        }
        if self.normalized == other.normalized {
            return Ordering::Equal;
        }

        if let (Some(left), Some(right)) = (self.prev_normal_version(), other.prev_normal_version())
        {
            let result = left.cmp(right);
            if result != Ordering::Equal {
                return result;
            }
        }

        // The aliases of the same version are ordered by the order they
        // appear in the list of the versions.
        let mut current = self.prev.as_deref();
        while let Some(GameVersionNumber::Special(special)) = current {
            if special.normalized == other.normalized {
                return Ordering::Greater;
            }
            current = special.prev.as_deref();
        }

        Ordering::Less
    }
}

/// Returns the length of the number that starts at the offset.
fn number_length(bytes: &[u8], offset: usize) -> usize {
    let mut current = offset;
    while current < bytes.len() && bytes[current].is_ascii_digit() {
        current += 1;
    }
    current - offset
}

/// A version number of Minecraft.
///
/// The versions are compared with the rules of the Java implementation of
/// HMCL, so the launcher shows them in the same order.
#[derive(Debug, Clone)]
pub enum GameVersionNumber {
    /// A version of the first generations, such as `b1.7.3`.
    Old(OldVersion),
    /// A release, such as `1.20.1`.
    Release(ReleaseVersion),
    /// A snapshot, such as `23w42a`.
    LegacySnapshot(LegacySnapshotVersion),
    /// A version the launcher does not know, such as `1.14_combat-212796`.
    Special(SpecialVersion),
}

impl GameVersionNumber {
    /// Creates the version that stands for an unknown version.
    fn special(value: &str) -> Self {
        Self::Special(SpecialVersion {
            value: value.to_string(),
            normalized: value.to_string(),
            prev: None,
        })
    }

    /// Parses a version without looking at the known special versions.
    ///
    /// The known versions are used while the table of the versions is built,
    /// so the table cannot be read while it is built.
    fn parse_plain(value: &str) -> Self {
        if !value.is_empty() {
            let first = value.as_bytes()[0];
            if matches!(first, b'r' | b'a' | b'b' | b'c' | b'i') {
                return match OldVersion::parse(value) {
                    Some(old) => Self::Old(old),
                    None => Self::special(value),
                };
            }

            if value == "0.0" {
                return Self::Release(ReleaseVersion::zero());
            }

            if value.len() >= 6 && value.as_bytes()[2] == b'w' {
                return match LegacySnapshotVersion::parse(value) {
                    Some(snapshot) => Self::LegacySnapshot(snapshot),
                    None => Self::special(value),
                };
            }

            return match ReleaseVersion::parse(value) {
                Some(release) => Self::Release(release),
                None => Self::special(value),
            };
        }

        Self::special(value)
    }

    /// Parses a version.
    ///
    /// The versions the launcher knows, such as the experiments and the
    /// April Fools snapshots, are taken from the table of the versions; every
    /// other text is parsed on its own.
    pub fn as_game_version(value: &str) -> Self {
        match tables().specials.get(value) {
            Some(known) => known.as_ref().clone(),
            None => Self::parse_plain(value),
        }
    }

    /// Returns the text of the version.
    pub fn value(&self) -> &str {
        match self {
            Self::Old(version) => &version.value,
            Self::Release(version) => &version.value,
            Self::LegacySnapshot(version) => &version.value,
            Self::Special(version) => &version.value,
        }
    }

    /// Returns the normalized text of the version.
    ///
    /// The normalized text of a release drops the parts that are only used
    /// for the display, such as the spaces of `1.0.1 Pre-release 1`.
    pub fn to_normalized_string(&self) -> &str {
        match self {
            Self::Old(version) => &version.value,
            Self::Release(version) => &version.normalized,
            Self::LegacySnapshot(version) => &version.normalized,
            Self::Special(version) => &version.normalized,
        }
    }

    /// Returns the group the version belongs to.
    fn version_type(&self) -> VersionType {
        match self {
            Self::Old(version) => version.version_type,
            Self::Release(_) | Self::LegacySnapshot(_) | Self::Special(_) => VersionType::New,
        }
    }

    /// Returns whether the version is an April Fools snapshot.
    pub fn is_april_fools(&self) -> bool {
        match self {
            Self::Special(version) => {
                let normalized = &version.normalized;
                (!normalized.starts_with("1.") && normalized != "13w12~")
                    || normalized == "1.RV-Pre1"
            }
            Self::LegacySnapshot(version) => {
                version.year() == 15 && version.week() == 14 && version.suffix() == 'a'
            }
            _ => false,
        }
    }

    /// Returns the release the version belongs to.
    ///
    /// The early access versions belong to the release they are the preview
    /// of; the snapshots belong to the release that came after them. The
    /// versions that do not belong to a release return `None`.
    pub fn get_release_of_snapshot(&self) -> Option<Self> {
        match self {
            Self::Release(release) => {
                if release.release_type == ReleaseType::Ga {
                    return None;
                }
                if release.patch > 0 {
                    Some(Self::Release(ReleaseVersion {
                        value: format!("{}.{}.{}", release.major, release.minor, release.patch),
                        normalized: format!(
                            "{}.{}.{}",
                            release.major, release.minor, release.patch
                        ),
                        major: release.major,
                        minor: release.minor,
                        patch: release.patch,
                        release_type: ReleaseType::Ga,
                        ea_version: VersionNumber::parse("0"),
                        additional: Additional::None,
                    }))
                } else {
                    Some(Self::Release(ReleaseVersion {
                        value: format!("{}.{}", release.major, release.minor),
                        normalized: format!("{}.{}", release.major, release.minor),
                        major: release.major,
                        minor: release.minor,
                        patch: 0,
                        release_type: ReleaseType::Ga,
                        ea_version: VersionNumber::parse("0"),
                        additional: Additional::None,
                    }))
                }
            }
            Self::LegacySnapshot(snapshot) => {
                let tables = tables();
                // The list of the finished releases is ordered from the
                // newest to the oldest one, so it is walked backwards.
                for default_version in tables.default_game_versions.iter().rev() {
                    let release = match Self::as_game_version(default_version) {
                        Self::Release(release) => release,
                        _ => continue,
                    };
                    if release.compare_to_snapshot(snapshot) == Ordering::Greater {
                        return Some(Self::Release(release));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Compares the version with another version of the same group.
    fn compare_to_impl(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Old(left), Self::Old(right)) => left.version_number.cmp(&right.version_number),
            (Self::Release(left), Self::Release(right)) => left.compare_to_release(right),
            (Self::Release(left), Self::LegacySnapshot(right)) => left.compare_to_snapshot(right),
            (Self::Release(_), Self::Special(right)) => {
                right.compare_to_release_or_snapshot(self).reverse()
            }
            (Self::LegacySnapshot(left), Self::Release(right)) => {
                right.compare_to_snapshot(left).reverse()
            }
            (Self::LegacySnapshot(left), Self::LegacySnapshot(right)) => {
                left.int_value.cmp(&right.int_value)
            }
            (Self::LegacySnapshot(_), Self::Special(right)) => {
                right.compare_to_release_or_snapshot(self).reverse()
            }
            (Self::Special(left), Self::Special(right)) => left.compare_to_special(right),
            (Self::Special(left), _) => left.compare_to_release_or_snapshot(other),
            _ => Ordering::Equal,
        }
    }
}

impl fmt::Display for GameVersionNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.value())
    }
}

impl PartialEq for GameVersionNumber {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Old(left), Self::Old(right)) => {
                left.version_type == right.version_type
                    && left.version_number == right.version_number
            }
            (Self::Release(left), Self::Release(right)) => {
                left.major == right.major
                    && left.minor == right.minor
                    && left.patch == right.patch
                    && left.release_type == right.release_type
                    && left.ea_version == right.ea_version
                    && left.additional == right.additional
            }
            (Self::LegacySnapshot(left), Self::LegacySnapshot(right)) => {
                left.int_value == right.int_value
            }
            (Self::Special(left), Self::Special(right)) => left.normalized == right.normalized,
            _ => false,
        }
    }
}

impl Eq for GameVersionNumber {}

impl PartialOrd for GameVersionNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GameVersionNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        let left_type = self.version_type();
        let right_type = other.version_type();
        if left_type != right_type {
            return left_type.cmp(&right_type);
        }
        self.compare_to_impl(other)
    }
}

/// The tables that are built from the list of the versions of Minecraft.
struct Tables {
    /// The versions the launcher knows, such as the experiments.
    specials: HashMap<String, Arc<GameVersionNumber>>,
    /// The finished releases, from the newest to the oldest one.
    default_game_versions: Vec<String>,
    /// The numbers of the snapshots, in the order they were released.
    snapshot_ints: Vec<i32>,
    /// The release that came before each snapshot.
    snapshot_prev: Vec<Option<ReleaseVersion>>,
}

/// The tables that are built from the list of the versions of Minecraft.
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(build_tables)
}

/// Builds the tables of the versions of Minecraft.
///
/// The list of the versions is read once, when the tables are built; the
/// versions are then known to the launcher.
fn build_tables() -> Tables {
    let mut specials: HashMap<String, Arc<GameVersionNumber>> = HashMap::new();
    let mut default_game_versions: Vec<String> = Vec::new();
    let mut snapshot_ints: Vec<i32> = Vec::new();
    let mut snapshot_prev: Vec<Option<ReleaseVersion>> = Vec::new();
    let mut current_release: Option<ReleaseVersion> = None;
    let mut previous: Option<Arc<GameVersionNumber>> = None;

    for line in VERSIONS_TXT.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let version: Arc<GameVersionNumber> = match specials.get(line) {
            Some(known) => Arc::clone(known),
            None => Arc::new(GameVersionNumber::parse_plain(line)),
        };

        let version = match version.as_ref() {
            GameVersionNumber::Special(special) => {
                let mut special = special.clone();
                special.prev = previous.clone();
                let version = Arc::new(GameVersionNumber::Special(special));
                specials.insert(line.to_string(), Arc::clone(&version));
                version
            }
            GameVersionNumber::LegacySnapshot(snapshot) => {
                snapshot_ints.push(snapshot.int_value);
                snapshot_prev.push(current_release.clone());
                version
            }
            GameVersionNumber::Release(release) => {
                current_release = Some(release.clone());
                if release.release_type == ReleaseType::Ga && release.additional == Additional::None
                {
                    default_game_versions.push(release.value.clone());
                }
                version
            }
            _ => version,
        };

        previous = Some(version);
    }

    // The finished releases are read from the oldest to the newest one, so
    // the list is turned around to keep the newest one first.
    default_game_versions.reverse();

    // The aliases of the special versions are known as well; they stand for
    // the version they are the alias of.
    for line in VERSION_ALIAS_CSV.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split(',');
        let normalized = match parts.next() {
            Some(normalized) => normalized.trim(),
            None => continue,
        };

        let normalized_version = match specials.get(normalized) {
            Some(version) => Arc::clone(version),
            None => continue,
        };
        let normalized_prev = match normalized_version.as_ref() {
            GameVersionNumber::Special(special) => special.prev.clone(),
            _ => None,
        };

        for alias in parts {
            let alias = alias.trim();
            if alias.is_empty() {
                continue;
            }
            specials.insert(
                alias.to_string(),
                Arc::new(GameVersionNumber::Special(SpecialVersion {
                    value: alias.to_string(),
                    normalized: normalized.to_string(),
                    prev: normalized_prev.clone(),
                })),
            );
        }
    }

    Tables {
        specials,
        default_game_versions,
        snapshot_ints,
        snapshot_prev,
    }
}

/// The version number of Minecraft.
///
/// The versions are compared with the rules of the Java implementation of
/// HMCL, so the launcher shows them in the same order.
impl GameVersionNumber {
    /// Compares two versions.
    pub fn compare(version1: &str, version2: &str) -> Ordering {
        Self::as_game_version(version1).cmp(&Self::as_game_version(version2))
    }

    /// Returns whether the text is a version of Minecraft.
    ///
    /// The versions the launcher does not know, such as the versions of the
    /// add-ons, are not versions of Minecraft.
    pub fn is_known(version: &str) -> bool {
        !matches!(
            Self::as_game_version(version),
            Self::Special(ref special) if special.prev.is_none()
        )
    }

    /// Returns whether the version is at least the given version.
    ///
    /// The feature a version supports is often introduced by a snapshot and
    /// released later, so the release and the snapshot are compared at the
    /// same time.
    ///
    /// When `strict_release_version` is `false`, the release is compared as
    /// if it were released after its preview versions.
    pub fn is_at_least(
        &self,
        release_version: &str,
        snapshot_version: &str,
        strict_release_version: bool,
    ) -> bool {
        match self {
            Self::Release(release) => {
                let other = if strict_release_version {
                    ReleaseVersion::parse(release_version)
                } else {
                    ReleaseVersion::parse_simple(release_version)
                };
                match other {
                    Some(other) => release.compare_to_release(&other) != Ordering::Less,
                    None => false,
                }
            }
            _ => match LegacySnapshotVersion::parse(snapshot_version) {
                Some(snapshot) => self.cmp(&Self::LegacySnapshot(snapshot)) != Ordering::Less,
                None => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the versions of Minecraft, in the order they were released.
    fn read_versions() -> Vec<String> {
        VERSIONS_TXT
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect()
    }

    /// Returns whether the two versions are the same.
    fn assert_versions_equal(version1: &str, version2: &str) {
        let left = GameVersionNumber::as_game_version(version1);
        let right = GameVersionNumber::as_game_version(version2);

        assert_eq!(left, right, "version1={version1}, version2={version2}");
        assert_eq!(right, left, "version1={version1}, version2={version2}");
        assert_eq!(
            left.cmp(&right),
            Ordering::Equal,
            "version1={version1}, version2={version2}"
        );
        assert_eq!(
            right.cmp(&left),
            Ordering::Equal,
            "version1={version1}, version2={version2}"
        );
    }

    /// Returns whether the versions are ordered from the oldest to the newest.
    fn assert_order(versions: &[&str]) {
        let parsed: Vec<GameVersionNumber> = versions
            .iter()
            .map(|version| GameVersionNumber::as_game_version(version))
            .collect();

        for (index, version) in parsed.iter().enumerate() {
            for other in parsed.iter().take(index) {
                assert!(version > other, "{version} should be newer than {other}");
                assert!(other < version, "{other} should be older than {version}");
                assert_ne!(version, other, "{version} should differ from {other}");
            }

            for other in parsed.iter().skip(index + 1) {
                assert!(version < other, "{version} should be older than {other}");
                assert!(other > version, "{other} should be newer than {version}");
                assert_ne!(version, other, "{version} should differ from {other}");
            }
        }

        assert_versions_equal(versions[0], versions[0]);
        assert_versions_equal(versions[versions.len() - 1], versions[versions.len() - 1]);
    }

    /// Returns whether the version has the normalized text.
    fn assert_normalized(normalized: &str, version: &str) {
        assert_versions_equal(version, version);
        assert_versions_equal(normalized, version);
        assert_eq!(
            GameVersionNumber::as_game_version(version).to_normalized_string(),
            normalized
        );
    }

    #[test]
    fn april_fools_versions_are_detected() {
        for version in [
            "15w14a",
            "1.RV-Pre1",
            "3D Shareware v1.34",
            "2.0",
            "20w14infinite",
            "22w13oneBlockAtATime",
            "23w13a_or_b",
            "24w14potato",
            "25w14craftmine",
        ] {
            assert!(
                GameVersionNumber::as_game_version(version).is_april_fools(),
                "{version} should be an April Fools version"
            );
        }

        for version in [
            "1.21.8",
            "1.21.8-rc1",
            "25w21a",
            "13w12~",
            "15w14b",
            "25w45a_unobfuscated",
        ] {
            assert!(
                !GameVersionNumber::as_game_version(version).is_april_fools(),
                "{version} should not be an April Fools version"
            );
        }
    }

    #[test]
    fn the_versions_are_sorted_in_the_order_they_were_released() {
        let versions = read_versions();
        let mut sorted = versions.clone();
        sorted.sort_by(|left, right| {
            GameVersionNumber::as_game_version(left).cmp(&GameVersionNumber::as_game_version(right))
        });

        assert_eq!(versions, sorted);
    }

    #[test]
    fn the_old_versions_are_parsed() {
        for (version, version_type, number) in [
            ("rd-132211", VersionType::PreClassic, "132211"),
            ("in-20100223", VersionType::Indev, "20100223"),
            ("in-20100212-2", VersionType::Indev, "20100212-2"),
            ("inf-20100618", VersionType::Infdev, "20100618"),
            ("inf-20100330-1", VersionType::Infdev, "20100330-1"),
            ("a1.0.6", VersionType::Alpha, "1.0.6"),
            ("a1.0.8_01", VersionType::Alpha, "1.0.8_01"),
            ("a1.0.13_01-1", VersionType::Alpha, "1.0.13_01-1"),
            ("b1.0", VersionType::Beta, "1.0"),
            ("b1.0_01", VersionType::Beta, "1.0_01"),
            ("b1.6-tb3", VersionType::Beta, "1.6-tb3"),
            ("b1.8-pre1-2", VersionType::Beta, "1.8-pre1-2"),
            ("b1.9-pre1", VersionType::Beta, "1.9-pre1"),
        ] {
            match GameVersionNumber::as_game_version(version) {
                GameVersionNumber::Old(old) => {
                    assert_eq!(old.version_type, version_type, "version={version}");
                    assert_eq!(old.version_number, VersionNumber::parse(number));
                }
                other => panic!("version={version} should be an old version, got {other:?}"),
            }
        }

        for version in [
            "",
            "1.21",
            "r-132211",
            "rd-",
            "rd-a",
            "i-20100223",
            "in-",
            "in-a",
        ] {
            assert!(
                !matches!(
                    GameVersionNumber::as_game_version(version),
                    GameVersionNumber::Old(_)
                ),
                "version={version} should not be an old version"
            );
        }
    }

    #[test]
    fn the_snapshots_are_parsed() {
        let (year, week, suffix) = (25, 46, 'a');
        {
            let raw = format!("{year:02}w{week:02}{suffix}");
            match GameVersionNumber::as_game_version(&raw) {
                GameVersionNumber::LegacySnapshot(snapshot) => {
                    assert_eq!(snapshot.value, raw);
                    assert_eq!(snapshot.normalized, raw);
                    assert_eq!(snapshot.year(), year);
                    assert_eq!(snapshot.week(), week);
                    assert_eq!(snapshot.suffix(), suffix);
                    assert!(!snapshot.is_unobfuscated());
                }
                other => panic!("version={raw} should be a snapshot, got {other:?}"),
            }

            let unobfuscated = format!("{raw}_unobfuscated");
            match GameVersionNumber::as_game_version(&unobfuscated) {
                GameVersionNumber::LegacySnapshot(snapshot) => {
                    assert_eq!(snapshot.value, unobfuscated);
                    assert_eq!(snapshot.normalized, unobfuscated);
                    assert_eq!(snapshot.year(), year);
                    assert_eq!(snapshot.week(), week);
                    assert_eq!(snapshot.suffix(), suffix);
                    assert!(snapshot.is_unobfuscated());
                }
                other => panic!("version={unobfuscated} should be a snapshot, got {other:?}"),
            }

            let spaced = format!("{raw} Unobfuscated");
            match GameVersionNumber::as_game_version(&spaced) {
                GameVersionNumber::LegacySnapshot(snapshot) => {
                    assert_eq!(snapshot.value, spaced);
                    assert_eq!(snapshot.normalized, unobfuscated);
                    assert_eq!(snapshot.year(), year);
                    assert_eq!(snapshot.week(), week);
                    assert_eq!(snapshot.suffix(), suffix);
                    assert!(snapshot.is_unobfuscated());
                }
                other => panic!("version={spaced} should be a snapshot, got {other:?}"),
            }
        }
    }

    #[test]
    fn the_simple_releases_are_parsed() {
        for (version, major, minor, patch) in [
            ("1.0", 1, 0, 0),
            ("1.13", 1, 13, 0),
            ("1.21.8", 1, 21, 8),
            ("26.1", 26, 1, 0),
            ("26.1.1", 26, 1, 1),
        ] {
            match ReleaseVersion::parse_simple(version) {
                Some(release) => {
                    assert_eq!(release.major(), major);
                    assert_eq!(release.minor(), minor);
                    assert_eq!(release.patch(), patch);
                    assert_eq!(release.release_type(), ReleaseType::Unknown);
                    assert_eq!(release.ea_version(), &VersionNumber::parse("0"));
                }
                None => panic!("version={version} should be a release"),
            }
        }

        for version in [
            "26", "24.0.0", "24.0", "2.0", "1", "1..0", "1.0.", "1.a", "1.1a", "1.0a", "1.0.0.0",
        ] {
            assert!(
                ReleaseVersion::parse_simple(version).is_none(),
                "version={version} should not be a release"
            );
        }
    }

    #[test]
    fn the_releases_are_compared() {
        assert_versions_equal("0.0", "0.0");
        assert_versions_equal("1.100", "1.100");
        assert_versions_equal("1.100.1", "1.100.1");
        assert_versions_equal("1.100.1-pre1", "1.100.1-pre1");
        assert_versions_equal("1.100.1-pre1", "1.100.1 Pre-Release 1");

        assert_order(&[
            "0.0",
            "1.0",
            "1.99",
            "1.99.1-pre1",
            "1.99.1 Pre-Release 2",
            "1.99.1-rc1",
            "1.99.1",
            "1.100",
            "1.100.1",
            "26.1",
        ]);
    }

    #[test]
    fn the_snapshots_are_compared() {
        assert_order(&["90w01a", "90w01b", "90w01e", "90w02a"]);
    }

    #[test]
    fn the_aliases_of_the_special_versions_are_compared() {
        assert_normalized("2.0_purple", "2point0_purple");
        assert_normalized("2.0_red", "2point0_red");
        assert_normalized("2.0_blue", "2point0_blue");

        for purple in ["2.0_purple", "2point0_purple"] {
            for red in ["2.0_red", "2point0_red"] {
                for blue in ["2.0_blue", "2point0_blue"] {
                    assert_order(&["1.5.1", "2.0", purple, red, blue, "1.5.2"]);
                }
            }
        }
    }

    #[test]
    fn the_versions_of_every_kind_are_compared() {
        assert_order(&[
            "rd-132211",
            "rd-161348",
            "rd-20090515",
            "c0.0.11a",
            "c0.0.13a",
            "c0.0.13a_03",
            "c0.30_01c",
            "inf-20100330-1",
            "inf-20100330-2",
            "inf-20100618",
            "a1.0.4",
            "a1.0.8_01",
            "a1.0.10",
            "a1.0.13_01-1",
            "a1.0.17_02",
            "a1.0.17_04",
            "a1.1.0",
            "a1.1.1",
            "b1.0",
            "b1.0_01",
            "b1.1_02",
            "b1.2",
            "b1.8-pre1-2",
            "b1.8.1",
            "0.0",
            "1.0.0-rc1",
            "1.0.0-rc2-1",
            "1.0.0-rc2-2",
            "1.0.0-rc2-3",
            "1.0",
            "11w47a",
            "1.1",
            "1.5.1",
            "2.0",
            "1.5.2",
            "1.9.2",
            "1.RV-Pre1",
            "16w14a",
            "1.9.3-pre1",
            "1.13.2",
            "19w13b",
            "3D Shareware v1.34",
            "19w14a",
            "1.14 Pre-Release 1",
            "1.14",
            "1.15.2",
            "20w06a",
            "20w13b",
            "20w14infinite",
            "20w22a",
            "1.16-pre1",
            "1.16",
            "1.18.2",
            "22w13oneblockatatime",
            "22w11a",
            "1.19-pre1",
            "1.19.4",
            "23w13a",
            "23w13a_or_b",
            "23w14a",
            "1.20",
            "24w13a",
            "24w14potato",
            "24w14a",
            "25w46a",
            "25w46a_unobfuscated",
            "1.21.11-pre1",
            "1.21.11-pre1_unobfuscated",
            "1.21.11-pre2",
            "1.21.11-pre2_unobfuscated",
            "26.1-snapshot-1",
            "26.1-snapshot-2",
            "26.1",
            "26.1.1",
            "26w14a",
            "26.2-snapshot-1",
            "26.2-snapshot-2",
            "26.2",
            "100.0",
            "Unknown",
        ]);
    }

    #[test]
    fn the_unknown_versions_are_compared() {
        assert_order(&[
            "23w35a",
            "1.20.2-pre1",
            "1.20.2-rc1",
            "1.20.2",
            "23w35b",
            "23w40a",
        ]);

        assert_order(&["1.20.4", "24w04a", "1.100"]);

        assert_order(&["1.19.4", "23w18a", "1.19.5", "1.20"]);

        assert_order(&["1.0", "10w47a", "11w47a", "1.1"]);
    }

    #[test]
    fn the_versions_are_normalized() {
        for version in read_versions() {
            assert_normalized(&version, &version);
        }

        assert_normalized("1.21.11-pre3", "1.21.11-pre-3");
        assert_normalized("1.21.11-pre3", "1.21.11 Pre-Release 3");
        assert_normalized(
            "1.21.11-pre3_unobfuscated",
            "1.21.11 Pre-Release 3 Unobfuscated",
        );
        assert_normalized("1.21.11-pre3_unobfuscated", "1.21.11-pre3 Unobfuscated");
        assert_normalized("1.21.11-rc1", "1.21.11-rc-1");
        assert_normalized("1.21.11-rc1", "1.21.11 Release Candidate 1");
        assert_normalized(
            "1.21.11-rc1_unobfuscated",
            "1.21.11 Release Candidate 1 Unobfuscated",
        );
        assert_normalized("1.14_combat-212796", "1.14.3 - Combat Test");
        assert_normalized("1.14_combat-0", "Combat Test 2");
        assert_normalized("1.14_combat-3", "Combat Test 3");
        assert_normalized("1.15_combat-1", "Combat Test 4");
        assert_normalized("1.15_combat-6", "Combat Test 5");
        assert_normalized("1.16_combat-0", "Combat Test 6");
        assert_normalized("1.16_combat-1", "Combat Test 7");
        assert_normalized("1.16_combat-2", "Combat Test 7b");
        assert_normalized("1.16_combat-3", "Combat Test 7c");
        assert_normalized("1.16_combat-4", "Combat Test 8");
        assert_normalized("1.16_combat-5", "Combat Test 8b");
        assert_normalized("1.16_combat-6", "Combat Test 8c");
        assert_normalized("1.16.2-pre1", "1.16.2 Pre-release 1");
        assert_normalized(
            "1.18_experimental-snapshot-1",
            "1.18 Experimental Snapshot 1",
        );
        assert_normalized(
            "1.18_experimental-snapshot-2",
            "1.18 experimental snapshot 2",
        );
        assert_normalized(
            "1.18_experimental-snapshot-3",
            "1.18 experimental snapshot 3",
        );
        assert_normalized(
            "1.18_experimental-snapshot-4",
            "1.18 experimental snapshot 4",
        );
        assert_normalized(
            "1.18_experimental-snapshot-5",
            "1.18 experimental snapshot 5",
        );
        assert_normalized(
            "1.18_experimental-snapshot-6",
            "1.18 experimental snapshot 6",
        );
        assert_normalized(
            "1.18_experimental-snapshot-7",
            "1.18 experimental snapshot 7",
        );
        assert_normalized(
            "1.19_deep_dark_experimental_snapshot-1",
            "Deep Dark Experimental Snapshot 1",
        );
        assert_normalized("20w14infinite", "20w14~");
        assert_normalized("22w13oneBlockAtATime", "22w13oneblockatatime");
        assert_normalized("26.1-snapshot-1", "26.1 Snapshot 1");
        assert_normalized("26.1-rc-1", "26.1-rc1");
        assert_normalized("26.1-pre-1", "26.1-pre1");
    }

    #[test]
    fn the_features_of_the_versions_are_compared() {
        // The versions that support the feature, no matter whether the
        // release is compared strictly.
        for version in ["1.13", "1.13.1", "1.14", "17w43a", "17w43b", "17w45a"] {
            let version = GameVersionNumber::as_game_version(version);
            assert!(
                version.is_at_least("1.13", "17w43a", true),
                "{version} should support the feature"
            );
            assert!(
                version.is_at_least("1.13", "17w43a", false),
                "{version} should support the feature"
            );
        }

        // The preview versions of the release support the feature, but only
        // when the release is not compared strictly.
        for version in ["1.13-rc1", "1.13-pre1"] {
            let version = GameVersionNumber::as_game_version(version);
            assert!(
                version.is_at_least("1.13", "17w43a", false),
                "{version} should support the feature"
            );
            assert!(
                !version.is_at_least("1.13", "17w43a", true),
                "{version} should not support the feature"
            );
        }

        // The versions that do not support the feature.
        for version in [
            "17w31a",
            "1.12",
            "1.12.2",
            "1.12.2-pre1",
            "rd-132211",
            "a1.0.6",
        ] {
            let version = GameVersionNumber::as_game_version(version);
            assert!(
                !version.is_at_least("1.13", "17w43a", true),
                "{version} should not support the feature"
            );
            assert!(
                !version.is_at_least("1.13", "17w43a", false),
                "{version} should not support the feature"
            );
        }
    }

    #[test]
    fn the_release_of_a_snapshot_is_found() {
        let release = GameVersionNumber::as_game_version("1.21.11");
        assert_eq!(
            GameVersionNumber::as_game_version("25w45a").get_release_of_snapshot(),
            Some(release.clone())
        );
        assert_eq!(
            GameVersionNumber::as_game_version("25w45a_unobfuscated").get_release_of_snapshot(),
            Some(release.clone())
        );
        assert_eq!(
            GameVersionNumber::as_game_version("1.21.11-pre3").get_release_of_snapshot(),
            Some(release)
        );
        assert_eq!(
            GameVersionNumber::as_game_version("26.1-snapshot-9").get_release_of_snapshot(),
            Some(GameVersionNumber::as_game_version("26.1"))
        );
        assert_eq!(
            GameVersionNumber::as_game_version("26.1").get_release_of_snapshot(),
            None
        );
        assert_eq!(
            GameVersionNumber::as_game_version("20w14infinite").get_release_of_snapshot(),
            None
        );
    }
}
