//! The list of the versions of the game.
//!
//! The servers of the game publish the list of the versions in a manifest;
//! every version of the list has the address of the file that holds its
//! details.

use serde::{Deserialize, Serialize};

use super::release_type::ReleaseType;

/// The list of the versions of the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionManifest {
    /// The newest versions of every kind.
    pub latest: LatestVersions,
    /// The versions of the game, from the newest to the oldest.
    pub versions: Vec<VersionManifestEntry>,
}

/// The newest versions of every kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestVersions {
    /// The identifier of the newest release.
    pub release: String,
    /// The identifier of the newest snapshot.
    pub snapshot: String,
}

/// A version of the game in the list of the versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionManifestEntry {
    /// The identifier of the version.
    pub id: String,
    /// The kind of the version.
    #[serde(rename = "type")]
    pub version_type: ReleaseType,
    /// The address of the file that holds the details of the version.
    pub url: String,
    /// The moment the version was published.
    #[serde(default)]
    pub time: Option<String>,
    /// The moment the version was released.
    #[serde(default, rename = "releaseTime")]
    pub release_time: Option<String>,
    /// The digest of the file that holds the details of the version.
    #[serde(default)]
    pub sha1: Option<String>,
    /// The level the version asks the launcher to obey.
    #[serde(default, rename = "complianceLevel")]
    pub compliance_level: Option<i32>,
    /// The arguments the client writes into its log.
    #[serde(default)]
    pub logging: Option<Logging>,
}

/// The arguments the client writes into its log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Logging {
    /// The file the client writes into.
    #[serde(default)]
    pub client: Option<LoggingArgument>,
    /// The file the server writes into.
    #[serde(default)]
    pub server: Option<LoggingArgument>,
}

/// The argument of the file the client writes into.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingArgument {
    /// The text of the argument.
    pub argument: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "latest": {
            "release": "1.20.1",
            "snapshot": "23w42a"
        },
        "versions": [
            {
                "id": "1.20.1",
                "type": "release",
                "url": "https://piston-meta.mojang.com/v1/packages/xxx/1.20.1.json",
                "time": "2023-06-12T13:25:51+00:00",
                "releaseTime": "2023-06-12T13:25:51+00:00",
                "sha1": "b1b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b",
                "complianceLevel": 1
            },
            {
                "id": "23w42a",
                "type": "snapshot",
                "url": "https://piston-meta.mojang.com/v1/packages/yyy/23w42a.json",
                "time": "2023-08-02T11:22:33+00:00",
                "releaseTime": "2023-08-02T11:22:33+00:00"
            }
        ]
    }"#;

    #[test]
    fn the_list_of_the_versions_is_read() {
        let manifest: VersionManifest =
            serde_json::from_str(MANIFEST).expect("the list should be read");

        assert_eq!(manifest.latest.release, "1.20.1");
        assert_eq!(manifest.latest.snapshot, "23w42a");
        assert_eq!(manifest.versions.len(), 2);
        assert_eq!(manifest.versions[0].id, "1.20.1");
        assert_eq!(manifest.versions[0].version_type, ReleaseType::Release);
        assert_eq!(manifest.versions[0].compliance_level, Some(1));
        assert_eq!(manifest.versions[1].version_type, ReleaseType::Snapshot);
        assert!(manifest.versions[1].sha1.is_none());
    }
}
