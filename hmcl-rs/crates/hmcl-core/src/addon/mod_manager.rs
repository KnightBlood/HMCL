//! The mods a player has installed in a game directory.
//!
//! The launcher lists the files of `mods` and reads the metadata of every file
//! that may be a mod. A mod is disabled by appending `.disabled` to its name
//! and kept as a backup of an older version by appending `.old`, so the game
//! only loads the files that carry neither suffix.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::addon::local_mod::{display_name, ModFile, DISABLED_EXTENSION, OLD_EXTENSION};
use crate::addon::mod_loader::ModLoaderType;
use crate::game::component::{ComponentAnalyzer, GameComponentType};

/// The failure of managing the mods of a game directory.
#[derive(Debug)]
pub enum ModError {
    /// The mods directory cannot be read or a file cannot be moved.
    Io {
        /// The path the operation was about.
        path: PathBuf,
        /// The reason the operation failed.
        error: std::io::Error,
    },
    /// The file is not a mod, so it is not added to the directory of the mods.
    NotAMod {
        /// The file that is not a mod.
        path: PathBuf,
    },
    /// The mod is not one of the mods the launcher read.
    UnknownMod {
        /// The file of the mod.
        path: PathBuf,
    },
    /// A mod cannot be rolled back to a backup.
    RollbackRefused {
        /// The reason the rollback was refused.
        reason: String,
    },
}

impl std::fmt::Display for ModError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, error } => {
                write!(formatter, "{} cannot be used: {error}", path.display())
            }
            Self::NotAMod { path } => {
                write!(formatter, "{} is not a mod file", path.display())
            }
            Self::UnknownMod { path } => {
                write!(
                    formatter,
                    "{} is not a mod the launcher read",
                    path.display()
                )
            }
            Self::RollbackRefused { reason } => {
                write!(formatter, "the mod cannot be rolled back: {reason}")
            }
        }
    }
}

impl std::error::Error for ModError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// The mods of a game directory.
#[derive(Debug, Clone)]
pub struct ModManager {
    /// The directory the mods are kept in.
    directory: PathBuf,
    /// The loaders the game runs, which are read first.
    game_loaders: Vec<ModLoaderType>,
    /// The version of the game, which decides whether NeoForge loads the mods
    /// of Forge.
    game_version: String,
    /// The mods that were read.
    files: Vec<ModFile>,
    /// The loaders the game directory can load mods for.
    supported_loaders: BTreeSet<ModLoaderType>,
    /// Whether the mods were read.
    loaded: bool,
}

impl ModManager {
    /// Creates a manager of the mods of a directory.
    ///
    /// The manager does not read the directory until the mods are asked for,
    /// because the directory may not exist yet.
    pub fn new(
        directory: impl Into<PathBuf>,
        game_loaders: Vec<ModLoaderType>,
        game_version: impl Into<String>,
    ) -> Self {
        Self {
            directory: directory.into(),
            game_loaders,
            game_version: game_version.into(),
            files: Vec::new(),
            supported_loaders: BTreeSet::new(),
            loaded: false,
        }
    }

    /// Returns the directory the mods are kept in.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Returns the version of the game the mods are read for.
    pub fn game_version(&self) -> &str {
        &self.game_version
    }

    /// Returns the loaders the game directory can load mods for.
    pub fn supported_loaders(&self) -> &BTreeSet<ModLoaderType> {
        &self.supported_loaders
    }

    /// Returns whether the directory can load mods for a loader.
    pub fn supports(&self, loader: ModLoaderType) -> bool {
        self.supported_loaders.contains(&loader)
    }

    /// Returns the mods of the directory, sorted by their file names.
    ///
    /// The directory is read when it has not been read yet.
    pub fn files(&mut self) -> Result<&[ModFile], ModError> {
        if !self.loaded {
            self.refresh()?;
        }
        Ok(&self.files)
    }

    /// Returns the mods of the directory, sorted by their file names.
    ///
    /// The directory is always read again, so a file the player added or
    /// removed is seen.
    pub fn refresh(&mut self) -> Result<(), ModError> {
        self.files.clear();

        if self.directory.is_dir() {
            let can_use_subfolders = self.game_loaders.iter().any(|loader| {
                matches!(
                    loader,
                    ModLoaderType::Forge
                        | ModLoaderType::Quilt
                        | ModLoaderType::Cleanroom
                        | ModLoaderType::LiteLoader
                )
            });

            let entries = std::fs::read_dir(&self.directory).map_err(|error| ModError::Io {
                path: self.directory.clone(),
                error,
            })?;

            for entry in entries {
                let entry = entry.map_err(|error| ModError::Io {
                    path: self.directory.clone(),
                    error,
                })?;
                let path = entry.path();

                // Forge, Quilt, Cleanroom and LiteLoader read the mods that are
                // kept in a directory under `mods`; the other loaders read the
                // files of `mods` alone.
                if can_use_subfolders
                    && path.is_dir()
                    && !path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.eq_ignore_ascii_case(".connector"))
                {
                    let sub_entries = std::fs::read_dir(&path).map_err(|error| ModError::Io {
                        path: path.clone(),
                        error,
                    })?;
                    for sub_entry in sub_entries {
                        let sub_entry = sub_entry.map_err(|error| ModError::Io {
                            path: path.clone(),
                            error,
                        })?;
                        self.read_mod(sub_entry.path());
                    }
                } else {
                    self.read_mod(path);
                }
            }
        }

        self.supported_loaders = self.compute_supported_loaders();

        // The old mods are backups, so they are read but not loaded; the list
        // is sorted by the file name of the mods, like the Java launcher does.
        self.files.sort_by(|left, right| {
            left.file_name()
                .to_lowercase()
                .cmp(&right.file_name().to_lowercase())
        });

        self.loaded = true;
        Ok(())
    }

    /// Reads a file of the directory of the mods, if it is a mod.
    fn read_mod(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let local_name = display_name(name);

        let extension = local_name
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase());
        let is_mod_file = match &extension {
            Some(extension) => matches!(extension.as_str(), "jar" | "zip" | "litemod"),
            None => false,
        };
        if !is_mod_file {
            return;
        }

        self.files.push(ModFile::open(&path, &self.game_loaders));
    }

    /// Returns the loaders the game directory can load mods for.
    ///
    /// The loader of the game is the one that is used, and the variants of it
    /// that run the mods of the other loaders are added like the Java launcher
    /// adds them.
    fn compute_supported_loaders(&self) -> BTreeSet<ModLoaderType> {
        let mut loaders = BTreeSet::new();

        if self.game_loaders.is_empty() {
            // A directory whose loader the launcher cannot tell is read as a
            // directory that loads every kind of mod.
            loaders.extend(ModLoaderType::ALL);
            loaders.remove(&ModLoaderType::Unknown);
            return loaders;
        }

        for loader in &self.game_loaders {
            loaders.insert(*loader);

            // Cleanroom runs the mods of Forge.
            if *loader == ModLoaderType::Cleanroom {
                loaders.insert(ModLoaderType::Forge);
            }
        }

        // NeoForge 1.20.1 forks Forge and runs the mods of Forge.
        if self.game_loaders.contains(&ModLoaderType::NeoForge) && self.game_version == "1.20.1" {
            loaders.insert(ModLoaderType::Forge);
        }

        // Quilt runs the mods of Fabric, and so does Legacy Fabric.
        if self.game_loaders.contains(&ModLoaderType::Quilt)
            || self.game_loaders.contains(&ModLoaderType::LegacyFabric)
        {
            loaders.insert(ModLoaderType::Fabric);
        }

        // Kilt runs the mods of Forge and NeoForge on Fabric.
        if self.game_loaders.contains(&ModLoaderType::Fabric)
            && self.has_mod("kilt", ModLoaderType::Fabric)
        {
            loaders.insert(ModLoaderType::Forge);
            loaders.insert(ModLoaderType::NeoForge);
        }

        // Sinytra Connector runs the mods of Fabric on NeoForge and Forge.
        let connector_runs_fabric = self.game_loaders.contains(&ModLoaderType::NeoForge)
            && (self.has_mod("connector", ModLoaderType::NeoForge)
                || self.has_mod("connectormod", ModLoaderType::NeoForge));
        let connector_runs_fabric_on_forge = self.game_version == "1.20.1"
            && self.game_loaders.contains(&ModLoaderType::Forge)
            && self.has_mod("connectormod", ModLoaderType::Forge);
        if connector_runs_fabric || connector_runs_fabric_on_forge {
            loaders.insert(ModLoaderType::Fabric);
        }

        loaders
    }

    /// Returns whether a mod of a loader is among the mods that were read.
    pub fn has_mod(&self, id: &str, loader: ModLoaderType) -> bool {
        self.files
            .iter()
            .any(|file| file.id() == id && file.loader() == loader)
    }

    /// Returns whether a file of the directory exists in any of its states.
    ///
    /// The file is looked for as it is named, as a disabled mod and as a
    /// backup, so a mod that is installed but not loaded is still found.
    pub fn has_simple_mod(&self, file_name: &str) -> bool {
        let enabled = display_name(file_name);
        let disabled = format!("{enabled}{DISABLED_EXTENSION}");
        let old = format!("{enabled}{OLD_EXTENSION}");

        [enabled, disabled, old]
            .iter()
            .any(|name| self.directory.join(name).exists())
    }

    /// Returns the path a file of the directory would have.
    pub fn simple_mod_path(&self, file_name: &str) -> PathBuf {
        self.directory.join(file_name)
    }

    /// Adds a file to the directory of the mods.
    ///
    /// The file is copied, not moved, so the file the player chose is kept.
    pub fn add_mod(&mut self, file: &Path) -> Result<&ModFile, ModError> {
        if !ModFile::is_file_name_mod(file) {
            return Err(ModError::NotAMod {
                path: file.to_path_buf(),
            });
        }

        if !self.loaded {
            self.refresh()?;
        }

        std::fs::create_dir_all(&self.directory).map_err(|error| ModError::Io {
            path: self.directory.clone(),
            error,
        })?;

        let file_name = file
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default();
        let target = self.directory.join(&file_name);

        std::fs::copy(file, &target).map_err(|error| ModError::Io {
            path: target.clone(),
            error,
        })?;

        self.read_mod(target);
        self.files.sort_by(|left, right| {
            left.file_name()
                .to_lowercase()
                .cmp(&right.file_name().to_lowercase())
        });

        let index = self
            .files
            .iter()
            .position(|mod_file| {
                mod_file
                    .path()
                    .file_name()
                    .map(|name| name == file_name.as_os_str())
                    .unwrap_or(false)
            })
            .ok_or_else(|| ModError::UnknownMod {
                path: self.directory.join(&file_name),
            })?;

        Ok(&self.files[index])
    }

    /// Returns whether a file of the directory is a disabled mod.
    pub fn is_disabled(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(DISABLED_EXTENSION))
    }

    /// Returns whether a file of the directory is a backup of an older version.
    pub fn is_old(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(OLD_EXTENSION))
    }

    /// Disables a mod and returns the path of the disabled file.
    ///
    /// A mod that is a backup of an older version is not disabled, because it
    /// is not loaded anyway.
    pub fn disable_mod(&mut self, path: &Path) -> Result<PathBuf, ModError> {
        if Self::is_old(path) || Self::is_disabled(path) {
            return Ok(path.to_path_buf());
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let disabled = path.with_file_name(format!("{file_name}{DISABLED_EXTENSION}"));

        move_file(path, &disabled)?;
        self.rename_entry(path, &disabled);

        Ok(disabled)
    }

    /// Enables a mod and returns the path of the enabled file.
    pub fn enable_mod(&mut self, path: &Path) -> Result<PathBuf, ModError> {
        if Self::is_old(path) {
            return Ok(path.to_path_buf());
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let enabled = path.with_file_name(display_name(file_name));

        move_file(path, &enabled)?;
        self.rename_entry(path, &enabled);

        Ok(enabled)
    }

    /// Marks a mod as a backup of an older version, or restores it.
    ///
    /// A mod that is disabled is enabled as well when it is marked as a backup,
    /// so a backup is never a disabled file.
    pub fn set_old(&mut self, path: &Path, old: bool) -> Result<PathBuf, ModError> {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| path.to_str().unwrap_or_default());

        let new_path = if old {
            let enabled = display_name(file_name);
            path.with_file_name(format!("{enabled}{OLD_EXTENSION}"))
        } else {
            let enabled = display_name(file_name);
            path.with_file_name(enabled)
        };

        move_file(path, &new_path)?;

        // The state of the mod follows the new name of the file.
        let is_old = Self::is_old(&new_path);
        let is_disabled = Self::is_disabled(&new_path);
        if let Some(file) = self.files.iter_mut().find(|file| file.path() == path) {
            file.set_state(&new_path, is_old, is_disabled);
        }

        Ok(new_path)
    }

    /// Deletes the file of a mod.
    pub fn delete_mod(&mut self, path: &Path) -> Result<(), ModError> {
        std::fs::remove_file(path).map_err(|error| ModError::Io {
            path: path.to_path_buf(),
            error,
        })?;

        self.files.retain(|file| file.path() != path);
        Ok(())
    }

    /// Rolls a mod back to a backup of an older version.
    ///
    /// The mod is made a backup and the backup takes its place, so the version
    /// the player had is the one the game loads again. The backup has to be a
    /// backup of the same mod, and the two files must not have the same name.
    pub fn rollback(&mut self, from: &Path, to: &Path) -> Result<(), ModError> {
        if !self.files.iter().any(|file| file.path() == from) {
            return Err(ModError::UnknownMod {
                path: from.to_path_buf(),
            });
        }
        if Self::is_old(from) {
            return Err(ModError::RollbackRefused {
                reason: format!("{} is already a backup", from.display()),
            });
        }
        if !Self::is_old(to) {
            return Err(ModError::RollbackRefused {
                reason: format!("{} is not a backup", to.display()),
            });
        }

        let from_name = from
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let to_name = to
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if display_name(from_name) == display_name(to_name) {
            // The two files would have the same name once the suffixes are
            // left out, so the rollback cannot be done.
            return Ok(());
        }

        let from_id = self
            .files
            .iter()
            .find(|file| file.path() == from)
            .map(|file| (file.id().to_string(), file.loader()));
        let to_id = self
            .files
            .iter()
            .find(|file| file.path() == to)
            .map(|file| (file.id().to_string(), file.loader()));
        if from_id != to_id {
            return Err(ModError::RollbackRefused {
                reason: "the two files are mods of different versions".to_string(),
            });
        }

        // The current mod is kept as the backup, and the backup is loaded.
        self.set_old(from, true)?;
        self.set_old(to, false)?;

        Ok(())
    }

    /// Makes the path of a mod follow the new path of its file.
    fn rename_entry(&mut self, from: &Path, to: &Path) {
        let is_old = Self::is_old(to);
        let is_disabled = Self::is_disabled(to);

        if let Some(file) = self.files.iter_mut().find(|file| file.path() == from) {
            file.set_state(to, is_old, is_disabled);
        }
    }
}

/// Moves a file, replacing the file that is already at the destination.
fn move_file(from: &Path, to: &Path) -> Result<(), ModError> {
    if !from.exists() {
        return Ok(());
    }

    // A file cannot be moved over a file that exists on some file systems, so
    // the destination is removed first.
    if to.exists() && to != from {
        std::fs::remove_file(to).map_err(|error| ModError::Io {
            path: to.to_path_buf(),
            error,
        })?;
    }

    if to == from {
        return Ok(());
    }

    std::fs::rename(from, to).map_err(|error| ModError::Io {
        path: from.to_path_buf(),
        error,
    })
}

/// Returns the loaders of a component analyzer.
///
/// The mods of a game directory are read as the mods of the loaders the version
/// names, so the component analyzer decides which readers are tried first.
pub fn loaders_of(analyzer: &ComponentAnalyzer) -> Vec<ModLoaderType> {
    let mut loaders = Vec::new();

    for component in GameComponentType::ALL {
        if !component.is_mod_loader() || !analyzer.has(component) {
            continue;
        }

        let loader = match component {
            GameComponentType::LegacyFabric => ModLoaderType::LegacyFabric,
            GameComponentType::Fabric => ModLoaderType::Fabric,
            GameComponentType::Forge => ModLoaderType::Forge,
            GameComponentType::Cleanroom => ModLoaderType::Cleanroom,
            GameComponentType::NeoForge => ModLoaderType::NeoForge,
            GameComponentType::LiteLoader => ModLoaderType::LiteLoader,
            GameComponentType::Quilt => ModLoaderType::Quilt,
            _ => continue,
        };

        loaders.push(loader);
    }

    loaders
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-mods-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    /// Returns the manager of a directory of mods.
    fn manager(directory: &Path, loaders: Vec<ModLoaderType>) -> ModManager {
        ModManager::new(directory.join("mods"), loaders, "1.20.1")
    }

    #[test]
    fn the_mods_of_the_directory_are_read() {
        let base = directory("read");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("the directory should be made");
        std::fs::write(mods.join("a.jar"), b"not an archive").expect("the file should be written");
        std::fs::write(mods.join("b.jar.disabled"), b"").expect("the file should be written");
        std::fs::write(mods.join("notes.txt"), b"").expect("the file should be written");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        let files = manager.files().expect("the mods should be read");

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].file_name(), "a");
        assert!(files[0].is_active());
        assert!(!files[0].has_metadata());
        // The mod whose metadata cannot be read is shown with its file name.
        assert_eq!(files[0].name(), "a");
        assert_eq!(files[1].file_name(), "b");
        assert!(files[1].is_disabled());
        assert!(!files[1].is_active());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_missing_directory_holds_no_mod() {
        let base = directory("missing");
        let mut manager = manager(&base, vec![ModLoaderType::Forge]);
        assert!(manager.files().expect("the mods should be read").is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_mods_of_the_subfolders_are_read_when_the_loader_reads_them() {
        let base = directory("subfolders");
        let mods = base.join("mods");
        std::fs::create_dir_all(mods.join("1.20.1")).expect("the directory should be made");
        std::fs::write(mods.join("1.20.1").join("inner.jar"), b"")
            .expect("the file should be written");

        let mut forge = manager(&base, vec![ModLoaderType::Forge]);
        assert_eq!(forge.files().expect("the mods should be read").len(), 1);

        let mut fabric = manager(&base, vec![ModLoaderType::Fabric]);
        assert!(fabric.files().expect("the mods should be read").is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_connector_directory_is_not_read() {
        let base = directory("connector-dir");
        let mods = base.join("mods");
        std::fs::create_dir_all(mods.join(".connector")).expect("the directory should be made");
        std::fs::write(mods.join(".connector").join("inner.jar"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Forge]);
        assert!(manager.files().expect("the mods should be read").is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mod_is_disabled_and_enabled_again() {
        let base = directory("toggle");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        std::fs::write(mods.join("sodium.jar"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        manager.files().expect("the mods should be read");

        let disabled = manager
            .disable_mod(&mods.join("sodium.jar"))
            .expect("the mod should be disabled");
        assert_eq!(disabled, mods.join("sodium.jar.disabled"));
        assert!(disabled.exists());
        assert!(!mods.join("sodium.jar").exists());
        assert!(manager.files[0].is_disabled());
        assert_eq!(manager.files[0].path(), disabled);

        let enabled = manager
            .enable_mod(&disabled)
            .expect("the mod should be enabled");
        assert_eq!(enabled, mods.join("sodium.jar"));
        assert!(enabled.exists());
        assert!(!manager.files[0].is_disabled());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_disabled_mod_is_not_disabled_twice() {
        let base = directory("disable-twice");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        std::fs::write(mods.join("a.jar.disabled"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        let path = mods.join("a.jar.disabled");
        assert_eq!(
            manager
                .disable_mod(&path)
                .expect("the mod should be left as it is"),
            path
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mod_is_marked_as_an_old_version_and_restored() {
        let base = directory("old");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        std::fs::write(mods.join("a.jar"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        manager.files().expect("the mods should be read");

        let old = manager
            .set_old(&mods.join("a.jar"), true)
            .expect("the mod should be marked old");
        assert_eq!(old, mods.join("a.jar.old"));
        assert!(old.exists());
        assert!(manager.files[0].is_old());
        assert!(!manager.files[0].is_active());

        let restored = manager
            .set_old(&old, false)
            .expect("the mod should be restored");
        assert_eq!(restored, mods.join("a.jar"));
        assert!(!manager.files[0].is_old());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_disabled_mod_is_enabled_when_it_is_marked_as_old() {
        let base = directory("old-disabled");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        std::fs::write(mods.join("a.jar.disabled"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        manager.files().expect("the mods should be read");

        let old = manager
            .set_old(&mods.join("a.jar.disabled"), true)
            .expect("the mod should be marked old");
        assert_eq!(old, mods.join("a.jar.old"));
        assert!(!ModManager::is_disabled(&old));
        assert!(old.exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_old_mods_are_kept_in_the_read_mods() {
        let base = directory("old-read");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        std::fs::write(mods.join("a.jar.old"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        let files = manager.files().expect("the mods should be read");
        assert_eq!(files.len(), 1);
        assert!(files[0].is_old());
        assert!(!files[0].is_active());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mod_is_deleted() {
        let base = directory("delete");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        std::fs::write(mods.join("a.jar"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        manager.files().expect("the mods should be read");

        manager
            .delete_mod(&mods.join("a.jar"))
            .expect("the mod should be deleted");
        assert!(!mods.join("a.jar").exists());
        assert!(manager.files.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_that_is_not_a_mod_is_refused() {
        let base = directory("not-a-mod");
        std::fs::create_dir_all(base.join("mods")).expect("dir");
        std::fs::write(base.join("notes.txt"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        let error = manager
            .add_mod(&base.join("notes.txt"))
            .expect_err("the file should be refused");
        assert!(matches!(error, ModError::NotAMod { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mod_is_added_to_the_directory() {
        let base = directory("add");
        std::fs::create_dir_all(base.join("mods")).expect("dir");
        std::fs::write(base.join("sodium.jar"), b"data").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        let added = manager
            .add_mod(&base.join("sodium.jar"))
            .expect("the mod should be added");

        assert_eq!(added.file_name(), "sodium");
        assert_eq!(added.path(), base.join("mods").join("sodium.jar"));
        // The file the player chose is kept.
        assert!(base.join("sodium.jar").exists());
        assert!(base.join("mods").join("sodium.jar").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mod_is_found_whatever_state_it_is_in() {
        let base = directory("simple");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        std::fs::write(mods.join("a.jar.old"), b"").expect("write");

        let manager = manager(&base, vec![ModLoaderType::Fabric]);
        assert!(manager.has_simple_mod("a.jar"));
        assert!(manager.has_simple_mod("a.jar.disabled"));
        assert!(!manager.has_simple_mod("b.jar"));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Returns the bytes of an archive that stores the given files.
    ///
    /// The archive is written without compression, which the reader of the
    /// launcher reads as well as the archives the game ships.
    fn archive(files: &[(&str, &str)]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut directory = Vec::new();

        for (name, content) in files {
            let offset = data.len() as u32;
            let name = name.as_bytes();
            let content = content.as_bytes();

            data.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
            data.extend_from_slice(&20u16.to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&(content.len() as u32).to_le_bytes());
            data.extend_from_slice(&(content.len() as u32).to_le_bytes());
            data.extend_from_slice(&(name.len() as u16).to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(name);
            data.extend_from_slice(content);

            directory.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u32.to_le_bytes());
            directory.extend_from_slice(&(content.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(content.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u32.to_le_bytes());
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(name);
        }

        let directory_offset = data.len() as u32;
        let directory_size = directory.len() as u32;
        data.extend_from_slice(&directory);
        data.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&(files.len() as u16).to_le_bytes());
        data.extend_from_slice(&(files.len() as u16).to_le_bytes());
        data.extend_from_slice(&directory_size.to_le_bytes());
        data.extend_from_slice(&directory_offset.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data
    }

    /// Writes a Fabric mod that names itself by an identifier and a version.
    fn write_fabric_mod(path: &Path, id: &str, version: &str) {
        let metadata = format!(r#"{{ "id": "{id}", "name": "{id}", "version": "{version}" }}"#);
        std::fs::write(path, archive(&[("fabric.mod.json", &metadata)]))
            .expect("the mod should be written");
    }

    #[test]
    fn the_metadata_of_a_mod_file_is_read() {
        let base = directory("metadata");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        write_fabric_mod(&mods.join("sodium.jar"), "sodium", "0.5.8");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        let files = manager.files().expect("the mods should be read");

        assert_eq!(files.len(), 1);
        assert!(files[0].has_metadata());
        assert_eq!(files[0].id(), "sodium");
        assert_eq!(files[0].name(), "sodium");
        assert_eq!(files[0].version(), "0.5.8");
        assert_eq!(files[0].loader(), ModLoaderType::Fabric);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_that_holds_the_metadata_of_a_mod_is_a_mod() {
        let base = directory("is-file-mod");
        write_fabric_mod(&base.join("sodium.jar"), "sodium", "1");

        assert!(ModFile::is_file_mod(&base.join("sodium.jar")));
        assert!(!ModFile::is_file_mod(&base));
        assert!(!ModFile::is_file_mod(&base.join("missing.jar")));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_mod_is_rolled_back_to_its_older_version() {
        let base = directory("rollback");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        write_fabric_mod(&mods.join("a-2.0.jar"), "a", "2.0");
        write_fabric_mod(&mods.join("a-1.0.jar.old"), "a", "1.0");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        manager.files().expect("the mods should be read");

        manager
            .rollback(&mods.join("a-2.0.jar"), &mods.join("a-1.0.jar.old"))
            .expect("the mod should be rolled back");

        assert!(mods.join("a-2.0.jar.old").exists());
        assert!(mods.join("a-1.0.jar").exists());
        assert!(!mods.join("a-2.0.jar").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_rollback_to_a_mod_of_another_identifier_is_refused() {
        let base = directory("rollback-other");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        write_fabric_mod(&mods.join("a.jar"), "a", "2.0");
        write_fabric_mod(&mods.join("b.jar.old"), "b", "1.0");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        manager.files().expect("the mods should be read");

        let error = manager
            .rollback(&mods.join("a.jar"), &mods.join("b.jar.old"))
            .expect_err("the rollback should be refused");
        assert!(matches!(error, ModError::RollbackRefused { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_rollback_to_a_file_that_is_not_a_backup_is_refused() {
        let base = directory("rollback-refused");
        let mods = base.join("mods");
        std::fs::create_dir_all(&mods).expect("dir");
        std::fs::write(mods.join("a.jar"), b"").expect("write");
        std::fs::write(mods.join("b.jar"), b"").expect("write");

        let mut manager = manager(&base, vec![ModLoaderType::Fabric]);
        manager.files().expect("the mods should be read");

        let error = manager
            .rollback(&mods.join("a.jar"), &mods.join("b.jar"))
            .expect_err("the rollback should be refused");
        assert!(matches!(error, ModError::RollbackRefused { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_loaders_of_a_cleanroom_directory_include_forge() {
        let base = directory("cleanroom");
        let mut manager = manager(&base, vec![ModLoaderType::Cleanroom]);
        manager.files().expect("the mods should be read");

        assert!(manager.supports(ModLoaderType::Cleanroom));
        assert!(manager.supports(ModLoaderType::Forge));
        assert!(!manager.supports(ModLoaderType::Fabric));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_loaders_of_a_neoforge_1_20_1_directory_include_forge() {
        let base = directory("neoforge-forge");
        let mut manager =
            ModManager::new(base.join("mods"), vec![ModLoaderType::NeoForge], "1.20.1");
        manager.files().expect("the mods should be read");
        assert!(manager.supports(ModLoaderType::Forge));

        let mut other = ModManager::new(base.join("mods"), vec![ModLoaderType::NeoForge], "1.21.1");
        other.files().expect("the mods should be read");
        assert!(!other.supports(ModLoaderType::Forge));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_loaders_of_a_quilt_directory_include_fabric() {
        let base = directory("quilt-fabric");
        let mut manager = manager(&base, vec![ModLoaderType::Quilt]);
        manager.files().expect("the mods should be read");
        assert!(manager.supports(ModLoaderType::Fabric));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_directory_of_an_unknown_loader_loads_every_kind_of_mod() {
        let base = directory("unknown-loader");
        let mut manager = manager(&base, Vec::new());
        manager.files().expect("the mods should be read");

        assert!(manager.supports(ModLoaderType::Forge));
        assert!(manager.supports(ModLoaderType::Quilt));
        assert!(!manager.supports(ModLoaderType::Unknown));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_files_of_the_mods_are_named() {
        assert!(ModManager::is_disabled(Path::new("a.jar.disabled")));
        assert!(!ModManager::is_disabled(Path::new("a.jar")));
        assert!(ModManager::is_old(Path::new("a.jar.old")));
        assert!(!ModManager::is_old(Path::new("a.jar.disabled")));
    }
}
