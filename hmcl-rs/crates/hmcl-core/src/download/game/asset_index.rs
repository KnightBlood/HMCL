//! The index of the assets of a version of the game.
//!
//! The index holds the files the game reads its textures, its sounds and its
//! texts from; the files are named by the hash of their content, so the same
//! file is downloaded once and used by every version of the game.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The address the files of the assets are downloaded from.
pub const ASSET_URL: &str = "https://resources.download.minecraft.net/";

/// A file of the assets of a version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetObject {
    /// The digest of the file, in the hexadecimal of SHA-1.
    pub hash: String,
    /// The size of the file, in bytes.
    #[serde(default)]
    pub size: u64,
}

impl AssetObject {
    /// Returns the path of the file in the directory of the assets.
    ///
    /// The files are kept under a directory named after the first two
    /// characters of their digest, so a directory does not hold every file of
    /// the game.
    pub fn location(&self) -> String {
        let prefix: String = self.hash.chars().take(2).collect();
        format!("{prefix}/{}", self.hash)
    }

    /// Returns the address the file is downloaded from.
    pub fn url(&self) -> String {
        format!("{ASSET_URL}{}", self.location())
    }
}

/// The index of the assets of a version.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetIndexFile {
    /// Whether the game reads the assets from a directory that maps the names
    /// of the files to the files themselves.
    ///
    /// The versions before the game stored its assets by their hashes read the
    /// assets from such a directory.
    #[serde(default, rename = "virtual")]
    pub virtual_assets: bool,
    /// Whether the assets are mapped to the directory of the resources.
    ///
    /// The old versions of the game read some of their assets from a directory
    /// that mirrors the directory of the resources.
    #[serde(default)]
    pub map_to_resources: bool,
    /// The files of the assets, by the names the game reads them by.
    #[serde(default)]
    pub objects: BTreeMap<String, AssetObject>,
}

impl AssetIndexFile {
    /// Returns whether the game reads the assets from a directory that maps the
    /// names of the files to the files themselves.
    pub fn is_virtual(&self) -> bool {
        self.virtual_assets || self.map_to_resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_of_the_assets_are_read() {
        let index: AssetIndexFile = serde_json::from_str(
            r#"{
                "objects": {
                    "minecraft/sounds/click.ogg": {
                        "hash": "abcdef0123456789",
                        "size": 1234
                    }
                }
            }"#,
        )
        .expect("the index should be read");

        assert_eq!(index.objects.len(), 1);
        assert!(!index.is_virtual());

        let object = index
            .objects
            .get("minecraft/sounds/click.ogg")
            .expect("the file should be read");
        assert_eq!(object.hash, "abcdef0123456789");
        assert_eq!(object.size, 1234);
        assert_eq!(object.location(), "ab/abcdef0123456789");
        assert_eq!(
            object.url(),
            "https://resources.download.minecraft.net/ab/abcdef0123456789"
        );
    }

    #[test]
    fn the_old_index_of_the_assets_is_read() {
        let index: AssetIndexFile = serde_json::from_str(
            r#"{
                "virtual": true,
                "map_to_resources": false,
                "objects": {}
            }"#,
        )
        .expect("the index should be read");

        assert!(index.is_virtual());
        assert!(index.objects.is_empty());
    }

    #[test]
    fn an_index_without_the_virtual_flag_is_read() {
        let index: AssetIndexFile =
            serde_json::from_str(r#"{ "objects": {} }"#).expect("the index should be read");

        assert!(!index.is_virtual());
    }
}
