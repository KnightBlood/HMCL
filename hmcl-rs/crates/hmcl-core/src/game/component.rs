//! The components of a version of the game.
//!
//! A version of the game may hold a mod loader, OptiFine or a loader of the
//! old versions of the game. The launcher recognizes the components by the
//! libraries of the version and by the patches the repository published, so it
//! knows which version of every component the instance runs.

use std::collections::BTreeMap;

use crate::addon::AddonLoader;
use crate::download::game::{Arguments, InstanceManifest, Library};

/// The main class of the game without a mod loader.
pub const VANILLA_MAIN: &str = "net.minecraft.client.main.Main";
/// The main class of the versions started through LaunchWrapper.
pub const LAUNCH_WRAPPER_MAIN: &str = "net.minecraft.launchwrapper.Launch";
/// The main class of the versions started through ModLauncher.
pub const MOD_LAUNCHER_MAIN: &str = "cpw.mods.modlauncher.Launcher";
/// The main class of the versions started through BootstrapLauncher.
pub const BOOTSTRAP_LAUNCHER_MAIN: &str = "cpw.mods.bootstraplauncher.BootstrapLauncher";
/// The main class of the versions started through the bootstrap of Forge.
pub const FORGE_BOOTSTRAP_MAIN: &str = "net.minecraftforge.bootstrap.ForgeBootstrap";
/// The main class of the versions started through the bootstrap of NeoForge.
pub const NEO_FORGE_BOOTSTRAP_MAIN: &str = "net.neoforged.fml.startup.Client";

/// The tweak classes of Forge, in the order of the versions of the game.
pub const FORGE_TWEAKERS: [&str; 3] = [
    "net.minecraftforge.legacy._1_5_2.LibraryFixerTweaker",
    "cpw.mods.fml.common.launcher.FMLTweaker",
    "net.minecraftforge.fml.common.launcher.FMLTweaker",
];

/// The tweak classes of OptiFine, the tweaker of the game first.
pub const OPTIFINE_TWEAKERS: [&str; 2] =
    ["optifine.OptiFineTweaker", "optifine.OptiFineForgeTweaker"];

/// The tweak class of LiteLoader.
pub const LITELOADER_TWEAKER: &str = "com.mumfrey.liteloader.launch.LiteLoaderTweaker";

/// The packages of the main classes of the mod loaders.
pub const MOD_LOADER_MAIN_CLASSES_PACKAGES: [&str; 6] = [
    "net.minecraftforge",
    "net.neoforged",
    "top.outlands",
    "net.fabricmc",
    "org.quiltmc",
    "cpw.mods",
];

/// A part of a version of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GameComponentType {
    /// The game itself.
    Game,
    /// The loader of Legacy Fabric.
    LegacyFabric,
    /// The API of Legacy Fabric.
    LegacyFabricApi,
    /// The loader of Fabric.
    Fabric,
    /// The API of Fabric.
    FabricApi,
    /// Forge.
    Forge,
    /// Cleanroom.
    Cleanroom,
    /// NeoForge.
    NeoForge,
    /// LiteLoader.
    LiteLoader,
    /// OptiFine.
    OptiFine,
    /// The loader of Quilt.
    Quilt,
    /// The API of Quilt.
    QuiltApi,
}

impl GameComponentType {
    /// Every component the launcher recognizes.
    pub const ALL: [Self; 12] = [
        Self::Game,
        Self::LegacyFabric,
        Self::LegacyFabricApi,
        Self::Fabric,
        Self::FabricApi,
        Self::Forge,
        Self::Cleanroom,
        Self::NeoForge,
        Self::LiteLoader,
        Self::OptiFine,
        Self::Quilt,
        Self::QuiltApi,
    ];

    /// Returns the identifier the patches of the repository use.
    pub const fn patch_id(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::LegacyFabric => "legacyfabric",
            Self::LegacyFabricApi => "legacyfabric-api",
            Self::Fabric => "fabric",
            Self::FabricApi => "fabric-api",
            Self::Forge => "forge",
            Self::Cleanroom => "cleanroom",
            Self::NeoForge => "neoforge",
            Self::LiteLoader => "liteloader",
            Self::OptiFine => "optifine",
            Self::Quilt => "quilt",
            Self::QuiltApi => "quilt-api",
        }
    }

    /// Returns the component with the identifier of a patch.
    pub fn from_patch_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|component| component.patch_id() == id)
    }

    /// Returns the name of the mod loader of the component.
    pub fn mod_loader(self) -> Option<&'static str> {
        match self {
            Self::LegacyFabric => Some("legacy-fabric"),
            Self::Fabric => Some(AddonLoader::FABRIC),
            Self::Forge => Some(AddonLoader::FORGE),
            Self::Cleanroom => Some("cleanroom"),
            Self::NeoForge => Some(AddonLoader::NEO_FORGE),
            Self::LiteLoader => Some(AddonLoader::LITE_LOADER),
            Self::Quilt => Some(AddonLoader::QUILT),
            _ => None,
        }
    }

    /// Returns whether the component is a mod loader.
    pub fn is_mod_loader(self) -> bool {
        self.mod_loader().is_some()
    }

    /// Returns whether the library is the part of the version the component is
    /// recognized by.
    pub fn matches_library(self, library: &Library, libraries: &[Library]) -> bool {
        let Some(artifact) = library.artifact().ok() else {
            return false;
        };

        match self {
            Self::Game => false,
            Self::LegacyFabric => {
                is(&artifact, "net.fabricmc", "fabric-loader")
                    && libraries
                        .iter()
                        .any(|library| group_is(library, "net.legacyfabric"))
            }
            Self::LegacyFabricApi => is(&artifact, "net.legacyfabric", "legacyfabric-api"),
            Self::Fabric => {
                is(&artifact, "net.fabricmc", "fabric-loader")
                    && !libraries
                        .iter()
                        .any(|library| group_is(library, "net.legacyfabric"))
            }
            Self::FabricApi => is(&artifact, "net.fabricmc", "fabric-api"),
            Self::Forge => {
                artifact.group == "net.minecraftforge"
                    && matches!(
                        artifact.name.as_str(),
                        "forge" | "fmlloader" | "minecraftforge"
                    )
                    && !libraries
                        .iter()
                        .any(|library| Self::NeoForge.matches_library(library, libraries))
            }
            Self::Cleanroom => is(&artifact, "com.cleanroommc", "cleanroom"),
            Self::NeoForge => {
                artifact.group == "net.neoforged.fancymodloader"
                    && matches!(artifact.name.as_str(), "core" | "loader")
            }
            Self::LiteLoader => is(&artifact, "com.mumfrey", "liteloader"),
            Self::OptiFine => {
                matches!(artifact.group.as_str(), "net.optifine" | "optifine")
                    && !artifact.name.contains("launchwrapper")
            }
            Self::Quilt => is(&artifact, "org.quiltmc", "quilt-loader"),
            Self::QuiltApi => is(&artifact, "org.quiltmc", "quilt-api"),
        }
    }

    /// Returns the version of the component the library tells.
    ///
    /// Forge and NeoForge name the version of the game in the version of the
    /// library, so the version of the loader is read from the library or from
    /// the arguments of the version.
    pub fn component_version(
        self,
        manifest: &InstanceManifest,
        library_version: &str,
    ) -> Option<String> {
        match self {
            Self::NeoForge => scan_loaders(manifest)
                .or_else(|| forge_version(library_version))
                .or_else(|| Some(library_version.to_string())),
            Self::Forge => {
                forge_version(library_version).or_else(|| Some(library_version.to_string()))
            }
            _ => Some(library_version.to_string()),
        }
    }
}

/// Returns whether the library has the group and the name.
fn is(artifact: &crate::download::game::Artifact, group: &str, name: &str) -> bool {
    artifact.group == group && artifact.name == name
}

/// Returns whether the library belongs to the group.
fn group_is(library: &Library, group: &str) -> bool {
    library
        .artifact()
        .map(|artifact| artifact.group == group)
        .unwrap_or(false)
}

/// Returns the version of a loader from the version of its library.
///
/// The loaders name the versions of their libraries after the version of the
/// game and the version of the loader, such as `1.20.1-47.1.0`.
fn forge_version(library_version: &str) -> Option<String> {
    let mut parts = library_version.split('-');
    let game = parts.next()?;
    let loader = parts.next()?;

    let numeric = |text: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|character| character.is_ascii_digit() || character == '.')
    };

    if numeric(game) && numeric(loader) {
        Some(loader.to_string())
    } else {
        Some(library_version.to_string())
    }
}

/// Reads the version of the loader from the arguments of a version.
///
/// NeoForge passes the version of the loader to the game, and the
/// repositories of the modpacks keep the version only there.
fn scan_loaders(manifest: &InstanceManifest) -> Option<String> {
    scan_arguments(manifest.arguments.as_ref()).or_else(|| {
        manifest
            .patches
            .iter()
            .find_map(|patch| scan_arguments(patch.arguments.as_ref()))
    })
}

/// Reads the version of the loader from the arguments of the game.
fn scan_arguments(arguments: Option<&Arguments>) -> Option<String> {
    let game = arguments?.game.as_ref()?;

    for pair in game.windows(2) {
        let (Some(flag), Some(value)) = (pair[0].value(), pair[1].value()) else {
            continue;
        };
        if flag == "--fml.neoForgeVersion" || flag == "--fml.forgeVersion" {
            return Some(value.to_string());
        }
    }
    None
}

/// A component of a version of the game, with its version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    /// The component.
    pub component: GameComponentType,
    /// The version of the component.
    pub version: String,
    /// Whether the version was read from a patch instead of a library.
    pub from_patch: bool,
}

/// The components of a version of the game.
#[derive(Debug, Clone, Default)]
pub struct ComponentAnalyzer {
    /// The components of the version, by their kinds.
    marks: BTreeMap<GameComponentType, Mark>,
    /// The version of the bootstrap of the loader.
    bootstrap_version: Option<String>,
    /// The main class of the version.
    main_class: Option<String>,
}

/// The error of analyzing a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyzeError {
    /// The version still inherits from another version.
    UnresolvedInstance,
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnresolvedInstance => {
                formatter.write_str("The version still inherits from another version")
            }
        }
    }
}

impl std::error::Error for AnalyzeError {}

impl ComponentAnalyzer {
    /// Reads the components of a version of the game.
    ///
    /// The version must be resolved; the patches of the repository are read
    /// first and the libraries are read only for the components the patches do
    /// not name.
    pub fn analyze(
        manifest: &InstanceManifest,
        game_version: Option<&str>,
    ) -> Result<Self, AnalyzeError> {
        if manifest.inherits_from.is_some() {
            return Err(AnalyzeError::UnresolvedInstance);
        }

        let mut marks = BTreeMap::new();

        if let Some(version) = game_version {
            if version != "unknown" {
                marks.insert(
                    GameComponentType::Game,
                    Mark {
                        component: GameComponentType::Game,
                        version: version.to_string(),
                        from_patch: true,
                    },
                );
            }
        }

        // The patches of the repository tell the components exactly.
        for patch in &manifest.patches {
            if patch.hidden {
                continue;
            }
            let Some(id) = &patch.id else {
                continue;
            };
            let Some(component) = GameComponentType::from_patch_id(id) else {
                continue;
            };

            marks.insert(
                component,
                Mark {
                    component,
                    version: patch.version.clone().unwrap_or_default(),
                    from_patch: true,
                },
            );
        }

        let mut bootstrap_version = None;
        for library in &manifest.libraries {
            for component in GameComponentType::ALL {
                if marks.contains_key(&component) {
                    continue;
                }

                if component.matches_library(library, &manifest.libraries) {
                    let library_version = library
                        .artifact()
                        .map(|artifact| artifact.version)
                        .unwrap_or_default();
                    let version = component
                        .component_version(manifest, &library_version)
                        .unwrap_or_default();
                    marks.insert(
                        component,
                        Mark {
                            component,
                            version,
                            from_patch: false,
                        },
                    );
                    break;
                }
            }

            if bootstrap_version.is_none() {
                if let Ok(artifact) = library.artifact() {
                    if artifact.group == "cpw.mods" && artifact.name == "bootstraplauncher" {
                        bootstrap_version = Some(artifact.version);
                    }
                }
            }
        }

        Ok(Self {
            marks,
            bootstrap_version,
            main_class: manifest.main_class.clone(),
        })
    }

    /// Returns whether the version holds the component.
    pub fn has(&self, component: GameComponentType) -> bool {
        self.marks.contains_key(&component)
    }

    /// Returns whether the version holds the loader.
    pub fn has_mod_loader(&self, loader: &str) -> bool {
        self.marks
            .values()
            .any(|mark| mark.component.mod_loader() == Some(loader))
    }

    /// Returns the version of the component.
    pub fn version(&self, component: GameComponentType) -> Option<&str> {
        self.marks.get(&component).map(|mark| mark.version.as_str())
    }

    /// Returns the component, if the version holds it.
    pub fn mark(&self, component: GameComponentType) -> Option<&Mark> {
        self.marks.get(&component)
    }

    /// Returns the components of the version.
    pub fn marks(&self) -> impl Iterator<Item = &Mark> {
        self.marks.values()
    }

    /// Returns the version of the bootstrap of the loader.
    pub fn bootstrap_version(&self) -> Option<&str> {
        self.bootstrap_version.as_deref()
    }

    /// Returns whether the version is started through ModLauncher.
    pub fn has_forge_mod_launcher(&self) -> bool {
        self.main_class.as_deref() == Some(MOD_LAUNCHER_MAIN)
    }

    /// Returns whether the version holds a mod loader.
    ///
    /// A version started through LaunchWrapper is not modified by the
    /// launcher, even if the repository installed a loader into it.
    pub fn is_modded(&self) -> bool {
        let Some(main_class) = &self.main_class else {
            return false;
        };
        if main_class == LAUNCH_WRAPPER_MAIN {
            return false;
        }

        MOD_LOADER_MAIN_CLASSES_PACKAGES
            .iter()
            .any(|package| main_class.starts_with(package))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            ..Library::default()
        }
    }

    fn manifest(libraries: Vec<Library>, main_class: &str) -> InstanceManifest {
        InstanceManifest {
            id: Some("instance".to_string()),
            main_class: Some(main_class.to_string()),
            libraries,
            ..InstanceManifest::default()
        }
    }

    #[test]
    fn the_game_is_read_from_the_version() {
        let manifest = manifest(vec![library("com.mojang:client:1.20.1")], VANILLA_MAIN);
        let analyzer = ComponentAnalyzer::analyze(&manifest, Some("1.20.1"))
            .expect("the version should be analyzed");

        assert!(analyzer.has(GameComponentType::Game));
        assert_eq!(analyzer.version(GameComponentType::Game), Some("1.20.1"));
        assert!(!analyzer.is_modded());
        assert_eq!(analyzer.marks().count(), 1);
    }

    #[test]
    fn the_loaders_are_recognized_by_their_libraries() {
        let manifest = manifest(
            vec![
                library("net.fabricmc:fabric-loader:0.15.11"),
                library("net.fabricmc:fabric-api:0.92.2"),
            ],
            "net.fabricmc.loader.impl.launch.knot.KnotClient",
        );
        let analyzer = ComponentAnalyzer::analyze(&manifest, Some("1.20.1"))
            .expect("the version should be analyzed");

        assert_eq!(analyzer.version(GameComponentType::Fabric), Some("0.15.11"));
        assert!(analyzer.has(GameComponentType::FabricApi));
        assert!(!analyzer.has(GameComponentType::LegacyFabric));
        assert!(analyzer.has_mod_loader("fabric"));
        assert!(analyzer.is_modded());
    }

    #[test]
    fn the_legacy_fabric_is_told_apart_from_fabric() {
        let manifest = manifest(
            vec![
                library("net.fabricmc:fabric-loader:0.15.11"),
                library("net.legacyfabric:legacyfabric-api:0.92.2"),
            ],
            "net.fabricmc.loader.impl.launch.knot.KnotClient",
        );
        let analyzer = ComponentAnalyzer::analyze(&manifest, Some("1.12.2"))
            .expect("the version should be analyzed");

        assert!(analyzer.has(GameComponentType::LegacyFabric));
        assert!(analyzer.has(GameComponentType::LegacyFabricApi));
        assert!(!analyzer.has(GameComponentType::Fabric));
    }

    #[test]
    fn the_version_of_forge_is_read_from_the_library() {
        let manifest = manifest(
            vec![library("net.minecraftforge:forge:1.20.1-47.1.0")],
            MOD_LAUNCHER_MAIN,
        );
        let analyzer = ComponentAnalyzer::analyze(&manifest, Some("1.20.1"))
            .expect("the version should be analyzed");

        assert_eq!(analyzer.version(GameComponentType::Forge), Some("47.1.0"));
        assert!(analyzer.has_forge_mod_launcher());
        assert!(analyzer.is_modded());
    }

    #[test]
    fn the_version_of_neoforge_is_read_from_the_arguments() {
        let mut manifest = manifest(
            vec![library("net.neoforged.fancymodloader:core:1.20.1-47.1.0")],
            BOOTSTRAP_LAUNCHER_MAIN,
        );
        manifest.arguments = Some(Arguments {
            game: Some(vec![
                crate::download::game::Argument::Text("--fml.neoForgeVersion".to_string()),
                crate::download::game::Argument::Text("20.4.237".to_string()),
            ]),
            jvm: None,
        });

        let analyzer = ComponentAnalyzer::analyze(&manifest, Some("1.20.1"))
            .expect("the version should be analyzed");

        assert_eq!(
            analyzer.version(GameComponentType::NeoForge),
            Some("20.4.237")
        );
        assert!(analyzer.has_mod_loader("neoforge"));
    }

    #[test]
    fn the_patches_tell_the_components_exactly() {
        let mut manifest = manifest(Vec::new(), MOD_LAUNCHER_MAIN);
        manifest.patches = vec![crate::download::game::InstancePatch {
            id: Some("forge".to_string()),
            version: Some("47.1.0".to_string()),
            ..crate::download::game::InstancePatch::default()
        }];

        let analyzer = ComponentAnalyzer::analyze(&manifest, Some("1.20.1"))
            .expect("the version should be analyzed");

        let mark = analyzer
            .mark(GameComponentType::Forge)
            .expect("the patch should be read");
        assert_eq!(mark.version, "47.1.0");
        assert!(mark.from_patch);
    }

    #[test]
    fn a_version_that_inherits_is_refused() {
        let mut manifest = manifest(Vec::new(), VANILLA_MAIN);
        manifest.inherits_from = Some("1.20.1".to_string());

        let error =
            ComponentAnalyzer::analyze(&manifest, None).expect_err("the version should be refused");
        assert_eq!(error, AnalyzeError::UnresolvedInstance);
    }

    #[test]
    fn the_versions_of_the_loaders_are_derived() {
        assert_eq!(forge_version("1.20.1-47.1.0"), Some("47.1.0".to_string()));
        assert_eq!(
            forge_version("1.7.10-10.13.4.1614"),
            Some("10.13.4.1614".to_string())
        );
        assert_eq!(forge_version("47.1.0"), None);
    }
}
