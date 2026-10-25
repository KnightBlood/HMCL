//! The OAuth login of Microsoft.
//!
//! A player logs in with a Microsoft account through the OAuth server of
//! Microsoft. The launcher shows the player the page of the authorization, the
//! player logs in there, and the server hands the launcher a code, which the
//! launcher exchanges for a token of the account. The launcher also offers the
//! way without a page of its own: the server names a code and an address, the
//! player opens the address on any device and writes the code, and the launcher
//! asks the server for the token again and again until the player is done.
//!
//! This module holds the addresses of the server, the requests of the two ways
//! and the reading of the refusals of the server. The page itself, the server
//! that receives the code and the opening of the browser belong to the program
//! that runs the launcher, which names them through [`OAuthCallback`].
//!
//! The core of the launcher opens no connection: the requests are sent by the
//! [`HttpClient`] the program gives, so the login can be tested against the
//! answers of a server that is not there.

use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::util::base64;
use crate::util::digest::Sha256;
use crate::util::http_request::{send, with_query, HttpClient, HttpRequest};

use super::error::AuthenticationError;

/// The address of the page where the player logs in to Microsoft.
const MICROSOFT_AUTHORIZATION_URL: &str = "https://login.live.com/oauth20_authorize.srf";
/// The address where the code of the page is exchanged for a token.
const MICROSOFT_ACCESS_TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
/// The address where the device flow asks for the code the player writes.
const MICROSOFT_DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
/// The address the device flow asks for the token until the player is done.
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

/// The number of the attempts a request of the login makes.
///
/// The launcher of Java asks the server of the accounts five times, because a
/// login that fails for a moment would make the player log in again.
const ATTEMPTS: u32 = 5;

/// The longest time the device flow waits for the player, in seconds.
///
/// The server names the life of the code, which may be longer than the launcher
/// waits; the launcher of Java gives up after fifteen minutes so the player is
/// not left in front of a page that waits forever.
const MAX_DEVICE_WAIT_SECONDS: u64 = 900;

/// The kind of the login the player chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrantFlow {
    /// The player logs in on the page of Microsoft, which answers the launcher
    /// with a code through a server of the launcher.
    AuthorizationCode,
    /// The player writes a code the server named on the page of Microsoft.
    Device,
}

impl GrantFlow {
    /// Returns the name of the kind, which is the name the launcher of Java
    /// writes for the same kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthorizationCode => "AUTHORIZATION_CODE",
            Self::Device => "DEVICE",
        }
    }
}

/// The tokens the account proves itself with.
///
/// The access token is short lived and is renewed through the refresh token;
/// the refresh token is the long lived secret of the account and is stored in
/// the protected private data of the launcher.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OAuthTokens {
    /// The token the account is used with.
    pub access_token: String,
    /// The token that renews the access token.
    pub refresh_token: String,
}

/// The server of the OAuth login.
///
/// The addresses are held by the value because the launcher of Java names them
/// once for Microsoft, and a server of another kind is a value with other
/// addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuth {
    /// The address of the page where the player logs in.
    authorization_url: &'static str,
    /// The address where the code of the page is exchanged for a token.
    access_token_url: &'static str,
    /// The address where the device flow asks for the code.
    device_code_url: &'static str,
    /// The address the device flow asks for the token.
    token_url: &'static str,
}

impl OAuth {
    /// Creates the server of a login.
    pub const fn new(
        authorization_url: &'static str,
        access_token_url: &'static str,
        device_code_url: &'static str,
        token_url: &'static str,
    ) -> Self {
        Self {
            authorization_url,
            access_token_url,
            device_code_url,
            token_url,
        }
    }

    /// Returns the addresses of the server of Microsoft.
    pub const fn microsoft() -> Self {
        Self::new(
            MICROSOFT_AUTHORIZATION_URL,
            MICROSOFT_ACCESS_TOKEN_URL,
            MICROSOFT_DEVICE_CODE_URL,
            MICROSOFT_TOKEN_URL,
        )
    }

    /// Returns the address of the page where the player logs in.
    pub const fn authorization_url(&self) -> &'static str {
        self.authorization_url
    }

    /// Returns the address where the code of the page is exchanged for a token.
    pub const fn access_token_url(&self) -> &'static str {
        self.access_token_url
    }

    /// Returns the address where the device flow asks for the code.
    pub const fn device_code_url(&self) -> &'static str {
        self.device_code_url
    }

    /// Returns the address the device flow asks for the token.
    pub const fn token_url(&self) -> &'static str {
        self.token_url
    }

    /// Logs the player in through the way the player chose.
    ///
    /// The callback of the options opens the page of Microsoft and, for the
    /// login with a page, starts the server that receives the code of the
    /// server. The function returns the tokens of the account, which the
    /// servers of Xbox and of the game are asked with afterwards.
    pub fn authenticate(
        &self,
        client: &dyn HttpClient,
        flow: GrantFlow,
        options: &Options<'_>,
    ) -> Result<OAuthTokens, AuthenticationError> {
        match flow {
            GrantFlow::AuthorizationCode => self.authenticate_authorization_code(client, options),
            GrantFlow::Device => {
                self.authenticate_device(client, options, &mut SystemClock::default())
            }
        }
    }

    /// Renews the tokens of an account with the refresh token of the account.
    ///
    /// The refresh token is the long lived secret of an account; the server
    /// answers with a new access token and, usually, with a new refresh token,
    /// which replaces the old one.
    pub fn refresh(
        &self,
        client: &dyn HttpClient,
        refresh_token: &str,
        options: &Options<'_>,
    ) -> Result<OAuthTokens, AuthenticationError> {
        let client_id = options.callback.client_id();
        let request = HttpRequest::post(self.token_url)
            .form(&[
                ("client_id", Some(client_id.as_str())),
                ("refresh_token", Some(refresh_token)),
                ("grant_type", Some("refresh_token")),
            ])
            .accept("application/json")
            .ignore_status()
            .attempts(ATTEMPTS);

        let response: RefreshResponse = read_json(client, &request)?;
        check_error(&response.refusal)?;
        response.into_tokens(refresh_token)
    }

    /// Logs the player in with the page of Microsoft.
    ///
    /// The launcher opens the page with the state and the challenge of the
    /// proof key it created, the server answers the local server of the
    /// launcher with a code, and the code is exchanged for the tokens together
    /// with the verifier of the challenge.
    fn authenticate_authorization_code(
        &self,
        client: &dyn HttpClient,
        options: &Options<'_>,
    ) -> Result<OAuthTokens, AuthenticationError> {
        let session = options.callback.start_session()?;
        let code_verifier = session.code_verifier();
        let state = session.state();
        let code_challenge = generate_code_challenge(&code_verifier);
        let redirect_uri = session.redirect_uri();
        let client_id = options.callback.client_id();

        let url = with_query(
            self.authorization_url,
            &[
                ("client_id", Some(client_id.as_str())),
                ("response_type", Some("code")),
                ("redirect_uri", Some(redirect_uri.as_str())),
                ("scope", Some(options.scope)),
                ("prompt", Some("select_account")),
                ("code_challenge", Some(code_challenge.as_str())),
                ("state", Some(state.as_str())),
                ("code_challenge_method", Some("S256")),
            ],
        );

        options
            .callback
            .open_browser(GrantFlow::AuthorizationCode, &url)
            .map_err(AuthenticationError::disconnect)?;
        let code = session.wait_for_code()?;

        let request = HttpRequest::post(self.access_token_url)
            .form(&[
                ("client_id", Some(client_id.as_str())),
                ("code", Some(code.as_str())),
                ("grant_type", Some("authorization_code")),
                ("code_verifier", Some(code_verifier.as_str())),
                ("redirect_uri", Some(redirect_uri.as_str())),
                ("scope", Some(options.scope)),
            ])
            .ignore_status()
            .attempts(ATTEMPTS);

        let response: AuthorizationResponse = read_json(client, &request)?;
        check_error(&response.refusal)?;
        response.into_tokens()
    }

    /// Logs the player in with a code the player writes on another device.
    ///
    /// The server names a code and an address; the launcher shows both and asks
    /// the server for the token again and again until the player is done. The
    /// wait between two questions grows when the server asks for it, and the
    /// login ends when the code expires or the wait is longer than
    /// [`MAX_DEVICE_WAIT_SECONDS`].
    fn authenticate_device(
        &self,
        client: &dyn HttpClient,
        options: &Options<'_>,
        clock: &mut dyn Clock,
    ) -> Result<OAuthTokens, AuthenticationError> {
        let client_id = options.callback.client_id();

        let request = HttpRequest::post(self.device_code_url)
            .form(&[
                ("client_id", Some(client_id.as_str())),
                ("scope", Some(options.scope)),
            ])
            .ignore_status()
            .attempts(ATTEMPTS);

        let device: DeviceCodeResponse = read_json(client, &request)?;
        check_error(&device.refusal)?;

        options
            .callback
            .grant_device_code(&device.user_code, &device.verification_uri);
        options
            .callback
            .open_browser(GrantFlow::Device, &device.verification_uri)
            .map_err(AuthenticationError::disconnect)?;

        let waited_seconds = u64::try_from(device.expires_in)
            .unwrap_or(0)
            .min(MAX_DEVICE_WAIT_SECONDS);
        let mut interval = Duration::from_secs(u64::try_from(device.interval).unwrap_or(0));

        loop {
            // The server refuses two questions in the same millisecond, so the
            // wait is at least one millisecond even when the server names none.
            clock.sleep(interval.max(Duration::from_millis(1)));

            if clock.elapsed().as_secs() >= waited_seconds {
                return Err(AuthenticationError::NoSelectedCharacter);
            }

            let request = HttpRequest::post(self.token_url)
                .form(&[
                    (
                        "grant_type",
                        Some("urn:ietf:params:oauth:grant-type:device_code"),
                    ),
                    ("code", Some(device.device_code.as_str())),
                    ("client_id", Some(client_id.as_str())),
                ])
                .ignore_status()
                .attempts(ATTEMPTS);

            let token: TokenResponse = read_json(client, &request)?;

            match token.reason() {
                // The player is not done yet, so the launcher asks again.
                Some("authorization_pending") => continue,
                Some("expired_token") => return Err(AuthenticationError::NoSelectedCharacter),
                // The launcher asked too often, so it waits longer.
                Some("slow_down") => {
                    interval += Duration::from_secs(5);
                    continue;
                }
                _ => {}
            }

            // The launcher of Java returns the tokens without reading a refusal
            // here, which hands an account without a token to the rest of the
            // login; the reason of the server is reported instead.
            check_error(&token.refusal)?;

            options.callback.login_completed_device_code();
            return token.into_tokens();
        }
    }
}

/// The scope of the login the launcher asks Microsoft for.
///
/// The scope asks for the sign in of Xbox Live, which the servers of the game
/// are asked with, and for the refresh token, without which the account would
/// have to be logged in again every hour.
pub const MICROSOFT_SCOPE: &str = "XboxLive.signin offline_access";

/// The page of the login of a player and the ways the launcher shows it.
///
/// The program that runs the launcher implements this: it opens the browser,
/// holds the local server that receives the code, and shows the code of the
/// device flow.
pub trait OAuthCallback {
    /// Starts the local server that receives the code of the page.
    ///
    /// The server picks a port, creates the state and the verifier of the proof
    /// key, and waits for the answer of the page until the session is dropped.
    fn start_session(&self) -> Result<Box<dyn OAuthSession>, AuthenticationError>;

    /// Shows the player the code and the address of the device flow.
    fn grant_device_code(&self, user_code: &str, verification_uri: &str);

    /// Tells the player the login of the device flow is done.
    fn login_completed_device_code(&self);

    /// Opens the page of Microsoft in the browser of the player.
    ///
    /// The failure keeps its message, which is reported the way the launcher of
    /// Java reports the failure of the browser.
    fn open_browser(&self, flow: GrantFlow, url: &str) -> Result<(), String>;

    /// Returns the identifier of the application of the launcher.
    ///
    /// A build without an identifier cannot log a player in, which is why the
    /// answers of the servers are refused with
    /// [`AuthenticationError::MicrosoftNotSupported`].
    fn client_id(&self) -> String;
}

/// The server of the launcher that receives the code of the page.
pub trait OAuthSession {
    /// Returns the state the launcher sent with the page and expects back.
    fn state(&self) -> String;

    /// Returns the verifier of the proof key the challenge of the page was
    /// made of.
    fn code_verifier(&self) -> String;

    /// Returns the address the page answers with the code.
    fn redirect_uri(&self) -> String;

    /// Waits until the page answers with the code.
    ///
    /// A player who closes the page without logging in ends the wait with
    /// [`AuthenticationError::NoSelectedCharacter`].
    fn wait_for_code(&self) -> Result<String, AuthenticationError>;
}

/// The settings of a login.
pub struct Options<'a> {
    /// The scope the launcher asks the server for.
    pub scope: &'a str,
    /// The page of the login of the program that runs the launcher.
    pub callback: &'a dyn OAuthCallback,
}

/// The way the device flow waits between two questions.
///
/// The tests replace the way, so a login that waits fifteen minutes does not
/// keep the tests waiting.
trait Clock {
    /// Waits for the duration.
    fn sleep(&mut self, duration: Duration);

    /// Returns the time that passed since the login began.
    fn elapsed(&self) -> Duration;
}

/// The clock of the launcher, which waits for the real time.
#[derive(Debug, Default)]
struct SystemClock {
    /// The moment the login began.
    start: Option<Instant>,
}

impl Clock for SystemClock {
    fn sleep(&mut self, duration: Duration) {
        self.start.get_or_insert_with(Instant::now);
        std::thread::sleep(duration);
    }

    fn elapsed(&self) -> Duration {
        self.start.map_or(Duration::ZERO, |start| start.elapsed())
    }
}

/// Sends a request and reads the body of the answer as a value.
///
/// The body is read even when the status is not a success, because the servers
/// of the accounts name the reason of a refused login in the body.
fn read_json<T: DeserializeOwned>(
    client: &dyn HttpClient,
    request: &HttpRequest,
) -> Result<T, AuthenticationError> {
    let response = send(client, request).map_err(AuthenticationError::from)?;
    serde_json::from_str(&response.body).map_err(|error| {
        AuthenticationError::malformed(format!(
            "the answer of {} is not what the launcher reads: {error}",
            request.url()
        ))
    })
}

/// Reports the refusal of the server, if the answer holds one.
///
/// The server of the accounts names a refused login with `error` and explains
/// it in `error_description`; the launcher of Java reads the answer as a
/// refusal only when both fields are there, and this function does the same, so
/// an answer that names a reason without explaining it is read as a success.
fn check_error(response: &Refusal) -> Result<(), AuthenticationError> {
    let (Some(error), Some(description)) = (&response.error, &response.error_description) else {
        return Ok(());
    };

    // The credentials of the account are not accepted any more, which only a
    // new login of the player solves.
    if error == "invalid_grant" && description.contains("AADSTS70000") {
        return Err(AuthenticationError::CredentialExpired { message: None });
    }

    Err(AuthenticationError::RemoteAuthentication {
        name: error.clone(),
        message: description.clone(),
        cause: String::new(),
    })
}

/// Returns the challenge of the proof key of a verifier (RFC 7636).
///
/// The challenge is the SHA-256 digest of the verifier, written in the Base64
/// encoding that is safe in an address and without padding.
fn generate_code_challenge(code_verifier: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(code_verifier.as_bytes());
    base64::encode_url_safe_unpadded(&digest.finish())
}

/// The answer of the server to the request of the code of the device flow.
///
/// A refused request carries the fields of [`Refusal`] instead of the code.
#[derive(Debug, Clone, Deserialize)]
struct DeviceCodeResponse {
    /// The refusal of the server, if the server refused the request.
    #[serde(flatten)]
    refusal: Refusal,
    /// The code the player writes on the page.
    #[serde(default, rename = "user_code")]
    user_code: String,
    /// The code the launcher asks for the token with.
    #[serde(default, rename = "device_code")]
    device_code: String,
    /// The page where the player writes the code.
    #[serde(default, rename = "verification_uri")]
    verification_uri: String,
    /// The life of the code, in seconds.
    #[serde(default, rename = "expires_in")]
    expires_in: i32,
    /// The wait between two questions, in seconds.
    #[serde(default)]
    interval: i32,
}

/// The answer of the server to the exchange of the code of the page.
#[derive(Debug, Clone, Deserialize)]
struct AuthorizationResponse {
    /// The refusal of the server, if the server refused the exchange.
    #[serde(flatten)]
    refusal: Refusal,
    /// The token the account is used with.
    #[serde(default, rename = "access_token")]
    access_token: Option<String>,
    /// The token that renews the access token.
    #[serde(default, rename = "refresh_token")]
    refresh_token: Option<String>,
}

impl AuthorizationResponse {
    /// Returns the tokens of the answer.
    ///
    /// The server of the accounts always names an access token with a success,
    /// so an answer without one is an answer the launcher cannot read.
    fn into_tokens(self) -> Result<OAuthTokens, AuthenticationError> {
        let Some(access_token) = self.access_token.filter(|token| !token.is_empty()) else {
            return Err(AuthenticationError::malformed(
                "the answer of the server holds no access token",
            ));
        };

        Ok(OAuthTokens {
            access_token,
            refresh_token: self.refresh_token.unwrap_or_default(),
        })
    }
}

/// The answer of the server to the exchange of a refresh token.
#[derive(Debug, Clone, Deserialize)]
struct RefreshResponse {
    /// The refusal of the server, if the server refused the exchange.
    #[serde(flatten)]
    refusal: Refusal,
    /// The token the account is used with.
    #[serde(default, rename = "access_token")]
    access_token: Option<String>,
    /// The token that renews the access token.
    #[serde(default, rename = "refresh_token")]
    refresh_token: Option<String>,
}

impl RefreshResponse {
    /// Returns the tokens of the answer.
    ///
    /// A server that renews a token without naming a new refresh token keeps
    /// the old one valid, so the caller keeps its own.
    fn into_tokens(self, previous_refresh_token: &str) -> Result<OAuthTokens, AuthenticationError> {
        let Some(access_token) = self.access_token.filter(|token| !token.is_empty()) else {
            return Err(AuthenticationError::malformed(
                "the answer of the server holds no access token",
            ));
        };

        Ok(OAuthTokens {
            access_token,
            refresh_token: self
                .refresh_token
                .filter(|token| !token.is_empty())
                .unwrap_or_else(|| previous_refresh_token.to_string()),
        })
    }
}

/// The answer of the server that asks for the token of the device flow.
#[derive(Debug, Clone, Deserialize)]
struct TokenResponse {
    /// The refusal of the server, if the server refused the request.
    #[serde(flatten)]
    refusal: Refusal,
    /// The token the account is used with.
    #[serde(default, rename = "access_token")]
    access_token: Option<String>,
    /// The token that renews the access token.
    #[serde(default, rename = "refresh_token")]
    refresh_token: Option<String>,
}

impl TokenResponse {
    /// Returns the reason the player is not done yet, if the answer names one.
    fn reason(&self) -> Option<&str> {
        self.refusal.error.as_deref()
    }

    /// Returns the tokens of the answer.
    fn into_tokens(self) -> Result<OAuthTokens, AuthenticationError> {
        let Some(access_token) = self.access_token.filter(|token| !token.is_empty()) else {
            return Err(AuthenticationError::malformed(
                "the answer of the server holds no access token",
            ));
        };

        Ok(OAuthTokens {
            access_token,
            refresh_token: self.refresh_token.unwrap_or_default(),
        })
    }
}

/// The fields an answer of the servers of the accounts names a refusal with.
///
/// A refusal also names the identifier of the request, which the launcher of
/// Java keeps without showing it, so it is not read here.
#[derive(Debug, Clone, Default, Deserialize)]
struct Refusal {
    /// The name of the reason, such as `invalid_grant`.
    #[serde(default)]
    error: Option<String>,
    /// The explanation of the reason.
    #[serde(default, rename = "error_description")]
    error_description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::http_request::HttpResponse;
    use std::cell::{Cell, RefCell};

    /// A client that answers with the answers it is given and keeps the
    /// requests it was asked.
    struct FakeClient {
        /// The answers, in the order they are returned.
        answers: RefCell<Vec<Result<HttpResponse, String>>>,
        /// The requests the client was asked, in the order they were asked.
        asked: RefCell<Vec<HttpRequest>>,
    }

    impl FakeClient {
        fn new(answers: Vec<&str>) -> Self {
            Self {
                answers: RefCell::new(
                    answers
                        .into_iter()
                        .map(|body| Ok(HttpResponse::new(200, body)))
                        .collect(),
                ),
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for FakeClient {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
            self.asked.borrow_mut().push(request.clone());
            let mut answers = self.answers.borrow_mut();
            if answers.is_empty() {
                return Err("the client was asked too often".to_string());
            }
            answers.remove(0)
        }
    }

    /// The page of a login that answers at once and keeps what it was told.
    struct FakeSession {
        /// The code the page answers with.
        code: Result<String, AuthenticationError>,
    }

    impl OAuthSession for FakeSession {
        fn state(&self) -> String {
            "state".to_string()
        }

        fn code_verifier(&self) -> String {
            "verifier".to_string()
        }

        fn redirect_uri(&self) -> String {
            "http://localhost:29111/auth-response".to_string()
        }

        fn wait_for_code(&self) -> Result<String, AuthenticationError> {
            self.code.clone()
        }
    }

    /// The callback of the tests, which keeps the pages it was asked for.
    struct FakeCallback {
        /// The code the page answers with.
        code: Result<String, AuthenticationError>,
        /// The addresses of the pages the launcher opened.
        opened: RefCell<Vec<String>>,
        /// The code of the device flow the launcher showed.
        device_code: RefCell<Option<(String, String)>>,
        /// Whether the device flow reported a completed login.
        completed: Cell<bool>,
    }

    impl FakeCallback {
        fn new(code: &str) -> Self {
            Self {
                code: Ok(code.to_string()),
                opened: RefCell::new(Vec::new()),
                device_code: RefCell::new(None),
                completed: Cell::new(false),
            }
        }
    }

    impl OAuthCallback for FakeCallback {
        fn start_session(&self) -> Result<Box<dyn OAuthSession>, AuthenticationError> {
            Ok(Box::new(FakeSession {
                code: self.code.clone(),
            }))
        }

        fn grant_device_code(&self, user_code: &str, verification_uri: &str) {
            *self.device_code.borrow_mut() =
                Some((user_code.to_string(), verification_uri.to_string()));
        }

        fn login_completed_device_code(&self) {
            self.completed.set(true);
        }

        fn open_browser(&self, _flow: GrantFlow, url: &str) -> Result<(), String> {
            self.opened.borrow_mut().push(url.to_string());
            Ok(())
        }

        fn client_id(&self) -> String {
            "client".to_string()
        }
    }

    /// The options of a login with a callback.
    fn options<'a>(callback: &'a FakeCallback) -> Options<'a> {
        Options {
            scope: MICROSOFT_SCOPE,
            callback,
        }
    }

    #[test]
    fn the_challenge_of_the_proof_key_is_the_digest_of_the_verifier() {
        // The verifier and the challenge of the example of RFC 7636.
        assert_eq!(
            generate_code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn the_page_of_the_login_names_the_challenge_and_the_state() {
        let client = FakeClient::new(vec![
            r#"{"access_token":"access","refresh_token":"refresh"}"#,
        ]);
        let callback = FakeCallback::new("code");
        let tokens = OAuth::microsoft()
            .authenticate(&client, GrantFlow::AuthorizationCode, &options(&callback))
            .expect("the login should succeed");

        assert_eq!(tokens.access_token, "access");
        assert_eq!(tokens.refresh_token, "refresh");

        let opened = callback.opened.borrow();
        assert_eq!(opened.len(), 1);
        let url = &opened[0];
        assert!(url.starts_with(MICROSOFT_AUTHORIZATION_URL), "{url}");
        assert!(url.contains("response_type=code"), "{url}");
        assert!(url.contains("code_challenge_method=S256"), "{url}");
        assert!(url.contains("prompt=select_account"), "{url}");
        assert!(
            url.contains(&format!(
                "state={}",
                crate::util::http_request::encode_form_component("state")
            )),
            "{url}"
        );
        assert!(
            url.contains(&format!(
                "code_challenge={}",
                generate_code_challenge("verifier")
            )),
            "{url}"
        );
    }

    #[test]
    fn the_code_of_the_page_is_exchanged_with_the_verifier() {
        let client = FakeClient::new(vec![
            r#"{"access_token":"access","refresh_token":"refresh"}"#,
        ]);
        let callback = FakeCallback::new("the-code");
        OAuth::microsoft()
            .authenticate(&client, GrantFlow::AuthorizationCode, &options(&callback))
            .expect("the login should succeed");

        let asked = client.asked.borrow();
        assert_eq!(asked.len(), 1);
        let body = String::from_utf8(asked[0].body().unwrap().to_vec()).unwrap();
        assert_eq!(
            body,
            "client_id=client&code=the-code&grant_type=authorization_code&code_verifier=verifier\
             &redirect_uri=http%3A%2F%2Flocalhost%3A29111%2Fauth-response&scope=XboxLive.signin+offline_access"
        );
    }

    #[test]
    fn a_page_that_is_closed_ends_the_login() {
        let client = FakeClient::new(vec![]);
        let mut callback = FakeCallback::new("code");
        callback.code = Err(AuthenticationError::NoSelectedCharacter);

        let error = OAuth::microsoft()
            .authenticate(&client, GrantFlow::AuthorizationCode, &options(&callback))
            .expect_err("the login should fail");

        assert_eq!(error, AuthenticationError::NoSelectedCharacter);
        assert!(client.asked.borrow().is_empty());
    }

    #[test]
    fn a_refusal_of_the_server_names_the_reason_and_the_description() {
        let client = FakeClient::new(vec![
            r#"{"error":"invalid_grant","error_description":"the code is not valid"}"#,
        ]);
        let callback = FakeCallback::new("code");

        let error = OAuth::microsoft()
            .authenticate(&client, GrantFlow::AuthorizationCode, &options(&callback))
            .expect_err("the login should fail");

        assert_eq!(
            error,
            AuthenticationError::RemoteAuthentication {
                name: "invalid_grant".to_string(),
                message: "the code is not valid".to_string(),
                cause: String::new(),
            }
        );
    }

    #[test]
    fn expired_credentials_are_told_apart_from_other_refusals() {
        let client = FakeClient::new(vec![
            r#"{"error":"invalid_grant","error_description":"AADSTS70000: the account is gone"}"#,
        ]);
        let callback = FakeCallback::new("code");

        let error = OAuth::microsoft()
            .authenticate(&client, GrantFlow::AuthorizationCode, &options(&callback))
            .expect_err("the login should fail");
        assert_eq!(
            error,
            AuthenticationError::CredentialExpired { message: None }
        );
        assert!(error.requires_login());

        // A refusal of another reason is not a login that expired.
        let client = FakeClient::new(vec![
            r#"{"error":"invalid_grant","error_description":"AADSTS90019: the tenant is not known"}"#,
        ]);
        let error = OAuth::microsoft()
            .authenticate(&client, GrantFlow::AuthorizationCode, &options(&callback))
            .expect_err("the login should fail");
        assert!(matches!(
            error,
            AuthenticationError::RemoteAuthentication { .. }
        ));
        assert!(!error.requires_login());
    }

    #[test]
    fn a_reason_without_an_explanation_is_not_a_refusal() {
        // The launcher of Java reads the answer as a refusal only when the
        // server names the reason and explains it.
        let refusal = Refusal {
            error: Some("invalid_grant".to_string()),
            error_description: None,
        };
        assert!(check_error(&refusal).is_ok());

        let refusal = Refusal {
            error: None,
            error_description: Some("the code is not valid".to_string()),
        };
        assert!(check_error(&refusal).is_ok());
    }

    #[test]
    fn the_refresh_names_the_token_and_the_type_of_the_answer_it_reads() {
        let client = FakeClient::new(vec![
            r#"{"access_token":"access","refresh_token":"refresh","expires_in":3600}"#,
        ]);
        let callback = FakeCallback::new("code");

        let tokens = OAuth::microsoft()
            .refresh(&client, "the-refresh", &options(&callback))
            .expect("the refresh should succeed");

        assert_eq!(tokens.access_token, "access");
        assert_eq!(tokens.refresh_token, "refresh");

        let asked = client.asked.borrow();
        assert_eq!(asked.len(), 1);
        assert_eq!(
            asked[0]
                .headers()
                .iter()
                .find(|(name, _)| name == "Accept")
                .map(|(_, value)| value.as_str()),
            Some("application/json")
        );
        assert_eq!(
            String::from_utf8(asked[0].body().unwrap().to_vec()).unwrap(),
            "client_id=client&refresh_token=the-refresh&grant_type=refresh_token"
        );
    }

    #[test]
    fn a_refresh_that_names_no_new_refresh_token_keeps_the_old_one() {
        let client = FakeClient::new(vec![r#"{"access_token":"access"}"#]);
        let callback = FakeCallback::new("code");

        let tokens = OAuth::microsoft()
            .refresh(&client, "the-refresh", &options(&callback))
            .expect("the refresh should succeed");

        assert_eq!(tokens.refresh_token, "the-refresh");
    }

    #[test]
    fn an_answer_without_a_token_is_malformed() {
        let client = FakeClient::new(vec![r#"{"refresh_token":"refresh"}"#]);
        let callback = FakeCallback::new("code");

        let error = OAuth::microsoft()
            .refresh(&client, "the-refresh", &options(&callback))
            .expect_err("the refresh should fail");

        assert!(matches!(
            error,
            AuthenticationError::ServerResponseMalformed { .. }
        ));
    }

    #[test]
    fn an_answer_that_is_not_json_is_malformed() {
        let client = FakeClient::new(vec!["<html>the page is not here</html>"]);
        let callback = FakeCallback::new("code");

        let error = OAuth::microsoft()
            .refresh(&client, "the-refresh", &options(&callback))
            .expect_err("the refresh should fail");

        assert!(matches!(
            error,
            AuthenticationError::ServerResponseMalformed { .. }
        ));
    }

    #[test]
    fn the_device_flow_asks_again_until_the_player_is_done() {
        let client = FakeClient::new(vec![
            // The code the player writes.
            r#"{"user_code":"ABCD","device_code":"the-device","verification_uri":"https://microsoft.com/link","expires_in":900,"interval":1}"#,
            // The player has not written the code yet.
            r#"{"error":"authorization_pending"}"#,
            // The launcher asked too often, so it waits longer.
            r#"{"error":"slow_down"}"#,
            // The player is done.
            r#"{"access_token":"access","refresh_token":"refresh"}"#,
        ]);
        let callback = FakeCallback::new("code");
        let mut clock = TestClock::default();

        let tokens = OAuth::microsoft()
            .authenticate_device(&client, &options(&callback), &mut clock)
            .expect("the login should succeed");

        assert_eq!(tokens.access_token, "access");
        assert_eq!(
            callback.device_code.borrow().clone(),
            Some(("ABCD".to_string(), "https://microsoft.com/link".to_string()))
        );
        assert!(callback.completed.get());

        // The waits are the interval of the server, and the interval grows by
        // five seconds when the server asks for it.
        assert_eq!(
            clock.sleeps,
            vec![
                Duration::from_secs(1),
                Duration::from_secs(1),
                Duration::from_secs(6)
            ]
        );

        // The first question is asked after the first wait.
        let asked = client.asked.borrow();
        assert_eq!(asked.len(), 4);
        assert_eq!(
            String::from_utf8(asked[1].body().unwrap().to_vec()).unwrap(),
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&code=the-device\
             &client_id=client"
        );
    }

    #[test]
    fn a_device_code_that_expires_ends_the_login() {
        let client = FakeClient::new(vec![
            r#"{"user_code":"ABCD","device_code":"the-device","verification_uri":"https://microsoft.com/link","expires_in":900,"interval":1}"#,
            r#"{"error":"expired_token"}"#,
        ]);
        let callback = FakeCallback::new("code");
        let mut clock = TestClock::default();

        let error = OAuth::microsoft()
            .authenticate_device(&client, &options(&callback), &mut clock)
            .expect_err("the login should fail");

        assert_eq!(error, AuthenticationError::NoSelectedCharacter);
        assert!(!callback.completed.get());
    }

    #[test]
    fn the_wait_of_the_device_flow_ends_after_fifteen_minutes() {
        // The server names a longer life than the launcher waits, so the
        // launcher gives up after the fifteen minutes it waits.
        let answers: Vec<&str> = std::iter::once(
            r#"{"user_code":"ABCD","device_code":"the-device","verification_uri":"https://microsoft.com/link","expires_in":3600,"interval":1}"#,
        )
        .chain(std::iter::repeat_n(
            r#"{"error":"authorization_pending"}"#,
            MAX_DEVICE_WAIT_SECONDS as usize,
        ))
        .collect();
        let client = FakeClient::new(answers);
        let callback = FakeCallback::new("code");
        let mut clock = TestClock::default();

        let error = OAuth::microsoft()
            .authenticate_device(&client, &options(&callback), &mut clock)
            .expect_err("the login should fail");

        assert_eq!(error, AuthenticationError::NoSelectedCharacter);
        assert_eq!(clock.sleeps.len(), MAX_DEVICE_WAIT_SECONDS as usize);
    }

    #[test]
    fn the_device_flow_waits_at_least_a_millisecond() {
        // A server that names no interval would make the launcher ask without
        // a pause, which the server refuses, so the launcher waits a
        // millisecond.
        let client = FakeClient::new(vec![
            r#"{"user_code":"ABCD","device_code":"the-device","verification_uri":"https://microsoft.com/link","expires_in":900,"interval":0}"#,
            r#"{"access_token":"access","refresh_token":"refresh"}"#,
        ]);
        let callback = FakeCallback::new("code");
        let mut clock = TestClock::default();

        OAuth::microsoft()
            .authenticate_device(&client, &options(&callback), &mut clock)
            .expect("the login should succeed");

        assert_eq!(clock.sleeps, vec![Duration::from_millis(1)]);
    }

    #[test]
    fn a_refused_request_of_the_device_code_is_not_read_as_a_code() {
        let client = FakeClient::new(vec![
            r#"{"error":"invalid_client","error_description":"the client is not known"}"#,
        ]);
        let callback = FakeCallback::new("code");
        let mut clock = TestClock::default();

        let error = OAuth::microsoft()
            .authenticate_device(&client, &options(&callback), &mut clock)
            .expect_err("the login should fail");

        assert_eq!(
            error,
            AuthenticationError::RemoteAuthentication {
                name: "invalid_client".to_string(),
                message: "the client is not known".to_string(),
                cause: String::new(),
            }
        );
    }

    /// The clock of the tests, which records the waits instead of sleeping.
    #[derive(Debug, Default)]
    struct TestClock {
        /// The durations the login waited for.
        sleeps: Vec<Duration>,
        /// The time the login has waited.
        elapsed: Duration,
    }

    impl Clock for TestClock {
        fn sleep(&mut self, duration: Duration) {
            self.sleeps.push(duration);
            // The time of the login moves with the wait, so a login that waits
            // fifteen minutes ends without the tests waiting for them.
            self.elapsed += duration;
        }

        fn elapsed(&self) -> Duration {
            self.elapsed
        }
    }
}
