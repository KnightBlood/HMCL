//! What an install of a modpack reports about the files it could not place.
//!
//! An install of a modpack downloads the files the manifest names, but a file
//! of a mod repository is named by its identifiers only: the launcher asks the
//! repository for the name of the file and for the address it is downloaded
//! from. A repository may be unreachable or may have deleted the file, and the
//! launcher then cannot install the file. The launcher does not fail the whole
//! install for one file: it installs the files it could get and reports the
//! files it could not, so the player knows what the modpack is missing.
//!
//! The report also carries the progress of the install: the stages are the ones
//! the user interface shows and the counts are the number of the files.

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use crate::addon::local_mod::{DISABLED_EXTENSION, OLD_EXTENSION};

use super::curse::CurseManifest;
use super::mcbbs::{McbbsFile, McbbsModpackManifest};
use super::modrinth::ModrinthManifest;

/// The stage of an install that unpacks the archive of the modpack.
pub const UNPACK_STAGE: &str = "hmcl.modpack";

/// The file the launcher keeps the optional files of a modpack the player did
/// not install in.
///
/// The file is a list of the keys of the files
/// ([`CurseFile::key`](super::curse::CurseFile::key) and
/// [`ModrinthFile::key`](super::modrinth::ModrinthFile::key)), so the launcher
/// completes the instance the same way after a restart.
pub const EXCLUDED_FILE: &str = "excluded.json";

/// The stage of an install that downloads the files of the modpack.
///
/// The launcher completes an instance from the server of the modpack in the
/// same stage, because the player sees a single download.
pub const DOWNLOAD_STAGE: &str = "hmcl.modpack.download";

/// The kind of a file the launcher could not find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedKind {
    /// The launcher has not asked the repository for the file yet.
    ///
    /// The launcher resolves the files of a modpack one by one, and a file the
    /// repository named needs no asking; the report of a modpack that has just
    /// been read therefore holds the files the repository has to name.
    NotResolved,
    /// The repository of the modpack does not hold the file any more.
    Deleted,
    /// The launcher could not reach the repository.
    Unreachable,
    /// The repository answered with a file the launcher does not understand.
    Malformed,
}

impl fmt::Display for UnresolvedKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotResolved => formatter.write_str("not resolved"),
            Self::Deleted => formatter.write_str("deleted"),
            Self::Unreachable => formatter.write_str("unreachable"),
            Self::Malformed => formatter.write_str("malformed"),
        }
    }
}

/// A file of a modpack the launcher could not resolve.
///
/// The identifiers are the ones the manifest names, so the player can look the
/// mod up by hand; the reason tells whether the file is gone or the launcher
/// could not ask for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedFile {
    /// The kind of the file, such as `curse`.
    pub kind: &'static str,
    /// The name of the file, if the manifest names it.
    pub name: Option<String>,
    /// The identifier of the project, if the file belongs to one.
    pub project: Option<i32>,
    /// The identifier of the file, if the file belongs to a project.
    pub file: Option<i32>,
    /// Why the launcher could not resolve the file.
    pub reason: UnresolvedKind,
}

impl UnresolvedFile {
    /// Returns the file of a mod of CurseForge the launcher could not resolve.
    pub fn curse(project: i32, file: i32, reason: UnresolvedKind) -> Self {
        Self {
            kind: "curse",
            name: None,
            project: Some(project),
            file: Some(file),
            reason,
        }
    }

    /// Returns the file of a mod of CurseForge whose name the launcher knows.
    ///
    /// A file the repository named but whose address the launcher could not
    /// build is reported with its name, so the player can look the file up.
    pub fn curse_named(
        project: i32,
        file: i32,
        name: impl Into<String>,
        reason: UnresolvedKind,
    ) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::curse(project, file, reason)
        }
    }

    /// Returns the text the launcher shows the player.
    pub fn describe(&self) -> String {
        let name = match (&self.name, self.project, self.file) {
            (Some(name), _, _) => name.clone(),
            (None, Some(project), Some(file)) => format!("{}{project}/{file}", self.kind),
            _ => self.kind.to_string(),
        };

        format!("{name}: {reason}", reason = self.reason)
    }
}

impl fmt::Display for UnresolvedFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.describe())
    }
}

/// The progress and the result of an install of a modpack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionReport {
    /// The stage the install is in.
    pub stage: String,
    /// The number of the files the install has to place.
    pub total: u64,
    /// The number of the files the install placed.
    pub done: u64,
    /// The files the launcher could not resolve.
    pub unresolved: Vec<UnresolvedFile>,
    /// What the install noticed and the player should know.
    pub warnings: Vec<String>,
}

impl CompletionReport {
    /// Creates a report of an install that is about to place the files.
    pub fn new(stage: impl Into<String>, total: u64) -> Self {
        Self {
            stage: stage.into(),
            total,
            ..CompletionReport::default()
        }
    }

    /// Records that the install placed one more file.
    ///
    /// The progress never goes past the total, because the count of the files
    /// is the number the report was created with and the files the launcher
    /// skips are counted as done.
    pub fn advance(&mut self) {
        if self.done < self.total {
            self.done += 1;
        }
    }

    /// Records that the install could not resolve a file.
    pub fn unresolved(&mut self, file: UnresolvedFile) {
        self.unresolved.push(file);
    }

    /// Records what the install noticed.
    pub fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    /// Returns the progress of the install, as the user interface shows it.
    pub fn progress(&self) -> (u64, u64) {
        (self.done, self.total)
    }

    /// Returns whether the install placed every file.
    pub fn is_complete(&self) -> bool {
        self.unresolved.is_empty()
    }

    /// Returns the text the launcher shows when the install is done.
    ///
    /// A modpack the launcher could not complete is reported to the player
    /// instead of being installed silently with files missing.
    pub fn summary(&self) -> Option<String> {
        if self.unresolved.is_empty() {
            return None;
        }

        let files: Vec<String> = self
            .unresolved
            .iter()
            .map(UnresolvedFile::describe)
            .collect();
        Some(format!(
            "The modpack was installed without {} of its files: {}",
            files.len(),
            files.join("; ")
        ))
    }

    /// Returns the failure an install that could not place every file reports.
    ///
    /// The launcher tells a file the repository deleted from a file it could not
    /// ask about, because the first makes the modpack unusable and the second is
    /// worth asking again later.
    pub fn error(&self) -> Option<CompletionError> {
        if self.unresolved.is_empty() {
            None
        } else if self
            .unresolved
            .iter()
            .any(|file| file.reason == UnresolvedKind::Deleted)
        {
            Some(CompletionError::Deleted)
        } else {
            Some(CompletionError::Incomplete)
        }
    }
}

/// Why an install could not place every file of a modpack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionError {
    /// The repository no longer holds a file the modpack needs.
    Deleted,
    /// The launcher could not ask the repository for a file.
    Incomplete,
}

impl fmt::Display for CompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deleted => formatter.write_str("The repository deleted a file of the modpack"),
            Self::Incomplete => {
                formatter.write_str("The files of the modpack could not be resolved")
            }
        }
    }
}

impl std::error::Error for CompletionError {}

/// A file a completion downloads, with every address it may be downloaded from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionDownload {
    /// The addresses of the file, in the order they are tried.
    ///
    /// The launcher tries each of them until one answers with the file the
    /// manifest names.
    pub urls: Vec<String>,
    /// The file the download is written to.
    pub path: std::path::PathBuf,
    /// The algorithm and the digest the file must have, if the manifest or the
    /// repository names one.
    pub integrity: Option<(String, String)>,
}

impl CompletionDownload {
    /// Creates a download of a file from one address.
    pub fn of(url: impl Into<String>, path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            urls: vec![url.into()],
            path: path.into(),
            integrity: None,
        }
    }
}

/// The work a completion of a modpack has to do.
///
/// A completion is the second half of an install: the launcher has unpacked the
/// archive of the modpack, and the files the manifest names by identifiers only
/// still have to be downloaded. The plan names the files to download, the files
/// the launcher has to ask the repository for, and what it noticed on the way.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionPlan {
    /// The files the completion downloads.
    pub downloads: Vec<CompletionDownload>,
    /// The files the launcher removes, because the new version of the modpack
    /// no longer holds them.
    ///
    /// A plain completion removes nothing; an update fills the list.
    pub removals: Vec<std::path::PathBuf>,
    /// The files the launcher could not place.
    pub unresolved: Vec<UnresolvedFile>,
    /// What the completion noticed and the player should know.
    pub warnings: Vec<String>,
    /// The number of the files the manifest names.
    pub total: u64,
}

impl CompletionPlan {
    /// Returns the report the user interface shows of the completion.
    pub fn report(&self) -> CompletionReport {
        CompletionReport {
            stage: DOWNLOAD_STAGE.to_string(),
            total: self.total,
            done: 0,
            unresolved: self.unresolved.clone(),
            warnings: self.warnings.clone(),
        }
    }

    /// Returns whether the plan has nothing to download.
    pub fn is_empty(&self) -> bool {
        self.downloads.is_empty()
    }
}

/// Returns the files of a modpack of MCBBS the launcher still has to resolve.
///
/// A mod of CurseForge whose file name the manifest names is resolved: the
/// name tells where the file goes and the identifier tells what to ask for.
/// The files the manifest names with a name only need no server, so the
/// launcher downloads them from the manifest of the modpack itself.
pub fn unresolved_files(manifest: &McbbsModpackManifest) -> Vec<UnresolvedFile> {
    let mut unresolved = Vec::new();

    for file in manifest.files() {
        if let McbbsFile::Curse {
            project_id,
            file_id,
            file_name,
            ..
        } = file
        {
            if file_name.as_deref().is_none_or(str::is_empty) {
                unresolved.push(UnresolvedFile::curse(
                    *project_id,
                    *file_id,
                    UnresolvedKind::NotResolved,
                ));
            }
        }
    }

    unresolved
}

/// Returns the report of a modpack of MCBBS that has not been completed yet.
///
/// The report names the files the launcher has to ask the repository for, so
/// the user interface can show the work before it starts.
pub fn report_of(manifest: &McbbsModpackManifest) -> CompletionReport {
    let mut report = CompletionReport::new(DOWNLOAD_STAGE, manifest.files().len() as u64);
    for file in unresolved_files(manifest) {
        report.unresolved(file);
    }
    report
}

/// Returns the files of a modpack of CurseForge the launcher still has to resolve.
///
/// A file the manifest names by name and address is resolved; a file whose name
/// the manifest does not name has to be asked for, which is what the launcher
/// tells the player before the download starts.
pub fn unresolved_curse_files(manifest: &CurseManifest) -> Vec<UnresolvedFile> {
    manifest
        .files
        .iter()
        .filter(|file| file.file_name.as_deref().is_none_or(str::is_empty))
        .map(|file| {
            UnresolvedFile::curse(file.project_id, file.file_id, UnresolvedKind::NotResolved)
        })
        .collect()
}

/// Returns the report of a modpack of CurseForge that has not been completed yet.
pub fn report_of_curse(manifest: &CurseManifest) -> CompletionReport {
    let mut report = CompletionReport::new(DOWNLOAD_STAGE, manifest.files.len() as u64);
    for file in unresolved_curse_files(manifest) {
        report.unresolved(file);
    }
    report
}

/// Returns the report of a modpack of Modrinth that has not been completed yet.
///
/// Every file of a modpack of Modrinth names its own path and addresses, so
/// the launcher does not have to ask a repository; the report of such a modpack
/// is always complete.
pub fn report_of_modrinth(manifest: &ModrinthManifest) -> CompletionReport {
    CompletionReport::new(DOWNLOAD_STAGE, manifest.files.len() as u64)
}

/// Returns the file of a mod of Modrinth the launcher could not resolve.
impl UnresolvedFile {
    /// Returns the file of a mod of Modrinth the launcher could not resolve.
    pub fn modrinth(path: impl Into<String>, reason: UnresolvedKind) -> Self {
        Self {
            kind: "modrinth",
            name: Some(path.into()),
            project: None,
            file: None,
            reason,
        }
    }
}

/// Reads the keys of the optional files the player did not install.
///
/// The launcher keeps the keys in [`EXCLUDED_FILE`] under the root of the
/// instance. A file that is not there means the player installed every optional
/// file, which is what an instance of an older launcher is taken to mean.
///
/// Returns `None` when the file is not there or cannot be read: a list the
/// launcher cannot read must not make the install refuse the whole modpack.
pub fn read_excluded(root: &Path) -> Option<BTreeSet<String>> {
    let text = std::fs::read_to_string(root.join(EXCLUDED_FILE)).ok()?;
    let excluded: Vec<String> = serde_json::from_str(&text).ok()?;
    Some(excluded.into_iter().collect())
}

/// Writes the keys of the optional files the player did not install.
///
/// The launcher writes the list sorted, so the file of an instance does not
/// change when the player chooses the same files in another order. An empty
/// list removes the file, because an instance that installs every optional file
/// needs no list.
pub fn write_excluded(root: &Path, excluded: &BTreeSet<String>) -> std::io::Result<()> {
    let path = root.join(EXCLUDED_FILE);
    if excluded.is_empty() {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        };
    }

    let keys: Vec<&str> = excluded.iter().map(String::as_str).collect();
    let text = serde_json::to_string_pretty(&keys).map_err(std::io::Error::other)?;
    std::fs::create_dir_all(root)?;
    std::fs::write(path, text)
}

/// Whether a mod of the name is already in a directory of mods.
///
/// The launcher keeps a mod the player disabled next to the mod with the suffix
/// [`DISABLED_EXTENSION`], and a mod it replaced with the suffix
/// [`OLD_EXTENSION`]. Any of the four names means the mod is installed already,
/// which is the rule of `ModManager.hasSimpleMod`.
pub fn has_simple_mod(mods_directory: &Path, file_name: &str) -> bool {
    let without_disabled = file_name
        .strip_suffix(DISABLED_EXTENSION)
        .unwrap_or(file_name);
    let without_old = without_disabled
        .strip_suffix(OLD_EXTENSION)
        .unwrap_or(without_disabled);

    [
        file_name.to_string(),
        format!("{without_disabled}{DISABLED_EXTENSION}"),
        without_old.to_string(),
        format!("{without_old}{OLD_EXTENSION}"),
    ]
    .iter()
    .any(|name| mods_directory.join(name).exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpack::mcbbs::Addon;

    /// Returns a manifest with the files the test names.
    fn manifest(files: Vec<McbbsFile>) -> McbbsModpackManifest {
        McbbsModpackManifest {
            manifest_type: Some(crate::modpack::mcbbs::MANIFEST_TYPE.to_string()),
            name: "World".to_string(),
            version: "1.0".to_string(),
            file_api: Some("https://example.com/pack".to_string()),
            addons: Some(vec![Addon {
                id: "game".to_string(),
                version: "1.20.1".to_string(),
            }]),
            files: Some(files),
            ..McbbsModpackManifest::default()
        }
    }

    #[test]
    fn the_progress_counts_the_files() {
        let mut report = CompletionReport::new(DOWNLOAD_STAGE, 3);
        assert_eq!(report.progress(), (0, 3));

        report.advance();
        report.advance();
        assert_eq!(report.progress(), (2, 3));

        // The count does not go past the total.
        report.advance();
        report.advance();
        assert_eq!(report.progress(), (3, 3));
        assert!(report.summary().is_none());
    }

    #[test]
    fn the_files_the_launcher_could_not_resolve_are_reported() {
        let manifest = manifest(vec![
            McbbsFile::Addon {
                force: false,
                path: "config/a.toml".to_string(),
                hash: "abc".to_string(),
            },
            McbbsFile::Curse {
                force: false,
                project_id: 12,
                file_id: 34,
                file_name: Some("sodium.jar".to_string()),
                url: None,
            },
            McbbsFile::Curse {
                force: false,
                project_id: 56,
                file_id: 78,
                file_name: None,
                url: None,
            },
        ]);

        let unresolved = unresolved_files(&manifest);
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].project, Some(56));
        assert_eq!(unresolved[0].file, Some(78));
        assert_eq!(unresolved[0].describe(), "curse56/78: not resolved");

        let report = report_of(&manifest);
        assert_eq!(report.total, 3);
        assert!(!report.is_complete());
        let summary = report.summary().expect("the report should name the files");
        assert!(summary.contains("1 of its files"));
    }

    #[test]
    fn a_modpack_whose_files_are_all_known_is_complete() {
        let manifest = manifest(vec![McbbsFile::Curse {
            force: false,
            project_id: 12,
            file_id: 34,
            file_name: Some("sodium.jar".to_string()),
            url: Some("https://example.com/sodium.jar".to_string()),
        }]);

        let report = report_of(&manifest);
        assert!(report.is_complete());
        assert_eq!(report.unresolved.len(), 0);
    }

    #[test]
    fn a_file_the_launcher_could_not_resolve_names_the_reason() {
        let mut report = CompletionReport::new(DOWNLOAD_STAGE, 2);
        report.advance();
        report.unresolved(UnresolvedFile::curse(1, 2, UnresolvedKind::Deleted));
        report.warn("the repository answered with 404");

        assert_eq!(report.progress(), (1, 2));
        assert_eq!(report.summary().as_deref().map(str::to_string), {
            Some("The modpack was installed without 1 of its files: curse1/2: deleted".to_string())
        });
        assert_eq!(report.warnings, vec!["the repository answered with 404"]);
        assert_eq!(report.error(), Some(CompletionError::Deleted));
    }

    #[test]
    fn a_file_the_launcher_could_not_ask_about_is_not_a_deletion() {
        let mut report = CompletionReport::new(DOWNLOAD_STAGE, 1);
        assert_eq!(report.error(), None);

        report.unresolved(UnresolvedFile::curse(1, 2, UnresolvedKind::Unreachable));
        assert_eq!(report.error(), Some(CompletionError::Incomplete));

        report.unresolved = Vec::new();
        report.unresolved(UnresolvedFile::curse_named(
            1,
            2,
            "sodium.jar",
            UnresolvedKind::NotResolved,
        ));
        assert_eq!(report.error(), Some(CompletionError::Incomplete));
        assert_eq!(
            report.summary().as_deref(),
            Some("The modpack was installed without 1 of its files: sodium.jar: not resolved")
        );
    }

    #[test]
    fn the_files_the_player_did_not_install_are_kept_beside_the_instance() {
        let root = std::env::temp_dir().join(format!("hmcl-rs-excluded-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the root should be made");

        assert_eq!(read_excluded(&root), None);

        let excluded: BTreeSet<String> = [
            "curseforge:1:2".to_string(),
            "modrinth:a:mods/b.jar".to_string(),
        ]
        .into_iter()
        .collect();
        write_excluded(&root, &excluded).expect("the list should be written");
        assert_eq!(read_excluded(&root), Some(excluded.clone()));

        // An instance that installs every optional file needs no list.
        write_excluded(&root, &BTreeSet::new()).expect("the list should go");
        assert_eq!(read_excluded(&root), None);

        std::fs::write(root.join(EXCLUDED_FILE), "not a list").expect("the file should be written");
        assert_eq!(read_excluded(&root), None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_mod_the_player_disabled_or_replaced_is_the_mod_of_the_pack() {
        let mods = std::env::temp_dir().join(format!("hmcl-rs-mods-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mods);
        std::fs::create_dir_all(&mods).expect("the mods should be made");

        assert!(!has_simple_mod(&mods, "sodium.jar"));

        std::fs::write(mods.join("sodium.jar"), b"a mod").expect("the mod should be written");
        assert!(has_simple_mod(&mods, "sodium.jar"));
        assert!(has_simple_mod(&mods, "sodium.jar.disabled"));

        std::fs::remove_file(mods.join("sodium.jar")).expect("the mod should go");
        std::fs::write(mods.join("sodium.jar.old"), b"an older mod")
            .expect("the mod should be written");
        assert!(has_simple_mod(&mods, "sodium.jar"));
        assert!(has_simple_mod(&mods, "sodium.jar.old"));
        assert!(!has_simple_mod(&mods, "iris.jar"));

        let _ = std::fs::remove_dir_all(&mods);
    }

    #[test]
    fn the_reports_of_curse_and_modrinth_are_built() {
        let text = r#"{
            "manifestType": "minecraftModpack",
            "manifestVersion": 1,
            "name": "World",
            "version": "1.0",
            "author": "player",
            "overrides": "overrides",
            "minecraft": {"version": "1.20.1", "modLoaders": [{"id": "forge-47.2.0", "primary": true}]},
            "files": [
                {"projectID": 12, "fileID": 34567, "required": true},
                {"projectID": 56, "fileID": 78, "fileName": "sodium.jar", "url": "https://example.com/sodium.jar"}
            ]
        }"#;
        let curse = CurseManifest::read(text).expect("the manifest should be read");
        let report = report_of_curse(&curse);
        assert_eq!(report.total, 2);
        assert_eq!(report.unresolved.len(), 1);

        let modrinth_text = r#"{
            "game": "minecraft",
            "formatVersion": 1,
            "versionId": "1.0",
            "name": "World",
            "files": [{
                "path": "mods/a.jar",
                "hashes": {"sha1": "da39a3ee5e6b4b0d3255bfef95601890afd80709", "sha512": "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"},
                "downloads": ["https://example.com/file.jar"],
                "fileSize": 3
            }],
            "dependencies": {"minecraft": "1.20.1", "fabric-loader": "0.15.0"}
        }"#;
        let modrinth = ModrinthManifest::read(modrinth_text).expect("the manifest should be read");
        let report = report_of_modrinth(&modrinth);
        assert_eq!(report.total, 1);
        assert!(report.is_complete());
    }
}
