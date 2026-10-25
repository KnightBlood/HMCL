//! The settings of the user of the launcher.
//!
//! The settings are stored as JSON, like in the Java launcher, so the file
//! can be inspected and edited by hand. The fields the launcher does not know
//! yet are kept, so a newer launcher does not lose them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The address of the JSON schema that describes the user settings.
pub const USER_SETTINGS_SCHEMA: &str = "https://schemas.glavo.site/hmcl/user-settings/1.0.0";

/// The settings of the user of the launcher.
///
/// The settings that are not set keep the default value of the launcher, so
/// a settings file that only holds a part of the settings is valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UserSettings {
    /// The address of the JSON schema that describes the settings.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// The number of the log files the launcher keeps.
    pub log_retention: Option<u32>,
    /// Whether the user may add an account that does not need a password.
    pub enable_offline_account: bool,
    /// The way the fonts of the launcher are drawn.
    pub font_anti_aliasing: Option<String>,
    /// The Java runtimes the user added.
    pub user_java: Vec<String>,
    /// The Java runtimes the user turned off.
    pub disabled_java: Vec<String>,
    /// The settings the launcher does not know yet.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            schema: USER_SETTINGS_SCHEMA.to_string(),
            log_retention: None,
            enable_offline_account: true,
            font_anti_aliasing: None,
            user_java: Vec::new(),
            disabled_java: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

impl super::schema::SchemaSettings for UserSettings {
    fn schema(&self) -> &str {
        &self.schema
    }

    fn set_schema(&mut self, schema: &str) {
        self.schema = schema.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_are_written_in_the_format_of_the_java_launcher() {
        let settings = UserSettings::default();
        let json = serde_json::to_string(&settings).expect("the settings should be written");

        assert!(
            json.contains("\"$schema\":\"https://schemas.glavo.site/hmcl/user-settings/1.0.0\"")
        );
        assert!(json.contains("\"enableOfflineAccount\":true"));
    }

    #[test]
    fn the_settings_are_read_back() {
        let json = r#"{
            "$schema": "https://schemas.glavo.site/hmcl/user-settings/1.0.0",
            "logRetention": 3,
            "enableOfflineAccount": false,
            "userJava": ["/java/21"],
            "disabledJava": ["/java/8"]
        }"#;

        let settings: UserSettings =
            serde_json::from_str(json).expect("the settings should be read");

        assert_eq!(settings.log_retention, Some(3));
        assert!(!settings.enable_offline_account);
        assert_eq!(settings.user_java, vec!["/java/21".to_string()]);
        assert_eq!(settings.disabled_java, vec!["/java/8".to_string()]);
    }
}
