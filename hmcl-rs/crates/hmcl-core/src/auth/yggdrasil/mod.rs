//! The login to a server that follows the protocol of Yggdrasil.
//!
//! The launcher logs a player in to the servers of Microsoft through the chain
//! of [`microsoft`](super::microsoft); the players of the servers of the
//! authlib-injector log in to the server the player named instead. Those
//! servers speak the protocol of Yggdrasil: the protocol of the old service of
//! Mojang, which the game reads, so the launcher keeps speaking it.
//!
//! The protocol is four requests that answer JSON and one that uploads a file.
//! [`service`] sends them, [`model`] holds what they answer, [`session`] holds
//! the tokens of a logged in player and the way they are stored, and
//! [`account`] is the account of a player as the launcher holds it while it
//! runs.
//!
//! As in the rest of the core, no connection is opened here: the requests are
//! sent by the [`HttpClient`] the program gives, so the whole login is tested
//! against the answers of a server that is not there.
//!
//! [`HttpClient`]: crate::util::http_request::HttpClient

pub mod account;
pub mod model;
pub mod provider;
pub mod service;
pub mod session;

pub use account::{CharacterSelector, YggdrasilAccount};
pub use model::{CompleteGameProfile, GameProfile, Texture, TextureModel, TextureType, User};
pub use provider::{AuthlibInjectorProvider, YggdrasilProvider};
pub use service::YggdrasilService;
pub use session::YggdrasilSession;
