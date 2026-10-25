//! The platform independent part of HMCL: metadata, the paths the launcher
//! works with, operating system detection and the task abstraction.
//!
//! The crate has no user interface dependency, so it can be tested without a
//! display and reused by another front end.

pub mod addon;
pub mod auth;
pub mod download;
pub mod error;
pub mod event;
pub mod game;
pub mod i18n;
pub mod java;
pub mod metadata;
pub mod modpack;
pub mod schematic;
pub mod screenshot;
pub mod setting;
pub mod task;
pub mod theme;
pub mod util;

pub use util::versioning::{GameVersionNumber, VersionNumber};
