//! The view model of the wizard that writes an instance as a modpack.
//!
//! The wizard of the C# version keeps the state of an export in
//! `ModpackExportContext` and asks it in three steps: the type of the pack, the
//! information of its manifest and the files of the instance it carries. The
//! view model here is the same state, so the pages of the wizard bind to it and
//! the writer of the core turns it into an archive.
//!
//! The pages show the entries of the instance as check boxes, which the player
//! unchecks to leave a file out; the view model keeps the names of the entries
//! the player kept, which is the list the writer asks for.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use ferroui::base::collections::FerroList;
use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use ferroui::controls::ItemsSource;
use hmcl_core::modpack::export::{
    write_mcbbs, write_modrinth, write_server, ExportError, ExportFormat, ExportOutcome,
    ModpackExportInfo,
};
use hmcl_core::modpack::{ModAdviser, ModSuggestion};

use crate::display_model;

display_model!(
    /// One entry of the instance the wizard of an export lists.
    ///
    /// An entry is a file or a directory under the run directory of the
    /// instance; the player checks the ones the exported pack carries.
    ModpackFileEntryViewModel,
    namespace: "HMCL.ViewModels",
    properties: {
        /// The name of the entry, with a slash at its end when it is a directory.
        Name: String => name_property,
        /// The name of the entry in the instance, without the slash.
        EntryName: String => entry_name_property,
        /// Whether the entry is a directory.
        IsDirectory: bool => is_directory_property,
        /// Whether the exported pack carries the entry.
        IsIncluded: bool => is_included_property,
    }
);

impl ModpackFileEntryViewModel {
    /// Returns the name of the entry, as the page shows it.
    pub fn name(&self) -> String {
        self.get_value(Self::name_property())
    }

    /// Sets the name of the entry the page shows.
    pub fn set_name(&self, value: impl Into<String>) {
        self.set_value(Self::name_property(), value.into())
    }

    /// Returns the name of the entry in the instance.
    pub fn entry_name(&self) -> String {
        self.get_value(Self::entry_name_property())
    }

    /// Sets the name of the entry in the instance.
    pub fn set_entry_name(&self, value: impl Into<String>) {
        self.set_value(Self::entry_name_property(), value.into())
    }

    /// Returns whether the entry is a directory.
    pub fn is_directory(&self) -> bool {
        self.get_value(Self::is_directory_property())
    }

    /// Sets whether the entry is a directory.
    pub fn set_is_directory(&self, value: bool) {
        self.set_value(Self::is_directory_property(), value)
    }

    /// Returns whether the exported pack carries the entry.
    pub fn is_included(&self) -> bool {
        self.get_value(Self::is_included_property())
    }

    /// Sets whether the exported pack carries the entry.
    pub fn set_is_included(&self, value: bool) {
        self.set_value(Self::is_included_property(), value)
    }
}

/// An entry of the instance, before it is given to a model.
#[allow(dead_code)]
struct Entry {
    /// The name of the entry in the instance.
    name: String,
    /// The name the page shows, with a slash at its end for a directory.
    shown: String,
    /// Whether the entry is a directory.
    is_directory: bool,
    /// Whether the launcher suggests the entry, so it starts checked.
    suggested: bool,
}

impl Entry {
    /// Returns the entry of a directory entry, or `None` when the launcher
    /// hides it.
    fn of(entry: std::fs::DirEntry) -> Option<Self> {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_directory = entry.path().is_dir();
        let suggestion = ModAdviser::standard().advise(&name, is_directory);
        if suggestion == ModSuggestion::Hidden {
            return None;
        }
        let shown = if is_directory {
            format!("{name}/")
        } else {
            name.clone()
        };
        Some(Self {
            name,
            shown,
            is_directory,
            suggested: suggestion == ModSuggestion::Suggested,
        })
    }
}

display_model!(
    /// The view model of the wizard that writes an instance as a modpack.
    ///
    /// The pages of the wizard bind to it: the page of the type names the
    /// format, the page of the information names the manifest and the page of
    /// the files names the entries of the instance and the path of the archive.
    ModpackExportWizardViewModel,
    namespace: "HMCL.ViewModels",
    properties: {
        /// The name of the modpack.
        Name: String => name_property,
        /// The version of the modpack.
        Version: String => version_property,
        /// The author of the modpack.
        Author: String => author_property,
        /// The description of the modpack.
        Description: String => description_property,
        /// The page of the modpack.
        Url: String => url_property,
        /// The address the manifest of the newest version is read from.
        FileApi: String => file_api_property,
        /// The path of the archive the export writes.
        ExportPath: String => export_path_property,
        /// The name of the format of the export, as the wizard stores it.
        TypeName: String => type_name_property,
        /// Whether the export writes the format of MCBBS.
        IsMcbbs: bool => is_mcbbs_property,
        /// Whether the export writes the format of the server of HMCL.
        IsServer: bool => is_server_property,
        /// Whether the export writes the format of Modrinth.
        IsModrinth: bool => is_modrinth_property,
        /// The smallest amount of the memory the game needs, in mebibytes.
        MinMemory: i32 => min_memory_property,
        /// Whether the launcher updates the instance anyway.
        ForceUpdate: bool => force_update_property,
        /// Whether the archive holds the launcher itself.
        PackWithLauncher: bool => pack_with_launcher_property,
        /// Whether the mods the player disabled are written as well.
        IncludeOptional: bool => include_optional_property,
        /// Whether the export has everything it needs to be written.
        CanFinish: bool => can_finish_property,
        /// The entries of the instance the page of the files lists.
        Files: Option<ItemsSource> => files_property,
    }
);

impl ModpackExportWizardViewModel {
    /// Returns the name of the modpack.
    pub fn name(&self) -> String {
        self.get_value(Self::name_property())
    }

    /// Sets the name of the modpack.
    pub fn set_name(&self, value: impl Into<String>) {
        self.set_value(Self::name_property(), value.into());
        self.refresh_can_finish();
    }

    /// Returns the version of the modpack.
    pub fn version(&self) -> String {
        self.get_value(Self::version_property())
    }

    /// Sets the version of the modpack.
    pub fn set_version(&self, value: impl Into<String>) {
        self.set_value(Self::version_property(), value.into());
        self.refresh_can_finish();
    }

    /// Returns the author of the modpack.
    pub fn author(&self) -> String {
        self.get_value(Self::author_property())
    }

    /// Sets the author of the modpack.
    pub fn set_author(&self, value: impl Into<String>) {
        self.set_value(Self::author_property(), value.into())
    }

    /// Returns the description of the modpack.
    pub fn description(&self) -> String {
        self.get_value(Self::description_property())
    }

    /// Sets the description of the modpack.
    pub fn set_description(&self, value: impl Into<String>) {
        self.set_value(Self::description_property(), value.into())
    }

    /// Returns the page of the modpack.
    pub fn url(&self) -> String {
        self.get_value(Self::url_property())
    }

    /// Sets the page of the modpack.
    pub fn set_url(&self, value: impl Into<String>) {
        self.set_value(Self::url_property(), value.into())
    }

    /// Returns the address the manifest of the newest version is read from.
    pub fn file_api(&self) -> String {
        self.get_value(Self::file_api_property())
    }

    /// Sets the address the manifest of the newest version is read from.
    pub fn set_file_api(&self, value: impl Into<String>) {
        self.set_value(Self::file_api_property(), value.into())
    }

    /// Returns the path of the archive the export writes.
    pub fn export_path(&self) -> String {
        self.get_value(Self::export_path_property())
    }

    /// Sets the path of the archive the export writes.
    pub fn set_export_path(&self, value: impl Into<String>) {
        self.set_value(Self::export_path_property(), value.into());
        self.refresh_can_finish();
    }

    /// Returns the name of the format of the export.
    pub fn type_name(&self) -> String {
        self.get_value(Self::type_name_property())
    }

    /// Sets the format of the export by its name.
    ///
    /// The name is the one the wizard stores, which is `mcbbs`, `server` or
    /// `modrinth`; a name the launcher does not know leaves the format as it
    /// is, because a page that asks for a format the launcher does not write
    /// must not take the wizard to a step it cannot finish.
    pub fn set_type_name(&self, value: impl Into<String>) {
        let name = value.into();
        if ExportFormat::from_name(&name).is_some() {
            self.set_value(Self::type_name_property(), name.clone());
            self.set_value(Self::is_mcbbs_property(), name == "mcbbs");
            self.set_value(Self::is_server_property(), name == "server");
            self.set_value(Self::is_modrinth_property(), name == "modrinth");
        }
    }

    /// Returns the format of the export.
    ///
    /// An export the wizard has not named a format for is written as Modrinth,
    /// which is the format the wizard of the C# version starts with.
    pub fn format(&self) -> ExportFormat {
        ExportFormat::from_name(&self.type_name()).unwrap_or(ExportFormat::Modrinth)
    }

    /// Returns the smallest amount of the memory the game needs.
    pub fn min_memory(&self) -> i32 {
        self.get_value(Self::min_memory_property())
    }

    /// Sets the smallest amount of the memory the game needs.
    pub fn set_min_memory(&self, value: i32) {
        self.set_value(Self::min_memory_property(), value)
    }

    /// Returns whether the launcher updates the instance anyway.
    pub fn force_update(&self) -> bool {
        self.get_value(Self::force_update_property())
    }

    /// Sets whether the launcher updates the instance anyway.
    pub fn set_force_update(&self, value: bool) {
        self.set_value(Self::force_update_property(), value)
    }

    /// Returns whether the archive holds the launcher itself.
    pub fn pack_with_launcher(&self) -> bool {
        self.get_value(Self::pack_with_launcher_property())
    }

    /// Sets whether the archive holds the launcher itself.
    pub fn set_pack_with_launcher(&self, value: bool) {
        self.set_value(Self::pack_with_launcher_property(), value)
    }

    /// Returns whether the mods the player disabled are written as well.
    pub fn include_optional(&self) -> bool {
        self.get_value(Self::include_optional_property())
    }

    /// Sets whether the mods the player disabled are written as well.
    pub fn set_include_optional(&self, value: bool) {
        self.set_value(Self::include_optional_property(), value)
    }

    /// Returns whether the export has everything it needs to be written.
    pub fn can_finish(&self) -> bool {
        self.get_value(Self::can_finish_property())
    }

    /// Lists the entries of the instance the pages show, sorted by their names.
    ///
    /// The entries the launcher hides are left out, and the ones it suggests
    /// are checked, which is the way the wizard of the C# version starts: the
    /// player unchecks what the pack must not carry.
    pub fn load_files(&self, run_directory: &std::path::Path) {
        let entries = std::fs::read_dir(run_directory)
            .map(|entries| {
                let mut names: Vec<Entry> = entries
                    .filter_map(Result::ok)
                    .filter_map(Entry::of)
                    .collect();
                names.sort_by(|left, right| left.shown.cmp(&right.shown));
                names
            })
            .unwrap_or_default();

        let list: Rc<FerroList<Ref<ModpackFileEntryViewModel>>> = Rc::new(FerroList::new());
        for entry in entries {
            let model = ModpackFileEntryViewModel::new();
            model.set_name(entry.shown);
            model.set_entry_name(entry.name);
            model.set_is_directory(entry.is_directory);
            model.set_is_included(entry.suggested);
            list.add(model);
        }

        self.set_value(Self::files_property(), Some(ItemsSource::from(list)));
    }

    /// Returns the models of the entries of the instance.
    fn file_entries(&self) -> Vec<Ref<ModpackFileEntryViewModel>> {
        let Some(source) = self.get_value(Self::files_property()) else {
            return Vec::new();
        };
        source
            .downcast_ref::<FerroList<Ref<ModpackFileEntryViewModel>>>()
            .map(|list| list.to_vec())
            .unwrap_or_default()
    }

    /// Returns the names of the entries the player kept.
    ///
    /// An entry is named the way the writer of the core reads it, which is the
    /// name of the file or of the directory in the instance.
    pub fn included_entries(&self) -> Vec<String> {
        self.file_entries()
            .into_iter()
            .filter(|entry| entry.is_included())
            .map(|entry| entry.entry_name())
            .collect()
    }

    /// Checks or unchecks every entry of the instance.
    pub fn set_all_included(&self, included: bool) {
        for entry in self.file_entries() {
            entry.set_is_included(included);
        }
    }

    /// Returns the settings the writer of the core asks for.
    pub fn export_info(&self) -> ModpackExportInfo {
        ModpackExportInfo {
            whitelist: self.included_entries(),
            name: self.name(),
            author: self.author(),
            version: self.version(),
            description: self.description(),
            url: self.url(),
            file_api: {
                let api = self.file_api();
                (!api.is_empty()).then_some(api)
            },
            min_memory: self.min_memory(),
            supported_java_versions: None,
            launch_arguments: String::new(),
            java_arguments: String::new(),
            authlib_injector_server: None,
            origins: Vec::new(),
            force_update: self.force_update(),
            pack_with_launcher: self.pack_with_launcher(),
            no_create_remote_files: false,
            skip_curse_forge_remote_files: false,
        }
    }

    /// Returns the path of the archive the export writes.
    ///
    /// The page of the files names the path as the player wrote it; a path that
    /// names no file is not a path the writer can use.
    pub fn archive_path(&self) -> Option<PathBuf> {
        let path = self.export_path().trim().to_string();
        if path.is_empty() {
            return None;
        }
        Some(PathBuf::from(path))
    }

    /// Fills the view model from an instance, which is how the wizard starts.
    ///
    /// The name of the instance is the name of the pack, the player is its
    /// author and the version starts at `1.0`, as in the wizard of the C#
    /// version.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_instance(&self, display_name: &str, default_path: &str) {
        self.set_name(display_name);
        self.set_version("1.0");
        self.set_author(author_of_the_player());
        self.set_export_path(default_path);
        self.set_type_name("modrinth");
    }

    /// Sets whether the export has everything it needs to be written.
    ///
    /// The wizard asks for a name, a version and a path; the writer refuses an
    /// export that names none of them.
    fn refresh_can_finish(&self) {
        let ready = !self.name().trim().is_empty()
            && !self.version().trim().is_empty()
            && !self.export_path().trim().is_empty();
        self.set_value(Self::can_finish_property(), ready);
    }

    /// Writes the instance to the archive the player named.
    ///
    /// The writer of the core is asked for the format the player chose and the
    /// entries the player kept; the game version and the components are the
    /// ones of the instance the export starts from. Bundling the launcher into
    /// the archive needs its jar and its settings, which the page supplies when
    /// it calls this method, so bundling stays with the page.
    pub fn write_export(
        &self,
        run_directory: &Path,
        game_version: &str,
        components: &[(String, String)],
    ) -> Result<ExportOutcome, ExportError> {
        let info = self.export_info();
        let archive = self
            .archive_path()
            .ok_or(ExportError::Missing("archive path"))?;
        let outcome = match self.format() {
            ExportFormat::Mcbbs => {
                write_mcbbs(run_directory, &archive, &info, components, game_version)?
            }
            ExportFormat::Server => {
                write_server(run_directory, &archive, &info, components, game_version)?
            }
            ExportFormat::Modrinth => write_modrinth(
                run_directory,
                &archive,
                &info,
                components,
                game_version,
                |_, _| None,
            )?,
        };
        Ok(outcome)
    }
}

/// Returns the name of the player, which the wizard names as the author.
#[allow(dead_code)]
fn author_of_the_player() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "player".to_string())
}
