//! The changes the launcher applies to an instance when it is resolved.
//!
//! The repositories of the loaders publish the changes a loader makes to a
//! version of the game; the launcher keeps the changes in the manifest of the
//! instance and applies them when the instance is resolved.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::instance_manifest::InstanceManifest;
use super::release_type::ReleaseType;
use super::version_metadata::{
    Arguments, AssetIndex, CompatibilityRule, DownloadInfo, JavaVersion, Library, Logging,
};

/// The changes of an instance of the game.
///
/// A patch holds what a loader or the repository added to the version the
/// instance inherits from; the patch wins over the data of the version, and
/// the libraries and the arguments of the patch are added to the ones of the
/// version.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePatch {
    /// The identifier of the component the patch belongs to, if the patch
    /// belongs to one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The version of the component the patch belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The order the patches are applied in; the smaller priorities first.
    #[serde(default)]
    pub priority: i32,
    /// The arguments of the game, in the old format.
    #[serde(
        default,
        rename = "minecraftArguments",
        skip_serializing_if = "Option::is_none"
    )]
    pub minecraft_arguments: Option<String>,
    /// The arguments of the game and of the virtual machine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Arguments>,
    /// The class the game is started with.
    #[serde(default, rename = "mainClass", skip_serializing_if = "Option::is_none")]
    pub main_class: Option<String>,
    /// The file of the game the instance uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jar: Option<String>,
    /// The index of the assets of the instance.
    #[serde(
        default,
        rename = "assetIndex",
        skip_serializing_if = "Option::is_none"
    )]
    pub asset_index: Option<AssetIndex>,
    /// The name of the assets of the instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<String>,
    /// The level the instance asks the launcher to obey.
    #[serde(
        default,
        rename = "complianceLevel",
        skip_serializing_if = "Option::is_none"
    )]
    pub compliance_level: Option<i32>,
    /// The version of Java the instance needs.
    #[serde(
        default,
        rename = "javaVersion",
        skip_serializing_if = "Option::is_none"
    )]
    pub java_version: Option<JavaVersion>,
    /// The libraries the patch adds.
    #[serde(default)]
    pub libraries: Vec<Library>,
    /// The rules the patch adds.
    #[serde(default, rename = "compatibilityRules")]
    pub compatibility_rules: Vec<CompatibilityRule>,
    /// The files that are downloaded for the instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<BTreeMap<String, DownloadInfo>>,
    /// The arguments the client writes into its log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<Logging>,
    /// The kind of the version the instance was created from.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub version_type: Option<ReleaseType>,
    /// The moment the version was published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// The moment the version was released.
    #[serde(
        default,
        rename = "releaseTime",
        skip_serializing_if = "Option::is_none"
    )]
    pub release_time: Option<String>,
    /// The oldest version of the launcher that understands the patch.
    #[serde(
        default,
        rename = "minimumLauncherVersion",
        skip_serializing_if = "Option::is_none"
    )]
    pub minimum_launcher_version: Option<i32>,
    /// Whether the patch is hidden from the list of the components.
    #[serde(default)]
    pub hidden: bool,
}

impl InstancePatch {
    /// Creates the patch of a resolved version.
    ///
    /// The version was installed from a manifest; the patch holds what the
    /// version adds to the version it inherits from, so the changes may be
    /// analyzed and applied again.
    pub fn from_manifest(manifest: &InstanceManifest) -> Self {
        Self {
            id: manifest.id.clone(),
            version: None,
            priority: 0,
            minecraft_arguments: manifest.minecraft_arguments.clone(),
            arguments: manifest.arguments.clone(),
            main_class: manifest.main_class.clone(),
            jar: manifest.jar.clone(),
            asset_index: manifest.asset_index.clone(),
            assets: manifest.assets.clone(),
            compliance_level: manifest.compliance_level,
            java_version: manifest.java_version.clone(),
            libraries: manifest.libraries.clone(),
            compatibility_rules: manifest.compatibility_rules.clone().unwrap_or_default(),
            downloads: manifest.downloads.clone(),
            logging: manifest.logging.clone(),
            version_type: manifest.version_type,
            time: manifest.time.clone(),
            release_time: manifest.release_time.clone(),
            minimum_launcher_version: manifest.minimum_launcher_version,
            hidden: manifest.hidden.unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_patch_of_a_version_holds_its_changes() {
        let manifest = InstanceManifest {
            id: Some("fabric-loader-0.15".to_string()),
            main_class: Some("net.fabricmc.loader.impl.launch.knot.KnotClient".to_string()),
            libraries: vec![Library {
                name: "net.fabricmc:fabric-loader:0.15".to_string(),
                ..Library::default()
            }],
            ..InstanceManifest::default()
        };

        let patch = InstancePatch::from_manifest(&manifest);

        assert_eq!(patch.id.as_deref(), Some("fabric-loader-0.15"));
        assert_eq!(
            patch.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        assert_eq!(patch.libraries.len(), 1);
        assert_eq!(patch.priority, 0);
    }

    #[test]
    fn the_patches_are_read_and_written() {
        let json = r#"{
            "id": "fabric",
            "version": "0.15",
            "priority": 1,
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [ { "name": "net.fabricmc:fabric-loader:0.15" } ]
        }"#;

        let patch: InstancePatch = serde_json::from_str(json).expect("the patch should be read");
        assert_eq!(patch.id.as_deref(), Some("fabric"));
        assert_eq!(patch.version.as_deref(), Some("0.15"));
        assert_eq!(patch.priority, 1);
        assert_eq!(patch.libraries.len(), 1);

        let written = serde_json::to_string(&patch).expect("the patch should be written");
        assert!(written.contains("\"version\":\"0.15\""));
        assert!(written.contains("\"libraries\""));
        assert!(!written.contains("\"mainClass\":null"));
    }
}
