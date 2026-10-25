//! What the launcher has to know about a kind of account before it logs in.
//!
//! The pages of the login ask the factory of the account what it needs: an
//! account of Microsoft checks the credentials of the player in its own way and
//! needs neither a name nor a password, a player without an account needs a
//! name, and an account of a server of the authlib-injector needs both.
//!
//! The factory of a kind is also what names the kind in the settings of the
//! launcher, so the name a record is written with and the name the launcher
//! reads it back by come from the same place.

use super::account::AccountKind;

/// What a factory needs before it can log a player in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountLoginType {
    /// The factory needs neither a name nor a password, because it checks the
    /// credentials in its own way.
    None,
    /// The factory needs the name of the player only.
    Username,
    /// The factory needs the name and the password of the player.
    UsernamePassword,
}

impl AccountLoginType {
    /// Returns whether the page of the login asks the player for a name.
    pub const fn requires_username(self) -> bool {
        matches!(self, Self::Username | Self::UsernamePassword)
    }

    /// Returns whether the page of the login asks the player for a password.
    pub const fn requires_password(self) -> bool {
        matches!(self, Self::UsernamePassword)
    }
}

/// The kinds of accounts the launcher can log in.
///
/// The launcher of Java keeps a factory object for every kind and a map from
/// the name of a kind to its factory; the launcher reads a record by the name
/// it was written with, so the name is what this type holds. A bound factory
/// of a server of the authlib-injector is a factory of the same kind, so it is
/// written with the same name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountFactoryKind {
    /// The accounts of the players that play without an account.
    Offline,
    /// The accounts of Microsoft.
    Microsoft,
    /// The accounts of the servers of the authlib-injector.
    AuthlibInjector,
}

impl AccountFactoryKind {
    /// Returns the kind of the factory that logs an account in.
    pub const fn of_kind(kind: &AccountKind) -> Self {
        match kind {
            AccountKind::Offline(_) => Self::Offline,
            AccountKind::Microsoft(_) => Self::Microsoft,
            AccountKind::AuthlibInjector(_) => Self::AuthlibInjector,
        }
    }

    /// Returns the kind of the factory a record names.
    ///
    /// The names are the ones the settings of the launcher write, which are
    /// not the names of the variants: the launcher of Java writes
    /// `authlibInjector` for the servers of the authlib-injector. A name the
    /// launcher does not know is not a kind of account, so it is not reported
    /// as one.
    pub fn from_login_type(login_type: &str) -> Option<Self> {
        match login_type {
            "offline" => Some(Self::Offline),
            "microsoft" => Some(Self::Microsoft),
            "authlibInjector" => Some(Self::AuthlibInjector),
            _ => None,
        }
    }

    /// Returns the name of the kind, as the settings of the launcher write it.
    pub const fn login_type_id(self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Microsoft => "microsoft",
            Self::AuthlibInjector => "authlibInjector",
        }
    }

    /// Returns what the factory of the kind needs before it logs a player in.
    pub const fn login_type(self) -> AccountLoginType {
        match self {
            // An account of Microsoft opens the page of its own login.
            Self::Microsoft => AccountLoginType::None,
            // A player without an account is named and nothing else.
            Self::Offline => AccountLoginType::Username,
            Self::AuthlibInjector => AccountLoginType::UsernamePassword,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::{Map, Value};

    use crate::auth::account::{AuthlibInjectorAccount, MicrosoftAccount, OfflineAccount};
    use crate::auth::AccountId;

    #[test]
    fn a_page_of_the_login_asks_for_what_the_kind_needs() {
        let microsoft = AccountFactoryKind::Microsoft.login_type();
        assert!(!microsoft.requires_username());
        assert!(!microsoft.requires_password());

        let offline = AccountFactoryKind::Offline.login_type();
        assert!(offline.requires_username());
        assert!(!offline.requires_password());

        let authlib = AccountFactoryKind::AuthlibInjector.login_type();
        assert!(authlib.requires_username());
        assert!(authlib.requires_password());
    }

    #[test]
    fn a_kind_is_named_the_way_the_launcher_of_java_names_it() {
        assert_eq!(AccountFactoryKind::Offline.login_type_id(), "offline");
        assert_eq!(AccountFactoryKind::Microsoft.login_type_id(), "microsoft");
        assert_eq!(
            AccountFactoryKind::AuthlibInjector.login_type_id(),
            "authlibInjector"
        );
    }

    #[test]
    fn the_kinds_of_the_records_are_found_by_the_names_they_are_written_with() {
        let offline = AccountKind::Offline(OfflineAccount {
            profile_name: "Notch".to_string(),
            profile_id: crate::auth::Uuid::offline("Notch"),
            skin: None,
        });
        let microsoft = AccountKind::Microsoft(MicrosoftAccount::default());
        let authlib = AccountKind::AuthlibInjector(AuthlibInjectorAccount {
            server_url: "https://example.com/api/yggdrasil/".to_string(),
            login_name: "notch@example.com".to_string(),
            profile_name: None,
            profile_id: None,
        });

        for (kind, name) in [
            (offline, "offline"),
            (microsoft, "microsoft"),
            (authlib, "authlibInjector"),
        ] {
            let factory = AccountFactoryKind::of_kind(&kind);
            assert_eq!(factory.login_type_id(), name);
            assert_eq!(AccountFactoryKind::from_login_type(name), Some(factory));
        }
    }

    #[test]
    fn a_name_the_launcher_does_not_know_is_not_a_kind() {
        assert_eq!(
            AccountFactoryKind::from_login_type("authlib-injector"),
            None
        );
        assert_eq!(AccountFactoryKind::from_login_type(""), None);
        assert_eq!(AccountFactoryKind::from_login_type("Offline"), None);
    }

    #[test]
    fn a_record_of_an_account_is_written_with_the_name_of_its_factory() {
        let account = crate::auth::Account::new(
            AccountId::generate(),
            AccountKind::AuthlibInjector(AuthlibInjectorAccount {
                server_url: "https://example.com/api/yggdrasil/".to_string(),
                login_name: "notch@example.com".to_string(),
                profile_name: None,
                profile_id: None,
            }),
        );

        let mut metadata = Map::new();
        account.write_metadata(&mut metadata);

        // The launcher of Java writes `authlibInjector`, and the record reads
        // back as the same kind of account.
        assert_eq!(
            metadata.get("type").and_then(Value::as_str),
            Some("authlibInjector")
        );
    }
}
