//! The theme packs the user installs and exports.
//!
//! A theme pack is a zip file that holds a manifest and the assets the
//! launcher reads to change its appearance. The manifest is written as
//! `manifest.json` and declares the themes the pack offers.
//!
//! The types mirror `ThemePackManifest`, `ThemePackAuthor`, `ThemePackAsset`
//! and `ThemePackExporter` of Java, so a pack written here can be read by the
//! launcher of Java and the other way round.

use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::i18n::LocalizedText;

/// The JSON schema for the current manifest format.
pub const MANIFEST_SCHEMA_URL: &str = "https://schemas.glavo.site/hmcl/theme-pack/1.0.0";

/// The name of the manifest entry of every exported theme pack.
pub const MANIFEST_ENTRY: &str = "manifest.json";

/// The recommended file extension of a theme pack file.
pub const FILE_EXTENSION: &str = ".hmcl-theme";

/// The required prefix of every asset entry of an exported theme pack.
const ASSETS_PREFIX: &str = "assets/";

/// The package and theme ID format used by theme-pack manifests.
static PACKAGE_ID_PATTERN: OnceLock<regex::Regex> = OnceLock::new();

fn package_id_pattern() -> &'static regex::Regex {
    PACKAGE_ID_PATTERN.get_or_init(|| {
        regex::Regex::new(r"^[A-Za-z0-9][A-Za-z0-9_-]*(\.[A-Za-z0-9][A-Za-z0-9_-]*)*$")
            .expect("the package ID pattern should compile")
    })
}

/// The stable package identifier of a theme pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePackId(String);

impl ThemePackId {
    /// Returns a package identifier matching the package ID format.
    ///
    /// # Errors
    ///
    /// Returns an error when the text does not match the package ID format.
    pub fn new(text: &str) -> Result<Self, ThemePackError> {
        let id = require_non_blank(text, "id")?;
        if !package_id_pattern().is_match(id) {
            return Err(ThemePackError::InvalidFormat(format!(
                "Theme-pack manifest ID must follow package ID format: {id}"
            )));
        }
        Ok(Self(id.to_string()))
    }

    /// Returns the text of the package identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ThemePackId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for ThemePackId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// The error of reading or writing a theme pack.
#[derive(Debug)]
pub enum ThemePackError {
    /// The content of a file does not have the shape the launcher expects.
    InvalidFormat(String),
    /// A file of the file system could not be read or written.
    Io {
        /// The path of the file, if the operation named one.
        path: Option<PathBuf>,
        /// The failure the operating system reported.
        source: std::io::Error,
    },
}

impl fmt::Display for ThemePackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(reason) => formatter.write_str(reason),
            Self::Io { path: _, source } => {
                write!(formatter, "The file could not be read or written: {source}")
            }
        }
    }
}

impl std::error::Error for ThemePackError {}

impl From<ThemePackError> for Error {
    fn from(error: ThemePackError) -> Self {
        match error {
            ThemePackError::InvalidFormat(reason) => {
                Error::invalid_format(PathBuf::new(), format!("theme pack: {reason}"))
            }
            ThemePackError::Io { path, source } => match path {
                Some(path) => Error::io(path, source),
                None => Error::io_without_path(source),
            },
        }
    }
}

fn require_non_blank<'a>(value: &'a str, field: &str) -> Result<&'a str, ThemePackError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(ThemePackError::InvalidFormat(format!(
            "Theme-pack manifest field is blank: {field}"
        )))
    } else {
        Ok(trimmed)
    }
}

/// An author of a theme pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePackAuthor {
    /// The localized display name of the author.
    pub name: LocalizedText,
}

impl ThemePackAuthor {
    /// Creates author metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when the name is empty, because an author without a
    /// name cannot be shown.
    pub fn new(name: LocalizedText) -> Result<Self, ThemePackError> {
        if name.may_be_empty() {
            return Err(ThemePackError::InvalidFormat(
                "The author name cannot be empty".to_string(),
            ));
        }
        Ok(Self { name })
    }

    /// Parses author metadata from a JSON value.
    pub fn from_json(value: &serde_json::Value) -> Result<Option<Self>, ThemePackError> {
        match value {
            serde_json::Value::Null => Ok(None),
            serde_json::Value::String(text) => Ok(Some(Self::new(LocalizedText::plain(text))?)),
            serde_json::Value::Object(object) => {
                let name_value = object.get("name").ok_or_else(|| {
                    ThemePackError::InvalidFormat(format!("Missing author name: {object:?}"))
                })?;
                let name =
                    serde_json::from_value::<LocalizedText>(name_value.clone()).map_err(|_| {
                        ThemePackError::InvalidFormat(
                            "Theme-pack author name must be a string or object".to_string(),
                        )
                    })?;
                Ok(Some(Self::new(name)?))
            }
            other => Err(ThemePackError::InvalidFormat(format!(
                "Theme-pack author must be an object or a string: {other}"
            ))),
        }
    }

    /// Parses author metadata from a JSON array.
    pub fn parse_authors(value: Option<&serde_json::Value>) -> Result<Vec<Self>, ThemePackError> {
        let Some(value) = value else {
            return Ok(Vec::new());
        };
        let array = value.as_array().ok_or_else(|| {
            ThemePackError::InvalidFormat("Theme-pack authors must be an array".to_string())
        })?;
        let mut authors = Vec::with_capacity(array.len());
        for author_json in array {
            if let Ok(Some(author)) = Self::from_json(author_json) {
                authors.push(author);
            }
        }
        Ok(authors)
    }

    /// Returns the author display name in the current locale.
    pub fn display_name(&self, languages: &[&str]) -> String {
        self.name.text(languages).to_string()
    }

    /// Converts this author to its JSON representation.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({ "name": self.name })
    }
}

/// A resource copied into an exported theme-pack zip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemePackAssetSource {
    /// A local file to copy.
    File(PathBuf),
    /// The content of a file to write directly.
    Content(Vec<u8>),
}

/// One resource copied into an exported theme-pack zip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePackAsset {
    /// The resource to copy.
    pub source: ThemePackAssetSource,
    /// The normalized zip entry name under `assets/`.
    pub entry_name: String,
}

impl ThemePackAsset {
    /// Creates a theme-pack asset from a local file.
    pub fn from_file(source: PathBuf, entry_name: &str) -> Result<Self, ThemePackError> {
        Ok(Self {
            source: ThemePackAssetSource::File(source),
            entry_name: Self::normalize_entry_name(entry_name)?,
        })
    }

    /// Creates a theme-pack asset entry from content.
    pub fn from_content(content: Vec<u8>, entry_name: &str) -> Result<Self, ThemePackError> {
        Ok(Self {
            source: ThemePackAssetSource::Content(content),
            entry_name: Self::normalize_entry_name(entry_name)?,
        })
    }

    /// Normalizes and validates a zip entry name.
    ///
    /// # Errors
    ///
    /// Returns an error when the entry name is unsafe or outside `assets/`.
    pub fn normalize_entry_name(entry_name: &str) -> Result<String, ThemePackError> {
        let normalized = entry_name.trim().replace('\\', "/");
        if normalized.is_empty() {
            return Err(ThemePackError::InvalidFormat(
                "Theme-pack asset entry is empty".to_string(),
            ));
        }
        if normalized.starts_with('/') || normalized.chars().nth(1).is_some_and(|ch| ch == ':') {
            return Err(ThemePackError::InvalidFormat(format!(
                "Theme-pack asset entry must be relative: {entry_name}"
            )));
        }
        if !normalized.starts_with(ASSETS_PREFIX) {
            return Err(ThemePackError::InvalidFormat(format!(
                "Theme-pack asset entry must be under assets/: {entry_name}"
            )));
        }
        if normalized.ends_with('/') {
            return Err(ThemePackError::InvalidFormat(format!(
                "Theme-pack asset entry must be a file: {entry_name}"
            )));
        }

        for segment in normalized.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(ThemePackError::InvalidFormat(format!(
                    "Theme-pack asset entry contains an unsafe segment: {entry_name}"
                )));
            }
            if segment.chars().any(|ch| ch.is_control() || ch == '\0') {
                return Err(ThemePackError::InvalidFormat(format!(
                    "Theme-pack asset entry contains a control character: {entry_name}"
                )));
            }
        }
        Ok(normalized)
    }
}

/// One selectable theme inside a theme pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    /// The stable theme identifier inside its pack, or `None` for an unnamed
    /// single-theme pack.
    pub id: Option<String>,
    /// The localized display name, or `None` for an unnamed single-theme pack.
    pub name: Option<LocalizedText>,
    /// The authors of this specific theme.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<ThemePackAuthor>,
    /// The optional localized description.
    pub description: Option<LocalizedText>,
    /// The optional theme-pack relative icon path.
    pub icon: Option<String>,
    /// The default appearance fields.
    pub appearance: serde_json::Value,
    /// Conditional appearance patches applied in declaration order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<serde_json::Value>,
}

impl Theme {
    /// Creates a theme.
    ///
    /// # Errors
    ///
    /// Returns an error when the theme ID or name is invalid, or when the
    /// icon path is unsafe.
    pub fn new(
        id: Option<String>,
        name: Option<LocalizedText>,
        authors: Vec<ThemePackAuthor>,
        description: Option<LocalizedText>,
        icon: Option<String>,
        appearance: serde_json::Value,
        overrides: Vec<serde_json::Value>,
    ) -> Result<Self, ThemePackError> {
        let id = match id {
            Some(id) => {
                let id = require_theme_id(&id)?;
                Some(id)
            }
            None => None,
        };
        if let Some(name) = &name {
            if name.may_be_empty() {
                return Err(ThemePackError::InvalidFormat(
                    "Theme name is empty".to_string(),
                ));
            }
        }
        let icon = match icon {
            Some(icon) => Some(ThemePackAsset::normalize_entry_name(&icon)?),
            None => None,
        };
        Ok(Self {
            id,
            name,
            authors,
            description,
            icon,
            appearance,
            overrides,
        })
    }

    /// Returns the theme identifier, or `None` when the theme is unnamed.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Returns the theme name, or `None` when the theme is unnamed.
    pub fn name(&self) -> Option<&LocalizedText> {
        self.name.as_ref()
    }
}

fn require_theme_id(value: &str) -> Result<String, ThemePackError> {
    let id = require_non_blank(value, "theme id")?;
    if !package_id_pattern().is_match(id) {
        return Err(ThemePackError::InvalidFormat(format!(
            "Theme ID must follow package ID format: {id}"
        )));
    }
    Ok(id.to_string())
}

/// The manifest of a theme pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePackManifest {
    /// The stable package identifier.
    pub id: ThemePackId,
    /// The package version string.
    pub version: String,
    /// The localized display name.
    pub name: LocalizedText,
    /// The package authors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<ThemePackAuthor>,
    /// The optional theme-pack relative icon path.
    pub icon: Option<String>,
    /// The optional localized package description.
    pub description: Option<LocalizedText>,
    /// The selectable themes declared by the package.
    pub themes: Vec<Theme>,
}

impl ThemePackManifest {
    /// Creates a theme-pack manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the identifier, version or name is invalid, when
    /// the icon path is unsafe, or when the pack declares no themes.
    pub fn new(
        id: &str,
        version: &str,
        name: LocalizedText,
        authors: Vec<ThemePackAuthor>,
        icon: Option<String>,
        description: Option<LocalizedText>,
        themes: Vec<Theme>,
    ) -> Result<Self, ThemePackError> {
        let id = ThemePackId::new(id)?;
        let version = require_non_blank(version, "version")?;
        if themes.is_empty() {
            return Err(ThemePackError::InvalidFormat(
                "Theme pack must declare at least one theme".to_string(),
            ));
        }
        check_theme_identities(&themes)?;
        Ok(Self {
            id,
            version: version.to_string(),
            name,
            authors,
            icon: icon
                .map(|icon| ThemePackAsset::normalize_entry_name(&icon))
                .transpose()?,
            description,
            themes,
        })
    }

    /// Returns whether the pack is a simple theme pack: a single theme with no
    /// identifier and no name.
    pub fn is_simple_theme_pack(&self) -> bool {
        self.themes.len() == 1 && self.themes[0].id.is_none() && self.themes[0].name.is_none()
    }

    /// Returns the package display name in the current locale.
    pub fn display_name(&self, languages: &[&str]) -> String {
        let name = self.name.text(languages);
        if name.is_empty() {
            self.id.as_str().to_string()
        } else {
            name.to_string()
        }
    }

    /// Returns the package description in the current locale.
    pub fn display_description(&self, languages: &[&str]) -> Option<String> {
        self.description
            .as_ref()
            .map(|description| description.text(languages).to_string())
    }

    /// Returns the first theme with the given ID.
    ///
    /// A `None` theme ID matches only a single-theme manifest.
    pub fn find_theme(&self, theme_id: Option<&str>) -> Option<&Theme> {
        match theme_id {
            None => {
                if self.themes.len() == 1 {
                    return Some(&self.themes[0]);
                }
                None
            }
            Some(theme_id) => self
                .themes
                .iter()
                .find(|theme| theme.id.as_deref() == Some(theme_id)),
        }
    }
}

fn check_theme_identities(themes: &[Theme]) -> Result<(), ThemePackError> {
    if themes.len() <= 1 {
        return Ok(());
    }
    for theme in themes {
        if theme.id.is_none() {
            return Err(ThemePackError::InvalidFormat(
                "Theme ID is required when a theme pack declares multiple themes".to_string(),
            ));
        }
        if theme.name.is_none() {
            return Err(ThemePackError::InvalidFormat(
                "Theme name is required when a theme pack declares multiple themes".to_string(),
            ));
        }
    }
    Ok(())
}

/// Converts a zip error into a theme pack error.
fn zip_error(path: Option<PathBuf>, source: crate::util::zip::ZipError) -> ThemePackError {
    match source {
        crate::util::zip::ZipError::Io(error) => ThemePackError::Io {
            path,
            source: error,
        },
        crate::util::zip::ZipError::Malformed(reason) => {
            ThemePackError::InvalidFormat(reason.to_string())
        }
    }
}

/// Writes theme-pack manifests and assets into zip-compatible files.
pub struct ThemePackExporter;

impl ThemePackExporter {
    /// Exports a theme pack to a zip-compatible file.
    ///
    /// # Errors
    ///
    /// Returns an error when a source file cannot be read, the zip cannot be
    /// written, or an asset entry is duplicated or unsafe.
    pub fn export(
        manifest: &ThemePackManifest,
        assets: &[ThemePackAsset],
        output_file: &std::path::Path,
    ) -> Result<(), ThemePackError> {
        Self::validate_assets(assets)?;

        let output = output_file.to_path_buf();
        if let Some(parent) = output.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| ThemePackError::Io {
                    path: Some(output.clone()),
                    source,
                })?;
            }
        }

        let temporary = {
            let mut name = output.file_name().unwrap_or_default().to_os_string();
            name.push(".part");
            output.with_file_name(name)
        };
        let _ = std::fs::remove_file(&temporary);

        {
            let mut writer = crate::util::zip::ZipWriter::create(&temporary)
                .map_err(|source| zip_error(Some(temporary.clone()), source))?;

            let mut manifest_value =
                serde_json::to_value(manifest).map_err(|source| ThemePackError::Io {
                    path: Some(temporary.clone()),
                    source: std::io::Error::other(source),
                })?;
            if let Some(object) = manifest_value.as_object_mut() {
                object.insert(
                    "$schema".to_string(),
                    serde_json::json!(MANIFEST_SCHEMA_URL),
                );
            }
            let manifest_json =
                serde_json::to_string(&manifest_value).map_err(|source| ThemePackError::Io {
                    path: Some(temporary.clone()),
                    source: std::io::Error::other(source),
                })?;
            writer
                .add_file(MANIFEST_ENTRY, manifest_json.as_bytes())
                .map_err(|source| zip_error(Some(temporary.clone()), source))?;

            for asset in assets {
                match &asset.source {
                    ThemePackAssetSource::File(path) => {
                        writer
                            .add_path(&asset.entry_name, path)
                            .map_err(|source| zip_error(Some(path.clone()), source))?;
                    }
                    ThemePackAssetSource::Content(content) => {
                        writer
                            .add_file(&asset.entry_name, content)
                            .map_err(|source| zip_error(None, source))?;
                    }
                }
            }

            writer
                .finish()
                .map_err(|source| zip_error(Some(temporary.clone()), source))?;
        }

        std::fs::rename(&temporary, &output).map_err(|source| ThemePackError::Io {
            path: Some(output.clone()),
            source,
        })?;

        Ok(())
    }

    /// Validates all asset entries and source files before the zip is written.
    fn validate_assets(assets: &[ThemePackAsset]) -> Result<(), ThemePackError> {
        use std::collections::HashSet;

        let mut entries = HashSet::new();
        entries.insert(MANIFEST_ENTRY.to_string());

        for asset in assets {
            match &asset.source {
                ThemePackAssetSource::File(path) => {
                    if !path.is_file() {
                        return Err(ThemePackError::InvalidFormat(format!(
                            "Theme-pack asset source is not a regular file: {}",
                            path.display()
                        )));
                    }
                    let _ = std::fs::File::open(path);
                }
                ThemePackAssetSource::Content(_) => {}
            }
            if !entries.insert(asset.entry_name.clone()) {
                return Err(ThemePackError::InvalidFormat(format!(
                    "Duplicate theme-pack zip entry: {}",
                    asset.entry_name
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ThemePackManifest {
        ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain("Example Theme"),
            Vec::new(),
            None,
            None,
            vec![Theme::new(
                None,
                Some(LocalizedText::plain("Default")),
                Vec::new(),
                None,
                None,
                serde_json::json!({}),
                Vec::new(),
            )
            .expect("the theme should be created")],
        )
        .expect("the manifest should be created")
    }

    #[test]
    fn the_package_id_is_validated() {
        assert!(ThemePackId::new("com.example.theme").is_ok());
        assert!(ThemePackId::new("Theme").is_ok());
        assert!(ThemePackId::new("a.b.c").is_ok());
        assert!(ThemePackId::new("").is_err());
        assert!(ThemePackId::new("   ").is_err());
        assert!(ThemePackId::new("with space").is_err());
        assert!(ThemePackId::new("-leading").is_err());
        assert!(ThemePackId::new("under_score").is_ok());
    }

    #[test]
    fn the_package_id_displays_as_its_text() {
        let id = ThemePackId::new("com.example.theme").expect("the id should be created");
        assert_eq!(id.to_string(), "com.example.theme");
        assert_eq!(id.as_str(), "com.example.theme");
    }

    #[test]
    fn an_asset_entry_under_assets_is_normalized() {
        assert_eq!(
            ThemePackAsset::normalize_entry_name("assets/icon.png").unwrap(),
            "assets/icon.png"
        );
        assert_eq!(
            ThemePackAsset::normalize_entry_name("assets\\sub\\icon.png").unwrap(),
            "assets/sub/icon.png"
        );
        assert_eq!(
            ThemePackAsset::normalize_entry_name("  assets/icon.png  ").unwrap(),
            "assets/icon.png"
        );
    }

    #[test]
    fn an_unsafe_asset_entry_is_rejected() {
        assert!(ThemePackAsset::normalize_entry_name("").is_err());
        assert!(ThemePackAsset::normalize_entry_name("/absolute.png").is_err());
        assert!(ThemePackAsset::normalize_entry_name("C:\\icon.png").is_err());
        assert!(ThemePackAsset::normalize_entry_name("icon.png").is_err());
        assert!(ThemePackAsset::normalize_entry_name("assets/").is_err());
        assert!(ThemePackAsset::normalize_entry_name("assets/./icon.png").is_err());
        assert!(ThemePackAsset::normalize_entry_name("assets/../icon.png").is_err());
        assert!(ThemePackAsset::normalize_entry_name("assets//icon.png").is_err());
    }

    #[test]
    fn an_asset_entry_with_a_control_character_is_rejected() {
        assert!(ThemePackAsset::normalize_entry_name("assets/\u{7f}icon.png").is_err());
    }

    #[test]
    fn an_author_is_parsed_from_a_string_and_an_object() {
        let string = serde_json::json!("Gluon");
        let object = serde_json::json!({ "name": "Gluon" });

        assert_eq!(
            ThemePackAuthor::from_json(&string).unwrap().unwrap().name,
            LocalizedText::plain("Gluon")
        );
        assert_eq!(
            ThemePackAuthor::from_json(&object).unwrap().unwrap().name,
            LocalizedText::plain("Gluon")
        );
        assert_eq!(
            ThemePackAuthor::from_json(&serde_json::json!(null)).unwrap(),
            None
        );
    }

    #[test]
    fn authors_are_parsed_from_an_array() {
        let array = serde_json::json!([
            "Gluon",
            { "name": { "en": "Gluon", "zh-Hans": "胶质" } },
            42
        ]);

        let authors = ThemePackAuthor::parse_authors(Some(&array)).unwrap();
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].display_name(&["en"]), "Gluon");
        assert_eq!(authors[1].display_name(&["zh-Hans"]), "胶质");
    }

    #[test]
    fn a_manifest_is_created_and_serialized_with_the_schema() {
        let manifest = manifest();
        let value = serde_json::to_value(&manifest).unwrap();

        assert_eq!(value["id"], "com.example.theme");
        assert_eq!(value["version"], "1.0.0");
        assert_eq!(value["name"], "Example Theme");
        assert_eq!(value["themes"].as_array().unwrap().len(), 1);
        // The schema address is added when the manifest is exported.
    }

    #[test]
    fn a_single_unnamed_theme_is_a_simple_theme_pack() {
        let simple = ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain("Example Theme"),
            Vec::new(),
            None,
            None,
            vec![Theme::new(
                None,
                None,
                Vec::new(),
                None,
                None,
                serde_json::json!({}),
                Vec::new(),
            )
            .expect("the theme should be created")],
        )
        .expect("the manifest should be created");
        assert!(simple.is_simple_theme_pack());

        let multi = ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain("Example Theme"),
            Vec::new(),
            None,
            None,
            vec![
                Theme::new(
                    Some("a".to_string()),
                    Some(LocalizedText::plain("A")),
                    Vec::new(),
                    None,
                    None,
                    serde_json::json!({}),
                    Vec::new(),
                )
                .expect("the theme should be created"),
                Theme::new(
                    Some("b".to_string()),
                    Some(LocalizedText::plain("B")),
                    Vec::new(),
                    None,
                    None,
                    serde_json::json!({}),
                    Vec::new(),
                )
                .expect("the theme should be created"),
            ],
        )
        .expect("the manifest should be created");
        assert!(!multi.is_simple_theme_pack());
    }

    #[test]
    fn a_theme_is_found_by_its_identifier() {
        let manifest = manifest();
        assert!(manifest.find_theme(None).is_some());
        assert!(manifest.find_theme(Some("does-not-exist")).is_none());

        let multi = ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain("Example Theme"),
            Vec::new(),
            None,
            None,
            vec![
                Theme::new(
                    Some("a".to_string()),
                    Some(LocalizedText::plain("A")),
                    Vec::new(),
                    None,
                    None,
                    serde_json::json!({}),
                    Vec::new(),
                )
                .expect("the theme should be created"),
                Theme::new(
                    Some("b".to_string()),
                    Some(LocalizedText::plain("B")),
                    Vec::new(),
                    None,
                    None,
                    serde_json::json!({}),
                    Vec::new(),
                )
                .expect("the theme should be created"),
            ],
        )
        .expect("the manifest should be created");
        assert_eq!(
            multi.find_theme(Some("a")).unwrap().name().unwrap(),
            &LocalizedText::plain("A")
        );
        // A None id does not match a multi-theme manifest.
        assert!(multi.find_theme(None).is_none());
    }

    #[test]
    fn the_display_name_falls_back_to_the_package_id() {
        let manifest = manifest();
        assert_eq!(manifest.display_name(&["en"]), "Example Theme");

        let plain_id = ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain(""),
            Vec::new(),
            None,
            None,
            vec![Theme::new(
                None,
                None,
                Vec::new(),
                None,
                None,
                serde_json::json!({}),
                Vec::new(),
            )
            .expect("the theme should be created")],
        )
        .expect("the manifest should be created");
        assert_eq!(plain_id.display_name(&["en"]), "com.example.theme");
    }

    #[test]
    fn the_display_description_is_optional() {
        let manifest = manifest();
        assert_eq!(manifest.display_description(&["en"]), None);

        let described = ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain("Example Theme"),
            Vec::new(),
            None,
            Some(LocalizedText::plain("A description")),
            vec![Theme::new(
                None,
                Some(LocalizedText::plain("Default")),
                Vec::new(),
                None,
                None,
                serde_json::json!({}),
                Vec::new(),
            )
            .expect("the theme should be created")],
        )
        .expect("the manifest should be created");
        assert_eq!(
            described.display_description(&["en"]),
            Some("A description".to_string())
        );
    }

    #[test]
    fn a_theme_without_an_id_is_rejected_when_there_are_multiple_themes() {
        let result = ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain("Example Theme"),
            Vec::new(),
            None,
            None,
            vec![
                Theme::new(
                    None,
                    Some(LocalizedText::plain("A")),
                    Vec::new(),
                    None,
                    None,
                    serde_json::json!({}),
                    Vec::new(),
                )
                .expect("the theme should be created"),
                Theme::new(
                    Some("b".to_string()),
                    Some(LocalizedText::plain("B")),
                    Vec::new(),
                    None,
                    None,
                    serde_json::json!({}),
                    Vec::new(),
                )
                .expect("the theme should be created"),
            ],
        );
        assert!(result.is_err());
    }

    #[test]
    fn an_icon_path_is_normalized() {
        let manifest = ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain("Example Theme"),
            Vec::new(),
            Some("assets/icon.png".to_string()),
            None,
            vec![Theme::new(
                None,
                Some(LocalizedText::plain("Default")),
                Vec::new(),
                None,
                None,
                serde_json::json!({}),
                Vec::new(),
            )
            .expect("the theme should be created")],
        )
        .expect("the manifest should be created");
        assert_eq!(manifest.icon.as_deref(), Some("assets/icon.png"));

        let bad = ThemePackManifest::new(
            "com.example.theme",
            "1.0.0",
            LocalizedText::plain("Example Theme"),
            Vec::new(),
            Some("/absolute.png".to_string()),
            None,
            vec![Theme::new(
                None,
                Some(LocalizedText::plain("Default")),
                Vec::new(),
                None,
                None,
                serde_json::json!({}),
                Vec::new(),
            )
            .expect("the theme should be created")],
        );
        assert!(bad.is_err());
    }

    #[test]
    fn a_theme_pack_is_exported_and_can_be_read_back() {
        let directory = std::env::temp_dir().join("hmcl-theme-pack-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        let asset_path = directory.join("source.png");
        std::fs::write(&asset_path, b"PNG").expect("the source file should be written");

        let output = directory.join(format!("pack{}", FILE_EXTENSION));
        let manifest = manifest();
        let assets = vec![ThemePackAsset::from_file(asset_path, "assets/icon.png")
            .expect("the asset should be created")];

        ThemePackExporter::export(&manifest, &assets, &output)
            .expect("the theme pack should be exported");

        assert!(output.is_file());

        let entries = crate::util::zip::entries(&output).expect("the archive should be read");
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert!(names.contains(&"manifest.json"));
        assert!(names.contains(&"assets/icon.png"));

        let manifest_bytes = crate::util::zip::read_entry(&output, "manifest.json")
            .expect("the manifest should be read")
            .expect("the manifest should exist");
        let read: ThemePackManifest =
            serde_json::from_slice(&manifest_bytes).expect("the manifest should be parsed");
        assert_eq!(read.id, manifest.id);
        assert_eq!(read.version, manifest.version);
        assert_eq!(read.name, manifest.name);
        assert!(read.find_theme(None).is_some());

        let icon_bytes = crate::util::zip::read_entry(&output, "assets/icon.png")
            .expect("the icon should be read")
            .expect("the icon should exist");
        assert_eq!(icon_bytes, b"PNG");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_duplicate_asset_entry_is_rejected() {
        let manifest = manifest();
        let assets = vec![
            ThemePackAsset::from_content(b"a".to_vec(), "assets/a.png")
                .expect("the asset should be created"),
            ThemePackAsset::from_content(b"b".to_vec(), "assets/a.png")
                .expect("the asset should be created"),
        ];

        let directory = std::env::temp_dir().join("hmcl-theme-pack-dup-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");
        let output = directory.join(format!("pack{}", FILE_EXTENSION));

        let error = ThemePackExporter::export(&manifest, &assets, &output)
            .expect_err("the duplicate entry should be rejected");
        assert!(matches!(error, ThemePackError::InvalidFormat(_)));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn an_output_file_in_a_new_directory_is_created() {
        let directory = std::env::temp_dir().join("hmcl-theme-pack-newdir-test");
        let _ = std::fs::remove_dir_all(&directory);
        let output = directory
            .join("nested")
            .join(format!("pack{}", FILE_EXTENSION));

        let manifest = manifest();
        let assets: Vec<ThemePackAsset> = Vec::new();

        ThemePackExporter::export(&manifest, &assets, &output)
            .expect("the theme pack should be exported");
        assert!(output.is_file());

        let _ = std::fs::remove_dir_all(&directory);
    }
}
