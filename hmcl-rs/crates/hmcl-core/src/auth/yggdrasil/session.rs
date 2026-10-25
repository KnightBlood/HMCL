//! The tokens of a logged in player of a server of Yggdrasil.
//!
//! A login of a server of Yggdrasil answers the token the game is started
//! with, the token that renews it, the profile the player selected and, often,
//! the profiles the player may select instead, and the properties of the user
//! of the account. The session is the whole answer, and it is what the
//! launcher stores: the public data of the account names the profile, and the
//! protected data holds the tokens, the name of the profile and its
//! properties.
//!
//! A session read from the settings knows neither the profiles the player may
//! select nor the profile the server selected, because a login does not read
//! them: the launcher writes the profile the player selected and reads it back
//! as the selected profile.

use serde_json::{Map, Value};

use crate::auth::{AuthInfo, AuthenticationError, UserType, Uuid};
use crate::util::logging;

use super::model::{GameProfile, User};

/// The tokens and the profiles of a logged in player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YggdrasilSession {
    /// The token that names the launcher to the server.
    client_token: String,
    /// The token the game is started with.
    access_token: String,
    /// The profile the server selected, if it selected one.
    selected_profile: Option<GameProfile>,
    /// The profiles the player may select, if the server named them.
    available_profiles: Option<Vec<GameProfile>>,
    /// The properties of the user of the account, if the server named them.
    user_properties: Option<std::collections::BTreeMap<String, String>>,
}

impl YggdrasilSession {
    /// Creates a session out of the answer of a server.
    ///
    /// The access token is registered with the log, so a log the player sends
    /// with a report does not hold the token of the account.
    pub fn new(
        client_token: impl Into<String>,
        access_token: impl Into<String>,
        selected_profile: Option<GameProfile>,
        available_profiles: Option<Vec<GameProfile>>,
        user_properties: Option<std::collections::BTreeMap<String, String>>,
    ) -> Self {
        let access_token = access_token.into();
        logging::register_access_token(&access_token);

        Self {
            client_token: client_token.into(),
            access_token,
            selected_profile,
            available_profiles,
            user_properties,
        }
    }

    /// Returns the token that names the launcher to the server.
    pub fn client_token(&self) -> &str {
        &self.client_token
    }

    /// Returns the token the game is started with.
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// Returns the profile the server selected.
    ///
    /// The profile is absent when the server logged the player in without
    /// selecting one of the profiles of the account, which is the case of an
    /// account that holds several profiles: the player selects one, and the
    /// session is renewed with the selection.
    pub fn selected_profile(&self) -> Option<&GameProfile> {
        self.selected_profile.as_ref()
    }

    /// Returns the profiles the player may select.
    ///
    /// The profiles are absent when the session was read from the settings of
    /// the launcher, because a login does not read them again.
    pub fn available_profiles(&self) -> Option<&[GameProfile]> {
        self.available_profiles.as_deref()
    }

    /// Returns the properties of the user of the account.
    pub fn user_properties(&self) -> Option<&std::collections::BTreeMap<String, String>> {
        self.user_properties.as_ref()
    }

    /// Returns whether the session names the player.
    ///
    /// A session whose selected profile has no name was read from a launcher
    /// that did not know the name of the player, and the game cannot be started
    /// with it.
    pub fn has_profile_name(&self) -> bool {
        self.selected_profile
            .as_ref()
            .is_some_and(|profile| !profile.name().trim().is_empty())
    }

    /// Replaces the tokens and the profiles with those of another answer.
    ///
    /// The profiles the player may select are kept when the new answer names
    /// none, which is the way the launcher of Java reads an answer: a server
    /// that selects a profile names the profiles as well, and a server that
    /// does not is asked for the selection with a new request.
    pub fn replace(&mut self, session: YggdrasilSession) {
        *self = session;
    }

    /// Reads a session out of the settings of the launcher.
    ///
    /// The identifier of the player is written with the record of the account
    /// and the name and the tokens with the protected data; the properties of
    /// the user are absent when the launcher did not read them.
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

        let client_token = required_storage_string(private_data, "clientToken")?;
        let access_token = required_storage_string(private_data, "accessToken")?;

        let user_properties = match private_data.get("userProperties") {
            Some(Value::Object(properties)) => {
                Some(super::model::read_properties_object(properties)?)
            }
            _ => None,
        };

        Ok(Self::new(
            client_token,
            access_token,
            Some(GameProfile::new(profile_id, profile_name)),
            None,
            user_properties,
        ))
    }

    /// Writes the tokens and the profile into the protected data.
    ///
    /// The entries are written in the order the launcher of Java writes them,
    /// so a file written by one launcher is read by the other. A session
    /// without a selected profile has no name to write, so it cannot be
    /// stored.
    pub fn write_private_data(&self, private_data: &mut Map<String, Value>) {
        let Some(profile) = self.selected_profile.as_ref() else {
            return;
        };

        private_data.insert(
            "clientToken".to_string(),
            Value::String(self.client_token.clone()),
        );
        private_data.insert(
            "accessToken".to_string(),
            Value::String(self.access_token.clone()),
        );
        private_data.insert(
            "profileName".to_string(),
            Value::String(profile.name().to_string()),
        );
        private_data.insert(
            "userProperties".to_string(),
            match &self.user_properties {
                Some(properties) => super::model::properties_object(properties),
                None => Value::Object(Map::new()),
            },
        );
    }

    /// Returns the information the game is started with.
    ///
    /// The properties of the user are written the way the game reads them: the
    /// properties of a user are a map of a name to a single value, and the game
    /// reads a map of a name to a list of values, which is what the properties
    /// of a profile of the game are. The kinds of the accounts of Yggdrasil are
    /// told to the game as an account of Microsoft, which is the kind the
    /// launcher of Java tells the game, because the game reads no kind of the
    /// servers of Yggdrasil.
    pub fn to_auth_info(&self) -> Result<AuthInfo, AuthenticationError> {
        let profile = self
            .selected_profile
            .as_ref()
            .ok_or(AuthenticationError::NoSelectedCharacter)?;

        Ok(AuthInfo::new(
            profile.name(),
            profile.id(),
            self.access_token.clone(),
            UserType::Msa,
            Some(user_properties_json(self.user_properties.as_ref())),
        ))
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

/// Writes the properties of a user as the game reads them.
///
/// The properties are written as a map of a name to a list of values, and a
/// launcher that knows no property writes an empty object, which is what the
/// launcher of Java writes.
pub fn user_properties_json(
    properties: Option<&std::collections::BTreeMap<String, String>>,
) -> String {
    let Some(properties) = properties else {
        return "{}".to_string();
    };

    let mut object = Map::new();
    for (name, value) in properties {
        object.insert(
            name.clone(),
            Value::Array(vec![Value::String(value.clone())]),
        );
    }
    Value::Object(object).to_string()
}

/// Reads the properties of a user out of the answer of a server.
pub(crate) fn read_user_properties(
    user: Option<User>,
) -> Option<std::collections::BTreeMap<String, String>> {
    user.and_then(|user| user.properties().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// The identifier of the player of the tests.
    fn player() -> Uuid {
        Uuid::parse("069a79f4-44e9-4726-a5be-fca90e38aaf5").expect("the identifier should be read")
    }

    /// The session of a logged in player.
    fn session() -> YggdrasilSession {
        let mut properties = BTreeMap::new();
        properties.insert("preferredLanguage".to_string(), "zh_CN".to_string());

        YggdrasilSession::new(
            "client-token",
            "access-token",
            Some(GameProfile::new(player(), "Notch")),
            Some(vec![GameProfile::new(player(), "Notch")]),
            Some(properties),
        )
    }

    #[test]
    fn the_tokens_and_the_profile_are_written_and_read() {
        let session = session();
        let mut private_data = Map::new();
        session.write_private_data(&mut private_data);

        assert_eq!(
            private_data.get("clientToken").and_then(Value::as_str),
            Some("client-token")
        );
        assert_eq!(
            private_data.get("accessToken").and_then(Value::as_str),
            Some("access-token")
        );
        assert_eq!(
            private_data.get("profileName").and_then(Value::as_str),
            Some("Notch")
        );

        let mut metadata = Map::new();
        metadata.insert("profileID".to_string(), Value::String(player().to_string()));

        let read = YggdrasilSession::from_storage(&Value::Object(metadata), &private_data)
            .expect("the session should be read");
        assert_eq!(read.access_token(), "access-token");
        assert_eq!(read.client_token(), "client-token");
        assert_eq!(
            read.selected_profile().map(GameProfile::name),
            Some("Notch")
        );
        assert_eq!(
            read.user_properties()
                .and_then(|properties| properties.get("preferredLanguage"))
                .map(String::as_str),
            Some("zh_CN")
        );
    }

    #[test]
    fn a_session_read_from_the_settings_knows_no_available_profiles() {
        let mut metadata = Map::new();
        metadata.insert("profileID".to_string(), Value::String(player().to_string()));
        let mut private_data = Map::new();
        private_data.insert(
            "clientToken".to_string(),
            Value::String("client-token".to_string()),
        );
        private_data.insert(
            "accessToken".to_string(),
            Value::String("access-token".to_string()),
        );

        let read = YggdrasilSession::from_storage(&Value::Object(metadata), &private_data)
            .expect("the session should be read");
        assert!(read.available_profiles().is_none());
        assert_eq!(read.selected_profile().map(GameProfile::name), Some(""));
        assert!(!read.has_profile_name());
    }

    #[test]
    fn a_session_without_a_token_cannot_be_read() {
        let mut metadata = Map::new();
        metadata.insert("profileID".to_string(), Value::String(player().to_string()));
        let mut private_data = Map::new();
        private_data.insert(
            "accessToken".to_string(),
            Value::String("access-token".to_string()),
        );

        let error = YggdrasilSession::from_storage(&Value::Object(metadata), &private_data)
            .expect_err("the session should not be read");
        assert_eq!(
            error,
            AuthenticationError::invalid_stored("clientToken is missing")
        );
    }

    #[test]
    fn a_session_without_a_profile_identifier_cannot_be_read() {
        let error = YggdrasilSession::from_storage(&Value::Object(Map::new()), &Map::new())
            .expect_err("the session should not be read");
        assert_eq!(
            error,
            AuthenticationError::invalid_stored("profileID is missing")
        );
    }

    #[test]
    fn the_information_of_the_game_names_the_profile_and_the_user() {
        let info = session()
            .to_auth_info()
            .expect("the information should be built");
        assert_eq!(info.username, "Notch");
        assert_eq!(info.uuid, player());
        assert_eq!(info.access_token, "access-token");
        assert_eq!(info.user_type, UserType::Msa);
        assert_eq!(
            info.user_properties.as_deref(),
            Some(r#"{"preferredLanguage":["zh_CN"]}"#)
        );
    }

    #[test]
    fn a_session_without_a_profile_has_no_information_of_the_game() {
        let session = YggdrasilSession::new("client-token", "access-token", None, None, None);
        assert_eq!(
            session.to_auth_info(),
            Err(AuthenticationError::NoSelectedCharacter)
        );
        assert!(!session.has_profile_name());
    }

    #[test]
    fn a_session_without_a_profile_is_not_written() {
        let session = YggdrasilSession::new("client-token", "access-token", None, None, None);
        let mut private_data = Map::new();
        session.write_private_data(&mut private_data);
        assert!(private_data.is_empty());
    }

    #[test]
    fn the_properties_of_a_user_are_written_as_lists() {
        assert_eq!(user_properties_json(None), "{}");

        let mut properties = BTreeMap::new();
        properties.insert("a".to_string(), "1".to_string());
        properties.insert("b".to_string(), "2".to_string());
        assert_eq!(
            user_properties_json(Some(&properties)),
            r#"{"a":["1"],"b":["2"]}"#
        );
    }
}
