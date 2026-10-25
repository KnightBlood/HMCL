//! The JSON schemas of the settings of the launcher.
//!
//! Every settings file starts with the address of the JSON schema that
//! describes it. The address tells the launcher which version of the file it
//! reads, so a file of a newer launcher is not overwritten by an older one.

use std::fmt;

use serde_json::Value;

/// The beginning of the address of the schemas of the launcher.
pub const SCHEMA_URL_PREFIX: &str = "https://schemas.glavo.site/hmcl/";

/// The name of the member that holds the address of the schema.
pub const SCHEMA_PROPERTY: &str = "$schema";

/// The version of a JSON schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaVersion {
    /// The main version of the schema.
    major: u32,
    /// The version of the schema inside the main version.
    minor: u32,
    /// The version of the schema inside the other versions.
    patch: u32,
}

impl SchemaVersion {
    /// Creates a version of a schema.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Reads a version of a schema.
    ///
    /// The version is written as `major.minor` or as `major.minor.patch`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let major = parts.next().and_then(parse_part)?;
        let minor = parts.next().and_then(parse_part)?;
        let patch = match parts.next() {
            Some(part) => parse_part(part)?,
            None => 0,
        };

        if parts.next().is_some() {
            return None;
        }

        Some(Self::new(major, minor, patch))
    }

    /// Returns the main version of the schema.
    pub const fn major(self) -> u32 {
        self.major
    }

    /// Returns the version of the schema inside the main version.
    pub const fn minor(self) -> u32 {
        self.minor
    }

    /// Returns the version of the schema inside the other versions.
    pub const fn patch(self) -> u32 {
        self.patch
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Reads a part of a version of a schema.
fn parse_part(text: &str) -> Option<u32> {
    if text.is_empty() || !text.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// The JSON schema of a settings file.
///
/// The address of the schema is kept as it is, so a file of a newer launcher
/// keeps its address when it is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    /// The address of the schema, as it is written in the file.
    value: String,
    /// The name and the version of the schema, if the address is one of the
    /// schemas of the launcher.
    parsed: Option<(String, SchemaVersion)>,
}

impl Schema {
    /// Creates a schema from an address.
    pub fn parse(value: &str) -> Self {
        Self {
            value: value.to_string(),
            parsed: parse_schema_url(value),
        }
    }

    /// Creates a schema from the name and the version of the schema.
    pub fn new(id: &str, version: SchemaVersion) -> Self {
        Self {
            value: format!("{SCHEMA_URL_PREFIX}{id}/{version}"),
            parsed: Some((id.to_string(), version)),
        }
    }

    /// Returns the address of the schema.
    pub fn url(&self) -> &str {
        &self.value
    }

    /// Returns the name of the schema, if the address is one of the schemas
    /// of the launcher.
    pub fn id(&self) -> Option<&str> {
        self.parsed.as_ref().map(|(id, _)| id.as_str())
    }

    /// Returns the version of the schema, if the address is one of the
    /// schemas of the launcher.
    pub fn version(&self) -> Option<SchemaVersion> {
        self.parsed.as_ref().map(|(_, version)| *version)
    }

    /// Returns whether the address is one of the schemas of the launcher.
    pub fn is_parsed(&self) -> bool {
        self.parsed.is_some()
    }
}

impl fmt::Display for Schema {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)
    }
}

/// Reads the name and the version of a schema from an address.
fn parse_schema_url(value: &str) -> Option<(String, SchemaVersion)> {
    let path = value.strip_prefix(SCHEMA_URL_PREFIX)?;
    let (id, version_text) = path.split_once('/')?;

    if id.is_empty() || !is_valid_id(id) {
        return None;
    }

    let version = SchemaVersion::parse(version_text)?;

    // The address of a schema is written with all three parts, unless the
    // last one is zero.
    let short = format!("{}.{}", version.major(), version.minor());
    if version_text != version.to_string() && !(version.patch() == 0 && version_text == short) {
        return None;
    }

    Some((id.to_string(), version))
}

/// Returns whether the name of a schema is valid.
fn is_valid_id(id: &str) -> bool {
    let mut characters = id.chars();
    match characters.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }

    characters.all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    })
}

/// The way the launcher may use a file, after the schema of the file was
/// compared with the schema the launcher knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityStatus {
    /// The file holds an older schema and may be written again.
    ReadWrite,
    /// The file holds the same schema and may be written again.
    ReadWritePreserveSchema,
    /// The file holds a newer schema, so it may be read but not written.
    ReadOnlyPreserveSchema,
    /// The file does not name a schema.
    Missing,
    /// The file names a schema that is not a text.
    Invalid,
    /// The file names a schema the launcher does not know.
    Unparseable,
    /// The file holds the schema of another kind of settings.
    UnexpectedId,
    /// The file holds a schema of another main version.
    UnsupportedMajor,
}

impl CompatibilityStatus {
    /// Returns whether the file may be read.
    pub const fn readable(self) -> bool {
        matches!(
            self,
            Self::ReadWrite | Self::ReadWritePreserveSchema | Self::ReadOnlyPreserveSchema
        )
    }

    /// Returns whether the file may be written again.
    pub const fn allow_save(self) -> bool {
        matches!(self, Self::ReadWrite | Self::ReadWritePreserveSchema)
    }

    /// Returns whether the address of the schema of the file must be kept.
    pub const fn preserve_schema(self) -> bool {
        matches!(
            self,
            Self::ReadWritePreserveSchema | Self::ReadOnlyPreserveSchema
        )
    }
}

/// The result of the comparison of the schema of a file with the schema the
/// launcher knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compatibility {
    /// The schema of the file, if the file names one.
    pub actual: Option<Schema>,
    /// The way the launcher may use the file.
    pub status: CompatibilityStatus,
    /// The text of the address of the schema, if the address is not a text.
    pub invalid_value: Option<String>,
}

impl Compatibility {
    /// Returns whether the file may be read.
    pub fn readable(&self) -> bool {
        self.status.readable()
    }

    /// Returns whether the file may be written again.
    pub fn allow_save(&self) -> bool {
        self.status.allow_save()
    }

    /// Returns whether the address of the schema of the file must be kept.
    pub fn preserve_schema(&self) -> bool {
        self.status.preserve_schema()
    }
}

/// Compares the schema of a file with the schema the launcher knows.
///
/// The rules are the rules of the Java launcher:
///
/// * a file without a schema cannot be read,
/// * a file of another kind of settings cannot be read,
/// * a file of another main version cannot be read,
/// * a file of a newer version may be read, but not written,
/// * a file of the same version may be read and written, and keeps its
///   address,
/// * a file of an older version may be read and written, and the address is
///   replaced with the one the launcher knows.
pub fn check(object: &Value, expected: &Schema) -> Compatibility {
    let expected_parsed = match expected.parsed.as_ref() {
        Some(parsed) => parsed,
        None => {
            return Compatibility {
                actual: None,
                status: CompatibilityStatus::Unparseable,
                invalid_value: None,
            };
        }
    };

    let member = match object.get(SCHEMA_PROPERTY) {
        Some(member) => member,
        None => {
            return Compatibility {
                actual: None,
                status: CompatibilityStatus::Missing,
                invalid_value: None,
            };
        }
    };

    let value = match member.as_str() {
        Some(value) => value,
        None => {
            return Compatibility {
                actual: None,
                status: CompatibilityStatus::Invalid,
                invalid_value: Some(member.to_string()),
            };
        }
    };

    let actual = Schema::parse(value);
    let actual_parsed = match actual.parsed.as_ref() {
        Some(parsed) => parsed,
        None => {
            return Compatibility {
                actual: Some(actual),
                status: CompatibilityStatus::Unparseable,
                invalid_value: None,
            };
        }
    };

    if actual_parsed.0 != expected_parsed.0 {
        return Compatibility {
            actual: Some(actual),
            status: CompatibilityStatus::UnexpectedId,
            invalid_value: None,
        };
    }

    if actual_parsed.1.major() != expected_parsed.1.major() {
        return Compatibility {
            actual: Some(actual),
            status: CompatibilityStatus::UnsupportedMajor,
            invalid_value: None,
        };
    }

    if actual_parsed.1.minor() > expected_parsed.1.minor() {
        return Compatibility {
            actual: Some(actual),
            status: CompatibilityStatus::ReadOnlyPreserveSchema,
            invalid_value: None,
        };
    }

    if actual_parsed.1.minor() == expected_parsed.1.minor() {
        return Compatibility {
            actual: Some(actual),
            status: CompatibilityStatus::ReadWritePreserveSchema,
            invalid_value: None,
        };
    }

    Compatibility {
        actual: Some(actual),
        status: CompatibilityStatus::ReadWrite,
        invalid_value: None,
    }
}

/// The settings that keep the address of their JSON schema.
pub trait SchemaSettings {
    /// Returns the address of the schema of the settings.
    fn schema(&self) -> &str;

    /// Sets the address of the schema of the settings.
    fn set_schema(&mut self, schema: &str);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The schema of the settings of the launcher.
    fn launcher_schema() -> Schema {
        Schema::new("launcher-settings", SchemaVersion::new(1, 0, 0))
    }

    /// Returns the JSON of a settings file with a schema.
    fn object(schema: &str) -> Value {
        serde_json::json!({ "$schema": schema, "language": "en" })
    }

    #[test]
    fn the_versions_of_the_schemas_are_read() {
        let version = SchemaVersion::parse("1.0.0").expect("the version should be read");
        assert_eq!(version.major(), 1);
        assert_eq!(version.minor(), 0);
        assert_eq!(version.patch(), 0);
        assert_eq!(version.to_string(), "1.0.0");

        let version = SchemaVersion::parse("2.3").expect("the version should be read");
        assert_eq!(version.patch(), 0);

        assert!(SchemaVersion::parse("1").is_none());
        assert!(SchemaVersion::parse("1.0.0.0").is_none());
        assert!(SchemaVersion::parse("1.x").is_none());
    }

    #[test]
    fn the_schemas_of_the_launcher_are_read() {
        let schema = launcher_schema();
        assert_eq!(schema.id(), Some("launcher-settings"));
        assert_eq!(schema.version(), Some(SchemaVersion::new(1, 0, 0)));
        assert!(schema.is_parsed());
        assert_eq!(
            schema.url(),
            "https://schemas.glavo.site/hmcl/launcher-settings/1.0.0"
        );

        let schema = Schema::parse("https://schemas.glavo.site/hmcl/user-settings/1.0.0");
        assert_eq!(schema.id(), Some("user-settings"));

        let schema = Schema::parse("https://example.com/schema.json");
        assert!(!schema.is_parsed());
        assert_eq!(schema.id(), None);
    }

    #[test]
    fn a_file_of_the_same_schema_may_be_written_again() {
        let compatibility = check(
            &object("https://schemas.glavo.site/hmcl/launcher-settings/1.0.0"),
            &launcher_schema(),
        );

        assert_eq!(
            compatibility.status,
            CompatibilityStatus::ReadWritePreserveSchema
        );
        assert!(compatibility.readable());
        assert!(compatibility.allow_save());
        assert!(compatibility.preserve_schema());
    }

    #[test]
    fn a_file_of_an_older_schema_is_written_again() {
        // The launcher knows the version 1.1 of the schema, so the file of
        // the version 1.0 is older than the one the launcher knows.
        let expected = Schema::new("launcher-settings", SchemaVersion::new(1, 1, 0));
        let compatibility = check(
            &object("https://schemas.glavo.site/hmcl/launcher-settings/1.0.0"),
            &expected,
        );

        assert_eq!(compatibility.status, CompatibilityStatus::ReadWrite);
        assert!(compatibility.readable());
        assert!(compatibility.allow_save());
        assert!(!compatibility.preserve_schema());
    }

    #[test]
    fn a_file_of_a_newer_schema_is_only_read() {
        let compatibility = check(
            &object("https://schemas.glavo.site/hmcl/launcher-settings/1.1.0"),
            &launcher_schema(),
        );

        assert_eq!(
            compatibility.status,
            CompatibilityStatus::ReadOnlyPreserveSchema
        );
        assert!(compatibility.readable());
        assert!(!compatibility.allow_save());
        assert!(compatibility.preserve_schema());
    }

    #[test]
    fn the_files_that_cannot_be_read_are_told_apart() {
        let compatibility = check(&serde_json::json!({}), &launcher_schema());
        assert_eq!(compatibility.status, CompatibilityStatus::Missing);
        assert!(!compatibility.readable());

        let compatibility = check(&serde_json::json!({ "$schema": 42 }), &launcher_schema());
        assert_eq!(compatibility.status, CompatibilityStatus::Invalid);
        assert!(!compatibility.readable());

        let compatibility = check(
            &object("https://example.com/schema.json"),
            &launcher_schema(),
        );
        assert_eq!(compatibility.status, CompatibilityStatus::Unparseable);
        assert!(!compatibility.readable());

        let compatibility = check(
            &object("https://schemas.glavo.site/hmcl/user-settings/1.0.0"),
            &launcher_schema(),
        );
        assert_eq!(compatibility.status, CompatibilityStatus::UnexpectedId);
        assert!(!compatibility.readable());

        let compatibility = check(
            &object("https://schemas.glavo.site/hmcl/launcher-settings/2.0.0"),
            &launcher_schema(),
        );
        assert_eq!(compatibility.status, CompatibilityStatus::UnsupportedMajor);
        assert!(!compatibility.readable());
    }
}
