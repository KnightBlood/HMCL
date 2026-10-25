//! The coordinates of a library.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The coordinates of a library.
///
/// The libraries of the game are named the way the repositories of Java name
/// them: `group:name:version`, with an optional classifier and an optional
/// extension.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Artifact {
    /// The group the library belongs to.
    pub group: String,
    /// The name of the library.
    pub name: String,
    /// The version of the library.
    pub version: String,
    /// The classifier of the library, such as `natives-windows`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classifier: Option<String>,
    /// The extension of the file of the library.
    pub extension: String,
}

impl Artifact {
    /// Creates the coordinates of a library.
    pub fn new(
        group: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        classifier: Option<String>,
        extension: Option<String>,
    ) -> Self {
        let group = group.into();
        let name = name.into();
        let version = version.into();
        let extension = extension.unwrap_or_else(|| "jar".to_string());

        Self {
            group,
            name,
            version,
            classifier,
            extension,
        }
    }

    /// Reads the coordinates of a library from its name.
    ///
    /// The name is written as `group:name:version`, with an optional
    /// classifier and an optional extension:
    ///
    /// ```text
    /// com.example:library:1.0
    /// com.example:library:1.0:natives-windows
    /// com.example:library:1.0:natives-windows@zip
    /// ```
    pub fn from_descriptor(descriptor: &str) -> Result<Self, String> {
        let mut parts: Vec<&str> = descriptor.splitn(4, ':').collect();
        if parts.len() < 3 {
            return Err(format!(
                "The name of the library is malformed: {descriptor}"
            ));
        }

        let mut extension = None;
        let last = parts.len() - 1;
        let pieces: Vec<&str> = parts[last].split('@').collect();
        match pieces.len() {
            1 => {}
            2 => {
                parts[last] = pieces[0];
                extension = Some(pieces[1].to_string());
            }
            _ => {
                return Err(format!(
                    "The name of the library is malformed: {descriptor}"
                ))
            }
        }

        Ok(Self::new(
            parts[0].replace('\\', "/"),
            parts[1],
            parts[2],
            parts.get(3).map(|classifier| classifier.to_string()),
            extension,
        ))
    }

    /// Returns the name of the file of the library.
    pub fn file_name(&self) -> String {
        let mut name = format!("{}-{}", self.name, self.version);
        if let Some(classifier) = &self.classifier {
            name.push('-');
            name.push_str(classifier);
        }
        name.push('.');
        name.push_str(&self.extension);
        name
    }

    /// Returns the path of the file of the library, relative to the root of
    /// the repositories.
    pub fn path(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.group.replace('.', "/"),
            self.name,
            self.version,
            self.file_name()
        )
    }

    /// Returns the name of the library, as it is written in the versions of
    /// the game.
    pub fn descriptor(&self) -> String {
        let mut descriptor = format!("{}:{}:{}", self.group, self.name, self.version);
        if let Some(classifier) = &self.classifier {
            descriptor.push(':');
            descriptor.push_str(classifier);
        }
        if self.extension != "jar" {
            descriptor.push('@');
            descriptor.push_str(&self.extension);
        }
        descriptor
    }
}

impl fmt::Display for Artifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.descriptor())
    }
}

impl std::str::FromStr for Artifact {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::from_descriptor(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_of_the_libraries_are_read() {
        let artifact =
            Artifact::from_descriptor("com.example:library:1.0").expect("the name should be read");
        assert_eq!(artifact.group, "com.example");
        assert_eq!(artifact.name, "library");
        assert_eq!(artifact.version, "1.0");
        assert_eq!(artifact.classifier, None);
        assert_eq!(artifact.extension, "jar");
    }

    #[test]
    fn the_classifiers_and_the_extensions_are_read() {
        let artifact = Artifact::from_descriptor("com.example:library:1.0:natives-windows@zip")
            .expect("the name should be read");
        assert_eq!(artifact.classifier.as_deref(), Some("natives-windows"));
        assert_eq!(artifact.extension, "zip");
    }

    #[test]
    fn the_names_that_are_malformed_are_refused() {
        assert!(Artifact::from_descriptor("com.example:library").is_err());
        assert!(Artifact::from_descriptor("com.example:library:1.0:a@b@c").is_err());
    }

    #[test]
    fn the_files_of_the_libraries_are_named() {
        let artifact = Artifact::new("com.example", "library", "1.0", None, None);
        assert_eq!(artifact.file_name(), "library-1.0.jar");
        assert_eq!(artifact.path(), "com/example/library/1.0/library-1.0.jar");
        assert_eq!(artifact.descriptor(), "com.example:library:1.0");
        assert_eq!(artifact.to_string(), "com.example:library:1.0");

        let artifact = Artifact::new(
            "com.example",
            "library",
            "1.0",
            Some("natives".into()),
            Some("zip".into()),
        );
        assert_eq!(artifact.file_name(), "library-1.0-natives.zip");
        assert_eq!(artifact.descriptor(), "com.example:library:1.0:natives@zip");
    }
}
