//! The addons the repositories of the launcher offer.
//!
//! The repositories offer mods, modpacks, resource packs, shaders and worlds;
//! the launcher reads the list of the addons and the files of their versions
//! from the repositories.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::addon::mod_loader::ModLoaderType;

/// The repository an addon comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddonSource {
    /// The repository of CurseForge.
    CurseForge,
    /// The repository of Modrinth.
    Modrinth,
}

impl AddonSource {
    /// Returns the name of the repository, as the launcher writes it.
    pub const fn id(self) -> &'static str {
        match self {
            Self::CurseForge => "curseforge",
            Self::Modrinth => "modrinth",
        }
    }
}

impl fmt::Display for AddonSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id())
    }
}

/// The kind of an addon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddonType {
    /// A mod of the game.
    Mod,
    /// A pack that holds mods, configurations and scripts.
    Modpack,
    /// A pack that changes the looks of the game.
    ResourcePack,
    /// A pack that changes how the game is drawn.
    ShaderPack,
    /// A world.
    World,
    /// A pack that changes the texts and the menus of the launcher.
    Customization,
}

impl AddonType {
    /// Returns the name of the kind, as the launcher writes it.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Modpack => "modpack",
            Self::ResourcePack => "resource-pack",
            Self::ShaderPack => "shader-pack",
            Self::World => "world",
            Self::Customization => "customization",
        }
    }
}

/// How finished a version of an addon is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionType {
    /// A finished version.
    #[default]
    Release,
    /// A version that is tested by the author.
    Beta,
    /// A version that is not tested.
    Alpha,
}

/// A mod loader an addon works with.
///
/// The repositories name the loaders with their own identifiers, so the model
/// keeps the name as it was read.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AddonLoader(String);

impl AddonLoader {
    /// The loader of Forge.
    pub const FORGE: &'static str = "forge";
    /// The loader of NeoForge.
    pub const NEO_FORGE: &'static str = "neoforge";
    /// The loader of Fabric.
    pub const FABRIC: &'static str = "fabric";
    /// The loader of Quilt.
    pub const QUILT: &'static str = "quilt";
    /// The loader of LiteLoader.
    pub const LITE_LOADER: &'static str = "liteloader";

    /// Creates a loader with its name.
    pub fn of(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Returns the name of the loader.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns whether the name is the name of a loader the launcher knows.
    pub fn might_be_loader(name: &str) -> bool {
        matches!(
            name,
            Self::FORGE | Self::NEO_FORGE | Self::FABRIC | Self::QUILT | Self::LITE_LOADER
        )
    }
}

impl fmt::Display for AddonLoader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// How an addon needs another addon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    /// The addon does not work without the other addon.
    #[default]
    Required,
    /// The addon works better with the other addon.
    Optional,
    /// The other addon is part of the file of the addon.
    Embedded,
    /// The two addons do not work together.
    Incompatible,
    /// The launcher could not read the dependency.
    Broken,
}

/// An addon another addon needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    /// How the addon needs the other addon.
    #[serde(rename = "type")]
    pub dependency_type: DependencyType,
    /// The repository the other addon is found in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<AddonSource>,
    /// The identifier of the other addon in its repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Dependency {
    /// Creates a dependency of an addon.
    pub fn new(
        dependency_type: DependencyType,
        source: AddonSource,
        id: impl Into<String>,
    ) -> Self {
        Self {
            dependency_type,
            source: Some(source),
            id: Some(id.into()),
        }
    }

    /// Creates the dependency the launcher could not read.
    pub const fn broken() -> Self {
        Self {
            dependency_type: DependencyType::Broken,
            source: None,
            id: None,
        }
    }
}

/// A file of a version of an addon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonFile {
    /// The digests of the file, by the names of the algorithms.
    #[serde(default)]
    pub hashes: BTreeMap<String, String>,
    /// The address the file is downloaded from.
    pub url: String,
    /// The name of the file.
    pub filename: String,
}

impl AddonFile {
    /// Returns the algorithm and the digest the file is checked with.
    ///
    /// The strongest digest the repository offers is used; the file is not
    /// checked when the repository offers no digest.
    pub fn integrity_check(&self) -> Option<(&'static str, &str)> {
        for algorithm in ["sha512", "sha256", "sha1", "md5"] {
            if let Some(digest) = self.hashes.get(algorithm) {
                return Some((algorithm, digest));
            }
        }
        None
    }
}

/// A version of an addon in a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonVersion {
    /// The repository the version comes from.
    pub source: AddonSource,
    /// The identifier of the version in its repository.
    pub version_id: String,
    /// The identifier of the addon in its repository.
    pub project_id: String,
    /// The name of the version.
    pub name: String,
    /// The number of the version.
    pub version: String,
    /// The moment the version was published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_published: Option<String>,
    /// How finished the version is.
    pub version_type: VersionType,
    /// The file of the version.
    pub file: AddonFile,
    /// The addons the version needs.
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    /// The versions of the game the version works with.
    #[serde(default)]
    pub game_versions: Vec<String>,
    /// The loaders the version works with.
    #[serde(default)]
    pub loaders: Vec<AddonLoader>,
}

/// An addon a repository offers.
///
/// The versions of the addon and the dependencies of its versions are read
/// when the user asks for them, so the addon itself only holds what the list
/// of the repository shows.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoteAddon {
    /// The name of the addon in its repository.
    pub slug: String,
    /// The author of the addon.
    pub author: String,
    /// The title of the addon.
    pub title: String,
    /// The description of the addon.
    pub description: String,
    /// The categories of the addon.
    #[serde(default)]
    pub categories: Vec<String>,
    /// The address of the page of the addon in its repository.
    pub page_url: String,
    /// The address of the icon of the addon.
    pub icon_url: String,
    /// The kind of the addon, if the repository tells it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<AddonType>,
}

impl RemoteAddon {
    /// Returns whether the launcher could not read the addon.
    ///
    /// The repositories sometimes offer an addon the launcher cannot read; the
    /// launcher shows the addon so the dependency may still be resolved.
    pub fn is_broken(&self) -> bool {
        self.slug.is_empty() && self.page_url.is_empty()
    }
}

/// An update of a file the player has installed.
///
/// The update names the version the file is now and the version it can be
/// replaced with, so the page of the updates shows both without asking the
/// repository again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonUpdate {
    /// The version the file of the player is.
    pub current: AddonVersion,
    /// The version the file can be replaced with.
    pub target: AddonVersion,
}

/// The updates the launcher found for the addons of an instance.
///
/// The launcher of Java keeps two lists: the newest version of every addon
/// whatever channel it is from, and the newest finished version of the addons
/// that have one. A player who only wants finished versions picks the second
/// list; the first list is what the page of the updates shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddonUpdates {
    /// The newest version of every addon, whatever channel it is from.
    pub common: Vec<AddonUpdate>,
    /// The newest finished version of the addons that have one.
    pub releases: Vec<AddonUpdate>,
}

impl AddonUpdates {
    /// Returns whether the launcher found no update at all.
    pub fn is_empty(&self) -> bool {
        self.common.is_empty()
    }
}

/// A condition a version has to satisfy to become the target of an update.
///
/// The launcher of Java lets a local addon file name its own conditions, for a
/// mod is only replaced by a version that is written for the loader of the
/// mod, while a resource pack archive accepts every version. The conditions are
/// listed here rather than given as functions, so a caller can tell them apart
/// and show them to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateRestriction {
    /// The version has to be written for the loader.
    Loader(ModLoaderType),
}

impl UpdateRestriction {
    /// Returns the condition that a version is written for a loader.
    pub const fn loader(loader: ModLoaderType) -> Self {
        Self::Loader(loader)
    }

    /// Returns whether a version satisfies the condition.
    pub fn accepts(&self, version: &AddonVersion) -> bool {
        match self {
            Self::Loader(loader) => version
                .loaders
                .iter()
                .filter_map(|name| ModLoaderType::from_addon_loader(name.as_str()))
                .any(|named| named == *loader),
        }
    }
}

/// Returns the update of a version the player has to the newest version a
/// repository offers.
///
/// The launcher of Java only offers a version the game of the instance runs:
/// a version of another game version is not an update, for the player would
/// have to change the game to use it. A version of the same game version is
/// an update only when it was published after the version the player has, so
/// the launcher does not offer a file the player already has; the versions of
/// the other game versions are all offered, for the player upgrades the game
/// at the same time.
///
/// The newest version of the repository is the update of every channel; the
/// newest finished version is the update of the players who only want
/// finished versions, and is `None` when the repository offers none.
pub fn find_update(
    game_version: &str,
    current: &AddonVersion,
    versions: impl IntoIterator<Item = AddonVersion>,
) -> Option<(AddonUpdate, Option<AddonUpdate>)> {
    check_updates(game_version, current, versions, &[])
}

/// Returns the update of a local addon file to the newest version a repository
/// offers that satisfies every restriction.
///
/// The restrictions are the ones the local file needs: a mod needs a version
/// that is written for the loader of the mod, while a resource pack archive is
/// loaded whatever version it was written for. A file without restrictions
/// accepts every version the game version accepts.
///
/// The launcher of Java only offers a version the game of the instance runs:
/// a version of another game version is not an update, for the player would
/// have to change the game to use it. A version of the same game version is an
/// update only when it was published after the version the player has, so the
/// launcher does not offer a file the player already has; the versions of the
/// other game versions are all offered, for the player upgrades the game at the
/// same time.
///
/// The newest version of the repository is the update of every channel; the
/// newest finished version is the update of the players who only want finished
/// versions, and is `None` when the repository offers none.
pub fn check_updates(
    game_version: &str,
    current: &AddonVersion,
    versions: impl IntoIterator<Item = AddonVersion>,
    restrictions: &[UpdateRestriction],
) -> Option<(AddonUpdate, Option<AddonUpdate>)> {
    let same_game_version = current
        .game_versions
        .iter()
        .any(|version| version == game_version);

    let mut candidates: Vec<AddonVersion> = versions
        .into_iter()
        .filter(|version| {
            version
                .game_versions
                .iter()
                .any(|name| name == game_version)
        })
        .filter(|version| {
            // A version of the same game version is an update only when it is
            // newer than the file of the player.
            if !same_game_version {
                return true;
            }
            match (&version.date_published, &current.date_published) {
                (Some(newer), Some(current)) => newer.as_str() > current.as_str(),
                // A version the repository dates none is offered, for the
                // launcher cannot tell it apart from an update.
                _ => true,
            }
        })
        .filter(|version| restrictions.iter().all(|rule| rule.accepts(version)))
        .collect();

    // The repository names the dates the way ISO 8601 does, which sorts the
    // versions the way the launcher of Java sorts them.
    candidates.sort_by(|left, right| right.date_published.cmp(&left.date_published));

    let target = candidates.first()?.clone();
    let release = candidates
        .iter()
        .find(|version| version.version_type == VersionType::Release)
        .cloned()
        .map(|release| AddonUpdate {
            current: current.clone(),
            target: release,
        });

    Some((
        AddonUpdate {
            current: current.clone(),
            target,
        },
        release,
    ))
}

/// Returns the update of a local addon file over the repositories the launcher
/// knows.
///
/// Every repository is asked on its own, because the repositories do not know
/// the versions of each other. The newest version of all the repositories is
/// the update of every channel, and the newest finished version of all the
/// repositories is the update of the players who only want finished versions.
/// The two are chosen apart, so a repository that offers the newest finished
/// version of the file wins the finished channel even when another repository
/// offers a newer beta version.
///
/// A file that has no update of any channel in any repository has no update at
/// all, which is the rule of the launcher of Java: a file whose finished
/// channel has no update is not offered, even when its common channel has one.
pub fn check_update_of_sources(
    game_version: &str,
    current: &AddonVersion,
    restrictions: &[UpdateRestriction],
    versions_by_source: impl IntoIterator<Item = Vec<AddonVersion>>,
) -> Option<(AddonUpdate, Option<AddonUpdate>)> {
    let mut candidate: Option<AddonUpdate> = None;
    let mut candidate_release: Option<AddonUpdate> = None;

    for versions in versions_by_source {
        let Some((update, release)) = check_updates(game_version, current, versions, restrictions)
        else {
            continue;
        };

        // The repositories date their versions the way ISO 8601 does, so the
        // later date is the newer version.
        if candidate
            .as_ref()
            .map(|found| is_later(&update.target, &found.target))
            .unwrap_or(true)
        {
            candidate = Some(update);
        }

        if let Some(release) = release {
            if candidate_release
                .as_ref()
                .map(|found| is_later(&release.target, &found.target))
                .unwrap_or(true)
            {
                candidate_release = Some(release);
            }
        }
    }

    Some((candidate?, candidate_release))
}

/// Returns whether a version was published after another.
///
/// A version the repository dates none is older than a version it dates, for
/// the launcher cannot tell it apart from an old version.
fn is_later(version: &AddonVersion, other: &AddonVersion) -> bool {
    match (&version.date_published, &other.date_published) {
        (Some(later), Some(earlier)) => later.as_str() > earlier.as_str(),
        (Some(_), None) => true,
        (None, _) => false,
    }
}

/// Returns the updates of the addons of an instance, in the order the addons
/// are given.
///
/// The launcher of Java asks every repository for the versions of every file
/// and keeps the newest of them; this function does the same with the
/// versions the caller read, so the network stays in the caller and the
/// choice stays here.
pub fn updates_of(
    game_version: &str,
    installed: impl IntoIterator<Item = (AddonVersion, Vec<AddonVersion>)>,
) -> AddonUpdates {
    updates_of_with_restrictions(
        game_version,
        installed
            .into_iter()
            .map(|(current, versions)| (current, versions, Vec::new())),
    )
}

/// Returns the updates of the addons of an instance, with the restrictions
/// that every file names.
///
/// The caller pairs every version of the player with the versions a repository
/// offers, and with the conditions the local file of the version needs. The
/// conditions are the same for all the repositories of a file here, because a
/// mod is written for one loader whatever repository it comes from.
pub fn updates_of_with_restrictions(
    game_version: &str,
    installed: impl IntoIterator<Item = (AddonVersion, Vec<AddonVersion>, Vec<UpdateRestriction>)>,
) -> AddonUpdates {
    let mut updates = AddonUpdates::default();

    for (current, versions, restrictions) in installed {
        if let Some((update, release)) =
            check_updates(game_version, &current, versions, &restrictions)
        {
            updates.common.push(update);
            if let Some(release) = release {
                updates.releases.push(release);
            }
        }
    }

    updates
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDON: &str = r#"{
        "slug": "sodium",
        "author": "CaffeineMC",
        "title": "Sodium",
        "description": "A modern rendering engine",
        "categories": [ "optimization" ],
        "page_url": "https://modrinth.com/mod/sodium",
        "icon_url": "https://cdn.modrinth.com/data/AANobbMI/icon.png",
        "kind": "mod"
    }"#;

    const VERSION: &str = r#"{
        "source": "modrinth",
        "version_id": "abc123",
        "project_id": "AANobbMI",
        "name": "Sodium 0.5.8",
        "version": "0.5.8",
        "date_published": "2024-04-01T12:00:00Z",
        "version_type": "release",
        "file": {
            "hashes": {
                "sha1": "1111111111111111111111111111111111111111",
                "sha512": "2222"
            },
            "url": "https://cdn.modrinth.com/data/AANobbMI/versions/abc123/sodium.jar",
            "filename": "sodium-0.5.8.jar"
        },
        "dependencies": [
            { "type": "required", "source": "modrinth", "id": "P7dR8mSH" }
        ],
        "game_versions": [ "1.20.1" ],
        "loaders": [ "fabric" ]
    }"#;

    #[test]
    fn the_addons_are_read() {
        let addon: RemoteAddon = serde_json::from_str(ADDON).expect("the addon should be read");

        assert_eq!(addon.slug, "sodium");
        assert_eq!(addon.author, "CaffeineMC");
        assert_eq!(addon.title, "Sodium");
        assert_eq!(addon.categories, vec!["optimization"]);
        assert_eq!(addon.kind, Some(AddonType::Mod));
        assert!(!addon.is_broken());
    }

    #[test]
    fn the_versions_of_the_addons_are_read() {
        let version: AddonVersion =
            serde_json::from_str(VERSION).expect("the version should be read");

        assert_eq!(version.source, AddonSource::Modrinth);
        assert_eq!(version.version_type, VersionType::Release);
        assert_eq!(version.loaders, vec![AddonLoader::of("fabric")]);
        assert_eq!(version.dependencies.len(), 1);
        assert_eq!(
            version.dependencies[0].dependency_type,
            DependencyType::Required
        );
        assert_eq!(version.file.filename, "sodium-0.5.8.jar");
        assert_eq!(version.file.integrity_check(), Some(("sha512", "2222")));
    }

    #[test]
    fn the_files_are_checked_with_the_strongest_digest() {
        let mut file = AddonFile {
            hashes: BTreeMap::new(),
            url: String::new(),
            filename: String::new(),
        };
        assert_eq!(file.integrity_check(), None);

        file.hashes.insert("md5".to_string(), "aaa".to_string());
        assert_eq!(file.integrity_check(), Some(("md5", "aaa")));

        file.hashes.insert("sha1".to_string(), "bbb".to_string());
        assert_eq!(file.integrity_check(), Some(("sha1", "bbb")));

        file.hashes.insert("sha256".to_string(), "ccc".to_string());
        assert_eq!(file.integrity_check(), Some(("sha256", "ccc")));
    }

    #[test]
    fn the_dependencies_are_named() {
        let dependency = Dependency::new(DependencyType::Required, AddonSource::CurseForge, "123");
        assert_eq!(dependency.source, Some(AddonSource::CurseForge));
        assert_eq!(dependency.id.as_deref(), Some("123"));

        let broken = Dependency::broken();
        assert_eq!(broken.dependency_type, DependencyType::Broken);
        assert_eq!(broken.id, None);
    }

    #[test]
    fn the_names_of_the_kinds_are_written() {
        assert_eq!(AddonType::ResourcePack.id(), "resource-pack");
        assert_eq!(AddonSource::CurseForge.id(), "curseforge");
        assert!(AddonLoader::might_be_loader("fabric"));
        assert!(!AddonLoader::might_be_loader("sodium"));
    }

    /// Returns a version of an addon for the tests.
    fn version(
        number: &str,
        date: Option<&str>,
        version_type: VersionType,
        game_versions: &[&str],
    ) -> AddonVersion {
        AddonVersion {
            source: AddonSource::Modrinth,
            version_id: number.to_string(),
            project_id: "AANobbMI".to_string(),
            name: number.to_string(),
            version: number.to_string(),
            date_published: date.map(str::to_string),
            version_type,
            file: AddonFile {
                hashes: BTreeMap::new(),
                url: String::new(),
                filename: format!("sodium-{number}.jar"),
            },
            dependencies: Vec::new(),
            game_versions: game_versions.iter().map(|name| name.to_string()).collect(),
            loaders: vec![AddonLoader::of("fabric")],
        }
    }

    #[test]
    fn the_newest_version_of_the_game_version_is_the_update() {
        let current = version(
            "0.5.8",
            Some("2024-04-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );
        let versions = vec![
            version(
                "0.5.9",
                Some("2024-05-01T12:00:00Z"),
                VersionType::Release,
                &["1.20.1"],
            ),
            version(
                "0.6.0",
                Some("2024-06-01T12:00:00Z"),
                VersionType::Beta,
                &["1.20.1"],
            ),
        ];

        let (update, release) =
            find_update("1.20.1", &current, versions).expect("an update should be found");

        // The newest version is the beta one; the release channel keeps the
        // newest finished one.
        assert_eq!(update.target.version, "0.6.0");
        assert_eq!(
            release.expect("a release should be found").target.version,
            "0.5.9"
        );
    }

    #[test]
    fn a_version_of_another_game_version_is_no_update() {
        let current = version(
            "0.5.8",
            Some("2024-04-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );
        let versions = vec![version(
            "0.6.0",
            Some("2024-06-01T12:00:00Z"),
            VersionType::Release,
            &["1.21"],
        )];

        assert_eq!(find_update("1.20.1", &current, versions), None);
    }

    #[test]
    fn a_version_the_player_has_is_no_update() {
        let current = version(
            "0.5.8",
            Some("2024-04-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );
        // The repository offers the file the player has and an older one.
        let versions = vec![
            version(
                "0.5.8",
                Some("2024-04-01T12:00:00Z"),
                VersionType::Release,
                &["1.20.1"],
            ),
            version(
                "0.5.7",
                Some("2024-03-01T12:00:00Z"),
                VersionType::Release,
                &["1.20.1"],
            ),
        ];

        assert_eq!(find_update("1.20.1", &current, versions), None);
    }

    #[test]
    fn a_file_of_another_game_version_is_upgraded() {
        // The player has a file of another game version, which means the
        // player upgraded the game; every version of the new game version is
        // an update, whatever its date.
        let current = version(
            "0.5.8",
            Some("2024-04-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );
        let versions = vec![version(
            "0.5.1",
            Some("2024-01-01T12:00:00Z"),
            VersionType::Release,
            &["1.21"],
        )];

        let (update, _) =
            find_update("1.21", &current, versions).expect("an update should be found");
        assert_eq!(update.target.version, "0.5.1");
    }

    #[test]
    fn the_updates_of_an_instance_are_kept_by_channel() {
        let installed = version(
            "1.0",
            Some("2024-01-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );
        // The first addon offers a finished version and a newer beta one.
        let finished = version(
            "1.5",
            Some("2024-02-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );
        let beta = version(
            "2.0",
            Some("2024-03-01T12:00:00Z"),
            VersionType::Beta,
            &["1.20.1"],
        );
        // The second addon offers only a beta version.
        let only_beta = version(
            "3.0",
            Some("2024-04-01T12:00:00Z"),
            VersionType::Beta,
            &["1.20.1"],
        );

        let updates = updates_of(
            "1.20.1",
            vec![
                (installed.clone(), vec![finished.clone(), beta]),
                (installed, vec![only_beta]),
            ],
        );

        // Both addons have an update of some channel.
        assert_eq!(updates.common.len(), 2);
        assert_eq!(updates.common[0].target.version, "2.0");
        assert_eq!(updates.common[1].target.version, "3.0");

        // Only the first addon has a finished version to offer.
        assert_eq!(updates.releases.len(), 1);
        assert_eq!(updates.releases[0].target.version, "1.5");
        assert!(!updates.is_empty());
    }

    #[test]
    fn an_instance_with_no_update_has_no_updates() {
        let current = version(
            "1.0",
            Some("2024-01-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );

        let updates = updates_of("1.20.1", vec![(current.clone(), vec![current])]);

        assert!(updates.is_empty());
        assert!(updates.common.is_empty());
        assert!(updates.releases.is_empty());
    }

    /// Returns a version that is written for a loader.
    fn version_of_loader(
        number: &str,
        date: &str,
        version_type: VersionType,
        loader: &str,
    ) -> AddonVersion {
        let mut version = version(number, Some(date), version_type, &["1.20.1"]);
        version.loaders = vec![AddonLoader::of(loader)];
        version
    }

    #[test]
    fn a_version_of_another_loader_is_no_update_of_a_mod() {
        let current =
            version_of_loader("1.0", "2024-01-01T12:00:00Z", VersionType::Release, "forge");
        let fabric = version_of_loader(
            "2.0",
            "2024-02-01T12:00:00Z",
            VersionType::Release,
            "fabric",
        );
        let forge = version_of_loader("1.5", "2024-03-01T12:00:00Z", VersionType::Release, "forge");

        let update = check_updates(
            "1.20.1",
            &current,
            vec![fabric, forge],
            &[UpdateRestriction::loader(ModLoaderType::Forge)],
        )
        .expect("an update should be found");

        // The newer file of another loader is skipped, even when the player
        // installed a loader the launcher of Java knows.
        assert_eq!(update.0.target.version, "1.5");
    }

    #[test]
    fn a_resource_pack_accepts_a_version_of_any_loader() {
        let current = version_of_loader(
            "1.0",
            "2024-01-01T12:00:00Z",
            VersionType::Release,
            "fabric",
        );
        let forge = version_of_loader("2.0", "2024-02-01T12:00:00Z", VersionType::Release, "forge");

        // A resource pack archive names no condition at all.
        let update =
            check_updates("1.20.1", &current, vec![forge], &[]).expect("an update should be found");

        assert_eq!(update.0.target.version, "2.0");
    }

    #[test]
    fn the_newest_version_of_all_the_repositories_is_the_update() {
        let current = version(
            "1.0",
            Some("2024-01-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );

        // The repository of CurseForge offers a beta version, the repository
        // of Modrinth a finished one that was published earlier.
        let curseforge = vec![version(
            "3.0",
            Some("2024-04-01T12:00:00Z"),
            VersionType::Beta,
            &["1.20.1"],
        )];
        let modrinth = vec![version(
            "2.0",
            Some("2024-03-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        )];

        let (update, release) =
            check_update_of_sources("1.20.1", &current, &[], vec![curseforge, modrinth])
                .expect("an update should be found");

        // The channels are chosen apart: the beta version wins the common
        // channel and the finished version wins the finished channel.
        assert_eq!(update.target.version, "3.0");
        assert_eq!(
            release.expect("a release should be found").target.version,
            "2.0"
        );
    }

    #[test]
    fn a_file_that_no_repository_offers_an_update_for_has_no_update() {
        let current = version(
            "1.0",
            Some("2024-01-01T12:00:00Z"),
            VersionType::Release,
            &["1.20.1"],
        );
        // Both repositories only offer the file the player has.
        let offered = vec![current.clone()];

        assert!(
            check_update_of_sources("1.20.1", &current, &[], vec![offered.clone(), offered])
                .is_none()
        );
    }

    #[test]
    fn the_restrictions_of_the_files_are_applied() {
        let current = version_of_loader(
            "1.0",
            "2024-01-01T12:00:00Z",
            VersionType::Release,
            "fabric",
        );
        let fabric = version_of_loader(
            "3.0",
            "2024-04-01T12:00:00Z",
            VersionType::Release,
            "fabric",
        );
        let forge = version_of_loader("4.0", "2024-05-01T12:00:00Z", VersionType::Release, "forge");

        let updates = updates_of_with_restrictions(
            "1.20.1",
            vec![(
                current,
                vec![fabric, forge],
                vec![UpdateRestriction::loader(ModLoaderType::Fabric)],
            )],
        );

        assert_eq!(updates.common.len(), 1);
        assert_eq!(updates.common[0].target.version, "3.0");
    }
}
