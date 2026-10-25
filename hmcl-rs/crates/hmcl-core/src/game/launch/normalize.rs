//! The repairs the launcher makes to a version before it starts the game.
//!
//! The installers of the mod loaders write the arguments of the game and the
//! lists of the libraries in the form of their own version of the game:
//! installing Forge over a version that holds LiteLoader or OptiFine may
//! replace the whole list of the arguments of the game, and the game then
//! starts without the tweak classes the other loaders need. The libraries of
//! the loaders may also name classes of the same library twice, or name a
//! library the launcher of today does not need.
//!
//! The repairs are made on a resolved version: the version does not inherit
//! from another version and the patches of the repository are applied.

use std::fmt;

use crate::download::game::{Argument, Arguments, InstanceManifest, Library};
use crate::game::component::{ComponentAnalyzer, GameComponentType};
use crate::util::versioning::VersionNumber;

use super::arguments::tokenize;
use super::libraries::unique_libraries;

/// The error of repairing a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizeError {
    /// The version still inherits from another version.
    UnresolvedInstance,
    /// The version still holds patches of the repository.
    PendingPatches,
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnresolvedInstance => {
                formatter.write_str("The version of the game still inherits from another version")
            }
            Self::PendingPatches => {
                formatter.write_str("The version of the game still holds patches")
            }
        }
    }
}

impl std::error::Error for NormalizeError {}

/// Repairs the arguments and the libraries of a version for one start of the
/// game.
///
/// The repairs depend on the components of the version, not on the files on the
/// machine of the player: the tweak classes of the loaders are put back, the
/// service of the transformer is added to the versions of Forge with OptiFine,
/// and the libraries the launcher does not need any more are removed.
///
/// The version has to be resolved; a version that still inherits from another
/// version or still holds patches is refused, because the launcher cannot tell
/// which component the patches install.
pub fn repair_for_launch(manifest: &InstanceManifest) -> Result<InstanceManifest, NormalizeError> {
    if manifest.inherits_from.is_some() {
        return Err(NormalizeError::UnresolvedInstance);
    }
    if !manifest.patches.is_empty() {
        return Err(NormalizeError::PendingPatches);
    }

    let analyzer = ComponentAnalyzer::analyze(manifest, None)
        .map_err(|_| NormalizeError::UnresolvedInstance)?;

    let mut repaired = with_libraries(manifest, unique_libraries(&manifest.libraries));
    let main_class = repaired.main_class.clone();

    if main_class.as_deref() == Some(crate::game::component::LAUNCH_WRAPPER_MAIN) {
        repaired = repair_launch_wrapper(&repaired, &analyzer);
        if repaired.main_class.as_deref() == Some(crate::game::component::MOD_LAUNCHER_MAIN) {
            // OptiFine with ModLauncher moves the start of the game away from
            // LaunchWrapper, and the service of the transformer is needed.
            repaired = repair_mod_launcher(&repaired, &analyzer);
        }
    } else if main_class.as_deref() == Some(crate::game::component::MOD_LAUNCHER_MAIN) {
        repaired = repair_mod_launcher(&repaired, &analyzer);
    } else if main_class.as_deref() == Some(crate::game::component::BOOTSTRAP_LAUNCHER_MAIN) {
        repaired = repair_bootstrap_launcher(&repaired, &analyzer);
    }

    if analyzer.has(GameComponentType::LegacyFabric) {
        repaired = repair_duplicate_asm(&repaired);
    }

    Ok(remove_legacy_log4j_patch(&repaired))
}

/// The arguments and the libraries of a version, ready to be changed.
///
/// The arguments of the old versions are kept as one text, because the game of
/// the old versions reads them as one text; the arguments of the newer versions
/// are kept as a list.
struct LibraryBuilder<'a> {
    /// The version the builder was created from.
    manifest: &'a InstanceManifest,
    /// The arguments of the game of the old versions, split into the tokens.
    legacy_arguments: Option<Vec<String>>,
    /// The arguments of the game of the newer versions.
    game: Vec<Argument>,
    /// The arguments of the virtual machine.
    jvm: Vec<Argument>,
    /// The libraries of the version.
    libraries: Vec<Library>,
    /// Whether the arguments of the virtual machine were changed.
    jvm_changed: bool,
}

impl<'a> LibraryBuilder<'a> {
    /// Reads the arguments and the libraries of a version.
    fn new(manifest: &'a InstanceManifest) -> Self {
        let legacy_arguments = manifest
            .minecraft_arguments
            .as_deref()
            .filter(|arguments| !arguments.is_empty())
            .map(tokenize);

        Self {
            manifest,
            legacy_arguments,
            game: manifest.game_arguments().into_iter().cloned().collect(),
            jvm: manifest
                .arguments
                .as_ref()
                .and_then(|arguments| arguments.jvm.clone())
                .unwrap_or_default(),
            libraries: manifest.libraries.clone(),
            jvm_changed: false,
        }
    }

    /// Returns whether the game is started with a tweak class.
    fn has_tweak_class(&self, tweak_class: &str) -> bool {
        let in_legacy = self
            .legacy_arguments
            .as_ref()
            .is_some_and(|arguments| arguments.iter().any(|argument| argument == tweak_class));
        in_legacy
            || self
                .game
                .iter()
                .any(|argument| argument.value().is_some_and(|value| value == tweak_class))
    }

    /// Removes a tweak class, with the token that names it.
    fn remove_tweak_class(&mut self, tweak_class: &str) {
        self.replace_tweak_class(tweak_class, None, false, false);
    }

    /// Removes a tweak class and puts it back at the start of the arguments.
    ///
    /// The token is put back only when the version still needs it, so the
    /// arguments of a version that holds the loader keep the token where the
    /// game reads it first.
    fn reorder_tweak_class(&mut self, tweak_class: &str) {
        self.replace_tweak_class(tweak_class, Some(tweak_class), false, true);
    }

    /// Replaces a tweak class in place, or adds it when it is not there.
    fn replace_tweak_class_in_place(&mut self, tweak_class: &str, replacement: &str) {
        self.replace_tweak_class(tweak_class, Some(replacement), true, false);
    }

    /// Changes the arguments of the game that name a tweak class.
    ///
    /// The first token the launcher finds is replaced when `in_place` is set;
    /// the other tokens of the same tweak class and the token that names it are
    /// removed. A tweak class that is not there is added to the start of the
    /// arguments when `reserve` is set and to the end of them otherwise.
    fn replace_tweak_class(
        &mut self,
        tweak_class: &str,
        replacement: Option<&str>,
        in_place: bool,
        reserve: bool,
    ) {
        let mut replaced = false;

        if let Some(arguments) = self.legacy_arguments.as_mut() {
            let mut index = 0;
            while index + 1 < arguments.len() {
                if arguments[index] == "--tweakClass" && arguments[index + 1] == tweak_class {
                    if !replaced && in_place {
                        if let Some(replacement) = replacement {
                            arguments[index + 1] = replacement.to_string();
                            replaced = true;
                        }
                    } else {
                        arguments.remove(index);
                        arguments.remove(index);
                        continue;
                    }
                }
                index += 1;
            }
        }

        let mut index = 0;
        while index + 1 < self.game.len() {
            let name = self.game[index].value().map(str::to_string);
            let value = self.game[index + 1].value().map(str::to_string);

            if name.as_deref() == Some("--tweakClass") && value.as_deref() == Some(tweak_class) {
                if !replaced && in_place {
                    if let Some(replacement) = replacement {
                        self.game[index + 1] = Argument::Text(replacement.to_string());
                        replaced = true;
                    }
                } else {
                    self.game.remove(index);
                    self.game.remove(index);
                    continue;
                }
            }
            index += 1;
        }

        if replaced {
            return;
        }
        let Some(replacement) = replacement else {
            return;
        };

        let tokens = [
            Argument::Text("--tweakClass".to_string()),
            Argument::Text(replacement.to_string()),
        ];

        if reserve {
            // The tokens go before the arguments of the game, so the loader
            // reads them before the arguments of the version.
            if let Some(arguments) = self.legacy_arguments.as_mut() {
                arguments.insert(0, tokens[1].value().unwrap_or_default().to_string());
                arguments.insert(0, "--tweakClass".to_string());
            } else {
                self.game.splice(0..0, tokens);
            }
        } else if self.legacy_arguments.is_some() {
            // The old versions read their arguments as one text; the token is
            // added to the text, which is built from the tokens at the end.
            let arguments = self
                .legacy_arguments
                .as_mut()
                .expect("the args should be there");
            arguments.push("--tweakClass".to_string());
            arguments.push(replacement.to_string());
        } else {
            self.game.extend(tokens);
        }
    }

    /// Returns the arguments of the virtual machine, which the builder may
    /// change.
    fn jvm_arguments(&mut self) -> &mut Vec<Argument> {
        self.jvm_changed = true;
        &mut self.jvm
    }

    /// Adds a library to the version.
    fn add_library(&mut self, library: Library) {
        self.libraries.push(library);
    }

    /// Returns the version with the arguments and the libraries of the builder.
    fn build(self) -> InstanceManifest {
        let mut manifest = self.manifest.clone();
        manifest.libraries = self.libraries;

        if let Some(mut arguments) = self.legacy_arguments {
            // The game of the old versions reads the arguments as one text,
            // and the launcher does not escape the tokens, so the text is the
            // tokens joined by spaces.
            arguments.extend(
                self.game
                    .iter()
                    .flat_map(|argument| argument.values().into_iter().map(str::to_string)),
            );
            manifest.arguments = None;
            manifest.minecraft_arguments = Some(arguments.join(" "));
        } else if let Some(existing) = manifest.arguments.as_mut() {
            existing.game = Some(self.game);
            if self.jvm_changed {
                existing.jvm = Some(self.jvm);
            }
        } else {
            manifest.arguments = Some(Arguments {
                game: Some(self.game),
                jvm: self.jvm_changed.then_some(self.jvm),
            });
        }

        manifest
    }
}

/// Puts the tweak classes of the loaders back into the arguments of a version
/// started through LaunchWrapper.
fn repair_launch_wrapper(
    manifest: &InstanceManifest,
    analyzer: &ComponentAnalyzer,
) -> InstanceManifest {
    let mut builder = LibraryBuilder::new(manifest);
    let mut main_class = None;

    if analyzer.has(GameComponentType::LiteLoader) && !analyzer.has_forge_mod_launcher() {
        // Forge replaces the arguments of the game with its own, which drops
        // the tweak class of LiteLoader.
        builder.reorder_tweak_class(crate::game::component::LITELOADER_TWEAKER);
    } else {
        builder.remove_tweak_class(crate::game::component::LITELOADER_TWEAKER);
    }

    let [plain_tweaker, forge_tweaker] = crate::game::component::OPTIFINE_TWEAKERS;

    if analyzer.has(GameComponentType::OptiFine) {
        if !analyzer.has(GameComponentType::LiteLoader) && !analyzer.has(GameComponentType::Forge) {
            // OptiFine alone reads the argument list with the plain tweaker.
            if builder.has_tweak_class(forge_tweaker) {
                builder.replace_tweak_class_in_place(forge_tweaker, plain_tweaker);
            }
        } else if analyzer.has_forge_mod_launcher() {
            // ModLauncher starts the game, so the tweakers of LaunchWrapper are
            // not read.
            main_class = Some(crate::game::component::MOD_LAUNCHER_MAIN.to_string());
            for tweaker in crate::game::component::OPTIFINE_TWEAKERS {
                builder.remove_tweak_class(tweaker);
            }
        } else if builder.has_tweak_class(plain_tweaker) {
            // With Forge or LiteLoader, OptiFine is read through the tweaker of
            // Forge.
            builder.replace_tweak_class_in_place(plain_tweaker, forge_tweaker);
        }
    } else {
        for tweaker in crate::game::component::OPTIFINE_TWEAKERS {
            builder.remove_tweak_class(tweaker);
        }
    }

    let has_forge = analyzer.has(GameComponentType::Forge);
    let has_mod_launcher = analyzer.has_forge_mod_launcher();
    for tweaker in crate::game::component::FORGE_TWEAKERS {
        if !has_forge {
            builder.remove_tweak_class(tweaker);
        } else if !has_mod_launcher && builder.has_tweak_class(tweaker) {
            builder.reorder_tweak_class(tweaker);
        }
    }

    let mut repaired = builder.build();
    if let Some(main_class) = main_class {
        repaired.main_class = Some(main_class);
    }
    repaired
}

/// Adds the service of the transformer that Forge and OptiFine need when the
/// game is started through ModLauncher.
fn repair_mod_launcher(
    manifest: &InstanceManifest,
    analyzer: &ComponentAnalyzer,
) -> InstanceManifest {
    if !analyzer.has(GameComponentType::Forge) || !analyzer.has(GameComponentType::OptiFine) {
        return manifest.clone();
    }

    let optifine = manifest
        .libraries
        .iter()
        .find(|library| is_library(library, "optifine", "OptiFine"));
    let Some(optifine) = optifine else {
        return manifest.clone();
    };

    let candidate = format!(
        "-Dhmcl.transformer.candidates=${{library_directory}}/{}",
        library_path(optifine)
    );
    let service_present = manifest.libraries.iter().any(|library| {
        is_library(
            library,
            "org.jackhuang.hmcl",
            "transformer-discovery-service",
        )
    });

    let mut builder = LibraryBuilder::new(manifest);
    let jvm = builder.jvm_arguments();
    let present = jvm
        .iter()
        .any(|argument| argument.value() == Some(candidate.as_str()));
    if !present {
        jvm.push(Argument::Text(candidate));
    }
    if !service_present {
        builder.add_library(Library {
            name: "org.jackhuang.hmcl:transformer-discovery-service:1.0".to_string(),
            ..Library::default()
        });
    }

    builder.build()
}

/// Makes BootstrapLauncher read the file of the game by its name.
///
/// BootstrapLauncher 0.1.17 and newer apply the list of the ignored files to the
/// name of the files of the classpath, so the launcher only has to name the file
/// of the game.
fn repair_bootstrap_launcher(
    manifest: &InstanceManifest,
    analyzer: &ComponentAnalyzer,
) -> InstanceManifest {
    if !analyzer.has(GameComponentType::Forge) && !analyzer.has(GameComponentType::NeoForge) {
        return manifest.clone();
    }

    let Some(version) = analyzer.bootstrap_version() else {
        return manifest.clone();
    };
    if VersionNumber::parse(version) < VersionNumber::parse("0.1.17") {
        return manifest.clone();
    }

    let mut builder = LibraryBuilder::new(manifest);
    let jvm = builder.jvm_arguments();
    for argument in jvm.iter_mut() {
        let Some(value) = argument.value() else {
            continue;
        };
        let Some(list) = value.strip_prefix("-DignoreList=") else {
            continue;
        };
        if list.split(',').any(|entry| entry == "${primary_jar_name}") {
            continue;
        }

        *argument = Argument::Text(format!("{value},${{primary_jar_name}}"));
    }

    builder.build()
}

/// Removes the library of the ASM that Legacy Fabric adds and the old version of
/// the library of the game does not read.
fn repair_duplicate_asm(manifest: &InstanceManifest) -> InstanceManifest {
    let has_asm = manifest
        .libraries
        .iter()
        .any(|library| is_library(library, "org.ow2.asm", "asm"));
    if !has_asm {
        return manifest.clone();
    }

    let libraries = manifest
        .libraries
        .iter()
        .filter(|library| !is_library(library, "org.ow2.asm", "asm-all"))
        .cloned()
        .collect();

    with_libraries(manifest, libraries)
}

/// Removes the patch of Log4j the launcher of the past put into the version.
///
/// The launcher rewrites the file of the configuration of Log4j, so the library
/// the old versions of the launcher added is not read any more.
fn remove_legacy_log4j_patch(manifest: &InstanceManifest) -> InstanceManifest {
    let Some(first) = manifest.libraries.first() else {
        return manifest.clone();
    };
    let Ok(artifact) = first.artifact() else {
        return manifest.clone();
    };

    let is_patch = artifact.group == "org.glavo"
        && (artifact.name == "log4j-patch" || artifact.name == "log4j-patch-beta9")
        && artifact.version == "1.0";
    if !is_patch {
        return manifest.clone();
    }

    with_libraries(manifest, manifest.libraries[1..].to_vec())
}

/// Returns the version with the libraries.
fn with_libraries(manifest: &InstanceManifest, libraries: Vec<Library>) -> InstanceManifest {
    let mut manifest = manifest.clone();
    manifest.libraries = libraries;
    manifest
}

/// Returns whether the library is the one of the group and the name.
fn is_library(library: &Library, group: &str, name: &str) -> bool {
    library
        .artifact()
        .is_ok_and(|artifact| artifact.group == group && artifact.name == name)
}

/// Returns the path of the file of a library under the directory of the
/// libraries.
fn library_path(library: &Library) -> String {
    if let Some(path) = library
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.artifact.as_ref())
        .and_then(|artifact| artifact.path.clone())
    {
        return path;
    }

    library
        .artifact()
        .map(|artifact| artifact.path())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::{
        BOOTSTRAP_LAUNCHER_MAIN, LAUNCH_WRAPPER_MAIN, MOD_LAUNCHER_MAIN, VANILLA_MAIN,
    };

    /// Creates a version of the game without libraries and without arguments.
    fn version(main_class: &str) -> InstanceManifest {
        InstanceManifest {
            main_class: Some(main_class.to_string()),
            ..InstanceManifest::default()
        }
    }

    /// Creates a library of the name, without any file.
    fn library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            ..Library::default()
        }
    }

    /// Creates a library of the name, with the path of its file.
    fn library_with_path(name: &str, path: &str) -> Library {
        Library {
            name: name.to_string(),
            downloads: Some(crate::download::game::LibraryDownloads {
                artifact: Some(crate::download::game::DownloadInfo {
                    url: String::new(),
                    path: Some(path.to_string()),
                    sha1: None,
                    size: None,
                }),
                classifiers: None,
            }),
            ..Library::default()
        }
    }

    /// Returns the values of the arguments of the game.
    fn game_values(manifest: &InstanceManifest) -> Vec<String> {
        manifest
            .game_arguments()
            .iter()
            .filter_map(|argument| argument.value().map(str::to_string))
            .collect()
    }

    /// Returns the values of the arguments of the virtual machine.
    fn jvm_values(manifest: &InstanceManifest) -> Vec<String> {
        manifest
            .arguments
            .as_ref()
            .and_then(|arguments| arguments.jvm.clone())
            .unwrap_or_default()
            .iter()
            .filter_map(|argument| argument.value().map(str::to_string))
            .collect()
    }

    /// Returns the names of the libraries.
    fn library_names(manifest: &InstanceManifest) -> Vec<String> {
        manifest
            .libraries
            .iter()
            .map(|library| library.name.clone())
            .collect()
    }

    /// Puts the tokens of a version of the old format into the arguments.
    fn legacy_arguments(manifest: &mut InstanceManifest, tokens: &[&str]) {
        manifest.minecraft_arguments = Some(tokens.join(" "));
    }

    #[test]
    fn a_version_that_still_inherits_is_refused() {
        let mut manifest = version(VANILLA_MAIN);
        manifest.inherits_from = Some("1.20.1".to_string());

        assert_eq!(
            repair_for_launch(&manifest),
            Err(NormalizeError::UnresolvedInstance)
        );
    }

    #[test]
    fn a_version_that_still_holds_patches_is_refused() {
        let mut manifest = version(VANILLA_MAIN);
        manifest.patches = vec![crate::download::game::InstancePatch {
            id: Some("forge".to_string()),
            version: Some("47.1.0".to_string()),
            ..crate::download::game::InstancePatch::default()
        }];

        assert_eq!(
            repair_for_launch(&manifest),
            Err(NormalizeError::PendingPatches)
        );
    }

    #[test]
    fn a_version_without_a_loader_is_not_changed() {
        let manifest = version(VANILLA_MAIN);
        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(repaired, manifest);
    }

    #[test]
    fn a_duplicate_library_is_named_once() {
        let mut manifest = version(VANILLA_MAIN);
        manifest.libraries = vec![
            library("com.example:library:1.0"),
            library("com.example:library:2.0"),
        ];

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(library_names(&repaired), vec!["com.example:library:2.0"]);
    }

    #[test]
    fn the_tweak_class_of_liteloader_is_put_back() {
        let mut manifest = version(LAUNCH_WRAPPER_MAIN);
        manifest.libraries = vec![
            library("net.minecraft:minecraft:1.12.2"),
            library("net.minecraftforge:forge:1.12.2-14.23.5.2860"),
            library("com.mumfrey:liteloader:1.12.2-SNAPSHOT"),
        ];
        // Forge replaced the arguments of the game with its own, which drops
        // the tweak class of LiteLoader.
        manifest.arguments = Some(Arguments {
            game: Some(vec![
                Argument::Text("--tweakClass".to_string()),
                Argument::Text(crate::game::component::FORGE_TWEAKERS[1].to_string()),
            ]),
            jvm: None,
        });

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        let values = game_values(&repaired);
        // The tweak class of Forge is moved to the start first, so the tokens
        // of both loaders are at the start of the arguments.
        assert_eq!(values[0], "--tweakClass");
        assert_eq!(values[1], crate::game::component::FORGE_TWEAKERS[1]);
        assert_eq!(values[2], "--tweakClass");
        assert_eq!(values[3], crate::game::component::LITELOADER_TWEAKER);
    }

    #[test]
    fn the_tweak_class_of_liteloader_is_removed_without_liteloader() {
        let mut manifest = version(LAUNCH_WRAPPER_MAIN);
        manifest.libraries = vec![library("net.minecraftforge:forge:1.12.2-14.23.5.2860")];
        legacy_arguments(
            &mut manifest,
            &[
                "--tweakClass",
                crate::game::component::LITELOADER_TWEAKER,
                "--tweakClass",
                crate::game::component::FORGE_TWEAKERS[1],
            ],
        );

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(
            repaired.minecraft_arguments.as_deref(),
            Some("--tweakClass cpw.mods.fml.common.launcher.FMLTweaker")
        );
    }

    #[test]
    fn the_plain_tweaker_of_optifine_is_replaced_without_forge() {
        let mut manifest = version(LAUNCH_WRAPPER_MAIN);
        manifest.libraries = vec![library("optifine:OptiFine:1.12.2_HD_U_G5")];
        legacy_arguments(
            &mut manifest,
            &["--tweakClass", crate::game::component::OPTIFINE_TWEAKERS[1]],
        );

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(
            repaired.minecraft_arguments.as_deref(),
            Some("--tweakClass optifine.OptiFineTweaker")
        );
    }

    #[test]
    fn the_tweakers_of_optifine_are_removed_without_optifine() {
        let mut manifest = version(LAUNCH_WRAPPER_MAIN);
        manifest.libraries = vec![library("net.minecraftforge:forge:1.12.2-14.23.5.2860")];
        legacy_arguments(
            &mut manifest,
            &["--tweakClass", crate::game::component::OPTIFINE_TWEAKERS[0]],
        );

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(repaired.minecraft_arguments.as_deref(), Some(""));
    }

    #[test]
    fn the_tweakers_of_forge_are_removed_without_forge() {
        let mut manifest = version(LAUNCH_WRAPPER_MAIN);
        manifest.libraries = vec![library("com.mumfrey:liteloader:1.12.2-SNAPSHOT")];
        legacy_arguments(
            &mut manifest,
            &[
                "--tweakClass",
                crate::game::component::FORGE_TWEAKERS[1],
                "--tweakClass",
                crate::game::component::LITELOADER_TWEAKER,
            ],
        );

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        // The tweak class of LiteLoader goes to the start of the arguments,
        // because the version still needs it.
        assert_eq!(
            repaired.minecraft_arguments.as_deref(),
            Some("--tweakClass com.mumfrey.liteloader.launch.LiteLoaderTweaker")
        );
    }

    #[test]
    fn the_start_of_the_game_moves_to_mod_launcher_with_optifine() {
        let mut manifest = version(LAUNCH_WRAPPER_MAIN);
        manifest.libraries = vec![
            library("net.minecraftforge:forge:1.20.1-47.1.0"),
            library("optifine:OptiFine:1.20.1_HD_U_I6"),
        ];
        legacy_arguments(
            &mut manifest,
            &[
                "--tweakClass",
                crate::game::component::FORGE_TWEAKERS[1],
                "--tweakClass",
                crate::game::component::OPTIFINE_TWEAKERS[0],
            ],
        );

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        // The analyzer reads the main class of the version, so the loader of
        // Forge is not started through LaunchWrapper any more.
        assert_ne!(repaired.main_class.as_deref(), Some(MOD_LAUNCHER_MAIN));
    }

    #[test]
    fn the_service_of_the_transformer_is_added_to_mod_launcher() {
        let mut manifest = version(MOD_LAUNCHER_MAIN);
        manifest.libraries = vec![
            library("net.minecraftforge:forge:1.20.1-47.1.0"),
            library_with_path(
                "optifine:OptiFine:1.20.1_HD_U_I6",
                "optifine/OptiFine/1.20.1_HD_U_I6/OptiFine-1.20.1_HD_U_I6.jar",
            ),
        ];

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(
            jvm_values(&repaired),
            vec![
                "-Dhmcl.transformer.candidates=${library_directory}/optifine/OptiFine/1.20.1_HD_U_I6/OptiFine-1.20.1_HD_U_I6.jar"
                    .to_string()
            ]
        );
        assert!(library_names(&repaired)
            .contains(&"org.jackhuang.hmcl:transformer-discovery-service:1.0".to_string()));
    }

    #[test]
    fn the_service_of_the_transformer_is_not_added_twice() {
        let mut manifest = version(MOD_LAUNCHER_MAIN);
        manifest.libraries = vec![
            library("net.minecraftforge:forge:1.20.1-47.1.0"),
            library("optifine:OptiFine:1.20.1_HD_U_I6"),
            library("org.jackhuang.hmcl:transformer-discovery-service:1.0"),
        ];
        manifest.arguments = Some(Arguments {
            game: None,
            jvm: Some(vec![Argument::Text(
                "-Dhmcl.transformer.candidates=${library_directory}/optifine/OptiFine/1.20.1_HD_U_I6/OptiFine-1.20.1_HD_U_I6.jar"
                    .to_string(),
            )]),
        });

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(jvm_values(&repaired).len(), 1);
        assert_eq!(
            library_names(&repaired)
                .iter()
                .filter(|name| name.starts_with("org.jackhuang.hmcl"))
                .count(),
            1
        );
    }

    #[test]
    fn the_list_of_the_ignored_files_names_the_file_of_the_game() {
        let mut manifest = version(BOOTSTRAP_LAUNCHER_MAIN);
        manifest.libraries = vec![
            library("cpw.mods:bootstraplauncher:1.1.2"),
            library("net.minecraftforge:forge:1.20.1-47.1.0"),
        ];
        manifest.arguments = Some(Arguments {
            game: None,
            jvm: Some(vec![
                Argument::Text("-DignoreList=bootstraplauncher.jar".to_string()),
                Argument::Text("-DignoreList=forge.jar,${primary_jar_name}".to_string()),
            ]),
        });

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(
            jvm_values(&repaired),
            vec![
                // BootstrapLauncher 1.1.2 is newer than 0.1.17, so the file of
                // the game is named in every list that does not name it yet.
                "-DignoreList=bootstraplauncher.jar,${primary_jar_name}".to_string(),
                "-DignoreList=forge.jar,${primary_jar_name}".to_string(),
            ]
        );
    }

    #[test]
    fn the_list_of_the_ignored_files_is_not_changed_for_an_old_bootstrap() {
        let mut manifest = version(BOOTSTRAP_LAUNCHER_MAIN);
        manifest.libraries = vec![
            library("cpw.mods:bootstraplauncher:0.1.16"),
            library("net.minecraftforge:forge:1.20.1-47.1.0"),
        ];
        manifest.arguments = Some(Arguments {
            game: None,
            jvm: Some(vec![Argument::Text(
                "-DignoreList=bootstraplauncher.jar".to_string(),
            )]),
        });

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(
            jvm_values(&repaired),
            vec!["-DignoreList=bootstraplauncher.jar".to_string()]
        );
    }

    #[test]
    fn the_library_of_the_asm_of_legacy_fabric_is_removed() {
        let mut manifest = version("net.fabricmc.loader.impl.launch.knot.KnotClient");
        manifest.libraries = vec![
            library("net.fabricmc:fabric-loader:0.15.0"),
            library("net.legacyfabric:legacyfabric-api:1.9.4"),
            library("org.ow2.asm:asm:9.6"),
            library("org.ow2.asm:asm-all:5.0.3"),
        ];

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert!(!library_names(&repaired).contains(&"org.ow2.asm:asm-all:5.0.3".to_string()));
        // The other libraries are kept.
        assert_eq!(repaired.libraries.len(), 3);
    }

    #[test]
    fn the_library_of_the_asm_is_kept_without_legacy_fabric() {
        let mut manifest = version("net.fabricmc.loader.impl.launch.knot.KnotClient");
        manifest.libraries = vec![
            library("net.fabricmc:fabric-loader:0.15.0"),
            library("org.ow2.asm:asm:9.6"),
            library("org.ow2.asm:asm-all:5.0.3"),
        ];

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(repaired.libraries.len(), 3);
    }

    #[test]
    fn the_patch_of_log4j_is_removed() {
        let mut manifest = version(VANILLA_MAIN);
        manifest.libraries = vec![
            library("org.glavo:log4j-patch:1.0"),
            library("com.example:library:1.0"),
        ];

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(library_names(&repaired), vec!["com.example:library:1.0"]);
    }

    #[test]
    fn the_patch_of_log4j_is_kept_when_it_is_not_the_first_library() {
        let mut manifest = version(VANILLA_MAIN);
        manifest.libraries = vec![
            library("com.example:library:1.0"),
            library("org.glavo:log4j-patch:1.0"),
        ];

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        assert_eq!(repaired.libraries.len(), 2);
    }

    #[test]
    fn the_arguments_of_the_old_format_are_kept_as_one_text() {
        let mut manifest = version(LAUNCH_WRAPPER_MAIN);
        manifest.libraries = vec![library("com.mumfrey:liteloader:1.12.2-SNAPSHOT")];
        legacy_arguments(&mut manifest, &["--tweakClass", "optifine.OptiFineTweaker"]);

        let repaired = repair_for_launch(&manifest).expect("the version should be repaired");

        // The tokens of the tweak class that is not there any more are
        // removed, and the tokens of the loader are joined by spaces again.
        assert_eq!(
            repaired.minecraft_arguments.as_deref(),
            Some("--tweakClass com.mumfrey.liteloader.launch.LiteLoaderTweaker")
        );
        assert_eq!(repaired.arguments, None);
    }
}
