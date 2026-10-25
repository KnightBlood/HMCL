//! The priority the operating system gives the process of the game.

use serde::{Deserialize, Serialize};

/// The priority of the process of the game.
///
/// A higher priority lets the game use the processor before other programs
/// when the processor is busy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProcessPriority {
    /// The game runs after the other programs.
    Low,
    /// The game runs after the programs of the usual priority.
    BelowNormal,
    /// The game runs with the usual priority.
    #[default]
    Normal,
    /// The game runs before the programs of the usual priority.
    AboveNormal,
    /// The game runs before the other programs.
    High,
}

impl ProcessPriority {
    /// Returns the number the operating system uses for the priority.
    ///
    /// The numbers are the ones of the `nice` value of Unix; the launcher
    /// needs them on every system it runs on.
    pub const fn nice_value(self) -> i32 {
        match self {
            Self::Low => 10,
            Self::BelowNormal => 5,
            Self::Normal => 0,
            Self::AboveNormal => -5,
            Self::High => -10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_priorities_are_ordered() {
        assert!(ProcessPriority::Low.nice_value() > ProcessPriority::Normal.nice_value());
        assert!(ProcessPriority::High.nice_value() < ProcessPriority::Normal.nice_value());
    }
}
