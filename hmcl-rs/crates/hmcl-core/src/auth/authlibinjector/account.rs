//! The account of a player of a server of the authlib-injector.
//!
//! The login is the one of [`YggdrasilAccount`], which speaks the protocol of
//! the server; an account of the authlib-injector adds what the game needs to
//! talk to that server instead of the service of Mojang: the agent of the
//! virtual machine and the metadata of the server, which the agent reads before
//! the game asks the server anything.
//!
//! Both are fetched while the player logs in, so the game starts without
//! waiting for the network when the account is played offline.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::auth::yggdrasil::account::uploadable_textures as textures_of_profile;
use crate::auth::yggdrasil::model::read_properties_object;
use crate::auth::yggdrasil::{
    AuthlibInjectorProvider, CharacterSelector, CompleteGameProfile, TextureType, YggdrasilAccount,
    YggdrasilService, YggdrasilSession,
};
use crate::auth::{Account, AccountId, AuthInfo, AuthenticationError, Uuid};
use crate::setting::authlib_servers::AuthlibInjectorServer;
use crate::util::base64;
use crate::util::http_request::HttpClient;

use super::artifact::{AuthlibInjectorArtifactInfo, AuthlibInjectorArtifactProvider};

/// The name of the property that holds the metadata the agent reads.
const PREFETCHED_PROPERTY: &str = "authlibinjector.yggdrasil.prefetched";

/// The account of a player of a server of the authlib-injector.
///
/// The account is the account of Yggdrasil of the server, together with the
/// agent the game is started with and the metadata the agent reads. The
/// metadata is kept as the text the server answered with, so the agent reads
/// exactly what the server wrote.
#[derive(Clone)]
pub struct AuthlibInjectorAccount {
    /// The account of the protocol of Yggdrasil.
    account: YggdrasilAccount,
    /// The server the player logs in to.
    server: AuthlibInjectorServer,
    /// The file of the agent the game is started with.
    downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
}

impl fmt::Debug for AuthlibInjectorAccount {
    /// Writes the account without the file of the agent, which has no text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthlibInjectorAccount")
            .field("account", &self.account)
            .field("server", &self.server)
            .finish_non_exhaustive()
    }
}

impl AuthlibInjectorAccount {
    /// Creates the account of a player that logged in with a password.
    ///
    /// The login of the protocol of Yggdrasil is done first, because a player
    /// whose password is refused is not logged in at all; the agent and the
    /// metadata are read afterwards, so a failure of the network that keeps
    /// the agent from being written is reported as the login failure it is.
    pub fn authenticate(
        account_id: AccountId,
        server: AuthlibInjectorServer,
        downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
        client: &dyn HttpClient,
        login_name: impl Into<String>,
        password: &str,
        selector: &dyn CharacterSelector,
    ) -> Result<Self, AuthenticationError> {
        let account = YggdrasilAccount::authenticate(
            account_id,
            service_of(&server),
            client,
            login_name,
            password,
            selector,
        )?;

        let mut account = Self {
            account,
            server,
            downloader,
        };
        // The agent and the metadata are read now, so a failure is reported to
        // the player who logs in rather than when the game is started.
        account.prefetch(client)?;
        Ok(account)
    }

    /// Creates the account of a session the launcher already has.
    ///
    /// The agent and the metadata are not read here: the account of a player
    /// who logged in during this run already holds them, and a launcher that
    /// read the account from its settings reads them when the player logs in
    /// or starts the game.
    pub fn new(
        account_id: AccountId,
        server: AuthlibInjectorServer,
        downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
        login_name: impl Into<String>,
        session: YggdrasilSession,
    ) -> Result<Self, AuthenticationError> {
        let account = YggdrasilAccount::new(account_id, service_of(&server), login_name, session)?;
        Ok(Self {
            account,
            server,
            downloader,
        })
    }

    /// Reads the account of a player from the settings of the launcher.
    ///
    /// The public data names the server, the name the player logs in with and
    /// the profile; the protected data holds the tokens and the properties of
    /// the profile the launcher read the last time. The server is looked up by
    /// the address the record names, so the account is the same server the
    /// player chose in the list of the servers.
    ///
    /// The properties are put into the cache of the service and dropped at
    /// once, so the skin of the player is drawn before the server is asked
    /// again, which is what the launcher of Java does.
    pub fn from_storage(
        metadata: &Value,
        private_data: &Map<String, Value>,
        downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
        server_lookup: impl Fn(&str) -> AuthlibInjectorServer,
    ) -> Result<Self, AuthenticationError> {
        let metadata_object = metadata
            .as_object()
            .ok_or_else(|| AuthenticationError::invalid_stored("metadata is not an object"))?;

        let api_root = metadata_object
            .get("serverBaseURL")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AuthenticationError::invalid_stored("storage does not have API root.")
            })?;

        let server = server_lookup(api_root);
        Self::from_storage_with_server(metadata, private_data, downloader, server)
    }

    /// Reads the account of a player from the settings of the launcher, with
    /// the server the record belongs to.
    ///
    /// The factory of the accounts of a server the player named holds the
    /// server itself, so it reads the record without looking the server up by
    /// the address the record names.
    pub fn from_storage_with_server(
        metadata: &Value,
        private_data: &Map<String, Value>,
        downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
        server: AuthlibInjectorServer,
    ) -> Result<Self, AuthenticationError> {
        let metadata_object = metadata
            .as_object()
            .ok_or_else(|| AuthenticationError::invalid_stored("metadata is not an object"))?;

        let account_id = Account::read_account_id(metadata_object)
            .ok_or_else(|| AuthenticationError::invalid_stored("accountID is missing"))?;

        let login_name = metadata_object
            .get("loginName")
            .and_then(Value::as_str)
            .ok_or_else(|| AuthenticationError::invalid_stored("storage does not have loginName"))?
            .to_string();

        let session = YggdrasilSession::from_storage(metadata, private_data)?;
        let service = service_of(&server);

        if let Some(Value::Object(properties)) = private_data.get("profileProperties") {
            if let Some(selected) = session.selected_profile() {
                let properties = read_properties_object(properties)?;
                service.put_profile(CompleteGameProfile::of(selected.clone(), properties));
                service.invalidate_profile(selected.id());
            }
        }

        let account = YggdrasilAccount::new(account_id, service, login_name, session)?;
        Ok(Self {
            account,
            server,
            downloader,
        })
    }

    /// Returns the account of the protocol of Yggdrasil.
    pub const fn account(&self) -> &YggdrasilAccount {
        &self.account
    }

    /// Returns the server the player logs in to.
    pub const fn server(&self) -> &AuthlibInjectorServer {
        &self.server
    }

    /// Returns the identifier of the record of the account.
    pub const fn account_id(&self) -> AccountId {
        self.account.account_id()
    }

    /// Returns the name the player logs in with.
    pub fn login_name(&self) -> &str {
        self.account.login_name()
    }

    /// Returns the identifier of the profile the player selected.
    pub const fn profile_id(&self) -> Uuid {
        self.account.profile_id()
    }

    /// Returns the name of the profile the player selected.
    pub fn profile_name(&self) -> &str {
        self.account.profile_name()
    }

    /// Returns the tokens of the last login.
    pub fn session(&self) -> &YggdrasilSession {
        self.account.session()
    }

    /// Returns the information the game is started with, after checking the
    /// token of the account.
    ///
    /// The agent and the metadata are read again when the launcher does not
    /// hold them, which is the case of an account read from the settings of a
    /// launcher that did not log in during this run.
    pub fn log_in(&mut self, client: &dyn HttpClient) -> Result<AuthInfo, AuthenticationError> {
        let info = self.account.log_in(client)?;
        let artifact = self.prefetch(client)?;
        self.launch_info(info, &artifact)
    }

    /// Logs the player in with the password of the account.
    pub fn log_in_with_password(
        &mut self,
        client: &dyn HttpClient,
        password: &str,
    ) -> Result<AuthInfo, AuthenticationError> {
        let info = self.account.log_in_with_password(client, password)?;
        let artifact = self.prefetch(client)?;
        self.launch_info(info, &artifact)
    }

    /// Returns the information of the player without asking a server.
    ///
    /// The agent and the metadata must be there, because the game cannot talk
    /// to the server without them; a player who never logged in has no agent,
    /// so the player logs in rather than starting a game that cannot reach the
    /// server.
    pub fn play_offline(&self) -> Result<AuthInfo, AuthenticationError> {
        let info = self.account.play_offline()?;

        let artifact = self
            .downloader
            .get_artifact_info_immediately()
            .ok_or(AuthenticationError::NotLoggedIn)?;

        if self.server.metadata_response().is_none() {
            return Err(AuthenticationError::NotLoggedIn);
        }

        self.launch_info(info, &artifact)
    }

    /// Writes the public data of the account into the settings of the launcher.
    ///
    /// The address of the server is written first, so the launcher finds the
    /// server again when it reads the account, which is what the launcher of
    /// Java writes.
    pub fn write_metadata(&self, metadata: &mut Map<String, Value>) {
        metadata.insert(
            "serverBaseURL".to_string(),
            Value::String(self.server.url().to_string()),
        );
        self.account.write_metadata(metadata);
    }

    /// Writes the credentials of the account into the protected data.
    pub fn write_private_data(&self, private_data: &mut Map<String, Value>) {
        self.account.write_private_data(private_data);
    }

    /// Forgets the properties of the profile of the player and the metadata of
    /// the server.
    ///
    /// The metadata is read again the next time the game is started, so a
    /// server that changed its name or its links is shown as it is now.
    pub fn clear_cache(&mut self) {
        self.account.clear_cache();
        self.server.invalidate_metadata_cache();
    }

    /// Returns the characters of the account the server accepts a texture for.
    ///
    /// The names of the textures are the ones the server named in the profile
    /// of the player, so a server that accepts none gives no names.
    pub fn uploadable_textures(&self) -> BTreeSet<TextureType> {
        match self.account.service().cached_profile(self.profile_id()) {
            Some(profile) => textures_of_profile(&profile),
            None => BTreeSet::new(),
        }
    }

    /// Writes the agent out when it is not there and reads the metadata of the
    /// server, returning the file the game is started with.
    ///
    /// The agent is written before the server is asked, so a player who cannot
    /// reach the server is told that the agent could not be written only when
    /// that is what failed.
    fn prefetch(
        &mut self,
        client: &dyn HttpClient,
    ) -> Result<AuthlibInjectorArtifactInfo, AuthenticationError> {
        let artifact = self.downloader.get_artifact_info().map_err(|error| {
            AuthenticationError::InjectorDownload {
                message: error.to_string(),
            }
        })?;

        self.server
            .fetch_metadata_response(client)
            .map_err(|error| AuthenticationError::disconnect(error.to_string()))?;

        Ok(artifact)
    }

    /// Returns the information of the player with the arguments of the agent.
    ///
    /// The arguments are the ones the launcher of Java writes: the agent with
    /// the address of the server, the side the agent works on, and the metadata
    /// the agent reads instead of asking the server while the game starts.
    fn launch_info(
        &self,
        info: AuthInfo,
        artifact: &AuthlibInjectorArtifactInfo,
    ) -> Result<AuthInfo, AuthenticationError> {
        let metadata = self
            .server
            .metadata_response()
            .ok_or(AuthenticationError::NotLoggedIn)?;

        Ok(info.with_launch_arguments(vec![
            format!(
                "-javaagent:{}={}",
                artifact.location().display(),
                self.server.url()
            ),
            "-Dauthlibinjector.side=client".to_string(),
            format!(
                "-D{PREFETCHED_PROPERTY}={}",
                base64::encode(metadata.as_bytes())
            ),
        ]))
    }
}

impl fmt::Display for AuthlibInjectorAccount {
    /// Writes the account the way the launcher of Java writes it.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "AuthlibInjectorAccount [accountID={}, profileID={}, loginName={}, server={}]",
            self.account.account_id(),
            self.account.profile_id(),
            self.account.login_name(),
            self.server.url()
        )
    }
}

/// Returns the service of the protocol of a server.
///
/// The service holds the cache of the profiles of the server. The launcher of
/// Java keeps the service in the server itself; here the server is a value of
/// the settings, so the service is created for the account and shared by the
/// account of one record.
fn service_of(server: &AuthlibInjectorServer) -> Arc<YggdrasilService<AuthlibInjectorProvider>> {
    Arc::new(YggdrasilService::new(AuthlibInjectorProvider::new(
        server.url(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::authlibinjector::artifact::SimpleAuthlibInjectorArtifactProvider;
    use crate::auth::yggdrasil::GameProfile;
    use crate::util::http_request::{HttpRequest, HttpResponse};
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// The identifier of the player of the tests.
    const PLAYER: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";

    /// The root of the API of the server of the tests.
    const API_ROOT: &str = "https://example.com/api/yggdrasil/";

    /// The text of the metadata of the server of the tests.
    const METADATA: &str = r#"{"meta":{"serverName":"Example"}}"#;

    /// A client that answers with the answers it is given and keeps the
    /// requests it was asked.
    struct FakeClient {
        /// The answers, in the order they are returned.
        answers: RefCell<Vec<String>>,
        /// The addresses the client was asked, in the order they were asked.
        asked: RefCell<Vec<String>>,
    }

    impl FakeClient {
        /// Creates a client that answers with the answers in order.
        fn new(answers: Vec<String>) -> Self {
            Self {
                answers: RefCell::new(answers),
                asked: RefCell::new(Vec::new()),
            }
        }

        /// Returns the addresses of the requests the client was asked.
        fn urls(&self) -> Vec<String> {
            self.asked.borrow().clone()
        }
    }

    impl HttpClient for FakeClient {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
            self.asked.borrow_mut().push(request.url().to_string());
            let mut answers = self.answers.borrow_mut();
            if answers.is_empty() {
                return Err("the client was asked too often".to_string());
            }
            Ok(HttpResponse::new(
                200,
                echo_client_token(answers.remove(0), request),
            ))
        }
    }

    /// Returns the answer of the server of the tests for the request it
    /// answers.
    ///
    /// The server of the protocol answers the token the launcher named, so the
    /// answer of the tests is written with the token of the request; a server
    /// that answered another token is a failure of the login rather than an
    /// answer of the server.
    fn echo_client_token(answer: String, request: &HttpRequest) -> String {
        let Some(body) = request.body() else {
            return answer;
        };
        let Ok(request_body) = serde_json::from_slice::<Value>(body) else {
            return answer;
        };
        let Some(client_token) = request_body.get("clientToken").and_then(Value::as_str) else {
            return answer;
        };
        let Ok(mut answer) = serde_json::from_str::<Value>(&answer) else {
            return answer;
        };
        if let Some(value) = answer.get_mut("clientToken") {
            *value = Value::String(client_token.to_string());
        }
        answer.to_string()
    }

    /// Returns a directory of the tests that no other test writes into.
    fn temporary_directory(name: &str) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let index = COUNTER.fetch_add(1, Ordering::Relaxed);

        let directory = std::env::temp_dir()
            .join("hmcl-authlibinjector-account")
            .join(format!("{name}-{}-{index}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");
        directory
    }

    /// Returns the file of the agent the tests start the game with.
    ///
    /// The file is the one the launcher ships, written into a directory of the
    /// tests.
    fn artifact() -> Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync> {
        let extractor = crate::auth::authlibinjector::AuthlibInjectorExtractor::shipped(
            temporary_directory("artifact").join("authlib-injector.jar"),
        );
        extractor
            .get_artifact_info()
            .expect("the agent should be written");
        Arc::new(extractor)
    }

    /// Returns a provider that has no file at all.
    fn missing_artifact() -> Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync> {
        Arc::new(SimpleAuthlibInjectorArtifactProvider::new(
            temporary_directory("missing").join("authlib-injector.jar"),
        ))
    }

    /// Returns the answer of a login that selects the player.
    fn login_answer() -> String {
        r#"{"accessToken":"access","clientToken":"client","selectedProfile":{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"},"availableProfiles":[{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"}],"user":{"id":"user"}}"#.to_string()
    }

    /// Returns the selector of the tests, which picks the first character.
    fn first_character(
        _service: &YggdrasilService<AuthlibInjectorProvider>,
        profiles: &[GameProfile],
    ) -> Result<GameProfile, AuthenticationError> {
        profiles
            .first()
            .cloned()
            .ok_or(AuthenticationError::NoSelectedCharacter)
    }

    /// Returns the session of the tests, which names the player.
    fn session() -> YggdrasilSession {
        YggdrasilSession::new(
            "client",
            "access",
            Some(GameProfile::new(
                Uuid::parse(PLAYER).expect("the identifier"),
                "Notch",
            )),
            None,
            None,
        )
    }

    /// Returns the account of the tests, which holds the session of the tests.
    fn account(
        downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
    ) -> AuthlibInjectorAccount {
        AuthlibInjectorAccount::new(
            AccountId::generate(),
            AuthlibInjectorServer::new(API_ROOT),
            downloader,
            "notch@example.com",
            session(),
        )
        .expect("the account should be created")
    }

    /// Returns the account of the tests whose metadata was read already.
    fn account_with_metadata(
        downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
    ) -> AuthlibInjectorAccount {
        let mut account = account(downloader);
        account
            .server
            .set_metadata_response(METADATA, 1000)
            .expect("the metadata should be read");
        account
    }

    #[test]
    fn a_password_login_reads_the_agent_and_the_metadata() {
        let client = FakeClient::new(vec![login_answer(), METADATA.to_string()]);
        let account = AuthlibInjectorAccount::authenticate(
            AccountId::generate(),
            AuthlibInjectorServer::new(API_ROOT),
            artifact(),
            &client,
            "notch@example.com",
            "the-password",
            &first_character,
        )
        .expect("the player should be logged in");

        // The login asked the server for the login and then for the metadata,
        // which is what the agent reads while the game starts.
        assert_eq!(
            client.urls(),
            vec![
                format!("{API_ROOT}authserver/authenticate"),
                API_ROOT.to_string()
            ]
        );
        assert_eq!(account.server().metadata_response(), Some(METADATA));

        let info = account.play_offline().expect("the account should play");
        assert_eq!(info.username, "Notch");
    }

    #[test]
    fn the_arguments_of_the_game_name_the_agent_and_the_metadata() {
        let info = account_with_metadata(artifact())
            .play_offline()
            .expect("the account should play");

        let arguments = info
            .launch_arguments
            .expect("the account should name arguments");
        assert_eq!(arguments.len(), 3);

        let agent = &arguments[0];
        assert!(agent.starts_with("-javaagent:"), "{agent}");
        assert!(agent.ends_with(&format!("={API_ROOT}")), "{agent}");
        assert!(agent.contains("authlib-injector.jar"), "{agent}");

        assert_eq!(arguments[1], "-Dauthlibinjector.side=client");
        assert_eq!(
            arguments[2],
            format!(
                "-Dauthlibinjector.yggdrasil.prefetched={}",
                base64::encode(METADATA.as_bytes())
            )
        );
    }

    #[test]
    fn the_metadata_is_written_into_the_arguments_as_the_server_wrote_it() {
        let info = account_with_metadata(artifact())
            .play_offline()
            .expect("the account should play");
        let arguments = info
            .launch_arguments
            .expect("the account should name arguments");

        // The text is encoded with the standard alphabet and padding, so the
        // agent reads the bytes the server answered with.
        let encoded = arguments[2]
            .rsplit_once('=')
            .expect("the argument should name the metadata")
            .1;
        let decoded = base64::decode(encoded).expect("the metadata should be decoded");
        assert_eq!(decoded, METADATA.as_bytes());
    }

    #[test]
    fn a_player_that_never_logged_in_cannot_play_offline() {
        let account = account_with_metadata(missing_artifact());
        assert_eq!(
            account
                .play_offline()
                .expect_err("the account should not play"),
            AuthenticationError::NotLoggedIn
        );
    }

    #[test]
    fn a_player_whose_metadata_was_not_read_cannot_play_offline() {
        let account = account(artifact());
        assert_eq!(
            account
                .play_offline()
                .expect_err("the account should not play"),
            AuthenticationError::NotLoggedIn
        );
    }

    #[test]
    fn the_address_of_the_server_is_written_with_the_account() {
        let mut metadata = Map::new();
        metadata.insert("profileID".to_string(), Value::String(PLAYER.to_string()));
        account(artifact()).write_metadata(&mut metadata);

        // The address is written together with the data of the account of
        // Yggdrasil, so the launcher finds the server again when it reads the
        // record.
        assert_eq!(
            metadata.get("serverBaseURL").and_then(Value::as_str),
            Some(API_ROOT)
        );
        assert_eq!(
            metadata.get("loginName").and_then(Value::as_str),
            Some("notch@example.com")
        );
        assert_eq!(
            metadata.get("profileID").and_then(Value::as_str),
            Some(PLAYER)
        );
    }

    #[test]
    fn the_metadata_of_the_server_is_read_again_after_the_cache_is_cleared() {
        let client = FakeClient::new(vec![METADATA.to_string()]);
        let mut account = account(artifact());
        account
            .server
            .set_metadata_response(METADATA, 1000)
            .expect("the metadata should be read");

        account.clear_cache();

        // The text is kept, so the pages still show the name of the server,
        // but the next start asks the server again.
        assert_eq!(account.server().metadata_response(), Some(METADATA));
        account
            .server
            .fetch_metadata_response(&client)
            .expect("the metadata should be read again");
        assert_eq!(client.urls(), vec![API_ROOT.to_string()]);
    }

    #[test]
    fn the_account_is_written_with_the_name_the_launcher_of_java_uses() {
        let account = account(artifact());
        let text = account.to_string();

        assert!(
            text.starts_with("AuthlibInjectorAccount [accountID="),
            "{text}"
        );
        assert!(text.contains(&format!("profileID={PLAYER}")), "{text}");
        assert!(text.contains("loginName=notch@example.com"), "{text}");
        assert!(text.ends_with(&format!("server={API_ROOT}]")), "{text}");
    }

    #[test]
    fn a_stored_account_names_its_server_and_its_login_name() {
        let mut metadata = Map::new();
        metadata.insert(
            "accountID".to_string(),
            Value::String(AccountId::generate().to_string()),
        );
        metadata.insert(
            "serverBaseURL".to_string(),
            Value::String(API_ROOT.to_string()),
        );
        metadata.insert(
            "loginName".to_string(),
            Value::String("notch@example.com".to_string()),
        );
        metadata.insert("profileID".to_string(), Value::String(PLAYER.to_string()));

        let mut private_data = Map::new();
        private_data.insert(
            "clientToken".to_string(),
            Value::String("client".to_string()),
        );
        private_data.insert(
            "accessToken".to_string(),
            Value::String("access".to_string()),
        );
        private_data.insert(
            "profileName".to_string(),
            Value::String("Notch".to_string()),
        );

        let account = AuthlibInjectorAccount::from_storage(
            &Value::Object(metadata),
            &private_data,
            artifact(),
            |url| AuthlibInjectorServer::new(url),
        )
        .expect("the account should be read");

        assert_eq!(account.login_name(), "notch@example.com");
        assert_eq!(account.server().url(), API_ROOT);
        assert_eq!(account.profile_name(), "Notch");
    }

    #[test]
    fn a_stored_account_without_a_server_is_refused() {
        let mut metadata = Map::new();
        metadata.insert(
            "accountID".to_string(),
            Value::String(AccountId::generate().to_string()),
        );
        metadata.insert(
            "loginName".to_string(),
            Value::String("notch@example.com".to_string()),
        );

        let error = AuthlibInjectorAccount::from_storage(
            &Value::Object(metadata),
            &Map::new(),
            artifact(),
            |url| AuthlibInjectorServer::new(url),
        )
        .expect_err("the account should be refused");

        assert_eq!(
            error,
            AuthenticationError::invalid_stored("storage does not have API root.")
        );
    }

    #[test]
    fn a_stored_account_without_a_login_name_is_refused() {
        let mut metadata = Map::new();
        metadata.insert(
            "accountID".to_string(),
            Value::String(AccountId::generate().to_string()),
        );
        metadata.insert(
            "serverBaseURL".to_string(),
            Value::String(API_ROOT.to_string()),
        );

        let error = AuthlibInjectorAccount::from_storage(
            &Value::Object(metadata),
            &Map::new(),
            artifact(),
            |url| AuthlibInjectorServer::new(url),
        )
        .expect_err("the account should be refused");

        assert_eq!(
            error,
            AuthenticationError::invalid_stored("storage does not have loginName")
        );
    }

    #[test]
    fn the_properties_of_a_stored_profile_are_read_into_the_cache() {
        let mut metadata = Map::new();
        metadata.insert(
            "accountID".to_string(),
            Value::String(AccountId::generate().to_string()),
        );
        metadata.insert(
            "serverBaseURL".to_string(),
            Value::String(API_ROOT.to_string()),
        );
        metadata.insert(
            "loginName".to_string(),
            Value::String("notch@example.com".to_string()),
        );
        metadata.insert("profileID".to_string(), Value::String(PLAYER.to_string()));

        let mut private_data = Map::new();
        private_data.insert(
            "clientToken".to_string(),
            Value::String("client".to_string()),
        );
        private_data.insert(
            "accessToken".to_string(),
            Value::String("access".to_string()),
        );
        private_data.insert(
            "profileName".to_string(),
            Value::String("Notch".to_string()),
        );
        private_data.insert(
            "profileProperties".to_string(),
            serde_json::json!({"uploadableTextures": "skin,cape"}),
        );

        let account = AuthlibInjectorAccount::from_storage(
            &Value::Object(metadata),
            &private_data,
            artifact(),
            |url| AuthlibInjectorServer::new(url),
        )
        .expect("the account should be read");

        // The properties are put into the cache and dropped at once, so the
        // profile the cache holds is the one the next read asks the server for.
        assert_eq!(
            account
                .account()
                .service()
                .cached_profile(account.profile_id())
                .map(|profile| profile.property("uploadableTextures").map(str::to_string)),
            None
        );
    }

    #[test]
    fn a_login_reads_the_metadata_again_when_the_server_is_asked() {
        let client = FakeClient::new(vec![String::new(), METADATA.to_string()]);
        let mut account = account_with_metadata(artifact());

        // The token of the session is checked first and accepted, so the
        // session is kept; the metadata is read again because the account did
        // not read it during this run.
        let info = account
            .log_in(&client)
            .expect("the player should be logged in");

        assert_eq!(
            client.urls(),
            vec![
                format!("{API_ROOT}authserver/validate"),
                API_ROOT.to_string()
            ]
        );
        assert!(info.launch_arguments.is_some());
    }
}
