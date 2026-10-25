//! The identifiers the launcher and the game use.

use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::util::digest::Md5;

/// A universally unique identifier.
///
/// The identifiers are written as 32 hexadecimal digits in five groups:
/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`. The game uses them for the
/// players, and the launcher uses them for its account records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Uuid([u8; 16]);

/// The error of reading an identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UuidParseError {
    /// The text that could not be read.
    text: String,
}

impl UuidParseError {
    /// Creates an error for the text that could not be read.
    pub fn of(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    /// Returns the text that could not be read.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for UuidParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "The identifier is malformed: {}", self.text)
    }
}

impl std::error::Error for UuidParseError {}

impl Uuid {
    /// The identifier of no object; it has every bit cleared.
    pub const NIL: Self = Self([0; 16]);

    /// Creates an identifier from its bytes.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the bytes of the identifier.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Returns whether the identifier is the identifier of no object.
    pub fn is_nil(&self) -> bool {
        self.0.iter().all(|byte| *byte == 0)
    }

    /// Returns the version of the identifier.
    ///
    /// The version is the first hexadecimal digit of the third group; the
    /// identifiers the launcher creates are of version 7, and the identifiers
    /// of the players that play without an account are of version 3.
    pub fn version(&self) -> u8 {
        self.0[6] >> 4
    }

    /// Returns the identifier without the hyphens.
    ///
    /// The game reads the identifier of the player without the hyphens.
    pub fn compact(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Reads an identifier from its text.
    ///
    /// The text may be written with the hyphens or without them; the case of
    /// the digits does not matter.
    pub fn parse(text: &str) -> Result<Self, UuidParseError> {
        let text = text.trim();
        let compact = text.replace('-', "");
        if compact.len() != 32
            || !compact
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(UuidParseError {
                text: text.to_string(),
            });
        }

        let mut bytes = [0u8; 16];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&compact[index * 2..index * 2 + 2], 16)
                .expect("the text was checked to be hexadecimal");
        }
        Ok(Self(bytes))
    }

    /// Returns the identifier of a player that plays without an account.
    ///
    /// The identifier is the digest of the name of the player, with the
    /// version 3 and the variant of the identifiers of the game.
    pub fn offline(name: &str) -> Self {
        let mut digest = Md5::new();
        digest.update(b"OfflinePlayer:");
        digest.update(name.as_bytes());
        let bytes = digest.finish();

        let mut uuid = Self(bytes);
        uuid.0[6] = (uuid.0[6] & 0x0f) | 0x30;
        uuid.0[8] = (uuid.0[8] & 0x3f) | 0x80;
        uuid
    }

    /// Returns the identifier of a name inside a namespace.
    ///
    /// The identifier is the digest of the namespace and the name, with the
    /// version 5 and the variant of the identifiers of the game, so the same
    /// name inside the same namespace always has the same identifier. The
    /// launcher uses this to name the records it creates while migrating the
    /// settings of an older launcher, so the migration may be run again
    /// without creating other records.
    pub fn generate_v5(namespace: Self, name: &str) -> Self {
        let mut digest = crate::util::digest::Sha1::new();
        digest.update(&namespace.0);
        digest.update(name.as_bytes());
        let digest = digest.finish();

        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);

        let mut uuid = Self(bytes);
        uuid.0[6] = (uuid.0[6] & 0x0f) | 0x50;
        uuid.0[8] = (uuid.0[8] & 0x3f) | 0x80;
        uuid
    }

    /// Creates an identifier of version 7.
    ///
    /// The first 48 bits hold the number of milliseconds since the start of
    /// the epoch, so the identifiers sort by the moment they were created;
    /// the other bits are random.
    pub fn generate_v7() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);

        let mut bytes = [0u8; 16];
        bytes[..6].copy_from_slice(&millis.to_be_bytes()[2..]);
        bytes[6..].copy_from_slice(&random_bytes()[..10]);
        bytes[6] = (bytes[6] & 0x0f) | 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;

        Self(bytes)
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = crate::util::digest::to_hex(&self.0);
        write!(
            formatter,
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }
}

impl FromStr for Uuid {
    type Err = UuidParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Uuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// Returns ten random bytes for the identifiers the launcher creates.
///
/// The bytes come from a counter, the clock and the process; the launcher
/// does not need a cryptographic source for its account identifiers.
fn random_bytes() -> [u8; 16] {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as u64)
        .unwrap_or(0);

    let mut state = counter
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(nanos ^ ((std::process::id() as u64) << 32));

    let mut bytes = [0u8; 16];
    for chunk in bytes.chunks_mut(8) {
        state = splitmix64(state);
        chunk.copy_from_slice(&state.to_be_bytes()[..chunk.len()]);
    }
    bytes
}

/// Advances the state of the random numbers.
fn splitmix64(state: u64) -> u64 {
    let mut value = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifiers_are_read_and_written() {
        let text = "b50ad385-829d-3141-a216-7e7d7539ba7f";
        let uuid = Uuid::parse(text).expect("the identifier should be read");
        assert_eq!(uuid.to_string(), text);
        assert_eq!(uuid.version(), 3);

        let compact =
            Uuid::parse("B50AD385829D3141A2167E7D7539BA7F").expect("the identifier should be read");
        assert_eq!(compact, uuid);
    }

    #[test]
    fn the_identifiers_that_are_malformed_are_refused() {
        assert!(Uuid::parse("").is_err());
        assert!(Uuid::parse("b50ad385-829d-3141-a216").is_err());
        assert!(Uuid::parse("zz0ad385-829d-3141-a216-7e7d7539ba7f").is_err());
    }

    #[test]
    fn the_identifiers_of_the_players_are_derived_from_their_names() {
        let uuid = Uuid::offline("Notch");
        assert_eq!(uuid.to_string(), "b50ad385-829d-3141-a216-7e7d7539ba7f");
        assert_eq!(Uuid::offline("Notch"), uuid);
        assert_ne!(Uuid::offline("Notch"), Uuid::offline("notch"));
    }

    #[test]
    fn the_identifiers_of_a_name_are_derived_from_it() {
        // The namespaces are the ones the launcher of Java uses for the
        // records it creates while migrating the settings of an older
        // launcher.
        let namespace = Uuid::parse("d5193464-a32e-52ad-b50a-d88fe967c3a2")
            .expect("the namespace should be read");

        assert_eq!(
            Uuid::generate_v5(namespace, "Default").to_string(),
            "7105bc1f-490e-5e8c-878c-f5844c3d4bc3"
        );
        assert_eq!(
            Uuid::generate_v5(namespace, "Home").to_string(),
            "f3eafde8-506e-5a77-bc88-f24b4728dfb2"
        );
        assert_eq!(
            Uuid::generate_v5(namespace, "Default"),
            Uuid::generate_v5(namespace, "Default")
        );
        assert_eq!(Uuid::generate_v5(namespace, "Default").version(), 5);
    }

    #[test]
    fn the_created_identifiers_are_of_version_7_and_unique() {
        let first = Uuid::generate_v7();
        let second = Uuid::generate_v7();

        assert_eq!(first.version(), 7);
        assert_eq!(second.version(), 7);
        assert_ne!(first, second);
        assert!(!first.is_nil());
        assert!(Uuid::NIL.is_nil());
    }

    #[test]
    fn the_identifiers_are_read_and_written_as_json() {
        let uuid = Uuid::offline("Notch");
        let json = serde_json::to_string(&uuid).expect("the identifier should be written");
        assert_eq!(json, "\"b50ad385-829d-3141-a216-7e7d7539ba7f\"");

        let read: Uuid = serde_json::from_str(&json).expect("the identifier should be read");
        assert_eq!(read, uuid);
    }
}
