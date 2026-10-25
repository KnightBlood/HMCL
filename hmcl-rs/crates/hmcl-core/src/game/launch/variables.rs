//! The variables the arguments of a version are written with.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::auth::AuthInfo;
use crate::download::game::{InstanceManifest, ReleaseType};

use super::classpath::{classpath_separator, join_classpath};
use super::options::{LaunchOptions, LaunchPaths};

/// The values of the variables the arguments of a version are written with.
///
/// The versions write the arguments with variables instead of the values of
/// the player, so one version may be started by any player; the launcher
/// replaces the variables with the values it knows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchVariables {
    /// The values, by the names of the variables.
    values: BTreeMap<String, String>,
}

impl LaunchVariables {
    /// Creates the values of the variables of an instance.
    ///
    /// The values are the ones the versions of the game and the mod loaders
    /// read; the classpath and the paths of the files are written as the
    /// system writes them.
    pub fn for_instance(
        manifest: &InstanceManifest,
        auth: &AuthInfo,
        options: &LaunchOptions,
        paths: &LaunchPaths,
        classpath: &[PathBuf],
    ) -> Self {
        let mut variables = Self::default();

        let version_name = options
            .version_name
            .clone()
            .or_else(|| manifest.id.clone())
            .unwrap_or_default();
        let version_type = options
            .version_type
            .clone()
            .or_else(|| manifest.version_type.map(|kind| kind.id().to_string()))
            .unwrap_or_else(|| ReleaseType::Unknown.id().to_string());

        variables.set("auth_player_name", auth.username.clone());
        variables.set("auth_session", auth.access_token.clone());
        variables.set("auth_access_token", auth.access_token.clone());
        variables.set("auth_uuid", auth.uuid.compact());
        variables.set("version_name", version_name);
        variables.set(
            "profile_name",
            options
                .profile_name
                .clone()
                .unwrap_or_else(|| "Minecraft".to_string()),
        );
        variables.set("version_type", version_type);
        variables.set(
            "game_directory",
            path_text(options.working_directory(paths)),
        );
        variables.set("user_type", auth.user_type.as_str());
        variables.set("assets_index_name", paths.asset_index.clone());
        variables.set(
            "user_properties",
            auth.user_properties
                .clone()
                .unwrap_or_else(|| "{}".to_string()),
        );
        variables.set("resolution_width", options.width.to_string());
        variables.set("resolution_height", options.height.to_string());
        variables.set("library_directory", path_text(&paths.libraries_directory));
        variables.set("libraries_directory", path_text(&paths.libraries_directory));
        variables.set("classpath_separator", classpath_separator().to_string());
        variables.set("primary_jar", path_text(&paths.client_jar));
        variables.set(
            "primary_jar_name",
            paths
                .client_jar
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
        variables.set("language", options.language.clone());
        variables.set("file_separator", std::path::MAIN_SEPARATOR.to_string());
        variables.set("classpath", join_classpath(classpath));
        variables.set("game_assets", path_text(&paths.assets_directory));
        variables.set("assets_root", path_text(&paths.assets_directory));
        variables.set("natives_directory", path_text(&paths.natives_directory));
        variables.set("launcher_name", options.launcher_name.clone());
        variables.set("launcher_version", options.launcher_version.clone());

        variables
    }

    /// Sets the value of a variable.
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        self.values.insert(name.to_string(), value.into());
    }

    /// Returns the value of a variable.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    /// Returns the values, by the names of the variables.
    pub fn values(&self) -> &BTreeMap<String, String> {
        &self.values
    }

    /// Replaces the variables of a text with the values.
    ///
    /// A variable the launcher does not know is kept as it is, so the game
    /// reads the text the version wrote.
    pub fn substitute(&self, text: &str) -> String {
        substitute(text, &self.values)
    }
}

/// Replaces the variables of a text with the values.
///
/// The variables are written as `${name}`; the values of the variables the
/// launcher does not know are not replaced.
pub fn substitute(text: &str, values: &BTreeMap<String, String>) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("${") {
        let Some(offset) = rest[start + 2..].find('}') else {
            break;
        };
        let end = start + 2 + offset;

        result.push_str(&rest[..start]);
        let name = &rest[start + 2..end];
        match values.get(name) {
            Some(value) => result.push_str(value),
            None => result.push_str(&rest[start..=end]),
        }

        rest = &rest[end + 1..];
    }

    result.push_str(rest);
    result
}

/// Returns the text of a path, as the system writes it.
fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variables(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn the_variables_are_replaced() {
        let values = variables(&[("auth_player_name", "Steve"), ("version_name", "1.20.1")]);

        assert_eq!(
            substitute(
                "--username ${auth_player_name} --version ${version_name}",
                &values
            ),
            "--username Steve --version 1.20.1"
        );
    }

    #[test]
    fn the_unknown_variables_are_kept() {
        let values = variables(&[("auth_player_name", "Steve")]);

        assert_eq!(
            substitute("${unknown} ${auth_player_name}", &values),
            "${unknown} Steve"
        );
        assert_eq!(substitute("${unclosed", &values), "${unclosed");
        assert_eq!(substitute("plain text", &values), "plain text");
    }

    #[test]
    fn the_values_of_an_instance_are_read() {
        let manifest = InstanceManifest {
            id: Some("fabric-1.20.1".to_string()),
            version_type: Some(ReleaseType::Release),
            ..InstanceManifest::default()
        };
        let auth = AuthInfo::offline("Steve");
        let options = LaunchOptions::default();
        let paths = LaunchPaths {
            game_directory: PathBuf::from("game"),
            assets_directory: PathBuf::from("assets"),
            asset_index: "5".to_string(),
            natives_directory: PathBuf::from("natives"),
            libraries_directory: PathBuf::from("libraries"),
            client_jar: PathBuf::from("versions/fabric-1.20.1/fabric-1.20.1.jar"),
        };
        let classpath = vec![PathBuf::from("a.jar"), PathBuf::from("b.jar")];

        let variables =
            LaunchVariables::for_instance(&manifest, &auth, &options, &paths, &classpath);

        assert_eq!(variables.get("auth_player_name"), Some("Steve"));
        assert_eq!(variables.get("version_name"), Some("fabric-1.20.1"));
        assert_eq!(variables.get("version_type"), Some("release"));
        assert_eq!(variables.get("assets_index_name"), Some("5"));
        assert_eq!(variables.get("user_type"), Some("legacy"));
        assert_eq!(variables.get("primary_jar_name"), Some("fabric-1.20.1.jar"));
        assert_eq!(
            variables.get("classpath"),
            Some(join_classpath(&classpath).as_str())
        );
        assert!(variables
            .get("auth_uuid")
            .is_some_and(|uuid| !uuid.contains('-')));
    }
}
