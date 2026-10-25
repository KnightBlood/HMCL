//! The failures of the login of an account.
//!
//! A login tells the player what went wrong, so the reasons the launcher of
//! Java tells apart are told apart here as well: the network is down, the
//! server answered something the launcher cannot read, the credentials of the
//! account expired, or the servers refused the login and said why. The pages
//! of the launcher show a text of their own for each reason, so the reason is
//! a value and not a text.

use std::fmt;

use crate::util::http_request::RequestFailure;

use super::uuid::Uuid;

/// The reason a login of an account failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticationError {
    /// The server could not be reached or the connection broke.
    ServerDisconnect {
        /// The message of the failure of the connection.
        message: String,
    },
    /// The server answered something the launcher cannot read.
    ServerResponseMalformed {
        /// What is wrong with the answer.
        message: String,
    },
    /// The stored credentials of the account are not accepted any more, so the
    /// player has to log in again.
    CredentialExpired {
        /// The reason the credentials are rejected, if the server named one.
        message: Option<String>,
    },
    /// The player did not select a character before the login ended.
    NoSelectedCharacter,
    /// The account holds no character the launcher may log in with.
    NoCharacter,
    /// The character the account selected does not exist any more.
    CharacterDeleted,
    /// The account is not logged in.
    NotLoggedIn,
    /// The server of the accounts refused the login and said why.
    RemoteAuthentication {
        /// The name of the reason, such as `invalid_grant`.
        name: String,
        /// The message of the server.
        message: String,
        /// The cause the server named, which is often empty.
        cause: String,
    },
    /// The credentials belong to another account than the one that was
    /// refreshed.
    WrongAccount {
        /// The identifier of the account the launcher expected.
        expected: Uuid,
        /// The identifier of the account the servers answered with.
        actual: Uuid,
    },
    /// This build of the launcher does not know how to log in with a Microsoft
    /// account, because it was built without the identifier of its
    /// application.
    MicrosoftNotSupported,
    /// Xbox Live refused the login and named a code the launcher shows the
    /// player.
    XboxAuthorization {
        /// The code of the refusal, such as
        /// [`XBOX_BANNED`](super::microsoft::XBOX_BANNED).
        code: i64,
        /// The page the player may open to solve the refusal, if the server
        /// named one.
        redirect: Option<String>,
    },
    /// Xbox Live refused the request of the launcher itself.
    XboxBadRequest,
    /// The account owns the game but has no profile of the game yet.
    MinecraftProfileNotFound,
    /// The account does not own the game.
    MinecraftLicenseNotFound,
    /// The answer of Xbox Live holds no user hash.
    NoUserHash,
    /// The page of the login stopped before the player logged in.
    ///
    /// The page is the local server of the launcher, so the message is written
    /// by the launcher and not by a server of Microsoft.
    LoginAborted {
        /// The message of the failure, as the page of the login wrote it.
        message: String,
    },
    /// A server answered with a status the launcher did not expect.
    ResponseCode {
        /// The address the launcher asked.
        url: String,
        /// The code of the answer.
        status: u16,
        /// The body of the answer, when the launcher read one.
        data: Option<String>,
    },
    /// The stored data of the account does not hold what a login needs.
    InvalidStoredAccount {
        /// What is missing or malformed.
        message: String,
    },
    /// The file of the authlib-injector could not be read or written, so the
    /// game cannot be started with an account of a server of the
    /// authlib-injector.
    InjectorDownload {
        /// The message of the failure of the file.
        message: String,
    },
}

impl AuthenticationError {
    /// Creates a failure of the connection with a message.
    pub fn disconnect(message: impl Into<String>) -> Self {
        Self::ServerDisconnect {
            message: message.into(),
        }
    }

    /// Creates a failure that reports an answer the launcher cannot read.
    pub fn malformed(message: impl Into<String>) -> Self {
        Self::ServerResponseMalformed {
            message: message.into(),
        }
    }

    /// Creates a failure that reports stored data a login cannot use.
    pub fn invalid_stored(message: impl Into<String>) -> Self {
        Self::InvalidStoredAccount {
            message: message.into(),
        }
    }

    /// Creates a failure that reports the end of the page of the login.
    pub fn aborted(message: impl Into<String>) -> Self {
        Self::LoginAborted {
            message: message.into(),
        }
    }

    /// Returns whether the player has to log in again for the account to work.
    ///
    /// The pages of the launcher ask this to tell a login that failed for a
    /// moment from a login that will keep failing until the player logs in.
    pub const fn requires_login(&self) -> bool {
        matches!(
            self,
            Self::CredentialExpired { .. }
                | Self::NotLoggedIn
                | Self::NoSelectedCharacter
                | Self::WrongAccount { .. }
        )
    }
}

/// Writes the message of a refusal of a server.
///
/// The message is the name of the reason, and then the message and the cause
/// the server named, which is the way the launcher of Java writes it: a server
/// that names only the reason is written without a colon.
fn remote_message(name: &str, message: &str, cause: &str) -> String {
    let mut text = String::from(name);
    if !message.is_empty() {
        text.push_str(": ");
        text.push_str(message);
    }
    if !cause.is_empty() {
        text.push_str(": ");
        text.push_str(cause);
    }
    text
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServerDisconnect { message } => {
                write!(
                    formatter,
                    "Failed to connect to the authentication server: {message}"
                )
            }
            Self::ServerResponseMalformed { message } => {
                write!(
                    formatter,
                    "The authentication server answered with a response the launcher cannot read: {message}"
                )
            }
            Self::CredentialExpired { message } => match message {
                Some(message) => write!(
                    formatter,
                    "The credentials of the account expired: {message}"
                ),
                None => formatter.write_str("The credentials of the account expired"),
            },
            Self::NoSelectedCharacter => formatter.write_str("No character was selected"),
            Self::NoCharacter => formatter.write_str("The account holds no character"),
            Self::CharacterDeleted => formatter.write_str("The character has been deleted"),
            Self::NotLoggedIn => formatter.write_str("The account is not logged in"),
            Self::RemoteAuthentication {
                name,
                message,
                cause,
            } => formatter.write_str(&remote_message(name, message, cause)),
            Self::WrongAccount { expected, actual } => {
                write!(formatter, "Expected account {expected}, but found {actual}")
            }
            Self::MicrosoftNotSupported => formatter
                .write_str("This build of the launcher cannot log in with a Microsoft account"),
            Self::XboxAuthorization { code, redirect } => match redirect {
                Some(redirect) => write!(
                    formatter,
                    "Xbox Live refused the login with the error code {code}, see {redirect}"
                ),
                None => write!(
                    formatter,
                    "Xbox Live refused the login with the error code {code}"
                ),
            },
            Self::XboxBadRequest => {
                formatter.write_str("Xbox Live refused the request of the launcher")
            }
            Self::MinecraftProfileNotFound => {
                formatter.write_str("The account has no profile of Minecraft: Java Edition yet")
            }
            Self::MinecraftLicenseNotFound => {
                formatter.write_str("The account does not own Minecraft: Java Edition")
            }
            Self::NoUserHash => formatter.write_str("The answer of Xbox Live holds no user hash"),
            Self::LoginAborted { message } => formatter.write_str(message),
            Self::ResponseCode { url, status, data } => match data {
                Some(data) => write!(
                    formatter,
                    "Unable to request url {url}, response code: {status}, data: {data}"
                ),
                None => write!(
                    formatter,
                    "Unable to request url {url}, response code: {status}"
                ),
            },
            Self::InvalidStoredAccount { message } => {
                write!(formatter, "The stored account cannot be used: {message}")
            }
            Self::InjectorDownload { message } => write!(
                formatter,
                "Failed to download the authlib-injector. Please check your network, or try switching to a different download source. ({message})"
            ),
        }
    }
}

impl std::error::Error for AuthenticationError {}

impl From<RequestFailure> for AuthenticationError {
    /// Reads the failure of a request as a failure of the login.
    ///
    /// A connection that broke is reported as the server being away, and a
    /// status the launcher did not expect keeps its address and its code, so
    /// the player is told which server refused the login.
    fn from(failure: RequestFailure) -> Self {
        match failure {
            RequestFailure::Network { url, message } => Self::ServerDisconnect {
                message: format!("{url}: {message}"),
            },
            RequestFailure::Status { url, status, body } => Self::ResponseCode {
                url,
                status,
                data: body,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_of_a_server_is_written_with_the_parts_it_has() {
        let error = AuthenticationError::RemoteAuthentication {
            name: "invalid_grant".to_string(),
            message: "The credentials are invalid".to_string(),
            cause: String::new(),
        };
        assert_eq!(
            error.to_string(),
            "invalid_grant: The credentials are invalid"
        );

        let error = AuthenticationError::RemoteAuthentication {
            name: "invalid_grant".to_string(),
            message: String::new(),
            cause: "AADSTS70000".to_string(),
        };
        assert_eq!(error.to_string(), "invalid_grant: AADSTS70000");

        let error = AuthenticationError::RemoteAuthentication {
            name: "invalid_grant".to_string(),
            message: String::new(),
            cause: String::new(),
        };
        assert_eq!(error.to_string(), "invalid_grant");
    }

    #[test]
    fn a_wrong_account_names_both_accounts() {
        let expected = Uuid::offline("Notch");
        let actual = Uuid::offline("Herobrine");
        let error = AuthenticationError::WrongAccount { expected, actual };

        let text = error.to_string();
        assert!(text.starts_with("Expected account "), "{text}");
        assert!(text.contains(&expected.to_string()), "{text}");
        assert!(text.contains(&actual.to_string()), "{text}");
    }

    #[test]
    fn a_status_that_is_not_expected_keeps_its_address_and_its_code() {
        let error = AuthenticationError::ResponseCode {
            url: "https://api.minecraftservices.com/minecraft/profile".to_string(),
            status: 403,
            data: None,
        };
        assert_eq!(
            error.to_string(),
            "Unable to request url https://api.minecraftservices.com/minecraft/profile, response code: 403"
        );

        let error = AuthenticationError::ResponseCode {
            url: "https://example.com/a".to_string(),
            status: 500,
            data: Some("boom".to_string()),
        };
        assert!(error.to_string().ends_with("data: boom"));
    }

    #[test]
    fn the_failures_of_a_request_are_read_as_failures_of_a_login() {
        let error = AuthenticationError::from(RequestFailure::Network {
            url: "https://example.com/a".to_string(),
            message: "the connection broke".to_string(),
        });
        assert!(matches!(
            error,
            AuthenticationError::ServerDisconnect { .. }
        ));
        assert!(error.to_string().contains("the connection broke"));

        let error = AuthenticationError::from(RequestFailure::Status {
            url: "https://example.com/a".to_string(),
            status: 404,
            body: Some("gone".to_string()),
        });
        assert_eq!(
            error,
            AuthenticationError::ResponseCode {
                url: "https://example.com/a".to_string(),
                status: 404,
                data: Some("gone".to_string()),
            }
        );
    }

    #[test]
    fn the_failures_that_need_a_login_are_told_apart() {
        assert!(AuthenticationError::CredentialExpired { message: None }.requires_login());
        assert!(AuthenticationError::NotLoggedIn.requires_login());
        assert!(!AuthenticationError::NoCharacter.requires_login());
        assert!(!AuthenticationError::disconnect("away").requires_login());
    }

    #[test]
    fn the_failure_of_the_injector_is_written_with_the_text_of_the_page() {
        // The player is told to check the network, which is what the page of
        // the launcher of Java shows for this failure; the reason the file
        // could not be written follows it.
        let error = AuthenticationError::InjectorDownload {
            message: "the file is not an archive".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Failed to download the authlib-injector. Please check your network, or try switching to a different download source. (the file is not an archive)"
        );
        assert!(!error.requires_login());
    }
}
