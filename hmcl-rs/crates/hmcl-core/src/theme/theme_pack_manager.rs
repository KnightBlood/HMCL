//! The theme packs the launcher installed.
//!
//! The launcher keeps the theme packs the player installed in a directory of
//! its own; a pack is a zip file named after the identifier of the pack. The
//! launcher lists them, installs a pack a player chose and removes one again.
//!
//! The rules of the launcher of Java are kept: a file whose name does not begin
//! with a dot and ends with the extension of a theme pack is a candidate, and a
//! directory that holds a manifest is a candidate as well. The packs are shown
//! with the name of the pack first, so the list is the same whatever the order
//! of the directory is.

use std::path::{Path, PathBuf};

use crate::util::zip;

use super::theme_pack::{ThemePackError, ThemePackManifest, FILE_EXTENSION, MANIFEST_ENTRY};

/// The name of the directory that holds the installed theme packs.
pub const THEME_PACKS_DIRECTORY: &str = "themes";

/// The error of managing the theme packs of the launcher.
#[derive(Debug)]
pub enum ThemePackManagerError {
    /// The content of a file does not have the shape the launcher expects.
    InvalidFormat(String),
    /// A file of the file system could not be read or written.
    Io {
        /// The path of the file, if the operation named one.
        path: Option<PathBuf>,
        /// The failure the operating system reported.
        error: std::io::Error,
    },
}

impl std::fmt::Display for ThemePackManagerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(reason) => formatter.write_str(reason),
            Self::Io { path, error } => {
                write!(formatter, "The file could not be read or written: {error}")?;
                if let Some(path) = path {
                    write!(formatter, " ({})", path.display())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ThemePackManagerError {}

impl From<ThemePackError> for ThemePackManagerError {
    fn from(error: ThemePackError) -> Self {
        match error {
            ThemePackError::InvalidFormat(reason) => Self::InvalidFormat(reason),
            ThemePackError::Io { path, source } => Self::Io {
                path,
                error: source,
            },
        }
    }
}

impl From<zip::ZipError> for ThemePackManagerError {
    fn from(error: zip::ZipError) -> Self {
        match error {
            zip::ZipError::Io(error) => Self::Io { path: None, error },
            zip::ZipError::Malformed(reason) => Self::InvalidFormat(reason.to_string()),
        }
    }
}

/// A theme pack the launcher read, either from a file or from a directory.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedThemePack {
    /// The file or directory the theme pack was read from.
    pub file: PathBuf,
    /// The manifest the theme pack declares.
    pub manifest: ThemePackManifest,
}

/// Where an installed theme pack is stored.
#[derive(Debug, Clone, PartialEq)]
pub enum ThemePackLocation {
    /// A theme pack that comes with the launcher.
    Builtin,
    /// A theme pack the launcher copied into its own directory.
    Local(PathBuf),
    /// A theme pack the user keeps in their own directory.
    User(PathBuf),
}

impl ThemePackLocation {
    /// Returns the file or directory of the location, or `None` for a
    /// built-in pack.
    pub fn file(&self) -> Option<&Path> {
        match self {
            Self::Builtin => None,
            Self::Local(file) | Self::User(file) => Some(file.as_path()),
        }
    }
}

/// A theme pack that is installed in the launcher.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledThemePack {
    /// Where the theme pack is stored.
    pub location: ThemePackLocation,
    /// The manifest the theme pack declares.
    pub manifest: ThemePackManifest,
}

impl InstalledThemePack {
    /// Returns the file or directory of the theme pack, or `None` for a
    /// built-in pack.
    pub fn file(&self) -> Option<&Path> {
        self.location.file()
    }

    /// Returns whether the theme pack comes with the launcher.
    pub fn builtin(&self) -> bool {
        matches!(self.location, ThemePackLocation::Builtin)
    }
}

/// The theme packs the launcher installed.
pub struct ThemePackManager;

impl ThemePackManager {
    /// Reads a theme pack from a file or an unpacked directory.
    ///
    /// The manifest is read from `manifest.json`; a file that does not hold
    /// one is refused.
    pub fn load(file: &Path) -> Result<LoadedThemePack, ThemePackManagerError> {
        let file = file.to_path_buf();

        if file.is_dir() {
            let manifest_file = file.join(MANIFEST_ENTRY);
            if !manifest_file.is_file() {
                return Err(ThemePackManagerError::InvalidFormat(format!(
                    "Theme pack directory does not contain {MANIFEST_ENTRY}"
                )));
            }
            let manifest = read_manifest(&manifest_file)?;
            return Ok(LoadedThemePack { file, manifest });
        }

        let bytes = zip::read_entry(&file, MANIFEST_ENTRY)?.ok_or_else(|| {
            ThemePackManagerError::InvalidFormat(format!(
                "Theme pack does not contain {MANIFEST_ENTRY}"
            ))
        })?;
        let manifest = parse_manifest(&bytes)?;
        Ok(LoadedThemePack { file, manifest })
    }

    /// Returns the file a theme pack of the given identifier is installed to.
    ///
    /// The file is named after the identifier of the pack, so a pack that is
    /// installed again replaces the pack that was installed before.
    pub fn installed_theme_pack_file(directory: &Path, pack_id: &str) -> PathBuf {
        directory.join(format!("{pack_id}{FILE_EXTENSION}"))
    }

    /// Installs a theme pack into a directory.
    ///
    /// A pack that is installed again replaces the pack with the same
    /// identifier. The file is written next to the file it replaces, so an
    /// interrupted install leaves the pack that was there before.
    pub fn install(
        directory: &Path,
        loaded: &LoadedThemePack,
    ) -> Result<InstalledThemePack, ThemePackManagerError> {
        let target = Self::installed_theme_pack_file(directory, loaded.manifest.id.as_str());

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| ThemePackManagerError::Io {
                path: Some(parent.to_path_buf()),
                error,
            })?;
        }

        let temporary = {
            let mut name = target.file_name().unwrap_or_default().to_os_string();
            name.push(".part");
            target.with_file_name(name)
        };
        let _ = std::fs::remove_file(&temporary);
        std::fs::copy(&loaded.file, &temporary).map_err(|error| ThemePackManagerError::Io {
            path: Some(loaded.file.clone()),
            error,
        })?;
        std::fs::rename(&temporary, &target).map_err(|error| ThemePackManagerError::Io {
            path: Some(target.clone()),
            error,
        })?;

        Ok(InstalledThemePack {
            location: ThemePackLocation::Local(target),
            manifest: loaded.manifest.clone(),
        })
    }

    /// Removes a theme pack that was installed in a directory.
    ///
    /// A built-in pack is not removed, and a pack that is not stored in the
    /// directory of the launcher is not removed either, so a player cannot
    /// delete the files of the launcher by removing a pack.
    pub fn uninstall(
        directory: &Path,
        installed: &InstalledThemePack,
    ) -> Result<(), ThemePackManagerError> {
        if installed.builtin() {
            return Err(ThemePackManagerError::InvalidFormat(format!(
                "Cannot delete a built-in theme pack: {}",
                installed.manifest.id
            )));
        }

        let Some(file) = installed.file() else {
            return Err(ThemePackManagerError::InvalidFormat(format!(
                "Theme pack does not have a local file: {}",
                installed.manifest.id
            )));
        };

        if !file.starts_with(directory) {
            return Err(ThemePackManagerError::InvalidFormat(format!(
                "Theme-pack file is outside the managed directory: {}",
                file.display()
            )));
        }

        std::fs::remove_file(file).map_err(|error| ThemePackManagerError::Io {
            path: Some(file.to_path_buf()),
            error,
        })
    }

    /// Returns the theme packs of one directory.
    ///
    /// A pack that cannot be read is left out, so one broken pack does not
    /// hide the packs that were installed after it.
    pub fn list_installed(
        directory: &Path,
    ) -> Result<Vec<InstalledThemePack>, ThemePackManagerError> {
        if !directory.is_dir() {
            return Ok(Vec::new());
        }

        let mut theme_packs = Vec::new();
        let entries = std::fs::read_dir(directory).map_err(|error| ThemePackManagerError::Io {
            path: Some(directory.to_path_buf()),
            error,
        })?;

        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if !Self::is_installed_theme_pack_path(&path) {
                continue;
            }
            if let Ok(loaded) = Self::load(&path) {
                theme_packs.push(InstalledThemePack {
                    location: ThemePackLocation::Local(path),
                    manifest: loaded.manifest,
                });
            }
        }

        theme_packs.sort_by(|left, right| {
            let left_name = left.manifest.display_name(&[]);
            let right_name = right.manifest.display_name(&[]);
            left_name
                .to_lowercase()
                .cmp(&right_name.to_lowercase())
                .then_with(|| {
                    left.manifest
                        .id
                        .as_str()
                        .to_lowercase()
                        .cmp(&right.manifest.id.as_str().to_lowercase())
                })
        });

        Ok(theme_packs)
    }

    /// Returns the theme pack of one directory with the given identifier.
    ///
    /// The file that is named after the identifier is read first; a pack that
    /// was installed under another name is found by its manifest.
    pub fn find_installed(
        directory: &Path,
        pack_id: &str,
    ) -> Result<Option<InstalledThemePack>, ThemePackManagerError> {
        let file = Self::installed_theme_pack_file(directory, pack_id);
        if file.is_file() {
            if let Ok(loaded) = Self::load(&file) {
                if loaded.manifest.id.as_str() == pack_id {
                    return Ok(Some(InstalledThemePack {
                        location: ThemePackLocation::Local(file),
                        manifest: loaded.manifest,
                    }));
                }
            }
        }

        Self::find_installed_by_manifest_id(directory, pack_id, &file)
    }

    /// Returns the theme packs of two directories.
    ///
    /// A pack that is in both directories is shown once, with the pack of the
    /// first directory: the directory of the launcher hides the pack of the
    /// user with the same identifier. The two directories are the ones the
    /// launcher of Java reads, and the packs are shown with the name of the
    /// pack first.
    pub fn list_installed_merged(
        directory: &Path,
        user_directory: &Path,
    ) -> Result<Vec<InstalledThemePack>, ThemePackManagerError> {
        let mut theme_packs: Vec<InstalledThemePack> = Vec::new();
        let mut seen: Vec<String> = Vec::new();

        // The packs of the directory of the launcher hide the packs of the
        // directory of the user with the same identifier.
        for installed in Self::list_installed(directory)? {
            seen.push(installed.manifest.id.as_str().to_string());
            theme_packs.push(installed);
        }

        for installed in Self::list_installed(user_directory)? {
            if seen.iter().any(|id| id == installed.manifest.id.as_str()) {
                continue;
            }
            theme_packs.push(installed);
        }

        theme_packs.sort_by(|left, right| {
            let left_name = left.manifest.display_name(&[]);
            let right_name = right.manifest.display_name(&[]);
            left_name
                .to_lowercase()
                .cmp(&right_name.to_lowercase())
                .then_with(|| {
                    left.manifest
                        .id
                        .as_str()
                        .to_lowercase()
                        .cmp(&right.manifest.id.as_str().to_lowercase())
                })
        });

        Ok(theme_packs)
    }

    /// Returns whether a path is a theme pack the launcher installed.
    ///
    /// A file is one when its name does not begin with a dot and ends with
    /// the extension of a theme pack; a directory is one when it holds a
    /// manifest and its name does not begin with a dot.
    pub fn is_installed_theme_pack_path(path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };
        if name.starts_with('.') {
            return false;
        }

        if path.is_dir() {
            return path.join(MANIFEST_ENTRY).is_file();
        }
        name.ends_with(FILE_EXTENSION)
    }

    /// Reads a theme pack from the manifests of a directory.
    ///
    /// A path that was read already is left out, so the pack the file of the
    /// identifier points to wins.
    fn find_installed_by_manifest_id(
        directory: &Path,
        pack_id: &str,
        excluded: &Path,
    ) -> Result<Option<InstalledThemePack>, ThemePackManagerError> {
        if !directory.is_dir() {
            return Ok(None);
        }

        let mut paths: Vec<PathBuf> = Vec::new();
        let entries = std::fs::read_dir(directory).map_err(|error| ThemePackManagerError::Io {
            path: Some(directory.to_path_buf()),
            error,
        })?;

        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if path == excluded || !Self::is_installed_theme_pack_path(&path) {
                continue;
            }
            paths.push(path);
        }

        paths.sort_by_key(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_lowercase())
                .unwrap_or_default()
        });

        for path in paths {
            if let Ok(loaded) = Self::load(&path) {
                if loaded.manifest.id.as_str() == pack_id {
                    return Ok(Some(InstalledThemePack {
                        location: ThemePackLocation::Local(path),
                        manifest: loaded.manifest,
                    }));
                }
            }
        }

        Ok(None)
    }
}

/// Reads a manifest from the file system.
fn read_manifest(file: &Path) -> Result<ThemePackManifest, ThemePackManagerError> {
    let bytes = std::fs::read(file).map_err(|error| ThemePackManagerError::Io {
        path: Some(file.to_path_buf()),
        error,
    })?;
    parse_manifest(&bytes)
}

/// Parses a manifest that was read from a file or from an archive.
fn parse_manifest(bytes: &[u8]) -> Result<ThemePackManifest, ThemePackManagerError> {
    let manifest: ThemePackManifest = serde_json::from_slice(bytes)
        .map_err(|error| ThemePackManagerError::InvalidFormat(error.to_string()))?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::LocalizedText;
    use crate::theme::theme_pack::{Theme, ThemePackAsset, ThemePackExporter};
    use serde_json::json;

    fn manifest(id: &str, name: &str) -> ThemePackManifest {
        ThemePackManifest::new(
            id,
            "1.0.0",
            LocalizedText::plain(name),
            Vec::new(),
            None,
            None,
            vec![Theme::new(
                None,
                Some(LocalizedText::plain("Default")),
                Vec::new(),
                None,
                None,
                json!({}),
                Vec::new(),
            )
            .expect("the theme should be created")],
        )
        .expect("the manifest should be created")
    }

    fn write_pack(directory: &Path, pack_id: &str, name: &str) -> PathBuf {
        let source = directory.join(format!("source-{pack_id}{FILE_EXTENSION}"));
        ThemePackExporter::export(
            &manifest(pack_id, name),
            &[
                ThemePackAsset::from_content(b"PNG".to_vec(), "assets/icon.png")
                    .expect("the asset should be created"),
            ],
            &source,
        )
        .expect("the theme pack should be written");
        source
    }

    /// Returns a directory that only the test that asks for it uses.
    ///
    /// The tests of this module run in the same process and the same moment,
    /// so the name of the directory holds a number that is different for
    /// every test: one test cannot remove the files of another test.
    fn directory() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let directory = std::env::temp_dir().join(format!(
            "hmcl-theme-pack-manager-test-{}-{counter}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");
        directory
    }

    #[test]
    fn a_theme_pack_is_installed_and_read_back() {
        let directory = directory();
        let source = write_pack(&directory, "com.example.theme", "Example");

        let loaded = ThemePackManager::load(&source).expect("the pack should be loaded");
        let installed =
            ThemePackManager::install(&directory, &loaded).expect("the pack should be installed");

        assert_eq!(installed.manifest.id.as_str(), "com.example.theme");
        let target = ThemePackManager::installed_theme_pack_file(&directory, "com.example.theme");
        assert!(target.is_file());
        assert!(!installed.builtin());
        assert_eq!(installed.file(), Some(target.as_path()));

        let read = ThemePackManager::load(&target).expect("the pack should be read");
        assert_eq!(read.manifest.id.as_str(), "com.example.theme");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_theme_pack_that_is_installed_again_is_replaced() {
        let directory = directory();
        let source = write_pack(&directory, "com.example.theme", "Example");

        let first = ThemePackManager::install(
            &directory,
            &ThemePackManager::load(&source).expect("the pack should be loaded"),
        )
        .expect("the first install should work");

        // A pack with another display name and the same identifier is written.
        let second_source = write_pack(&directory, "com.example.theme", "Renamed");
        let second = ThemePackManager::install(
            &directory,
            &ThemePackManager::load(&second_source).expect("the pack should be loaded"),
        )
        .expect("the second install should work");

        assert_eq!(first.file(), second.file());
        let second_file = second.file().expect("the pack should have a file");
        let read = ThemePackManager::load(second_file).expect("the pack should be read");
        assert_eq!(read.manifest.display_name(&[]), "Renamed");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_theme_packs_of_a_directory_are_listed_in_order() {
        let directory = directory();
        let themes = directory.join(THEME_PACKS_DIRECTORY);
        std::fs::create_dir_all(&themes).expect("the directory should be created");

        for (id, name) in [
            ("com.example.zeta", "Zeta"),
            ("com.example.alpha", "Alpha"),
            ("com.example.middle", "Middle"),
        ] {
            let source = write_pack(&directory, id, name);
            ThemePackManager::install(
                &themes,
                &ThemePackManager::load(&source).expect("the pack should be loaded"),
            )
            .expect("the pack should be installed");
        }

        // A hidden file is not a theme pack.
        std::fs::write(themes.join(".hidden"), b"").expect("the file should be written");

        let installed =
            ThemePackManager::list_installed(&themes).expect("the packs should be listed");
        assert_eq!(installed.len(), 3);
        assert_eq!(installed[0].manifest.display_name(&[]), "Alpha");
        assert_eq!(installed[1].manifest.display_name(&[]), "Middle");
        assert_eq!(installed[2].manifest.display_name(&[]), "Zeta");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_theme_pack_is_found_by_its_identifier() {
        let directory = directory();
        let themes = directory.join(THEME_PACKS_DIRECTORY);
        std::fs::create_dir_all(&themes).expect("the directory should be created");

        let source = write_pack(&directory, "com.example.theme", "Example");
        ThemePackManager::install(
            &themes,
            &ThemePackManager::load(&source).expect("the pack should be loaded"),
        )
        .expect("the pack should be installed");

        let found = ThemePackManager::find_installed(&themes, "com.example.theme")
            .expect("the pack should be found")
            .expect("the pack should exist");
        assert_eq!(found.manifest.id.as_str(), "com.example.theme");

        let missing = ThemePackManager::find_installed(&themes, "com.example.missing")
            .expect("the search should work");
        assert!(missing.is_none());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_theme_pack_of_two_directories_is_listed_once() {
        let directory = directory();
        let local = directory.join(THEME_PACKS_DIRECTORY);
        let user = directory.join("user").join(THEME_PACKS_DIRECTORY);
        std::fs::create_dir_all(&local).expect("the directory should be created");
        std::fs::create_dir_all(&user).expect("the directory should be created");

        let shared = write_pack(&directory, "com.example.shared", "Shared");
        ThemePackManager::install(
            &local,
            &ThemePackManager::load(&shared).expect("the pack should be loaded"),
        )
        .expect("the pack should be installed");
        ThemePackManager::install(
            &user,
            &ThemePackManager::load(&shared).expect("the pack should be loaded"),
        )
        .expect("the pack should be installed");

        let other = write_pack(&directory, "com.example.other", "Other");
        ThemePackManager::install(
            &user,
            &ThemePackManager::load(&other).expect("the pack should be loaded"),
        )
        .expect("the pack should be installed");

        let merged = ThemePackManager::list_installed_merged(&local, &user)
            .expect("the packs should be listed");
        assert_eq!(merged.len(), 2);

        // The pack of the directory of the launcher hides the pack of the
        // directory of the user with the same identifier.
        let shared_pack = merged
            .iter()
            .find(|pack| pack.manifest.id.as_str() == "com.example.shared")
            .expect("the shared pack should be listed");
        assert!(shared_pack.file().unwrap().starts_with(&local));
        assert!(merged
            .iter()
            .any(|pack| pack.manifest.id.as_str() == "com.example.other"));

        // The packs are shown with the name of the pack first.
        assert_eq!(merged[0].manifest.display_name(&[]), "Other");
        assert_eq!(merged[1].manifest.display_name(&[]), "Shared");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_theme_pack_that_is_installed_can_be_removed() {
        let directory = directory();
        let themes = directory.join(THEME_PACKS_DIRECTORY);
        std::fs::create_dir_all(&themes).expect("the directory should be created");

        let source = write_pack(&directory, "com.example.theme", "Example");
        let installed = ThemePackManager::install(
            &themes,
            &ThemePackManager::load(&source).expect("the pack should be loaded"),
        )
        .expect("the pack should be installed");

        ThemePackManager::uninstall(&themes, &installed).expect("the pack should be removed");
        assert!(!installed.file().unwrap().exists());
        assert!(ThemePackManager::list_installed(&themes)
            .expect("the packs should be listed")
            .is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_built_in_theme_pack_is_not_removed() {
        let directory = directory();
        let installed = InstalledThemePack {
            location: ThemePackLocation::Builtin,
            manifest: manifest("com.example.builtin", "Builtin"),
        };

        let error = ThemePackManager::uninstall(&directory, &installed)
            .expect_err("the built-in pack should be refused");
        assert!(matches!(error, ThemePackManagerError::InvalidFormat(_)));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_theme_pack_outside_the_directory_is_not_removed() {
        let directory = directory();
        let themes = directory.join(THEME_PACKS_DIRECTORY);
        std::fs::create_dir_all(&themes).expect("the directory should be created");

        let outside = write_pack(&directory, "com.example.outside", "Outside");
        let installed = InstalledThemePack {
            location: ThemePackLocation::Local(outside.clone()),
            manifest: manifest("com.example.outside", "Outside"),
        };

        let error = ThemePackManager::uninstall(&themes, &installed)
            .expect_err("the pack outside the directory should be refused");
        assert!(matches!(error, ThemePackManagerError::InvalidFormat(_)));
        assert!(outside.is_file());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_theme_pack_without_a_manifest_is_refused() {
        let directory = directory();
        let broken = directory.join(format!("broken{FILE_EXTENSION}"));
        std::fs::write(&broken, b"not a zip").expect("the file should be written");

        let error = ThemePackManager::load(&broken).expect_err("the broken pack should be refused");
        assert!(matches!(
            error,
            ThemePackManagerError::Io { .. } | ThemePackManagerError::InvalidFormat(_)
        ));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_directory_with_a_manifest_is_a_theme_pack() {
        let directory = directory();
        let unpacked = directory.join("unpacked");
        std::fs::create_dir_all(&unpacked).expect("the directory should be created");

        let json = serde_json::to_string(&manifest("com.example.unpacked", "Unpacked"))
            .expect("the manifest should be written");
        std::fs::write(unpacked.join(MANIFEST_ENTRY), json)
            .expect("the manifest should be written");

        assert!(ThemePackManager::is_installed_theme_pack_path(&unpacked));

        let loaded = ThemePackManager::load(&unpacked).expect("the pack should be loaded");
        assert_eq!(loaded.manifest.id.as_str(), "com.example.unpacked");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_hidden_file_is_not_a_theme_pack() {
        let directory = directory();
        let hidden = directory.join(format!(".hidden{FILE_EXTENSION}"));
        std::fs::write(&hidden, b"").expect("the file should be written");
        assert!(!ThemePackManager::is_installed_theme_pack_path(&hidden));

        let _ = std::fs::remove_dir_all(&directory);
    }
}
