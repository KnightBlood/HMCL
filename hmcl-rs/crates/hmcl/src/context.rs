//! The state of the launcher.
//!
//! The state holds the settings of the launcher, the settings of the user
//! and the texts of the launcher in the language the user selected. It is
//! created once, when the launcher starts, and is used by the pages of the
//! window.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use hmcl_core::auth::authlibinjector::AUTHLIB_INJECTOR_LOGIN_TYPE;
use hmcl_core::error::Error;
use hmcl_core::i18n::{I18n, Locale};
use hmcl_core::metadata;
use hmcl_core::setting::{
    authlib_injector_metadata_path, authlib_injector_servers_path, authlib_injectors_path,
    is_new_workspace, launcher_settings_path, read_authlib_injectors_urls, user_settings_path,
    AuthlibInjectorServer, AuthlibInjectorServerList, AuthlibInjectorServerMetadataCache,
    GameDirectories, GameDirectory, LauncherSettings, LauncherState, Schema, SchemaVersion,
    SettingsFile, UserSettings,
};
use hmcl_core::theme;
use hmcl_core::util::logging;

use crate::http_fetcher;
use crate::http_fetcher::HttpFetcher;

/// The state of the launcher.
///
/// The state is created when it is asked for the first time, so every part
/// of the launcher uses the same settings and the same texts.
pub fn launcher() -> &'static Launcher {
    static LAUNCHER: OnceLock<Launcher> = OnceLock::new();
    let launcher = LAUNCHER.get_or_init(Launcher::load);

    // The servers the file of the launcher of Java names are located after
    // the launcher read its files, so the launcher starts while they are
    // asked for.
    launcher.import_configured_authlib_servers();

    launcher
}

/// The schema of the settings of the launcher.
fn launcher_settings_schema() -> Schema {
    Schema::new("launcher-settings", SchemaVersion::new(1, 0, 0))
}

/// The schema of the settings of the user.
fn user_settings_schema() -> Schema {
    Schema::new("user-settings", SchemaVersion::new(1, 0, 0))
}

/// Locks a value of the launcher, even when a thread that held it failed.
///
/// A thread that could not have the value it asked for leaves the lock
/// failed, but the value is still the one the launcher keeps, so reading it
/// again is better than losing it.
fn locked<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Reads the addresses of the servers of the authlib-injector a provider
/// named, when the launcher runs in a new workspace.
///
/// The file the launcher of Java reads is looked for next to the program of
/// this launcher, which is where a provider drops it; a file that cannot be
/// read is told in the log instead of guessing which addresses the player
/// was given, which is the way the launcher of Java reads it as well. A
/// provider names no server in a file without addresses, so such a file is
/// read as naming none.
fn pending_authlib_servers(newly_created: bool) -> Vec<String> {
    if !newly_created {
        return Vec::new();
    }

    let path = authlib_injectors_path();
    if !path.exists() {
        return Vec::new();
    }

    match read_authlib_injectors_urls(&path) {
        Ok(urls) if !urls.is_empty() => urls,
        Ok(_) => Vec::new(),
        Err(error) => {
            logging::warn(&format!("Malformed authlib-injectors.json: {error}"));
            Vec::new()
        }
    }
}

/// The state of the launcher.
pub struct Launcher {
    /// The directory the settings of the launcher are kept in.
    user_home: PathBuf,
    /// The directory the files the launcher downloads are kept in.
    local_home: PathBuf,
    /// The settings of the launcher.
    launcher_settings: SettingsFile<LauncherSettings>,
    /// The settings of the user.
    user_settings: SettingsFile<UserSettings>,
    /// The state of the launcher, such as the rectangle of its window.
    launcher_state: Mutex<LauncherState>,
    /// The servers of the authlib-injector the player named.
    ///
    /// The list is what the page of the accounts shows; it is written back
    /// when a server is named or removed, which is what the launcher of Java
    /// keeps in `config/authlib-injector-servers.json`.
    authlib_servers: Mutex<SettingsFile<AuthlibInjectorServerList>>,
    /// The metadata the launcher read from the servers of the player.
    ///
    /// The metadata is kept apart from the list, so an update of a server
    /// does not ask the player to name it again; it is kept in
    /// `cache/authlib-injector-server-metadata.json`, which is where the
    /// launcher of Java keeps its own.
    authlib_metadata: Mutex<SettingsFile<AuthlibInjectorServerMetadataCache>>,
    /// Whether the workspace the launcher started in is a new one.
    ///
    /// The launcher of Java reads the servers a provider named only in a
    /// workspace no launcher has written files into, and the page of the
    /// accounts asks the way accounts are added there.
    newly_created: bool,
    /// The addresses of the servers of the authlib-injector a provider named
    /// and the launcher has not asked for yet.
    ///
    /// The addresses are read from the file of the launcher of Java and are
    /// emptied once the launcher asked every server it names for its
    /// metadata, so the asking happens once.
    pending_authlib_servers: Mutex<Vec<String>>,
    /// The texts of the launcher.
    i18n: I18n,
}

impl Launcher {
    /// Reads the state of the launcher from the files of the user.
    ///
    /// A file that cannot be read is replaced by the default settings, so
    /// the launcher always starts.
    pub fn load() -> Self {
        let user_home = metadata::user_home();
        let local_home = metadata::local_home();

        // The workspace is judged by the files it holds before any of them
        // exists: a new workspace is one neither launcher wrote a file of
        // its settings into, and the migration below writes the same files,
        // so the judgement has to be made first. This is the way the
        // launcher of Java reads the servers a provider named.
        let newly_created = is_new_workspace(&user_home, &local_home);

        // The settings of an older launcher are read before the settings of
        // this launcher, so a player who comes from the older launcher finds
        // the language and the window they had.
        if let Err(error) = hmcl_core::setting::migrate_legacy_settings(&local_home, &user_home) {
            hmcl_core::util::logging::warn(&format!(
                "The settings of the older launcher cannot be migrated: {error}"
            ));
        }

        // The accounts the older launcher shared between the workspaces are
        // read as well, so a player finds the accounts they signed in to.
        if let Err(error) = hmcl_core::setting::migrate_legacy_user_accounts(&user_home) {
            hmcl_core::util::logging::warn(&format!(
                "The shared accounts of the older launcher cannot be migrated: {error}"
            ));
        }

        let mut launcher_settings = SettingsFile::load_with_schema(
            launcher_settings_path(&user_home),
            &launcher_settings_schema(),
        )
        .unwrap_or_else(|error| {
            hmcl_core::util::logging::warn(&format!(
                "The settings of the launcher cannot be read: {error}"
            ));
            SettingsFile::new(
                launcher_settings_path(&user_home),
                LauncherSettings::default(),
            )
        });

        let user_settings =
            SettingsFile::load_with_schema(user_settings_path(&user_home), &user_settings_schema())
                .unwrap_or_else(|error| {
                    hmcl_core::util::logging::warn(&format!(
                        "The settings of the user cannot be read: {error}"
                    ));
                    SettingsFile::new(user_settings_path(&user_home), UserSettings::default())
                });

        let locale = launcher_settings
            .settings()
            .language
            .as_deref()
            .map(Locale::from_code)
            .unwrap_or_default();

        // The servers of the authlib-injector a provider named are read the
        // first time the launcher runs in a workspace, which is where a
        // provider hands a launcher with a file to read; the addresses are
        // kept until the servers were asked for their metadata.
        let pending_authlib_servers = pending_authlib_servers(newly_created);
        if !pending_authlib_servers.is_empty() {
            launcher_settings.settings_mut().preferred_login_type =
                Some(AUTHLIB_INJECTOR_LOGIN_TYPE.to_string());
        }

        // The state of the launcher is only what the window of the launcher
        // was when it was closed last, so a state that cannot be read is
        // replaced by the state of a launcher that starts the first time.
        let launcher_state = LauncherState::load(&user_home)
            .map(|file| file.into_settings())
            .unwrap_or_else(|error| {
                hmcl_core::util::logging::warn(&format!(
                    "The state of the launcher cannot be read: {error}"
                ));
                LauncherState::default()
            });

        // The servers the player named and the metadata the launcher read
        // from them are kept apart, so a server the player named again is
        // shown with the name it told before.
        let servers_existed = authlib_injector_servers_path(&user_home).exists();
        let authlib_servers = AuthlibInjectorServerList::load(&user_home).unwrap_or_else(|error| {
            hmcl_core::util::logging::warn(&format!(
                "The servers of the authlib-injector cannot be read: {error}"
            ));
            SettingsFile::new(
                authlib_injector_servers_path(&user_home),
                AuthlibInjectorServerList::new(),
            )
        });
        let authlib_metadata =
            AuthlibInjectorServerMetadataCache::load(&user_home).unwrap_or_else(|error| {
                hmcl_core::util::logging::warn(&format!(
                    "The metadata of the servers cannot be read: {error}"
                ));
                SettingsFile::new(
                    authlib_injector_metadata_path(&user_home),
                    AuthlibInjectorServerMetadataCache::new(),
                )
            });

        let mut this = Self {
            user_home,
            local_home,
            launcher_settings,
            user_settings,
            launcher_state: Mutex::new(launcher_state),
            authlib_servers: Mutex::new(authlib_servers),
            authlib_metadata: Mutex::new(authlib_metadata),
            newly_created,
            pending_authlib_servers: Mutex::new(pending_authlib_servers),
            i18n: I18n::new(locale),
        };

        // The metadata the launcher read in an earlier run is put back into
        // the list, so a server the player named shows the name it told.
        this.restore_authlib_metadata();

        // The list is written the first time the launcher reads it, so the
        // launcher of the next run finds the server LittleSkin is suggested
        // in; this is what the launcher of Java does with a list it names
        // for the first time.
        if !servers_existed {
            this.save_authlib_servers();
        }

        this
    }

    /// Returns the directory the settings of the launcher are kept in.
    pub fn user_home(&self) -> &PathBuf {
        &self.user_home
    }

    /// Returns the directory the files the launcher downloads are kept in.
    pub fn local_home(&self) -> &PathBuf {
        &self.local_home
    }

    /// Returns the settings of the launcher.
    pub fn launcher_settings(&self) -> &LauncherSettings {
        self.launcher_settings.settings()
    }

    /// Returns the settings of the user.
    pub fn user_settings(&self) -> &UserSettings {
        self.user_settings.settings()
    }

    /// Returns the game directories of the user, merged from the workspace
    /// and the shared store.
    ///
    /// The directories of the workspace hide the directories of the user with
    /// the same identifier, which is the way the launcher of Java merges
    /// them. The two directories the launcher creates itself are added when
    /// neither store has any, so the launcher always has a directory to
    /// show.
    #[allow(dead_code)]
    pub fn game_directories(&self) -> Vec<GameDirectory> {
        let local = GameDirectories::load(&self.local_home)
            .map(|file| file.into_settings())
            .unwrap_or_default();
        let user = GameDirectories::load(&self.user_home)
            .map(|file| file.into_settings())
            .unwrap_or_default();

        let mut merged = local.merged_with(&user);
        if merged.is_empty() {
            let home = hmcl_core::setting::default_home_game_directory();
            let mut store = GameDirectories::new();
            store.ensure_built_in_directories(&home);
            merged = store.directories;
        }
        merged
    }

    /// Returns the texts of the launcher.
    pub fn i18n(&self) -> &I18n {
        &self.i18n
    }

    /// Returns the fetcher the launcher downloads its files with.
    ///
    /// The fetcher is shared by every download of the program, so the
    /// connections the launcher keeps open to the same server are reused. It
    /// is made from the settings the launcher was started with: the proxy the
    /// player wrote and the language the texts of the launcher are shown in,
    /// which is the language the servers answer in. A launcher that changes
    /// the settings of the proxy has to be restarted for the change to take,
    /// which is what the launcher of Java does as well, because the programs
    /// of the network read the proxy when they are started.
    #[allow(dead_code)]
    pub fn fetcher(&self) -> &'static HttpFetcher {
        HttpFetcher::shared(
            self.launcher_settings(),
            http_fetcher::language_tag(self.locale().code()),
        )
    }

    /// Returns the client the launcher logs a player in with.
    ///
    /// The client sends the requests of a login the way the launcher of Java
    /// does: the headers of the launcher, the language of the player and the
    /// proxy of the settings. The connections are the ones of the downloads of
    /// the launcher, so a login does not open a second connection to the
    /// servers of Microsoft.
    pub fn http_client(&self) -> &'static crate::http_client::UreqHttpClient {
        static CLIENT: OnceLock<crate::http_client::UreqHttpClient> = OnceLock::new();
        CLIENT.get_or_init(|| {
            crate::http_client::UreqHttpClient::new(
                self.launcher_settings(),
                http_fetcher::language_tag(self.locale().code()),
            )
        })
    }

    /// Returns the page of the login of Microsoft.
    ///
    /// The page is shown by the browser of the player, so its colors and its
    /// texts are read when the login starts: a launcher whose language or theme
    /// changed shows the page the way it looks now.
    #[allow(dead_code)]
    pub fn oauth_callback(&self) -> crate::oauth_server::DesktopOAuthCallback {
        let dark = self.dark_theme();
        crate::oauth_server::DesktopOAuthCallback::new(
            crate::oauth_server::client_id(),
            crate::oauth_server::Page::new(
                self.i18n.clone(),
                http_fetcher::language_tag(self.locale().code()),
                dark,
            ),
        )
    }

    /// Returns whether the pages of the launcher are drawn in the dark theme.
    ///
    /// The user may name the brightness of the theme, and a user who names
    /// neither follows the platform. The launcher has no reader of the
    /// brightness of the platform yet, so a user who names neither is given the
    /// light theme, which is the theme a platform that names none shows.
    fn dark_theme(&self) -> bool {
        // A launcher that follows the platform uses the theme of the window,
        // which is the light theme until the launcher reads the brightness of
        // the platform.
        self.launcher_settings().theme_brightness_mode == "dark"
    }

    /// Returns the directory the theme packs of the launcher are kept in.
    ///
    /// The directory is inside the directory the launcher keeps its own
    /// files in, so a theme pack the launcher installed is kept next to the
    /// files the launcher downloads. The theme packs the user keeps in their
    /// own directory are read as well, so the packs of both directories are
    /// listed by [`Self::theme_packs`].
    #[allow(dead_code)]
    pub fn theme_packs_directory(&self) -> PathBuf {
        self.local_home.join(theme::THEME_PACKS_DIRECTORY)
    }

    /// Returns the theme packs of the launcher.
    ///
    /// The theme packs of the directory of the launcher hide the theme packs
    /// of the directory of the user with the same identifier. The packs are
    /// shown with the name of the pack first.
    ///
    /// # Errors
    ///
    /// Returns an error when a directory of the theme packs cannot be read.
    #[allow(dead_code)]
    pub fn theme_packs(
        &self,
    ) -> Result<Vec<theme::InstalledThemePack>, theme::ThemePackManagerError> {
        theme::ThemePackManager::list_installed_merged(
            &self.theme_packs_directory(),
            &self.user_home.join(theme::THEME_PACKS_DIRECTORY),
        )
    }

    /// Returns the state of the launcher, such as the rectangle of its
    /// window.
    ///
    /// The lock is held while the state is read; the caller must not ask for
    /// the state again while it holds the lock, which is why
    /// [`Self::update_launcher_state`] changes the state in one step.
    pub fn launcher_state(&self) -> std::sync::MutexGuard<'_, LauncherState> {
        self.launcher_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Changes the state of the launcher.
    pub fn update_launcher_state(&self, change: impl FnOnce(&mut LauncherState)) {
        let mut state = self.launcher_state();
        change(&mut state);
    }

    /// Returns the language of the user interface.
    pub fn locale(&self) -> Locale {
        self.i18n.locale()
    }

    /// Writes the settings of the launcher and of the user to the files of
    /// the user.
    ///
    /// The servers of the authlib-injector the player named and the metadata
    /// the launcher read from them are written as well, the way the launcher
    /// of Java writes them when the player made a change; a file of a newer
    /// launcher is only read, so the launcher does not write it and keeps
    /// going.
    pub fn save(&self) -> Result<(), Error> {
        self.launcher_settings.save()?;
        self.user_settings.save()?;
        self.save_authlib_servers();
        self.save_authlib_metadata();
        Ok(())
    }

    /// Writes the state of the launcher to the file of the user.
    ///
    /// The state is written apart from the settings, so a launcher that
    /// cannot write the state still keeps the settings the user made.
    pub fn save_state(&self) -> Result<(), Error> {
        self.launcher_state().save(&self.user_home)
    }

    /// Returns whether the workspace the launcher runs in is a new one.
    ///
    /// The store of the servers of the authlib-injector is read only in a
    /// new workspace, and the page of the accounts names the way accounts
    /// are added there, so the pages of the launcher ask this before they
    /// suggest a way of signing in.
    #[allow(dead_code)]
    pub fn newly_created(&self) -> bool {
        self.newly_created
    }

    /// Returns the servers of the authlib-injector the player named.
    ///
    /// The servers are named in the order the player added them, which is
    /// the order the page of the accounts shows; the list is not the live
    /// one, so a server another thread named while the pages read the list
    /// is found the next time it is asked for.
    #[allow(dead_code)]
    pub fn authlib_servers(&self) -> Vec<AuthlibInjectorServer> {
        locked(&self.authlib_servers).settings().servers().to_vec()
    }

    /// Names a server of the authlib-injector, and returns whether it was
    /// new.
    ///
    /// The address the player wrote is turned into an HTTPS address, which
    /// is where a server of Yggdrasil answers; a server the player already
    /// named is not named a second time. The change is written to the file
    /// of the servers, the way the launcher of Java writes it as soon as the
    /// list changes.
    #[allow(dead_code)]
    pub fn add_authlib_server(&self, url: &str) -> bool {
        let added = {
            let mut file = locked(&self.authlib_servers);
            file.settings_mut().add(url)
        };

        if added {
            self.save_authlib_servers();
        }
        added
    }

    /// Forgets a server of the authlib-injector, and returns whether the
    /// player named it.
    ///
    /// The metadata the launcher kept of the server is dropped as well, so
    /// a server that is named again is asked for its metadata again, which
    /// is what the launcher of Java does when the list loses a server.
    #[allow(dead_code)]
    pub fn remove_authlib_server(&self, url: &str) -> bool {
        let removed = {
            let mut file = locked(&self.authlib_servers);
            file.settings_mut().remove(url)
        };

        if removed {
            self.save_authlib_servers();
            {
                let mut cache = locked(&self.authlib_metadata);
                cache.settings_mut().remove(url);
            }
            self.save_authlib_metadata();
        }
        removed
    }

    /// Returns the server an account of the authlib-injector names.
    ///
    /// An account the launcher read from its settings names the address of
    /// its server; the launcher gives the account the server of the list, or
    /// names the server when the player removed it, so the account still
    /// signs in. This is the way the launcher of Java keeps its accounts
    /// working after a server was named again.
    #[allow(dead_code)]
    pub fn get_or_create_authlib_server(&self, url: &str) -> AuthlibInjectorServer {
        let (server, added) = {
            let mut file = locked(&self.authlib_servers);
            let added = file.settings().get(url).is_none();
            let server = file.settings_mut().get_or_add(url);
            (server, added)
        };

        if added {
            self.save_authlib_servers();
        }
        server
    }

    /// Writes the file of the servers the player named, telling the reason
    /// when it cannot be written.
    ///
    /// A file of a newer launcher is only read, which is why no attempt to
    /// write it is made and no reason is told.
    fn save_authlib_servers(&self) {
        if let Err(error) = locked(&self.authlib_servers).save() {
            logging::warn(&format!(
                "The servers of the authlib-injector cannot be written: {error}"
            ));
        }
    }

    /// Writes the file of the metadata of the servers, telling the reason
    /// when it cannot be written.
    fn save_authlib_metadata(&self) {
        if let Err(error) = locked(&self.authlib_metadata).save() {
            logging::warn(&format!(
                "The metadata of the servers cannot be written: {error}"
            ));
        }
    }

    /// Puts the metadata of the servers the launcher read in an earlier run
    /// back into the list.
    ///
    /// The cache is the only place the text a server answered with is kept,
    /// and the pages of the launcher show the names of it; the launcher of
    /// Java binds its cache to the list by restoring the metadata of every
    /// server of the list when both are read, which is what this does. An
    /// entry the cache cannot read is dropped, so the launcher asks the
    /// server for its metadata again.
    ///
    /// The metadata is not marked as fresh by the launcher of Java either:
    /// a launcher that starts the game with a server asks the server again
    /// first.
    fn restore_authlib_metadata(&mut self) {
        let (entries, dropped): (Vec<(String, Option<AuthlibInjectorServer>)>, bool) = {
            let mut cache = locked(&self.authlib_metadata);
            let before = cache.settings().entries().len();

            let urls: Vec<String> = locked(&self.authlib_servers)
                .settings()
                .servers()
                .iter()
                .map(|server| server.url.clone())
                .collect();
            let entries: Vec<(String, Option<AuthlibInjectorServer>)> = urls
                .into_iter()
                .map(|url| (url.clone(), cache.settings_mut().restore(&url)))
                .collect();

            let dropped = cache.settings().entries().len() < before;
            (entries, dropped)
        };

        // The metadata of a server that is restored is written into its
        // entry of the list, so the pages of the launcher show the name the
        // server told the last time it was asked.
        for (url, server) in entries {
            let Some(server) = server else {
                continue;
            };
            let Some(response) = server.metadata_response().map(str::to_string) else {
                continue;
            };

            if let Some(entry) = locked(&self.authlib_servers)
                .settings_mut()
                .server_mut(&url)
            {
                let _ = entry.restore_metadata_cache(
                    response,
                    server.metadata_timestamp().unwrap_or_default(),
                );
            }
        }

        // A metadata the cache cannot read is gone from the cache, so the
        // cache has to be written again.
        if dropped {
            self.save_authlib_metadata();
        }
    }

    /// Locates the servers the file of the launcher of Java names.
    ///
    /// The launcher starts while the servers are asked, which is what the
    /// launcher of Java does as well: the metadata of every server is asked
    /// from the server itself, in a thread of the launcher's own per server.
    /// A server that answers is named in the list with the metadata it
    /// answered with, and one that cannot be reached is told in the log and
    /// is not named, which is the way the launcher of Java treats a server
    /// the player was given.
    ///
    /// The addresses are named only once; asking again happens the next time
    /// the launcher starts in a new workspace, since the file of the
    /// launcher of Java is read only there.
    fn import_configured_authlib_servers(&'static self) {
        let urls = locked(&self.pending_authlib_servers)
            .drain(..)
            .collect::<Vec<_>>();
        if urls.is_empty() {
            return;
        }

        let client = self.http_client();
        for url in urls {
            std::thread::spawn(
                move || match AuthlibInjectorServer::locate_server(client, &url) {
                    Ok(server) => self.add_located_authlib_server(server),
                    Err(error) => logging::warn(&format!(
                        "Failed to fetch authlib-injector server metadata: {error}"
                    )),
                },
            );
        }
    }

    /// Names a server the launcher located, and keeps the metadata it
    /// answered with.
    ///
    /// The list holds the server, so the page of the accounts shows it the
    /// next time it is opened; the cache holds the text the server answered
    /// with, so the launcher does not ask the metadata of the server again
    /// while it runs.
    fn add_located_authlib_server(&self, server: AuthlibInjectorServer) {
        {
            let mut file = locked(&self.authlib_servers);
            file.settings_mut().add_server(server.clone());
        }
        self.save_authlib_servers();

        {
            let mut cache = locked(&self.authlib_metadata);
            cache.settings_mut().store(&server);
        }
        self.save_authlib_metadata();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmcl_core::setting::ContentBounds;

    #[test]
    fn the_state_of_the_window_survives_a_save_and_a_load() {
        let directory = std::env::temp_dir().join("hmcl-context-state-test");
        let _ = std::fs::remove_dir_all(&directory);

        let mut state = LauncherState::default();
        state.record_content_bounds(
            &ContentBounds {
                x: 400.0,
                y: 200.0,
                width: 1200.0,
                height: 700.0,
            },
            (1920.0, 1080.0),
        );
        state.set_prompted_version(Some("3.6.15".to_string()));
        state.save(&directory).expect("the state should be written");

        let read = LauncherState::load(&directory).expect("the state should be read");
        let read = read.settings();
        assert_eq!(read.width(), 1200.0);
        assert_eq!(read.height(), 700.0);
        assert_eq!(read.prompted_version(), Some("3.6.15"));
        assert!((read.x() - 400.0 / 1920.0).abs() < 1e-9);

        let _ = std::fs::remove_dir_all(&directory);
    }
}
