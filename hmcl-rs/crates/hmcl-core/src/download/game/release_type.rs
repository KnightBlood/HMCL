//! The kinds of the versions of the game.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The kind of a version of the game.
///
/// The kinds are the ones the servers of the game use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReleaseType {
    /// A finished release.
    Release,
    /// A snapshot of the upcoming release.
    Snapshot,
    /// A version that was changed after it was released.
    Modified,
    /// A version of the beta period.
    OldBeta,
    /// A version of the alpha period.
    OldAlpha,
    /// A version that was announced but is not released yet.
    Pending,
    /// A version that was not obfuscated.
    Unobfuscated,
    /// A version of an unknown kind.
    #[default]
    Unknown,
}

impl ReleaseType {
    /// Returns the identifier of the kind, as the servers of the game use it.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Snapshot => "snapshot",
            Self::Modified => "modified",
            Self::OldBeta => "old-beta",
            Self::OldAlpha => "old-alpha",
            Self::Pending => "pending",
            Self::Unobfuscated => "unobfuscated",
            Self::Unknown => "unknown",
        }
    }

    /// Returns the kind of the identifier.
    pub fn from_id(id: &str) -> Self {
        match id {
            "release" => Self::Release,
            "snapshot" => Self::Snapshot,
            "modified" => Self::Modified,
            "old-beta" => Self::OldBeta,
            "old-alpha" => Self::OldAlpha,
            "pending" => Self::Pending,
            "unobfuscated" => Self::Unobfuscated,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for ReleaseType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_are_read_and_written() {
        assert_eq!(ReleaseType::from_id("release"), ReleaseType::Release);
        assert_eq!(ReleaseType::from_id("old-beta"), ReleaseType::OldBeta);
        assert_eq!(ReleaseType::from_id("something"), ReleaseType::Unknown);

        assert_eq!(ReleaseType::OldBeta.id(), "old-beta");
        assert_eq!(ReleaseType::OldAlpha.to_string(), "old-alpha");
    }

    #[test]
    fn the_kinds_are_read_from_the_json() {
        assert_eq!(
            serde_json::from_str::<ReleaseType>("\"snapshot\"").unwrap(),
            ReleaseType::Snapshot
        );
        assert_eq!(
            serde_json::to_string(&ReleaseType::OldAlpha).unwrap(),
            "\"old-alpha\""
        );
    }
}
