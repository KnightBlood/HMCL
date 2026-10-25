//! The account of a player of a server that follows the protocol of
//! Yggdrasil.
//!
//! [`YggdrasilService`] sends the requests; an account is the state of one
//! player: the identifier of the record, the name the player logs in with, the
//! profile the player selected and the tokens of the last login. The account
//! decides when the login has to ask a server again, which is the part of the
//! protocol the servers do not answer: a session read from the settings is only
//! good while a server accepts its token, and the refreshment of a token that
//! selects another profile than the one the player holds is a failure rather
//! than a login.
//!
//! The account is created by a password login, through [`YggdrasilAccount::authenticate`],
//! or read from the settings of the launcher, through
//! [`YggdrasilAccount::from_storage`].
//!
//! An account of a server often holds several profiles, because the server
//! lets one player play several characters. The protocol logs such a player in
//! without selecting one of the characters, so the launcher asks the player
//! through a [`CharacterSelector`] and renews the session with the selection.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::auth::{AccountId, AuthInfo, AuthenticationError, Uuid};
use crate::util::http_request::HttpClient;
use crate::util::logging;

use super::model::{properties_object, CompleteGameProfile, GameProfile, Texture, TextureType};
use super::provider::AuthlibInjectorProvider;
use super::service::YggdrasilService;
use super::session::YggdrasilSession;

/// The name a server refuses a token it does not know with.
const FORBIDDEN_OPERATION: &str = "ForbiddenOperationException";

/// Lets the player choose one of the characters of an account.
///
/// A server may let one player play several characters; the protocol then logs
/// the player in without selecting one, and the launcher has to ask which
/// character is played. The implementation is the page of the launcher, so this
/// trait keeps the protocol free of a user interface.
pub trait CharacterSelector {
    /// Returns the character the player chose.
    ///
    /// The profiles are the characters the account holds, and the service may
    /// be asked for the skin of the characters so the page may draw them. A
    /// player who closes the page without choosing makes the login fail with
    /// [`AuthenticationError::NoSelectedCharacter`].
    fn select(
        &self,
        service: &YggdrasilService<AuthlibInjectorProvider>,
        profiles: &[GameProfile],
    ) -> Result<GameProfile, AuthenticationError>;
}

impl<F> CharacterSelector for F
where
    F: Fn(
        &YggdrasilService<AuthlibInjectorProvider>,
        &[GameProfile],
    ) -> Result<GameProfile, AuthenticationError>,
{
    fn select(
        &self,
        service: &YggdrasilService<AuthlibInjectorProvider>,
        profiles: &[GameProfile],
    ) -> Result<GameProfile, AuthenticationError> {
        self(service, profiles)
    }
}

/// The account of a player of a server that follows the protocol of Yggdrasil.
///
/// The account holds the tokens of the last login and the profile the player
/// selected; the profile does not change while the account is logged in, so a
/// server that answers another profile makes the login fail rather than
/// silently replacing the player of the record.
#[derive(Debug, Clone)]
pub struct YggdrasilAccount {
    /// The identifier of the record of the account.
    account_id: AccountId,
    /// The server the account logs in to.
    service: Arc<YggdrasilService<AuthlibInjectorProvider>>,
    /// The name the player logs in with.
    login_name: String,
    /// The identifier of the profile the player selected.
    profile_id: Uuid,
    /// The tokens of the last login.
    session: YggdrasilSession,
}

impl YggdrasilAccount {
    /// Creates the account of a session the launcher already has.
    ///
    /// The session must name the profile the player selected, because the
    /// identifier of the profile is what the account holds and what the game is
    /// told; a session without a profile is a session the launcher cannot use.
    pub fn new(
        account_id: AccountId,
        service: Arc<YggdrasilService<AuthlibInjectorProvider>>,
        login_name: impl Into<String>,
        session: YggdrasilSession,
    ) -> Result<Self, AuthenticationError> {
        let profile_id = session
            .selected_profile()
            .map(GameProfile::id)
            .ok_or(AuthenticationError::NoSelectedCharacter)?;

        Ok(Self {
            account_id,
            service,
            login_name: login_name.into(),
            profile_id,
            session,
        })
    }

    /// Logs a player in with a name and a password and creates the account of
    /// the player.
    ///
    /// A server that holds one character selects it and the login ends there; a
    /// server that holds several selects none, so the player chooses one
    /// through the selector and the session is renewed with the choice. An
    /// account without a character cannot be played, which is a failure of the
    /// login rather than a character the player may choose.
    pub fn authenticate(
        account_id: AccountId,
        service: Arc<YggdrasilService<AuthlibInjectorProvider>>,
        client: &dyn HttpClient,
        login_name: impl Into<String>,
        password: &str,
        selector: &dyn CharacterSelector,
    ) -> Result<Self, AuthenticationError> {
        let login_name = login_name.into();
        let client_token = random_client_token();
        let acquired = service.authenticate(client, &login_name, password, &client_token)?;

        // A server that selected a character answered the whole session, which
        // is the common case of a player who plays one character. A server that
        // selected none answered the characters the player may choose, so the
        // player chooses one and the session is renewed with the choice.
        let session = match acquired.selected_profile() {
            Some(_) => acquired,
            None => select_character(&service, client, acquired, selector)?,
        };

        Self::new(account_id, service, login_name, session)
    }

    /// Reads the account of a player from the settings of the launcher.
    ///
    /// The public data names the name the player logs in with and the profile,
    /// and the protected data holds the tokens. A record without either cannot
    /// be logged in, which the launcher reports rather than dropping the record
    /// the player may still see.
    pub fn from_storage(
        account_id: AccountId,
        service: Arc<YggdrasilService<AuthlibInjectorProvider>>,
        metadata: &Value,
        private_data: &Map<String, Value>,
    ) -> Result<Self, AuthenticationError> {
        let login_name = metadata
            .get("loginName")
            .and_then(Value::as_str)
            .ok_or_else(|| AuthenticationError::invalid_stored("loginName is missing"))?
            .to_string();

        let session = YggdrasilSession::from_storage(metadata, private_data)?;
        Self::new(account_id, service, login_name, session)
    }

    /// Returns the identifier of the record of the account.
    pub const fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the server the account logs in to.
    pub fn service(&self) -> &YggdrasilService<AuthlibInjectorProvider> {
        &self.service
    }

    /// Returns the name the player logs in with.
    pub fn login_name(&self) -> &str {
        &self.login_name
    }

    /// Returns the identifier of the profile the player selected.
    pub const fn profile_id(&self) -> Uuid {
        self.profile_id
    }

    /// Returns the name of the profile the player selected.
    ///
    /// The name is empty when the session was read from a launcher that did not
    /// know it, which is the case a login renews the session for.
    pub fn profile_name(&self) -> &str {
        self.session
            .selected_profile()
            .map(GameProfile::name)
            .unwrap_or_default()
    }

    /// Returns the tokens of the last login.
    pub fn session(&self) -> &YggdrasilSession {
        &self.session
    }

    /// Returns the information the game is started with, after checking the
    /// token of the account.
    ///
    /// The launcher does not trust the token it read from the settings until a
    /// server accepted it, so the token is checked first and renewed when the
    /// server refuses it, when it has no name of the player, or when the
    /// launcher did not log in during this run. A server that refuses the
    /// renewal with [`FORBIDDEN_OPERATION`] means the token of the account is
    /// not accepted any more, which the player solves by logging in again; a
    /// server that answers another profile, or a profile without a name, is a
    /// failure of the server rather than a login.
    pub fn log_in(&mut self, client: &dyn HttpClient) -> Result<AuthInfo, AuthenticationError> {
        let valid = self.session.has_profile_name()
            && self.service.validate(
                client,
                self.session.access_token(),
                Some(self.session.client_token()),
            )?;

        if !valid {
            let acquired = match self.service.refresh(
                client,
                self.session.access_token(),
                self.session.client_token(),
                None,
            ) {
                Err(AuthenticationError::RemoteAuthentication { name, .. })
                    if name == FORBIDDEN_OPERATION =>
                {
                    return Err(AuthenticationError::CredentialExpired {
                        message: Some(name),
                    })
                }
                other => other?,
            };

            if acquired.selected_profile().map(GameProfile::id) != Some(self.profile_id) {
                return Err(AuthenticationError::malformed("Selected profile changed"));
            }
            if !acquired.has_profile_name() {
                return Err(AuthenticationError::malformed("Profile name is missing"));
            }

            self.session = acquired;
        }

        self.session.to_auth_info()
    }

    /// Logs the player in with the password of the account.
    ///
    /// The password is asked for when the credentials of the account expired,
    /// because the server then refuses the token of the account but accepts the
    /// password. The character the account holds is selected, and a server that
    /// no longer knows it is a failure: the record names a player the account
    /// does not hold any more.
    pub fn log_in_with_password(
        &mut self,
        client: &dyn HttpClient,
        password: &str,
    ) -> Result<AuthInfo, AuthenticationError> {
        let client_token = random_client_token();
        let acquired =
            self.service
                .authenticate(client, &self.login_name, password, &client_token)?;

        let session = match acquired.selected_profile() {
            Some(selected) => {
                if selected.id() != self.profile_id {
                    return Err(AuthenticationError::CharacterDeleted);
                }
                acquired
            }
            None => {
                let profiles = acquired.available_profiles().unwrap_or_default();
                let character = profiles
                    .iter()
                    .find(|profile| profile.id() == self.profile_id)
                    .ok_or(AuthenticationError::CharacterDeleted)?;

                self.service.refresh(
                    client,
                    acquired.access_token(),
                    acquired.client_token(),
                    Some(character),
                )?
            }
        };

        self.session = session;
        self.session.to_auth_info()
    }

    /// Returns the information of the player without asking a server.
    ///
    /// The game accepts the token the launcher stored, so a player who plays
    /// alone does not wait for the server. A session without a name of the
    /// player cannot start the game, which is the one case the player has to log
    /// in again.
    pub fn play_offline(&self) -> Result<AuthInfo, AuthenticationError> {
        if !self.session.has_profile_name() {
            return Err(AuthenticationError::CredentialExpired {
                message: Some("Profile name is missing".to_string()),
            });
        }

        self.session.to_auth_info()
    }

    /// Writes the public data of the account into the settings of the launcher.
    ///
    /// The name the player logs in with is written so the launcher may ask for
    /// the password again, and the profile is written with the hyphens, which
    /// is the form the launcher of Java writes.
    pub fn write_metadata(&self, metadata: &mut Map<String, Value>) {
        metadata.insert(
            "loginName".to_string(),
            Value::String(self.login_name.clone()),
        );
        metadata.insert(
            "profileID".to_string(),
            Value::String(self.profile_id.to_string()),
        );
    }

    /// Writes the credentials of the account into the protected data.
    ///
    /// The tokens are written by the session, and the properties of the profile
    /// are written next to them when the launcher read them, so the skin of the
    /// player is drawn before the server is asked again.
    pub fn write_private_data(&self, private_data: &mut Map<String, Value>) {
        self.session.write_private_data(private_data);

        if let Some(profile) = self.service.cached_profile(self.profile_id) {
            private_data.insert(
                "profileProperties".to_string(),
                properties_object(profile.properties()),
            );
        }
    }

    /// Forgets the properties of the profile of the player.
    ///
    /// The launcher calls this when the server answered that the profile
    /// changed, such as when the player uploaded another skin.
    pub fn clear_cache(&self) {
        self.service.invalidate_profile(self.profile_id);
    }

    /// Returns the textures of the profile of the player, if the launcher read
    /// them.
    ///
    /// A server that answered a payload the launcher cannot read gives the
    /// player no textures rather than a failure of the page: the skin is drawn
    /// with the default one, which is what the launcher of Java does.
    pub fn textures(&self) -> Option<BTreeMap<TextureType, Texture>> {
        let profile = self.service.cached_profile(self.profile_id)?;

        match profile.textures() {
            Ok(textures) => textures,
            Err(error) => {
                logging::warn(&format!("Failed to parse texture payload: {error}"));
                None
            }
        }
    }

    /// Returns whether the server accepts another skin of the player.
    ///
    /// Every server of the protocol of Yggdrasil accepts an upload, even when
    /// the server then refuses it.
    pub const fn can_upload_skin(&self) -> bool {
        true
    }

    /// Uploads the file of the skin of the player.
    ///
    /// The file is read by the caller, because the core does not open files: a
    /// page of the launcher reads the file the player chose and hands the bytes
    /// over. The name of the file is told to the server, which is why it is not
    /// thrown away.
    pub fn upload_skin(
        &self,
        client: &dyn HttpClient,
        slim: bool,
        file_name: &str,
        data: &[u8],
    ) -> Result<(), AuthenticationError> {
        self.service.upload_skin(
            client,
            self.profile_id,
            self.session.access_token(),
            slim,
            file_name,
            data,
        )
    }
}

/// Returns the characters of the account the server may upload a texture for.
///
/// A server names the textures it accepts in the property `uploadableTextures`,
/// as a list of names separated by commas. A server that names no property
/// accepts none, because the launcher of Java reads the absence as a server that
/// does not accept an upload.
pub fn uploadable_textures(profile: &CompleteGameProfile) -> BTreeSet<TextureType> {
    let Some(names) = profile.property("uploadableTextures") else {
        return BTreeSet::new();
    };

    names
        .split(',')
        .filter_map(|name| TextureType::of(&name.trim().to_uppercase()))
        .collect()
}

/// Renews the session of a login with the character the player chose.
///
/// The character is handed to the server, which selects it; the session of the
/// answer is the session of the account, so a server that selects another
/// character is refused by [`YggdrasilService::refresh`].
fn select_character(
    service: &YggdrasilService<AuthlibInjectorProvider>,
    client: &dyn HttpClient,
    acquired: YggdrasilSession,
    selector: &dyn CharacterSelector,
) -> Result<YggdrasilSession, AuthenticationError> {
    let profiles = acquired
        .available_profiles()
        .filter(|profiles| !profiles.is_empty())
        .ok_or(AuthenticationError::NoCharacter)?;

    let character = selector.select(service, profiles)?;

    service.refresh(
        client,
        acquired.access_token(),
        acquired.client_token(),
        Some(&character),
    )
}

/// Returns the token that names the launcher to a server.
///
/// The token is written without the hyphens, which is the form the launcher of
/// Java writes: it is a random identifier, and the server keeps it to tell the
/// launcher apart from the other programs that log the account in.
fn random_client_token() -> String {
    crate::util::digest::to_hex(&crate::util::random::bytes::<16>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::http_request::{HttpRequest, HttpResponse};
    use std::cell::RefCell;

    /// The identifier of the player of the tests.
    const PLAYER: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";

    /// The root of the API of the server of the tests.
    const API_ROOT: &str = "https://example.com/api/yggdrasil/";

    /// A client that answers with the answers it is given and keeps the
    /// requests it was asked.
    struct FakeClient {
        /// The bodies of the answers, in the order they are returned.
        answers: RefCell<Vec<String>>,
        /// The requests the client was asked, in the order they were asked.
        asked: RefCell<Vec<HttpRequest>>,
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
            self.asked
                .borrow()
                .iter()
                .map(|request| request.url().to_string())
                .collect()
        }
    }

    impl HttpClient for FakeClient {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
            self.asked.borrow_mut().push(request.clone());
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

    /// Returns the answer of the server of the tests for the request it answers.
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

    /// The service of the tests, which asks the server of the tests.
    fn service() -> Arc<YggdrasilService<AuthlibInjectorProvider>> {
        Arc::new(YggdrasilService::new(AuthlibInjectorProvider::new(
            API_ROOT,
        )))
    }

    /// The answer of a login that selects the player.
    fn login_answer() -> String {
        r#"{"accessToken":"access","clientToken":"client","selectedProfile":{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"},"availableProfiles":[{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"}],"user":{"id":"user"}}"#.to_string()
    }

    /// The answer of a login that selects no player of two.
    fn select_answer() -> String {
        r#"{"accessToken":"access","clientToken":"client","availableProfiles":[{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"},{"id":"b50ad385829d3141a2167e7d7539ba7f","name":"Herobrine"}],"user":{"id":"user"}}"#.to_string()
    }

    /// The answer of a renewal that selects the player.
    fn refresh_answer() -> String {
        r#"{"accessToken":"renewed","clientToken":"client","selectedProfile":{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"},"availableProfiles":[{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch"}],"user":{"id":"user"}}"#.to_string()
    }

    /// The refusal of a token the server does not know.
    fn forbidden_answer() -> String {
        r#"{"error":"ForbiddenOperationException","errorMessage":"Invalid token."}"#.to_string()
    }

    /// The account of the tests, whose session names the player.
    fn account() -> YggdrasilAccount {
        let session = YggdrasilSession::new(
            "client",
            "access",
            Some(GameProfile::new(
                Uuid::parse(PLAYER).expect("the identifier"),
                "Notch",
            )),
            None,
            None,
        );

        YggdrasilAccount::new(
            AccountId::generate(),
            service(),
            "notch@example.com",
            session,
        )
        .expect("the account should be created")
    }

    /// The selector of the tests, which picks the first character.
    fn first_character(
        _service: &YggdrasilService<AuthlibInjectorProvider>,
        profiles: &[GameProfile],
    ) -> Result<GameProfile, AuthenticationError> {
        profiles
            .first()
            .cloned()
            .ok_or(AuthenticationError::NoSelectedCharacter)
    }

    #[test]
    fn a_password_login_creates_the_account_of_the_selected_character() {
        let client = FakeClient::new(vec![login_answer()]);
        let account = YggdrasilAccount::authenticate(
            AccountId::generate(),
            service(),
            &client,
            "notch@example.com",
            "the-password",
            &first_character,
        )
        .expect("the player should be logged in");

        assert_eq!(
            account.profile_id(),
            Uuid::parse(PLAYER).expect("the identifier")
        );
        assert_eq!(account.profile_name(), "Notch");
        assert_eq!(account.login_name(), "notch@example.com");
        assert_eq!(
            client.urls(),
            vec![format!("{API_ROOT}authserver/authenticate")]
        );
    }

    #[test]
    fn a_password_login_of_several_characters_asks_the_player() {
        let client = FakeClient::new(vec![select_answer(), refresh_answer()]);
        let account = YggdrasilAccount::authenticate(
            AccountId::generate(),
            service(),
            &client,
            "notch@example.com",
            "the-password",
            &first_character,
        )
        .expect("the player should be logged in");

        assert_eq!(
            client.urls(),
            vec![
                format!("{API_ROOT}authserver/authenticate"),
                format!("{API_ROOT}authserver/refresh"),
            ]
        );
        assert_eq!(account.session().access_token(), "renewed");
        assert_eq!(account.profile_name(), "Notch");
    }

    #[test]
    fn a_password_login_without_a_character_cannot_be_played() {
        let client = FakeClient::new(vec![
            r#"{"accessToken":"access","clientToken":"client","availableProfiles":[],"user":{"id":"user"}}"#
                .to_string(),
        ]);

        let error = YggdrasilAccount::authenticate(
            AccountId::generate(),
            service(),
            &client,
            "notch@example.com",
            "the-password",
            &first_character,
        )
        .expect_err("the login should fail");
        assert_eq!(error, AuthenticationError::NoCharacter);
    }

    #[test]
    fn a_player_who_chooses_no_character_makes_the_login_fail() {
        let client = FakeClient::new(vec![select_answer()]);
        let refuse = |_: &YggdrasilService<AuthlibInjectorProvider>,
                      _: &[GameProfile]|
         -> Result<GameProfile, AuthenticationError> {
            Err(AuthenticationError::NoSelectedCharacter)
        };

        let error = YggdrasilAccount::authenticate(
            AccountId::generate(),
            service(),
            &client,
            "notch@example.com",
            "the-password",
            &refuse,
        )
        .expect_err("the login should fail");
        assert_eq!(error, AuthenticationError::NoSelectedCharacter);
        // The session was not renewed, because no character was chosen.
        assert_eq!(client.urls().len(), 1);
    }

    #[test]
    fn a_session_read_from_the_settings_is_written_back() {
        let account = account();
        let mut metadata = Map::new();
        let mut private_data = Map::new();
        account.write_metadata(&mut metadata);
        account.write_private_data(&mut private_data);

        assert_eq!(
            metadata.get("loginName").and_then(Value::as_str),
            Some("notch@example.com")
        );
        assert_eq!(
            metadata.get("profileID").and_then(Value::as_str),
            Some(PLAYER)
        );
        assert_eq!(
            private_data.get("clientToken").and_then(Value::as_str),
            Some("client")
        );

        let read = YggdrasilAccount::from_storage(
            account.account_id(),
            service(),
            &Value::Object(metadata),
            &private_data,
        )
        .expect("the account should be read");
        assert_eq!(read.profile_id(), account.profile_id());
        assert_eq!(read.login_name(), "notch@example.com");
        assert_eq!(read.session().access_token(), "access");
    }

    #[test]
    fn a_record_without_a_login_name_cannot_be_read() {
        let error = YggdrasilAccount::from_storage(
            AccountId::generate(),
            service(),
            &Value::Object(Map::new()),
            &Map::new(),
        )
        .expect_err("the account should not be read");
        assert_eq!(
            error,
            AuthenticationError::invalid_stored("loginName is missing")
        );
    }

    #[test]
    fn a_session_without_a_profile_cannot_be_the_account() {
        let session = YggdrasilSession::new("client", "access", None, None, None);
        let error = YggdrasilAccount::new(AccountId::generate(), service(), "name", session)
            .expect_err("the account should not be created");
        assert_eq!(error, AuthenticationError::NoSelectedCharacter);
    }

    #[test]
    fn a_login_that_found_a_valid_token_asks_nothing_else() {
        let mut account = account();
        let client = FakeClient::new(vec![String::new()]);

        let info = account
            .log_in(&client)
            .expect("the player should be logged in");
        assert_eq!(info.username, "Notch");
        assert_eq!(
            client.urls(),
            vec![format!("{API_ROOT}authserver/validate")]
        );
        assert_eq!(account.session().access_token(), "access");
    }

    #[test]
    fn a_login_that_found_no_valid_token_renews_it() {
        let mut account = account();
        let client = FakeClient::new(vec![forbidden_answer(), refresh_answer()]);

        let info = account
            .log_in(&client)
            .expect("the player should be logged in");
        assert_eq!(info.access_token, "renewed");
        assert_eq!(
            client.urls(),
            vec![
                format!("{API_ROOT}authserver/validate"),
                format!("{API_ROOT}authserver/refresh"),
            ]
        );
    }

    #[test]
    fn a_login_whose_renewal_is_refused_needs_a_new_login() {
        let mut account = account();
        let client = FakeClient::new(vec![forbidden_answer(), forbidden_answer()]);

        let error = account.log_in(&client).expect_err("the login should fail");
        assert_eq!(
            error,
            AuthenticationError::CredentialExpired {
                message: Some(FORBIDDEN_OPERATION.to_string()),
            }
        );
        assert!(error.requires_login());
    }

    #[test]
    fn a_renewal_that_names_another_profile_is_refused() {
        let mut account = account();
        let client = FakeClient::new(vec![
            forbidden_answer(),
            r#"{"accessToken":"renewed","clientToken":"client","selectedProfile":{"id":"b50ad385829d3141a2167e7d7539ba7f","name":"Herobrine"},"availableProfiles":[],"user":{"id":"user"}}"#.to_string(),
        ]);

        let error = account.log_in(&client).expect_err("the login should fail");
        assert_eq!(
            error,
            AuthenticationError::malformed("Selected profile changed")
        );
    }

    #[test]
    fn a_renewal_without_a_name_of_the_player_is_refused() {
        let mut account = account();
        let client = FakeClient::new(vec![
            forbidden_answer(),
            r#"{"accessToken":"renewed","clientToken":"client","selectedProfile":{"id":"069a79f444e94726a5befca90e38aaf5","name":""},"availableProfiles":[],"user":{"id":"user"}}"#.to_string(),
        ]);

        let error = account.log_in(&client).expect_err("the login should fail");
        assert_eq!(
            error,
            AuthenticationError::malformed("Profile name is missing")
        );
    }

    #[test]
    fn a_login_with_a_password_names_the_character_of_the_account() {
        let mut account = account();
        let client = FakeClient::new(vec![select_answer(), refresh_answer()]);

        let info = account
            .log_in_with_password(&client, "the-password")
            .expect("the player should be logged in");
        assert_eq!(info.access_token, "renewed");

        let body: Value = serde_json::from_slice(
            client
                .asked
                .borrow()
                .last()
                .and_then(|request| request.body())
                .expect("the renewal should have a body"),
        )
        .expect("the body should be JSON");
        assert_eq!(
            body["selectedProfile"]["id"],
            "069a79f444e94726a5befca90e38aaf5"
        );
    }

    #[test]
    fn a_password_login_that_names_another_character_is_refused() {
        let mut account = account();
        let client = FakeClient::new(vec![
            r#"{"accessToken":"access","clientToken":"client","selectedProfile":{"id":"b50ad385829d3141a2167e7d7539ba7f","name":"Herobrine"},"availableProfiles":[],"user":{"id":"user"}}"#.to_string(),
        ]);

        let error = account
            .log_in_with_password(&client, "the-password")
            .expect_err("the login should fail");
        assert_eq!(error, AuthenticationError::CharacterDeleted);
    }

    #[test]
    fn a_password_login_that_lost_the_character_is_refused() {
        let mut account = account();
        let client = FakeClient::new(vec![
            r#"{"accessToken":"access","clientToken":"client","availableProfiles":[],"user":{"id":"user"}}"#
                .to_string(),
        ]);

        let error = account
            .log_in_with_password(&client, "the-password")
            .expect_err("the login should fail");
        assert_eq!(error, AuthenticationError::CharacterDeleted);
    }

    #[test]
    fn a_player_without_a_name_cannot_play_offline() {
        let session = YggdrasilSession::new(
            "client",
            "access",
            Some(GameProfile::new(
                Uuid::parse(PLAYER).expect("the identifier"),
                "",
            )),
            None,
            None,
        );
        let account = YggdrasilAccount::new(AccountId::generate(), service(), "name", session)
            .expect("the account should be created");

        assert_eq!(
            account.play_offline(),
            Err(AuthenticationError::CredentialExpired {
                message: Some("Profile name is missing".to_string()),
            })
        );
    }

    #[test]
    fn the_player_plays_offline_with_the_stored_token() {
        let account = account();
        let info = account.play_offline().expect("the player should play");
        assert_eq!(info.access_token, "access");
        assert_eq!(info.uuid, Uuid::parse(PLAYER).expect("the identifier"));
    }

    #[test]
    fn the_properties_of_the_profile_are_written_with_the_credentials() {
        let account = account();
        account.service().put_profile(CompleteGameProfile::new(
            account.profile_id(),
            "Notch",
            BTreeMap::from([("textures".to_string(), "dGV4dHVyZXM=".to_string())]),
        ));

        let mut private_data = Map::new();
        account.write_private_data(&mut private_data);
        assert_eq!(
            private_data.get("profileProperties"),
            Some(&serde_json::json!({ "textures": "dGV4dHVyZXM=" }))
        );

        account.clear_cache();
        let mut private_data = Map::new();
        account.write_private_data(&mut private_data);
        assert!(private_data.get("profileProperties").is_none());
    }

    #[test]
    fn a_payload_the_launcher_cannot_read_gives_no_textures() {
        let account = account();
        account.service().put_profile(CompleteGameProfile::new(
            account.profile_id(),
            "Notch",
            BTreeMap::from([("textures".to_string(), "not base64!".to_string())]),
        ));

        assert!(account.textures().is_none());
    }

    #[test]
    fn the_skin_is_uploaded_with_the_token_of_the_account() {
        let account = account();
        let client = FakeClient::new(vec![String::new()]);

        account
            .upload_skin(&client, true, "skin.png", b"the-skin")
            .expect("the skin should be uploaded");

        assert!(account.can_upload_skin());
        assert_eq!(
            client.urls(),
            vec![format!(
                "{API_ROOT}api/user/profile/069a79f444e94726a5befca90e38aaf5/skin"
            )]
        );
    }

    #[test]
    fn the_uploadable_textures_are_read_from_the_properties() {
        let profile = CompleteGameProfile::new(
            Uuid::parse(PLAYER).expect("the identifier"),
            "Notch",
            BTreeMap::from([(
                "uploadableTextures".to_string(),
                "skin,CAPE,elytra".to_string(),
            )]),
        );
        let textures = uploadable_textures(&profile);
        assert_eq!(
            textures.into_iter().collect::<Vec<_>>(),
            vec![TextureType::Skin, TextureType::Cape]
        );

        let profile = CompleteGameProfile::new(
            Uuid::parse(PLAYER).expect("the identifier"),
            "Notch",
            BTreeMap::new(),
        );
        assert!(uploadable_textures(&profile).is_empty());
    }
}
