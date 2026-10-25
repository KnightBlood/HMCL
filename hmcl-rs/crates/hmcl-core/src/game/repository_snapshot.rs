//! The published index of the instances of a game directory.
//!
//! A snapshot is published as a whole value: once a snapshot is visible the
//! launcher never edits it, it replaces it with a newer one. A caller that
//! reads several instances in a row therefore keeps one snapshot, so the
//! instance that is listed and the manifest that is read belong together even
//! when another part of the launcher writes the repository at the same time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::download::game::InstanceManifest;
use crate::util::versioning::GameVersionNumber;

/// An immutable index of the instances of a game directory.
///
/// The snapshot holds the manifests that were read when it was published, the
/// files they were read from and the versions of the game of the instances
/// whose version the launcher could read. A snapshot is created by
/// [`GameRepository::read`](super::GameRepository::read) and replaced as a
/// whole by [`GameRepository::refresh`](super::GameRepository::refresh) and by
/// the commit of a [`GameRepositoryDraft`](super::GameRepositoryDraft).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RepositorySnapshot {
    /// The directory the game directory keeps its files in.
    pub(crate) base: PathBuf,
    /// The manifests of the instances, by their identifiers.
    pub(crate) manifests: BTreeMap<String, InstanceManifest>,
    /// The files the manifests of the instances are read from.
    pub(crate) manifest_files: BTreeMap<String, PathBuf>,
    /// The versions of the game of the instances whose version is known.
    pub(crate) versions: BTreeMap<String, GameVersionNumber>,
}

impl RepositorySnapshot {
    /// Creates an empty index of the instances of a game directory.
    pub(crate) fn new(base: impl Into<PathBuf>) -> Self {
        Self {
            base: base.into(),
            ..Self::default()
        }
    }

    /// Returns the directory the game directory keeps its files in.
    pub fn base_directory(&self) -> &Path {
        &self.base
    }

    /// Returns the identifiers of the instances.
    pub fn instance_ids(&self) -> impl Iterator<Item = &str> {
        self.manifests.keys().map(String::as_str)
    }

    /// Returns how many instances the snapshot holds.
    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    /// Returns whether the snapshot holds no instance.
    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }

    /// Returns whether the snapshot holds an instance.
    pub fn contains(&self, id: &str) -> bool {
        self.manifests.contains_key(id)
    }

    /// Returns the manifest of an instance, as it is written on the disk.
    pub fn manifest(&self, id: &str) -> Option<&InstanceManifest> {
        self.manifests.get(id)
    }

    /// Returns the file the manifest of an instance is read from.
    pub fn manifest_file(&self, id: &str) -> Option<&Path> {
        self.manifest_files.get(id).map(PathBuf::as_path)
    }

    /// Returns the file of the game of an instance.
    ///
    /// The file is the file of the manifest with the extension of the game, so
    /// an instance whose manifest is not named after it still finds its game.
    pub fn jar_file(&self, id: &str) -> Option<PathBuf> {
        self.manifest_file(id)
            .map(|file| file.with_extension("jar"))
    }

    /// Returns the version of the game of an instance, if the launcher read it.
    pub fn version(&self, id: &str) -> Option<&GameVersionNumber> {
        self.versions.get(id)
    }

    /// Returns the root directory of an instance.
    ///
    /// The directory is the one the instance keeps its files in, whether or not
    /// the snapshot holds the instance; an instance the snapshot does not know
    /// has the directory its identifier names.
    pub fn instance_root(&self, id: &str) -> PathBuf {
        self.base.join("versions").join(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_instances_of_a_snapshot_are_read() {
        let mut snapshot = RepositorySnapshot::new("/games/.minecraft");
        snapshot
            .manifests
            .insert("1.20.1".to_string(), InstanceManifest::default());
        snapshot.manifest_files.insert(
            "1.20.1".to_string(),
            PathBuf::from("/games/.minecraft/versions/1.20.1/1.20.1.json"),
        );

        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.contains("1.20.1"));
        assert_eq!(snapshot.instance_ids().collect::<Vec<_>>(), ["1.20.1"]);
        assert_eq!(
            snapshot.jar_file("1.20.1"),
            Some(PathBuf::from(
                "/games/.minecraft/versions/1.20.1/1.20.1.jar"
            ))
        );
        assert_eq!(
            snapshot.instance_root("other"),
            PathBuf::from("/games/.minecraft/versions/other")
        );
        assert_eq!(snapshot.base_directory(), Path::new("/games/.minecraft"));
    }
}
