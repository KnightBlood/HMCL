//! The metadata of the mods a player has installed.
//!
//! A mod is a file the game loads, and the file names its own metadata: Forge
//! and NeoForge write TOML, Fabric and Quilt write JSON, and LiteLoader writes
//! JSON with other names. The launcher reads the metadata to show the mod with
//! its name, its version and the loader that is needed for it, and keeps the
//! mod with the name of its file when the metadata cannot be read.

use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use crate::addon::mod_loader::ModLoaderType;
use crate::util::toml;
use crate::util::zip;

/// The suffix of a mod file that is disabled.
pub const DISABLED_EXTENSION: &str = ".disabled";

/// The suffix of a mod file that is kept as a backup of an older version.
pub const OLD_EXTENSION: &str = ".old";

/// The extensions of the files that may be a mod.
pub const MOD_EXTENSIONS: [&str; 3] = ["jar", "zip", "litemod"];

/// A mod the player has installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFile {
    /// The file of the mod.
    path: PathBuf,
    /// The file name of the mod without its suffixes or its extension.
    file_name: String,
    /// The name the mod gives itself.
    name: String,
    /// The identifier the mod gives itself.
    id: String,
    /// The version of the mod.
    version: String,
    /// The version of the game the mod names, if it names one.
    game_version: String,
    /// The authors of the mod.
    authors: String,
    /// The description of the mod.
    description: String,
    /// The address of the page of the mod.
    url: String,
    /// The path of the logo of the mod, inside the file of the mod.
    logo: String,
    /// The loader the mod is written for.
    loader: ModLoaderType,
    /// Whether the mod is disabled.
    disabled: bool,
    /// Whether the mod is kept as a backup of an older version.
    old: bool,
    /// Whether the metadata of the mod was read.
    metadata_read: bool,
}

impl ModFile {
    /// Reads the metadata of a mod file.
    ///
    /// The file is not required to exist: when it is missing or its metadata
    /// cannot be read, the mod is kept with the name of the file. The readers
    /// of the loaders that are given are tried first, so a file that could be a
    /// mod of more than one loader is read as a mod of a loader of the game.
    pub fn open(path: impl AsRef<Path>, loaders: &[ModLoaderType]) -> Self {
        let path = path.as_ref();
        let raw_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        let disabled = raw_name.ends_with(DISABLED_EXTENSION);
        let old = raw_name.ends_with(OLD_EXTENSION);
        let local_name = display_name(&raw_name);
        let file_name = local_name
            .rsplit_once('.')
            .map(|(stem, _)| stem.to_string())
            .unwrap_or_else(|| local_name.clone());

        let mut file = Self {
            path: path.to_path_buf(),
            file_name,
            name: String::new(),
            id: String::new(),
            version: String::new(),
            game_version: String::new(),
            authors: String::new(),
            description: String::new(),
            url: String::new(),
            logo: String::new(),
            loader: ModLoaderType::Unknown,
            disabled,
            old,
            metadata_read: false,
        };

        if let Some(metadata) = read_metadata(path, &local_name, loaders) {
            file.id = metadata.id;
            file.name = metadata.name;
            file.version = metadata.version;
            file.game_version = metadata.game_version;
            file.authors = metadata.authors;
            file.description = metadata.description;
            file.url = metadata.url;
            file.logo = metadata.logo;
            file.loader = metadata.loader;
            file.metadata_read = true;
        }

        // A mod whose metadata names nothing is shown with its file name.
        if file.name.is_empty() {
            file.name.clone_from(&file.file_name);
        }
        if file.id.is_empty() {
            file.id.clone_from(&file.file_name);
        }

        file
    }

    /// Returns the file of the mod.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file name of the mod without its suffixes or its extension.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the name the mod gives itself.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the identifier the mod gives itself.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the version of the mod.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the version of the game the mod names.
    pub fn game_version(&self) -> &str {
        &self.game_version
    }

    /// Returns the authors of the mod.
    pub fn authors(&self) -> &str {
        &self.authors
    }

    /// Returns the description of the mod.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the address of the page of the mod.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the path of the logo of the mod, inside its file.
    pub fn logo(&self) -> &str {
        &self.logo
    }

    /// Returns the loader the mod is written for.
    pub fn loader(&self) -> ModLoaderType {
        self.loader
    }

    /// Returns whether the mod is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Returns whether the mod is kept as a backup of an older version.
    pub fn is_old(&self) -> bool {
        self.old
    }

    /// Returns whether the metadata of the mod was read.
    ///
    /// A mod whose metadata cannot be read is still shown, because the player
    /// installed it and the file is in the directory of the mods.
    pub fn has_metadata(&self) -> bool {
        self.metadata_read
    }

    /// Returns whether the mod is loaded by the game.
    pub fn is_active(&self) -> bool {
        !self.disabled && !self.old
    }

    /// Returns whether the file name of a path is the name of a mod file.
    ///
    /// The suffixes of a disabled mod and of a backup are left out, so a file
    /// that is named like a mod is a mod whatever state it is in.
    pub fn is_file_name_mod(path: &Path) -> bool {
        let name = display_name(
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(""),
        );
        let extension = name.rsplit_once('.').map(|(_, extension)| extension);
        extension.is_some_and(|extension| {
            MOD_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
    }

    /// Returns whether the file of a mod holds the metadata of a mod.
    ///
    /// The file is read as an archive; a file that is not an archive holds no
    /// metadata. The name of the file does not matter here, so a file that is
    /// named like a mod but is something else is not a mod.
    pub fn is_file_mod(path: &Path) -> bool {
        let Ok(entries) = zip::entries(path) else {
            return false;
        };
        entries.iter().any(|entry| is_metadata_path(&entry.name))
    }

    /// Makes the mod follow the new path of its file.
    ///
    /// The state of the mod is read from the new name of the file, so the
    /// manager reports the mod as it is on the disk after a rename.
    pub fn set_state(&mut self, path: &Path, old: bool, disabled: bool) {
        self.path = path.to_path_buf();
        self.old = old;
        self.disabled = disabled;
    }
}

/// Returns the file name of a mod without the suffixes of a disabled mod and of
/// a backup.
pub fn display_name(file_name: &str) -> String {
    let name = file_name.strip_suffix(OLD_EXTENSION).unwrap_or(file_name);
    let name = name.strip_suffix(DISABLED_EXTENSION).unwrap_or(name);
    name.to_string()
}

/// Returns whether the path of a file of an archive is the path of a metadata
/// file of a mod.
fn is_metadata_path(name: &str) -> bool {
    matches!(
        name,
        "mcmod.info"
            | "META-INF/mods.toml"
            | "META-INF/neoforge.mods.toml"
            | "fabric.mod.json"
            | "quilt.mod.json"
            | "litemod.json"
    )
}

/// The metadata of a mod.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Metadata {
    /// The identifier the mod gives itself.
    id: String,
    /// The name the mod gives itself.
    name: String,
    /// The version of the mod.
    version: String,
    /// The version of the game the mod names.
    game_version: String,
    /// The authors of the mod.
    authors: String,
    /// The description of the mod.
    description: String,
    /// The address of the page of the mod.
    url: String,
    /// The path of the logo of the mod, inside its file.
    logo: String,
    /// The loader the mod is written for.
    loader: ModLoaderType,
}

/// A reader of the metadata of a mod, which reads the name of a file of the
/// archive of the mod.
type EntryReader<'a> = dyn Fn(&str) -> Option<Vec<u8>> + 'a;

/// The readers of the metadata of a mod of one extension.
type Readers = &'static [(ModLoaderType, fn(&EntryReader<'_>) -> Option<Metadata>)];

/// The readers of the mods of an extension.
///
/// A `.litemod` file is only a mod of LiteLoader; the other files are archives
/// that a loader of the game writes its metadata into.
fn readers_for(extension: &str) -> Option<Readers> {
    match extension {
        "litemod" => Some(&[(ModLoaderType::LiteLoader, read_litemod)]),
        "jar" | "zip" => Some(&[
            (ModLoaderType::Forge, read_forge_new),
            (ModLoaderType::NeoForge, read_neoforge),
            (ModLoaderType::Forge, read_forge_old),
            (ModLoaderType::Fabric, read_fabric),
            (ModLoaderType::Quilt, read_quilt),
        ]),
        _ => None,
    }
}

/// Reads the metadata of a mod file.
///
/// The extension of the file decides which readers are tried: a `.litemod` file
/// is only read as a mod of LiteLoader, and the other files are read as the
/// mods of the loaders that write JSON or TOML into the archive.
fn read_metadata(path: &Path, local_name: &str, loaders: &[ModLoaderType]) -> Option<Metadata> {
    let extension = local_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())?;

    let readers = readers_for(&extension)?;

    let read: Box<EntryReader<'_>> =
        Box::new(|name: &str| zip::read_entry(path, name).ok().flatten());

    // The readers of the loaders of the game are tried first, like the Java
    // launcher does, so a file that could be a mod of more than one loader is
    // read as a mod of the loader the game runs.
    for supported in [true, false] {
        for (loader, reader) in readers {
            let is_supported = loaders.contains(loader);
            if is_supported == supported {
                if let Some(metadata) = reader(&*read) {
                    return Some(metadata);
                }
            }
        }
    }

    None
}

/// Reads the metadata of a Fabric mod.
fn read_fabric(read: &EntryReader<'_>) -> Option<Metadata> {
    let json = read_json(read, "fabric.mod.json")?;

    let authors = json
        .get("authors")
        .and_then(Json::as_array)
        .map(|authors| {
            authors
                .iter()
                .filter_map(|author| match author {
                    Json::String(name) => Some(name.clone()),
                    Json::Object(_) => author
                        .get("name")
                        .and_then(Json::as_str)
                        .map(str::to_string),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    Some(Metadata {
        id: json_text(&json, "id"),
        name: json_text(&json, "name"),
        version: json_text(&json, "version"),
        authors,
        description: json_text_or_object(&json, "description"),
        url: json
            .get("contact")
            .and_then(|contact| contact.get("homepage"))
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_string(),
        logo: json_text(&json, "icon"),
        loader: ModLoaderType::Fabric,
        ..Metadata::default()
    })
}

/// Reads the metadata of a Quilt mod.
fn read_quilt(read: &EntryReader<'_>) -> Option<Metadata> {
    let json = read_json(read, "quilt.mod.json")?;

    // The launcher only reads the first version of the metadata of Quilt.
    if json.get("schema_version").and_then(Json::as_u64) != Some(1) {
        return None;
    }

    let loader = json.get("quilt_loader")?;
    let metadata = loader.get("metadata");

    let authors = metadata
        .and_then(|metadata| metadata.get("contributors"))
        .and_then(Json::as_object)
        .map(|contributors| {
            contributors
                .iter()
                .map(|(name, role)| match role {
                    Json::String(role) => format!("{name} ({role})"),
                    _ => name.clone(),
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let description = metadata
        .map(|metadata| json_text_or_object(metadata, "description"))
        .unwrap_or_default();
    let icon = metadata
        .map(|metadata| json_text(metadata, "icon"))
        .unwrap_or_default();
    let url = metadata
        .and_then(|metadata| metadata.get("contact"))
        .and_then(|contact| contact.get("homepage"))
        .and_then(Json::as_str)
        .unwrap_or_default()
        .to_string();
    let name = metadata
        .map(|metadata| json_text(metadata, "name"))
        .unwrap_or_default();

    Some(Metadata {
        id: json_text(loader, "id"),
        name,
        version: json_text(loader, "version"),
        authors,
        description,
        url,
        logo: icon,
        loader: ModLoaderType::Quilt,
        ..Metadata::default()
    })
}

/// Reads the metadata of a Forge mod written before 1.13.
fn read_forge_old(read: &EntryReader<'_>) -> Option<Metadata> {
    let json = read_json(read, "mcmod.info")?;

    // The file holds a list of the mods, or a table that holds the list.
    let list = match &json {
        Json::Array(items) => items.iter().collect::<Vec<_>>(),
        Json::Object(table) => table
            .get("modList")
            .and_then(Json::as_array)
            .map(|items| items.iter().collect())
            .unwrap_or_default(),
        _ => return None,
    };

    let mod_info = list.first()?;

    let mut authors = json_text(mod_info, "author");
    if crate::util::string::is_blank(&authors) {
        authors = json_string_list(mod_info, "authors");
    }
    if crate::util::string::is_blank(&authors) {
        authors = json_string_list(mod_info, "authorList");
    }
    if crate::util::string::is_blank(&authors) {
        authors = json_text(mod_info, "credits");
    }

    let url = json_text(mod_info, "url");
    let url = if crate::util::string::is_blank(&url) {
        json_text(mod_info, "updateUrl")
    } else {
        url
    };

    Some(Metadata {
        id: json_text(mod_info, "modid"),
        name: json_text(mod_info, "name"),
        version: json_text(mod_info, "version"),
        game_version: json_text(mod_info, "mcversion"),
        authors,
        description: json_text_or_object(mod_info, "description"),
        url,
        logo: json_text(mod_info, "logoFile"),
        loader: ModLoaderType::Forge,
    })
}

/// Reads the metadata of a Forge mod of 1.13 and later.
fn read_forge_new(read: &EntryReader<'_>) -> Option<Metadata> {
    read_mods_toml(read, "META-INF/mods.toml", ModLoaderType::Forge)
}

/// Reads the metadata of a NeoForge mod.
fn read_neoforge(read: &EntryReader<'_>) -> Option<Metadata> {
    read_mods_toml(read, "META-INF/neoforge.mods.toml", ModLoaderType::NeoForge)
}

/// Reads the metadata of a mod of Forge or NeoForge from its TOML file.
fn read_mods_toml(read: &EntryReader<'_>, file: &str, loader: ModLoaderType) -> Option<Metadata> {
    let data = read(file)?;
    let text = String::from_utf8_lossy(&data);
    let table = toml::parse(&text).ok()?;

    let mods = table.get("mods").and_then(toml::Value::as_array)?;
    let first = mods.first()?.as_table()?;

    let mod_id = toml_text(first, "modId");

    // The version is written by the build of the mod when the metadata is
    // packed, so a version that names the version of the file is filled in.
    let jar_version = read_manifest_version(read).unwrap_or_default();
    let version = toml_text(first, "version").replace("${file.jarVersion}", &jar_version);

    let logo = {
        let own = toml_text(first, "logoFile");
        if crate::util::string::is_blank(&own) {
            toml_text(&table, "logoFile")
        } else {
            own
        }
    };

    Some(Metadata {
        id: mod_id.clone(),
        name: toml_text(first, "displayName"),
        version,
        game_version: String::new(),
        authors: toml_text(first, "authors"),
        description: toml_text(first, "description"),
        url: toml_text(first, "displayURL"),
        logo,
        loader: analyze_loader(&table, &mod_id, loader),
    })
}

/// Reads the metadata of a LiteLoader mod.
fn read_litemod(read: &EntryReader<'_>) -> Option<Metadata> {
    let json = read_json(read, "litemod.json")?;

    Some(Metadata {
        // LiteLoader names the mod by its name, so the name is its identifier.
        id: json_text(&json, "name"),
        name: json_text(&json, "name"),
        version: json_text(&json, "version"),
        game_version: json_text(&json, "mcversion"),
        authors: json_text(&json, "author"),
        description: json_text_or_object(&json, "description"),
        url: json_text(&json, "updateURI"),
        loader: ModLoaderType::LiteLoader,
        ..Metadata::default()
    })
}

/// Returns the loader a mod of Forge or NeoForge declares it needs.
///
/// A mod of Forge 1.20.1 and later may run on NeoForge and the other way
/// around, so the metadata names the loader the mod was built for in its
/// dependencies. The loader the file was read as is kept when the metadata
/// names neither.
fn analyze_loader(table: &toml::Table, mod_id: &str, loader: ModLoaderType) -> ModLoaderType {
    // The dependencies of a mod are written under its identifier, but some
    // mods write them under `dependencies` alone.
    let dependencies = table
        .get("dependencies")
        .and_then(|dependencies| match dependencies {
            toml::Value::Table(table) => table.get(mod_id).or(Some(dependencies)),
            _ => Some(dependencies),
        })
        .and_then(toml::Value::as_array);

    let Some(dependencies) = dependencies else {
        return loader;
    };

    for dependency in dependencies {
        let Some(dependency) = dependency.as_table() else {
            continue;
        };
        match toml_text(dependency, "modId").as_str() {
            "forge" => return ModLoaderType::Forge,
            "neoforge" => return ModLoaderType::NeoForge,
            _ => {}
        }
    }

    loader
}

/// Reads the version of the file of a mod from its manifest.
///
/// The build of a mod writes the version of the file into `MANIFEST.MF`, and
/// the metadata of the mod names it, so the version of a mod that is built by a
/// tool chain is only known from the manifest.
fn read_manifest_version(read: &EntryReader<'_>) -> Option<String> {
    let data = read("META-INF/MANIFEST.MF")?;
    let text = String::from_utf8_lossy(&data);
    manifest_attribute(&text, "Implementation-Version")
}

/// Returns the value of an attribute of a manifest.
///
/// A manifest writes a line that is too long as several lines, where every line
/// after the first starts with a space; the lines are joined before the
/// attributes are read.
fn manifest_attribute(manifest: &str, name: &str) -> Option<String> {
    let mut attributes: Vec<String> = Vec::new();
    for line in manifest.lines() {
        match line.strip_prefix(' ') {
            Some(continuation) => {
                if let Some(last) = attributes.last_mut() {
                    last.push_str(continuation);
                }
            }
            None => attributes.push(line.trim_end_matches('\r').to_string()),
        }
    }

    attributes.iter().find_map(|attribute| {
        let (key, value) = attribute.split_once(':')?;
        if key.trim().eq_ignore_ascii_case(name) {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

/// Reads a file of an archive as JSON.
fn read_json(read: &EntryReader<'_>, file: &str) -> Option<Json> {
    let data = read(file)?;
    serde_json::from_slice(&data).ok()
}

/// Returns the text of a key of a table of JSON.
fn json_text(json: &Json, key: &str) -> String {
    json.get(key)
        .and_then(Json::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Returns the text of a key of a table of JSON.
///
/// The description of a mod is sometimes a table of the texts by the language
/// of the player, so the table is joined into a text.
fn json_text_or_object(json: &Json, key: &str) -> String {
    match json.get(key) {
        Some(Json::String(text)) => text.clone(),
        Some(Json::Object(table)) => table
            .values()
            .filter_map(Json::as_str)
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// Returns the texts of a list of JSON that are joined.
fn json_string_list(json: &Json, key: &str) -> String {
    json.get(key)
        .and_then(Json::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Json::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// Returns the text of a key of a table of TOML.
fn toml_text(table: &toml::Table, key: &str) -> String {
    table
        .get(key)
        .and_then(|value| match value {
            toml::Value::String(text) => Some(text.clone()),
            toml::Value::Integer(number) => Some(number.to_string()),
            toml::Value::Float(number) => Some(number.to_string()),
            toml::Value::Boolean(value) => Some(value.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Reads the metadata of a mod whose archive holds the given files.
    fn metadata_of(files: &[(&str, &str)], loaders: &[ModLoaderType]) -> Option<Metadata> {
        metadata_of_name("mod.jar", files, loaders)
    }

    /// Reads the metadata of a mod whose file has the name.
    fn metadata_of_name(
        file_name: &str,
        files: &[(&str, &str)],
        loaders: &[ModLoaderType],
    ) -> Option<Metadata> {
        let entries: BTreeMap<String, Vec<u8>> = files
            .iter()
            .map(|(name, content)| ((*name).to_string(), content.as_bytes().to_vec()))
            .collect();

        let read: Box<EntryReader<'_>> = Box::new(move |name: &str| entries.get(name).cloned());

        let extension = file_name
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())?;
        let readers = readers_for(&extension)?;

        for supported in [true, false] {
            for (loader, reader) in readers {
                if loaders.contains(loader) == supported {
                    if let Some(metadata) = reader(&*read) {
                        return Some(metadata);
                    }
                }
            }
        }

        None
    }

    #[test]
    fn the_metadata_of_a_fabric_mod_is_read() {
        let metadata = metadata_of(
            &[(
                "fabric.mod.json",
                r#"{
                    "id": "sodium",
                    "name": "Sodium",
                    "version": "0.5.8",
                    "description": "A modern rendering engine",
                    "icon": "assets/sodium/icon.png",
                    "authors": [ "jellysquid3", { "name": "CaffeineMC" } ],
                    "contact": { "homepage": "https://example.com/sodium" }
                }"#,
            )],
            &[ModLoaderType::Fabric],
        )
        .expect("the metadata should be read");

        assert_eq!(metadata.id, "sodium");
        assert_eq!(metadata.name, "Sodium");
        assert_eq!(metadata.version, "0.5.8");
        assert_eq!(metadata.authors, "jellysquid3, CaffeineMC");
        assert_eq!(metadata.url, "https://example.com/sodium");
        assert_eq!(metadata.logo, "assets/sodium/icon.png");
        assert_eq!(metadata.loader, ModLoaderType::Fabric);
    }

    #[test]
    fn the_metadata_of_a_quilt_mod_is_read() {
        let metadata = metadata_of(
            &[(
                "quilt.mod.json",
                r#"{
                    "schema_version": 1,
                    "quilt_loader": {
                        "id": "example",
                        "version": "2.0.0",
                        "metadata": {
                            "name": "Example",
                            "description": "Does things",
                            "contributors": { "alice": "Author" },
                            "icon": "icon.png",
                            "contact": { "homepage": "https://example.com" }
                        }
                    }
                }"#,
            )],
            &[ModLoaderType::Quilt],
        )
        .expect("the metadata should be read");

        assert_eq!(metadata.id, "example");
        assert_eq!(metadata.name, "Example");
        assert_eq!(metadata.authors, "alice (Author)");
        assert_eq!(metadata.loader, ModLoaderType::Quilt);
    }

    #[test]
    fn the_metadata_of_another_version_of_quilt_is_refused() {
        let metadata = metadata_of(
            &[(
                "quilt.mod.json",
                r#"{ "schema_version": 2, "quilt_loader": { "id": "x" } }"#,
            )],
            &[ModLoaderType::Quilt],
        );
        assert!(metadata.is_none());
    }

    #[test]
    fn the_metadata_of_a_forge_mod_of_an_old_version_is_read() {
        let metadata = metadata_of(
            &[(
                "mcmod.info",
                r#"[{
                    "modid": "example",
                    "name": "Example",
                    "description": "Does things",
                    "authorList": [ "alice", "bob" ],
                    "version": "1.0",
                    "mcversion": "1.12.2",
                    "updateUrl": "",
                    "url": "https://example.com",
                    "logoFile": "logo.png"
                }]"#,
            )],
            &[ModLoaderType::Forge],
        )
        .expect("the metadata should be read");

        assert_eq!(metadata.id, "example");
        assert_eq!(metadata.game_version, "1.12.2");
        assert_eq!(metadata.authors, "alice, bob");
        assert_eq!(metadata.url, "https://example.com");
    }

    #[test]
    fn the_metadata_of_a_forge_mod_is_read_from_the_mod_list() {
        let metadata = metadata_of(
            &[(
                "mcmod.info",
                r#"{ "modListVersion": 2, "modList": [ { "modid": "wrapped", "name": "Wrapped" } ] }"#,
            )],
            &[ModLoaderType::Forge],
        )
        .expect("the metadata should be read");

        assert_eq!(metadata.id, "wrapped");
        assert_eq!(metadata.name, "Wrapped");
    }

    #[test]
    fn the_metadata_of_a_forge_mod_of_a_new_version_is_read() {
        let metadata = metadata_of(
            &[
                (
                    "META-INF/mods.toml",
                    r#"
modLoader = "javafml"
loaderVersion = "[47,)"
license = "MIT"

[[mods]]
modId = "example"
version = "${file.jarVersion}"
displayName = "Example"
authors = "Somebody"
description = "Does things"
displayURL = "https://example.com"

[[dependencies.example]]
modId = "forge"
mandatory = true
versionRange = "[47,)"
"#,
                ),
                (
                    "META-INF/MANIFEST.MF",
                    "Manifest-Version: 1.0\r\nImplementation-Version: 1.2.3\r\n\r\n",
                ),
            ],
            &[ModLoaderType::Forge],
        )
        .expect("the metadata should be read");

        assert_eq!(metadata.id, "example");
        assert_eq!(metadata.version, "1.2.3");
        assert_eq!(metadata.authors, "Somebody");
        assert_eq!(metadata.loader, ModLoaderType::Forge);
    }

    #[test]
    fn the_metadata_of_a_neoforge_mod_names_the_loader() {
        let metadata = metadata_of(
            &[(
                "META-INF/neoforge.mods.toml",
                r#"
modLoader = "javafml"

[[mods]]
modId = "example"
version = "1.0"
displayName = "Example"

[[dependencies.example]]
modId = "neoforge"
type = "required"
"#,
            )],
            &[ModLoaderType::NeoForge],
        )
        .expect("the metadata should be read");

        assert_eq!(metadata.loader, ModLoaderType::NeoForge);
    }

    #[test]
    fn the_metadata_of_a_forge_mod_that_runs_on_neoforge_names_the_loader() {
        let metadata = metadata_of(
            &[(
                "META-INF/mods.toml",
                r#"
[[mods]]
modId = "example"
version = "1.0"
displayName = "Example"

[[dependencies.example]]
modId = "neoforge"
"#,
            )],
            &[ModLoaderType::Forge],
        )
        .expect("the metadata should be read");

        assert_eq!(metadata.loader, ModLoaderType::NeoForge);
    }

    #[test]
    fn the_metadata_of_a_litemod_is_read() {
        let metadata = metadata_of_name(
            "example.litemod",
            &[(
                "litemod.json",
                r#"{ "name": "Example", "version": "1.0", "mcversion": "1.12.2", "author": "alice", "updateURI": "https://example.com" }"#,
            )],
            &[ModLoaderType::LiteLoader],
        )
        .expect("the metadata should be read");

        assert_eq!(metadata.name, "Example");
        assert_eq!(metadata.id, "Example");
        assert_eq!(metadata.game_version, "1.12.2");
        assert_eq!(metadata.url, "https://example.com");
        assert_eq!(metadata.loader, ModLoaderType::LiteLoader);
    }

    #[test]
    fn a_container_that_is_not_a_mod_file_holds_no_metadata() {
        let metadata = metadata_of_name(
            "notes.txt",
            &[(
                "fabric.mod.json",
                r#"{ "id": "example", "name": "Example" }"#,
            )],
            &[],
        );
        assert!(metadata.is_none());
    }

    #[test]
    fn a_file_without_metadata_holds_no_mod() {
        assert!(metadata_of(&[("something.txt", "x")], &[]).is_none());
        assert!(metadata_of(&[("fabric.mod.json", "not json")], &[]).is_none());
    }

    #[test]
    fn the_name_of_a_mod_file_is_read_without_its_suffixes() {
        assert_eq!(display_name("sodium-0.5.8.jar"), "sodium-0.5.8.jar");
        assert_eq!(display_name("sodium.jar.disabled"), "sodium.jar");
        assert_eq!(display_name("sodium.jar.disabled.old"), "sodium.jar");
        assert_eq!(display_name("sodium.jar.old"), "sodium.jar");
    }

    #[test]
    fn the_names_of_the_mod_files_are_recognized() {
        assert!(ModFile::is_file_name_mod(Path::new("mods/sodium.jar")));
        assert!(ModFile::is_file_name_mod(Path::new("mods/sodium.JAR")));
        assert!(ModFile::is_file_name_mod(Path::new(
            "mods/sodium.jar.disabled"
        )));
        assert!(ModFile::is_file_name_mod(Path::new("mods/old.litemod")));
        assert!(!ModFile::is_file_name_mod(Path::new("mods/notes.txt")));
        assert!(!ModFile::is_file_name_mod(Path::new("mods/sodium")));
    }

    #[test]
    fn the_attributes_of_a_manifest_are_read() {
        let manifest = "Manifest-Version: 1.0\r\nImplementation-Version: 1.2.3\r\n\r\n";
        assert_eq!(
            manifest_attribute(manifest, "Implementation-Version"),
            Some("1.2.3".to_string())
        );

        // A manifest breaks a long line after a space: the space is kept at the
        // end of the line and the next line starts with one that is dropped.
        let wrapped = "Manifest-Version: 1.0\nImplementation-Title: a name that is \n long\n";
        assert_eq!(
            manifest_attribute(wrapped, "Implementation-Title"),
            Some("a name that is long".to_string())
        );
        assert_eq!(manifest_attribute(wrapped, "Missing"), None);
    }
}
