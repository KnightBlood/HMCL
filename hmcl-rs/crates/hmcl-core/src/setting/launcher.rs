//! The settings of the launcher itself.
//!
//! The settings are stored as JSON, like in the Java launcher, so the file
//! can be inspected and edited by hand. The fields the launcher does not know
//! yet are kept, so a newer launcher does not lose them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The address of the JSON schema that describes the launcher settings.
pub const LAUNCHER_SETTINGS_SCHEMA: &str =
    "https://schemas.glavo.site/hmcl/launcher-settings/1.0.0";

/// The name of the setting of the brightness of the theme, as the parts of the
/// theme the user changed name it.
pub const THEME_APPEARANCE_BRIGHTNESS_MODE: &str = "themeBrightnessMode";

/// The name of the setting of the color of the theme, as the parts of the
/// theme the user changed name it.
pub const THEME_APPEARANCE_COLOR: &str = "themeColor";

/// The name of the setting of the style of the color of the theme, as the
/// parts of the theme the user changed name it.
pub const THEME_APPEARANCE_COLOR_STYLE: &str = "themeColorStyle";

/// The name of the setting of the title bar, as the parts of the theme the
/// user changed name it.
pub const THEME_APPEARANCE_TITLE_BAR_TRANSPARENT: &str = "titleBarTransparent";

/// The name of the setting of the transparency of the window, as the parts of
/// the theme the user changed name it.
pub const THEME_APPEARANCE_WINDOW_TRANSPARENT: &str = "windowTransparent";

/// The name of the setting of the background, as the parts of the theme the
/// user changed name it.
pub const THEME_APPEARANCE_BACKGROUND: &str = "background";

/// The name of the setting of the opacity of the background, as the parts of
/// the theme the user changed name it.
pub const THEME_APPEARANCE_BACKGROUND_OPACITY: &str = "backgroundOpacity";

/// The source the launcher downloads the files from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadSource {
    /// The source the launcher chooses by itself.
    Default,
    /// The official source of the files.
    Official,
    /// A mirror of the files.
    Mirror,
}

impl DownloadSource {
    /// Returns the name the settings of the launcher write the source with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "DEFAULT",
            Self::Official => "OFFICIAL",
            Self::Mirror => "MIRROR",
        }
    }

    /// Returns the source with a name, or `None` when the launcher does not
    /// know the name.
    ///
    /// The names are read without the case of their letters, so the names the
    /// older launchers wrote are read as well.
    pub fn of(name: &str) -> Option<Self> {
        match name.to_ascii_uppercase().as_str() {
            "DEFAULT" => Some(Self::Default),
            "OFFICIAL" => Some(Self::Official),
            "MIRROR" => Some(Self::Mirror),
            _ => None,
        }
    }
}

/// The way the launcher chooses the common directory of the instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommonDirectoryType {
    /// The default directory of the launcher.
    Default,
    /// The directory the user selected.
    Custom,
}

/// The way the launcher reaches the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProxyType {
    /// The proxy of the system.
    System,
    /// No proxy at all.
    Direct,
    /// An HTTP proxy.
    Http,
    /// A SOCKS proxy.
    Socks,
}

impl ProxyType {
    /// Returns the name the settings of the launcher write the mode with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::System => "SYSTEM",
            Self::Direct => "DIRECT",
            Self::Http => "HTTP",
            Self::Socks => "SOCKS",
        }
    }

    /// Returns whether the mode names a host and a port of its own.
    ///
    /// The system proxy and the direct connection reach the network without
    /// a host the launcher knows, so the fields of the host are only read for
    /// the other two modes.
    pub const fn uses_custom_address(self) -> bool {
        matches!(self, Self::Http | Self::Socks)
    }
}

/// The proxy the launcher reaches the network through.
///
/// The settings of the launcher name the mode, the host and the port of the
/// proxy; this is what the launcher of Java builds out of them: a mode of its
/// own with a host that is written and a port that is a port, or no proxy at
/// all when the player did not fill the address in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyAddress {
    /// The proxy of the system, which the launcher does not know.
    System,
    /// No proxy at all.
    Direct,
    /// A proxy of a protocol at a host and a port.
    Custom {
        /// The protocol of the proxy.
        proxy_type: ProxyType,
        /// The host of the proxy.
        host: String,
        /// The port of the proxy.
        port: u16,
    },
}

/// The credentials the launcher signs in to the proxy with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyCredentials {
    /// The user name of the proxy.
    pub user: String,
    /// The password of the proxy.
    pub password: String,
}

/// The source of the color of the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThemeColorType {
    /// The color the launcher chooses by itself.
    Default,
    /// The color the user selected.
    Custom,
    /// The color of the background.
    Background,
}

/// The style of the color of the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThemeColorStyle {
    /// The colors that stand out the most.
    TonalSpot,
    /// The colors of the system.
    Fidelity,
    /// The shades of a single color.
    Monochrome,
    /// The quiet colors.
    Neutral,
    /// The bright colors.
    Vibrant,
    /// The colors that stand out the most.
    Expressive,
    /// The colors of the content.
    Content,
    /// Every color of the palette.
    Rainbow,
    /// The colors of a fruit salad.
    FruitSalad,
}

/// The source of the background of the launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackgroundType {
    /// The background the launcher chooses by itself.
    Default,
    /// One of the backgrounds the launcher ships with.
    Builtin,
    /// An image of the computer of the user.
    Custom,
    /// An image the launcher downloads.
    Network,
    /// A picture the user painted.
    Paint,
    /// The color of the theme.
    ThemeColor,
}

impl BackgroundType {
    /// Returns the name the settings of the launcher write the background
    /// with.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "DEFAULT",
            Self::Builtin => "BUILTIN",
            Self::Custom => "CUSTOM",
            Self::Network => "NETWORK",
            Self::Paint => "PAINT",
            Self::ThemeColor => "THEME_COLOR",
        }
    }

    /// Returns the names of every background, in the order they are declared.
    pub const fn names() -> &'static [&'static str] {
        BACKGROUND_TYPE_NAMES
    }
}

/// The names of the backgrounds, in the order they are declared.
const BACKGROUND_TYPE_NAMES: &[&str] = &[
    BackgroundType::Default.name(),
    BackgroundType::Builtin.name(),
    BackgroundType::Custom.name(),
    BackgroundType::Network.name(),
    BackgroundType::Paint.name(),
    BackgroundType::ThemeColor.name(),
];

/// The source of the background that is shown when the selected background
/// cannot be loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackgroundFallbackType {
    /// One of the backgrounds the launcher ships with.
    Builtin,
    /// A picture the user painted.
    Paint,
    /// The color of the theme.
    ThemeColor,
}

/// The way the launcher caches the images it downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetworkBackgroundImageCachePolicy {
    /// The images are cached.
    Enabled,
    /// The images are not cached.
    Disabled,
}

/// The way the launcher shows its window while the background is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackgroundLoadPolicy {
    /// The window is shown when the background is loaded.
    WaitForBackground,
    /// The window is shown at once and the background follows.
    ShowFallbackWhileLoading,
}

/// The theme the launcher shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSelection {
    /// The identifier of the theme pack.
    pub pack_id: String,
    /// The identifier of the theme inside the pack, if the pack holds more
    /// than one theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_id: Option<String>,
}

/// The settings of the launcher itself.
///
/// The settings that are not set keep the default value of the launcher, so
/// a settings file that only holds a part of the settings is valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LauncherSettings {
    /// The address of the JSON schema that describes the settings.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// The language of the user interface, such as `zh_Hans`.
    pub language: Option<String>,
    /// Whether the launcher accepts the preview versions of the launcher.
    pub accept_preview_update: bool,
    /// Whether the launcher does not show the dialog of the updates.
    pub disable_auto_show_update_dialog: bool,
    /// Whether the launcher does not show the April Fools features.
    pub disable_april_fools: bool,
    /// The way the launcher chooses the common directory of the instances.
    pub common_directory_type: CommonDirectoryType,
    /// The common directory of the instances, if the user selected one.
    pub common_directory: Option<String>,
    /// The number of the lines the log views keep.
    pub log_lines: Option<u32>,
    /// The theme the launcher shows.
    pub selected_theme: Option<ThemeSelection>,
    /// The parts of the theme the user changed.
    pub theme_appearance_overrides: Vec<String>,
    /// The brightness of the theme, such as `auto`.
    pub theme_brightness_mode: String,
    /// The source of the color of the theme.
    pub theme_color_type: ThemeColorType,
    /// The color of the theme the user chose, such as `#336699`.
    pub custom_theme_color: Option<String>,
    /// The style of the color of the theme.
    pub theme_color_style: ThemeColorStyle,
    /// Whether the title bar of the window is transparent.
    pub title_bar_transparent: bool,
    /// The source of the background of the launcher.
    pub background_type: BackgroundType,
    /// The identifier of the background the launcher ships with.
    pub builtin_background_id: String,
    /// The path of the background image of the user.
    pub custom_background_image_path: Option<String>,
    /// The address of the background image the launcher downloads.
    pub network_background_image_url: Option<String>,
    /// The picture the user painted for the background, such as `#336699`.
    pub custom_background_paint: Option<String>,
    /// The opacity of the background of the launcher.
    pub background_opacity: f64,
    /// The way the launcher caches the images it downloads.
    pub network_background_image_cache_policy: NetworkBackgroundImageCachePolicy,
    /// The background that is shown when the selected one cannot be loaded.
    pub background_fallback_type: BackgroundFallbackType,
    /// The way the launcher shows its window while the background is loaded.
    pub background_load_policy: BackgroundLoadPolicy,
    /// The font of the log views.
    pub log_font_family: Option<String>,
    /// The size of the font of the log views.
    pub log_font_size: f64,
    /// The font of the window of the launcher.
    pub launcher_font_family: Option<String>,
    /// Whether the window of the launcher is transparent.
    pub window_transparent: bool,
    /// Whether the animations of the user interface are turned off.
    pub animation_disabled: Option<bool>,
    /// Whether the launcher chooses the number of the download threads by
    /// itself.
    pub auto_download_threads: bool,
    /// The number of the threads the launcher downloads the files with.
    pub download_threads: u32,
    /// The source the list of the versions of the game comes from.
    pub version_list_source: DownloadSource,
    /// The source the files of the game come from.
    pub file_download_source: DownloadSource,
    /// The source the add-ons come from.
    pub default_addon_source: String,
    /// Whether the proxy needs a user name and a password.
    pub has_proxy_auth: bool,
    /// The way the launcher reaches the network.
    pub proxy_type: ProxyType,
    /// The host of the proxy.
    pub proxy_host: Option<String>,
    /// The port of the proxy.
    pub proxy_port: Option<u16>,
    /// The user name of the proxy.
    pub proxy_user: Option<String>,
    /// The password of the proxy.
    pub proxy_password: Option<String>,
    /// The identifier of the game directory the user selected.
    pub selected_game_directory: Option<String>,
    /// The identifier of the game settings preset the user selected.
    pub default_game_settings_preset: Option<String>,
    /// The identifiers of the instances the user selected, by the identifier
    /// of the game directory they belong to.
    pub selected_instance: BTreeMap<String, String>,
    /// The way the user adds an account.
    pub preferred_login_type: Option<String>,
    /// The identifier of the account the user selected.
    pub selected_account: Option<String>,
    /// The settings the launcher does not know yet.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            schema: LAUNCHER_SETTINGS_SCHEMA.to_string(),
            language: None,
            accept_preview_update: false,
            disable_auto_show_update_dialog: false,
            disable_april_fools: false,
            common_directory_type: CommonDirectoryType::Default,
            common_directory: None,
            log_lines: None,
            selected_theme: None,
            theme_appearance_overrides: Vec::new(),
            theme_brightness_mode: "auto".to_string(),
            theme_color_type: ThemeColorType::Default,
            custom_theme_color: None,
            theme_color_style: ThemeColorStyle::Fidelity,
            title_bar_transparent: false,
            background_type: BackgroundType::Default,
            builtin_background_id: "grass".to_string(),
            custom_background_image_path: None,
            network_background_image_url: None,
            custom_background_paint: None,
            background_opacity: 1.0,
            network_background_image_cache_policy: NetworkBackgroundImageCachePolicy::Enabled,
            background_fallback_type: BackgroundFallbackType::Builtin,
            background_load_policy: BackgroundLoadPolicy::WaitForBackground,
            log_font_family: None,
            log_font_size: 12.0,
            launcher_font_family: None,
            window_transparent: false,
            animation_disabled: None,
            auto_download_threads: true,
            download_threads: 64,
            version_list_source: DownloadSource::Default,
            file_download_source: DownloadSource::Default,
            default_addon_source: "modrinth".to_string(),
            has_proxy_auth: false,
            proxy_type: ProxyType::System,
            proxy_host: None,
            proxy_port: None,
            proxy_user: None,
            proxy_password: None,
            selected_game_directory: None,
            default_game_settings_preset: None,
            selected_instance: BTreeMap::new(),
            preferred_login_type: None,
            selected_account: None,
            extra: BTreeMap::new(),
        }
    }
}

/// Returns the number of the threads the launcher downloads the files with
/// when the player lets it choose.
///
/// The launcher of Java runs four download threads for every processor the
/// machine has, but never more than 64, so a machine with many processors
/// does not flood the servers it downloads from.
pub fn default_download_threads() -> u32 {
    let processors = std::thread::available_parallelism()
        .map(|parallelism| parallelism.get() as u32)
        .unwrap_or(1);

    (processors * 4).clamp(1, 64)
}

impl LauncherSettings {
    /// Returns the number of the threads the launcher downloads the files
    /// with.
    ///
    /// The launcher of Java feeds the executor of its downloads with the
    /// number the player set, and keeps at least one thread, so a number the
    /// player did not ask for is a request for one; when the player lets the
    /// launcher choose the number, [`default_download_threads`] is used.
    pub fn effective_download_threads(&self) -> u32 {
        if self.auto_download_threads {
            default_download_threads()
        } else {
            self.download_threads.max(1)
        }
    }

    /// Sets the number of the threads the launcher downloads the files with,
    /// which is never fewer than one.
    ///
    /// Asking the launcher for a number of its own is turned off, which is
    /// the way the page of the network settings of the launcher of Java
    /// applies the number the player entered.
    pub fn set_download_threads(&mut self, threads: u32) {
        self.auto_download_threads = false;
        self.download_threads = threads.max(1);
    }

    /// Returns the proxy the launcher reaches the network through.
    ///
    /// A mode of the launcher's own with a host the player did not write, or
    /// with a port that is not a port, is the same as no proxy at all: the
    /// launcher of Java refuses such a setting rather than failing every
    /// request, and the launcher of today does the same.
    pub fn proxy_address(&self) -> ProxyAddress {
        match self.proxy_type {
            ProxyType::System => ProxyAddress::System,
            ProxyType::Direct => ProxyAddress::Direct,
            proxy_type => {
                let Some(host) = self
                    .proxy_host
                    .as_deref()
                    .map(str::trim)
                    .filter(|host| !host.is_empty())
                else {
                    return ProxyAddress::Direct;
                };

                match self.proxy_port {
                    Some(port) if port > 0 => ProxyAddress::Custom {
                        proxy_type,
                        host: host.to_string(),
                        port,
                    },
                    _ => ProxyAddress::Direct,
                }
            }
        }
    }

    /// Returns the credentials the launcher signs in to the proxy with.
    ///
    /// The launcher of Java asks for a user name and a password only when the
    /// player selected a proxy of its own and asked for the credentials; a
    /// user name and a password that are both empty is no credentials at all,
    /// so the launcher does not sign in.
    pub fn proxy_credentials(&self) -> Option<ProxyCredentials> {
        if !self.proxy_type.uses_custom_address() || !self.has_proxy_auth {
            return None;
        }

        let user = self.proxy_user.as_deref().unwrap_or_default();
        let password = self.proxy_password.as_deref().unwrap_or_default();
        if user.is_empty() && password.is_empty() {
            return None;
        }

        Some(ProxyCredentials {
            user: user.to_string(),
            password: password.to_string(),
        })
    }

    /// Returns the common directory of the instances.
    ///
    /// The directory the user selected is only used when the user asked for
    /// it; otherwise the default directory of the launcher is used.
    pub fn resolved_common_directory(&self, default_directory: &str) -> String {
        match (&self.common_directory_type, &self.common_directory) {
            (CommonDirectoryType::Custom, Some(directory)) if !directory.trim().is_empty() => {
                directory.clone()
            }
            _ => default_directory.to_string(),
        }
    }
}

impl super::schema::SchemaSettings for LauncherSettings {
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
        let settings = LauncherSettings::default();
        let json = serde_json::to_string(&settings).expect("the settings should be written");

        assert!(json
            .contains("\"$schema\":\"https://schemas.glavo.site/hmcl/launcher-settings/1.0.0\""));
        assert!(json.contains("\"downloadThreads\":64"));
        assert!(json.contains("\"proxyType\":\"SYSTEM\""));
        assert!(json.contains("\"themeColorStyle\":\"FIDELITY\""));
        assert!(json.contains("\"backgroundLoadPolicy\":\"WAIT_FOR_BACKGROUND\""));
    }

    #[test]
    fn the_settings_are_read_back() {
        let json = r#"{
            "$schema": "https://schemas.glavo.site/hmcl/launcher-settings/1.0.0",
            "language": "zh_Hans",
            "downloadThreads": 8,
            "proxyType": "SOCKS",
            "selectedInstance": {
                "game-directory:00000000-0000-0000-0000-000000000000": "instance"
            },
            "somethingNew": 42
        }"#;

        let settings: LauncherSettings =
            serde_json::from_str(json).expect("the settings should be read");

        assert_eq!(settings.language.as_deref(), Some("zh_Hans"));
        assert_eq!(settings.download_threads, 8);
        assert_eq!(settings.proxy_type, ProxyType::Socks);
        assert_eq!(settings.selected_instance.len(), 1);
        assert_eq!(
            settings.extra.get("somethingNew"),
            Some(&serde_json::json!(42))
        );

        // The settings that the file does not mention keep their default.
        assert!(!settings.accept_preview_update);
        assert!(settings.auto_download_threads);
    }

    #[test]
    fn the_settings_the_launcher_does_not_know_are_kept() {
        let settings = LauncherSettings::default();
        let mut json = serde_json::to_value(&settings).expect("the settings should be written");
        json.as_object_mut()
            .expect("the settings should be an object")
            .insert("futureOption".to_string(), serde_json::json!(true));

        let settings: LauncherSettings =
            serde_json::from_value(json).expect("the settings should be read");
        assert_eq!(
            settings.extra.get("futureOption"),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn the_common_directory_is_resolved() {
        let mut settings = LauncherSettings::default();
        assert_eq!(settings.resolved_common_directory("/default"), "/default");

        settings.common_directory_type = CommonDirectoryType::Custom;
        settings.common_directory = Some("/custom".to_string());
        assert_eq!(settings.resolved_common_directory("/default"), "/custom");

        settings.common_directory = Some("  ".to_string());
        assert_eq!(settings.resolved_common_directory("/default"), "/default");
    }

    #[test]
    fn the_number_of_the_threads_the_launcher_chooses_fits_the_machine() {
        let threads = default_download_threads();

        assert!((1..=64).contains(&threads), "{threads} threads");
        assert_eq!(
            LauncherSettings::default().effective_download_threads(),
            threads,
            "the launcher chooses the number itself"
        );
    }

    #[test]
    fn the_number_of_the_threads_the_player_asked_for_is_used() {
        let mut settings = LauncherSettings::default();

        settings.set_download_threads(8);
        assert!(!settings.auto_download_threads);
        assert_eq!(settings.download_threads, 8);
        assert_eq!(settings.effective_download_threads(), 8);

        // A number the player did not ask for is a request for one.
        settings.set_download_threads(0);
        assert_eq!(settings.effective_download_threads(), 1);

        // The player may let the launcher choose again.
        settings.auto_download_threads = true;
        assert_eq!(
            settings.effective_download_threads(),
            default_download_threads()
        );
    }

    #[test]
    fn the_proxy_of_the_system_is_the_proxy_of_the_system() {
        let settings = LauncherSettings::default();

        assert_eq!(settings.proxy_type, ProxyType::System);
        assert_eq!(settings.proxy_address(), ProxyAddress::System);

        let mut settings = settings;
        settings.proxy_type = ProxyType::Direct;
        assert_eq!(settings.proxy_address(), ProxyAddress::Direct);
    }

    #[test]
    fn a_proxy_the_player_wrote_is_used() {
        let mut settings = LauncherSettings {
            proxy_type: ProxyType::Http,
            proxy_host: Some("127.0.0.1".to_string()),
            proxy_port: Some(8080),
            ..LauncherSettings::default()
        };

        assert_eq!(
            settings.proxy_address(),
            ProxyAddress::Custom {
                proxy_type: ProxyType::Http,
                host: "127.0.0.1".to_string(),
                port: 8080
            }
        );

        // The host is written with spaces around it by the page.
        settings.proxy_host = Some("  proxy.example.com  ".to_string());
        settings.proxy_type = ProxyType::Socks;
        settings.proxy_port = Some(1080);
        assert_eq!(
            settings.proxy_address(),
            ProxyAddress::Custom {
                proxy_type: ProxyType::Socks,
                host: "proxy.example.com".to_string(),
                port: 1080
            }
        );
    }

    #[test]
    fn a_proxy_without_a_usable_address_is_no_proxy() {
        let mut settings = LauncherSettings {
            proxy_type: ProxyType::Http,
            ..LauncherSettings::default()
        };

        // The player selected a proxy but did not write the host.
        settings.proxy_host = None;
        assert_eq!(settings.proxy_address(), ProxyAddress::Direct);
        settings.proxy_host = Some("   ".to_string());
        assert_eq!(settings.proxy_address(), ProxyAddress::Direct);

        // A port the launcher cannot use is refused the same way.
        settings.proxy_host = Some("proxy.example.com".to_string());
        settings.proxy_port = None;
        assert_eq!(settings.proxy_address(), ProxyAddress::Direct);
        settings.proxy_port = Some(0);
        assert_eq!(settings.proxy_address(), ProxyAddress::Direct);
    }

    #[test]
    fn the_credentials_are_asked_for_only_by_a_proxy_of_its_own() {
        let mut settings = LauncherSettings {
            has_proxy_auth: true,
            proxy_user: Some("player".to_string()),
            proxy_password: Some("secret".to_string()),
            ..LauncherSettings::default()
        };

        // The system proxy and the direct connection have no credentials of
        // the launcher's own.
        assert_eq!(settings.proxy_credentials(), None);
        settings.proxy_type = ProxyType::Direct;
        assert_eq!(settings.proxy_credentials(), None);

        settings.proxy_type = ProxyType::Http;
        assert_eq!(
            settings.proxy_credentials(),
            Some(ProxyCredentials {
                user: "player".to_string(),
                password: "secret".to_string()
            })
        );

        // The player may ask for the credentials and write none of them.
        settings.has_proxy_auth = false;
        assert_eq!(settings.proxy_credentials(), None);

        settings.has_proxy_auth = true;
        settings.proxy_user = None;
        settings.proxy_password = None;
        assert_eq!(settings.proxy_credentials(), None);
    }
}
