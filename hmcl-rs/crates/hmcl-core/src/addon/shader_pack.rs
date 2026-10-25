//! The shader packs the player keeps in a game directory.
//!
//! A shader pack is a directory or an archive of shaders, which a mod such as
//! Iris or OptiFine loads to replace the shaders of the game. The launcher does
//! not read the shaders; it lists the packs, and it enables one the way the mod
//! records it, so the player can change the pack from the launcher.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The directory of the shader packs of a game directory.
pub const SHADER_PACKS_DIRECTORY: &str = "shaderpacks";

/// The file Iris keeps the name of the shader pack in, below `config`.
const IRIS_CONFIG: &str = "iris.properties";

/// The key of the name of the shader pack in the file of Iris.
const IRIS_SHADER_PACK_KEY: &str = "shaderPack";

/// The extensions of the archives of the shader packs.
const SHADER_PACK_EXTENSIONS: [&str; 2] = ["zip", "jar"];

/// A shader pack of a game directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderPack {
    /// The directory or the archive of the shader pack.
    path: PathBuf,
    /// The name of the pack, which is the name of its file or directory.
    name: String,
}

impl ShaderPack {
    /// Reads a shader pack of a directory or an archive.
    ///
    /// A shader pack is either a directory that holds the shaders or an archive
    /// of them, so a file of another kind is not a pack.
    pub fn open(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref();
        let name = path.file_name()?.to_str()?.to_string();

        if !Self::is_shader_pack(path) {
            return None;
        }

        Some(Self {
            path: path.to_path_buf(),
            name,
        })
    }

    /// Returns whether a path is a shader pack.
    pub fn is_shader_pack(path: &Path) -> bool {
        if path.is_dir() {
            return true;
        }
        if !path.is_file() {
            return false;
        }

        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                let extension = crate::util::string::substring_after_last(name, '.');
                SHADER_PACK_EXTENSIONS
                    .iter()
                    .any(|kind| extension.eq_ignore_ascii_case(kind))
            })
            .unwrap_or(false)
    }

    /// Returns the directory or the archive of the shader pack.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the name of the pack, which is the name of its file.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the name of the pack without the extension of its archive.
    pub fn display_name(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.name)
    }

    /// Deletes the pack.
    pub fn delete(&self) -> std::io::Result<()> {
        if self.path.is_dir() {
            std::fs::remove_dir_all(&self.path)
        } else {
            std::fs::remove_file(&self.path)
        }
    }
}

/// Lists the shader packs of a game directory, sorted by their names.
pub fn list_shader_packs(directory: &Path) -> Vec<ShaderPack> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut packs = entries
        .flatten()
        .filter_map(|entry| ShaderPack::open(entry.path()))
        .collect::<Vec<_>>();
    packs.sort_by(|left, right| left.name().cmp(right.name()));
    packs
}

/// Returns the name of the shader pack that is enabled, if one is.
///
/// Iris records the name of the pack it loads in its configuration; the value
/// is a file name, and an empty value tells that no pack is enabled.
pub fn enabled_shader_pack(game_directory: &Path) -> Option<String> {
    let config = game_directory.join("config").join(IRIS_CONFIG);
    let properties = read_properties(&config);

    properties
        .get(IRIS_SHADER_PACK_KEY)
        .filter(|name| !name.trim().is_empty())
        .cloned()
}

/// Enables a shader pack, or disables the shaders when no pack is given.
///
/// The name is written to the configuration of Iris, which the mod reads when
/// the game starts. The other properties of the file are kept, because they
/// hold the settings of the player. Returns whether the file was changed.
pub fn set_enabled_shader_pack(game_directory: &Path, name: Option<&str>) -> std::io::Result<bool> {
    let config = game_directory.join("config").join(IRIS_CONFIG);
    let mut properties = read_properties(&config);
    let value = name.unwrap_or_default();

    if properties
        .get(IRIS_SHADER_PACK_KEY)
        .is_some_and(|current| current == value)
    {
        return Ok(false);
    }

    properties.insert(IRIS_SHADER_PACK_KEY.to_string(), value.to_string());

    if let Some(parent) = config.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // The file is written next to the old one and moved into place, so a
    // failure does not leave a configuration the mod cannot read.
    let temporary = config.with_extension("properties.part");
    std::fs::write(&temporary, format_properties(&properties))?;
    match std::fs::rename(&temporary, &config) {
        Ok(()) => Ok(true),
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            Err(error)
        }
    }
}

/// Copies a shader pack into the directory of the packs.
///
/// A directory is copied with its files, and an archive is copied as it is. A
/// pack that already has the name is not replaced.
pub fn import_shader_pack(directory: &Path, path: &Path) -> std::io::Result<ShaderPack> {
    if !ShaderPack::is_shader_pack(path) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the path is not a shader pack",
        ));
    }

    std::fs::create_dir_all(directory)?;

    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "the pack has no name")
    })?;
    let target = directory.join(name);

    if target.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "the shader pack already exists",
        ));
    }

    if path.is_dir() {
        copy_directory(path, &target)?;
    } else {
        std::fs::copy(path, &target)?;
    }

    ShaderPack::open(&target).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the shader pack cannot be read",
        )
    })
}

/// Copies a directory and the files under it.
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

/// Reads the properties of a file.
///
/// A file that cannot be read holds no property, because the launcher shows the
/// shaders as disabled when the mod has not run yet.
fn read_properties(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };

    let mut properties = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }

        let (key, value) = match line.split_once('=') {
            Some(pair) => pair,
            // A line without a separator is a key with an empty value.
            None => (line, ""),
        };
        properties.insert(key.trim().to_string(), value.trim().to_string());
    }

    properties
}

/// Writes the properties of a file.
///
/// The keys are written in their order, so the file of the launcher is the same
/// every time it is written.
fn format_properties(properties: &BTreeMap<String, String>) -> String {
    let mut text = String::new();
    for (key, value) in properties {
        text.push_str(key);
        text.push('=');
        text.push_str(value);
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-shader-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    #[test]
    fn the_shader_packs_of_a_directory_are_read() {
        let base = directory("list");
        std::fs::create_dir_all(base.join("Complementary")).expect("dir");
        std::fs::write(base.join("BSL.zip"), b"zip").expect("write");
        std::fs::write(base.join("notes.txt"), b"text").expect("write");

        let packs = list_shader_packs(&base);
        let names = packs.iter().map(ShaderPack::name).collect::<Vec<_>>();
        assert_eq!(names, vec!["BSL.zip", "Complementary"]);

        assert_eq!(packs[0].display_name(), "BSL");
        assert_eq!(packs[1].display_name(), "Complementary");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_missing_directory_holds_no_shader_pack() {
        let base = directory("missing");
        assert!(list_shader_packs(&base.join("shaderpacks")).is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_kinds_of_the_packs_are_read() {
        let base = directory("kinds");
        std::fs::create_dir_all(base.join("folder")).expect("dir");
        std::fs::write(base.join("pack.zip"), b"zip").expect("write");
        std::fs::write(base.join("pack.JAR"), b"jar").expect("write");
        std::fs::write(base.join("pack.txt"), b"text").expect("write");

        assert!(ShaderPack::open(base.join("folder")).is_some());
        assert!(ShaderPack::open(base.join("pack.zip")).is_some());
        assert!(ShaderPack::open(base.join("pack.JAR")).is_some());
        assert!(ShaderPack::open(base.join("pack.txt")).is_none());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_shader_pack_is_enabled_and_disabled() {
        let base = directory("enable");
        let config = base.join("config").join(IRIS_CONFIG);
        std::fs::create_dir_all(base.join("config")).expect("dir");
        std::fs::write(&config, "# the settings of the player\nenabled=true\n").expect("write");

        assert_eq!(enabled_shader_pack(&base), None);

        let changed =
            set_enabled_shader_pack(&base, Some("BSL.zip")).expect("the pack should be set");
        assert!(changed);
        assert_eq!(enabled_shader_pack(&base).as_deref(), Some("BSL.zip"));

        // The other settings of the file are kept.
        let text = std::fs::read_to_string(&config).expect("read");
        assert!(text.contains("enabled=true"));
        assert!(text.contains("shaderPack=BSL.zip"));

        // Writing the same pack again does not change the file.
        assert!(!set_enabled_shader_pack(&base, Some("BSL.zip")).expect("the pack should be set"));

        let changed = set_enabled_shader_pack(&base, None).expect("the pack should be cleared");
        assert!(changed);
        assert_eq!(enabled_shader_pack(&base), None);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_shader_pack_is_imported() {
        let base = directory("import");
        let packs = base.join("shaderpacks");
        std::fs::create_dir_all(base.join("source/Complementary/shaders")).expect("dir");
        std::fs::write(
            base.join("source/Complementary/shaders/final.fsh"),
            b"shader",
        )
        .expect("write");

        let pack = import_shader_pack(&packs, &base.join("source/Complementary"))
            .expect("the pack should be imported");
        assert_eq!(pack.name(), "Complementary");
        assert!(packs.join("Complementary/shaders/final.fsh").is_file());

        let error = import_shader_pack(&packs, &base.join("source/Complementary"))
            .expect_err("the pack should not be replaced");
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_path_that_is_not_a_pack_is_not_imported() {
        let base = directory("import-invalid");
        std::fs::write(base.join("notes.txt"), b"text").expect("write");

        let error = import_shader_pack(&base.join("packs"), &base.join("notes.txt"))
            .expect_err("the file should be refused");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_shader_pack_is_deleted() {
        let base = directory("delete");
        std::fs::create_dir_all(base.join("pack")).expect("dir");
        std::fs::write(base.join("archive.zip"), b"zip").expect("write");

        ShaderPack::open(base.join("pack"))
            .expect("the pack")
            .delete()
            .expect("the pack should be deleted");
        ShaderPack::open(base.join("archive.zip"))
            .expect("the pack")
            .delete()
            .expect("the pack should be deleted");

        assert!(!base.join("pack").exists());
        assert!(!base.join("archive.zip").exists());

        let _ = std::fs::remove_dir_all(&base);
    }
}
