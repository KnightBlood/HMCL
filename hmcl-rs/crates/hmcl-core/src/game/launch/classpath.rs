//! The files the game reads its classes and its native libraries from.

use std::path::{Path, PathBuf};

use crate::download::game::{rules_allow, InstanceManifest, Library, RuleContext};

/// Returns the character that separates the files of a classpath.
///
/// The systems of Windows separate the files with a semicolon; the other
/// systems with a colon.
pub const fn classpath_separator() -> char {
    if cfg!(windows) {
        ';'
    } else {
        ':'
    }
}

/// Joins the files of a classpath into one text.
pub fn join_classpath(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<String>>()
        .join(&classpath_separator().to_string())
}

/// Returns whether the library holds the files of the native libraries.
///
/// The versions of the game either name the classifiers of the native files in
/// the `natives` map, or write the classifiers into the downloads.
pub fn is_native(library: &Library) -> bool {
    library.natives.is_some()
        || library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.classifiers.as_ref())
            .is_some_and(|classifiers| classifiers.keys().any(|name| name.starts_with("native")))
}

/// Returns the classifier of the native files of the library.
///
/// The versions name the native files after the system and the architecture;
/// the modpacks write the system and the architecture together, such as
/// `natives-windows-64`. The launcher reads the descriptors the versions and
/// the modpacks use.
pub fn native_classifier(library: &Library, context: &RuleContext<'_>) -> Option<String> {
    // A library may name its native files in its name, such as
    // `com.example:library:1.0:natives-windows`.
    if let Ok(artifact) = library.artifact() {
        if let Some(classifier) = artifact.classifier {
            return Some(classifier);
        }
    }

    if let Some(natives) = &library.natives {
        let bits = arch_bits(context.arch);
        for descriptor in native_descriptors(context.os_name, context.arch) {
            if let Some(classifier) = natives.get(&descriptor) {
                return Some(classifier.replace("${arch}", bits));
            }
        }
        return None;
    }

    let classifiers = library.downloads.as_ref()?.classifiers.as_ref()?;
    native_descriptors(context.os_name, context.arch)
        .into_iter()
        .find(|descriptor| classifiers.contains_key(descriptor))
}

/// Returns the descriptions of the native files of a system.
///
/// A description is written as `[natives-]system[-architecture]`, where the
/// architecture is the name of the architecture or the number of its bits.
fn native_descriptors(os: &str, arch: &str) -> Vec<String> {
    let bits = arch_bits(arch);
    let mut descriptors = Vec::new();

    for key in ["", arch, bits] {
        for variant in ["", "native-", "natives-"] {
            let mut descriptor = String::from(variant);
            descriptor.push_str(os);
            if !key.is_empty() {
                descriptor.push('-');
                descriptor.push_str(key);
            }
            if !descriptors.contains(&descriptor) {
                descriptors.push(descriptor);
            }
        }
    }

    descriptors
}

/// Returns the number of the bits of the architecture.
fn arch_bits(arch: &str) -> &'static str {
    if arch == "x86_64" || arch == "aarch64" {
        "64"
    } else {
        "32"
    }
}

/// Returns the files of the libraries the game reads its classes from.
///
/// The libraries about other systems are skipped, and so are the native
/// libraries: the game reads them from the directory the launcher extracts
/// them to.
pub fn library_paths(
    manifest: &InstanceManifest,
    libraries_directory: &Path,
    context: &RuleContext<'_>,
) -> Vec<PathBuf> {
    super::libraries::unique_libraries(&manifest.libraries)
        .iter()
        .filter(|library| rules_allow(library.rules.as_deref(), context) && !is_native(library))
        .filter_map(|library| library.artifact().ok())
        .map(|artifact| libraries_directory.join(artifact.path()))
        .collect()
}

/// Returns the classpath of the game.
///
/// The classpath holds the libraries the game reads its classes from and the
/// file of the game itself; the file of the game is kept last.
pub fn classpath(
    manifest: &InstanceManifest,
    libraries_directory: &Path,
    client_jar: &Path,
    context: &RuleContext<'_>,
) -> Vec<PathBuf> {
    let mut paths = library_paths(manifest, libraries_directory, context);
    if !paths.iter().any(|path| path == client_jar) {
        paths.push(client_jar.to_path_buf());
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{CompatibilityRule, OsRule, RuleAction};
    use std::collections::BTreeMap;

    fn library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            ..Library::default()
        }
    }

    fn windows_rule() -> CompatibilityRule {
        CompatibilityRule {
            action: RuleAction::Allow,
            os: Some(OsRule {
                name: Some("windows".to_string()),
                version: None,
                arch: None,
            }),
            features: None,
        }
    }

    fn context(features: &BTreeMap<String, bool>) -> RuleContext<'_> {
        RuleContext::new("windows", "10.0", "x86_64", features)
    }

    #[test]
    fn the_files_are_separated_the_way_the_system_names_them() {
        let separator = if cfg!(windows) { ';' } else { ':' };
        assert_eq!(classpath_separator(), separator);

        let paths = vec![PathBuf::from("a.jar"), PathBuf::from("b.jar")];
        assert_eq!(join_classpath(&paths), format!("a.jar{separator}b.jar"));
    }

    #[test]
    fn the_classpath_holds_the_libraries_and_the_game() {
        let manifest = InstanceManifest {
            libraries: vec![
                library("com.example:first:1.0"),
                library("com.example:second:1.0"),
            ],
            ..InstanceManifest::default()
        };
        let features = BTreeMap::new();
        let paths = classpath(
            &manifest,
            Path::new("libraries"),
            Path::new("libraries/client.jar"),
            &context(&features),
        );

        assert_eq!(
            paths,
            vec![
                PathBuf::from("libraries/com/example/first/1.0/first-1.0.jar"),
                PathBuf::from("libraries/com/example/second/1.0/second-1.0.jar"),
                PathBuf::from("libraries/client.jar"),
            ]
        );
    }

    #[test]
    fn the_classpath_holds_the_newest_version_of_a_library() {
        let manifest = InstanceManifest {
            libraries: vec![
                library("com.example:library:1.0"),
                library("com.example:library:2.0"),
            ],
            ..InstanceManifest::default()
        };
        let features = BTreeMap::new();

        assert_eq!(
            library_paths(&manifest, Path::new("libraries"), &context(&features)),
            vec![PathBuf::from(
                "libraries/com/example/library/2.0/library-2.0.jar"
            )]
        );
    }

    #[test]
    fn the_libraries_of_other_systems_are_skipped() {
        let mut mac = library("com.example:mac:1.0");
        mac.rules = Some(vec![CompatibilityRule {
            action: RuleAction::Allow,
            os: Some(OsRule {
                name: Some("osx".to_string()),
                version: None,
                arch: None,
            }),
            features: None,
        }]);

        let mut windows = library("com.example:windows:1.0");
        windows.rules = Some(vec![windows_rule()]);

        let manifest = InstanceManifest {
            libraries: vec![mac, windows],
            ..InstanceManifest::default()
        };
        let features = BTreeMap::new();
        let paths = library_paths(&manifest, Path::new("libraries"), &context(&features));

        assert_eq!(
            paths,
            vec![PathBuf::from(
                "libraries/com/example/windows/1.0/windows-1.0.jar"
            )]
        );
    }

    #[test]
    fn the_native_libraries_stay_out_of_the_classpath() {
        let mut library = library("org.lwjgl:lwjgl:3.3.1");
        library.natives = Some(BTreeMap::from([(
            "windows".to_string(),
            "natives-windows-${arch}".to_string(),
        )]));

        let manifest = InstanceManifest {
            libraries: vec![library],
            ..InstanceManifest::default()
        };
        let features = BTreeMap::new();

        assert!(library_paths(&manifest, Path::new("libraries"), &context(&features)).is_empty());
        assert_eq!(
            classpath(
                &manifest,
                Path::new("libraries"),
                Path::new("client.jar"),
                &context(&features)
            ),
            vec![PathBuf::from("client.jar")]
        );
    }

    #[test]
    fn the_native_classifier_reads_the_architecture() {
        let mut lwjgl = library("org.lwjgl:lwjgl:3.3.1");
        lwjgl.natives = Some(BTreeMap::from([(
            "windows".to_string(),
            "natives-windows-${arch}".to_string(),
        )]));
        let features = BTreeMap::new();

        assert_eq!(
            native_classifier(&lwjgl, &context(&features)).as_deref(),
            Some("natives-windows-64")
        );

        let mut lwjgl = library("org.lwjgl:lwjgl:3.3.1");
        lwjgl.natives = Some(BTreeMap::from([(
            "windows-64".to_string(),
            "natives-windows-x86_64".to_string(),
        )]));
        assert_eq!(
            native_classifier(&lwjgl, &context(&features)).as_deref(),
            Some("natives-windows-x86_64")
        );
    }
}
