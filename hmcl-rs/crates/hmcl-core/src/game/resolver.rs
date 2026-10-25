//! The resolution of the instances that inherit from other versions.
//!
//! The loaders publish their versions as changes to the version of the game
//! they work with: the manifest of an instance holds the changes and the
//! identifier of the version it inherits from. The resolver applies the
//! changes of the whole chain, so the launcher starts the game from one
//! manifest.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use crate::download::game::{InstanceManifest, InstancePatch};

/// The error of resolving an instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// The version the instance inherits from is not in the game directory.
    MissingVersion {
        /// The identifier of the version that was asked for.
        id: String,
        /// The identifier of the version that could not be found.
        missing: String,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingVersion { id, missing } => write!(
                formatter,
                "The version {id} inherits from {missing}, which is not in the game directory"
            ),
        }
    }
}

impl std::error::Error for ResolveError {}

/// The resolver of the instances of a game directory.
///
/// The resolver holds the manifests of the versions of the game directory and
/// applies the changes of the versions to the instances that inherit from
/// them.
pub struct InstanceResolver<'a> {
    /// The manifests of the versions, by their identifiers.
    manifests: &'a BTreeMap<String, InstanceManifest>,
}

impl<'a> InstanceResolver<'a> {
    /// Creates a resolver for the versions of a game directory.
    pub fn new(manifests: &'a BTreeMap<String, InstanceManifest>) -> Self {
        Self { manifests }
    }

    /// Resolves an instance.
    ///
    /// The changes of every version the instance inherits from are applied,
    /// and the patches of the instance are applied in the order of their
    /// priorities.
    pub fn resolve(&self, id: &str) -> Result<InstanceManifest, ResolveError> {
        let manifest = self
            .manifests
            .get(id)
            .ok_or_else(|| ResolveError::MissingVersion {
                id: id.to_string(),
                missing: id.to_string(),
            })?;

        self.resolve_manifest(manifest, &mut HashSet::new())
    }

    /// Resolves a manifest, keeping the versions that were visited.
    fn resolve_manifest(
        &self,
        manifest: &InstanceManifest,
        visited: &mut HashSet<String>,
    ) -> Result<InstanceManifest, ResolveError> {
        let mut resolved = match &manifest.inherits_from {
            None => manifest.clone(),
            Some(parent_id) => {
                let id = manifest.id.clone().unwrap_or_default();

                // The versions of a directory may inherit from each other in
                // a cycle when the files were edited by hand; the launcher
                // stops at the second visit instead of looping.
                if !visited.insert(id.clone()) {
                    let mut manifest = manifest.clone();
                    manifest.inherits_from = None;
                    manifest.jar = manifest.jar.or(Some(id));
                    return Ok(manifest);
                }

                let parent =
                    self.manifests
                        .get(parent_id)
                        .ok_or_else(|| ResolveError::MissingVersion {
                            id: id.clone(),
                            missing: parent_id.clone(),
                        })?;

                let parent = self.resolve_manifest(parent, visited)?;
                manifest.merge(&parent)
            }
        };

        // The patches of the repository of a modpack are applied after the
        // changes of the versions, the smallest priorities first.
        let mut patches: Vec<&InstancePatch> = manifest.patches.iter().collect();
        patches.sort_by_key(|patch| patch.priority);
        for patch in patches {
            resolved.apply_patch(patch);
        }

        resolved.id = manifest.id.clone();
        resolved.inherits_from = None;
        // The patches are applied to the resolved manifest, so the resolved
        // version holds no patch any more; the launcher reads the components
        // from the libraries of the version, not from the patches.
        resolved.patches.clear();
        if resolved.jar.is_none() {
            resolved.jar = resolved.id.clone();
        }

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::Library;

    fn library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            ..Library::default()
        }
    }

    fn manifest(id: &str) -> InstanceManifest {
        InstanceManifest {
            id: Some(id.to_string()),
            root: Some(true),
            ..InstanceManifest::default()
        }
    }

    #[test]
    fn the_changes_of_the_versions_are_applied() {
        let vanilla = InstanceManifest {
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            assets: Some("1.20".to_string()),
            libraries: vec![library("com.mojang:client:1.20")],
            ..manifest("1.20.1")
        };
        let mut fabric = manifest("fabric-loader-0.15-1.20.1");
        fabric.inherits_from = Some("1.20.1".to_string());
        fabric.main_class = Some("net.fabricmc.loader.impl.launch.knot.KnotClient".to_string());
        fabric.libraries = vec![library("net.fabricmc:fabric-loader:0.15")];

        let mut manifests = BTreeMap::new();
        manifests.insert("1.20.1".to_string(), vanilla);
        manifests.insert("fabric-loader-0.15-1.20.1".to_string(), fabric);

        let resolved = InstanceResolver::new(&manifests)
            .resolve("fabric-loader-0.15-1.20.1")
            .expect("the instance should be resolved");

        assert_eq!(resolved.id.as_deref(), Some("fabric-loader-0.15-1.20.1"));
        assert_eq!(resolved.inherits_from, None);
        assert_eq!(
            resolved.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        assert_eq!(resolved.assets.as_deref(), Some("1.20"));
        assert_eq!(resolved.jar.as_deref(), Some("1.20.1"));

        let names: Vec<&str> = resolved
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["net.fabricmc:fabric-loader:0.15", "com.mojang:client:1.20"]
        );
    }

    #[test]
    fn the_chain_of_the_versions_is_resolved() {
        let vanilla = manifest("1.20.1");
        let mut forge = manifest("forge-47.1.0-1.20.1");
        forge.inherits_from = Some("1.20.1".to_string());
        forge.libraries = vec![library("net.minecraftforge:forge:1.20.1-47.1.0")];
        let mut instance = manifest("my-instance");
        instance.inherits_from = Some("forge-47.1.0-1.20.1".to_string());
        instance.libraries = vec![library("com.example:mod:1.0")];

        let mut manifests = BTreeMap::new();
        manifests.insert("1.20.1".to_string(), vanilla);
        manifests.insert("forge-47.1.0-1.20.1".to_string(), forge);
        manifests.insert("my-instance".to_string(), instance);

        let resolved = InstanceResolver::new(&manifests)
            .resolve("my-instance")
            .expect("the instance should be resolved");

        let names: Vec<&str> = resolved
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "com.example:mod:1.0",
                "net.minecraftforge:forge:1.20.1-47.1.0"
            ]
        );
        assert_eq!(resolved.jar.as_deref(), Some("1.20.1"));
    }

    #[test]
    fn the_patches_are_applied_in_the_order_of_their_priorities() {
        let mut instance = manifest("my-instance");
        instance.patches = vec![
            InstancePatch {
                id: Some("second".to_string()),
                priority: 2,
                main_class: Some("second.Main".to_string()),
                ..InstancePatch::default()
            },
            InstancePatch {
                id: Some("first".to_string()),
                priority: 1,
                main_class: Some("first.Main".to_string()),
                libraries: vec![library("com.example:first:1.0")],
                ..InstancePatch::default()
            },
        ];

        let mut manifests = BTreeMap::new();
        manifests.insert("my-instance".to_string(), instance);

        let resolved = InstanceResolver::new(&manifests)
            .resolve("my-instance")
            .expect("the instance should be resolved");

        assert_eq!(resolved.main_class.as_deref(), Some("second.Main"));
        assert_eq!(resolved.libraries.len(), 1);
        // The patches are applied to the resolved version, so it holds none.
        assert!(resolved.patches.is_empty());
    }

    #[test]
    fn a_missing_version_is_refused() {
        let mut instance = manifest("my-instance");
        instance.inherits_from = Some("1.20.1".to_string());

        let mut manifests = BTreeMap::new();
        manifests.insert("my-instance".to_string(), instance);

        let error = InstanceResolver::new(&manifests)
            .resolve("my-instance")
            .expect_err("the instance should be refused");

        assert_eq!(
            error,
            ResolveError::MissingVersion {
                id: "my-instance".to_string(),
                missing: "1.20.1".to_string(),
            }
        );
    }

    #[test]
    fn a_cycle_of_the_versions_is_stopped() {
        let mut first = manifest("first");
        first.inherits_from = Some("second".to_string());
        let mut second = manifest("second");
        second.inherits_from = Some("first".to_string());

        let mut manifests = BTreeMap::new();
        manifests.insert("first".to_string(), first);
        manifests.insert("second".to_string(), second);

        let resolved = InstanceResolver::new(&manifests)
            .resolve("first")
            .expect("the instance should be resolved");

        assert_eq!(resolved.id.as_deref(), Some("first"));
        assert_eq!(resolved.inherits_from, None);
    }
}
