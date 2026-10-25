//! The accounts of the players.
//!
//! The launcher starts the game for a player; the models of the accounts hold
//! what the launcher knows about the player and are written into the settings
//! of the launcher.

pub mod account;
pub mod account_factory;
pub mod account_id;
pub mod auth_info;
pub mod authlibinjector;
pub mod error;
pub mod microsoft;
pub mod oauth;
pub mod uuid;
pub mod yggdrasil;

pub use account::{
    Account, AccountKind, AuthlibInjectorAccount, MicrosoftAccount, OfflineAccount, Skin, SkinModel,
};
pub use account_factory::{AccountFactoryKind, AccountLoginType};
pub use account_id::AccountId;
pub use auth_info::{AuthInfo, UserType};
pub use error::AuthenticationError;
pub use microsoft::{GameProfile, MicrosoftLogin, MicrosoftSession};
pub use oauth::{GrantFlow, OAuth, OAuthCallback, OAuthSession, OAuthTokens};
pub use uuid::{Uuid, UuidParseError};
