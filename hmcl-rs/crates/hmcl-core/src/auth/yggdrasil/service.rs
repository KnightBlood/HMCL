//! The requests of the protocol of Yggdrasil.
//!
//! The server of a player answers five requests: it logs a player in with a
//! name and a password, it renews an access token, it checks a token, it
//! revokes a token, and it names the properties of the profile of a player. All
//! of them answer JSON, and a refusal is a status that is not a success with a
//! body that says why, which is why the service reads the body of every answer
//! before it looks at the status.
//!
//! The service also keeps the complete profiles it read, because the profile of
//! a player is asked for every time the launcher shows the skin of the player,
//! and a server of the authlib-injector is often slower than the servers of
//! Mojang. The cache is the repository of profiles the launcher of Java keeps.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{AuthenticationError, Uuid};
use crate::util::http_request::{send, HttpClient, HttpMethod, HttpRequest};
use crate::util::multipart::MultipartForm;

use super::model::{CompleteGameProfile, GameProfile, User};
use super::provider::YggdrasilProvider;
use super::session::YggdrasilSession;

/// The number of the attempts a request of the service makes.
const ATTEMPTS: u32 = 5;

/// The name of the agent the launcher names itself with.
///
/// The servers of Yggdrasil read the name of the agent to know which program
/// asks them; the game and the launcher both name the game, which is what the
/// servers know.
const AGENT_NAME: &str = "Minecraft";

/// The version of the protocol the launcher speaks.
const AGENT_VERSION: u32 = 1;

/// The server of the protocol of Yggdrasil a player logs in to.
///
/// The service holds the provider of the addresses and the repository of the
/// complete profiles it read. The repository is shared by the accounts that log
/// in to the same server, so a launcher that shows the skin of a player twice
/// asks the server once.
#[derive(Debug)]
pub struct YggdrasilService<P> {
    /// The provider of the addresses of the server.
    provider: P,
    /// The complete profiles the launcher read, by the identifier of the
    /// player.
    profiles: Mutex<BTreeMap<Uuid, CompleteGameProfile>>,
}

impl<P: YggdrasilProvider> YggdrasilService<P> {
    /// Creates the service of a server.
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            profiles: Mutex::new(BTreeMap::new()),
        }
    }

    /// Returns the provider of the addresses of the server.
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Logs a player in with a name and a password.
    ///
    /// The server may select one of the profiles of the account, or none when
    /// the account holds several: the answer names the profiles the player may
    /// select, and the caller selects one and renews the session with the
    /// selection.
    pub fn authenticate(
        &self,
        client: &dyn HttpClient,
        username: &str,
        password: &str,
        client_token: &str,
    ) -> Result<YggdrasilSession, AuthenticationError> {
        let body = json!({
            "agent": { "name": AGENT_NAME, "version": AGENT_VERSION },
            "username": username,
            "password": password,
            "clientToken": client_token,
            "requestUser": true,
        });

        let text = self.request(client, self.provider.authentication_url()?, Some(&body))?;
        handle_authentication_response(&text, client_token)
    }

    /// Renews the tokens of an account.
    ///
    /// The profile the player selected is named in the request, so the server
    /// selects it again and the answer names it; a server that selects another
    /// profile, or none, is a failure rather than a renewal, because the
    /// launcher would otherwise log the player in as another player.
    pub fn refresh(
        &self,
        client: &dyn HttpClient,
        access_token: &str,
        client_token: &str,
        character_to_select: Option<&GameProfile>,
    ) -> Result<YggdrasilSession, AuthenticationError> {
        let mut body = json!({
            "accessToken": access_token,
            "clientToken": client_token,
            "requestUser": true,
        });

        if let Some(character) = character_to_select {
            body["selectedProfile"] = json!({
                "id": character.id().compact(),
                "name": character.name(),
            });
        }

        let text = self.request(client, self.provider.refreshment_url()?, Some(&body))?;
        let session = handle_authentication_response(&text, client_token)?;

        if let Some(character) = character_to_select {
            let selected = session.selected_profile();
            if selected.map(GameProfile::id) != Some(character.id()) {
                return Err(AuthenticationError::malformed("Failed to select character"));
            }
        }

        Ok(session)
    }

    /// Checks whether an access token is still accepted by the server.
    ///
    /// A server refuses a token it does not know with the name
    /// `ForbiddenOperationException`, which is the one refusal the caller reads
    /// as an expired token; every other refusal is a failure of the login, since
    /// it says something about the account rather than about the token.
    pub fn validate(
        &self,
        client: &dyn HttpClient,
        access_token: &str,
        client_token: Option<&str>,
    ) -> Result<bool, AuthenticationError> {
        let body = credentials(access_token, client_token);
        let text = self.request(client, self.provider.validation_url()?, Some(&body))?;

        match require_empty(&text) {
            Ok(()) => Ok(true),
            Err(error) => match &error {
                AuthenticationError::RemoteAuthentication { name, .. }
                    if name == FORBIDDEN_OPERATION =>
                {
                    Ok(false)
                }
                _ => Err(error),
            },
        }
    }

    /// Revokes an access token, so the server does not accept it any more.
    pub fn invalidate(
        &self,
        client: &dyn HttpClient,
        access_token: &str,
        client_token: Option<&str>,
    ) -> Result<(), AuthenticationError> {
        let body = credentials(access_token, client_token);
        let text = self.request(client, self.provider.invalidation_url()?, Some(&body))?;
        require_empty(&text)
    }

    /// Uploads the file of the skin of a player.
    ///
    /// The request is a form of two parts: the model of the skin, which is
    /// empty for the wide arms, and the file of the skin with the name and the
    /// type of the file. The type of the file is derived from the extension of
    /// the name, which is what the launcher of Java tells the server.
    pub fn upload_skin(
        &self,
        client: &dyn HttpClient,
        uuid: Uuid,
        access_token: &str,
        slim: bool,
        file_name: &str,
        data: &[u8],
    ) -> Result<(), AuthenticationError> {
        let content_type = format!("image/{}", extension(file_name));
        let body = MultipartForm::new()
            .field("model", if slim { "slim" } else { "" })
            .file("file", file_name, &content_type, data);

        let boundary_type = body.content_type();
        let request = HttpRequest::new(self.provider.skin_upload_url(uuid)?, HttpMethod::Put)
            .authorization(format!("Bearer {access_token}"))
            .raw_body(body.finish(), &boundary_type)
            .ignore_status()
            .attempts(ATTEMPTS);

        let response = send(client, &request).map_err(AuthenticationError::from)?;
        require_empty(&response.body)
    }

    /// Reads the properties of the profile of a player.
    ///
    /// A server that does not know the player answers the text `null`, which
    /// the launcher reads as a profile it does not know rather than as a
    /// failure of the server.
    pub fn complete_game_profile(
        &self,
        client: &dyn HttpClient,
        uuid: Uuid,
    ) -> Result<Option<CompleteGameProfile>, AuthenticationError> {
        let text = self.request(client, self.provider.profile_properties_url(uuid)?, None)?;

        if text.trim().is_empty() || text.trim() == "null" {
            return Ok(None);
        }

        let profile: CompleteGameProfile = serde_json::from_str(&text).map_err(|error| {
            AuthenticationError::malformed(format!(
                "the properties of the profile of the player are malformed: {error}"
            ))
        })?;

        self.put_profile(profile.clone());
        Ok(Some(profile))
    }

    /// Returns the complete profile of a player the launcher already read.
    pub fn cached_profile(&self, uuid: Uuid) -> Option<CompleteGameProfile> {
        lock(&self.profiles).get(&uuid).cloned()
    }

    /// Keeps a complete profile the launcher read elsewhere.
    ///
    /// The launcher reads the properties of the profiles it stored when it
    /// starts, so it shows the skin of the players before it asks a server.
    pub fn put_profile(&self, profile: CompleteGameProfile) {
        lock(&self.profiles).insert(profile.id(), profile);
    }

    /// Forgets the complete profile of a player, so the next read asks the
    /// server.
    pub fn invalidate_profile(&self, uuid: Uuid) {
        lock(&self.profiles).remove(&uuid);
    }

    /// Sends a request and returns the text of the answer.
    ///
    /// A request without a body is a `GET` and a request with one is a `POST`
    /// of JSON, which is the way the launcher of Java writes the two. The status
    /// of the answer is not read: a server refuses a login with a status that is
    /// not a success and names the reason in the body, so the body is what the
    /// caller reads.
    fn request(
        &self,
        client: &dyn HttpClient,
        url: String,
        payload: Option<&Value>,
    ) -> Result<String, AuthenticationError> {
        let request = match payload {
            None => HttpRequest::get(url),
            Some(payload) => HttpRequest::post(url).json(payload),
        }
        .ignore_status()
        .attempts(ATTEMPTS);

        let response = send(client, &request).map_err(AuthenticationError::from)?;
        Ok(response.body)
    }
}

/// Returns the body that names the credentials of an account.
fn credentials(access_token: &str, client_token: Option<&str>) -> Value {
    json!({
        "accessToken": access_token,
        "clientToken": client_token,
    })
}

/// The name a server refuses a token it does not know with.
const FORBIDDEN_OPERATION: &str = "ForbiddenOperationException";

/// Returns the extension of a name of a file.
///
/// The extension is the text after the last dot, or nothing when the name has
/// no dot, which is the way the launcher of Java reads it before it names the
/// type of the file.
fn extension(file_name: &str) -> &str {
    match file_name.rsplit_once('.') {
        Some((_, extension)) => extension,
        None => "",
    }
}

/// Returns the lock of the repository of profiles.
///
/// A thread that panicked while it held the lock left the repository whole, so
/// the poison of the lock is ignored rather than reported.
fn lock(
    profiles: &Mutex<BTreeMap<Uuid, CompleteGameProfile>>,
) -> std::sync::MutexGuard<'_, BTreeMap<Uuid, CompleteGameProfile>> {
    profiles.lock().unwrap_or_else(|error| error.into_inner())
}

/// Reads the answer of a login or of a renewal.
///
/// The answer names the tokens and the profiles, or the reason the server
/// refused the login. The token that names the launcher is checked, because a
/// server that answers another token is a server the launcher cannot trust: the
/// token is what tells the launcher apart from the other programs that ask the
/// server for the same account.
fn handle_authentication_response(
    text: &str,
    client_token: &str,
) -> Result<YggdrasilSession, AuthenticationError> {
    let response: AuthenticationResponse = serde_json::from_str(text).map_err(|error| {
        AuthenticationError::malformed(format!("the answer of the login is not JSON: {error}"))
    })?;

    handle_error_message(&response.error)?;

    match response.client_token.as_deref() {
        Some(answered) if answered == client_token => {}
        answered => {
            return Err(AuthenticationError::malformed(format!(
                "Client token changed from {client_token} to {}",
                answered.unwrap_or("null")
            )))
        }
    }

    let access_token = response.access_token.ok_or_else(|| {
        AuthenticationError::malformed("the answer of the login names no access token")
    })?;

    Ok(YggdrasilSession::new(
        response.client_token.unwrap_or_default(),
        access_token,
        response.selected_profile,
        response.available_profiles,
        super::session::read_user_properties(response.user),
    ))
}

/// Reads an answer that must hold no text, or the reason of a refusal.
///
/// A server that accepted a request may answer nothing at all, which is why a
/// blank answer is a success; an answer that holds a text is read as the
/// refusal of the server.
fn require_empty(text: &str) -> Result<(), AuthenticationError> {
    if text.trim().is_empty() {
        return Ok(());
    }

    let response: ErrorResponse = serde_json::from_str(text).map_err(|error| {
        AuthenticationError::malformed(format!(
            "the answer of the server is not the JSON the launcher reads: {error}"
        ))
    })?;
    handle_error_message(&response)
}

/// Reports the refusal a server answered with, if it named one.
fn handle_error_message(response: &ErrorResponse) -> Result<(), AuthenticationError> {
    match response.error.as_deref() {
        Some(name) if !name.trim().is_empty() => Err(AuthenticationError::RemoteAuthentication {
            name: name.to_string(),
            message: response.error_message.clone().unwrap_or_default(),
            cause: response.cause.clone().unwrap_or_default(),
        }),
        _ => Ok(()),
    }
}

/// The reason a server refused a request.
#[derive(Debug, Clone, Default, Deserialize)]
struct ErrorResponse {
    /// The name of the reason, such as `ForbiddenOperationException`.
    #[serde(default)]
    error: Option<String>,
    /// The message of the reason, which the launcher shows the player.
    #[serde(default, rename = "errorMessage")]
    error_message: Option<String>,
    /// The cause the server named, which is often empty.
    #[serde(default)]
    cause: Option<String>,
}

/// The answer of a login or of a renewal.
#[derive(Debug, Clone, Default, Deserialize)]
struct AuthenticationResponse {
    /// The refusal the server named instead of the tokens, if any.
    #[serde(flatten)]
    error: ErrorResponse,
    /// The token the game is started with.
    #[serde(default, rename = "accessToken")]
    access_token: Option<String>,
    /// The token that names the launcher.
    #[serde(default, rename = "clientToken")]
    client_token: Option<String>,
    /// The profile the server selected.
    #[serde(default, rename = "selectedProfile")]
    selected_profile: Option<GameProfile>,
    /// The profiles the player may select.
    #[serde(default, rename = "availableProfiles")]
    available_profiles: Option<Vec<GameProfile>>,
    /// The user of the account.
    #[serde(default)]
    user: Option<User>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::http_request::HttpResponse;
    use std::cell::RefCell;

    /// The identifier of the player of the tests.
    const PLAYER: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
    /// The identifier of the second player of the tests.
    const OTHER: &str = "b50ad385-829d-3141-a216-7e7d7539ba7f";

    /// The server of the tests, which answers with the answers it is given.
    struct Server {
        /// The addresses the service asks, in the order it asks them.
        urls: Vec<String>,
        /// The answers, in the order they are returned.
        answers: Vec<String>,
        /// The statuses of the answers, in the order they are returned.
        statuses: Vec<u16>,
    }

    impl Server {
        /// Creates a server that answers with the answers in order.
        fn new(urls: &[&str], answers: Vec<String>) -> Self {
            Self {
                urls: urls.iter().map(|url| url.to_string()).collect(),
                statuses: vec![200; answers.len()],
                answers,
            }
        }
    }

    impl YggdrasilProvider for Server {
        fn authentication_url(&self) -> Result<String, AuthenticationError> {
            Ok(self.urls[0].clone())
        }

        fn refreshment_url(&self) -> Result<String, AuthenticationError> {
            Ok(self.urls[1].clone())
        }

        fn validation_url(&self) -> Result<String, AuthenticationError> {
            Ok(self.urls[2].clone())
        }

        fn invalidation_url(&self) -> Result<String, AuthenticationError> {
            Ok(self.urls[3].clone())
        }

        fn skin_upload_url(&self, uuid: Uuid) -> Result<String, AuthenticationError> {
            Ok(format!("https://example.com/skin/{}", uuid.compact()))
        }

        fn profile_properties_url(&self, uuid: Uuid) -> Result<String, AuthenticationError> {
            Ok(format!("https://example.com/profile/{}", uuid.compact()))
        }
    }

    /// The addresses the servers of the tests answer at.
    fn urls() -> Vec<&'static str> {
        vec![
            "https://example.com/authenticate",
            "https://example.com/refresh",
            "https://example.com/validate",
            "https://example.com/invalidate",
        ]
    }

    /// A client that answers with the answers of a server and keeps the
    /// requests it was asked.
    struct FakeClient {
        /// The server whose answers are returned.
        server: RefCell<Server>,
        /// The requests the client was asked, in the order they were asked.
        asked: RefCell<Vec<HttpRequest>>,
    }

    impl FakeClient {
        /// Creates a client for a server.
        fn new(server: Server) -> Self {
            Self {
                server: RefCell::new(server),
                asked: RefCell::new(Vec::new()),
            }
        }

        /// Returns the bodies of the requests the client was asked.
        fn bodies(&self) -> Vec<String> {
            self.asked
                .borrow()
                .iter()
                .map(|request| {
                    String::from_utf8_lossy(request.body().unwrap_or_default()).to_string()
                })
                .collect()
        }

        /// Returns the requests the client was asked.
        fn requests(&self) -> Vec<HttpRequest> {
            self.asked.borrow().clone()
        }
    }

    impl HttpClient for FakeClient {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
            self.asked.borrow_mut().push(request.clone());
            let mut server = self.server.borrow_mut();
            if server.answers.is_empty() {
                return Err("the client was asked too often".to_string());
            }
            let answer = server.answers.remove(0);
            let status = server.statuses.remove(0);
            Ok(HttpResponse::new(status, answer))
        }
    }

    /// The answer of a login of the player.
    fn login_answer(client_token: &str) -> String {
        format!(
            r#"{{"accessToken":"access","clientToken":"{client_token}","selectedProfile":{{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"}},"availableProfiles":[{{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"}}],"user":{{"id":"user","properties":[{{"name":"preferredLanguage","value":"zh_CN"}}]}}}}"#
        )
    }

    /// The service of the tests, whose provider answers every address.
    fn service() -> YggdrasilService<Server> {
        YggdrasilService::new(Server::new(&urls(), Vec::new()))
    }

    #[test]
    fn a_login_names_the_agent_the_player_and_the_properties() {
        let client = FakeClient::new(Server::new(&urls(), vec![login_answer("client")]));
        let service = service();

        let session = service
            .authenticate(&client, "notch@example.com", "the-password", "client")
            .expect("the player should be logged in");

        let body: Value =
            serde_json::from_str(&client.bodies()[0]).expect("the body should be JSON");
        assert_eq!(body["agent"]["name"], "Minecraft");
        assert_eq!(body["agent"]["version"], 1);
        assert_eq!(body["username"], "notch@example.com");
        assert_eq!(body["password"], "the-password");
        assert_eq!(body["clientToken"], "client");
        assert_eq!(body["requestUser"], true);

        assert_eq!(session.access_token(), "access");
        assert_eq!(
            session.selected_profile().map(GameProfile::name),
            Some("Notch")
        );
        assert_eq!(
            session.available_profiles().map(<[GameProfile]>::len),
            Some(1)
        );
        assert_eq!(
            session
                .user_properties()
                .and_then(|properties| properties.get("preferredLanguage"))
                .map(String::as_str),
            Some("zh_CN")
        );
    }

    #[test]
    fn a_login_is_a_post_of_json() {
        let client = FakeClient::new(Server::new(&urls(), vec![login_answer("client")]));
        let service = service();
        service
            .authenticate(&client, "name", "password", "client")
            .expect("the player should be logged in");

        let request = &client.requests()[0];
        assert_eq!(request.method(), HttpMethod::Post);
        assert_eq!(request.url(), "https://example.com/authenticate");
        assert!(request.ignores_status());
    }

    #[test]
    fn a_refused_login_names_the_reason_of_the_server() {
        let client = FakeClient::new(Server::new(
            &urls(),
            vec![
                r#"{"error":"ForbiddenOperationException","errorMessage":"Invalid credentials. Invalid username or password.","cause":"the cause"}"#
                    .to_string(),
            ],
        ));
        let service = service();

        let error = service
            .authenticate(&client, "name", "password", "client")
            .expect_err("the login should be refused");
        assert_eq!(
            error,
            AuthenticationError::RemoteAuthentication {
                name: "ForbiddenOperationException".to_string(),
                message: "Invalid credentials. Invalid username or password.".to_string(),
                cause: "the cause".to_string(),
            }
        );
    }

    #[test]
    fn a_login_that_answers_another_token_is_refused() {
        let client = FakeClient::new(Server::new(&urls(), vec![login_answer("another")]));
        let service = service();

        let error = service
            .authenticate(&client, "name", "password", "client")
            .expect_err("the login should be refused");
        assert_eq!(
            error,
            AuthenticationError::malformed("Client token changed from client to another")
        );
    }

    #[test]
    fn a_login_without_a_profile_names_the_profiles_to_select() {
        let client = FakeClient::new(Server::new(
            &urls(),
            vec![r#"{"accessToken":"access","clientToken":"client","availableProfiles":[{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"},{"id":"b50ad385829d3141a2167e7d7539ba7f","name":"Herobrine"}],"user":{"id":"user"}}"#
                .to_string()],
        ));
        let service = service();

        let session = service
            .authenticate(&client, "name", "password", "client")
            .expect("the player should be logged in");
        assert!(session.selected_profile().is_none());
        assert_eq!(
            session.available_profiles().map(<[GameProfile]>::len),
            Some(2)
        );
        assert_eq!(
            session.to_auth_info(),
            Err(AuthenticationError::NoSelectedCharacter)
        );
    }

    #[test]
    fn a_renewal_names_the_profile_the_player_selected() {
        let client = FakeClient::new(Server::new(&urls(), vec![login_answer("client")]));
        let service = service();
        let character = GameProfile::new(Uuid::parse(PLAYER).expect("the identifier"), "Notch");

        service
            .refresh(&client, "access", "client", Some(&character))
            .expect("the session should be renewed");

        let body: Value =
            serde_json::from_str(&client.bodies()[0]).expect("the body should be JSON");
        assert_eq!(body["accessToken"], "access");
        assert_eq!(body["clientToken"], "client");
        assert_eq!(
            body["selectedProfile"]["id"],
            "069a79f444e94726a5befca90e38aaf5"
        );
        assert_eq!(body["selectedProfile"]["name"], "Notch");
    }

    #[test]
    fn a_renewal_without_a_selection_names_no_profile() {
        let client = FakeClient::new(Server::new(&urls(), vec![login_answer("client")]));
        let service = service();

        service
            .refresh(&client, "access", "client", None)
            .expect("the session should be renewed");

        let body: Value =
            serde_json::from_str(&client.bodies()[0]).expect("the body should be JSON");
        assert!(body.get("selectedProfile").is_none());
    }

    #[test]
    fn a_renewal_that_selects_another_profile_is_refused() {
        let character = GameProfile::new(Uuid::parse(OTHER).expect("the identifier"), "Herobrine");
        let client = FakeClient::new(Server::new(&urls(), vec![login_answer("client")]));
        let service = service();

        let error = service
            .refresh(&client, "access", "client", Some(&character))
            .expect_err("the renewal should be refused");
        assert_eq!(
            error,
            AuthenticationError::malformed("Failed to select character")
        );
    }

    #[test]
    fn a_token_the_server_accepts_is_valid() {
        let client = FakeClient::new(Server::new(&urls(), vec![String::new()]));
        let service = service();

        assert!(service
            .validate(&client, "access", Some("client"))
            .expect("the token should be checked"));

        let body: Value =
            serde_json::from_str(&client.bodies()[0]).expect("the body should be JSON");
        assert_eq!(body["accessToken"], "access");
        assert_eq!(body["clientToken"], "client");
    }

    #[test]
    fn a_token_the_server_forgot_is_not_valid() {
        let client = FakeClient::new(Server::new(
            &urls(),
            vec![
                r#"{"error":"ForbiddenOperationException","errorMessage":"Invalid token."}"#
                    .to_string(),
            ],
        ));
        let service = service();

        assert!(!service
            .validate(&client, "access", None)
            .expect("the token should be checked"));
    }

    #[test]
    fn the_other_refusals_of_the_server_are_failures() {
        let client = FakeClient::new(Server::new(
            &urls(),
            vec![r#"{"error":"IllegalArgumentException","errorMessage":"bad"}"#.to_string()],
        ));
        let service = service();

        let error = service
            .validate(&client, "access", None)
            .expect_err("the refusal should be a failure");
        assert!(matches!(
            error,
            AuthenticationError::RemoteAuthentication { .. }
        ));
    }

    #[test]
    fn a_token_is_revoked() {
        let client = FakeClient::new(Server::new(&urls(), vec![String::new()]));
        let service = service();

        service
            .invalidate(&client, "access", Some("client"))
            .expect("the token should be revoked");
        assert_eq!(client.requests()[0].url(), "https://example.com/invalidate");
    }

    #[test]
    fn a_revocation_the_server_refuses_is_a_failure() {
        let client = FakeClient::new(Server::new(&urls(), vec!["not json".to_string()]));
        let service = service();

        let error = service
            .invalidate(&client, "access", None)
            .expect_err("the refusal should be a failure");
        assert!(matches!(
            error,
            AuthenticationError::ServerResponseMalformed { .. }
        ));
    }

    #[test]
    fn the_skin_is_uploaded_as_a_form_of_two_parts() {
        let client = FakeClient::new(Server::new(&urls(), vec![String::new()]));
        let service = service();

        service
            .upload_skin(
                &client,
                Uuid::parse(PLAYER).expect("the identifier"),
                "access",
                true,
                "skin.png",
                b"the-skin",
            )
            .expect("the skin should be uploaded");

        let request = &client.requests()[0];
        assert_eq!(request.method(), HttpMethod::Put);
        assert_eq!(
            request.url(),
            "https://example.com/skin/069a79f444e94726a5befca90e38aaf5"
        );
        assert_eq!(
            request
                .headers()
                .iter()
                .find(|(name, _)| name == "Authorization")
                .map(|(_, value)| value.as_str()),
            Some("Bearer access")
        );

        let body = String::from_utf8_lossy(request.body().expect("the body")).to_string();
        assert!(body.contains("name=\"model\""), "{body}");
        assert!(body.contains("\r\n\r\nslim\r\n"), "{body}");
        assert!(
            body.contains("name=\"file\"; filename=\"skin.png\""),
            "{body}"
        );
        assert!(body.contains("Content-Type: image/png"), "{body}");
        assert!(body.contains("the-skin"), "{body}");
    }

    #[test]
    fn the_skin_of_the_wide_arms_names_an_empty_model() {
        let client = FakeClient::new(Server::new(&urls(), vec![String::new()]));
        let service = service();

        service
            .upload_skin(
                &client,
                Uuid::parse(PLAYER).expect("the identifier"),
                "access",
                false,
                "skin.png",
                b"the-skin",
            )
            .expect("the skin should be uploaded");

        let body =
            String::from_utf8_lossy(client.requests()[0].body().expect("the body")).to_string();
        assert!(body.contains("\r\n\r\n\r\n"), "{body}");
        assert!(!body.contains("slim"), "{body}");
    }

    #[test]
    fn the_complete_profile_is_read_and_kept() {
        let client = FakeClient::new(Server::new(
            &urls(),
            vec![r#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch","properties":[{"name":"textures","value":"dGV4dHVyZXM="}]}"#
                .to_string()],
        ));
        let service = service();
        let uuid = Uuid::parse(PLAYER).expect("the identifier");

        let profile = service
            .complete_game_profile(&client, uuid)
            .expect("the profile should be read")
            .expect("the profile should be named");
        assert_eq!(profile.name(), "Notch");
        assert_eq!(profile.property("textures"), Some("dGV4dHVyZXM="));
        assert_eq!(
            client.requests()[0].url(),
            "https://example.com/profile/069a79f444e94726a5befca90e38aaf5"
        );

        assert_eq!(service.cached_profile(uuid), Some(profile));

        service.invalidate_profile(uuid);
        assert_eq!(service.cached_profile(uuid), None);
    }

    #[test]
    fn a_server_that_knows_no_profile_answers_null() {
        let client = FakeClient::new(Server::new(&urls(), vec!["null".to_string()]));
        let service = service();

        assert_eq!(
            service
                .complete_game_profile(&client, Uuid::parse(PLAYER).expect("the identifier"))
                .expect("the profile should be read"),
            None
        );
    }

    #[test]
    fn a_profile_without_properties_is_malformed() {
        let client = FakeClient::new(Server::new(
            &urls(),
            vec![r#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"}"#.to_string()],
        ));
        let service = service();

        let error = service
            .complete_game_profile(&client, Uuid::parse(PLAYER).expect("the identifier"))
            .expect_err("the profile should not be read");
        assert!(matches!(
            error,
            AuthenticationError::ServerResponseMalformed { .. }
        ));
    }

    #[test]
    fn the_type_of_a_skin_is_derived_from_the_name_of_the_file() {
        assert_eq!(extension("skin.png"), "png");
        assert_eq!(extension("a.b.jpeg"), "jpeg");
        assert_eq!(extension("skin"), "");
        assert_eq!(extension(".png"), "png");
    }

    #[test]
    fn a_profile_the_launcher_read_elsewhere_is_kept() {
        let service = service();
        let uuid = Uuid::parse(PLAYER).expect("the identifier");
        let profile = CompleteGameProfile::new(uuid, "Notch", BTreeMap::new());

        service.put_profile(profile.clone());
        assert_eq!(service.cached_profile(uuid), Some(profile));
    }
}
