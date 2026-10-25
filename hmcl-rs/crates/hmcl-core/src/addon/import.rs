//! Importing the addon files a player picks in a file dialog or drops onto a
//! page of the launcher.
//!
//! The launcher of Java imports the files of a page one by one and remembers
//! how many of them it could not import, so the player is told which files were
//! taken and which were not instead of the whole batch being refused. The files
//! the page does not accept are left out before the work starts, which is what
//! the filters of the file dialog and of the drag listener do there.
//!
//! A data pack is not always a single pack: an archive may hold a `datapacks`
//! directory with several packs and a `resources.zip` with the resources the
//! packs need, so an import of such an archive unpacks the packs and merges the
//! resources into the resources of the world.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::addon::local_mod::ModFile;
use crate::addon::mod_manager::ModManager;
use crate::addon::pack::{is_file_resource_pack, PackManager, DISABLED_PACK_EXTENSION};
use crate::util::versioning::game_version_number::GameVersionNumber;
use crate::util::zip::{self, ZipEntry, ZipWriter};

/// The name of the archive a bundled data pack keeps its resources in.
const RESOURCES_ARCHIVE: &str = "resources.zip";

/// The name of the directory a bundled data pack keeps its packs in.
const DATA_PACK_DIRECTORY: &str = "datapacks";

/// The name of the icon of a pack, which the merged resources do not keep.
const PACK_ICON: &str = "pack.png";

/// The version from which the game reads the resources of a data pack from the
/// resource packs of the world rather than from the root of the world.
const RESOURCES_IN_RESOURCE_PACKS: &str = "26.1-snapshot-6";

/// The metadata the launcher writes into the resources of a world.
///
/// The resources of a bundled data pack are a resource pack of the world, so
/// the game needs a `pack.mcmeta` that names the format the launcher writes.
/// The text is the one the launcher of Java writes.
const GENERATED_RESOURCE_META: &str = r#"{
    "pack": {
        "pack_format": 4,
        "description": "Modified by HMCL."
    }
}"#;

/// An addon file the launcher imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAddon {
    /// The file the player chose.
    pub source: PathBuf,
    /// The files the addon is now in.
    ///
    /// A mod and a resource pack are one file, and so is a data pack that is a
    /// single pack. An archive of several data packs is unpacked, so the list
    /// holds every pack the archive held.
    pub targets: Vec<PathBuf>,
}

/// A file the launcher could not import, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFailure {
    /// The file the player chose.
    pub source: PathBuf,
    /// Why the file was not imported.
    pub reason: String,
}

/// The result of importing a list of files.
///
/// A file the page does not accept is skipped rather than failed, because the
/// player did not choose it as an addon of that page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// The files the launcher imported.
    pub imported: Vec<ImportedAddon>,
    /// The files the launcher could not import, with the reason.
    pub failed: Vec<ImportFailure>,
    /// The files the page does not accept, which are left out unused.
    pub skipped: Vec<PathBuf>,
}

impl ImportReport {
    /// Returns whether every file the page accepts was imported.
    pub fn is_complete(&self) -> bool {
        self.failed.is_empty()
    }

    /// Records a file the launcher could not import.
    fn fail(&mut self, source: &Path, reason: impl Into<String>) {
        self.failed.push(ImportFailure {
            source: source.to_path_buf(),
            reason: reason.into(),
        });
    }
}

/// The failure of importing an addon file.
#[derive(Debug)]
pub enum ImportError {
    /// A file or a directory cannot be read, written or deleted.
    Io {
        /// The path the operation was about.
        path: PathBuf,
        /// The reason the operation failed.
        error: std::io::Error,
    },
    /// The archive of a data pack cannot be read.
    Archive {
        /// The archive that cannot be read.
        path: PathBuf,
        /// The reason the archive cannot be read.
        error: zip::ZipError,
    },
    /// The archive is neither a data pack nor a bundle of data packs.
    NotADataPack {
        /// The archive that holds no data pack.
        path: PathBuf,
    },
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, error } => {
                write!(formatter, "{} cannot be used: {error}", path.display())
            }
            Self::Archive { path, error } => {
                write!(formatter, "{} cannot be read: {error}", path.display())
            }
            Self::NotADataPack { path } => {
                write!(formatter, "{} is not a data pack", path.display())
            }
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            Self::Archive { error, .. } => Some(error),
            Self::NotADataPack { .. } => None,
        }
    }
}

/// Returns whether a path is a mod the page of the mods accepts.
///
/// The page accepts the files the launcher reads the metadata of a mod from,
/// whichever state the file is in, so a mod that is disabled or kept as a
/// backup can be imported as well.
pub fn is_importable_mod(path: &Path) -> bool {
    ModFile::is_file_name_mod(path)
}

/// Returns whether a path is a resource pack the page of the packs accepts.
///
/// The page accepts an archive with the name of a pack and a directory that
/// holds the metadata of a pack.
pub fn is_importable_resource_pack(path: &Path) -> bool {
    is_file_resource_pack(path)
}

/// Returns whether a path is a data pack the page of the packs accepts.
///
/// The page of the data packs only accepts archives, because a data pack is
/// unpacked when it holds several packs. The extension is compared without the
/// case, for the file dialog of Windows does not tell the cases apart either.
pub fn is_importable_data_pack(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

/// Imports the mods a player chose into the directory of the mods.
///
/// Every file is imported on its own, so one file that cannot be copied does
/// not stop the others. The files the page does not accept are skipped.
pub fn import_mods(manager: &mut ModManager, paths: &[PathBuf]) -> ImportReport {
    let mut report = ImportReport::default();

    for path in paths {
        if !is_importable_mod(path) {
            report.skipped.push(path.clone());
            continue;
        }

        match manager.add_mod(path) {
            Ok(file) => report.imported.push(ImportedAddon {
                source: path.clone(),
                targets: vec![file.path().to_path_buf()],
            }),
            Err(error) => report.fail(path, error.to_string()),
        }
    }

    report
}

/// Imports the resource packs a player chose into the directory of the packs.
///
/// Every file is imported on its own. A pack that is a directory is copied with
/// its files, an archive is copied as it is, and a pack with the name of a pack
/// that is already there replaces it.
pub fn import_resource_packs(manager: &mut PackManager, paths: &[PathBuf]) -> ImportReport {
    let mut report = ImportReport::default();

    for path in paths {
        if !is_importable_resource_pack(path) {
            report.skipped.push(path.clone());
            continue;
        }

        match manager.import(path) {
            Ok(pack) => report.imported.push(ImportedAddon {
                source: path.clone(),
                targets: vec![pack.path().to_path_buf()],
            }),
            Err(error) => report.fail(path, error.to_string()),
        }
    }

    report
}

/// Imports the data packs a player chose into the directory of the packs of a
/// world.
///
/// An archive that is a single pack is copied into the directory, and an
/// archive that holds several packs is unpacked into it, with the resources it
/// carries merged into the resources of the world. A pack that is already in
/// the world under the same name is replaced, so importing a newer version of a
/// pack does not leave both versions behind.
///
/// The manager keeps the packs it read, so a caller has to read the directory
/// again after an import.
pub fn import_data_packs(manager: &mut PackManager, paths: &[PathBuf]) -> ImportReport {
    let mut report = ImportReport::default();
    let directory = manager.directory().to_path_buf();
    let world = world_of(&directory);
    let new_layout = resources_live_in_resource_packs(manager.game_version());

    for path in paths {
        if !is_importable_data_pack(path) {
            report.skipped.push(path.clone());
            continue;
        }

        match install_data_pack(&directory, &world, new_layout, path) {
            Ok(imported) => report.imported.push(imported),
            Err(error) => report.fail(path, error.to_string()),
        }
    }

    report
}

/// Returns the directory of the world a directory of data packs belongs to.
///
/// The game keeps the data packs of a world inside the world, so the resources
/// a bundled pack carries are put beside the directory of the packs.
fn world_of(directory: &Path) -> PathBuf {
    directory
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Returns whether the game reads the resources of a world from the resource
/// packs of the world.
///
/// The launcher of Java compares the version of the game with the snapshot that
/// moved the resources, and falls back to the old place when no version is
/// known.
fn resources_live_in_resource_packs(version: Option<&GameVersionNumber>) -> bool {
    let first = GameVersionNumber::as_game_version(RESOURCES_IN_RESOURCE_PACKS);
    version.is_some_and(|version| *version >= first)
}

/// Imports a data pack archive into the directory of the packs of a world.
fn install_data_pack(
    directory: &Path,
    world: &Path,
    new_layout: bool,
    path: &Path,
) -> Result<ImportedAddon, ImportError> {
    std::fs::create_dir_all(directory).map_err(|error| ImportError::Io {
        path: directory.to_path_buf(),
        error,
    })?;

    let entries = zip::entries(path).map_err(|error| ImportError::Archive {
        path: path.to_path_buf(),
        error,
    })?;

    let bundled = entries.iter().any(is_data_pack_directory);
    let single = entries.iter().any(|entry| entry.name == "pack.mcmeta");

    if !bundled && !single {
        return Err(ImportError::NotADataPack {
            path: path.to_path_buf(),
        });
    }

    if !bundled {
        return install_single_pack(directory, path);
    }

    // The packs the archive holds replace the packs the world already has under
    // the same name, whichever state the loose file is in.
    let names: BTreeSet<String> = entries.iter().filter_map(pack_name_of_entry).collect();
    remove_packs(directory, &names)?;

    let targets = zip::extract_under_directory(path, directory, DATA_PACK_DIRECTORY, |_, _| true)
        .map_err(|error| ImportError::Archive {
        path: path.to_path_buf(),
        error,
    })?;

    merge_resources(world, new_layout, path)?;

    Ok(ImportedAddon {
        source: path.to_path_buf(),
        targets,
    })
}

/// Imports an archive that holds a single data pack.
fn install_single_pack(directory: &Path, path: &Path) -> Result<ImportedAddon, ImportError> {
    let file_name = path.file_name().ok_or_else(|| ImportError::NotADataPack {
        path: path.to_path_buf(),
    })?;
    let target = directory.join(file_name);

    let name = name_without_extension(&file_name.to_string_lossy());
    remove_packs(directory, &BTreeSet::from([name]))?;

    std::fs::copy(path, &target).map_err(|error| ImportError::Io {
        path: target.clone(),
        error,
    })?;

    Ok(ImportedAddon {
        source: path.to_path_buf(),
        targets: vec![target],
    })
}

/// Returns whether an entry of an archive is the directory of the packs of a
/// bundle.
fn is_data_pack_directory(entry: &ZipEntry) -> bool {
    entry.name == DATA_PACK_DIRECTORY || entry.name.starts_with("datapacks/")
}

/// Returns the name of the pack an entry of the directory of a bundle belongs
/// to, if the entry is a direct child of the directory.
fn pack_name_of_entry(entry: &ZipEntry) -> Option<String> {
    let rest = entry.name.strip_prefix("datapacks/")?;
    match rest.split_once('/') {
        // A file of a directory of a pack: the directory names the pack.
        Some((name, _)) => Some(name.to_string()),
        // A pack that is an archive, or an empty directory of a pack.
        None => Some(name_without_extension(rest)),
    }
}

/// Removes the packs of a directory whose name names one of the names.
///
/// The name of a pack leaves out the extension of an archive and the suffix of
/// a pack the launcher disabled, so an enabled pack, a disabled pack and a pack
/// of another state are all found by the same name.
fn remove_packs(directory: &Path, names: &BTreeSet<String>) -> Result<(), ImportError> {
    if names.is_empty() || !directory.is_dir() {
        return Ok(());
    }

    let entries = std::fs::read_dir(directory).map_err(|error| ImportError::Io {
        path: directory.to_path_buf(),
        error,
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| ImportError::Io {
            path: directory.to_path_buf(),
            error,
        })?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let name = pack_name_of_file(&file_name);

        if !names.contains(&name) {
            continue;
        }

        let removed = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        removed.map_err(|error| ImportError::Io {
            path: path.clone(),
            error,
        })?;
    }

    Ok(())
}

/// Returns the name of a pack from the name of its file or directory.
///
/// The name leaves out the suffix of a pack the launcher disabled and then the
/// extension of the file, so an enabled pack, a disabled pack and a pack of
/// another state are all found by the same name. The launcher of Java reads a
/// pack name this way, even for a directory, whose dots then also go away.
fn pack_name_of_file(file_name: &str) -> String {
    let name = file_name
        .strip_suffix(&format!(".{DISABLED_PACK_EXTENSION}"))
        .unwrap_or(file_name);
    name_without_extension(name)
}

/// Returns the name of a file without the extension.
///
/// The extension of a name that holds no dot is the whole name in the launcher
/// of Java; the name is then kept as it is, which is what the game does with a
/// pack that has no extension.
fn name_without_extension(file_name: &str) -> String {
    file_name
        .rsplit_once('.')
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| file_name.to_string())
}

/// Merges the resources of a bundle into the resources of a world.
///
/// The resources of the world are a resource pack of the world: the files the
/// bundle carries replace the files of the world of the same name and the other
/// files of the world are kept. The launcher writes the metadata the game needs
/// and drops the icon of the pack, for the icon of a data pack is not the icon
/// of the resources of the world.
fn merge_resources(world: &Path, new_layout: bool, source: &Path) -> Result<PathBuf, ImportError> {
    let target = if new_layout {
        let packs = world.join("resourcepacks");
        std::fs::create_dir_all(&packs).map_err(|error| ImportError::Io {
            path: packs.clone(),
            error,
        })?;
        packs.join(RESOURCES_ARCHIVE)
    } else {
        world.join(RESOURCES_ARCHIVE)
    };

    let mut files = BTreeMap::new();
    if target.is_file() {
        for (name, data) in read_archive(&target)? {
            files.insert(name, data);
        }
    }

    if let Some(bytes) =
        zip::read_entry(source, RESOURCES_ARCHIVE).map_err(|error| ImportError::Archive {
            path: source.to_path_buf(),
            error,
        })?
    {
        for (name, data) in read_archive_of(&bytes, source)? {
            files.insert(name, data);
        }
    }

    files.remove(PACK_ICON);
    files.insert(
        "pack.mcmeta".to_string(),
        GENERATED_RESOURCE_META.as_bytes().to_vec(),
    );

    let mut writer = ZipWriter::create(&target).map_err(|error| ImportError::Archive {
        path: target.clone(),
        error,
    })?;
    for (name, data) in files {
        writer
            .add_file(&name, &data)
            .map_err(|error| ImportError::Archive {
                path: target.clone(),
                error,
            })?;
    }
    writer.finish().map_err(|error| ImportError::Archive {
        path: target.clone(),
        error,
    })?;

    Ok(target)
}

/// Reads the files of an archive.
fn read_archive(path: &Path) -> Result<Vec<(String, Vec<u8>)>, ImportError> {
    let entries = zip::entries(path).map_err(|error| ImportError::Archive {
        path: path.to_path_buf(),
        error,
    })?;

    let mut files = Vec::new();
    for entry in entries {
        if entry.directory {
            continue;
        }
        if let Some(data) =
            zip::read_entry(path, &entry.name).map_err(|error| ImportError::Archive {
                path: path.to_path_buf(),
                error,
            })?
        {
            files.push((entry.name, data));
        }
    }

    Ok(files)
}

/// Reads the files of an archive that is held in memory.
fn read_archive_of(bytes: &[u8], source: &Path) -> Result<Vec<(String, Vec<u8>)>, ImportError> {
    let mut reader = std::io::Cursor::new(bytes);
    let entries = zip::entries_from(&mut reader).map_err(|error| ImportError::Archive {
        path: source.to_path_buf(),
        error,
    })?;

    let mut files = Vec::new();
    for entry in entries {
        if entry.directory {
            continue;
        }
        if let Some(data) = zip::read_entry_from(&mut reader, &entry.name).map_err(|error| {
            ImportError::Archive {
                path: source.to_path_buf(),
                error,
            }
        })? {
            files.push((entry.name, data));
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addon::mod_loader::ModLoaderType;

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-rs-addon-import-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    /// Writes an archive that holds the given files.
    fn archive(path: &Path, files: &[(&str, &[u8])]) {
        let mut writer = ZipWriter::create(path).expect("the archive should be made");
        for (name, data) in files {
            writer
                .add_file(name, data)
                .expect("the file should be added");
        }
        writer.finish().expect("the archive should be written");
    }

    /// Writes an archive that holds a data pack.
    fn data_pack_archive(path: &Path, meta: &str) {
        archive(path, &[("pack.mcmeta", meta.as_bytes())]);
    }

    /// Returns the names of the files of an archive.
    fn names_of(path: &Path) -> Vec<String> {
        zip::entries(path)
            .expect("the archive should be read")
            .into_iter()
            .filter(|entry| !entry.directory)
            .map(|entry| entry.name)
            .collect()
    }

    #[test]
    fn the_files_the_page_accepts_are_imported() {
        let base = directory("mods");
        std::fs::create_dir_all(base.join("mods")).expect("dir");
        std::fs::write(base.join("sodium.jar"), b"data").expect("write");
        std::fs::write(base.join("notes.txt"), b"").expect("write");

        let mut manager = ModManager::new(base.join("mods"), vec![ModLoaderType::Fabric], "1.20.1");
        let report = import_mods(
            &mut manager,
            &[base.join("sodium.jar"), base.join("notes.txt")],
        );

        assert_eq!(report.imported.len(), 1);
        assert_eq!(
            report.imported[0].targets,
            vec![base.join("mods/sodium.jar")]
        );
        assert!(report.failed.is_empty());
        assert!(report.is_complete());
        // The file the player chose is kept, and the file of another kind is
        // left out rather than reported as a failure.
        assert!(base.join("sodium.jar").exists());
        assert_eq!(report.skipped, vec![base.join("notes.txt")]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_the_manager_refuses_does_not_stop_the_others() {
        let base = directory("partial");
        std::fs::create_dir_all(base.join("mods")).expect("dir");
        std::fs::write(base.join("a.jar"), b"a").expect("write");
        std::fs::write(base.join("b.jar"), b"b").expect("write");

        let mut manager = ModManager::new(base.join("mods"), vec![ModLoaderType::Fabric], "1.20.1");
        let report = import_mods(&mut manager, &[base.join("a.jar"), base.join("b.jar")]);

        assert_eq!(report.imported.len(), 2);
        assert!(base.join("mods/a.jar").exists());
        assert!(base.join("mods/b.jar").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_resource_pack_of_a_directory_is_copied_with_its_files() {
        let base = directory("packs");
        std::fs::create_dir_all(base.join("resourcepacks")).expect("dir");
        std::fs::create_dir_all(base.join("source")).expect("dir");
        std::fs::write(
            base.join("source/pack.mcmeta"),
            r#"{ "pack": { "pack_format": 15 } }"#,
        )
        .expect("write");
        std::fs::write(base.join("source/pack.png"), b"icon").expect("write");
        std::fs::write(base.join("source.mp3"), b"").expect("write");

        let mut manager =
            PackManager::resource_packs(base.join("resourcepacks"), base.join("options.txt"));
        let report = import_resource_packs(
            &mut manager,
            &[base.join("source"), base.join("source.mp3")],
        );

        assert_eq!(report.imported.len(), 1);
        assert_eq!(
            report.imported[0].targets,
            vec![base.join("resourcepacks/source")]
        );
        assert!(base.join("resourcepacks/source/pack.png").exists());
        assert_eq!(report.skipped, vec![base.join("source.mp3")]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_single_data_pack_is_copied() {
        let base = directory("single-data");
        let packs = base.join("world").join("datapacks");
        let source = base.join("one.zip");
        data_pack_archive(&source, r#"{ "pack": { "pack_format": 15 } }"#);

        let mut manager = PackManager::data_packs(&packs);
        let report = import_data_packs(&mut manager, std::slice::from_ref(&source));

        assert_eq!(report.imported.len(), 1);
        assert_eq!(report.imported[0].targets, vec![packs.join("one.zip")]);
        assert!(packs.join("one.zip").is_file());
        // A single pack carries no resources of its own.
        assert!(!base.join("world").join("resources.zip").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_bundle_of_data_packs_is_unpacked() {
        let base = directory("bundle");
        let world = base.join("world");
        let packs = world.join("datapacks");
        let source = base.join("bundle.zip");
        archive(
            &source,
            &[
                ("datapacks/one.zip", b"one"),
                ("datapacks/two/pack.mcmeta", b"two"),
                ("resources.zip", b""),
            ],
        );
        // The archive of the resources of the bundle is written on its own, for
        // an archive cannot be held inside another one by the writer.
        let resources = base.join("resources.zip");
        archive(
            &resources,
            &[("assets/two/lang/en_us.json", b"{}"), ("pack.png", b"icon")],
        );
        let bytes = std::fs::read(&resources).expect("read");
        archive(
            &source,
            &[
                ("datapacks/one.zip", b"one"),
                ("datapacks/two/pack.mcmeta", b"two"),
                ("resources.zip", &bytes),
            ],
        );

        let mut manager = PackManager::data_packs(&packs);
        let report = import_data_packs(&mut manager, &[source]);

        assert_eq!(report.imported.len(), 1);
        assert_eq!(
            report.imported[0].targets,
            vec![packs.join("one.zip"), packs.join("two/pack.mcmeta")]
        );
        assert!(packs.join("one.zip").is_file());

        // The resources are put beside the packs, with the metadata of the
        // launcher and without the icon of the bundle.
        let world_resources = world.join("resources.zip");
        let names = names_of(&world_resources);
        assert!(names.contains(&"assets/two/lang/en_us.json".to_string()));
        assert!(names.contains(&"pack.mcmeta".to_string()));
        assert!(!names.contains(&"pack.png".to_string()));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_resources_of_a_new_world_are_a_resource_pack() {
        let base = directory("new-layout");
        let world = base.join("world");
        let packs = world.join("datapacks");
        let source = base.join("bundle.zip");
        archive(&source, &[("datapacks/one.zip", b"one")]);

        let mut manager = PackManager::data_packs(&packs)
            .with_game_version(GameVersionNumber::as_game_version("26.2"));
        import_data_packs(&mut manager, &[source]);

        assert!(world.join("resourcepacks").join("resources.zip").is_file());
        assert!(!world.join("resources.zip").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_older_world_keeps_the_resources_at_its_root() {
        let base = directory("old-layout");
        let world = base.join("world");
        let packs = world.join("datapacks");
        let source = base.join("bundle.zip");
        archive(&source, &[("datapacks/one.zip", b"one")]);

        let mut manager = PackManager::data_packs(&packs)
            .with_game_version(GameVersionNumber::as_game_version("1.20.1"));
        import_data_packs(&mut manager, &[source]);

        assert!(world.join("resources.zip").is_file());
        assert!(!world.join("resourcepacks").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_resources_of_a_world_are_merged() {
        let base = directory("merge");
        let world = base.join("world");
        let packs = world.join("datapacks");
        std::fs::create_dir_all(&world).expect("dir");
        archive(
            &world.join("resources.zip"),
            &[
                ("assets/kept.txt", b"kept"),
                ("assets/replaced.txt", b"old"),
            ],
        );

        let source = base.join("bundle.zip");
        let resources = base.join("resources.zip");
        archive(&resources, &[("assets/replaced.txt", b"new")]);
        let bytes = std::fs::read(&resources).expect("read");
        archive(
            &source,
            &[("datapacks/one.zip", b"one"), ("resources.zip", &bytes)],
        );

        let mut manager = PackManager::data_packs(&packs);
        import_data_packs(&mut manager, &[source]);

        let merged = world.join("resources.zip");
        let names = names_of(&merged);
        assert!(names.contains(&"assets/kept.txt".to_string()));
        assert_eq!(
            zip::read_entry(&merged, "assets/replaced.txt")
                .expect("read")
                .expect("the file should be in the archive"),
            b"new"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_packs_of_the_same_name_are_replaced() {
        let base = directory("replace");
        let world = base.join("world");
        let packs = world.join("datapacks");
        std::fs::create_dir_all(&packs).expect("dir");
        // The world already has the pack as an enabled archive and as a
        // disabled directory, and another pack that has to survive.
        std::fs::write(packs.join("one.zip"), b"old").expect("write");
        std::fs::create_dir_all(packs.join("two.zip.disabled")).expect("dir");
        std::fs::write(packs.join("kept.zip"), b"kept").expect("write");

        let source = base.join("bundle.zip");
        archive(
            &source,
            &[
                ("datapacks/one.zip", b"new"),
                ("datapacks/two/pack.mcmeta", b"two"),
            ],
        );

        let mut manager = PackManager::data_packs(&packs);
        let report = import_data_packs(&mut manager, std::slice::from_ref(&source));
        assert!(report.is_complete());

        assert_eq!(std::fs::read(packs.join("one.zip")).expect("read"), b"new");
        assert!(!packs.join("two.zip.disabled").exists());
        assert!(packs.join("two/pack.mcmeta").is_file());
        assert!(packs.join("kept.zip").is_file());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_archive_that_is_not_a_data_pack_is_refused() {
        let base = directory("not-a-data-pack");
        let packs = base.join("world").join("datapacks");
        let source = base.join("notes.zip");
        archive(&source, &[("readme.txt", b"nothing")]);

        let mut manager = PackManager::data_packs(&packs);
        let report = import_data_packs(&mut manager, &[source]);

        assert_eq!(report.failed.len(), 1);
        assert!(report.failed[0].reason.contains("is not a data pack"));
        assert!(!report.is_complete());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_of_another_kind_is_skipped_by_the_page_of_the_data_packs() {
        let base = directory("data-skip");
        let packs = base.join("world").join("datapacks");
        std::fs::create_dir_all(base.join("source")).expect("dir");
        std::fs::write(base.join("source/pack.mcmeta"), b"{}").expect("write");

        let mut manager = PackManager::data_packs(&packs);
        let report = import_data_packs(&mut manager, &[base.join("source")]);

        assert!(report.imported.is_empty());
        assert_eq!(report.skipped, vec![base.join("source")]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_names_of_the_packs_leave_out_their_extensions() {
        assert_eq!(name_without_extension("one.zip"), "one");
        assert_eq!(name_without_extension("one"), "one");
        assert_eq!(pack_name_of_file("one.zip"), "one");
        assert_eq!(pack_name_of_file("one.zip.disabled"), "one");
        assert_eq!(pack_name_of_file("one.disabled"), "one");
        assert_eq!(pack_name_of_file("one"), "one");
    }
}
