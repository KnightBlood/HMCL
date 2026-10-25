//! The resource packs and the data packs of a game directory.
//!
//! A pack is a directory or a zip file that holds a `pack.mcmeta` which names
//! the version of the format the pack is written for. The launcher reads the
//! metadata of every pack and tells the player whether the game of the
//! directory can use the pack.
//!
//! Resource packs are enabled by the options of the game: the options name the
//! packs the game loads, so the launcher changes the options instead of moving
//! the files of the packs.

use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use crate::util::string::is_not_blank;
use crate::util::versioning::game_version_number::GameVersionNumber;
use crate::util::zip;

/// The failure of reading or managing the packs of a game directory.
#[derive(Debug)]
pub enum PackError {
    /// The directory of the packs cannot be read or a file cannot be copied.
    Io {
        /// The path the operation was about.
        path: PathBuf,
        /// The reason the operation failed.
        error: std::io::Error,
    },
    /// The file is not a pack, so it is not imported.
    NotAPack {
        /// The file that is not a pack.
        path: PathBuf,
    },
}

impl std::fmt::Display for PackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, error } => {
                write!(formatter, "{} cannot be used: {error}", path.display())
            }
            Self::NotAPack { path } => {
                write!(formatter, "{} is not a pack", path.display())
            }
        }
    }
}

impl std::error::Error for PackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// A version of the format of a pack.
///
/// The format is a major number and, after 1.21.9, a minor number; a minor
/// number of zero is written alone, so the format 15 is written as `15`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PackVersion {
    /// The major number of the format.
    pub major: i32,
    /// The minor number of the format.
    pub minor: i32,
}

impl PackVersion {
    /// The version of a pack whose format is not named.
    pub const UNSPECIFIED: Self = Self {
        major: -1,
        minor: -1,
    };

    /// Creates a version of the format of a pack.
    pub const fn new(major: i32, minor: i32) -> Self {
        Self { major, minor }
    }

    /// Returns whether the format is not named.
    pub const fn is_unspecified(self) -> bool {
        self.major < 0 || self.minor < 0
    }

    /// Returns the version of the format that names the major number alone.
    pub const fn of_major(major: i32) -> Self {
        Self { major, minor: 0 }
    }
}

impl PartialOrd for PackVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PackVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // The version of a pack that names no format is smaller than every
        // version that names one, so a pack without a format is ordered first.
        match (self.is_unspecified(), other.is_unspecified()) {
            (true, true) => return std::cmp::Ordering::Equal,
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            (false, false) => {}
        }

        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
    }
}

impl std::fmt::Display for PackVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_unspecified() {
            return formatter.write_str("UNSPECIFIED");
        }
        if self.minor != 0 {
            write!(formatter, "{}.{}", self.major, self.minor)
        } else {
            write!(formatter, "{}", self.major)
        }
    }
}

/// The formats a pack declares it supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportedFormats {
    /// The first format the pack supports.
    pub min: i32,
    /// The last format the pack supports.
    pub max: i32,
}

impl SupportedFormats {
    /// The formats of a pack that names none.
    pub const UNSPECIFIED: Self = Self { min: -1, max: -1 };

    /// Returns whether the pack names no formats.
    pub const fn is_unspecified(self) -> bool {
        self.min < 0 || self.max < 0 || self.min > self.max
    }
}

/// The metadata of a pack, which the pack writes into `pack.mcmeta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackInfo {
    /// The format of the pack.
    pub pack_format: i32,
    /// The formats the pack declares it supports, if it declares any.
    pub supported_formats: SupportedFormats,
    /// The first format the pack supports, if it names one.
    pub min_format: PackVersion,
    /// The last format the pack supports, if it names one.
    pub max_format: PackVersion,
    /// The description of the pack.
    pub description: String,
}

impl Default for PackInfo {
    fn default() -> Self {
        Self {
            pack_format: 0,
            supported_formats: SupportedFormats::UNSPECIFIED,
            min_format: PackVersion::UNSPECIFIED,
            max_format: PackVersion::UNSPECIFIED,
            description: String::new(),
        }
    }
}

impl PackInfo {
    /// Returns the first format of the pack.
    ///
    /// A pack that names the new format names the first version it supports
    /// there; a pack of the old format names the format it is written for.
    pub fn effective_min_version(&self) -> PackVersion {
        if self.min_format.is_unspecified() {
            PackVersion::of_major(self.pack_format)
        } else {
            self.min_format
        }
    }

    /// Returns the last format of the pack.
    pub fn effective_max_version(&self) -> PackVersion {
        if self.max_format.is_unspecified() {
            PackVersion::of_major(self.pack_format)
        } else {
            self.max_format
        }
    }

    /// Reads the metadata of a pack from the text of its `pack.mcmeta`.
    ///
    /// A pack whose metadata is not a table of the pack returns nothing, so the
    /// pack is read as a pack whose metadata is broken.
    pub fn from_json(text: &str) -> Option<Self> {
        let json: Json = serde_json::from_str(text).ok()?;
        let pack = json.get("pack")?;

        let pack_format = pack.get("pack_format").and_then(Json::as_i64).unwrap_or(0) as i32;

        let supported_formats = pack
            .get("supported_formats")
            .map(parse_supported_formats)
            .unwrap_or(SupportedFormats::UNSPECIFIED);

        let min_format = pack
            .get("min_format")
            .map(|value| parse_pack_version(value, false))
            .unwrap_or(PackVersion::UNSPECIFIED);
        let max_format = pack
            .get("max_format")
            .map(|value| parse_pack_version(value, true))
            .unwrap_or(PackVersion::UNSPECIFIED);

        Some(Self {
            pack_format,
            supported_formats,
            min_format,
            max_format,
            description: pack
                .get("description")
                .map(parse_description)
                .unwrap_or_default(),
        })
    }

    /// Returns the range of the formats the pack supports for a game that reads
    /// the old format.
    ///
    /// The game before 1.21.9 compares the formats by their major number and
    /// accepts the formats between the two numbers the pack names.
    pub fn old_version_range(&self) -> Option<(PackVersion, PackVersion)> {
        if self.pack_format <= 0 {
            return None;
        }
        if self.supported_formats.is_unspecified() {
            let version = PackVersion::of_major(self.pack_format);
            return Some((version, version));
        }
        Some((
            PackVersion::of_major(self.supported_formats.min),
            PackVersion::of_major(self.supported_formats.max),
        ))
    }

    /// Returns the range of the formats the pack supports for a game that reads
    /// the format the pack is written for alone.
    ///
    /// The game before the list of the supported formats was added reads the
    /// format the pack is written for, so a pack that names a range of the
    /// formats is read as a pack of the format it is written for.
    pub fn oldest_version_range(&self) -> Option<(PackVersion, PackVersion)> {
        if self.pack_format <= 0 {
            return None;
        }
        let version = PackVersion::of_major(self.pack_format);
        Some((version, version))
    }

    /// Returns the range of the formats the pack supports for a game that reads
    /// the new format.
    ///
    /// The game of 1.21.9 and later reads the formats a pack names as a range
    /// that holds a major number alone or a pair of numbers, and it still reads
    /// the old list of formats from a pack that names it. A pack whose formats
    /// do not agree with each other is refused, because the game refuses it.
    pub fn new_version_range(&self) -> Option<(PackVersion, PackVersion)> {
        let pack_format_unspecified = self.pack_format <= 0;
        let supported_unspecified = self.supported_formats.is_unspecified();

        if self.min_format.is_unspecified() || self.max_format.is_unspecified() {
            // The old format of the metadata.
            if !supported_unspecified {
                let min = self.supported_formats.min;
                let max = self.supported_formats.max;
                if max > 64 {
                    return None;
                }
                if pack_format_unspecified {
                    return None;
                }
                if is_pack_format_invalid(min, max, self.pack_format) {
                    return None;
                }
                return Some((PackVersion::of_major(min), PackVersion::of_major(max)));
            }
            if !pack_format_unspecified {
                if self.pack_format > 64 {
                    return None;
                }
                let version = PackVersion::of_major(self.pack_format);
                return Some((version, version));
            }
            return None;
        }

        // The new format of the metadata.
        if self.min_format > self.max_format {
            return None;
        }

        let min_major = self.min_format.major;
        let max_major = self.max_format.major;

        if min_major > 64 {
            if !supported_unspecified {
                return None;
            }
            if !pack_format_unspecified
                && is_pack_format_invalid(min_major, max_major, self.pack_format)
            {
                return None;
            }
        } else {
            if supported_unspecified {
                return None;
            }
            if self.supported_formats.min != min_major {
                return None;
            }
            if self.supported_formats.max != max_major && self.supported_formats.max != 64 {
                return None;
            }
            if pack_format_unspecified {
                return None;
            }
            if is_pack_format_invalid(min_major, max_major, self.pack_format) {
                return None;
            }
        }

        Some((self.min_format, self.max_format))
    }
}

/// Returns whether the format of a pack is refused by the game.
///
/// The game refuses a format that is outside the range the pack names and a
/// format below 15, because those are the formats of the game before the pack
/// format was used.
fn is_pack_format_invalid(min: i32, max: i32, pack_format: i32) -> bool {
    if (min..=max).contains(&pack_format) {
        pack_format < 15
    } else {
        true
    }
}

/// Reads the formats a pack declares it supports.
fn parse_supported_formats(value: &Json) -> SupportedFormats {
    match value {
        Json::Array(items) if items.len() == 2 => match (items[0].as_i64(), items[1].as_i64()) {
            (Some(min), Some(max)) => SupportedFormats {
                min: min as i32,
                max: max as i32,
            },
            _ => SupportedFormats::UNSPECIFIED,
        },
        Json::Object(table) => {
            match (
                table.get("min_inclusive").and_then(Json::as_i64),
                table.get("max_inclusive").and_then(Json::as_i64),
            ) {
                (Some(min), Some(max)) => SupportedFormats {
                    min: min as i32,
                    max: max as i32,
                },
                _ => SupportedFormats::UNSPECIFIED,
            }
        }
        _ => SupportedFormats::UNSPECIFIED,
    }
}

/// Reads a version of the format of a pack.
///
/// A version is written as a number, as a list of the major number or as a list
/// of the major and the minor number. A major number alone names every minor
/// number of it when `any_minor` is set, which the last version of the pack
/// does, so the game reads the last format of the pack after the release of a
/// minor version it does not know.
fn parse_pack_version(value: &Json, any_minor: bool) -> PackVersion {
    let minor = if any_minor { i32::MAX } else { 0 };

    match value {
        Json::Number(number) => match number.as_i64() {
            Some(major) => PackVersion::new(major as i32, minor),
            None => PackVersion::UNSPECIFIED,
        },
        Json::Array(items) if items.len() == 1 => match items[0].as_i64() {
            Some(major) => PackVersion::new(major as i32, minor),
            None => PackVersion::UNSPECIFIED,
        },
        Json::Array(items) if items.len() == 2 => match (items[0].as_i64(), items[1].as_i64()) {
            (Some(major), Some(minor)) => PackVersion::new(major as i32, minor as i32),
            _ => PackVersion::UNSPECIFIED,
        },
        _ => PackVersion::UNSPECIFIED,
    }
}

/// Reads the description of a pack.
///
/// A description is a text or a component of the texts the game joins, so the
/// texts of the component are joined into a text.
fn parse_description(value: &Json) -> String {
    let mut parts = Vec::new();
    collect_description(value, &mut parts);
    parts.join("")
}

/// Collects the texts of a component of the description of a pack.
fn collect_description(value: &Json, parts: &mut Vec<String>) {
    match value {
        Json::String(text) => parts.push(text.clone()),
        Json::Object(table) => {
            if let Some(Json::String(text)) = table.get("text") {
                parts.push(text.clone());
            }
            if let Some(extra) = table.get("extra") {
                collect_description(extra, parts);
            }
        }
        Json::Array(items) => {
            for item in items {
                collect_description(item, parts);
            }
        }
        _ => {}
    }
}

/// The kind of a pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackKind {
    /// A pack the game draws the looks of the world with.
    ResourcePack,
    /// A pack the game reads the data of the world from.
    DataPack,
}

impl PackKind {
    /// Returns the name of the directory the packs of the kind are kept in.
    pub fn directory_name(self) -> &'static str {
        match self {
            Self::ResourcePack => "resourcepacks",
            Self::DataPack => "datapacks",
        }
    }

    /// Returns the name of the file the pack writes its metadata into.
    pub fn metadata_file(self) -> &'static str {
        match self {
            Self::ResourcePack => "pack.mcmeta",
            Self::DataPack => "pack.mcmeta",
        }
    }
}

/// A pack of a game directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    /// The file or the directory of the pack.
    path: PathBuf,
    /// The name of the file of the pack, without its extension.
    file_name: String,
    /// The name of the file of the pack.
    file_name_with_extension: String,
    /// The kind of the pack.
    kind: PackKind,
    /// The metadata of the pack, if it was read.
    info: Option<PackInfo>,
    /// The name of the pack with the color codes of the game read.
    color_stripped_name: String,
    /// Whether the game loads the pack.
    ///
    /// The resource packs are enabled by the options of the game, so the state
    /// of a pack of the manager is read from them. The data packs are enabled
    /// by the name of their file, which the game does not read the options of.
    active: bool,
}

/// The extension the launcher gives a pack that is not enabled.
///
/// The game loads the data packs by the name of the file, so a pack the player
/// disables is renamed instead of being recorded in a file.
pub const DISABLED_PACK_EXTENSION: &str = "disabled";

impl Pack {
    /// Reads a pack from a file or a directory.
    ///
    /// A directory that holds no `pack.mcmeta` is not a pack, and neither is a
    /// file that is not an archive with one. The pack is returned with no
    /// metadata when the file cannot be read, so the player still sees the
    /// file the game will read.
    pub fn open(path: impl AsRef<Path>, kind: PackKind) -> Option<Self> {
        let path = path.as_ref();
        let mut active = !has_disabled_extension(path);

        if path.is_dir() {
            // The game reads the metadata of a directory pack by its name, so
            // the launcher marks the pack as disabled by renaming the metadata.
            let enabled = path.join(kind.metadata_file()).is_file();
            let disabled = path
                .join(format!(
                    "{}.{DISABLED_PACK_EXTENSION}",
                    kind.metadata_file()
                ))
                .is_file();
            if !enabled && !disabled {
                return None;
            }
            active = enabled;
        } else if path.is_file() {
            // A zip file is a pack when it holds the metadata anywhere, because
            // the game reads the metadata of a pack from the top of the file.
            let entries = zip::entries(path).ok()?;
            let has_metadata = entries
                .iter()
                .any(|entry| entry.name == kind.metadata_file());
            if !has_metadata {
                return None;
            }
        } else {
            return None;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        // The name of a pack leaves out the extension of its archive and the
        // extension the launcher adds to a pack it disabled.
        let content_name = file_name.clone();
        let content_name = content_name
            .strip_suffix(&format!(".{DISABLED_PACK_EXTENSION}"))
            .unwrap_or(&content_name);
        let content_name = if path.is_file() {
            content_name
                .rsplit_once('.')
                .map(|(stem, _)| stem.to_string())
                .unwrap_or_else(|| content_name.to_string())
        } else {
            content_name.to_string()
        };

        let info = read_pack_info(path, kind);

        Some(Self {
            path: path.to_path_buf(),
            file_name: strip_color_codes(&content_name),
            file_name_with_extension: file_name.clone(),
            kind,
            info,
            color_stripped_name: strip_color_codes(&content_name),
            active,
        })
    }

    /// Returns whether the game loads the pack.
    ///
    /// The value of a pack that the manager read is the state of its name; the
    /// resource packs are enabled by the options of the game, so the manager
    /// tells the state from the options instead.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the file or the directory of the pack.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the name of the file of the pack, without its extension.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the name of the file of the pack.
    pub fn file_name_with_extension(&self) -> &str {
        &self.file_name_with_extension
    }

    /// Returns the name of the pack with the color codes of the game read.
    pub fn color_stripped_name(&self) -> &str {
        &self.color_stripped_name
    }

    /// Returns the kind of the pack.
    pub fn kind(&self) -> PackKind {
        self.kind
    }

    /// Returns the metadata of the pack, if it was read.
    pub fn info(&self) -> Option<&PackInfo> {
        self.info.as_ref()
    }

    /// Returns the description of the pack.
    pub fn description(&self) -> &str {
        self.info
            .as_ref()
            .map(|info| info.description.as_str())
            .unwrap_or("")
    }

    /// Returns whether the pack is compatible with the game of a version.
    pub fn compatibility(&self, required: PackVersion) -> PackCompatibility {
        let Some(info) = self.info.as_ref() else {
            return PackCompatibility::MissingPackMeta;
        };
        if required.is_unspecified() {
            return PackCompatibility::MissingGameMeta;
        }

        let range = if required.major > 64 {
            info.new_version_range()
        } else if required.major > 15 {
            info.old_version_range()
        } else {
            info.oldest_version_range()
        };

        let Some((minimum, maximum)) = range else {
            return PackCompatibility::Invalid;
        };

        if maximum < required {
            PackCompatibility::TooOld
        } else if minimum > required {
            PackCompatibility::TooNew
        } else {
            PackCompatibility::Compatible
        }
    }

    /// Reads the icon of the pack, if it has one.
    ///
    /// The game draws the icon of a pack from `pack.png` at the top of the pack,
    /// so the file of a directory or the entry of an archive is read. A pack
    /// without an icon, or with one that cannot be read, returns nothing, and
    /// the caller shows the icon of the launcher.
    pub fn icon(&self) -> Option<Vec<u8>> {
        if self.path.is_dir() {
            return std::fs::read(self.path.join(PACK_ICON)).ok();
        }
        zip::read_entry(&self.path, PACK_ICON).ok().flatten()
    }

    /// Enables or disables the pack by the name of its file.
    ///
    /// The game loads the data packs by the name of the file or the directory,
    /// so a pack that is not enabled keeps the extension `.disabled`. The
    /// resource packs are enabled by the options of the game, so the launcher
    /// does not rename them.
    ///
    /// Returns the pack at the new path, or nothing when the file cannot be
    /// renamed.
    pub fn set_active(&self, active: bool) -> Option<Self> {
        if active == self.active {
            return Some(self.clone());
        }

        // The metadata of a directory pack is renamed with the directory, so
        // the kind of the pack is read before the directory moves.
        let is_directory = self.path.is_dir();

        let target = if active {
            let name = self
                .file_name_with_extension
                .strip_suffix(&format!(".{DISABLED_PACK_EXTENSION}"))?
                .to_string();
            self.path.with_file_name(name)
        } else {
            self.path.with_file_name(format!(
                "{}.{DISABLED_PACK_EXTENSION}",
                self.file_name_with_extension
            ))
        };

        if target.exists() {
            return None;
        }

        std::fs::rename(&self.path, &target).ok()?;

        if is_directory {
            // The game reads the metadata of a directory pack by its name, so
            // the metadata is renamed with the directory.
            let metadata = target.join(self.kind.metadata_file());
            let disabled = target.join(format!(
                "{}.{DISABLED_PACK_EXTENSION}",
                self.kind.metadata_file()
            ));
            if active {
                let _ = std::fs::rename(disabled, metadata);
            } else {
                let _ = std::fs::rename(metadata, disabled);
            }
        }

        Self::open(&target, self.kind)
    }
}

/// The name of the file of the icon of a pack.
const PACK_ICON: &str = "pack.png";

/// Returns whether the name of a path holds the extension of a disabled pack.
fn has_disabled_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(DISABLED_PACK_EXTENSION))
}

/// How well a pack works with the game of a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackCompatibility {
    /// The game reads the pack.
    Compatible,
    /// The pack is written for a newer game.
    TooNew,
    /// The pack is written for an older game.
    TooOld,
    /// The pack names formats the game refuses.
    Invalid,
    /// The pack holds no `pack.mcmeta`.
    MissingPackMeta,
    /// The launcher cannot tell the format of the game.
    MissingGameMeta,
}

/// Reads the metadata of a pack from its file or its directory.
///
/// A directory pack that is disabled keeps the metadata under the name the
/// launcher gave it, so the launcher reads the metadata of the pack there.
fn read_pack_info(path: &Path, kind: PackKind) -> Option<PackInfo> {
    let data = if path.is_dir() {
        let enabled = path.join(kind.metadata_file());
        let disabled = path.join(format!(
            "{}.{DISABLED_PACK_EXTENSION}",
            kind.metadata_file()
        ));
        let metadata = if enabled.is_file() { enabled } else { disabled };
        std::fs::read(metadata).ok()?
    } else {
        zip::read_entry(path, kind.metadata_file()).ok().flatten()?
    };

    let text = String::from_utf8_lossy(&data);
    PackInfo::from_json(&text)
}

/// Removes the color codes of the game from a text.
///
/// The game writes the color of a text as a section sign followed by a letter,
/// so the letters are removed together with the sign.
fn strip_color_codes(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut characters = text.chars();

    while let Some(character) = characters.next() {
        if character == '\u{a7}' {
            // The letter after the sign names the color or the style.
            characters.next();
        } else {
            stripped.push(character);
        }
    }

    stripped
}

/// Returns the version of the formats the game of a version reads.
///
/// The game before 1.13 names the format in its own code, and the game of 1.13
/// and later writes it into `version.json` of the game jar. The launcher reads
/// that file when it has the jar; this function tells the format of a version
/// without the jar, so the options and the packs of a version are read even
/// when the jar is missing.
pub fn pack_version_of(version: &GameVersionNumber) -> PackVersion {
    // The format the game writes into `version.json`: the release the format
    // was introduced in, and the format of the packs of that release.
    const FORMATS: [(&str, i32); 12] = [
        ("1.6.1", 1),
        ("1.9", 2),
        ("1.11", 3),
        ("1.13", 4),
        ("1.15", 5),
        ("1.16.2", 6),
        ("1.17", 7),
        ("1.18", 8),
        ("1.19", 9),
        ("1.19.3", 12),
        ("1.20", 15),
        ("1.21", 34),
    ];

    if !version.is_at_least("1.6.1", "13w24a", false) {
        return PackVersion::UNSPECIFIED;
    }

    let mut known = PackVersion::of_major(1);
    for (release, format) in FORMATS {
        if version.is_at_least(release, release, false) {
            known = PackVersion::of_major(format);
        } else {
            break;
        }
    }

    known
}

/// Returns the version of the formats the game of a version reads, read from
/// its jar.
///
/// The game writes the version of the format into `version.json` of its jar
/// after 18w47b; the game before it names the format in its code, which the
/// launcher lists by the snapshot the format was introduced in. A jar that
/// cannot be read or holds no `version.json` gives the format
/// [`pack_version_of`] tells without the jar.
pub fn pack_version_of_jar(version: &GameVersionNumber, game_jar: &Path) -> PackVersion {
    // The snapshots the game named the format in its code, with the format of
    // the versions before each of them. After 18w47b the game writes the
    // format into its jar, so the list stops there.
    const OLD_FORMATS: [(&str, &str, i32); 4] = [
        ("1.9", "15w31a", 1),
        ("1.11", "16w32a", 2),
        ("1.13", "17w48a", 3),
        ("1.14", "18w47b", 4),
    ];

    if !version.is_at_least("1.6.1", "13w24a", false) {
        return PackVersion::UNSPECIFIED;
    }

    for (release, snapshot, format) in OLD_FORMATS {
        if !version.is_at_least(release, snapshot, false) {
            return PackVersion::of_major(format);
        }
    }

    let Ok(Some(bytes)) = zip::read_entry(game_jar, "version.json") else {
        return pack_version_of(version);
    };
    let Ok(json) = serde_json::from_slice::<Json>(&bytes) else {
        return pack_version_of(version);
    };

    // After 25w31a the game writes the format as an object with the major and
    // the minor number, and before it as a single number.
    let Some(pack_version) = json.get("pack_version") else {
        return pack_version_of(version);
    };

    if let Some(formats) = pack_version.as_object() {
        let resource = formats.get("resource").and_then(Json::as_i64).unwrap_or(0);
        let major = formats
            .get("resource_major")
            .and_then(Json::as_i64)
            .unwrap_or(0);
        let minor = formats
            .get("resource_minor")
            .and_then(Json::as_i64)
            .unwrap_or(0);

        // The major number is only written by the game of 1.21.9 and later; a
        // jar that writes the object without it names the format by the
        // number the older jars used.
        return if major > 64 {
            PackVersion::new(major as i32, minor as i32)
        } else {
            PackVersion::of_major(resource as i32)
        };
    }

    match pack_version.as_i64() {
        Some(format) => PackVersion::of_major(format as i32),
        None => pack_version_of(version),
    }
}

/// Returns whether the game of a version reads resource packs at all.
pub fn is_version_supported(version: &GameVersionNumber) -> bool {
    version.is_at_least("1.6.1", "13w24a", false)
}

/// Returns whether the game of a version writes the list of the packs with the
/// identifiers the newer versions use.
///
/// The game before 1.13 names a pack by the name of its file; the game of 1.13
/// and later names it by the path of the file, which the launcher writes as
/// `file/<name>`.
pub fn supports_new_options_format(version: &GameVersionNumber) -> bool {
    version.is_at_least("1.13", "17w43a", false)
}

/// Returns whether a file or a directory is a resource pack.
///
/// A directory is a resource pack when it holds a `pack.mcmeta`, and a file is
/// one when its name ends with `.zip`.
pub fn is_file_resource_pack(path: &Path) -> bool {
    if path.is_dir() {
        return path.join("pack.mcmeta").is_file();
    }
    if path.is_file() {
        return path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    }
    false
}

/// Returns whether a file or a directory is a data pack.
pub fn is_file_data_pack(path: &Path) -> bool {
    is_file_resource_pack(path)
}

/// The packs of a game directory.
#[derive(Debug, Clone)]
pub struct PackManager {
    /// The directory the packs are kept in.
    directory: PathBuf,
    /// The file the game keeps the enabled packs in.
    options_file: PathBuf,
    /// The kind of the packs of the manager.
    kind: PackKind,
    /// The version of the game, which decides the formats the packs need.
    game_version: Option<GameVersionNumber>,
    /// The jar of the game, which names the format the game reads.
    game_jar: Option<PathBuf>,
    /// The packs that were read.
    packs: Vec<Pack>,
    /// Whether the packs were read.
    loaded: bool,
}

impl PackManager {
    /// Creates a manager of the resource packs of a game directory.
    pub fn resource_packs(directory: impl Into<PathBuf>, options_file: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            options_file: options_file.into(),
            kind: PackKind::ResourcePack,
            game_version: None,
            game_jar: None,
            packs: Vec::new(),
            loaded: false,
        }
    }

    /// Creates a manager of the data packs of a world directory.
    ///
    /// The game keeps the data packs of a world in the world, and the packs are
    /// enabled by the level of the world, not by the options of the game.
    pub fn data_packs(directory: impl Into<PathBuf>) -> Self {
        let directory = directory.into();
        Self {
            directory,
            options_file: PathBuf::new(),
            kind: PackKind::DataPack,
            game_version: None,
            game_jar: None,
            packs: Vec::new(),
            loaded: false,
        }
    }

    /// Sets the version of the game the packs are read for.
    pub fn with_game_version(mut self, version: GameVersionNumber) -> Self {
        self.game_version = Some(version);
        self.loaded = false;
        self
    }

    /// Sets the jar of the game, which names the format the game reads.
    ///
    /// The game of 1.14 and later writes the format into `version.json` of its
    /// jar, so the launcher reads the format from the jar when it is given.
    /// A jar that is not set or cannot be read leaves the format to the
    /// version of the game.
    pub fn with_game_jar(mut self, jar: impl Into<PathBuf>) -> Self {
        self.game_jar = Some(jar.into());
        self.loaded = false;
        self
    }

    /// Returns the directory the packs are kept in.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Returns the kind of the packs of the manager.
    pub fn kind(&self) -> PackKind {
        self.kind
    }

    /// Returns the version of the game the packs are read for.
    ///
    /// The version is unknown when the caller did not set one, which is what a
    /// caller that only manages the files of the packs wants.
    pub fn game_version(&self) -> Option<&GameVersionNumber> {
        self.game_version.as_ref()
    }

    /// Returns the packs of the directory, sorted by their file names.
    pub fn packs(&mut self) -> Result<&[Pack], PackError> {
        if !self.loaded {
            self.refresh()?;
        }
        Ok(&self.packs)
    }

    /// Reads the directory of the packs again.
    pub fn refresh(&mut self) -> Result<(), PackError> {
        self.packs.clear();

        if self.directory.is_dir() {
            let entries = std::fs::read_dir(&self.directory).map_err(|error| PackError::Io {
                path: self.directory.clone(),
                error,
            })?;

            for entry in entries {
                let entry = entry.map_err(|error| PackError::Io {
                    path: self.directory.clone(),
                    error,
                })?;
                if let Some(pack) = Pack::open(entry.path(), self.kind) {
                    self.packs.push(pack);
                }
            }
        }

        // The resource packs are enabled by the options of the game, so the
        // state of a pack is read from the options after the directory.
        if self.kind == PackKind::ResourcePack {
            let options = read_options(&self.options_file);
            let new_format = self
                .game_version
                .as_ref()
                .is_some_and(supports_new_options_format);
            let enabled = options
                .get("resourcePacks")
                .and_then(|text| parse_option_list(text))
                .unwrap_or_default();
            let incompatible = options
                .get("incompatibleResourcePacks")
                .and_then(|text| parse_option_list(text))
                .unwrap_or_default();
            let required = self.pack_version();

            for pack in &mut self.packs {
                let name = pack.file_name_with_extension().to_string();
                let named = contains_pack(&enabled, &name, new_format);
                let listed = contains_pack(&incompatible, &name, new_format);
                let is_incompatible = required
                    .map(|version| pack.compatibility(version) != PackCompatibility::Compatible)
                    .unwrap_or(false);

                pack.active = named && is_incompatible == listed;
            }
        }

        self.packs.sort_by(|left, right| {
            left.file_name_with_extension
                .cmp(&right.file_name_with_extension)
        });

        self.loaded = true;
        Ok(())
    }

    /// Adds a pack to the directory.
    ///
    /// A directory is copied with its files; a file is copied as it is. The
    /// file the player chose is kept.
    pub fn import(&mut self, path: &Path) -> Result<&Pack, PackError> {
        let is_pack = match self.kind {
            PackKind::ResourcePack => is_file_resource_pack(path),
            PackKind::DataPack => is_file_data_pack(path),
        };
        if !is_pack {
            return Err(PackError::NotAPack {
                path: path.to_path_buf(),
            });
        }

        if !self.loaded {
            self.refresh()?;
        }

        std::fs::create_dir_all(&self.directory).map_err(|error| PackError::Io {
            path: self.directory.clone(),
            error,
        })?;

        let file_name = path
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default();
        let target = self.directory.join(&file_name);

        if path.is_dir() {
            copy_directory(path, &target).map_err(|error| PackError::Io {
                path: target.clone(),
                error,
            })?;
        } else {
            std::fs::copy(path, &target).map_err(|error| PackError::Io {
                path: target.clone(),
                error,
            })?;
        }

        let pack = Pack::open(&target, self.kind).ok_or_else(|| PackError::NotAPack {
            path: target.clone(),
        })?;
        self.packs.push(pack);
        self.packs.sort_by(|left, right| {
            left.file_name_with_extension
                .cmp(&right.file_name_with_extension)
        });

        let index = self
            .packs
            .iter()
            .position(|pack| pack.path() == target)
            .ok_or_else(|| PackError::NotAPack {
                path: target.clone(),
            })?;

        Ok(&self.packs[index])
    }

    /// Deletes the file or the directory of a pack.
    pub fn delete(&mut self, path: &Path) -> Result<(), PackError> {
        if path.is_dir() {
            std::fs::remove_dir_all(path).map_err(|error| PackError::Io {
                path: path.to_path_buf(),
                error,
            })?;
        } else {
            std::fs::remove_file(path).map_err(|error| PackError::Io {
                path: path.to_path_buf(),
                error,
            })?;
        }

        self.packs.retain(|pack| pack.path() != path);
        Ok(())
    }

    /// Returns whether the game loads a pack.
    ///
    /// A data pack is loaded by the name of its file, so its state is the one
    /// the name records. A resource pack is loaded by the options of the game,
    /// which name the packs that are incompatible as well, because the player
    /// has to confirm the packs written for another version.
    pub fn is_enabled(&self, pack: &Pack) -> bool {
        if self.kind == PackKind::DataPack {
            return pack.is_active();
        }

        let options = read_options(&self.options_file);
        let enabled = options
            .get("resourcePacks")
            .and_then(|text| parse_option_list(text))
            .unwrap_or_default();
        let incompatible = options
            .get("incompatibleResourcePacks")
            .and_then(|text| parse_option_list(text))
            .unwrap_or_default();

        let new_format = self
            .game_version
            .as_ref()
            .is_some_and(supports_new_options_format);

        let name = pack.file_name_with_extension();
        let named = contains_pack(&enabled, name, new_format);
        if !named {
            return false;
        }

        let incompatible_named = contains_pack(&incompatible, name, new_format);
        let is_incompatible = self
            .pack_version()
            .map(|version| pack.compatibility(version) != PackCompatibility::Compatible)
            .unwrap_or(false);

        is_incompatible == incompatible_named
    }

    /// Enables the packs in the options of the game.
    ///
    /// Returns whether the options were changed.
    pub fn enable(&self, packs: &[&Pack]) -> bool {
        if self.kind == PackKind::DataPack {
            return self.set_data_packs_active(packs, true);
        }

        let mut options = read_options(&self.options_file);
        let mut enabled = options
            .get("resourcePacks")
            .and_then(|text| parse_option_list(text))
            .unwrap_or_default();
        let mut incompatible = options
            .get("incompatibleResourcePacks")
            .and_then(|text| parse_option_list(text))
            .unwrap_or_default();

        let new_format = self
            .game_version
            .as_ref()
            .is_some_and(supports_new_options_format);
        let required = self.pack_version();
        let mut modified = false;

        for pack in packs {
            let name = pack.file_name_with_extension();
            let id = format!("file/{name}");

            if !contains_pack(&enabled, name, new_format) {
                enabled.push(if new_format {
                    id.clone()
                } else {
                    name.to_string()
                });
                modified = true;
            }

            let is_incompatible = required
                .map(|version| pack.compatibility(version) != PackCompatibility::Compatible)
                .unwrap_or(false);
            let is_listed = contains_pack(&incompatible, name, new_format);

            if is_incompatible && !is_listed {
                incompatible.push(if new_format { id } else { name.to_string() });
                modified = true;
            } else if !is_incompatible && is_listed {
                incompatible.retain(|entry| entry != &id && entry != name);
                modified = true;
            }
        }

        if modified {
            options.insert("resourcePacks".to_string(), format_option_list(&enabled));
            options.insert(
                "incompatibleResourcePacks".to_string(),
                format_option_list(&incompatible),
            );
            let _ = write_options(&self.options_file, &options);
        }

        modified
    }

    /// Disables the packs in the options of the game.
    ///
    /// Returns whether the options were changed.
    pub fn disable(&self, packs: &[&Pack]) -> bool {
        if self.kind == PackKind::DataPack {
            return self.set_data_packs_active(packs, false);
        }

        let mut options = read_options(&self.options_file);
        let Some(enabled_text) = options.get("resourcePacks") else {
            return false;
        };
        let Some(incompatible_text) = options.get("incompatibleResourcePacks") else {
            return false;
        };

        let mut enabled = parse_option_list(enabled_text).unwrap_or_default();
        let mut incompatible = parse_option_list(incompatible_text).unwrap_or_default();
        let mut modified = false;

        for pack in packs {
            let name = pack.file_name_with_extension();
            let id = format!("file/{name}");

            for list in [&mut enabled, &mut incompatible] {
                let before = list.len();
                list.retain(|entry| entry != &id && entry != name);
                if list.len() != before {
                    modified = true;
                }
            }
        }

        if modified {
            options.insert("resourcePacks".to_string(), format_option_list(&enabled));
            options.insert(
                "incompatibleResourcePacks".to_string(),
                format_option_list(&incompatible),
            );
            let _ = write_options(&self.options_file, &options);
        }

        modified
    }

    /// Makes the options name a pack by its new name.
    ///
    /// The game names the packs by the name of the file, so a pack the player
    /// renamed is named by the new name in the options as well.
    pub fn rename(&self, old_name: &str, new_name: &str) -> bool {
        if old_name == new_name {
            return false;
        }

        let mut options = read_options(&self.options_file);
        let old_id = format!("file/{old_name}");
        let new_id = format!("file/{new_name}");
        let new_format = self
            .game_version
            .as_ref()
            .is_some_and(supports_new_options_format);
        let mut modified = false;

        for key in ["resourcePacks", "incompatibleResourcePacks"] {
            let Some(text) = options.get(key) else {
                continue;
            };
            let mut list = parse_option_list(text).unwrap_or_default();
            let mut changed = false;

            for entry in list.iter_mut() {
                if entry == old_name || entry == &old_id {
                    *entry = if new_format {
                        new_id.clone()
                    } else {
                        new_name.to_string()
                    };
                    changed = true;
                }
            }

            if changed {
                options.insert(key.to_string(), format_option_list(&list));
                modified = true;
            }
        }

        if modified {
            let _ = write_options(&self.options_file, &options);
        }

        modified
    }

    /// Returns the version of the formats the game reads.
    fn pack_version(&self) -> Option<PackVersion> {
        self.game_version
            .as_ref()
            .map(|version| match &self.game_jar {
                Some(jar) => pack_version_of_jar(version, jar),
                None => pack_version_of(version),
            })
    }

    /// Enables or disables the data packs by the names of their files.
    ///
    /// The game loads the data packs of a world by the name of each file or
    /// directory, so a pack that is not enabled keeps the extension `.disabled`.
    /// A pack that is already in the state the caller asks for is left alone.
    ///
    /// Returns whether a pack was renamed, so the caller knows to read the
    /// directory again.
    fn set_data_packs_active(&self, packs: &[&Pack], active: bool) -> bool {
        let mut modified = false;

        for pack in packs {
            if pack.is_active() == active {
                continue;
            }
            if pack.set_active(active).is_some() {
                modified = true;
            }
        }

        modified
    }
}

/// Returns whether the list of the packs of an option names a pack.
fn contains_pack(entries: &[String], name: &str, new_format: bool) -> bool {
    let id = format!("file/{name}");
    entries.iter().any(|entry| {
        if new_format {
            entry == name || entry == &id
        } else {
            entry == name
        }
    })
}

/// Reads the options of the game.
///
/// The options are a line for every option, where the name and the value are
/// separated by a colon. A line without a colon is skipped, because the game
/// writes the options that way and the launcher only reads the ones it needs.
pub fn read_options(path: &Path) -> std::collections::BTreeMap<String, String> {
    let mut options = std::collections::BTreeMap::new();
    let Ok(data) = std::fs::read(path) else {
        return options;
    };

    let text = String::from_utf8_lossy(&data);
    for line in text.lines() {
        if !is_not_blank(line) {
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            options.insert(name.to_string(), value.to_string());
        }
    }

    options
}

/// Writes the options of the game.
///
/// The options are written next to the file and moved into place, so a failure
/// does not leave a half-written file the game cannot read.
pub fn write_options(
    path: &Path,
    options: &std::collections::BTreeMap<String, String>,
) -> std::io::Result<()> {
    let mut text = String::new();
    for (name, value) in options {
        text.push_str(name);
        text.push(':');
        text.push_str(value);
        text.push('\n');
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let temporary = path.with_extension("txt.tmp");
    std::fs::write(&temporary, text)?;
    match std::fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            Err(error)
        }
    }
}

/// Reads the list of the packs of an option.
///
/// The game writes the list as a list of JSON texts, so a list that cannot be
/// read is read as an empty list.
pub fn parse_option_list(text: &str) -> Option<Vec<String>> {
    let value: Json = serde_json::from_str(text).ok()?;
    let items = value.as_array()?;
    Some(
        items
            .iter()
            .filter_map(Json::as_str)
            .map(str::to_string)
            .collect(),
    )
}

/// Writes the list of the packs of an option.
pub fn format_option_list(items: &[String]) -> String {
    let mut text = String::from("[");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        text.push_str(&Json::String(item.clone()).to_string());
    }
    text.push(']');
    text
}

/// Copies a directory with its files.
fn copy_directory(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let destination = to.join(entry.file_name());
        if source.is_dir() {
            copy_directory(&source, &destination)?;
        } else {
            std::fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-packs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    #[test]
    fn the_metadata_of_a_pack_is_read() {
        let info =
            PackInfo::from_json(r#"{ "pack": { "pack_format": 15, "description": "My Pack" } }"#)
                .expect("the metadata should be read");

        assert_eq!(info.pack_format, 15);
        assert_eq!(info.description, "My Pack");
        assert_eq!(info.effective_min_version(), PackVersion::new(15, 0));
        assert_eq!(info.effective_max_version(), PackVersion::new(15, 0));
        assert_eq!(
            info.old_version_range(),
            Some((PackVersion::new(15, 0), PackVersion::new(15, 0)))
        );
    }

    #[test]
    fn the_metadata_of_a_pack_with_a_range_of_formats_is_read() {
        let info = PackInfo::from_json(
            r#"{ "pack": { "pack_format": 15, "supported_formats": [ 15, 18 ] } }"#,
        )
        .expect("the metadata should be read");

        assert_eq!(
            info.supported_formats,
            SupportedFormats { min: 15, max: 18 }
        );
        assert_eq!(
            info.old_version_range(),
            Some((PackVersion::new(15, 0), PackVersion::new(18, 0)))
        );
    }

    #[test]
    fn the_metadata_of_a_pack_with_a_table_of_formats_is_read() {
        let info = PackInfo::from_json(
            r#"{ "pack": { "pack_format": 15, "supported_formats": { "min_inclusive": 15, "max_inclusive": 20 } } }"#,
        )
        .expect("the metadata should be read");

        assert_eq!(
            info.supported_formats,
            SupportedFormats { min: 15, max: 20 }
        );
    }

    #[test]
    fn the_metadata_of_a_pack_with_the_new_format_is_read() {
        let info = PackInfo::from_json(
            r#"{ "pack": { "pack_format": 65, "min_format": [ 65, 0 ], "max_format": [ 70, 1 ] } }"#,
        )
        .expect("the metadata should be read");

        assert_eq!(info.min_format, PackVersion::new(65, 0));
        assert_eq!(info.max_format, PackVersion::new(70, 1));
        assert_eq!(
            info.new_version_range(),
            Some((PackVersion::new(65, 0), PackVersion::new(70, 1)))
        );
    }

    #[test]
    fn a_pack_without_the_pack_of_the_metadata_names_nothing() {
        assert!(PackInfo::from_json(r#"{ "pack_format": 15 }"#).is_none());
        assert!(PackInfo::from_json("not json").is_none());
    }

    #[test]
    fn the_description_of_a_pack_is_joined() {
        let info = PackInfo::from_json(
            r#"{ "pack": { "pack_format": 15, "description": { "text": "My ", "extra": [ "Pack" ] } } }"#,
        )
        .expect("the metadata should be read");
        assert_eq!(info.description, "My Pack");
    }

    #[test]
    fn the_versions_of_the_formats_are_ordered() {
        assert!(PackVersion::new(15, 0) < PackVersion::new(18, 0));
        assert!(PackVersion::new(15, 0) < PackVersion::new(15, 1));
        assert!(PackVersion::UNSPECIFIED < PackVersion::new(1, 0));
        assert_eq!(PackVersion::new(0, 0).to_string(), "0");
        assert_eq!(PackVersion::new(65, 1).to_string(), "65.1");
        assert_eq!(PackVersion::UNSPECIFIED.to_string(), "UNSPECIFIED");
    }

    #[test]
    fn a_pack_of_the_new_format_names_its_minor_version() {
        let info = PackInfo::from_json(
            r#"{ "pack": { "pack_format": 65, "min_format": 65, "max_format": 70 } }"#,
        )
        .expect("the metadata should be read");

        // The first version names the minor version zero, the last names every
        // minor version of the major number.
        assert_eq!(info.min_format, PackVersion::new(65, 0));
        assert_eq!(info.max_format, PackVersion::new(70, i32::MAX));
    }

    #[test]
    fn a_pack_is_read_from_a_directory() {
        let base = directory("read-directory");
        let pack = base.join("MyPack");
        std::fs::create_dir_all(&pack).expect("dir");
        std::fs::write(
            pack.join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 15, "description": "Mine" } }"#,
        )
        .expect("write");

        let pack = Pack::open(&pack, PackKind::ResourcePack).expect("the pack should be read");
        assert_eq!(pack.file_name(), "MyPack");
        assert_eq!(pack.description(), "Mine");
        assert_eq!(
            pack.compatibility(PackVersion::new(15, 0)),
            PackCompatibility::Compatible
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_directory_without_metadata_is_not_a_pack() {
        let base = directory("no-metadata");
        assert!(Pack::open(&base, PackKind::ResourcePack).is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_pack_of_another_version_is_told_apart() {
        let info = PackInfo::from_json(r#"{ "pack": { "pack_format": 15 } }"#)
            .expect("the metadata should be read");

        let pack = Pack {
            path: PathBuf::from("a.zip"),
            file_name: "a".to_string(),
            file_name_with_extension: "a.zip".to_string(),
            kind: PackKind::ResourcePack,
            info: Some(info),
            color_stripped_name: "a".to_string(),
            active: false,
        };

        assert_eq!(
            pack.compatibility(PackVersion::new(15, 0)),
            PackCompatibility::Compatible
        );
        assert_eq!(
            pack.compatibility(PackVersion::new(18, 0)),
            PackCompatibility::TooOld
        );
        assert_eq!(
            pack.compatibility(PackVersion::new(12, 0)),
            PackCompatibility::TooNew
        );
        assert_eq!(
            pack.compatibility(PackVersion::UNSPECIFIED),
            PackCompatibility::MissingGameMeta
        );
    }

    #[test]
    fn a_pack_of_the_oldest_format_is_read_by_the_format_it_is_written_for() {
        let info = PackInfo::from_json(
            r#"{ "pack": { "pack_format": 3, "supported_formats": [ 3, 5 ] } }"#,
        )
        .expect("the metadata should be read");

        // The game before the list of the formats was added reads the format of
        // the pack, not the range it names.
        assert_eq!(
            info.oldest_version_range(),
            Some((PackVersion::new(3, 0), PackVersion::new(3, 0)))
        );
        assert_eq!(
            info.old_version_range(),
            Some((PackVersion::new(3, 0), PackVersion::new(5, 0)))
        );
    }

    #[test]
    fn the_packs_of_a_directory_are_read() {
        let base = directory("list");
        let packs = base.join("resourcepacks");
        std::fs::create_dir_all(packs.join("first")).expect("dir");
        std::fs::write(
            packs.join("first").join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 15 } }"#,
        )
        .expect("write");
        std::fs::create_dir_all(packs.join("not-a-pack")).expect("dir");

        let mut manager = PackManager::resource_packs(&packs, base.join("options.txt"));
        let read = manager.packs().expect("the packs should be read");

        assert_eq!(read.len(), 1);
        assert_eq!(read[0].file_name(), "first");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_pack_is_enabled_and_disabled_in_the_options() {
        let base = directory("enable");
        let packs = base.join("resourcepacks");
        std::fs::create_dir_all(packs.join("first")).expect("dir");
        std::fs::write(
            packs.join("first").join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 15 } }"#,
        )
        .expect("write");
        std::fs::write(
            base.join("options.txt"),
            "resourcePacks:[]\nincompatibleResourcePacks:[]\n",
        )
        .expect("write");

        let version = GameVersionNumber::as_game_version("1.20.1");
        let mut manager = PackManager::resource_packs(&packs, base.join("options.txt"))
            .with_game_version(version);
        let pack = manager.packs().expect("the packs should be read")[0].clone();

        assert!(!manager.is_enabled(&pack));
        assert!(manager.enable(&[&pack]));

        let options = read_options(&base.join("options.txt"));
        assert_eq!(
            options.get("resourcePacks").map(String::as_str),
            Some(r#"["file/first"]"#)
        );
        assert!(manager.is_enabled(&pack));

        assert!(manager.disable(&[&pack]));
        let options = read_options(&base.join("options.txt"));
        assert_eq!(options.get("resourcePacks").map(String::as_str), Some("[]"));
        assert!(!manager.is_enabled(&pack));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_pack_of_another_version_is_listed_as_incompatible() {
        let base = directory("incompatible");
        let packs = base.join("resourcepacks");
        std::fs::create_dir_all(packs.join("old")).expect("dir");
        std::fs::write(
            packs.join("old").join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 12 } }"#,
        )
        .expect("write");
        std::fs::write(
            base.join("options.txt"),
            "resourcePacks:[]\nincompatibleResourcePacks:[]\n",
        )
        .expect("write");

        let version = GameVersionNumber::as_game_version("1.20.1");
        let mut manager = PackManager::resource_packs(&packs, base.join("options.txt"))
            .with_game_version(version);
        let pack = manager.packs().expect("the packs should be read")[0].clone();

        manager.enable(&[&pack]);

        let options = read_options(&base.join("options.txt"));
        assert_eq!(
            options.get("incompatibleResourcePacks").map(String::as_str),
            Some(r#"["file/old"]"#)
        );
        assert!(manager.is_enabled(&pack));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_old_game_names_a_pack_by_its_file_name() {
        let base = directory("old-format");
        let packs = base.join("resourcepacks");
        std::fs::create_dir_all(packs.join("first")).expect("dir");
        std::fs::write(
            packs.join("first").join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 4 } }"#,
        )
        .expect("write");
        std::fs::write(base.join("options.txt"), "resourcePacks:[]\n").expect("write");

        let version = GameVersionNumber::as_game_version("1.12.2");
        let mut manager = PackManager::resource_packs(&packs, base.join("options.txt"))
            .with_game_version(version);
        let pack = manager.packs().expect("the packs should be read")[0].clone();

        manager.enable(&[&pack]);

        let options = read_options(&base.join("options.txt"));
        assert_eq!(
            options.get("resourcePacks").map(String::as_str),
            Some(r#"["first"]"#)
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_pack_is_imported_and_deleted() {
        let base = directory("import");
        let packs = base.join("resourcepacks");
        std::fs::create_dir_all(&packs).expect("dir");
        std::fs::create_dir_all(base.join("source")).expect("dir");
        std::fs::write(
            base.join("source").join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 15 } }"#,
        )
        .expect("write");

        let mut manager = PackManager::resource_packs(&packs, base.join("options.txt"));
        let pack = manager
            .import(&base.join("source"))
            .expect("the pack should be imported");
        assert_eq!(pack.file_name(), "source");
        assert!(packs.join("source").is_dir());

        manager
            .delete(&packs.join("source"))
            .expect("the pack should be deleted");
        assert!(!packs.join("source").exists());
        assert!(manager.packs.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_that_is_not_a_pack_is_refused() {
        let base = directory("not-a-pack");
        std::fs::create_dir_all(base.join("resourcepacks")).expect("dir");
        std::fs::write(base.join("notes.txt"), b"").expect("write");

        let mut manager =
            PackManager::resource_packs(base.join("resourcepacks"), base.join("options.txt"));
        let error = manager
            .import(&base.join("notes.txt"))
            .expect_err("the file should be refused");
        assert!(matches!(error, PackError::NotAPack { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_color_codes_of_the_names_are_removed() {
        assert_eq!(strip_color_codes("\u{a7}6My Pack"), "My Pack");
        assert_eq!(strip_color_codes("Plain"), "Plain");
    }

    #[test]
    fn the_lists_of_the_options_are_read_and_written() {
        assert_eq!(
            parse_option_list(r#"["a","b"]"#),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(parse_option_list("not json"), None);
        assert_eq!(
            format_option_list(&["a".to_string(), "b".to_string()]),
            r#"["a","b"]"#
        );
    }

    #[test]
    fn the_formats_of_the_game_are_read() {
        assert_eq!(
            pack_version_of(&GameVersionNumber::as_game_version("1.20.1")),
            PackVersion::new(15, 0)
        );
        assert_eq!(
            pack_version_of(&GameVersionNumber::as_game_version("1.21.1")),
            PackVersion::new(34, 0)
        );
        assert!(pack_version_of(&GameVersionNumber::as_game_version("1.5.2")).is_unspecified());
        assert!(is_version_supported(&GameVersionNumber::as_game_version(
            "1.12.2"
        )));
        assert!(!is_version_supported(&GameVersionNumber::as_game_version(
            "1.5.2"
        )));
        assert!(supports_new_options_format(
            &GameVersionNumber::as_game_version("1.20.1")
        ));
        assert!(!supports_new_options_format(
            &GameVersionNumber::as_game_version("1.12.2")
        ));
    }

    #[test]
    fn the_format_of_the_game_is_read_from_its_jar() {
        let base = directory("pack-version-jar");

        // The game before 18w47b names the format in its code, so the jar is
        // not read at all.
        let jar = base.join("old.jar");
        assert_eq!(
            pack_version_of_jar(&GameVersionNumber::as_game_version("1.12.2"), &jar),
            PackVersion::new(3, 0)
        );

        // The game of 1.14 to 1.21 writes the format as a single number.
        let mut writer = zip::ZipWriter::create(&jar).expect("the archive should be made");
        writer
            .add_file("version.json", br#"{ "pack_version": 42 }"#)
            .expect("the version should be added");
        writer.finish().expect("the archive should be written");
        assert_eq!(
            pack_version_of_jar(&GameVersionNumber::as_game_version("1.21.1"), &jar),
            PackVersion::new(42, 0)
        );

        // The game of 25w31a and later writes the major and the minor number.
        let mut writer = zip::ZipWriter::create(&jar).expect("the archive should be made");
        writer
            .add_file(
                "version.json",
                br#"{ "pack_version": { "resource": 64, "resource_major": 65, "resource_minor": 1 } }"#,
            )
            .expect("the version should be added");
        writer.finish().expect("the archive should be written");
        assert_eq!(
            pack_version_of_jar(&GameVersionNumber::as_game_version("1.21.9"), &jar),
            PackVersion::new(65, 1)
        );

        // A jar without a version file falls back to the version the launcher
        // knows without it.
        let empty = base.join("empty.jar");
        let writer = zip::ZipWriter::create(&empty).expect("the archive should be made");
        writer.finish().expect("the archive should be written");
        assert_eq!(
            pack_version_of_jar(&GameVersionNumber::as_game_version("1.21.1"), &empty),
            pack_version_of(&GameVersionNumber::as_game_version("1.21.1"))
        );

        // A jar that cannot be read gives the same fallback.
        assert_eq!(
            pack_version_of_jar(
                &GameVersionNumber::as_game_version("1.21.1"),
                &base.join("missing.jar")
            ),
            pack_version_of(&GameVersionNumber::as_game_version("1.21.1"))
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_icon_of_the_pack_is_read() {
        let base = directory("icon");
        std::fs::create_dir_all(base.join("folder")).expect("dir");
        std::fs::write(
            base.join("folder").join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 15 } }"#,
        )
        .expect("write");

        let pack = Pack::open(base.join("folder"), PackKind::ResourcePack).expect("the pack");
        // A pack without an icon has none.
        assert_eq!(pack.icon(), None);

        std::fs::write(base.join("folder").join("pack.png"), b"png").expect("write");
        let pack = Pack::open(base.join("folder"), PackKind::ResourcePack).expect("the pack");
        assert_eq!(pack.icon().as_deref(), Some(b"png".as_slice()));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_data_packs_are_enabled_by_the_names_of_their_files() {
        let base = directory("data-packs");
        let packs = base.join("datapacks");
        std::fs::create_dir_all(packs.join("first")).expect("dir");
        std::fs::write(
            packs.join("first").join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 15 } }"#,
        )
        .expect("write");

        let mut manager = PackManager::data_packs(&packs);
        let pack = manager.packs().expect("the packs should be read")[0].clone();
        assert!(manager.is_enabled(&pack));

        // A data pack is disabled by the extension of its directory, so the
        // game does not load it.
        assert!(manager.disable(&[&pack]));
        assert!(packs.join("first.disabled").is_dir());
        assert!(!packs.join("first").exists());

        manager.refresh().expect("the packs should be read");
        let pack = manager.packs().expect("the packs should be read")[0].clone();
        assert!(!pack.is_active());
        assert!(!manager.is_enabled(&pack));
        assert_eq!(pack.file_name(), "first");
        // The metadata of a disabled pack is still read.
        assert!(pack.info().is_some());

        assert!(manager.enable(&[&pack]));
        assert!(packs.join("first").is_dir());
        assert!(!packs.join("first.disabled").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_disabled_zip_data_pack_is_read() {
        let base = directory("data-zip");
        let packs = base.join("datapacks");
        std::fs::create_dir_all(&packs).expect("dir");

        // The archives of the data packs are disabled by the extension of the
        // file itself.
        let mut writer =
            crate::util::zip::ZipWriter::create(packs.join("pack.zip.disabled")).expect("the zip");
        writer
            .add_file("pack.mcmeta", br#"{ "pack": { "pack_format": 15 } }"#)
            .expect("the file should be added");
        writer.finish().expect("the zip should be written");

        let mut manager = PackManager::data_packs(&packs);
        let packs = manager.packs().expect("the packs should be read");
        assert_eq!(packs.len(), 1);
        assert!(!packs[0].is_active());
        assert_eq!(packs[0].file_name(), "pack");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_resource_packs_are_read_from_the_options() {
        let base = directory("options-state");
        let packs = base.join("resourcepacks");
        std::fs::create_dir_all(packs.join("first")).expect("dir");
        std::fs::write(
            packs.join("first").join("pack.mcmeta"),
            r#"{ "pack": { "pack_format": 15 } }"#,
        )
        .expect("write");
        std::fs::write(
            base.join("options.txt"),
            "resourcePacks:[\"file/first\"]\nincompatibleResourcePacks:[]\n",
        )
        .expect("write");

        let version = GameVersionNumber::as_game_version("1.20.1");
        let mut manager = PackManager::resource_packs(&packs, base.join("options.txt"))
            .with_game_version(version);
        let pack = manager.packs().expect("the packs should be read")[0].clone();

        // The packs read by the manager tell the state the options record.
        assert!(pack.is_active());
        assert!(manager.is_enabled(&pack));

        let _ = std::fs::remove_dir_all(&base);
    }
}
