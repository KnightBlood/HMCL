//! The optional files the player chooses to install.
//!
//! A modpack may name a file the game does not need; the wizard of an install
//! asks the player which of such files it should write ([`OptionalFilesPage`]
//! in the launcher of Java). The choice is kept as the keys of the files the
//! player did not install, which [`super::completion`] reads and writes under
//! the root of the instance.
//!
//! The formats of the modpacks name the keys in their own way, so the module
//! joins them with the trait [`ModpackFile`] and keeps the choice in
//! [`OptionalFiles`], which is what a page binds its check boxes to: a file is
//! installed when its key is not in the set of the excluded keys.
//!
//! [`OptionalFilesPage`]: https://github.com/HMCL-dev/HMCL/blob/master/HMCL/src/main/java/org/jackhuang/hmcl/ui/download/OptionalFilesPage.java

use std::collections::BTreeSet;

/// A file of a modpack the player may leave out.
///
/// The manifests of the formats name the files in their own way, and every one
/// of them names a key that is stable between the versions of a modpack, a path
/// in the instance and whether the game needs the file at all.
pub trait ModpackFile {
    /// Returns the key the choice of the player is kept by.
    ///
    /// The key is stable between the versions of a modpack, so the choice of the
    /// player survives an update; the name of a file may change between them.
    fn key(&self) -> String;

    /// Returns the path of the file in the instance, or `None` when the
    /// launcher has not resolved the file yet.
    fn path(&self) -> Option<String>;

    /// Returns the name of the file without the directories of its path, or
    /// `None` when the launcher has not resolved the file yet.
    fn file_name(&self) -> Option<String> {
        self.path()
            .and_then(|path| path.rsplit('/').next().map(|name| name.to_string()))
            .filter(|name| !name.is_empty())
    }

    /// Returns whether the game works without the file.
    fn optional(&self) -> bool;
}

impl ModpackFile for super::curse::CurseFile {
    fn key(&self) -> String {
        Self::key(self)
    }

    fn path(&self) -> Option<String> {
        Self::path(self)
    }

    fn optional(&self) -> bool {
        Self::optional(self)
    }
}

impl ModpackFile for super::modrinth::ModrinthFile {
    fn key(&self) -> String {
        Self::key(self)
    }

    fn path(&self) -> Option<String> {
        Some(self.path.clone())
    }

    fn optional(&self) -> bool {
        Self::optional(self)
    }
}

impl ModpackFile for super::mcbbs::McbbsFile {
    fn key(&self) -> String {
        Self::key(self)
    }

    fn path(&self) -> Option<String> {
        Self::path(self).map(|path| path.to_string())
    }

    fn optional(&self) -> bool {
        // The format of MCBBS names no file the game may do without.
        false
    }
}

/// The choice of the player about the optional files of a modpack.
///
/// The page of the wizard shows one check box per optional file; the launcher
/// keeps the keys of the files the player unchecked, so a file it has never
/// seen is installed, which is what an instance of an older launcher means.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionalFiles {
    /// The keys of the files the player did not install.
    excluded: BTreeSet<String>,
}

impl OptionalFiles {
    /// Returns a choice that installs every file.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a choice that leaves out the files of the given keys.
    pub fn with_excluded(excluded: BTreeSet<String>) -> Self {
        Self { excluded }
    }

    /// Returns the keys of the files the player did not install.
    pub const fn excluded(&self) -> &BTreeSet<String> {
        &self.excluded
    }

    /// Returns whether the player installs the file.
    pub fn selected(&self, file: &impl ModpackFile) -> bool {
        !self.excluded.contains(&file.key())
    }

    /// Installs the file, and returns whether the choice changed.
    pub fn select(&mut self, file: &impl ModpackFile) -> bool {
        self.excluded.remove(&file.key())
    }

    /// Leaves the file out, and returns whether the choice changed.
    pub fn deselect(&mut self, file: &impl ModpackFile) -> bool {
        self.excluded.insert(file.key())
    }

    /// Sets whether the player installs the file.
    pub fn set_selected(&mut self, file: &impl ModpackFile, selected: bool) {
        if selected {
            let _ = self.select(file);
        } else {
            let _ = self.deselect(file);
        }
    }

    /// Installs every optional file.
    pub fn select_all(&mut self) {
        self.excluded.clear();
    }

    /// Leaves every optional file out.
    pub fn clear(&mut self, files: &[&dyn ModpackFile]) {
        self.excluded = files.iter().map(|file| file.key()).collect();
    }

    /// Returns the files the player installs, in the order they are named.
    ///
    /// A file the game needs is always installed; only an optional file the
    /// player unchecked is left out.
    pub fn filter<'a, F: ModpackFile>(&self, files: &'a [F]) -> Vec<&'a F> {
        files
            .iter()
            .filter(|file| !file.optional() || self.selected(*file))
            .collect()
    }

    /// Returns the optional files of a list, in the order they are named.
    pub fn optional_of<F: ModpackFile>(files: &[F]) -> Vec<&F> {
        files.iter().filter(|file| file.optional()).collect()
    }

    /// Reads the choice the player made for the instance.
    ///
    /// The keys are kept in [`EXCLUDED_FILE`](super::completion::EXCLUDED_FILE)
    /// under the root of the instance; an instance that has no such file
    /// installs every optional file.
    pub fn read(root: &std::path::Path) -> Self {
        Self {
            excluded: super::completion::read_excluded(root).unwrap_or_default(),
        }
    }

    /// Writes the choice of the player under the root of the instance.
    pub fn write(&self, root: &std::path::Path) -> std::io::Result<()> {
        super::completion::write_excluded(root, &self.excluded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpack::curse::CurseFile;

    /// Returns a file of CurseForge the game needs.
    fn required() -> CurseFile {
        CurseFile {
            project_id: 12,
            file_id: 34,
            required: true,
            ..CurseFile::default()
        }
    }

    /// Returns a file of CurseForge the game does without.
    fn optional() -> CurseFile {
        CurseFile {
            project_id: 56,
            file_id: 78,
            required: false,
            file_name: Some("jei.jar".to_string()),
            ..CurseFile::default()
        }
    }

    #[test]
    fn a_file_the_game_needs_is_always_installed() {
        let choice = OptionalFiles::new();
        let file = required();
        assert!(choice.selected(&file));
        assert!(!file.optional());

        // The page of the wizard names the optional files only.
        assert!(OptionalFiles::optional_of(&[required(), optional()]).len() == 1);
    }

    #[test]
    fn the_choice_of_the_player_changes_the_files_installed() {
        let files = vec![required(), optional()];
        let mut choice = OptionalFiles::new();
        assert_eq!(choice.filter(&files).len(), 2);

        assert!(choice.deselect(&optional()));
        assert!(!choice.selected(&optional()));
        assert_eq!(choice.filter(&files).len(), 1);
        assert_eq!(choice.excluded().len(), 1);

        assert!(choice.select(&optional()));
        assert!(choice.selected(&optional()));
        assert!(choice.excluded().is_empty());
        assert_eq!(choice.filter(&files).len(), 2);
    }

    #[test]
    fn every_optional_file_can_be_left_out_or_installed() {
        let files = vec![required(), optional()];
        let mut choice = OptionalFiles::new();

        choice.clear(&[&files[0] as &dyn ModpackFile, &files[1] as &dyn ModpackFile]);
        assert_eq!(choice.excluded().len(), 2);
        // A file the game needs is installed even when the player unchecked it.
        assert_eq!(choice.filter(&files).len(), 1);

        choice.select_all();
        assert!(choice.excluded().is_empty());
        assert_eq!(choice.filter(&files).len(), 2);
    }

    #[test]
    fn the_name_of_a_file_is_the_last_part_of_its_path() {
        let file = optional();
        assert_eq!(file.path().as_deref(), Some("mods/jei.jar"));
        assert_eq!(file.file_name().as_deref(), Some("jei.jar"));
    }

    #[test]
    fn a_file_of_curseforge_is_optional_when_its_manifest_says_so() {
        assert!(optional().optional());
        assert!(!required().optional());
    }

    #[test]
    fn the_choice_of_the_player_is_kept_under_the_instance() {
        let root = std::env::temp_dir().join(format!(
            "hmcl-rs-optional-{}-{}",
            std::process::id(),
            "kept"
        ));
        let _ = std::fs::remove_dir_all(&root);

        // An instance that keeps no choice installs every optional file.
        assert_eq!(OptionalFiles::read(&root), OptionalFiles::new());

        let mut choice = OptionalFiles::new();
        let _ = choice.deselect(&optional());
        choice.write(&root).expect("the choice should be written");
        assert_eq!(OptionalFiles::read(&root), choice);
        assert!(!OptionalFiles::read(&root).selected(&optional()));

        // An empty choice removes the file again.
        OptionalFiles::new()
            .write(&root)
            .expect("the choice should be written");
        assert!(!root.join(crate::modpack::EXCLUDED_FILE).exists());

        let _ = std::fs::remove_dir_all(&root);
    }
}
