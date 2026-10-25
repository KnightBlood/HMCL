//! The files of the libraries of a version of the game.

use crate::download::game::{Library, RuleContext};

use super::classpath::{is_native, native_classifier};

/// The address the libraries that do not name their repository are downloaded
/// from.
pub const DEFAULT_LIBRARY_URL: &str = "https://libraries.minecraft.net/";

/// A file of a library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryFile {
    /// The path of the file under the directory of the libraries.
    pub path: String,
    /// The address the file is downloaded from.
    ///
    /// The libraries that are kept on the machine of the player, such as the
    /// libraries the player installed, are not downloaded; the address of such
    /// a library is not known.
    pub url: Option<String>,
    /// The digest of the file, in the hexadecimal of SHA-1.
    pub sha1: Option<String>,
    /// The size of the file, in bytes.
    pub size: Option<u64>,
    /// The classifier of the file, if the file is not the file of the library
    /// itself.
    pub classifier: Option<String>,
    /// Whether the file holds the native libraries.
    pub native: bool,
}

/// Returns the file of a library, if the library has a file on the system of
/// the player.
///
/// The libraries that name the native files of another system have no file on
/// the system of the player; [`super::classpath::native_classifier`] tells the
/// classifier of the native files the system reads.
pub fn library_file(library: &Library, context: &RuleContext<'_>) -> Option<LibraryFile> {
    let artifact = library.artifact().ok()?;
    let native = is_native(library);
    let classifier = native_classifier(library, context);

    // The native libraries name the file of the system the game runs on; a
    // native library without such a file has nothing to download.
    if native && classifier.is_none() {
        return None;
    }

    let download = if native {
        classifier.as_ref().and_then(|classifier| {
            library
                .downloads
                .as_ref()?
                .classifiers
                .as_ref()?
                .get(classifier)
        })
    } else {
        library
            .downloads
            .as_ref()
            .and_then(|downloads| downloads.artifact.as_ref())
    };

    let path = match download.and_then(|download| download.path.clone()) {
        Some(path) => path,
        None => {
            let mut artifact = artifact;
            artifact.classifier = classifier.clone();
            artifact.path()
        }
    };

    let url = download
        .map(|download| download.url.clone())
        .filter(|url| !url.is_empty())
        .or_else(|| {
            // The old libraries name the repository they are kept in; the
            // libraries that name neither a file nor a repository are kept on
            // the machine of the player.
            library.url.as_deref().map(|repository| {
                let mut url = repository.to_string();
                if !url.ends_with('/') {
                    url.push('/');
                }
                url.push_str(&path);
                url
            })
        });

    Some(LibraryFile {
        path,
        url,
        sha1: download.and_then(|download| download.sha1().map(str::to_string)),
        size: download.and_then(|download| download.size),
        classifier,
        native,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::{DownloadInfo, LibraryDownloads};
    use std::collections::BTreeMap;

    fn context(features: &BTreeMap<String, bool>) -> RuleContext<'_> {
        RuleContext::new("windows", "10.0", "x86_64", features)
    }

    fn download(path: &str) -> DownloadInfo {
        DownloadInfo {
            url: format!("https://example.com/{path}"),
            path: Some(path.to_string()),
            sha1: Some("abc".to_string()),
            size: Some(42),
        }
    }

    #[test]
    fn the_file_of_a_library_is_read() {
        let library = Library {
            name: "com.example:library:1.0".to_string(),
            downloads: Some(LibraryDownloads {
                artifact: Some(download("com/example/library/1.0/library-1.0.jar")),
                classifiers: None,
            }),
            ..Library::default()
        };
        let features = BTreeMap::new();
        let file = library_file(&library, &context(&features)).expect("the file should be read");

        assert_eq!(file.path, "com/example/library/1.0/library-1.0.jar");
        assert_eq!(
            file.url.as_deref(),
            Some("https://example.com/com/example/library/1.0/library-1.0.jar")
        );
        assert_eq!(file.sha1.as_deref(), Some("abc"));
        assert_eq!(file.size, Some(42));
        assert!(!file.native);
        assert_eq!(file.classifier, None);
    }

    #[test]
    fn the_file_of_an_old_library_is_read() {
        let library = Library {
            name: "com.example:library:1.0".to_string(),
            url: Some("https://maven.example.com/".to_string()),
            ..Library::default()
        };
        let features = BTreeMap::new();
        let file = library_file(&library, &context(&features)).expect("the file should be read");

        assert_eq!(file.path, "com/example/library/1.0/library-1.0.jar");
        assert_eq!(
            file.url.as_deref(),
            Some("https://maven.example.com/com/example/library/1.0/library-1.0.jar")
        );
    }

    #[test]
    fn a_library_that_is_kept_on_the_machine_is_not_downloaded() {
        let library = Library {
            name: "com.example:library:1.0".to_string(),
            hint: Some("local".to_string()),
            ..Library::default()
        };
        let features = BTreeMap::new();
        let file = library_file(&library, &context(&features)).expect("the file should be read");

        assert_eq!(file.url, None);
    }

    #[test]
    fn the_file_of_a_native_library_is_read() {
        let library = Library {
            name: "org.lwjgl:lwjgl:3.3.1".to_string(),
            natives: Some(BTreeMap::from([(
                "windows".to_string(),
                "natives-windows".to_string(),
            )])),
            downloads: Some(LibraryDownloads {
                artifact: Some(download("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar")),
                classifiers: Some(BTreeMap::from([(
                    "natives-windows".to_string(),
                    download("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"),
                )])),
            }),
            ..Library::default()
        };
        let features = BTreeMap::new();
        let file = library_file(&library, &context(&features)).expect("the file should be read");

        assert!(file.native);
        assert_eq!(file.classifier.as_deref(), Some("natives-windows"));
        assert_eq!(
            file.path,
            "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"
        );
    }

    #[test]
    fn a_native_library_without_a_file_for_the_system_is_skipped() {
        let library = Library {
            name: "org.lwjgl:lwjgl:3.3.1".to_string(),
            natives: Some(BTreeMap::from([(
                "osx".to_string(),
                "natives-osx".to_string(),
            )])),
            ..Library::default()
        };
        let features = BTreeMap::new();

        assert_eq!(library_file(&library, &context(&features)), None);
    }
}
