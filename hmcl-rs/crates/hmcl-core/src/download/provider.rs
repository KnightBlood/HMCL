//! The servers the launcher downloads the files of the game from.
//!
//! The files of the game are published by the services of Mojang, but a player
//! in the mainland of China reaches them slowly. The launcher therefore knows a
//! mirror that holds the same files, and chooses the address it asks first from
//! the place the player is and from the source the player chose. Every address
//! is returned with a fallback, so a download that fails on one server is tried
//! on the other.

use std::fmt;

use super::task::{DownloadCandidate, DownloadCandidates};

/// The root the mirror of the launcher serves its files from.
pub const DEFAULT_BMCLAPI_ROOT: &str = "https://bmclapi2.bangbang93.com";

/// The address the list of the versions of the game is published at.
pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest.json";

/// The address the versions of the loader of Fabric are listed at.
pub const FABRIC_LOADER_META_URL: &str = "https://meta.fabricmc.net/v2/versions/loader";

/// The address the versions of the game Fabric supports are listed at.
pub const FABRIC_GAME_META_URL: &str = "https://meta.fabricmc.net/v2/versions/game";

/// The address the versions of the loader of Quilt are listed at.
pub const QUILT_LOADER_META_URL: &str = "https://meta.quiltmc.org/v3/versions/loader";

/// The address the versions of the game Quilt supports are listed at.
pub const QUILT_GAME_META_URL: &str = "https://meta.quiltmc.org/v3/versions/game";

/// The address the versions of the loader of Legacy Fabric are listed at.
pub const LEGACY_FABRIC_LOADER_META_URL: &str = "https://meta.legacyfabric.net/v2/versions/loader";

/// The address the versions of the game Legacy Fabric supports are listed at.
pub const LEGACY_FABRIC_GAME_META_URL: &str = "https://meta.legacyfabric.net/v2/versions/game";

/// The address the versions of Forge are listed at.
pub const FORGE_LIST_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

/// The address the versions of NeoForge are listed at.
pub const NEO_FORGE_META_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

/// The address the old versions of NeoForge are listed at.
pub const NEO_FORGE_OLD_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/forge";

/// The address the versions of Cleanroom are listed at.
pub const CLEANROOM_LIST_URL: &str = "https://hmcl.glavo.site/metadata/cleanroom/index.json";

/// The address the files of the assets are served from.
pub use super::game::ASSET_URL;

/// The source of a file the player chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourcePreference {
    /// The launcher chooses the source from the place of the player.
    #[default]
    Default,
    /// The launcher asks the official server first.
    Official,
    /// The launcher asks the mirror first.
    Mirror,
}

/// A rule that maps an address of an official server to the address of a
/// mirror.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorRule {
    /// The beginning of the official address.
    pub source: String,
    /// The beginning that replaces [`MirrorRule::source`].
    pub target: String,
    /// Whether the official server is asked before the mirror even where the
    /// mirror is preferred.
    ///
    /// The interfaces of the mod repositories answer with a different shape
    /// than the repositories themselves, so the official answer is read first
    /// and the mirror is the fallback.
    pub fallback: bool,
}

impl MirrorRule {
    /// Creates the rule that makes the mirror the fallback of the official
    /// server.
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            fallback: false,
        }
    }

    /// Creates the rule that is only read when the official server fails.
    pub fn fallback(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            fallback: true,
        }
    }

    /// Returns the address of the mirror for the address of the official
    /// server, if the rule covers it.
    pub fn map(&self, url: &str) -> Option<String> {
        url.strip_prefix(&self.source)
            .map(|rest| format!("{}{rest}", self.target))
    }
}

/// The servers the launcher downloads the files of the game from.
///
/// The provider holds the rules that map the official addresses to the mirror
/// and the source the player chose. It does not download anything: it only
/// tells the order in which the addresses are tried.
#[derive(Debug, Clone)]
pub struct DownloadProvider {
    /// The root of the mirror, without the slash at the end.
    bmcl_root: String,
    /// The rules that map the official addresses to the mirror.
    rules: Vec<MirrorRule>,
    /// Whether the machine of the player is in the mainland of China.
    china_mainland: bool,
    /// The source of the files of the game.
    file_source: SourcePreference,
    /// The source of the lists of the versions.
    version_list_source: SourcePreference,
}

impl DownloadProvider {
    /// Creates the provider of the launcher.
    ///
    /// The rules are the ones of HMCL: the services of Mojang, the repositories
    /// of the loaders and the interfaces of the mod repositories all have an
    /// address on the mirror.
    pub fn new(bmcl_root: impl Into<String>, china_mainland: bool) -> Self {
        let bmcl_root = bmcl_root.into();
        let bmcl_root = bmcl_root.trim_end_matches('/').to_string();

        let rules = vec![
            MirrorRule::new("https://bmclapi2.bangbang93.com", bmcl_root.clone()),
            MirrorRule::new("https://launchermeta.mojang.com", bmcl_root.clone()),
            MirrorRule::new("https://piston-meta.mojang.com", bmcl_root.clone()),
            MirrorRule::new("https://piston-data.mojang.com", bmcl_root.clone()),
            MirrorRule::new("https://launcher.mojang.com", bmcl_root.clone()),
            MirrorRule::new(
                "https://libraries.minecraft.net",
                format!("{bmcl_root}/libraries"),
            ),
            MirrorRule::new(
                "http://files.minecraftforge.net/maven",
                format!("{bmcl_root}/maven"),
            ),
            MirrorRule::new(
                "https://files.minecraftforge.net/maven",
                format!("{bmcl_root}/maven"),
            ),
            MirrorRule::new(
                "https://maven.minecraftforge.net",
                format!("{bmcl_root}/maven"),
            ),
            MirrorRule::new(
                "https://maven.neoforged.net/releases/",
                format!("{bmcl_root}/maven/"),
            ),
            MirrorRule::new(
                "http://dl.liteloader.com/versions",
                format!("{bmcl_root}/maven"),
            ),
            MirrorRule::new(
                "https://dl.liteloader.com/versions",
                format!("{bmcl_root}/maven"),
            ),
            MirrorRule::new(
                "https://meta.fabricmc.net",
                format!("{bmcl_root}/fabric-meta"),
            ),
            MirrorRule::new("https://maven.fabricmc.net", format!("{bmcl_root}/maven")),
            MirrorRule::new(
                "https://authlib-injector.yushi.moe",
                format!("{bmcl_root}/mirrors/authlib-injector"),
            ),
            MirrorRule::new(
                "https://repo1.maven.org/maven2",
                "https://mirrors.cloud.tencent.com/nexus/repository/maven-public",
            ),
            MirrorRule::new(
                "https://repo.maven.apache.org/maven2",
                "https://mirrors.cloud.tencent.com/nexus/repository/maven-public",
            ),
            MirrorRule::new(
                "https://hmcl.glavo.site/metadata/cleanroom",
                "https://alist.8mi.tech/d/mirror/HMCL-Metadata/Auto/cleanroom",
            ),
            MirrorRule::new(
                "https://hmcl.glavo.site/metadata/fmllibs",
                "https://alist.8mi.tech/d/mirror/HMCL-Metadata/Auto/fmllibs",
            ),
            MirrorRule::new(
                "https://zkitefly.github.io/unlisted-versions-of-minecraft",
                "https://alist.8mi.tech/d/mirror/unlisted-versions-of-minecraft/Auto",
            ),
            MirrorRule::fallback(
                "https://api.modrinth.com",
                "https://mod.mcimirror.top/modrinth",
            ),
            MirrorRule::fallback("https://cdn.modrinth.com", "https://mod.mcimirror.top"),
            MirrorRule::fallback(
                "https://api.curseforge.com",
                "https://mod.mcimirror.top/curseforge",
            ),
            MirrorRule::fallback("https://edge.forgecdn.net", "https://mod.mcimirror.top"),
        ];

        Self {
            bmcl_root,
            rules,
            china_mainland,
            file_source: SourcePreference::Default,
            version_list_source: SourcePreference::Default,
        }
    }

    /// Returns the provider of HMCL with the mirror the launcher ships with.
    pub fn hmcl(china_mainland: bool) -> Self {
        Self::new(DEFAULT_BMCLAPI_ROOT, china_mainland)
    }

    /// Returns the root of the mirror.
    pub fn bmcl_root(&self) -> &str {
        &self.bmcl_root
    }

    /// Returns whether the machine of the player is in the mainland of China.
    pub fn is_china_mainland(&self) -> bool {
        self.china_mainland
    }

    /// Sets the source of the files of the game.
    pub fn set_file_source(&mut self, source: SourcePreference) {
        self.file_source = source;
    }

    /// Sets the source of the lists of the versions.
    pub fn set_version_list_source(&mut self, source: SourcePreference) {
        self.version_list_source = source;
    }

    /// Returns the source of the files of the game.
    pub fn file_source(&self) -> SourcePreference {
        self.file_source
    }

    /// Returns the source of the lists of the versions.
    pub fn version_list_source(&self) -> SourcePreference {
        self.version_list_source
    }

    /// Returns the addresses a file is downloaded from, the one that is tried
    /// first first.
    ///
    /// A file that is not published by a server the mirror knows is only
    /// downloaded from the address it names.
    pub fn download_candidates(&self, urls: &[String]) -> DownloadCandidates {
        self.candidates(self.file_source, urls)
    }

    /// Returns the addresses a list of the versions is downloaded from.
    pub fn version_list_candidates(&self, urls: &[String]) -> DownloadCandidates {
        self.candidates(self.version_list_source, urls)
    }

    /// Returns the addresses of a file of the assets.
    ///
    /// The files of the assets are named by the digest, so the mirror holds
    /// them under its own directory of the assets.
    pub fn asset_object_candidates(&self, location: &str) -> DownloadCandidates {
        self.pair_candidates(
            self.file_source,
            format!("{ASSET_URL}{location}"),
            format!("{}/assets/{location}", self.bmcl_root),
        )
    }

    /// Returns the addresses the list of the versions of the game is read from.
    pub fn game_version_list_candidates(&self) -> DownloadCandidates {
        self.pair_candidates(
            self.version_list_source,
            VERSION_MANIFEST_URL.to_string(),
            format!("{}/mc/game/version_manifest.json", self.bmcl_root),
        )
    }

    /// Returns the addresses the versions of the loader of Fabric are read
    /// from, and the addresses the versions of the game Fabric supports are
    /// read from.
    pub fn fabric_version_list_candidates(&self) -> FabricVersionListCandidates {
        FabricVersionListCandidates {
            loader: self.pair_candidates(
                self.version_list_source,
                FABRIC_LOADER_META_URL.to_string(),
                format!("{}/fabric-meta/v2/versions/loader", self.bmcl_root),
            ),
            game: self.pair_candidates(
                self.version_list_source,
                FABRIC_GAME_META_URL.to_string(),
                format!("{}/fabric-meta/v2/versions/game", self.bmcl_root),
            ),
        }
    }

    /// Returns the addresses the versions of the loader of Quilt are read
    /// from, and the addresses the versions of the game Quilt supports are
    /// read from.
    pub fn quilt_version_list_candidates(&self) -> FabricVersionListCandidates {
        FabricVersionListCandidates {
            loader: self.pair_candidates(
                self.version_list_source,
                QUILT_LOADER_META_URL.to_string(),
                format!("{}{QUILT_MIRROR_META_ROOT}/versions/loader", self.bmcl_root),
            ),
            game: self.pair_candidates(
                self.version_list_source,
                QUILT_GAME_META_URL.to_string(),
                format!("{}{QUILT_MIRROR_META_ROOT}/versions/game", self.bmcl_root),
            ),
        }
    }

    /// Returns the addresses the versions of the loader of Legacy Fabric are
    /// read from, and the addresses the versions of the game Legacy Fabric
    /// supports are read from.
    ///
    /// The mirror does not hold the service of Legacy Fabric, so both lists
    /// are read from the official server.
    pub fn legacy_fabric_version_list_candidates(&self) -> FabricVersionListCandidates {
        FabricVersionListCandidates {
            loader: DownloadCandidates::of(LEGACY_FABRIC_LOADER_META_URL),
            game: DownloadCandidates::of(LEGACY_FABRIC_GAME_META_URL),
        }
    }

    /// Returns the addresses of the metadata a loader of Fabric publishes
    /// about one of its versions.
    ///
    /// The service of Fabric and the service of Quilt both publish the loader
    /// of a version of the game at `{meta}/{gameVersion}/{loaderVersion}`, so
    /// the addresses are only read from the official server of Quilt and of
    /// Legacy Fabric, whose mirror does not exist.
    pub fn loader_meta_candidates(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> DownloadCandidates {
        self.download_candidates(&[format!(
            "{FABRIC_LOADER_META_URL}/{game_version}/{loader_version}"
        )])
    }

    /// Returns the addresses of the metadata a loader of Quilt publishes about
    /// one of its versions.
    pub fn quilt_meta_candidates(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> DownloadCandidates {
        self.download_candidates(&[format!(
            "{QUILT_LOADER_META_URL}/{game_version}/{loader_version}"
        )])
    }

    /// Returns the addresses of the metadata a loader of Legacy Fabric
    /// publishes about one of its versions.
    pub fn legacy_fabric_meta_candidates(
        &self,
        game_version: &str,
        loader_version: &str,
    ) -> DownloadCandidates {
        self.download_candidates(&[format!(
            "{LEGACY_FABRIC_LOADER_META_URL}/{game_version}/{loader_version}"
        )])
    }

    /// Returns the addresses the versions of Cleanroom are read from.
    ///
    /// The list of Cleanroom is not served by Mojang, so the mirror of the list
    /// is only read on a machine in the mainland of China.
    pub fn cleanroom_version_list_candidates(&self) -> DownloadCandidates {
        if self.china_mainland {
            DownloadCandidates::new(vec![
                DownloadCandidate::new(CLEANROOM_LIST_URL),
                DownloadCandidate::new(
                    "https://alist.8mi.tech/d/mirror/HMCL-Metadata/Auto/cleanroom/index.json",
                ),
            ])
        } else {
            DownloadCandidates::of(CLEANROOM_LIST_URL)
        }
    }

    /// Returns the addresses of the files of a list of the versions, the
    /// addresses of the mirror matched to the addresses of the official
    /// servers.
    fn candidates(&self, source: SourcePreference, urls: &[String]) -> DownloadCandidates {
        assert!(
            !urls.is_empty(),
            "the addresses of a file must not be empty"
        );

        // Whether the mirror is asked first; `None` means the mirror is not
        // asked when the machine is not in the mainland of China.
        let mirror_first = match (self.china_mainland, source) {
            (true, SourcePreference::Default | SourcePreference::Mirror) => Some(true),
            (true, SourcePreference::Official) => Some(false),
            (false, SourcePreference::Default | SourcePreference::Official) => None,
            (false, SourcePreference::Mirror) => Some(true),
        };

        let Some(mirror_first) = mirror_first else {
            return DownloadCandidates::new(
                urls.iter()
                    .map(|url| DownloadCandidate::new(url.clone()))
                    .collect(),
            );
        };

        let mut candidates = Vec::with_capacity(urls.len() * 2);
        for url in urls {
            let official = DownloadCandidate::new(url.clone());
            let mirror = self
                .rules
                .iter()
                .find_map(|rule| rule.map(url).map(|mirror| (mirror, rule.fallback)));

            let Some((mirror, fallback)) = mirror else {
                candidates.push(official);
                continue;
            };
            let mirror = DownloadCandidate::new(mirror);

            // The mirror of a mod repository answers with a different shape,
            // so the official server is read first where the rule says so.
            if mirror_first && !fallback {
                candidates.push(mirror);
                candidates.push(official);
            } else {
                candidates.push(official);
                candidates.push(mirror);
            }
        }

        DownloadCandidates::new(candidates)
    }

    /// Returns the addresses of a file that is published by an official server
    /// and by the mirror.
    fn pair_candidates(
        &self,
        source: SourcePreference,
        official: String,
        mirror: String,
    ) -> DownloadCandidates {
        let prefer_mirror = self.prefers_mirror(source);
        let candidates = if prefer_mirror {
            vec![
                DownloadCandidate::new(mirror),
                DownloadCandidate::new(official),
            ]
        } else {
            vec![
                DownloadCandidate::new(official),
                DownloadCandidate::new(mirror),
            ]
        };

        DownloadCandidates::new(candidates)
    }

    /// Returns whether the mirror is asked before the official server.
    fn prefers_mirror(&self, source: SourcePreference) -> bool {
        match (self.china_mainland, source) {
            (true, SourcePreference::Default) => true,
            (true, SourcePreference::Official) => false,
            (true, SourcePreference::Mirror) => true,
            (false, SourcePreference::Default) => false,
            (false, SourcePreference::Official) => false,
            (false, SourcePreference::Mirror) => true,
        }
    }
}

/// The addresses the versions of a loader that is listed by a service the
/// mirror keeps are read from.
///
/// The mirror only holds the versions of Fabric and of Quilt: the service of
/// Legacy Fabric is not mirrored, so the list of Legacy Fabric is read from the
/// official server only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricVersionListCandidates {
    /// The addresses the versions of the loader are read from.
    pub loader: DownloadCandidates,
    /// The addresses the versions of the game the loader supports are read
    /// from.
    pub game: DownloadCandidates,
}

/// The address of the mirror the versions of the loader of Quilt are read
/// from.
pub const QUILT_MIRROR_META_ROOT: &str = "/quilt-meta/v3";

/// Returns whether a machine that names the time zone and the country of the
/// player is in the mainland of China.
///
/// The launcher reads the time zone as the first sign: Shanghai is the zone of
/// the mainland. A machine in another zone of UTC+8 is still in the mainland
/// when it names China as its country, because the west of the country keeps
/// another zone.
pub fn is_china_mainland_machine(
    zone: Option<&str>,
    utc_offset_seconds: i32,
    country: Option<&str>,
) -> bool {
    if zone == Some("Asia/Shanghai") {
        return true;
    }

    utc_offset_seconds == 8 * 60 * 60 && country == Some("CN")
}

impl fmt::Display for SourcePreference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Default => "default",
            Self::Official => "official",
            Self::Mirror => "mirror",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the provider of a machine in the mainland of China.
    fn china() -> DownloadProvider {
        DownloadProvider::hmcl(true)
    }

    /// Returns the provider of a machine that is not in the mainland of China.
    fn elsewhere() -> DownloadProvider {
        DownloadProvider::hmcl(false)
    }

    /// Returns the addresses of the candidates.
    fn urls(candidates: &DownloadCandidates) -> Vec<String> {
        candidates
            .candidates()
            .iter()
            .map(|candidate| candidate.url.clone())
            .collect()
    }

    /// Returns the address of the library of the game.
    fn library() -> Vec<String> {
        vec!["https://libraries.minecraft.net/com/example/library/1.0/library-1.0.jar".to_string()]
    }

    #[test]
    fn the_official_server_is_asked_first_outside_the_mainland() {
        let provider = elsewhere();
        assert_eq!(
            urls(&provider.download_candidates(&library())),
            vec!["https://libraries.minecraft.net/com/example/library/1.0/library-1.0.jar"]
        );

        let mut provider = elsewhere();
        provider.set_file_source(SourcePreference::Mirror);
        assert_eq!(
            urls(&provider.download_candidates(&library())),
            vec![
                "https://bmclapi2.bangbang93.com/libraries/com/example/library/1.0/library-1.0.jar",
                "https://libraries.minecraft.net/com/example/library/1.0/library-1.0.jar"
            ]
        );
    }

    #[test]
    fn the_mirror_is_asked_first_in_the_mainland() {
        let provider = china();
        assert_eq!(
            urls(&provider.download_candidates(&library())),
            vec![
                "https://bmclapi2.bangbang93.com/libraries/com/example/library/1.0/library-1.0.jar",
                "https://libraries.minecraft.net/com/example/library/1.0/library-1.0.jar"
            ]
        );

        let mut provider = china();
        provider.set_file_source(SourcePreference::Official);
        assert_eq!(
            urls(&provider.download_candidates(&library())),
            vec![
                "https://libraries.minecraft.net/com/example/library/1.0/library-1.0.jar",
                "https://bmclapi2.bangbang93.com/libraries/com/example/library/1.0/library-1.0.jar"
            ]
        );
    }

    #[test]
    fn a_file_the_mirror_does_not_know_is_asked_once() {
        let provider = china();
        let url = "https://example.com/file.jar".to_string();
        assert_eq!(
            urls(&provider.download_candidates(std::slice::from_ref(&url))),
            vec![url]
        );
    }

    #[test]
    fn the_interface_of_a_mod_repository_is_asked_after_the_official_server() {
        // Even in the mainland, the interface of Modrinth is read from the
        // official server first, because the mirror answers with another shape.
        let provider = china();
        let url = "https://api.modrinth.com/v2/project/sodium".to_string();
        assert_eq!(
            urls(&provider.download_candidates(&[url])),
            vec![
                "https://api.modrinth.com/v2/project/sodium",
                "https://mod.mcimirror.top/modrinth/v2/project/sodium"
            ]
        );
    }

    #[test]
    fn the_files_of_the_assets_are_named_by_their_digest() {
        let provider = china();
        let location = "ab/abcdef0123456789";
        assert_eq!(
            urls(&provider.asset_object_candidates(location)),
            vec![
                "https://bmclapi2.bangbang93.com/assets/ab/abcdef0123456789",
                "https://resources.download.minecraft.net/ab/abcdef0123456789"
            ]
        );
    }

    #[test]
    fn the_list_of_the_versions_of_the_game_is_read_from_both_servers() {
        let provider = china();
        assert_eq!(
            urls(&provider.game_version_list_candidates()),
            vec![
                "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json",
                "https://piston-meta.mojang.com/mc/game/version_manifest.json"
            ]
        );

        let provider = elsewhere();
        assert_eq!(
            urls(&provider.game_version_list_candidates()),
            vec![
                "https://piston-meta.mojang.com/mc/game/version_manifest.json",
                "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json"
            ]
        );
    }

    #[test]
    fn the_lists_of_the_loader_of_fabric_are_read_from_the_mirror() {
        let provider = china();
        let candidates = provider.fabric_version_list_candidates();
        assert_eq!(
            urls(&candidates.loader),
            vec![
                "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader",
                "https://meta.fabricmc.net/v2/versions/loader"
            ]
        );
        assert_eq!(
            urls(&candidates.game),
            vec![
                "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/game",
                "https://meta.fabricmc.net/v2/versions/game"
            ]
        );
    }

    #[test]
    fn the_list_of_cleanroom_has_a_mirror_in_the_mainland() {
        assert_eq!(
            urls(&china().cleanroom_version_list_candidates()),
            vec![
                CLEANROOM_LIST_URL,
                "https://alist.8mi.tech/d/mirror/HMCL-Metadata/Auto/cleanroom/index.json"
            ]
        );
        assert_eq!(
            urls(&elsewhere().cleanroom_version_list_candidates()),
            vec![CLEANROOM_LIST_URL]
        );
    }

    #[test]
    fn the_rule_maps_only_the_addresses_it_covers() {
        let rule = MirrorRule::new("https://piston-meta.mojang.com", "https://mirror.example");
        assert_eq!(
            rule.map("https://piston-meta.mojang.com/mc/game/version_manifest.json"),
            Some("https://mirror.example/mc/game/version_manifest.json".to_string())
        );
        assert_eq!(rule.map("https://example.com/file.jar"), None);
    }

    #[test]
    fn a_mirror_root_with_a_slash_is_normalized() {
        let provider = DownloadProvider::new("https://mirror.example/", true);
        assert_eq!(provider.bmcl_root(), "https://mirror.example");
        assert_eq!(
            urls(&provider.game_version_list_candidates())[0],
            "https://mirror.example/mc/game/version_manifest.json"
        );
    }

    #[test]
    fn the_time_zone_of_the_mainland_is_recognized() {
        assert!(is_china_mainland_machine(Some("Asia/Shanghai"), 0, None));
        assert!(is_china_mainland_machine(None, 8 * 60 * 60, Some("CN")));
        assert!(!is_china_mainland_machine(
            Some("Asia/Tokyo"),
            9 * 60 * 60,
            Some("JP")
        ));
        // A machine in UTC+8 that does not name China is not in the mainland.
        assert!(!is_china_mainland_machine(
            Some("Asia/Singapore"),
            8 * 60 * 60,
            Some("SG")
        ));
    }
}
