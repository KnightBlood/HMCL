//! The work an update of a modpack makes.
//!
//! The launcher does not download a modpack at once: the formats that keep
//! their files on a server publish a manifest, and the launcher compares the
//! manifest with the files of the instance. The result is a plan that names
//! the files to download and the files to remove, which the download layer of
//! the launcher carries out. Keeping the comparison free of the download makes
//! it testable, and it tells the player what an update will change before it
//! starts.

use std::path::{Path, PathBuf};

/// A file an update downloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadRequest {
    /// The address the file is downloaded from.
    pub url: String,
    /// The file the download is written to.
    pub path: PathBuf,
    /// The SHA-1 digest the file must have, if the manifest tells one.
    pub sha1: Option<String>,
}

/// The files an update downloads and removes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdatePlan {
    /// The files the update downloads.
    pub downloads: Vec<DownloadRequest>,
    /// The files the update removes, because the new version of the modpack
    /// no longer holds them.
    pub removals: Vec<PathBuf>,
    /// What the launcher could not read of the manifest.
    ///
    /// The entries the launcher skips are reported instead of being silently
    /// dropped, so the player knows why a file of the modpack is not updated.
    pub warnings: Vec<String>,
}

impl UpdatePlan {
    /// Returns whether the plan changes the instance.
    pub fn is_empty(&self) -> bool {
        self.downloads.is_empty() && self.removals.is_empty()
    }
}

/// Returns the path of a file of a manifest under a directory.
///
/// The paths of the manifests always use `/` as the separator, whichever
/// system the launcher runs on. A path that leaves the directory is refused,
/// because the archive of a modpack is written by a stranger: a file outside
/// of the instance would overwrite the files of the player.
///
/// Returns `None` when the path is empty or leaves the directory; the caller
/// reports such a path instead of writing the file.
pub fn relative_path(base: &Path, path: &str) -> Option<PathBuf> {
    if path.is_empty() {
        return Some(base.to_path_buf());
    }

    let path = path.replace('\\', "/");
    if path.starts_with('/') || path.contains(':') {
        return None;
    }

    let mut relative = PathBuf::new();
    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            part => relative.push(part),
        }
    }

    if relative.as_os_str().is_empty() {
        Some(base.to_path_buf())
    } else {
        Some(base.join(relative))
    }
}

/// Returns the SHA-1 digest of a file, if the file can be read.
///
/// A file that is not there or cannot be read is reported as a file whose
/// digest is not known, which makes the update download it again.
pub fn sha1_of(path: &Path) -> Option<String> {
    crate::util::digest::Algorithm::Sha1
        .digest_file_to_string(path)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paths_of_a_manifest_are_read_under_the_instance() {
        let base = Path::new("/games/instances/world");
        assert_eq!(
            relative_path(base, "mods/a.jar"),
            Some(PathBuf::from("/games/instances/world/mods/a.jar"))
        );
        assert_eq!(
            relative_path(base, "mods\\a.jar"),
            Some(PathBuf::from("/games/instances/world/mods/a.jar"))
        );
        assert_eq!(
            relative_path(base, "config/./a.toml"),
            Some(PathBuf::from("/games/instances/world/config/a.toml"))
        );
    }

    #[test]
    fn a_path_that_leaves_the_instance_is_refused() {
        let base = Path::new("/games/instances/world");
        assert_eq!(relative_path(base, "mods/../../outside.jar"), None);
        assert_eq!(relative_path(base, "/etc/passwd"), None);
        assert_eq!(relative_path(base, "C:/Windows/system32"), None);
    }

    #[test]
    fn the_digest_of_a_missing_file_is_not_known() {
        let path =
            std::env::temp_dir().join(format!("hmcl-rs-plan-missing-{}", std::process::id()));
        assert_eq!(sha1_of(&path), None);
    }
}
