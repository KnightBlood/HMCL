//! The accounts of the launcher.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::account_factory::AccountFactoryKind;
use super::account_id::AccountId;
use super::uuid::Uuid;

/// How the skin of a player is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinModel {
    /// The arms of the skin are as wide as the legs.
    #[default]
    Default,
    /// The arms of the skin are thin.
    Slim,
}

/// The skin of a player that plays without an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skin {
    /// How the skin is drawn.
    pub model: SkinModel,
    /// The address of the file of the skin, or its digest.
    pub texture: String,
}

/// An account of a player that plays without an account.
///
/// The game is not told who the player is, so the identifier of the player is
/// derived from the name of the player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineAccount {
    /// The name of the player.
    pub profile_name: String,
    /// The identifier of the player.
    #[serde(rename = "profileID", alias = "profileId")]
    pub profile_id: Uuid,
    /// The skin the player chose, if the player chose one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin: Option<Skin>,
}

/// An account of Microsoft.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrosoftAccount {
    /// The name of the player, as it was last seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    /// The identifier of the player, as it was last seen.
    #[serde(
        default,
        rename = "profileID",
        alias = "profileId",
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_id: Option<Uuid>,
    /// Whether the account was authenticated and its tokens are held.
    pub authenticated: bool,
}

/// An account of a server that follows the authentication protocol of the
/// game, as the servers of the launcher do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthlibInjectorAccount {
    /// The address of the server.
    pub server_url: String,
    /// The name the player logs in with.
    pub login_name: String,
    /// The name of the player, as it was last seen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    /// The identifier of the player, as it was last seen.
    #[serde(
        default,
        rename = "profileID",
        alias = "profileId",
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_id: Option<Uuid>,
}

/// How an account proves who the player is.
///
/// The names of the variants in the settings are the names the launcher of
/// Java writes, which [`AccountFactoryKind`](super::AccountFactoryKind) holds:
/// a record of a server of the authlib-injector is written as
/// `authlibInjector`. The name this launcher wrote before it read the records
/// of Java is read as well, so a record of an older run is not lost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AccountKind {
    /// A player that plays without an account.
    #[serde(rename = "offline")]
    Offline(OfflineAccount),
    /// An account of Microsoft.
    #[serde(rename = "microsoft")]
    Microsoft(MicrosoftAccount),
    /// An account of a server that follows the authentication protocol of
    /// the game.
    #[serde(rename = "authlibInjector", alias = "authlib-injector")]
    AuthlibInjector(AuthlibInjectorAccount),
}

/// An account record of the launcher.
///
/// The record holds the identifier the user chose and what the launcher knows
/// about the player; the credentials are not part of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The identifier of the record.
    pub id: AccountId,
    /// Whether the account is stored with the game directory instead of with
    /// the launcher.
    #[serde(default)]
    pub portable: bool,
    /// How the account proves who the player is.
    #[serde(flatten)]
    pub kind: AccountKind,
}

impl Account {
    /// Creates an account record.
    pub fn new(id: AccountId, kind: AccountKind) -> Self {
        Self {
            id,
            portable: false,
            kind,
        }
    }

    /// Creates the record of a player that plays without an account.
    pub fn offline(name: impl Into<String>) -> Self {
        let profile_name = name.into();
        let profile_id = Uuid::offline(&profile_name);

        Self::new(
            AccountId::generate(),
            AccountKind::Offline(OfflineAccount {
                profile_name,
                profile_id,
                skin: None,
            }),
        )
    }

    /// Returns the name of the player, as the launcher shows it.
    pub fn profile_name(&self) -> &str {
        match &self.kind {
            AccountKind::Offline(account) => &account.profile_name,
            AccountKind::Microsoft(account) => account.profile_name.as_deref().unwrap_or_default(),
            AccountKind::AuthlibInjector(account) => account
                .profile_name
                .as_deref()
                .unwrap_or(&account.login_name),
        }
    }

    /// Returns the identifier of the player, if the launcher knows it.
    pub fn profile_id(&self) -> Option<Uuid> {
        match &self.kind {
            AccountKind::Offline(account) => Some(account.profile_id),
            AccountKind::Microsoft(account) => account.profile_id,
            AccountKind::AuthlibInjector(account) => account.profile_id,
        }
    }

    /// Returns the name of the kind of the account, as the settings of the
    /// launcher write it.
    ///
    /// The name is the one of the factory of the kind, so a record is written
    /// with the name the launcher of Java reads it back by.
    pub const fn kind_id(&self) -> &'static str {
        AccountFactoryKind::of_kind(&self.kind).login_type_id()
    }

    /// Writes the public data of the account into the settings of the
    /// launcher.
    ///
    /// The data is stored in the list of the accounts of the launcher and
    /// must not hold credentials.
    pub fn write_metadata(&self, metadata: &mut Map<String, Value>) {
        metadata.insert("accountID".to_string(), Value::String(self.id.to_string()));
        metadata.insert(
            "type".to_string(),
            Value::String(self.kind_id().to_string()),
        );
        metadata.insert("portable".to_string(), Value::Bool(self.portable));
    }

    /// Reads the identifier of the record from the settings of the launcher.
    pub fn read_account_id(metadata: &Map<String, Value>) -> Option<AccountId> {
        metadata
            .get("accountID")
            .and_then(Value::as_str)
            .and_then(|text| AccountId::parse(text).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_of_a_player_without_an_account_is_derived_from_the_name() {
        let account = Account::offline("Notch");

        assert_eq!(account.profile_name(), "Notch");
        assert_eq!(
            account.profile_id().map(|uuid| uuid.to_string()),
            Some("b50ad385-829d-3141-a216-7e7d7539ba7f".to_string())
        );
        assert_eq!(account.kind_id(), "offline");
    }

    #[test]
    fn the_records_of_the_other_kinds_are_named() {
        let microsoft = Account::new(
            AccountId::generate(),
            AccountKind::Microsoft(MicrosoftAccount {
                profile_name: Some("Notch".to_string()),
                profile_id: None,
                authenticated: true,
            }),
        );
        assert_eq!(microsoft.profile_name(), "Notch");
        assert_eq!(microsoft.profile_id(), None);
        assert_eq!(microsoft.kind_id(), "microsoft");

        let authlib = Account::new(
            AccountId::generate(),
            AccountKind::AuthlibInjector(AuthlibInjectorAccount {
                server_url: "https://example.com/api/yggdrasil".to_string(),
                login_name: "notch@example.com".to_string(),
                profile_name: None,
                profile_id: None,
            }),
        );
        assert_eq!(authlib.profile_name(), "notch@example.com");
        assert_eq!(authlib.kind_id(), "authlibInjector");
    }

    #[test]
    fn the_metadata_of_an_account_is_written_and_read() {
        let account = Account::offline("Notch");
        let mut metadata = Map::new();
        account.write_metadata(&mut metadata);

        assert_eq!(
            metadata.get("accountID").and_then(Value::as_str),
            Some(account.id.to_string().as_str())
        );
        assert_eq!(
            metadata.get("type").and_then(Value::as_str),
            Some("offline")
        );
        assert_eq!(
            metadata.get("portable").and_then(Value::as_bool),
            Some(false)
        );

        assert_eq!(Account::read_account_id(&metadata), Some(account.id));
        assert_eq!(Account::read_account_id(&Map::new()), None);
    }

    #[test]
    fn the_records_are_read_and_written() {
        let account = Account::offline("Notch");
        let json = serde_json::to_string(&account).expect("the account should be written");
        assert!(json.contains("\"type\":\"offline\""));

        let read: Account = serde_json::from_str(&json).expect("the account should be read");
        assert_eq!(read, account);
    }

    #[test]
    fn a_record_written_before_the_names_of_the_launcher_of_java_are_read() {
        // The launcher wrote `authlib-injector` before it read the records of
        // Java, which name the kind `authlibInjector`; the older record reads
        // as the same kind of account rather than being lost.
        let read: AccountKind = serde_json::from_str(
            r#"{"type":"authlib-injector","serverUrl":"https://example.com/api/yggdrasil/","loginName":"notch@example.com"}"#,
        )
        .expect("the record should be read");

        assert_eq!(
            crate::auth::AccountFactoryKind::of_kind(&read).login_type_id(),
            "authlibInjector"
        );
        let AccountKind::AuthlibInjector(account) = read else {
            panic!("the record should name a server of the authlib-injector");
        };
        assert_eq!(account.login_name, "notch@example.com");
    }

    #[test]
    fn the_skins_are_read_and_written() {
        let skin = Skin {
            model: SkinModel::Slim,
            texture: "https://example.com/skin.png".to_string(),
        };
        let json = serde_json::to_string(&skin).expect("the skin should be written");
        assert!(json.contains("\"model\":\"slim\""));

        let read: Skin = serde_json::from_str(&json).expect("the skin should be read");
        assert_eq!(read, skin);
    }
}
