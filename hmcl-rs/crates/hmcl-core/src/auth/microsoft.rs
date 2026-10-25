//! The login with a Microsoft account.
//!
//! A Microsoft account is not the account of the game. The launcher first logs
//! the player in to Microsoft with [`oauth`](super::oauth), then proves the
//! account to Xbox Live, then proves it to the servers of the game, and finally
//! reads the profile of the player. Each step answers with a token the next step
//! is asked with, so the chain is the whole login and not a single request.
//!
//! The values of the chain are held by [`MicrosoftSession`]: the token of the
//! game, the token that renews it, the moment it expires and the profile of the
//! player. The session is what the launcher stores, which is why it is the value
//! this module hands out.
//!
//! The core opens no connection: the requests are sent by the [`HttpClient`] the
//! program gives, so the whole chain can be tested against the answers of a
//! server that is not there.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::util::datetime::unix_millis;
use crate::util::http_request::{send, HttpClient, HttpRequest};
use crate::util::logging;

use super::auth_info::{AuthInfo, UserType};
use super::error::AuthenticationError;
use super::oauth::{GrantFlow, OAuth, OAuthCallback, OAuthTokens, Options, MICROSOFT_SCOPE};
use super::uuid::Uuid;

/// The address where the token of Microsoft is proved to Xbox Live.
const XBOX_USER_AUTHENTICATE_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
/// The address where the token of Xbox Live is proved to the servers of the
/// game.
const XBOX_XSTS_AUTHORIZE_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
/// The address where the token of Xbox Live is exchanged for the token of the
/// game.
const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
/// The address that tells whether the account owns the game.
const MINECRAFT_STORE_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
/// The address of the profile of the player.
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
/// The address that names the games the account owns.
const MINECRAFT_LICENSE_URL: &str = "https://api.minecraftservices.com/entitlements/license";

/// The number of the attempts a request of the login makes.
const ATTEMPTS: u32 = 5;

/// The code Xbox Live answers with when the account is banned.
pub const XBOX_BANNED: i64 = 2148916227;
/// The code Xbox Live answers with when the account has no profile of Xbox
/// Live.
pub const XBOX_MISSING_XBOX_ACCOUNT: i64 = 2148916233;
/// The code Xbox Live answers with when the country of the account cannot use
/// Xbox Live.
pub const XBOX_COUNTRY_UNAVAILABLE: i64 = 2148916235;
/// The code Xbox Live answers with when the account of a child is not linked to
/// a family.
pub const XBOX_ADD_FAMILY: i64 = 2148916238;

/// The profile of the player, as the servers of the game know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameProfile {
    /// The identifier of the player.
    pub id: Uuid,
    /// The name of the player.
    pub name: String,
}

/// The result of the login of a Microsoft account.
///
/// The session is what the launcher keeps: the token of the game and the token
/// that renews it, the moment the token expires, the identifier of the account
/// of Microsoft, and the profile of the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicrosoftSession {
    /// The kind of the access token, such as `Bearer`.
    token_type: String,
    /// The token the game is started with.
    access_token: String,
    /// The moment the access token expires, in milliseconds since the start of
    /// the epoch.
    not_after: i64,
    /// The token that renews the access token.
    refresh_token: String,
    /// The identifier of the account of Microsoft, which the servers of the
    /// game do not use.
    user_id: String,
    /// The profile of the player.
    profile: GameProfile,
}

impl MicrosoftSession {
    /// Creates a session of a logged in account.
    ///
    /// The access token is registered with the log, so a log the player sends
    /// with a report does not hold the token of the account.
    pub fn new(
        token_type: impl Into<String>,
        access_token: impl Into<String>,
        not_after: i64,
        refresh_token: impl Into<String>,
        user_id: impl Into<String>,
        profile: GameProfile,
    ) -> Self {
        let access_token = access_token.into();
        logging::register_access_token(&access_token);

        Self {
            token_type: token_type.into(),
            access_token,
            not_after,
            refresh_token: refresh_token.into(),
            user_id: user_id.into(),
            profile,
        }
    }

    /// Returns the kind of the access token, such as `Bearer`.
    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    /// Returns the token the game is started with.
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// Returns the moment the access token expires, in milliseconds since the
    /// start of the epoch.
    pub fn not_after(&self) -> i64 {
        self.not_after
    }

    /// Returns the token that renews the access token.
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }

    /// Returns the identifier of the account of Microsoft.
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Returns the profile of the player.
    pub fn profile(&self) -> &GameProfile {
        &self.profile
    }

    /// Returns the value the servers of the game are asked with.
    ///
    /// The value is the kind of the token and the token itself, written the way
    /// the header `Authorization` holds them.
    pub fn authorization(&self) -> String {
        format!("{} {}", self.token_type, self.access_token)
    }

    /// Returns whether the session names the player.
    ///
    /// A session read from the settings of a launcher that did not know the
    /// name of the player holds an empty name, which the game cannot be started
    /// with.
    pub fn has_profile_name(&self) -> bool {
        !self.profile.name.trim().is_empty()
    }

    /// Returns the information the game is started with.
    ///
    /// The properties of the player are written as an empty object, which is
    /// what the launcher of Java writes for an account of Microsoft: the game
    /// reads the skin of the player from the servers of the game.
    pub fn to_auth_info(&self) -> AuthInfo {
        AuthInfo::new(
            self.profile.name.clone(),
            self.profile.id,
            self.access_token.clone(),
            UserType::Msa,
            Some("{}".to_string()),
        )
    }

    /// Reads a session out of the settings of the launcher.
    ///
    /// The identifier of the player is written with the record of the account,
    /// and the tokens are written with the protected data, which is not part of
    /// the settings the launcher shows.
    pub fn from_storage(
        metadata: &Value,
        private_data: &Map<String, Value>,
    ) -> Result<Self, AuthenticationError> {
        let profile_id = metadata
            .get("profileID")
            .and_then(Value::as_str)
            .ok_or_else(|| AuthenticationError::invalid_stored("profileID is missing"))?;
        let profile_id = Uuid::parse(profile_id).map_err(|error| {
            AuthenticationError::invalid_stored(format!("profileID is malformed: {error}"))
        })?;

        let profile_name = match private_data.get("profileName") {
            Some(Value::String(name)) => name.clone(),
            _ => String::new(),
        };

        let token_type = required_storage_string(private_data, "tokenType")?;
        let access_token = required_storage_string(private_data, "accessToken")?;
        let refresh_token = required_storage_string(private_data, "refreshToken")?;
        let user_id = required_storage_string(private_data, "userid")?;

        // A moment that is not a number is read as a token that already
        // expired, which makes the next login renew the token.
        let not_after = private_data
            .get("notAfter")
            .and_then(Value::as_i64)
            .unwrap_or_default();

        Ok(Self::new(
            token_type,
            access_token,
            not_after,
            refresh_token,
            user_id,
            GameProfile {
                id: profile_id,
                name: profile_name,
            },
        ))
    }

    /// Writes the session into the protected data of the account.
    ///
    /// The entries are written in the order the launcher of Java writes them,
    /// so a file written by one launcher is read by the other.
    pub fn write_private_data(&self, private_data: &mut Map<String, Value>) {
        private_data.insert(
            "profileName".to_string(),
            Value::String(self.profile.name.clone()),
        );
        private_data.insert(
            "tokenType".to_string(),
            Value::String(self.token_type.clone()),
        );
        private_data.insert(
            "accessToken".to_string(),
            Value::String(self.access_token.clone()),
        );
        private_data.insert(
            "refreshToken".to_string(),
            Value::String(self.refresh_token.clone()),
        );
        private_data.insert("notAfter".to_string(), Value::from(self.not_after));
        private_data.insert("userid".to_string(), Value::String(self.user_id.clone()));
    }
}

/// Reads a text the settings of an account must hold.
fn required_storage_string(
    storage: &Map<String, Value>,
    name: &str,
) -> Result<String, AuthenticationError> {
    storage
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AuthenticationError::invalid_stored(format!("{name} is missing")))
}

/// Logs the player in with the way the player chose.
///
/// The token of Microsoft is proved to Xbox Live, the token of Xbox Live is
/// proved to the servers of the game, the token of the game is checked against
/// the copy of the game the account owns, and the profile of the player is read.
/// The refresh token of the login is kept, because it is what renews the tokens
/// of the account later.
pub fn authenticate(
    client: &dyn HttpClient,
    flow: GrantFlow,
    callback: &dyn OAuthCallback,
) -> Result<MicrosoftSession, AuthenticationError> {
    let oauth = OAuth::microsoft();
    let options = Options {
        scope: MICROSOFT_SCOPE,
        callback,
    };
    let tokens = oauth.authenticate(client, flow, &options)?;
    authenticate_via_live_access_token(client, &tokens)
}

/// Renews the tokens of an account.
///
/// The refresh token of the session is exchanged for a new access token, and
/// the new token is proved to the servers of the game the same way a fresh login
/// is. The profile of the player is read again, because the name of the player
/// may have changed.
pub fn refresh(
    client: &dyn HttpClient,
    session: &MicrosoftSession,
    callback: &dyn OAuthCallback,
) -> Result<MicrosoftSession, AuthenticationError> {
    let oauth = OAuth::microsoft();
    let options = Options {
        scope: MICROSOFT_SCOPE,
        callback,
    };
    let tokens = oauth.refresh(client, &session.refresh_token, &options)?;
    authenticate_via_live_access_token(client, &tokens)
}

/// Asks the servers of the game whether the account still owns the game.
///
/// The token of the session is used for the profile of the player: a token that
/// is accepted answers with the profile, and a token that is not accepted is
/// answered with a status that is not a success. The moment the token expires
/// is checked first so an expired token is not sent at all.
pub fn validate(
    client: &dyn HttpClient,
    session: &MicrosoftSession,
) -> Result<bool, AuthenticationError> {
    if unix_millis() > session.not_after {
        return Ok(false);
    }

    match get_minecraft_profile(client, &session.authorization()) {
        Ok(_) => Ok(true),
        Err(AuthenticationError::ResponseCode { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Reads the profile of the player.
///
/// A server that knows no profile of the account may still have an account that
/// owns the game but has not created a profile yet, so the copy of the game is
/// asked for: that tells the player who has to create a profile from the player
/// who has to buy the game.
pub fn get_minecraft_profile(
    client: &dyn HttpClient,
    authorization: &str,
) -> Result<MinecraftProfile, AuthenticationError> {
    let request = HttpRequest::get(MINECRAFT_PROFILE_URL)
        .authorization(authorization)
        .accept("application/json")
        .ignore_status()
        .attempts(ATTEMPTS);

    let response = send(client, &request).map_err(AuthenticationError::from)?;

    match response.status {
        200 => {
            let profile: MinecraftProfile = serde_json::from_str(&response.body)
                .map_err(|error| malformed_json(MINECRAFT_PROFILE_URL, &error))?;
            handle_error_response(&profile.error)?;

            // The profile of a player always names the identifier and the name
            // of the player; an answer without them is not a profile.
            if profile.id.is_none() || profile.name.is_none() {
                return Err(AuthenticationError::malformed(
                    "the answer of the profile names no identifier or no name of the player",
                ));
            }

            Ok(profile)
        }
        404 => {
            if has_minecraft_license(client, authorization)? {
                Err(AuthenticationError::MinecraftProfileNotFound)
            } else {
                Err(AuthenticationError::MinecraftLicenseNotFound)
            }
        }
        status => Err(AuthenticationError::ResponseCode {
            url: MINECRAFT_PROFILE_URL.to_string(),
            status,
            data: Some(response.body),
        }),
    }
}

/// Returns whether the account owns a copy of the game.
fn has_minecraft_license(
    client: &dyn HttpClient,
    authorization: &str,
) -> Result<bool, AuthenticationError> {
    let request = HttpRequest::get(MINECRAFT_LICENSE_URL)
        .authorization(authorization)
        .accept("application/json")
        .attempts(ATTEMPTS);

    let license: MinecraftLicense = read_json(client, &request)?;
    Ok(license
        .items
        .iter()
        .any(|item| item.name.as_deref() == Some("game_minecraft")))
}

/// Proves the token of Microsoft to Xbox Live and to the servers of the game.
///
/// The chain is three requests: the token of Microsoft is exchanged for a token
/// of Xbox Live, the token of Xbox Live is exchanged for a token that names the
/// user hash of the account and that the servers of the game accept, and the
/// token of the game is read with that user hash. The copy of the game is asked
/// for before the profile, because an account that owns no copy of the game has
/// no profile to read.
fn authenticate_via_live_access_token(
    client: &dyn HttpClient,
    tokens: &OAuthTokens,
) -> Result<MicrosoftSession, AuthenticationError> {
    // Authenticate with Xbox Live.
    let xbox_request = HttpRequest::post(XBOX_USER_AUTHENTICATE_URL)
        .json(&json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", tokens.access_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))
        .accept("application/json")
        .attempts(ATTEMPTS);

    let xbox: XboxLiveAuthentication = read_xbox_json(client, &xbox_request)?;
    let uhs = get_uhs(&xbox, None)?;

    // Authorize with XSTS, whose token names the user hash the servers of the
    // game read. A child account without a family is answered with the status
    // `401` and a body that names the reason, so that status is read.
    let xsts_request = HttpRequest::post(XBOX_XSTS_AUTHORIZE_URL)
        .json(&json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbox.token.clone().unwrap_or_default()],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .tolerate_status(401)
        .accept("application/json")
        .attempts(ATTEMPTS);

    let xsts: XboxLiveAuthentication = read_xbox_json(client, &xsts_request)?;
    get_uhs(&xsts, Some(&uhs))?;

    // Authenticate with the servers of the game.
    let login_request = HttpRequest::post(MINECRAFT_LOGIN_URL)
        .json(&json!({
            "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token.unwrap_or_default()),
        }))
        .accept("application/json")
        .attempts(ATTEMPTS);

    let login: MinecraftLogin = read_json(client, &login_request)?;

    let not_after = i64::from(login.expires_in)
        .saturating_mul(1000)
        .saturating_add(unix_millis());

    // The copy of the game is checked before the profile: an account that owns
    // the game may not have a profile yet, and an account that owns no copy
    // answers every request of the game with a refusal.
    let store_request = HttpRequest::get(MINECRAFT_STORE_URL)
        .authorization(format!("Bearer {}", login.access_token))
        .accept("application/json")
        .attempts(ATTEMPTS);

    let store = send(client, &store_request).map_err(AuthenticationError::from)?;
    if store.status != 200 {
        return Err(AuthenticationError::ResponseCode {
            url: MINECRAFT_STORE_URL.to_string(),
            status: store.status,
            data: Some(store.body),
        });
    }

    let profile = get_minecraft_profile(
        client,
        &format!("{} {}", login.token_type, login.access_token),
    )?;

    // A profile that names no identifier was refused above, so the identifier
    // is here; the name may be missing for an account that has not chosen one.
    let Some(profile_id) = profile.id else {
        return Err(AuthenticationError::malformed(
            "the answer of the profile names no identifier of the player",
        ));
    };

    Ok(MicrosoftSession::new(
        login.token_type,
        login.access_token,
        not_after,
        tokens.refresh_token.clone(),
        login.username.unwrap_or_default(),
        GameProfile {
            id: profile_id,
            name: profile.name.unwrap_or_default(),
        },
    ))
}

/// Returns the user hash of an answer of Xbox Live.
///
/// The user hash is the identifier of the account of Xbox Live, and the two
/// answers of the chain must name the same one: an answer that names another
/// one means the account of the second answer is not the account of the first.
fn get_uhs(
    response: &XboxLiveAuthentication,
    expected: Option<&str>,
) -> Result<String, AuthenticationError> {
    if let Some(code) = response.error_code.filter(|code| *code != 0) {
        return Err(AuthenticationError::XboxAuthorization {
            code,
            redirect: response.redirect.clone(),
        });
    }

    let Some(uhs) = response
        .display_claims
        .as_ref()
        .and_then(|claims| claims.xui.first())
        .and_then(|claims| claims.uhs.as_deref())
    else {
        return Err(AuthenticationError::NoUserHash);
    };

    if let Some(expected) = expected {
        if expected != uhs {
            return Err(AuthenticationError::malformed("uhs mismatched"));
        }
    }

    Ok(uhs.to_string())
}

/// Reads an answer of Xbox Live, which names its refusal with `XErr`.
///
/// The server refuses a request of the launcher itself with the status `400`,
/// which the player cannot solve; every other status keeps its address and its
/// code.
fn read_xbox_json(
    client: &dyn HttpClient,
    request: &HttpRequest,
) -> Result<XboxLiveAuthentication, AuthenticationError> {
    match read_json(client, request) {
        Ok(response) => Ok(response),
        Err(AuthenticationError::ResponseCode { status: 400, .. }) => {
            Err(AuthenticationError::XboxBadRequest)
        }
        Err(error) => Err(error),
    }
}

/// Sends a request and reads the body of the answer as a value.
fn read_json<T: DeserializeOwned>(
    client: &dyn HttpClient,
    request: &HttpRequest,
) -> Result<T, AuthenticationError> {
    let response = send(client, request).map_err(AuthenticationError::from)?;
    serde_json::from_str(&response.body).map_err(|error| malformed_json(request.url(), &error))
}

/// Reports an answer the launcher cannot read.
fn malformed_json(url: &str, error: &serde_json::Error) -> AuthenticationError {
    AuthenticationError::malformed(format!("the answer of {url} is not JSON: {error}"))
}

/// Reports the refusal the profile of the player names, if it names one.
fn handle_error_response(response: &MinecraftErrorResponse) -> Result<(), AuthenticationError> {
    match &response.error {
        Some(error) => Err(AuthenticationError::RemoteAuthentication {
            name: error.clone(),
            message: response.error_message.clone().unwrap_or_default(),
            cause: response.developer_message.clone().unwrap_or_default(),
        }),
        None => Ok(()),
    }
}

/// The answer of the servers of Xbox Live.
///
/// A success names the token and the user hash of the account; a refusal names
/// the code of the reason and, often, the page the player may open to solve it.
///
/// A refusal also names a message of the server, which the launcher of Java
/// does not show either, so only the code and the page are read.
#[derive(Debug, Clone, Default, Deserialize)]
struct XboxLiveAuthentication {
    /// The code of the refusal, which is `0` or absent with a success.
    #[serde(default, rename = "XErr")]
    error_code: Option<i64>,
    /// The page the player may open to solve the refusal.
    #[serde(default, rename = "Redirect")]
    redirect: Option<String>,
    /// The token of the account.
    #[serde(default, rename = "Token")]
    token: Option<String>,
    /// The claims of the account, which hold the user hash.
    #[serde(default, rename = "DisplayClaims")]
    display_claims: Option<DisplayClaims>,
}

/// The claims of the account of Xbox Live.
#[derive(Debug, Clone, Default, Deserialize)]
struct DisplayClaims {
    /// The claims of the account, of which the first one holds the user hash.
    #[serde(default)]
    xui: Vec<Xui>,
}

/// The claims of one account of Xbox Live.
#[derive(Debug, Clone, Default, Deserialize)]
struct Xui {
    /// The user hash of the account.
    #[serde(default)]
    uhs: Option<String>,
}

/// The answer of the servers of the game to the exchange of the token.
#[derive(Debug, Clone, Default, Deserialize)]
struct MinecraftLogin {
    /// The name of the account of the game.
    #[serde(default)]
    username: Option<String>,
    /// The token the servers of the game are asked with.
    #[serde(default, rename = "access_token")]
    access_token: String,
    /// The kind of the token, which is written before the token in the header
    /// `Authorization`.
    #[serde(default, rename = "token_type")]
    token_type: String,
    /// The life of the token, in seconds.
    #[serde(default, rename = "expires_in")]
    expires_in: i32,
}

/// The profile of the player, as the servers of the game answer it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MinecraftProfile {
    /// The refusal the servers named instead of the profile, if any.
    #[serde(flatten)]
    error: MinecraftErrorResponse,
    /// The identifier of the player.
    #[serde(default)]
    id: Option<Uuid>,
    /// The name of the player.
    #[serde(default)]
    name: Option<String>,
}

impl MinecraftProfile {
    /// Returns the identifier of the player.
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// Returns the name of the player.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// The fields a refusal of the servers of the game is written with.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct MinecraftErrorResponse {
    /// The address of the request the refusal answers.
    #[serde(default)]
    path: Option<String>,
    /// The kind of the refusal.
    #[serde(default, rename = "errorType")]
    error_type: Option<String>,
    /// The name of the reason.
    #[serde(default)]
    error: Option<String>,
    /// The message of the reason, which is shown to the player.
    #[serde(default, rename = "errorMessage")]
    error_message: Option<String>,
    /// The message of the reason for the developer of the launcher.
    #[serde(default, rename = "developerMessage")]
    developer_message: Option<String>,
}

/// The copies of the game an account owns.
#[derive(Debug, Clone, Default, Deserialize)]
struct MinecraftLicense {
    /// The copies of the game.
    #[serde(default)]
    items: Vec<MinecraftLicenseItem>,
}

/// One copy of the game an account owns.
#[derive(Debug, Clone, Default, Deserialize)]
struct MinecraftLicenseItem {
    /// The name of the copy, such as `game_minecraft`.
    #[serde(default)]
    name: Option<String>,
}

/// The login of an account of Microsoft, as the launcher keeps it.
///
/// The launcher does not log an account in for every launch of the game: it
/// keeps the session of the last login and uses the token of the session until
/// it expires, which saves the player the wait of the servers of Microsoft. The
/// value tells the token that is still good from the token that has to be
/// renewed, and it is the value the pages of the launcher log an account in
/// through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicrosoftLogin {
    /// The tokens and the profile of the last login.
    session: MicrosoftSession,
    /// Whether the token of the session was accepted by a login of this run of
    /// the launcher.
    authenticated: bool,
}

impl MicrosoftLogin {
    /// Creates the login of a session read from the settings of the launcher.
    ///
    /// A session read from the settings is not authenticated: the launcher of
    /// Java does not trust the stored token until a server accepted it, so the
    /// first login of a run checks the token or renews it.
    pub fn new(session: MicrosoftSession) -> Self {
        Self {
            session,
            authenticated: false,
        }
    }

    /// Creates the login of a session a login of this run acquired.
    pub fn authenticated(session: MicrosoftSession) -> Self {
        Self {
            session,
            authenticated: true,
        }
    }

    /// Returns the tokens and the profile of the last login.
    pub fn session(&self) -> &MicrosoftSession {
        &self.session
    }

    /// Returns whether the token of the session was accepted by a login of this
    /// run of the launcher.
    pub const fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Returns the name of the player the session knows.
    pub fn profile_name(&self) -> &str {
        &self.session.profile.name
    }

    /// Returns the identifier of the player the session knows.
    pub const fn profile_id(&self) -> Uuid {
        self.session.profile.id
    }

    /// Returns the information the game is started with.
    ///
    /// The token of the session is checked first when it is not known to be
    /// good: a token the servers of the game still accept is used as it is, and
    /// a token they refuse is renewed with the refresh token. A profile of
    /// another player than the one of the session, or a profile without a name,
    /// is a failure of the servers rather than a login.
    pub fn log_in(
        &mut self,
        client: &dyn HttpClient,
        callback: &dyn OAuthCallback,
    ) -> Result<AuthInfo, AuthenticationError> {
        let expired = !self.authenticated
            || !self.session.has_profile_name()
            || unix_millis() > self.session.not_after;

        if expired {
            // The token may be expired only by the clock of the launcher, so a
            // session with a name is checked before it is renewed: a token the
            // servers still accept is cheaper than a login of Microsoft.
            if self.session.has_profile_name() && validate(client, &self.session)? {
                self.authenticated = true;
            } else {
                let acquired = refresh(client, &self.session, callback)?;

                if acquired.profile.id != self.session.profile.id {
                    return Err(AuthenticationError::malformed("Selected profile changed"));
                }
                if !acquired.has_profile_name() {
                    return Err(AuthenticationError::malformed("Profile name is missing"));
                }

                self.session = acquired;
                self.authenticated = true;
            }
        }

        Ok(self.session.to_auth_info())
    }

    /// Logs the player in with a new login of Microsoft, for the case the
    /// credentials of the account are not accepted any more.
    ///
    /// The player writes a code on the page of Microsoft, so the login does not
    /// need the browser of the launcher. A player who logged in with another
    /// account is told apart from a player who logged in with the same account,
    /// so the launcher does not silently replace the account of a record.
    pub fn log_in_when_credentials_expired(
        &mut self,
        client: &dyn HttpClient,
        callback: &dyn OAuthCallback,
    ) -> Result<AuthInfo, AuthenticationError> {
        let acquired = authenticate(client, GrantFlow::Device, callback)?;

        if acquired.profile.id != self.session.profile.id {
            return Err(AuthenticationError::WrongAccount {
                expected: self.session.profile.id,
                actual: acquired.profile.id,
            });
        }

        self.session = acquired;
        self.authenticated = true;
        Ok(self.session.to_auth_info())
    }

    /// Returns the information of the player without asking a server.
    ///
    /// The game accepts a token the launcher read from the settings, so a
    /// player who plays alone does not wait for the servers of Microsoft. A
    /// session without a name cannot start the game, which is the one case the
    /// player has to log in again.
    pub fn play_offline(&self) -> Result<AuthInfo, AuthenticationError> {
        if !self.session.has_profile_name() {
            return Err(AuthenticationError::CredentialExpired {
                message: Some("Profile name is missing".to_string()),
            });
        }

        Ok(self.session.to_auth_info())
    }

    /// Writes the public data of the account into the settings of the
    /// launcher.
    ///
    /// The identifier of the player is written with the record of the account,
    /// because the launcher shows the account before the protected data is read.
    pub fn write_metadata(&self, metadata: &mut Map<String, Value>) {
        metadata.insert(
            "profileID".to_string(),
            Value::String(self.session.profile.id.to_string()),
        );
    }

    /// Writes the credentials of the account into the protected data.
    pub fn write_private_data(&self, private_data: &mut Map<String, Value>) {
        self.session.write_private_data(private_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::OAuthSession;
    use crate::util::http_request::HttpResponse;
    use std::cell::RefCell;

    /// The identifier of the player of the tests.
    const PLAYER: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";

    /// An answer of the server of the tests.
    struct Answer {
        /// The code of the answer.
        status: u16,
        /// The body of the answer.
        body: String,
    }

    impl Answer {
        /// Creates a successful answer with a body.
        fn ok(body: &str) -> Self {
            Self {
                status: 200,
                body: body.to_string(),
            }
        }

        /// Creates an answer with a status and a body.
        fn with_status(status: u16, body: &str) -> Self {
            Self {
                status,
                body: body.to_string(),
            }
        }
    }

    /// A client that answers with the answers it is given and keeps the
    /// requests it was asked.
    struct FakeClient {
        /// The answers, in the order they are returned.
        answers: RefCell<Vec<Answer>>,
        /// The requests the client was asked, in the order they were asked.
        asked: RefCell<Vec<HttpRequest>>,
    }

    impl FakeClient {
        /// Creates a client that answers with the answers in order.
        fn new(answers: Vec<Answer>) -> Self {
            Self {
                answers: RefCell::new(answers),
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
            let answer = answers.remove(0);
            Ok(HttpResponse::new(answer.status, answer.body))
        }
    }

    /// The page of a login that answers at once.
    struct FakeSession;

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
            Ok("code".to_string())
        }
    }

    /// The callback of the tests, which keeps the pages it was asked for.
    struct FakeCallback {
        /// The addresses of the pages the launcher opened.
        opened: RefCell<Vec<String>>,
    }

    impl FakeCallback {
        /// Creates a callback that keeps the pages it was asked for.
        fn new() -> Self {
            Self {
                opened: RefCell::new(Vec::new()),
            }
        }
    }

    impl OAuthCallback for FakeCallback {
        fn start_session(&self) -> Result<Box<dyn OAuthSession>, AuthenticationError> {
            Ok(Box::new(FakeSession))
        }

        fn grant_device_code(&self, _user_code: &str, _verification_uri: &str) {}

        fn login_completed_device_code(&self) {}

        fn open_browser(&self, _flow: GrantFlow, url: &str) -> Result<(), String> {
            self.opened.borrow_mut().push(url.to_string());
            Ok(())
        }

        fn client_id(&self) -> String {
            "client".to_string()
        }
    }

    /// The answers of a successful login, from the token of Microsoft to the
    /// profile of the player.
    fn successful_answers() -> Vec<Answer> {
        vec![
            Answer::ok(r#"{"Token":"xbox-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"Token":"xsts-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(
                r#"{"username":"069a79f444e94726a5befca90e38aaf5","access_token":"game-token","token_type":"Bearer","expires_in":86400}"#,
            ),
            Answer::ok(r#"{"items":[]}"#),
            Answer::ok(&format!(r#"{{"id":"{PLAYER}","name":"Notch"}}"#)),
        ]
    }

    /// A session of the tests that expires in an hour.
    fn session() -> MicrosoftSession {
        MicrosoftSession::new(
            "Bearer",
            "game-token",
            unix_millis() + 3_600_000,
            "refresh-token",
            "the-user",
            GameProfile {
                id: Uuid::parse(PLAYER).expect("the identifier should be valid"),
                name: "Notch".to_string(),
            },
        )
    }

    #[test]
    fn the_chain_of_the_login_asks_the_servers_in_the_order_of_the_launcher_of_java() {
        let tokens = OAuthTokens {
            access_token: "live-token".to_string(),
            refresh_token: "refresh-token".to_string(),
        };
        let client = FakeClient::new(successful_answers());

        let session =
            authenticate_via_live_access_token(&client, &tokens).expect("the login should succeed");

        assert_eq!(
            client.urls(),
            vec![
                XBOX_USER_AUTHENTICATE_URL.to_string(),
                XBOX_XSTS_AUTHORIZE_URL.to_string(),
                MINECRAFT_LOGIN_URL.to_string(),
                MINECRAFT_STORE_URL.to_string(),
                MINECRAFT_PROFILE_URL.to_string(),
            ]
        );

        assert_eq!(session.token_type, "Bearer");
        assert_eq!(session.access_token, "game-token");
        assert_eq!(session.refresh_token, "refresh-token");
        assert_eq!(session.user_id, "069a79f444e94726a5befca90e38aaf5");
        assert_eq!(session.profile.id.to_string(), PLAYER);
        assert_eq!(session.profile.name, "Notch");
        assert_eq!(session.authorization(), "Bearer game-token");

        // The token of Microsoft is proved with the relying party of the
        // servers of Xbox Live, and the token of Xbox Live with the relying
        // party of the servers of the game.
        let bodies = client.bodies();
        assert!(
            bodies[0].contains(r#""RpsTicket":"d=live-token""#),
            "{bodies:?}"
        );
        assert!(bodies[0].contains(r#""RelyingParty":"http://auth.xboxlive.com""#));
        assert!(bodies[0].contains(r#""AuthMethod":"RPS""#));
        assert!(
            bodies[1].contains(r#""UserTokens":["xbox-token"]"#),
            "{bodies:?}"
        );
        assert!(bodies[1].contains(r#""RelyingParty":"rp://api.minecraftservices.com/""#));
        assert!(bodies[1].contains(r#""SandboxId":"RETAIL""#));
        assert!(
            bodies[2].contains(r#""identityToken":"XBL3.0 x=the-uhs;xsts-token""#),
            "{bodies:?}"
        );
    }

    #[test]
    fn the_moment_the_token_expires_is_the_life_of_the_token_after_now() {
        let before = unix_millis();
        let tokens = OAuthTokens {
            access_token: "live-token".to_string(),
            refresh_token: "refresh-token".to_string(),
        };
        let client = FakeClient::new(successful_answers());

        let session =
            authenticate_via_live_access_token(&client, &tokens).expect("the login should succeed");
        let after = unix_millis();

        assert!(
            (before + 86_400_000..=after + 86_400_000).contains(&session.not_after),
            "{} is not within the window",
            session.not_after
        );
    }

    #[test]
    fn a_refusal_of_xbox_live_names_the_code_and_the_page() {
        let tokens = OAuthTokens {
            access_token: "live-token".to_string(),
            refresh_token: "refresh-token".to_string(),
        };
        let client = FakeClient::new(vec![Answer::ok(
            r#"{"Identity":"0","XErr":2148916238,"Message":"","Redirect":"https://start.ui.xboxlive.com/AddChildToFamily"}"#,
        )]);

        let error = authenticate_via_live_access_token(&client, &tokens)
            .expect_err("the login should fail");

        assert_eq!(
            error,
            AuthenticationError::XboxAuthorization {
                code: XBOX_ADD_FAMILY,
                redirect: Some("https://start.ui.xboxlive.com/AddChildToFamily".to_string()),
            }
        );
    }

    #[test]
    fn the_codes_of_the_refusals_of_xbox_live_are_the_codes_of_the_launcher_of_java() {
        assert_eq!(XBOX_BANNED, 2_148_916_227);
        assert_eq!(XBOX_MISSING_XBOX_ACCOUNT, 2_148_916_233);
        assert_eq!(XBOX_COUNTRY_UNAVAILABLE, 2_148_916_235);
        assert_eq!(XBOX_ADD_FAMILY, 2_148_916_238);
    }

    #[test]
    fn an_answer_of_xbox_live_without_a_user_hash_is_refused() {
        let response = XboxLiveAuthentication {
            error_code: Some(0),
            token: Some("token".to_string()),
            ..XboxLiveAuthentication::default()
        };

        assert_eq!(
            get_uhs(&response, None),
            Err(AuthenticationError::NoUserHash)
        );

        // A claim that holds no user hash is the same as no claim at all.
        let response = XboxLiveAuthentication {
            display_claims: Some(DisplayClaims {
                xui: vec![Xui { uhs: None }],
            }),
            ..XboxLiveAuthentication::default()
        };
        assert_eq!(
            get_uhs(&response, None),
            Err(AuthenticationError::NoUserHash)
        );
    }

    #[test]
    fn the_two_answers_of_xbox_live_must_name_the_same_user_hash() {
        let response = XboxLiveAuthentication {
            display_claims: Some(DisplayClaims {
                xui: vec![Xui {
                    uhs: Some("another".to_string()),
                }],
            }),
            ..XboxLiveAuthentication::default()
        };

        assert_eq!(
            get_uhs(&response, Some("the-uhs")),
            Err(AuthenticationError::malformed("uhs mismatched"))
        );
        assert_eq!(get_uhs(&response, None), Ok("another".to_string()));
    }

    #[test]
    fn a_request_the_launcher_wrote_wrongly_is_told_apart_from_a_refused_account() {
        let tokens = OAuthTokens {
            access_token: "live-token".to_string(),
            refresh_token: "refresh-token".to_string(),
        };
        let client = FakeClient::new(vec![Answer::with_status(400, "")]);

        let error = authenticate_via_live_access_token(&client, &tokens)
            .expect_err("the login should fail");
        assert_eq!(error, AuthenticationError::XboxBadRequest);
    }

    #[test]
    fn a_child_account_without_a_family_is_read_from_the_answer_of_xsts() {
        // The server of XSTS answers a child account without a family with the
        // status 401 and the reason in the body, so the body is read.
        let tokens = OAuthTokens {
            access_token: "live-token".to_string(),
            refresh_token: "refresh-token".to_string(),
        };
        let client = FakeClient::new(vec![
            Answer::ok(r#"{"Token":"xbox-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::with_status(
                401,
                r#"{"XErr":2148916238,"Redirect":"https://start.ui.xboxlive.com/AddChildToFamily"}"#,
            ),
        ]);

        let error = authenticate_via_live_access_token(&client, &tokens)
            .expect_err("the login should fail");

        assert_eq!(
            error,
            AuthenticationError::XboxAuthorization {
                code: XBOX_ADD_FAMILY,
                redirect: Some("https://start.ui.xboxlive.com/AddChildToFamily".to_string()),
            }
        );
    }

    #[test]
    fn an_account_without_a_copy_of_the_game_is_told_from_one_without_a_profile() {
        // The account owns no copy of the game.
        let tokens = OAuthTokens {
            access_token: "live-token".to_string(),
            refresh_token: "refresh-token".to_string(),
        };
        let client = FakeClient::new(vec![
            Answer::ok(r#"{"Token":"xbox-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"Token":"xsts-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"access_token":"game-token","token_type":"Bearer","expires_in":86400}"#),
            Answer::ok(r#"{"items":[]}"#),
            Answer::with_status(404, r#"{"error":"NOT_FOUND"}"#),
            Answer::ok(r#"{"items":[{"name":"product_minecraft"}]}"#),
        ]);

        let error = authenticate_via_live_access_token(&client, &tokens)
            .expect_err("the login should fail");
        assert_eq!(error, AuthenticationError::MinecraftLicenseNotFound);

        // The account owns a copy of the game but has no profile yet.
        let client = FakeClient::new(vec![
            Answer::ok(r#"{"Token":"xbox-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"Token":"xsts-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"access_token":"game-token","token_type":"Bearer","expires_in":86400}"#),
            Answer::ok(r#"{"items":[]}"#),
            Answer::with_status(404, r#"{"error":"NOT_FOUND"}"#),
            Answer::ok(r#"{"items":[{"name":"game_minecraft"}]}"#),
        ]);

        let error = authenticate_via_live_access_token(&client, &tokens)
            .expect_err("the login should fail");
        assert_eq!(error, AuthenticationError::MinecraftProfileNotFound);
    }

    #[test]
    fn a_store_that_is_not_a_success_is_reported_with_its_code() {
        let tokens = OAuthTokens {
            access_token: "live-token".to_string(),
            refresh_token: "refresh-token".to_string(),
        };
        let client = FakeClient::new(vec![
            Answer::ok(r#"{"Token":"xbox-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"Token":"xsts-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"access_token":"game-token","token_type":"Bearer","expires_in":86400}"#),
            Answer::with_status(500, "the server is down"),
        ]);

        let error = authenticate_via_live_access_token(&client, &tokens)
            .expect_err("the login should fail");

        assert_eq!(
            error,
            AuthenticationError::ResponseCode {
                url: MINECRAFT_STORE_URL.to_string(),
                status: 500,
                data: Some("the server is down".to_string()),
            }
        );
    }

    #[test]
    fn a_profile_that_names_a_refusal_is_reported_with_the_reason() {
        let client = FakeClient::new(vec![Answer::ok(
            r#"{"path":"/minecraft/profile","errorType":"FORBIDDEN","error":"FORBIDDEN","errorMessage":"the token is not valid","developerMessage":"use another token"}"#,
        )]);

        let error = get_minecraft_profile(&client, "Bearer game-token")
            .expect_err("the profile should not be read");

        assert_eq!(
            error,
            AuthenticationError::RemoteAuthentication {
                name: "FORBIDDEN".to_string(),
                message: "the token is not valid".to_string(),
                cause: "use another token".to_string(),
            }
        );
    }

    #[test]
    fn a_profile_without_a_name_or_an_identifier_is_not_a_profile() {
        let client = FakeClient::new(vec![Answer::ok(r#"{"name":"Notch"}"#)]);

        let error = get_minecraft_profile(&client, "Bearer game-token")
            .expect_err("the profile should not be read");

        assert!(matches!(
            error,
            AuthenticationError::ServerResponseMalformed { .. }
        ));
    }

    #[test]
    fn the_profile_names_the_identifier_and_the_name_of_the_player() {
        let client = FakeClient::new(vec![Answer::ok(&format!(
            r#"{{"id":"{PLAYER}","name":"Notch","skins":[]}}"#
        ))]);

        let profile = get_minecraft_profile(&client, "Bearer game-token")
            .expect("the profile should be read");

        assert_eq!(
            profile.id().map(|id| id.to_string()),
            Some(PLAYER.to_string())
        );
        assert_eq!(profile.name(), Some("Notch"));

        let asked = client.asked.borrow();
        assert_eq!(
            asked[0]
                .headers()
                .iter()
                .find(|(name, _)| name == "Authorization")
                .map(|(_, value)| value.as_str()),
            Some("Bearer game-token")
        );
    }

    #[test]
    fn the_token_of_a_session_that_expired_is_not_sent() {
        let session = MicrosoftSession::new(
            "Bearer",
            "game-token",
            unix_millis() - 1,
            "refresh-token",
            "the-user",
            GameProfile {
                id: Uuid::parse(PLAYER).expect("the identifier should be valid"),
                name: "Notch".to_string(),
            },
        );
        let client = FakeClient::new(vec![]);

        assert_eq!(validate(&client, &session), Ok(false));
        assert!(client.asked.borrow().is_empty());
    }

    #[test]
    fn a_token_the_servers_still_accept_makes_the_session_valid() {
        let client = FakeClient::new(vec![Answer::ok(&format!(
            r#"{{"id":"{PLAYER}","name":"Notch"}}"#
        ))]);

        assert_eq!(validate(&client, &session()), Ok(true));
        assert_eq!(client.urls(), vec![MINECRAFT_PROFILE_URL.to_string()]);
    }

    #[test]
    fn a_token_the_servers_refuse_makes_the_session_invalid() {
        let client = FakeClient::new(vec![Answer::with_status(401, "")]);

        assert_eq!(validate(&client, &session()), Ok(false));
    }

    #[test]
    fn the_session_is_written_and_read_the_way_the_launcher_of_java_writes_it() {
        let session = session();
        let mut metadata = Map::new();
        let mut private_data = Map::new();
        session.write_private_data(&mut private_data);

        assert_eq!(
            private_data["profileName"],
            Value::String("Notch".to_string())
        );
        assert_eq!(
            private_data["tokenType"],
            Value::String("Bearer".to_string())
        );
        assert_eq!(
            private_data["accessToken"],
            Value::String("game-token".to_string())
        );
        assert_eq!(
            private_data["refreshToken"],
            Value::String("refresh-token".to_string())
        );
        assert_eq!(private_data["notAfter"], Value::from(session.not_after()));
        assert_eq!(
            private_data["userid"],
            Value::String("the-user".to_string())
        );

        let read = MicrosoftSession::from_storage(&Value::Null, &private_data)
            .expect_err("a session without the identifier of the player is not read");
        assert!(matches!(
            read,
            AuthenticationError::InvalidStoredAccount { .. }
        ));

        metadata.insert("profileID".to_string(), Value::String(PLAYER.to_string()));
        let read = MicrosoftSession::from_storage(&Value::Object(metadata), &private_data)
            .expect("the session should be read");
        assert_eq!(read, session);
    }

    #[test]
    fn a_stored_session_without_a_token_is_not_read() {
        let mut metadata = Map::new();
        metadata.insert("profileID".to_string(), Value::String(PLAYER.to_string()));

        let mut private_data = Map::new();
        private_data.insert("tokenType".to_string(), Value::String("Bearer".into()));
        private_data.insert("accessToken".to_string(), Value::String("token".into()));

        let error = MicrosoftSession::from_storage(&Value::Object(metadata), &private_data)
            .expect_err("the session should not be read");
        assert_eq!(
            error,
            AuthenticationError::invalid_stored("refreshToken is missing")
        );
    }

    #[test]
    fn the_information_of_the_game_names_the_player_and_the_token() {
        let info = session().to_auth_info();

        assert_eq!(info.username, "Notch");
        assert_eq!(info.uuid.to_string(), PLAYER);
        assert_eq!(info.access_token, "game-token");
        assert_eq!(info.user_type, UserType::Msa);
        assert_eq!(info.user_properties.as_deref(), Some("{}"));
    }

    #[test]
    fn a_session_without_a_name_cannot_start_the_game() {
        assert!(!MicrosoftSession::new(
            "Bearer",
            "game-token",
            0,
            "refresh-token",
            "the-user",
            GameProfile {
                id: Uuid::parse(PLAYER).expect("the identifier should be valid"),
                name: "   ".to_string(),
            },
        )
        .has_profile_name());

        let login = MicrosoftLogin::new(MicrosoftSession::new(
            "Bearer",
            "game-token",
            0,
            "refresh-token",
            "the-user",
            GameProfile {
                id: Uuid::parse(PLAYER).expect("the identifier should be valid"),
                name: String::new(),
            },
        ));

        assert_eq!(
            login.play_offline(),
            Err(AuthenticationError::CredentialExpired {
                message: Some("Profile name is missing".to_string()),
            })
        );
    }

    #[test]
    fn a_session_of_this_run_is_used_without_asking_a_server() {
        let login = MicrosoftLogin::authenticated(session());

        assert!(login.is_authenticated());
        assert_eq!(login.profile_name(), "Notch");
        assert_eq!(login.profile_id().to_string(), PLAYER);
        assert_eq!(
            login.play_offline().map(|info| info.username),
            Ok("Notch".to_string())
        );
    }

    #[test]
    fn a_stored_session_is_checked_before_it_is_used() {
        let client = FakeClient::new(vec![Answer::ok(&format!(
            r#"{{"id":"{PLAYER}","name":"Notch"}}"#
        ))]);
        let callback = FakeCallback::new();
        let mut login = MicrosoftLogin::new(session());

        assert!(!login.is_authenticated());
        let info = login
            .log_in(&client, &callback)
            .expect("the login should succeed");

        assert_eq!(info.username, "Notch");
        assert!(login.is_authenticated());
        // The token was still accepted, so no token of Microsoft was needed.
        assert_eq!(client.urls(), vec![MINECRAFT_PROFILE_URL.to_string()]);
        assert!(callback.opened.borrow().is_empty());
    }

    #[test]
    fn a_refused_token_is_renewed_with_the_refresh_token() {
        let client = FakeClient::new(vec![
            // The token the session holds is not accepted any more.
            Answer::with_status(401, ""),
            // The refresh token is exchanged for a new token of Microsoft.
            Answer::ok(r#"{"access_token":"live-token","refresh_token":"new-refresh"}"#),
            Answer::ok(r#"{"Token":"xbox-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"Token":"xsts-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(
                r#"{"access_token":"new-game-token","token_type":"Bearer","expires_in":86400}"#,
            ),
            Answer::ok(r#"{"items":[]}"#),
            Answer::ok(&format!(r#"{{"id":"{PLAYER}","name":"Notch"}}"#)),
        ]);
        let callback = FakeCallback::new();
        let mut login = MicrosoftLogin::new(session());

        let info = login
            .log_in(&client, &callback)
            .expect("the login should succeed");

        assert_eq!(info.access_token, "new-game-token");
        assert_eq!(login.session().refresh_token(), "new-refresh");
        assert!(login.is_authenticated());
    }

    #[test]
    fn a_renewed_session_of_another_player_is_refused() {
        let other = "11111111-2222-3333-4444-555555555555";
        let client = FakeClient::new(vec![
            Answer::with_status(401, ""),
            Answer::ok(r#"{"access_token":"live-token","refresh_token":"new-refresh"}"#),
            Answer::ok(r#"{"Token":"xbox-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"Token":"xsts-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(
                r#"{"access_token":"new-game-token","token_type":"Bearer","expires_in":86400}"#,
            ),
            Answer::ok(r#"{"items":[]}"#),
            Answer::ok(&format!(r#"{{"id":"{other}","name":"Other"}}"#)),
        ]);
        let callback = FakeCallback::new();
        let mut login = MicrosoftLogin::new(session());

        let error = login
            .log_in(&client, &callback)
            .expect_err("the login should fail");

        assert_eq!(
            error,
            AuthenticationError::malformed("Selected profile changed")
        );
        assert!(!login.is_authenticated());
    }

    #[test]
    fn a_login_with_another_account_is_told_apart_from_a_login_with_the_same_one() {
        let other = "11111111-2222-3333-4444-555555555555";
        let client = FakeClient::new(vec![
            Answer::ok(
                r#"{"user_code":"ABCD","device_code":"the-device","verification_uri":"https://microsoft.com/link","expires_in":900,"interval":1}"#,
            ),
            Answer::ok(r#"{"access_token":"live-token","refresh_token":"new-refresh"}"#),
            Answer::ok(r#"{"Token":"xbox-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(r#"{"Token":"xsts-token","DisplayClaims":{"xui":[{"uhs":"the-uhs"}]}}"#),
            Answer::ok(
                r#"{"access_token":"new-game-token","token_type":"Bearer","expires_in":86400}"#,
            ),
            Answer::ok(r#"{"items":[]}"#),
            Answer::ok(&format!(r#"{{"id":"{other}","name":"Other"}}"#)),
        ]);
        let callback = FakeCallback::new();
        let mut login = MicrosoftLogin::new(session());

        let error = login
            .log_in_when_credentials_expired(&client, &callback)
            .expect_err("the login should fail");

        assert_eq!(
            error,
            AuthenticationError::WrongAccount {
                expected: Uuid::parse(PLAYER).expect("the identifier should be valid"),
                actual: Uuid::parse(other).expect("the identifier should be valid"),
            }
        );
        assert!(error.requires_login());
    }
}
