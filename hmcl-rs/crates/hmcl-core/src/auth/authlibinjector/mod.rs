//! The accounts of the servers of the authlib-injector.
//!
//! A server of the authlib-injector speaks the protocol of Yggdrasil, which
//! [`super::yggdrasil`] implements; what the account adds is the agent the game
//! is started with, which makes the game talk to that server instead of the
//! service of Mojang.
//!
//! [`artifact`] reads the file of the agent and writes the one the launcher
//! ships, and [`account`] is the account of a player of such a server.

pub mod account;
pub mod artifact;
pub mod factory;

pub use account::AuthlibInjectorAccount;
pub use artifact::{
    AuthlibInjectorArtifactInfo, AuthlibInjectorArtifactProvider, AuthlibInjectorExtractor,
    SimpleAuthlibInjectorArtifactProvider, AUTHLIB_INJECTOR_VERSION,
};
pub use factory::{
    AuthlibInjectorAccountFactory, BoundAuthlibInjectorAccountFactory, AUTHLIB_INJECTOR_LOGIN_TYPE,
};
