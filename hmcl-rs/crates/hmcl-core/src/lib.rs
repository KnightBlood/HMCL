//! The platform independent part of HMCL: metadata, the paths the launcher
//! works with, operating system detection and the task abstraction.
//!
//! The crate has no user interface dependency, so it can be tested without a
//! display and reused by another front end.

pub mod metadata;
pub mod task;
pub mod util;
