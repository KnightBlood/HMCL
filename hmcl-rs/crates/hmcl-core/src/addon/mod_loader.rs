//! The loaders the mods of the game are written for.
//!
//! The launcher reads the loader of a mod from the metadata of the mod, so the
//! list of the mods can show which loader needs which mod. A mod whose metadata
//! cannot be read is kept as a mod of an unknown loader.

use std::fmt;

/// A loader of the mods of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum ModLoaderType {
    /// The metadata of the mod does not name a loader.
    #[default]
    Unknown,
    /// Forge.
    Forge,
    /// Cleanroom.
    Cleanroom,
    /// NeoForge.
    NeoForge,
    /// Fabric.
    Fabric,
    /// Quilt.
    Quilt,
    /// LiteLoader.
    LiteLoader,
    /// Legacy Fabric.
    LegacyFabric,
}

impl ModLoaderType {
    /// Every loader the launcher knows, the unknown loader first.
    pub const ALL: [Self; 8] = [
        Self::Unknown,
        Self::Forge,
        Self::Cleanroom,
        Self::NeoForge,
        Self::Fabric,
        Self::Quilt,
        Self::LiteLoader,
        Self::LegacyFabric,
    ];

    /// Returns the name the launcher shows for the loader.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Forge => "Forge",
            Self::Cleanroom => "Cleanroom",
            Self::NeoForge => "NeoForge",
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::LiteLoader => "LiteLoader",
            Self::LegacyFabric => "Legacy Fabric",
        }
    }

    /// Returns the name the repositories of the addons use, if there is one.
    ///
    /// The unknown loader is not offered by a repository, so it has no name.
    pub const fn addon_loader(self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Forge => Some("forge"),
            Self::Cleanroom => Some("cleanroom"),
            Self::NeoForge => Some("neoforge"),
            Self::Fabric => Some("fabric"),
            Self::Quilt => Some("quilt"),
            Self::LiteLoader => Some("liteloader"),
            Self::LegacyFabric => Some("legacy-fabric"),
        }
    }

    /// Returns the name of the variable the launcher sets for the loader in the
    /// environment of the game, if the loader has one.
    pub const fn environment_name(self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Forge => Some("INST_FORGE"),
            Self::Cleanroom => Some("INST_CLEANROOM"),
            Self::NeoForge => Some("INST_NEOFORGE"),
            Self::Fabric => Some("INST_FABRIC"),
            Self::Quilt => Some("INST_QUILT"),
            Self::LiteLoader => Some("INST_LITELOADER"),
            Self::LegacyFabric => Some("INST_LEGACYFABRIC"),
        }
    }

    /// Returns the loader with the name the repositories use, if the launcher
    /// knows one.
    pub fn from_addon_loader(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|loader| loader.addon_loader() == Some(name))
    }
}

impl fmt::Display for ModLoaderType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loaders_are_named() {
        assert_eq!(ModLoaderType::Forge.display_name(), "Forge");
        assert_eq!(ModLoaderType::LegacyFabric.display_name(), "Legacy Fabric");
        assert_eq!(ModLoaderType::NeoForge.to_string(), "NeoForge");
    }

    #[test]
    fn the_loaders_are_found_by_the_names_of_the_repositories() {
        assert_eq!(
            ModLoaderType::from_addon_loader("neoforge"),
            Some(ModLoaderType::NeoForge)
        );
        assert_eq!(
            ModLoaderType::from_addon_loader("legacy-fabric"),
            Some(ModLoaderType::LegacyFabric)
        );
        assert_eq!(ModLoaderType::from_addon_loader("sodium"), None);
        // The unknown loader is not offered by a repository.
        assert_eq!(ModLoaderType::Unknown.addon_loader(), None);
    }

    #[test]
    fn the_loaders_name_the_variables_of_the_game() {
        assert_eq!(
            ModLoaderType::Fabric.environment_name(),
            Some("INST_FABRIC")
        );
        assert_eq!(ModLoaderType::Unknown.environment_name(), None);
    }
}
