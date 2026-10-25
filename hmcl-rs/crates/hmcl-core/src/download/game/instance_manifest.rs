//! The manifest of an instance of the game.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::instance_patch::InstancePatch;
use super::release_type::ReleaseType;
use super::version_metadata::{
    Argument, Arguments, AssetIndex, CompatibilityRule, DownloadInfo, JavaVersion, Library, Logging,
};

/// The manifest of an instance of the game.
///
/// The manifest holds the libraries the instance needs, the files that are
/// downloaded for it and the arguments the game is started with. It is the
/// version of the game the instance was created from, with the changes the
/// loaders of the instance made.
///
/// An instance may inherit from another version: the manifest then holds only
/// the changes of the loaders, and the changes of the version it inherits
/// from are applied when the instance is resolved.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceManifest {
    /// The identifier of the instance.
    #[serde(default)]
    pub id: Option<String>,
    /// The arguments of the game, in the old format.
    #[serde(default, rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    /// The arguments of the game and of the virtual machine.
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// The class the game is started with.
    #[serde(default, rename = "mainClass")]
    pub main_class: Option<String>,
    /// The instance the manifest was created from.
    #[serde(default, rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
    /// The file of the game the instance uses.
    #[serde(default)]
    pub jar: Option<String>,
    /// The index of the assets of the instance.
    #[serde(default, rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    /// The name of the assets of the instance.
    #[serde(default)]
    pub assets: Option<String>,
    /// The level the instance asks the launcher to obey.
    #[serde(default, rename = "complianceLevel")]
    pub compliance_level: Option<i32>,
    /// The version of Java the instance needs.
    #[serde(default, rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
    /// The libraries the instance needs.
    #[serde(default)]
    pub libraries: Vec<Library>,
    /// The rules that tell whether the libraries are used.
    #[serde(default, rename = "compatibilityRules")]
    pub compatibility_rules: Option<Vec<CompatibilityRule>>,
    /// The files that are downloaded for the instance.
    #[serde(default)]
    pub downloads: Option<BTreeMap<String, DownloadInfo>>,
    /// The arguments the client writes into its log.
    #[serde(default)]
    pub logging: Option<Logging>,
    /// The kind of the version the instance was created from.
    #[serde(default, rename = "type")]
    pub version_type: Option<ReleaseType>,
    /// The moment the version was published.
    #[serde(default)]
    pub time: Option<String>,
    /// The moment the version was released.
    #[serde(default, rename = "releaseTime")]
    pub release_time: Option<String>,
    /// The oldest version of the launcher that understands the manifest.
    #[serde(default, rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: Option<i32>,
    /// Whether the manifest is the root of the instance.
    #[serde(default)]
    pub root: Option<bool>,
    /// Whether the instance is hidden in the list of the instances.
    #[serde(default)]
    pub hidden: Option<bool>,
    /// The changes the launcher was told to apply when the instance is
    /// resolved, from the repository the instance was installed from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patches: Vec<InstancePatch>,
}

impl InstanceManifest {
    /// Returns whether the manifest is the root of the instance.
    ///
    /// The manifests the launcher writes for the instances it installs are
    /// roots; the manifests of the versions that are installed separately are
    /// not.
    pub fn is_root(&self) -> bool {
        self.root.unwrap_or(false)
    }

    /// Merges the manifest with the manifest it inherits from.
    ///
    /// The data of the manifest wins over the data of the parent; the
    /// libraries and the arguments of both are kept, the libraries of the
    /// instance first and the arguments of the parent first.
    pub fn merge(&self, parent: &InstanceManifest) -> InstanceManifest {
        let mut libraries = self.libraries.clone();
        libraries.extend(parent.libraries.iter().cloned());

        let compatibility_rules =
            merge_rules(&parent.compatibility_rules, &self.compatibility_rules);

        InstanceManifest {
            id: self.id.clone(),
            minecraft_arguments: self
                .minecraft_arguments
                .clone()
                .or_else(|| parent.minecraft_arguments.clone()),
            arguments: Arguments::merge(parent.arguments.as_ref(), self.arguments.as_ref()),
            main_class: self
                .main_class
                .clone()
                .or_else(|| parent.main_class.clone()),
            inherits_from: None,
            jar: self.jar.clone().or_else(|| parent.jar.clone()),
            asset_index: self
                .asset_index
                .clone()
                .or_else(|| parent.asset_index.clone()),
            assets: self.assets.clone().or_else(|| parent.assets.clone()),
            compliance_level: self.compliance_level,
            java_version: self
                .java_version
                .clone()
                .or_else(|| parent.java_version.clone()),
            libraries,
            compatibility_rules,
            downloads: self.downloads.clone().or_else(|| parent.downloads.clone()),
            logging: self.logging.clone().or_else(|| parent.logging.clone()),
            version_type: self.version_type.or(parent.version_type),
            time: self.time.clone().or_else(|| parent.time.clone()),
            release_time: self
                .release_time
                .clone()
                .or_else(|| parent.release_time.clone()),
            minimum_launcher_version: maximum(
                self.minimum_launcher_version,
                parent.minimum_launcher_version,
            ),
            root: Some(true),
            hidden: self.hidden,
            patches: self.patches.clone(),
        }
    }

    /// Applies a patch the launcher was told to apply.
    ///
    /// The data of the patch wins over the data of the manifest; the
    /// libraries and the arguments of the patch are kept.
    pub fn apply_patch(&mut self, patch: &InstancePatch) {
        if let Some(arguments) = &patch.minecraft_arguments {
            self.minecraft_arguments = Some(arguments.clone());
        }
        self.arguments = Arguments::merge(self.arguments.as_ref(), patch.arguments.as_ref());

        if let Some(main_class) = &patch.main_class {
            self.main_class = Some(main_class.clone());
        }
        if let Some(asset_index) = &patch.asset_index {
            self.asset_index = Some(asset_index.clone());
        }
        if let Some(assets) = &patch.assets {
            self.assets = Some(assets.clone());
        }
        if let Some(level) = patch.compliance_level {
            self.compliance_level = Some(level);
        }
        if let Some(java_version) = &patch.java_version {
            self.java_version = Some(java_version.clone());
        }
        if let Some(downloads) = &patch.downloads {
            self.downloads = Some(downloads.clone());
        }
        if let Some(logging) = &patch.logging {
            self.logging = Some(logging.clone());
        }
        if let Some(version_type) = patch.version_type {
            self.version_type = Some(version_type);
        }
        if let Some(time) = &patch.time {
            self.time = Some(time.clone());
        }
        if let Some(release_time) = &patch.release_time {
            self.release_time = Some(release_time.clone());
        }

        self.libraries.extend(patch.libraries.iter().cloned());
        self.compatibility_rules = merge_rules(
            &self.compatibility_rules,
            &Some(patch.compatibility_rules.clone()),
        );
        self.minimum_launcher_version = maximum(
            self.minimum_launcher_version,
            patch.minimum_launcher_version,
        );
    }

    /// Returns the arguments of the game.
    ///
    /// The instances that use the old format have the arguments in one text.
    pub fn game_arguments(&self) -> Vec<&Argument> {
        if let Some(arguments) = &self.arguments {
            if let Some(game) = &arguments.game {
                return game.iter().collect();
            }
        }
        Vec::new()
    }

    /// Returns the arguments of the game in the old format.
    pub fn legacy_game_arguments(&self) -> Option<&str> {
        self.minecraft_arguments.as_deref()
    }
}

/// Returns the larger of two versions of the launcher, if either is known.
fn maximum(first: Option<i32>, second: Option<i32>) -> Option<i32> {
    match (first, second) {
        (Some(first), Some(second)) => Some(first.max(second)),
        (Some(version), None) | (None, Some(version)) => Some(version),
        (None, None) => None,
    }
}

/// Joins two lists of rules, the first list first.
fn merge_rules(
    first: &Option<Vec<CompatibilityRule>>,
    second: &Option<Vec<CompatibilityRule>>,
) -> Option<Vec<CompatibilityRule>> {
    match (first, second) {
        (None, None) => None,
        _ => {
            let mut rules = first.clone().unwrap_or_default();
            rules.extend(second.clone().unwrap_or_default());
            Some(rules)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "id": "a1b2c3d4-0000-0000-0000-000000000000",
        "minecraftArguments": "--username ${auth_player_name}",
        "mainClass": "net.minecraft.client.main.Main",
        "assets": "1.20",
        "assetIndex": { "id": "1.20", "sha1": "abc", "size": 1, "totalSize": 2, "url": "https://example.com/1.20.json" },
        "complianceLevel": 1,
        "javaVersion": { "component": "java-runtime-gamma", "majorVersion": 17 },
        "libraries": [
            { "name": "com.example:library:1.0" }
        ],
        "downloads": {
            "client": { "sha1": "111", "size": 100, "url": "https://example.com/client.jar" }
        },
        "type": "release",
        "releaseTime": "2023-06-12T13:25:51+00:00",
        "time": "2023-06-12T13:25:51+00:00",
        "minimumLauncherVersion": 21,
        "root": true,
        "hidden": false
    }"#;

    fn library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            ..Library::default()
        }
    }

    #[test]
    fn the_manifest_of_an_instance_is_read() {
        let manifest: InstanceManifest =
            serde_json::from_str(MANIFEST).expect("the manifest should be read");

        assert_eq!(
            manifest.id.as_deref(),
            Some("a1b2c3d4-0000-0000-0000-000000000000")
        );
        assert_eq!(
            manifest.main_class.as_deref(),
            Some("net.minecraft.client.main.Main")
        );
        assert_eq!(manifest.assets.as_deref(), Some("1.20"));
        assert_eq!(manifest.libraries.len(), 1);
        assert_eq!(manifest.version_type, Some(ReleaseType::Release));
        assert_eq!(manifest.minimum_launcher_version, Some(21));
        assert!(manifest.is_root());
        assert_eq!(manifest.hidden, Some(false));
        assert_eq!(manifest.compliance_level, Some(1));
    }

    #[test]
    fn the_downloads_of_an_instance_are_read() {
        let manifest: InstanceManifest =
            serde_json::from_str(MANIFEST).expect("the manifest should be read");

        let downloads = manifest
            .downloads
            .as_ref()
            .expect("the downloads should be read");
        let client = downloads.get("client").expect("the client should be read");
        assert_eq!(client.size, Some(100));
        assert_eq!(client.url, "https://example.com/client.jar");
    }

    #[test]
    fn the_arguments_of_an_instance_are_read() {
        let manifest: InstanceManifest =
            serde_json::from_str(MANIFEST).expect("the manifest should be read");

        assert_eq!(
            manifest.legacy_game_arguments(),
            Some("--username ${auth_player_name}")
        );
        assert!(manifest.game_arguments().is_empty());
    }

    #[test]
    fn a_manifest_wins_over_the_manifest_it_inherits_from() {
        let parent = InstanceManifest {
            id: Some("1.20.1".to_string()),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            jar: Some("1.20.1".to_string()),
            assets: Some("1.20".to_string()),
            libraries: vec![library("com.example:parent:1.0")],
            minimum_launcher_version: Some(21),
            ..InstanceManifest::default()
        };
        let child = InstanceManifest {
            id: Some("fabric-loader-0.15".to_string()),
            main_class: Some("net.fabricmc.loader.impl.launch.knot.KnotClient".to_string()),
            inherits_from: Some("1.20.1".to_string()),
            libraries: vec![library("net.fabricmc:fabric-loader:0.15")],
            minimum_launcher_version: Some(18),
            ..InstanceManifest::default()
        };

        let merged = child.merge(&parent);

        assert_eq!(merged.id.as_deref(), Some("fabric-loader-0.15"));
        assert_eq!(merged.inherits_from, None);
        assert_eq!(
            merged.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        assert_eq!(merged.jar.as_deref(), Some("1.20.1"));
        assert_eq!(merged.assets.as_deref(), Some("1.20"));
        assert_eq!(merged.minimum_launcher_version, Some(21));
        assert_eq!(merged.root, Some(true));

        let names: Vec<&str> = merged
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["net.fabricmc:fabric-loader:0.15", "com.example:parent:1.0"]
        );
    }

    #[test]
    fn the_arguments_of_the_parent_are_kept_first() {
        let parent = InstanceManifest {
            arguments: Some(Arguments {
                game: Some(vec![Argument::Text("--parent".to_string())]),
                jvm: Some(vec![Argument::Text("-Xmx2G".to_string())]),
            }),
            ..InstanceManifest::default()
        };
        let child = InstanceManifest {
            arguments: Some(Arguments {
                game: Some(vec![Argument::Text("--child".to_string())]),
                jvm: None,
            }),
            ..InstanceManifest::default()
        };

        let merged = child.merge(&parent);
        let arguments = merged.arguments.expect("the arguments should be kept");

        let game: Vec<_> = arguments
            .game
            .expect("the arguments of the game should be kept")
            .iter()
            .filter_map(|argument| argument.value().map(str::to_string))
            .collect();
        assert_eq!(game, vec!["--parent", "--child"]);

        let jvm: Vec<_> = arguments
            .jvm
            .expect("the arguments of the virtual machine should be kept")
            .iter()
            .filter_map(|argument| argument.value().map(str::to_string))
            .collect();
        assert_eq!(jvm, vec!["-Xmx2G"]);
    }

    #[test]
    fn a_patch_wins_over_the_manifest() {
        let mut manifest: InstanceManifest =
            serde_json::from_str(MANIFEST).expect("the manifest should be read");

        let patch = InstancePatch {
            main_class: Some("cpw.mods.modlauncher.Launcher".to_string()),
            libraries: vec![library("net.minecraftforge:forge:1.20.1-47.1.0")],
            minimum_launcher_version: Some(23),
            ..InstancePatch::default()
        };

        manifest.apply_patch(&patch);

        assert_eq!(
            manifest.main_class.as_deref(),
            Some("cpw.mods.modlauncher.Launcher")
        );
        assert_eq!(manifest.libraries.len(), 2);
        assert_eq!(manifest.minimum_launcher_version, Some(23));
    }
}
