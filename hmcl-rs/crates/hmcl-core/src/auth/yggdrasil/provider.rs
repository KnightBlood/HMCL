//! The addresses of the endpoints of a server of Yggdrasil.
//!
//! The servers of the game that follow the protocol of Yggdrasil (called
//! Yggdrasil after the name of the old service of Mojang) answer the same
//! requests at the same paths, so the addresses are derived from the root of
//! the API of the server. A server the player names may answer at another
//! path, so the addresses are asked through the [`YggdrasilProvider`] trait
//! rather than built by the service.

use crate::auth::{AuthenticationError, Uuid};

/// Supplies the addresses the [`YggdrasilService`] asks.
///
/// [`YggdrasilService`]: super::service::YggdrasilService
pub trait YggdrasilProvider {
    /// Returns the address that logs a player in with a name and a password.
    fn authentication_url(&self) -> Result<String, AuthenticationError>;

    /// Returns the address that renews an access token.
    fn refreshment_url(&self) -> Result<String, AuthenticationError>;

    /// Returns the address that checks an access token.
    fn validation_url(&self) -> Result<String, AuthenticationError>;

    /// Returns the address that revokes an access token.
    fn invalidation_url(&self) -> Result<String, AuthenticationError>;

    /// Returns the address the skin of a player is uploaded to.
    ///
    /// The request is a `PUT` that proves itself with the access token of the
    /// player and sends the file of the skin as a form with two parts, `model`
    /// and `file`.
    fn skin_upload_url(&self, uuid: Uuid) -> Result<String, AuthenticationError>;

    /// Returns the address that names the properties of the profile of a
    /// player.
    fn profile_properties_url(&self, uuid: Uuid) -> Result<String, AuthenticationError>;
}

/// The addresses of a server of the authlib-injector.
///
/// The server names the root of its API of Yggdrasil, which always ends with a
/// slash, and every endpoint is under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthlibInjectorProvider {
    /// The root of the API of the server, with the slash at its end.
    api_root: String,
}

impl AuthlibInjectorProvider {
    /// Creates the provider of the server whose root is given.
    ///
    /// The root is the address the player named with a slash at its end, as
    /// [`crate::setting::authlib_servers::normalize_url`] writes it.
    pub fn new(api_root: impl Into<String>) -> Self {
        Self {
            api_root: api_root.into(),
        }
    }

    /// Returns the root of the API of the server.
    pub fn api_root(&self) -> &str {
        &self.api_root
    }

    /// Returns the address of an endpoint under the root.
    fn endpoint(&self, path: &str) -> Result<String, AuthenticationError> {
        Ok(format!("{}{path}", self.api_root))
    }
}

impl YggdrasilProvider for AuthlibInjectorProvider {
    fn authentication_url(&self) -> Result<String, AuthenticationError> {
        self.endpoint("authserver/authenticate")
    }

    fn refreshment_url(&self) -> Result<String, AuthenticationError> {
        self.endpoint("authserver/refresh")
    }

    fn validation_url(&self) -> Result<String, AuthenticationError> {
        self.endpoint("authserver/validate")
    }

    fn invalidation_url(&self) -> Result<String, AuthenticationError> {
        self.endpoint("authserver/invalidate")
    }

    fn skin_upload_url(&self, uuid: Uuid) -> Result<String, AuthenticationError> {
        // The servers of the authlib-injector are told the identifier of the
        // player without the hyphens, which is the form the game reads.
        self.endpoint(&format!("api/user/profile/{}/skin", uuid.compact()))
    }

    fn profile_properties_url(&self, uuid: Uuid) -> Result<String, AuthenticationError> {
        self.endpoint(&format!(
            "sessionserver/session/minecraft/profile/{}",
            uuid.compact()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The identifier of a player of the tests.
    fn player() -> Uuid {
        Uuid::parse("069a79f4-44e9-4726-a5be-fca90e38aaf5").expect("the identifier should be read")
    }

    #[test]
    fn the_endpoints_of_a_server_are_under_its_root() {
        let provider = AuthlibInjectorProvider::new("https://example.com/api/yggdrasil/");

        assert_eq!(
            provider.authentication_url().expect("the address"),
            "https://example.com/api/yggdrasil/authserver/authenticate"
        );
        assert_eq!(
            provider.refreshment_url().expect("the address"),
            "https://example.com/api/yggdrasil/authserver/refresh"
        );
        assert_eq!(
            provider.validation_url().expect("the address"),
            "https://example.com/api/yggdrasil/authserver/validate"
        );
        assert_eq!(
            provider.invalidation_url().expect("the address"),
            "https://example.com/api/yggdrasil/authserver/invalidate"
        );
    }

    #[test]
    fn the_endpoints_of_a_player_name_the_identifier_without_the_hyphens() {
        let provider = AuthlibInjectorProvider::new("https://example.com/api/yggdrasil/");

        assert_eq!(
            provider.skin_upload_url(player()).expect("the address"),
            "https://example.com/api/yggdrasil/api/user/profile/069a79f444e94726a5befca90e38aaf5/skin"
        );
        assert_eq!(
            provider
                .profile_properties_url(player())
                .expect("the address"),
            "https://example.com/api/yggdrasil/sessionserver/session/minecraft/profile/069a79f444e94726a5befca90e38aaf5"
        );
    }

    #[test]
    fn the_provider_keeps_the_root_it_was_given() {
        let provider = AuthlibInjectorProvider::new("https://example.com/api/yggdrasil/");
        assert_eq!(provider.api_root(), "https://example.com/api/yggdrasil/");
        assert_eq!(provider.clone(), provider);
    }
}
