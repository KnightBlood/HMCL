//! The world or the server the game opens when it starts.

use serde::{Deserialize, Serialize};

/// The kind of place the game opens when it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuickPlayType {
    /// The game shows its title screen, as usual.
    #[default]
    None,
    /// The game joins a server.
    Multiplayer,
    /// The game opens a world of the player.
    Singleplayer,
    /// The game joins a realm.
    Realms,
}

/// The place the game opens when it starts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QuickPlayOption {
    /// A world of the player, named by the folder of the world.
    SinglePlayer(String),
    /// A server, named by its address.
    MultiPlayer(String),
    /// A realm, named by its identifier.
    Realm(String),
}

impl QuickPlayOption {
    /// Returns the place the option points to.
    ///
    /// The place is the folder of the world, the address of the server or the
    /// identifier of the realm, depending on the kind of the option.
    pub fn target(&self) -> &str {
        match self {
            Self::SinglePlayer(target) | Self::MultiPlayer(target) | Self::Realm(target) => target,
        }
    }

    /// Returns the argument the game reads to open the place.
    ///
    /// The argument is one `key=value` pair, such as
    /// `quickPlaySingleplayer=world`.
    pub fn argument(&self) -> String {
        match self {
            Self::SinglePlayer(world) => format!("quickPlaySingleplayer={world}"),
            Self::MultiPlayer(server) => format!("quickPlayMultiplayer={server}"),
            Self::Realm(realm) => format!("quickPlayRealms={realm}"),
        }
    }

    /// Returns whether the place the option points to is one the game reads.
    ///
    /// A server is read with its address, so the address of a server must be
    /// one the launcher can read; the other places are names the game reads as
    /// they are. An empty text is read as well, because a player who has not
    /// written a place yet has not written a wrong one.
    pub fn is_valid(&self) -> bool {
        match self {
            Self::SinglePlayer(_) | Self::Realm(_) => true,
            Self::MultiPlayer(server) => {
                server.trim().is_empty()
                    || crate::util::server_address::ServerAddress::parse(server).is_ok()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_option_points_to_the_place_the_game_opens() {
        assert_eq!(
            QuickPlayOption::SinglePlayer("world".to_string()).target(),
            "world"
        );
        assert_eq!(
            QuickPlayOption::MultiPlayer("example.org".to_string()).target(),
            "example.org"
        );
        assert_eq!(
            QuickPlayOption::Realm("realm".to_string()).target(),
            "realm"
        );
    }

    #[test]
    fn the_option_is_written_as_the_argument_of_the_game() {
        assert_eq!(
            QuickPlayOption::SinglePlayer("world".to_string()).argument(),
            "quickPlaySingleplayer=world"
        );
        assert_eq!(
            QuickPlayOption::MultiPlayer("example.org".to_string()).argument(),
            "quickPlayMultiplayer=example.org"
        );
        assert_eq!(
            QuickPlayOption::Realm("realm".to_string()).argument(),
            "quickPlayRealms=realm"
        );
    }

    #[test]
    fn a_server_the_launcher_can_read_is_a_valid_place() {
        assert!(QuickPlayOption::MultiPlayer("example.org".to_string()).is_valid());
        assert!(QuickPlayOption::MultiPlayer("example.org:25565".to_string()).is_valid());
        assert!(QuickPlayOption::MultiPlayer("[2001:db8::1]:25565".to_string()).is_valid());
        // A player who has not written a place yet has not written a wrong one.
        assert!(QuickPlayOption::MultiPlayer(String::new()).is_valid());
        assert!(QuickPlayOption::MultiPlayer("  ".to_string()).is_valid());
    }

    #[test]
    fn a_server_the_launcher_cannot_read_is_not_a_valid_place() {
        assert!(!QuickPlayOption::MultiPlayer("example.org:".to_string()).is_valid());
        assert!(!QuickPlayOption::MultiPlayer("example.org:not-a-port".to_string()).is_valid());
        assert!(!QuickPlayOption::MultiPlayer("[2001:db8::1]25565".to_string()).is_valid());
    }

    #[test]
    fn a_world_or_a_realm_is_always_a_valid_place() {
        assert!(QuickPlayOption::SinglePlayer("world".to_string()).is_valid());
        assert!(QuickPlayOption::Realm("realm".to_string()).is_valid());
    }
}
