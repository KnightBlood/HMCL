//! The assets of the old versions of the game.
//!
//! The versions before the game named its assets after the digests of their
//! content read their assets from a directory that maps the names the game
//! reads to the files; the launcher copies the files of the index into that
//! directory before it starts the game.

use std::io;
use std::path::{Path, PathBuf};

use crate::download::game::{AssetIndexFile, AssetObject};

/// A file of the assets of a version that reads its assets by their names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualAsset {
    /// The name the game reads the file by.
    pub name: String,
    /// The file of the assets.
    pub object: AssetObject,
}

/// The files of the assets of a version that reads its assets by their names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualAssets {
    /// The directory the game reads the assets from.
    pub directory: PathBuf,
    /// The directory the assets are copied to when the version reads them from
    /// the resources of the game.
    pub resources_directory: Option<PathBuf>,
    /// The directory the files of the assets are kept in.
    pub objects_directory: PathBuf,
    /// The files of the assets, by the names the game reads them by.
    pub files: Vec<VirtualAsset>,
}

impl VirtualAssets {
    /// Copies the files of the assets the version reads by their names.
    ///
    /// A file that is already there is kept, so the launcher may copy the
    /// files again without reading the files of the game; the files the index
    /// tells but the launcher does not have are skipped, so the game runs with
    /// the files it has.
    pub fn copy(&self) -> io::Result<()> {
        for file in &self.files {
            let Some(relative) = relative_path(&file.name) else {
                continue;
            };
            let source = self.objects_directory.join(file.object.location());
            if !source.is_file() {
                continue;
            }

            copy_if_absent(&source, &self.directory.join(&relative))?;
            if let Some(resources) = &self.resources_directory {
                copy_if_absent(&source, &resources.join(&relative))?;
            }
        }

        Ok(())
    }
}

/// Returns the files of the assets of a version that reads its assets by their
/// names.
///
/// The versions that read the assets by the digests of their content, and the
/// versions that do not tell where their assets are, read the files from the
/// directory of the assets itself; such versions have no files to copy.
pub fn virtual_assets(
    index: &AssetIndexFile,
    assets_directory: &Path,
    asset_id: &str,
    game_directory: &Path,
) -> Option<VirtualAssets> {
    if !index.is_virtual() || !is_simple_name(asset_id) {
        return None;
    }

    Some(VirtualAssets {
        directory: assets_directory.join("virtual").join(asset_id),
        resources_directory: index
            .map_to_resources
            .then(|| game_directory.join("resources")),
        objects_directory: assets_directory.join("objects"),
        files: index
            .objects
            .iter()
            .map(|(name, object)| VirtualAsset {
                name: name.clone(),
                object: object.clone(),
            })
            .collect(),
    })
}

/// Returns whether a name is one part of a path.
fn is_simple_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':'])
}

/// Returns the path of a file of the assets under a directory.
///
/// The names of the assets are read as relative paths, so an index of the
/// assets cannot name a file outside of the directory.
fn relative_path(name: &str) -> Option<PathBuf> {
    let name = name.replace('\\', "/");
    if name.starts_with('/') || name.contains(':') {
        return None;
    }

    let mut path = PathBuf::new();
    for part in name.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            part => path.push(part),
        }
    }

    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

/// Copies a file unless the file is already there.
fn copy_if_absent(source: &Path, target: &Path) -> io::Result<()> {
    if target.is_file() {
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::copy(source, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn index(virtual_assets: bool, map_to_resources: bool) -> AssetIndexFile {
        AssetIndexFile {
            virtual_assets,
            map_to_resources,
            objects: BTreeMap::from([(
                "minecraft/sounds/click.ogg".to_string(),
                AssetObject {
                    hash: "abcdef0123456789".to_string(),
                    size: 5,
                },
            )]),
        }
    }

    fn directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join("hmcl-rs-assets-tests").join(name);
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    #[test]
    fn a_version_that_reads_the_assets_by_their_digests_has_no_files_to_copy() {
        let directory = directory("digests");

        assert_eq!(
            virtual_assets(&index(false, false), &directory, "1.20", &directory),
            None
        );
    }

    #[test]
    fn the_files_of_the_assets_are_copied_by_their_names() {
        let directory = directory("copy");
        let objects = directory.join("objects/ab");
        std::fs::create_dir_all(&objects).expect("the directory should be made");
        std::fs::write(objects.join("abcdef0123456789"), b"click")
            .expect("the file should be written");

        let assets = virtual_assets(&index(true, false), &directory, "1.6", &directory)
            .expect("the assets should be virtual");

        assert_eq!(assets.directory, directory.join("virtual/1.6"));
        assert_eq!(assets.resources_directory, None);
        assert_eq!(assets.files.len(), 1);
        assert_eq!(assets.files[0].name, "minecraft/sounds/click.ogg");

        assets.copy().expect("the assets should be copied");

        assert_eq!(
            std::fs::read(assets.directory.join("minecraft/sounds/click.ogg"))
                .expect("the file should be read"),
            b"click"
        );
        assert!(!directory.join("resources").exists());
    }

    #[test]
    fn the_assets_of_the_resources_are_copied_too() {
        let directory = directory("resources");
        let objects = directory.join("objects/ab");
        std::fs::create_dir_all(&objects).expect("the directory should be made");
        std::fs::write(objects.join("abcdef0123456789"), b"click")
            .expect("the file should be written");

        let assets = virtual_assets(&index(true, true), &directory, "1.6", &directory)
            .expect("the assets should be virtual");
        assets.copy().expect("the assets should be copied");

        assert_eq!(
            std::fs::read(directory.join("resources/minecraft/sounds/click.ogg"))
                .expect("the file should be read"),
            b"click"
        );
    }

    #[test]
    fn the_files_the_launcher_does_not_have_are_skipped() {
        let directory = directory("missing");
        let assets = virtual_assets(&index(true, false), &directory, "1.6", &directory)
            .expect("the assets should be virtual");

        assets.copy().expect("the assets should be copied");

        assert!(!assets.directory.exists());
    }

    #[test]
    fn a_name_outside_of_the_directory_is_skipped() {
        let directory = directory("outside");
        let objects = directory.join("objects/ab");
        std::fs::create_dir_all(&objects).expect("the directory should be made");
        std::fs::write(objects.join("abcdef0123456789"), b"click")
            .expect("the file should be written");

        let mut index = index(true, false);
        index.objects = BTreeMap::from([(
            "../outside.ogg".to_string(),
            AssetObject {
                hash: "abcdef0123456789".to_string(),
                size: 5,
            },
        )]);
        let assets = virtual_assets(&index, &directory, "1.6", &directory)
            .expect("the assets should be virtual");

        assets.copy().expect("the assets should be copied");

        assert!(!directory.join("outside.ogg").exists());
    }

    #[test]
    fn an_index_of_a_name_that_is_not_one_part_of_a_path_is_not_read() {
        let directory = directory("id");

        assert_eq!(
            virtual_assets(&index(true, false), &directory, "../1.6", &directory),
            None
        );
    }
}
