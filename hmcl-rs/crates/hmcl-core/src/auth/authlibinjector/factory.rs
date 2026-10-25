//! The factories that create and read the accounts of the servers of the
//! authlib-injector.
//!
//! A player of such a server logs in with a name and a password, so the page of
//! the login asks for both. The launcher holds two factories: one for the
//! servers the player named in the list, which looks a server up by the address
//! of a record, and one that is bound to a server the player chose, which is
//! what the page of the accounts uses when the player adds an account of the
//! server it looks at.
//!
//! Both read a record the same way, so a record written by either factory reads
//! back with the other one, which is what the launcher of Java does as well.

use std::sync::Arc;

use serde_json::{Map, Value};

use crate::auth::account_factory::{AccountFactoryKind, AccountLoginType};
use crate::auth::authlibinjector::artifact::AuthlibInjectorArtifactProvider;
use crate::auth::yggdrasil::CharacterSelector;
use crate::auth::{AccountId, AuthenticationError};
use crate::setting::authlib_servers::AuthlibInjectorServer;
use crate::util::http_request::HttpClient;

use super::account::AuthlibInjectorAccount;

/// The factory of the accounts of the servers the player named.
///
/// The factory holds the provider of the file of the agent and a function that
/// finds a server by the address of a record, which is what the launcher of
/// Java passes as `Accounts::getOrCreateAuthlibInjectorServer`: a server the
/// player named is the server the account belongs to, and a server the list
/// does not name is added to it rather than replaced by a new one.
#[derive(Clone)]
pub struct AuthlibInjectorAccountFactory<F> {
    /// The provider of the file of the agent the game is started with.
    downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
    /// The function that finds a server by the address a record names.
    server_lookup: F,
}

impl<F> AuthlibInjectorAccountFactory<F>
where
    F: Fn(&str) -> AuthlibInjectorServer,
{
    /// Creates the factory of the servers the player named.
    pub fn new(
        downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
        server_lookup: F,
    ) -> Self {
        Self {
            downloader,
            server_lookup,
        }
    }

    /// Returns what the factory needs before it logs a player in.
    ///
    /// A player of a server of the authlib-injector logs in with a name and a
    /// password, so the page of the login asks for both.
    pub const fn login_type(&self) -> AccountLoginType {
        AccountFactoryKind::AuthlibInjector.login_type()
    }

    /// Logs a player in and creates the account of the player.
    ///
    /// The server is the one the player chose in the list, which the page of
    /// the login passes as the extra data of the request. The password is not
    /// kept: what the account keeps is the session the server answered with.
    pub fn create(
        &self,
        client: &dyn HttpClient,
        selector: &dyn CharacterSelector,
        username: &str,
        password: &str,
        server: AuthlibInjectorServer,
    ) -> Result<AuthlibInjectorAccount, AuthenticationError> {
        self.create_with_id(
            client,
            selector,
            username,
            password,
            server,
            AccountId::generate(),
        )
    }

    /// Logs a player in and creates the account of the player with an
    /// identifier the caller names.
    ///
    /// A page that already reserved the identifier of the record passes it, so
    /// the record is written with the identifier the page shows; a page that
    /// has none calls [`Self::create`] instead.
    pub fn create_with_id(
        &self,
        client: &dyn HttpClient,
        selector: &dyn CharacterSelector,
        username: &str,
        password: &str,
        server: AuthlibInjectorServer,
        account_id: AccountId,
    ) -> Result<AuthlibInjectorAccount, AuthenticationError> {
        AuthlibInjectorAccount::authenticate(
            account_id,
            server,
            Arc::clone(&self.downloader),
            client,
            username,
            password,
            selector,
        )
    }

    /// Reads the account of a player from the settings of the launcher.
    ///
    /// The record names the address of the server, which is looked up in the
    /// list of the servers the player named; a record without an address is
    /// not an account of this kind, which is the failure the launcher of Java
    /// reports as `storage does not have API root.`.
    pub fn from_storage(
        &self,
        metadata: &Value,
        private_data: &Map<String, Value>,
    ) -> Result<AuthlibInjectorAccount, AuthenticationError> {
        let api_root = metadata
            .get("serverBaseURL")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AuthenticationError::invalid_stored("storage does not have API root.")
            })?;

        let server = (self.server_lookup)(api_root);
        self.from_storage_with_server(metadata, private_data, server)
    }

    /// Reads the account of a player with the server the record belongs to.
    pub fn from_storage_with_server(
        &self,
        metadata: &Value,
        private_data: &Map<String, Value>,
        server: AuthlibInjectorServer,
    ) -> Result<AuthlibInjectorAccount, AuthenticationError> {
        AuthlibInjectorAccount::from_storage_with_server(
            metadata,
            private_data,
            Arc::clone(&self.downloader),
            server,
        )
    }
}

impl<F> std::fmt::Debug for AuthlibInjectorAccountFactory<F> {
    /// Writes the factory without the function that finds the servers, which
    /// has no text.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthlibInjectorAccountFactory")
            .finish_non_exhaustive()
    }
}

/// The factory of the accounts of one server the player chose.
///
/// The page of the accounts opens the login of a server the player looks at,
/// so the server is known before the player logs in and the factory does not
/// look it up by the address of the record. The kind of the accounts is the
/// kind of the servers the player named, so a record of a bound factory is
/// written with the same name and read back by either factory.
#[derive(Clone)]
pub struct BoundAuthlibInjectorAccountFactory {
    /// The provider of the file of the agent the game is started with.
    downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
    /// The server the accounts belong to.
    server: AuthlibInjectorServer,
}

impl std::fmt::Debug for BoundAuthlibInjectorAccountFactory {
    /// Writes the factory with the server it holds; the provider of the file
    /// of the agent has no text.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BoundAuthlibInjectorAccountFactory")
            .field("server", &self.server)
            .finish_non_exhaustive()
    }
}

impl BoundAuthlibInjectorAccountFactory {
    /// Creates the factory of the accounts of a server.
    pub fn new(
        downloader: Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync>,
        server: AuthlibInjectorServer,
    ) -> Self {
        Self { downloader, server }
    }

    /// Returns what the factory needs before it logs a player in.
    pub const fn login_type(&self) -> AccountLoginType {
        AccountFactoryKind::AuthlibInjector.login_type()
    }

    /// Returns the server the accounts belong to.
    pub const fn server(&self) -> &AuthlibInjectorServer {
        &self.server
    }

    /// Logs a player in and creates the account of the player.
    ///
    /// The extra data the page of the login names is ignored, because the
    /// server is the one the factory was created for.
    pub fn create_with_client(
        &self,
        selector: &dyn CharacterSelector,
        username: &str,
        password: &str,
        account_id: AccountId,
        client: &dyn HttpClient,
    ) -> Result<AuthlibInjectorAccount, AuthenticationError> {
        AuthlibInjectorAccount::authenticate(
            account_id,
            self.server.clone(),
            Arc::clone(&self.downloader),
            client,
            username,
            password,
            selector,
        )
    }

    /// Reads the account of a player from the settings of the launcher.
    ///
    /// The address the record names is not read, because the factory is the
    /// factory of the server the account belongs to.
    pub fn from_storage(
        &self,
        metadata: &Value,
        private_data: &Map<String, Value>,
    ) -> Result<AuthlibInjectorAccount, AuthenticationError> {
        AuthlibInjectorAccount::from_storage_with_server(
            metadata,
            private_data,
            Arc::clone(&self.downloader),
            self.server.clone(),
        )
    }
}

/// The kind of the accounts of the servers of the authlib-injector.
///
/// The bound factory of a server is a factory of the same kind, so the name is
/// named here rather than in the two factories.
pub const AUTHLIB_INJECTOR_LOGIN_TYPE: &str = "authlibInjector";

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::json;

    use crate::auth::authlibinjector::artifact::SimpleAuthlibInjectorArtifactProvider;
    use crate::auth::yggdrasil::{AuthlibInjectorProvider, GameProfile, YggdrasilService};
    use crate::auth::{AuthlibInjectorAccount as StoredAuthlibInjectorAccount, Uuid};
    use crate::util::http_request::{HttpRequest, HttpResponse};

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
        /// The addresses the client was asked.
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
    /// answers, with the token the launcher named.
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
            .join("hmcl-authlibinjector-factory")
            .join(format!("{name}-{}-{index}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");
        directory
    }

    /// Returns a provider of the agent that has no file at all.
    fn missing_artifact() -> Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync> {
        Arc::new(SimpleAuthlibInjectorArtifactProvider::new(
            temporary_directory("missing").join("authlib-injector.jar"),
        ))
    }

    /// Returns the file of the agent the tests start the game with.
    ///
    /// A login reads the agent and the metadata, so the tests of a login use
    /// the file the launcher ships rather than a file that is not there.
    fn artifact() -> Arc<dyn AuthlibInjectorArtifactProvider + Send + Sync> {
        let extractor = crate::auth::authlibinjector::AuthlibInjectorExtractor::shipped(
            temporary_directory("artifact").join("authlib-injector.jar"),
        );
        extractor
            .get_artifact_info()
            .expect("the agent should be written");
        Arc::new(extractor)
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

    /// Returns the public and the protected data of a stored account.
    fn stored_account() -> (Value, Map<String, Value>) {
        let metadata = json!({
            "accountID": AccountId::generate().to_string(),
            "type": "authlibInjector",
            "serverBaseURL": API_ROOT,
            "loginName": "notch@example.com",
            "profileID": PLAYER,
        });
        let private_data = json!({
            "clientToken": "client",
            "accessToken": "access",
            "profileName": "Notch",
        })
        .as_object()
        .expect("the data should be an object")
        .clone();
        (metadata, private_data)
    }

    #[test]
    fn the_page_of_the_login_asks_a_player_of_a_server_for_a_name_and_a_password() {
        let factory = AuthlibInjectorAccountFactory::new(missing_artifact(), |url| {
            AuthlibInjectorServer::new(url)
        });

        assert!(factory.login_type().requires_username());
        assert!(factory.login_type().requires_password());
        assert_eq!(factory.login_type(), AccountLoginType::UsernamePassword);
    }

    #[test]
    fn a_bound_factory_asks_for_the_same_credentials_and_names_its_server() {
        let factory = BoundAuthlibInjectorAccountFactory::new(
            missing_artifact(),
            AuthlibInjectorServer::new(API_ROOT),
        );

        assert_eq!(factory.login_type(), AccountLoginType::UsernamePassword);
        assert_eq!(factory.server().url(), API_ROOT);
    }

    #[test]
    fn a_login_creates_the_account_of_the_player() {
        let client = FakeClient::new(vec![login_answer(), METADATA.to_string()]);
        let factory =
            AuthlibInjectorAccountFactory::new(artifact(), |url| AuthlibInjectorServer::new(url));

        let account = factory
            .create(
                &client,
                &first_character,
                "notch@example.com",
                "the-password",
                AuthlibInjectorServer::new(API_ROOT),
            )
            .expect("the player should be logged in");

        assert_eq!(account.login_name(), "notch@example.com");
        assert_eq!(account.profile_name(), "Notch");
        assert_eq!(account.server().url(), API_ROOT);
        assert_eq!(
            account.profile_id().to_string(),
            Uuid::parse(PLAYER).expect("the identifier").to_string()
        );
    }

    #[test]
    fn a_bound_factory_logs_in_to_the_server_it_holds() {
        let client = FakeClient::new(vec![login_answer(), METADATA.to_string()]);
        let factory = BoundAuthlibInjectorAccountFactory::new(
            artifact(),
            AuthlibInjectorServer::new(API_ROOT),
        );

        let account = factory
            .create_with_client(
                &first_character,
                "notch@example.com",
                "the-password",
                AccountId::generate(),
                &client,
            )
            .expect("the player should be logged in");

        // The login asked the server the factory holds, so the account belongs
        // to the server the player looked at rather than to another one.
        assert_eq!(account.server().url(), API_ROOT);
    }

    #[test]
    fn a_stored_record_is_read_with_the_server_the_record_names() {
        let (metadata, private_data) = stored_account();
        let factory = AuthlibInjectorAccountFactory::new(missing_artifact(), |url| {
            AuthlibInjectorServer::new(url)
        });

        let account = factory
            .from_storage(&metadata, &private_data)
            .expect("the account should be read");

        assert_eq!(account.server().url(), API_ROOT);
        assert_eq!(account.login_name(), "notch@example.com");
        assert_eq!(account.profile_name(), "Notch");
    }

    #[test]
    fn a_stored_record_without_a_server_is_refused() {
        let (mut metadata, private_data) = stored_account();
        metadata
            .as_object_mut()
            .expect("the data should be an object")
            .remove("serverBaseURL");
        let factory = AuthlibInjectorAccountFactory::new(missing_artifact(), |url| {
            AuthlibInjectorServer::new(url)
        });

        let error = factory
            .from_storage(&metadata, &private_data)
            .expect_err("the account should be refused");

        assert_eq!(
            error,
            AuthenticationError::invalid_stored("storage does not have API root.")
        );
    }

    #[test]
    fn a_bound_factory_reads_a_record_of_its_server() {
        let (mut metadata, private_data) = stored_account();
        // The record of the tests names another address, which the bound
        // factory does not read: the server is the one the factory holds.
        metadata
            .as_object_mut()
            .expect("the data should be an object")
            .insert(
                "serverBaseURL".to_string(),
                Value::String("https://other.example.com/api/yggdrasil/".to_string()),
            );
        let factory = BoundAuthlibInjectorAccountFactory::new(
            missing_artifact(),
            AuthlibInjectorServer::new(API_ROOT),
        );

        let account = factory
            .from_storage(&metadata, &private_data)
            .expect("the account should be read");

        assert_eq!(account.server().url(), API_ROOT);
    }

    #[test]
    fn a_stored_record_without_a_login_name_is_refused() {
        let (mut metadata, private_data) = stored_account();
        metadata
            .as_object_mut()
            .expect("the data should be an object")
            .remove("loginName");
        let factory = BoundAuthlibInjectorAccountFactory::new(
            missing_artifact(),
            AuthlibInjectorServer::new(API_ROOT),
        );

        let error = factory
            .from_storage(&metadata, &private_data)
            .expect_err("the account should be refused");

        assert_eq!(
            error,
            AuthenticationError::invalid_stored("storage does not have loginName")
        );
    }

    #[test]
    fn the_kind_of_the_accounts_is_the_kind_of_the_servers_of_the_authlib_injector() {
        assert_eq!(
            AccountFactoryKind::AuthlibInjector.login_type_id(),
            AUTHLIB_INJECTOR_LOGIN_TYPE
        );
        assert_eq!(
            AccountFactoryKind::from_login_type(AUTHLIB_INJECTOR_LOGIN_TYPE),
            Some(AccountFactoryKind::AuthlibInjector)
        );
    }

    #[test]
    fn the_record_of_the_kind_is_read_by_the_stored_account_of_the_kind() {
        // The record of the tests is the record the launcher writes, so the
        // reader of the settings reads it back as an account of a server.
        let (metadata, private_data) = stored_account();
        let account: StoredAuthlibInjectorAccount = serde_json::from_value(json!({
            "type": "authlibInjector",
            "serverUrl": API_ROOT,
            "loginName": "notch@example.com",
            "profileName": "Notch",
            "profileID": PLAYER,
        }))
        .expect("the record should be read");

        assert_eq!(account.login_name, "notch@example.com");
        assert_eq!(
            metadata.get("type").and_then(Value::as_str),
            Some("authlibInjector")
        );
        assert_eq!(
            private_data.get("accessToken").and_then(Value::as_str),
            Some("access")
        );
    }
}
