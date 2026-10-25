//! The picture the launcher shows for an instance.
//!
//! The player may pick a picture for an instance, and the launcher otherwise
//! draws one from the files the player added and from the version of the game:
//! a picture of the mod loader, of OptiFine, of the April Fools update or of the
//! old versions of the game.

use std::path::PathBuf;

use crate::addon::ModLoaderType;
use crate::setting::GameInstanceIconType;
use crate::util::versioning::game_version_number::{GameVersionNumber, ReleaseType};

/// The picture the launcher shows for an instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceIcon {
    /// One of the pictures the launcher ships.
    Builtin(GameInstanceIconType),
    /// The image file the player added to the instance.
    File(PathBuf),
}

impl InstanceIcon {
    /// Returns the picture the launcher ships, if the instance shows one.
    pub fn builtin(&self) -> Option<GameInstanceIconType> {
        match self {
            Self::Builtin(icon) => Some(*icon),
            Self::File(_) => None,
        }
    }

    /// Returns the image file the player added, if the instance shows one.
    pub fn file(&self) -> Option<&PathBuf> {
        match self {
            Self::File(file) => Some(file),
            Self::Builtin(_) => None,
        }
    }
}

impl std::fmt::Display for InstanceIcon {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin(icon) => write!(formatter, "{}", icon.resource_url()),
            Self::File(file) => write!(formatter, "{}", file.display()),
        }
    }
}

/// Returns the picture of the mod loader, if the launcher has one for it.
///
/// The unknown loader shows the picture of the command block, like the loader
/// the launcher cannot name.
pub fn icon_of_mod_loader(loader: ModLoaderType) -> GameInstanceIconType {
    match loader {
        ModLoaderType::Forge => GameInstanceIconType::Forge,
        ModLoaderType::NeoForge => GameInstanceIconType::NeoForge,
        ModLoaderType::Fabric => GameInstanceIconType::Fabric,
        ModLoaderType::LegacyFabric => GameInstanceIconType::LegacyFabric,
        ModLoaderType::Quilt => GameInstanceIconType::Quilt,
        ModLoaderType::LiteLoader => GameInstanceIconType::Chicken,
        ModLoaderType::Cleanroom => GameInstanceIconType::Cleanroom,
        ModLoaderType::Unknown => GameInstanceIconType::Command,
    }
}

/// Returns the picture the launcher shows for an instance.
///
/// The picture the player picked comes first, then the image file the player
/// added to the instance, then the picture of the first mod loader, of
/// OptiFine, of the April Fools update, of a preview or a snapshot, of an old
/// version, and finally the grass block of the ordinary game.
pub fn instance_icon(
    settings_icon: Option<GameInstanceIconType>,
    icon_file: Option<PathBuf>,
    mod_loaders: &[ModLoaderType],
    has_optifine: bool,
    version: Option<&GameVersionNumber>,
) -> InstanceIcon {
    let icon = settings_icon.unwrap_or_default();
    if icon != GameInstanceIconType::Default {
        return InstanceIcon::Builtin(icon);
    }

    if let Some(file) = icon_file {
        return InstanceIcon::File(file);
    }

    if let Some(loader) = mod_loaders.first() {
        return InstanceIcon::Builtin(icon_of_mod_loader(*loader));
    }

    if has_optifine {
        return InstanceIcon::Builtin(GameInstanceIconType::Optifine);
    }

    InstanceIcon::Builtin(icon_of_version(version))
}

/// Returns the picture the launcher shows for a version of the game.
///
/// The launcher shows a picture for the versions the launcher knows: the April
/// Fools update, a snapshot or a preview, and the versions of the first
/// generations. A version the launcher does not know shows the grass block,
/// because the launcher has no better picture for it.
fn icon_of_version(version: Option<&GameVersionNumber>) -> GameInstanceIconType {
    let Some(version) = version else {
        return GameInstanceIconType::Grass;
    };

    if version.is_april_fools() {
        return GameInstanceIconType::AprilFools;
    }

    match version {
        GameVersionNumber::LegacySnapshot(_) => GameInstanceIconType::Command,
        GameVersionNumber::Release(release) => {
            if release.release_type() == ReleaseType::Ga {
                GameInstanceIconType::Grass
            } else {
                GameInstanceIconType::Command
            }
        }
        GameVersionNumber::Old(_) => GameInstanceIconType::CraftTable,
        GameVersionNumber::Special(_) => GameInstanceIconType::Grass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads a version of the game.
    fn version(text: &str) -> GameVersionNumber {
        GameVersionNumber::as_game_version(text)
    }

    #[test]
    fn the_picture_the_player_picked_comes_first() {
        let icon = instance_icon(
            Some(GameInstanceIconType::Chest),
            Some(PathBuf::from("icon.png")),
            &[ModLoaderType::Fabric],
            true,
            Some(&version("1.20.1")),
        );

        assert_eq!(icon.builtin(), Some(GameInstanceIconType::Chest));
        assert_eq!(icon.file(), None);
    }

    #[test]
    fn the_image_file_comes_before_the_components() {
        let icon = instance_icon(
            None,
            Some(PathBuf::from("versions/1.20.1/icon.png")),
            &[ModLoaderType::Forge],
            true,
            Some(&version("1.20.1")),
        );

        assert_eq!(
            icon.file(),
            Some(&PathBuf::from("versions/1.20.1/icon.png"))
        );
        assert_eq!(icon.builtin(), None);
    }

    #[test]
    fn the_picture_of_the_components_is_read() {
        let icon = instance_icon(
            None,
            None,
            &[ModLoaderType::NeoForge, ModLoaderType::Forge],
            true,
            Some(&version("1.20.1")),
        );
        assert_eq!(icon.builtin(), Some(GameInstanceIconType::NeoForge));

        let icon = instance_icon(None, None, &[], true, Some(&version("1.20.1")));
        assert_eq!(icon.builtin(), Some(GameInstanceIconType::Optifine));
    }

    #[test]
    fn the_pictures_of_the_versions_are_read() {
        let icon = instance_icon(None, None, &[], false, Some(&version("1.20.1")));
        assert_eq!(icon.builtin(), Some(GameInstanceIconType::Grass));

        let icon = instance_icon(None, None, &[], false, Some(&version("b1.7.3")));
        assert_eq!(icon.builtin(), Some(GameInstanceIconType::CraftTable));

        let icon = instance_icon(None, None, &[], false, Some(&version("23w42a")));
        assert_eq!(icon.builtin(), Some(GameInstanceIconType::Command));

        let icon = instance_icon(None, None, &[], false, Some(&version("1.20.1-pre1")));
        assert_eq!(icon.builtin(), Some(GameInstanceIconType::Command));

        // The April Fools update is shown as itself, even when it is a snapshot.
        let icon = instance_icon(None, None, &[], false, Some(&version("15w14a")));
        assert_eq!(icon.builtin(), Some(GameInstanceIconType::AprilFools));

        let icon = instance_icon(None, None, &[], false, None);
        assert_eq!(icon.builtin(), Some(GameInstanceIconType::Grass));
    }

    #[test]
    fn the_pictures_of_the_loaders_are_read() {
        assert_eq!(
            icon_of_mod_loader(ModLoaderType::LiteLoader),
            GameInstanceIconType::Chicken
        );
        assert_eq!(
            icon_of_mod_loader(ModLoaderType::LegacyFabric),
            GameInstanceIconType::LegacyFabric
        );
        // A loader the launcher cannot name shows the command block.
        assert_eq!(
            icon_of_mod_loader(ModLoaderType::Unknown),
            GameInstanceIconType::Command
        );
    }
}
