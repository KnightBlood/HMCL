//! The comparison of the version numbers.
//!
//! The launcher compares the versions of Minecraft with two kinds of numbers:
//! the general version numbers, which follow the rules of Apache Maven, and
//! the version numbers of Minecraft, which know the shape of the releases,
//! of the snapshots and of the experiments.

pub mod game_version_number;
pub mod version_number;
pub mod version_range;

pub use game_version_number::{GameVersionNumber, ReleaseType};
pub use version_number::VersionNumber;
pub use version_range::VersionRange;
