//! What an export of a modpack offers the player.
//!
//! The launcher can write the instance of the player as a modpack, and the
//! wizard that asks for the settings shows the files of the instance as a
//! tree. The tree is not the whole instance: the launcher keeps the files of
//! itself, the logs and the caches out of the modpack, because they are useless
//! to the player who installs the modpack. The list of the files the launcher
//! hides and the list of the files the wizard does not select by itself are the
//! ones the launcher of Java uses, so the two launchers export the same files.
//!
//! The lists are written as names the launcher of Java writes them: a name
//! without a prefix is the name of a file or of a directory, and a name with
//! the prefix `regex:` is a regular expression the whole path must match.

use std::sync::OnceLock;

use regex::Regex;

/// The names of the files and of the directories an export leaves out.
pub const MODPACK_BLACK_LIST: [&str; 68] = [
    "regex:(.*?)\\.log",
    "regex:.*\\.dat_old$",
    "regex:.*\\.old$",
    "regex:.*\\.BakaCoreInfo$",
    "regex:.*-natives",
    "usernamecache.json",
    "usercache.json",
    "launcher_profiles.json",
    "launcher.pack.lzma",
    "launcher_accounts.json",
    "launcher_cef_log.txt",
    "launcher_log.txt",
    "launcher_msa_credentials.bin",
    "launcher_settings.json",
    "launcher_ui_state.json",
    "realms_persistence.json",
    "webcache2",
    "treatment_tags.json",
    "clientId.txt",
    "PCL.ini",
    ".hmcl",
    "backup",
    "pack.json",
    "launcher.jar",
    "cache",
    "modpack.cfg",
    "log4j2.xml",
    "hmclversion.cfg",
    "instance-game-settings.json",
    "manifest.json",
    "minecraftinstance.json",
    ".curseclient",
    "modrinth.index.json",
    ".fabric",
    ".mixin.out",
    ".optifine",
    "jars",
    "logs",
    "versions",
    "assets",
    "libraries",
    "crash-reports",
    "NVIDIA",
    "AMD",
    "screenshots",
    "natives",
    "native",
    "$native",
    "$natives",
    "server-resource-packs",
    "command_history.txt",
    "downloads",
    "essential",
    "asm",
    "backups",
    "TCNodeTracker",
    "CustomDISkins",
    "data",
    "CustomSkinLoader/caches",
    "debug",
    ".replay_cache",
    "replay_recordings",
    "replay_videos",
    "irisUpdateInfo.json",
    "modernfix",
    "modtranslations",
    "journeymap/data",
    "mods/.connector",
];

/// The names of the files and of the directories an export keeps but does not
/// select by itself.
///
/// The files of a modpack that the player expects to keep are the saves, the
/// options and the settings of the mods; an export adds them only when the
/// player selects them, because they belong to the player instead of the
/// modpack.
pub const MODPACK_SUGGESTED_BLACK_LIST: [&str; 10] = [
    "fonts",
    "saves",
    "servers.dat",
    "options.txt",
    "blueprints",
    "optionsof.txt",
    "journeymap",
    "schematics",
    "optionsshaders.txt",
    "mods/VoxelMods",
];

/// What the wizard suggests of a file of the instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModSuggestion {
    /// The file belongs to the modpack and is selected by itself.
    Suggested,
    /// The file is kept but the player selects it.
    Normal,
    /// The file is left out of the modpack.
    Hidden,
}

/// A way a name of a list matches a path.
#[derive(Debug)]
enum Matcher {
    /// The path is the name.
    Exact(String),
    /// The whole path matches the regular expression.
    Pattern(Regex),
    /// A pattern the launcher cannot read, which matches no path.
    Never,
}

impl Matcher {
    /// Reads a name of a list.
    ///
    /// A pattern the launcher of Java would accept but the regular expressions
    /// of Rust cannot read matches no path instead of failing the export: the
    /// launcher knows the lists, and a pattern that does not compile is a
    /// mistake of the launcher instead of a reason to refuse the instance.
    fn new(name: &str) -> Self {
        let Some(pattern) = name.strip_prefix("regex:") else {
            return Self::Exact(name.to_string());
        };

        // The launcher of Java matches the whole path, while the regular
        // expressions of Rust match a part of it, so the pattern is anchored.
        match Regex::new(&format!("^(?:{pattern})$")) {
            Ok(pattern) => Self::Pattern(pattern),
            Err(_) => Self::Never,
        }
    }
}

/// The lists an export uses.
#[derive(Debug)]
pub struct ModAdviser {
    /// The names of the files and of the directories an export leaves out.
    black_list: Vec<Matcher>,
    /// The names of the files and of the directories an export keeps but does
    /// not select by itself.
    suggested_black_list: Vec<Matcher>,
}

impl ModAdviser {
    /// Creates an adviser with the lists of the launcher.
    pub fn standard() -> &'static Self {
        static ADVISER: OnceLock<ModAdviser> = OnceLock::new();
        ADVISER.get_or_init(|| Self::new(&MODPACK_BLACK_LIST, &MODPACK_SUGGESTED_BLACK_LIST))
    }

    /// Creates an adviser with the lists an export uses.
    ///
    /// The names of the lists are read like the names of the lists of the
    /// launcher, so a caller adds the files of the instance itself with the
    /// same shape as the files the launcher knows: a name of a file, a name of
    /// a directory or a pattern with the prefix `regex:`.
    pub fn new(black_list: &[&str], suggested_black_list: &[&str]) -> Self {
        Self {
            black_list: black_list.iter().map(|name| Matcher::new(name)).collect(),
            suggested_black_list: suggested_black_list
                .iter()
                .map(|name| Matcher::new(name))
                .collect(),
        }
    }

    /// Returns what the wizard suggests of a path.
    ///
    /// The path of a directory ends with `/`; the path of a file does not. The
    /// paths are relative to the run directory of the instance and use `/` as
    /// the separator, whichever system the launcher runs on.
    pub fn advise(&self, path: &str, is_directory: bool) -> ModSuggestion {
        if matches(&self.black_list, path, is_directory) {
            ModSuggestion::Hidden
        } else if matches(&self.suggested_black_list, path, is_directory) {
            ModSuggestion::Normal
        } else {
            ModSuggestion::Suggested
        }
    }
}

/// Returns whether a path is named by a list.
///
/// A directory is named by a name it is under, so the name `logs` hides the
/// directory `logs` and everything under it; a file is named only by its exact
/// path, unless the name is a pattern.
fn matches(list: &[Matcher], path: &str, is_directory: bool) -> bool {
    // The path of a directory always ends with the separator, so the names of
    // the lists compare against the path with it.
    let directory_path = if is_directory {
        let mut path = path.to_string();
        if !path.ends_with('/') {
            path.push('/');
        }
        path
    } else {
        path.to_string()
    };

    list.iter().any(|matcher| match matcher {
        // A name that is not a pattern names a directory and everything under
        // it; the pattern of a directory is never compared, which the launcher
        // of Java does not do either.
        Matcher::Exact(name) => {
            if is_directory {
                directory_path.starts_with(name.as_str())
                    && directory_path.as_bytes().get(name.len()) == Some(&b'/')
            } else {
                directory_path == *name
            }
        }
        Matcher::Pattern(pattern) => !is_directory && pattern.is_match(&directory_path),
        Matcher::Never => false,
    })
}

/// Returns what the launcher suggests of a path, using the lists of the
/// launcher.
pub fn suggest_mod(path: &str, is_directory: bool) -> ModSuggestion {
    ModAdviser::standard().advise(path, is_directory)
}

/// Returns whether an export writes a path.
///
/// The files of the whitelist are the files the player selected in the wizard;
/// an empty whitelist keeps every file the black list does not hide, and a
/// whitelist keeps only the files it names. The paths are the ones of the
/// wizard: relative to the run directory, with `/` as the separator.
///
/// A path under a hidden directory is refused as well, so an export that lists
/// the files without walking the directories still leaves out the files of the
/// launcher instead of writing them into the archive.
pub fn accept_file(path: &str, black_list: &[&str], white_list: &[String]) -> bool {
    // The root of the instance is always written, because the archive needs it.
    if path.is_empty() {
        return true;
    }

    let matchers: Vec<Matcher> = black_list.iter().map(|name| Matcher::new(name)).collect();
    if matches(&matchers, path, false) {
        return false;
    }

    // Each directory the path is under is named by the lists as well.
    for (index, _) in path.match_indices('/') {
        if matches(&matchers, &path[..index], true) {
            return false;
        }
    }

    if white_list.is_empty() {
        return true;
    }

    white_list.iter().any(|name| name == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_of_the_launcher_are_hidden() {
        assert_eq!(suggest_mod("logs/latest.log", false), ModSuggestion::Hidden);
        assert_eq!(suggest_mod(".hmcl", true), ModSuggestion::Hidden);
        assert_eq!(
            suggest_mod("launcher_profiles.json", false),
            ModSuggestion::Hidden
        );
        // A pattern of the list names a file; the wizard hides the directory of
        // the natives of the instance itself, which the list cannot do.
        assert_eq!(suggest_mod("mods-natives", false), ModSuggestion::Hidden);
        assert_eq!(suggest_mod("mods-natives/", true), ModSuggestion::Suggested);
        assert_eq!(suggest_mod("level.dat_old", false), ModSuggestion::Hidden);
        assert_eq!(
            suggest_mod("mods/BakaCoreInfo.BakaCoreInfo", false),
            ModSuggestion::Hidden
        );
    }

    #[test]
    fn a_directory_is_hidden_with_everything_under_it() {
        assert_eq!(suggest_mod("libraries/", true), ModSuggestion::Hidden);
        assert_eq!(suggest_mod("journeymap/data/", true), ModSuggestion::Hidden);
        assert_eq!(suggest_mod("mods/.connector/", true), ModSuggestion::Hidden);

        // The name of a directory does not hide a file of the same name.
        assert_eq!(suggest_mod("cache", false), ModSuggestion::Hidden);
        assert_eq!(suggest_mod("mods", true), ModSuggestion::Suggested);
    }

    #[test]
    fn the_files_of_the_player_are_kept_but_not_selected() {
        assert_eq!(suggest_mod("saves/", true), ModSuggestion::Normal);
        assert_eq!(suggest_mod("options.txt", false), ModSuggestion::Normal);
        assert_eq!(suggest_mod("servers.dat", false), ModSuggestion::Normal);
        assert_eq!(suggest_mod("mods/VoxelMods/", true), ModSuggestion::Normal);
        assert_eq!(suggest_mod("journeymap/", true), ModSuggestion::Normal);
    }

    #[test]
    fn the_files_of_the_modpack_are_selected() {
        assert_eq!(
            suggest_mod("mods/sodium.jar", false),
            ModSuggestion::Suggested
        );
        assert_eq!(suggest_mod("config/", true), ModSuggestion::Suggested);
        assert_eq!(
            suggest_mod("resourcepacks/pack.zip", false),
            ModSuggestion::Suggested
        );
    }

    #[test]
    fn a_pattern_matches_the_whole_path() {
        assert_eq!(suggest_mod("log", false), ModSuggestion::Suggested);
        assert_eq!(suggest_mod("crash.log", false), ModSuggestion::Hidden);
        assert_eq!(suggest_mod("logs/crash.log", false), ModSuggestion::Hidden);
        assert_eq!(
            suggest_mod("saves/world/level.dat", false),
            ModSuggestion::Suggested
        );
    }

    #[test]
    fn a_whitelist_keeps_only_the_files_it_names() {
        let black_list = ["logs", "session.lock"];
        assert!(accept_file("mods/sodium.jar", &black_list, &[]));
        assert!(!accept_file("logs/latest.log", &black_list, &[]));
        assert!(accept_file("", &black_list, &[]));

        let white_list = vec!["options.txt".to_string()];
        assert!(accept_file("options.txt", &black_list, &white_list));
        assert!(!accept_file("mods/sodium.jar", &black_list, &white_list));
        assert!(!accept_file("logs/latest.log", &black_list, &white_list));
    }

    #[test]
    fn an_adviser_of_an_export_adds_the_files_of_the_instance() {
        let adviser = ModAdviser::new(&["regex:.*\\.jar", "mods"], &["config/a.toml"]);
        assert_eq!(adviser.advise("cache.jar", false), ModSuggestion::Hidden);
        assert_eq!(adviser.advise("mods/", true), ModSuggestion::Hidden);
        assert_eq!(
            adviser.advise("config/a.toml", false),
            ModSuggestion::Normal
        );
        // The name of a directory of the export does not hide a file.
        assert_eq!(adviser.advise("modsx/", true), ModSuggestion::Suggested);
    }
}
