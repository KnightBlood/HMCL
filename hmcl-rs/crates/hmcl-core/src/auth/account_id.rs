//! The identifiers of the account records of the launcher.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::uuid::{Uuid, UuidParseError};

/// The prefix of the identifiers of the account records.
pub const PREFIX: &str = "account";

/// The identifier of an account record of the launcher.
///
/// The identifier names the record, not the player: a record may be
/// authenticated again, renamed or moved, and its identifier stays the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(Uuid);

impl AccountId {
    /// Creates an identifier from its identifier of the record.
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Creates a new identifier.
    pub fn generate() -> Self {
        Self(Uuid::generate_v7())
    }

    /// Returns the identifier of the record.
    pub const fn uuid(&self) -> Uuid {
        self.0
    }

    /// Reads an identifier from its text.
    ///
    /// The text is written as `account-` followed by the identifier of the
    /// record.
    pub fn parse(text: &str) -> Result<Self, UuidParseError> {
        match text.strip_prefix(&format!("{PREFIX}-")) {
            Some(uuid) => Uuid::parse(uuid).map(Self),
            None => Err(UuidParseError::of(text)),
        }
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{PREFIX}-{}", self.0)
    }
}

impl FromStr for AccountId {
    type Err = UuidParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl Serialize for AccountId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AccountId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifiers_are_read_and_written() {
        let uuid = Uuid::offline("Notch");
        let id = AccountId::new(uuid);
        assert_eq!(id.to_string(), format!("account-{uuid}"));

        let read = AccountId::parse(&id.to_string()).expect("the identifier should be read");
        assert_eq!(read, id);
        assert_eq!(read.uuid(), uuid);
    }

    #[test]
    fn the_identifiers_without_the_prefix_are_refused() {
        assert!(AccountId::parse("b50ad385-829d-3141-a216-7e7d7539ba7f").is_err());
        assert!(AccountId::parse("account-").is_err());
    }

    #[test]
    fn the_created_identifiers_are_unique() {
        let first = AccountId::generate();
        let second = AccountId::generate();

        assert_ne!(first, second);
        assert_eq!(first.uuid().version(), 7);
    }

    #[test]
    fn the_identifiers_are_read_and_written_as_json() {
        let id = AccountId::generate();
        let json = serde_json::to_string(&id).expect("the identifier should be written");
        assert_eq!(json, format!("\"{id}\""));

        let read: AccountId = serde_json::from_str(&json).expect("the identifier should be read");
        assert_eq!(read, id);
    }
}
