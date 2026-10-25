//! The libraries of a version of the game.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::download::game::Library;
use crate::util::versioning::VersionNumber;

/// Returns the libraries of a version without the duplicates.
///
/// A version that inherits from another version may name the same library
/// twice; the launcher keeps the newest version of the library. The libraries
/// that are about different systems are both kept, because a version may name
/// a library for every system; the files of one version of a library that are
/// not the same, such as the file of the library and the file of its native
/// libraries, are both kept as well.
pub fn unique_libraries(libraries: &[Library]) -> Vec<Library> {
    let mut result: Vec<Library> = Vec::new();
    let mut indexes: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for library in libraries {
        let Ok(artifact) = library.artifact() else {
            // A library the launcher cannot read is kept; the launcher tells
            // the player about it when it reads the files of the libraries.
            result.push(library.clone());
            continue;
        };

        let key = format!("{}:{}", artifact.group, artifact.name);
        let positions = indexes.entry(key).or_default();

        if positions.is_empty() {
            positions.push(result.len());
            result.push(library.clone());
            continue;
        }

        let mut duplicate = false;
        for &position in positions.iter() {
            let other = &result[position];

            // The rules tell the libraries about different systems apart; a
            // version may name the same library for every system.
            if other.rules != library.rules {
                continue;
            }

            let Ok(other_artifact) = other.artifact() else {
                continue;
            };
            let comparison = VersionNumber::parse(&artifact.version)
                .cmp(&VersionNumber::parse(&other_artifact.version));

            match comparison {
                Ordering::Greater => {
                    // The library of the version the launcher reads later is
                    // the newer one.
                    result[position] = library.clone();
                }
                Ordering::Equal if *library == *other => {
                    // The same file named twice; the file with more data is
                    // kept, so the launcher downloads the file it can verify.
                    if serialized_length(library) > serialized_length(other) {
                        result[position] = library.clone();
                    }
                }
                Ordering::Equal => {
                    // The same library of the same version in different files,
                    // such as the file of the library and its native files.
                    continue;
                }
                Ordering::Less => {}
            }

            duplicate = true;
            break;
        }

        if !duplicate {
            positions.push(result.len());
            result.push(library.clone());
        }
    }

    result
}

/// Returns the length of the text of a library.
fn serialized_length(library: &Library) -> usize {
    serde_json::to_string(library)
        .map(|text| text.len())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{
        CompatibilityRule, DownloadInfo, LibraryDownloads, OsRule, RuleAction,
    };
    use std::collections::BTreeMap;

    fn library(name: &str) -> Library {
        Library {
            name: name.to_string(),
            ..Library::default()
        }
    }

    fn rule(system: &str) -> CompatibilityRule {
        CompatibilityRule {
            action: RuleAction::Allow,
            os: Some(OsRule {
                name: Some(system.to_string()),
                version: None,
                arch: None,
            }),
            features: None,
        }
    }

    #[test]
    fn the_newest_version_of_a_library_is_kept() {
        let libraries = vec![
            library("com.example:library:1.0"),
            library("com.example:other:1.0"),
            library("com.example:library:2.0"),
        ];
        let unique = unique_libraries(&libraries);

        assert_eq!(unique.len(), 2);
        assert_eq!(unique[0].name, "com.example:library:2.0");
        assert_eq!(unique[1].name, "com.example:other:1.0");
    }

    #[test]
    fn the_older_version_of_a_library_is_dropped() {
        let libraries = vec![
            library("com.example:library:2.0"),
            library("com.example:library:1.0"),
        ];
        let unique = unique_libraries(&libraries);

        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0].name, "com.example:library:2.0");
    }

    #[test]
    fn the_libraries_about_different_systems_are_kept() {
        let mut windows = library("com.example:library:1.0");
        windows.rules = Some(vec![rule("windows")]);
        let mut mac = library("com.example:library:1.0");
        mac.rules = Some(vec![rule("osx")]);

        let unique = unique_libraries(&[windows, mac]);

        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn the_files_of_a_library_of_one_version_are_kept() {
        let mut library = library("org.lwjgl:lwjgl:3.3.1");
        library.downloads = Some(LibraryDownloads {
            artifact: Some(DownloadInfo {
                url: "https://example.com/lwjgl.jar".to_string(),
                ..DownloadInfo::default()
            }),
            classifiers: None,
        });
        let mut natives = library.clone();
        natives.natives = Some(BTreeMap::from([(
            "windows".to_string(),
            "natives-windows".to_string(),
        )]));

        let unique = unique_libraries(&[library, natives]);

        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn the_same_library_is_kept_once() {
        let libraries = vec![
            library("com.example:library:1.0"),
            library("com.example:library:1.0"),
        ];

        assert_eq!(unique_libraries(&libraries).len(), 1);
    }

    #[test]
    fn a_library_that_cannot_be_read_is_kept() {
        let unique = unique_libraries(&[library("broken"), library("broken")]);

        assert_eq!(unique.len(), 2);
    }
}
