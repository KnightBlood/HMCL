//! The information the game is started with for a player.

use serde::{Deserialize, Serialize};

use super::uuid::Uuid;

/// The kind of the account the player logged in with.
///
/// The game is told the kind of the account so it knows which services it may
/// ask for the skin of the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserType {
    /// An account of Microsoft.
    #[default]
    Msa,
    /// An account of the old service of Mojang.
    Mojang,
    /// An account of the old versions of the game.
    Legacy,
}

impl UserType {
    /// Returns the name of the kind, as the game reads it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Msa => "msa",
            Self::Mojang => "mojang",
            Self::Legacy => "legacy",
        }
    }

    /// Returns the kind with the name.
    ///
    /// The names of the kinds are the ones the game reads; a name the
    /// launcher does not know is read as the kind of Microsoft, which is the
    /// kind the current versions of the game use.
    pub fn of(name: &str) -> Self {
        match name {
            "mojang" => Self::Mojang,
            "legacy" => Self::Legacy,
            _ => Self::Msa,
        }
    }
}

/// The information the game is started with for a player.
///
/// The access token is written into the arguments of the game; it must not be
/// written into the settings of the launcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthInfo {
    /// The name of the player.
    pub username: String,
    /// The identifier of the player.
    pub uuid: Uuid,
    /// The token the game proves the account with.
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// The kind of the account.
    #[serde(rename = "userType")]
    pub user_type: UserType,
    /// The properties of the player, in JSON.
    #[serde(
        default,
        rename = "userProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub user_properties: Option<String>,
    /// The arguments the game is started with for this account.
    ///
    /// An account of a server of the authlib-injector names the agent the game
    /// talks to the server with, which the arguments of the game do not hold.
    /// The arguments are added to the ones of the virtual machine, before the
    /// main class, which is the place Java writes them as well.
    ///
    /// The arguments are not written into the settings of the launcher,
    /// because they name the address of the server the account belongs to and
    /// the file of the agent of this machine.
    #[serde(skip)]
    pub launch_arguments: Option<Vec<String>>,
}

impl AuthInfo {
    /// Creates the information of a player.
    pub fn new(
        username: impl Into<String>,
        uuid: Uuid,
        access_token: impl Into<String>,
        user_type: UserType,
        user_properties: Option<String>,
    ) -> Self {
        Self {
            username: username.into(),
            uuid,
            access_token: access_token.into(),
            user_type,
            user_properties,
            launch_arguments: None,
        }
    }

    /// Returns the information with the arguments the game is started with.
    ///
    /// An account whose game needs an agent adds the arguments of the agent;
    /// the accounts that need none leave the field empty, which is what the
    /// launcher of Java does by returning `null` from
    /// `AuthInfo.getLaunchArguments`.
    pub fn with_launch_arguments(mut self, launch_arguments: Vec<String>) -> Self {
        self.launch_arguments = Some(launch_arguments);
        self
    }

    /// Creates the information of a player that plays without an account.
    ///
    /// The token of a player that plays without an account is not checked by
    /// the game, so the information holds no token.
    pub fn offline(name: impl Into<String>) -> Self {
        let username = name.into();
        let uuid = Uuid::offline(&username);
        Self::new(username, uuid, "", UserType::Legacy, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_of_the_accounts_are_named() {
        assert_eq!(UserType::Msa.as_str(), "msa");
        assert_eq!(UserType::Mojang.as_str(), "mojang");
        assert_eq!(UserType::Legacy.as_str(), "legacy");
        assert_eq!(UserType::of("mojang"), UserType::Mojang);
        assert_eq!(UserType::of("something"), UserType::Msa);
    }

    #[test]
    fn the_information_of_a_player_is_read_and_written() {
        let info = AuthInfo::new(
            "Notch",
            Uuid::offline("Notch"),
            "token",
            UserType::Msa,
            Some("[{\"name\":\"textures\"}]".to_string()),
        );

        let json = serde_json::to_string(&info).expect("the information should be written");
        assert!(json.contains("\"accessToken\":\"token\""));
        assert!(json.contains("\"userType\":\"msa\""));

        let read: AuthInfo = serde_json::from_str(&json).expect("the information should be read");
        assert_eq!(read, info);
    }

    #[test]
    fn the_information_of_a_player_without_an_account_is_derived_from_the_name() {
        let info = AuthInfo::offline("Notch");
        assert_eq!(info.username, "Notch");
        assert_eq!(
            info.uuid.to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(info.user_type, UserType::Legacy);
        assert!(info.access_token.is_empty());
        assert_eq!(info.launch_arguments, None);
    }

    #[test]
    fn the_arguments_of_an_account_are_kept_but_not_written() {
        let info = AuthInfo::offline("Notch").with_launch_arguments(vec![
            "-javaagent:authlib-injector.jar=https://example.com/".to_string(),
        ]);
        assert_eq!(
            info.launch_arguments.as_deref(),
            Some(["-javaagent:authlib-injector.jar=https://example.com/".to_string()].as_slice())
        );

        // The arguments name a file of this machine, so they are not written
        // into the settings of the launcher.
        let json = serde_json::to_string(&info).expect("the information should be written");
        assert!(!json.contains("javaagent"), "{json}");
        let read: AuthInfo = serde_json::from_str(&json).expect("the information should be read");
        assert_eq!(read.launch_arguments, None);
    }
}
