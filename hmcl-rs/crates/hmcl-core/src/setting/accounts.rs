//! The files the launcher keeps the accounts in.
//!
//! The launcher keeps the accounts in two files: the public data of every
//! account in `config/accounts.json`, and the credentials in
//! `private/account-private-data.json`. The two are separate so the public
//! data can be read without revealing the credentials.
//!
//! The private data is written in a protected envelope, like the launcher of
//! Java writes it: the payload is encrypted with a key both launchers carry,
//! which keeps the credentials from being read at a glance, and it is written
//! as Base64 split over padded lanes. The envelope is not a secret of the
//! machine, so it is protection against a casual reader rather than against
//! someone who has the launcher.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::Error;
use crate::util::base64;
use crate::util::chacha20poly1305;

use super::schema::{Schema, SchemaSettings, SchemaVersion};
use super::storage::SettingsFile;
use crate::auth::AccountId;

/// The name of the file that holds the public data of the accounts.
pub const ACCOUNTS_FILE: &str = "accounts.json";

/// The name of the file that holds the credentials of the accounts.
pub const ACCOUNT_PRIVATE_DATA_FILE: &str = "account-private-data.json";

/// The name of the file that holds the public data of the accounts every
/// workspace shares.
pub const USER_ACCOUNTS_FILE: &str = "user-accounts.json";

/// The name of the file that holds the credentials of the accounts every
/// workspace shares.
pub const USER_ACCOUNT_PRIVATE_DATA_FILE: &str = "user-account-private-data.json";

/// The name of the directory that holds the files of the accounts.
pub const ACCOUNTS_DIRECTORY: &str = "config";

/// The name of the directory that holds the files the launcher keeps away
/// from the other files of the launcher.
pub const PRIVATE_DIRECTORY: &str = "private";

/// The key the launcher of Java protects the private data with.
///
/// The key is written into the launcher rather than kept on the machine, so
/// both launchers read the same file. It is the key of the launcher of Java,
/// byte for byte.
const PROTECTION_KEY: [u8; chacha20poly1305::KEY_LENGTH] = [
    0x3c, 0xd8, 0xa2, 0x22, 0x11, 0xd2, 0x8d, 0x89, 0xb4, 0xf7, 0xd9, 0xb0, 0x65, 0xbc, 0x14, 0x8a,
    0x6e, 0xb0, 0xa9, 0x4d, 0xeb, 0x93, 0x99, 0x6f, 0x84, 0x07, 0x5a, 0x9e, 0xbd, 0xc8, 0xd1, 0xeb,
];

/// The member that tells how the payload of an envelope is protected.
pub const PROPERTY_PROTECTION: &str = "protection";

/// The member that holds the payload of an envelope.
pub const PROPERTY_PAYLOAD: &str = "payload";

/// The member that holds the nonce of an encrypted envelope.
pub const PROPERTY_NONCE: &str = "nonce";

/// The protection that writes the payload as it is.
pub const PROTECTION_PLAIN: &str = "plain";

/// The protection that encrypts the payload.
pub const PROTECTION_OBFUSCATED_V1: &str = "hmcl-obfuscated-v1";

/// The number of the lanes the encrypted payload is split over.
const OBFUSCATED_LANE_COUNT: usize = 4;

/// The number of the empty members written before each lane.
const LANE_PADDING_COUNT: usize = 63;

/// The number of the members of the array the payload is written in.
const WRITTEN_OBFUSCATED_PAYLOAD_SIZE: usize = 256;

/// Returns the schema of the file that holds the public data of the accounts.
pub fn accounts_schema() -> Schema {
    Schema::new("accounts", SchemaVersion::new(1, 0, 0))
}

/// Returns the schema of the file that holds the credentials of the accounts.
pub fn account_private_data_schema() -> Schema {
    Schema::new("account-private-data", SchemaVersion::new(1, 0, 0))
}

/// The public data of the accounts of the launcher.
///
/// A record is kept as it is read, so a field the launcher does not know yet
/// survives a save. The records are in the order the launcher shows them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountMetadataStore {
    /// The address of the schema of the file.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// The public data of the accounts.
    pub accounts: Vec<Map<String, Value>>,
}

impl Default for AccountMetadataStore {
    fn default() -> Self {
        Self {
            schema: accounts_schema().url().to_string(),
            accounts: Vec::new(),
        }
    }
}

impl AccountMetadataStore {
    /// Creates a store without accounts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a store from records that are already in the form of the file.
    pub fn from_records(records: Vec<Map<String, Value>>) -> Self {
        Self {
            schema: accounts_schema().url().to_string(),
            accounts: records,
        }
    }

    /// Returns the identifier of the account a record describes.
    ///
    /// A record without a readable identifier names no account of the
    /// launcher of today, so it is kept but not found.
    pub fn account_id(record: &Map<String, Value>) -> Option<AccountId> {
        record
            .get("accountID")
            .and_then(Value::as_str)
            .and_then(|text| AccountId::parse(text).ok())
    }

    /// Returns the record with the given identifier.
    pub fn get(&self, id: AccountId) -> Option<&Map<String, Value>> {
        self.accounts
            .iter()
            .find(|record| Self::account_id(record) == Some(id))
    }

    /// Returns the identifiers of the accounts, in the order the launcher
    /// shows them.
    pub fn account_ids(&self) -> Vec<AccountId> {
        self.accounts.iter().filter_map(Self::account_id).collect()
    }

    /// Reads the public data of the accounts from `config/accounts.json` of
    /// the launcher directory.
    pub fn load(home: &Path) -> Result<SettingsFile<Self>, Error> {
        Self::load_from(&accounts_path(home))
    }

    /// Reads the public data of the accounts from a file.
    pub fn load_from(path: &Path) -> Result<SettingsFile<Self>, Error> {
        SettingsFile::load_with_schema(path.to_path_buf(), &accounts_schema())
    }

    /// Writes the public data of the accounts to `config/accounts.json` of
    /// the launcher directory.
    pub fn save(home: &Path, store: &Self) -> Result<(), Error> {
        Self::save_to(&accounts_path(home), store)
    }

    /// Writes the public data of the accounts to a file.
    pub fn save_to(path: &Path, store: &Self) -> Result<(), Error> {
        SettingsFile::new(path.to_path_buf(), store.clone()).save()
    }
}

impl SchemaSettings for AccountMetadataStore {
    fn schema(&self) -> &str {
        &self.schema
    }

    fn set_schema(&mut self, schema: &str) {
        self.schema = schema.to_string();
    }
}

/// The credentials of the accounts of the launcher.
///
/// The credentials are kept apart from the public data, keyed by the
/// identifier of the account record, so a record may be renamed or moved
/// without the credentials moving with it.
///
/// The entries keep the order they were added in, which is the order the
/// launcher of Java writes them in; a map that sorts its keys would write the
/// same entries in another order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountPrivateData {
    /// The credentials of every account, in the order the accounts were added.
    entries: Vec<(AccountId, Map<String, Value>)>,
}

impl AccountPrivateData {
    /// Creates a store without credentials.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether the store holds no credentials.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of the accounts the store holds credentials for.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns the credentials of an account.
    pub fn get(&self, id: AccountId) -> Option<&Map<String, Value>> {
        self.entries
            .iter()
            .find(|(entry_id, _)| *entry_id == id)
            .map(|(_, data)| data)
    }

    /// Returns whether the store holds credentials for an account.
    pub fn contains(&self, id: AccountId) -> bool {
        self.get(id).is_some()
    }

    /// Returns the accounts the store holds credentials for, in the order the
    /// credentials are written in.
    pub fn account_ids(&self) -> impl Iterator<Item = AccountId> + '_ {
        self.entries.iter().map(|(id, _)| *id)
    }

    /// Stores the credentials of an account.
    ///
    /// Credentials that hold nothing are not stored, so an account without
    /// credentials leaves no entry behind. Storing credentials of an account
    /// that is already stored keeps the position of its entry, so a save does
    /// not reorder the accounts.
    pub fn put(&mut self, id: AccountId, data: Map<String, Value>) {
        if data.is_empty() {
            self.remove(id);
            return;
        }

        match self
            .entries
            .iter_mut()
            .find(|(entry_id, _)| *entry_id == id)
        {
            Some((_, entry)) => *entry = data,
            None => self.entries.push((id, data)),
        }
    }

    /// Removes the credentials of an account.
    ///
    /// Returns whether the store held credentials for the account.
    pub fn remove(&mut self, id: AccountId) -> bool {
        let Some(index) = self
            .entries
            .iter()
            .position(|(entry_id, _)| *entry_id == id)
        else {
            return false;
        };
        self.entries.remove(index);
        true
    }

    /// Returns the payload of the file, in the form the launcher of Java
    /// writes it.
    ///
    /// The payload is the list of the entries in the order they were added,
    /// which is the order the launcher of Java writes.
    pub fn payload(&self) -> Value {
        Value::Array(
            self.entries
                .iter()
                .map(|(id, data)| {
                    let mut entry = Map::new();
                    entry.insert("accountID".to_string(), Value::String(id.to_string()));
                    entry.insert("privateData".to_string(), Value::Object(data.clone()));
                    Value::Object(entry)
                })
                .collect(),
        )
    }

    /// Reads the credentials out of a payload.
    ///
    /// An entry whose identifier cannot be read or whose data is not an object
    /// is skipped, so one broken entry does not lose the other accounts.
    pub fn read_payload(payload: &Value) -> Self {
        let mut store = Self::new();
        let Some(entries) = payload.as_array() else {
            return store;
        };

        for entry in entries {
            let Some(entry) = entry.as_object() else {
                continue;
            };
            let Some(id) = entry
                .get("accountID")
                .and_then(Value::as_str)
                .and_then(|text| AccountId::parse(text).ok())
            else {
                continue;
            };
            let Some(data) = entry.get("privateData").and_then(Value::as_object) else {
                continue;
            };

            store.put(id, data.clone());
        }
        store
    }

    /// Reads the credentials from `private/account-private-data.json` of the
    /// launcher directory.
    ///
    /// A file that does not exist yet yields an empty store. A file whose
    /// envelope cannot be revealed is an error, so the credentials are not
    /// silently dropped.
    pub fn load(home: &Path) -> Result<Self, Error> {
        Self::load_from(&account_private_data_path(home))
    }

    /// Reads the credentials from a file.
    ///
    /// A file that does not exist yet yields an empty store. A file whose
    /// envelope cannot be revealed is an error, so the credentials are not
    /// silently dropped.
    pub fn load_from(path: &Path) -> Result<Self, Error> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(error) => return Err(Error::io(path, error)),
        };

        let envelope: Value = serde_json::from_str(&text)
            .map_err(|error| Error::invalid_format(path, error.to_string()))?;
        let envelope = envelope
            .as_object()
            .ok_or_else(|| Error::invalid_format(path, "the file does not hold an object"))?;

        let payload = reveal(envelope)
            .ok_or_else(|| Error::invalid_format(path, "the credentials cannot be revealed"))?;
        Ok(Self::read_payload(&payload))
    }

    /// Writes the credentials to `private/account-private-data.json` of the
    /// launcher directory.
    pub fn save(&self, home: &Path) -> Result<(), Error> {
        self.save_to(&account_private_data_path(home))
    }

    /// Writes the credentials to a file.
    pub fn save_to(&self, path: &Path) -> Result<(), Error> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;

        let mut envelope = Map::new();
        envelope.insert(
            "$schema".to_string(),
            Value::String(account_private_data_schema().url().to_string()),
        );
        protect(&mut envelope, &self.payload(), None);

        let content = serde_json::to_string_pretty(&Value::Object(envelope))
            .map_err(|error| Error::invalid_format(path, error.to_string()))?;

        // The file is written to a temporary file first, so an interrupted
        // write does not lose the credentials of every account.
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, content.as_bytes())
            .map_err(|error| Error::io(&temporary, error))?;
        match std::fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                Err(Error::io(path, error))
            }
        }
    }
}

/// Returns the path of the file that holds the public data of the accounts.
pub fn accounts_path(home: &Path) -> std::path::PathBuf {
    home.join(ACCOUNTS_DIRECTORY).join(ACCOUNTS_FILE)
}

/// Returns the path of the file that holds the credentials of the accounts.
pub fn account_private_data_path(home: &Path) -> std::path::PathBuf {
    home.join(PRIVATE_DIRECTORY).join(ACCOUNT_PRIVATE_DATA_FILE)
}

/// Returns the path of the file that holds the public data of the accounts
/// every workspace shares.
pub fn user_accounts_path(home: &Path) -> std::path::PathBuf {
    home.join(ACCOUNTS_DIRECTORY).join(USER_ACCOUNTS_FILE)
}

/// Returns the path of the file that holds the credentials of the accounts
/// every workspace shares.
pub fn user_account_private_data_path(home: &Path) -> std::path::PathBuf {
    home.join(PRIVATE_DIRECTORY)
        .join(USER_ACCOUNT_PRIVATE_DATA_FILE)
}

/// Writes a payload into an envelope.
///
/// The nonce is written by the caller when the test needs the same envelope
/// twice; the launcher draws a new one for every save.
fn protect(envelope: &mut Map<String, Value>, payload: &Value, nonce: Option<[u8; 12]>) {
    let nonce = nonce.unwrap_or_else(random_nonce);
    let text = serde_json::to_string(payload).unwrap_or_else(|_| "[]".to_string());
    let sealed = chacha20poly1305::seal(&PROTECTION_KEY, &nonce, text.as_bytes());

    envelope.insert(
        PROPERTY_PROTECTION.to_string(),
        Value::String(PROTECTION_OBFUSCATED_V1.to_string()),
    );
    envelope.insert(
        PROPERTY_PAYLOAD.to_string(),
        Value::Array(split_payload(&base64::encode(&sealed))),
    );
    envelope.insert(
        PROPERTY_NONCE.to_string(),
        Value::String(base64::encode(&nonce)),
    );
}

/// Reads the payload of an envelope.
///
/// The protection the envelope names is used; an envelope that names a
/// protection the launcher does not know cannot be read, and neither can one
/// whose payload was changed.
pub fn reveal(envelope: &Map<String, Value>) -> Option<Value> {
    let protection = envelope.get(PROPERTY_PROTECTION)?.as_str()?;
    match protection {
        PROTECTION_PLAIN => envelope.get(PROPERTY_PAYLOAD).cloned(),
        PROTECTION_OBFUSCATED_V1 => {
            let nonce = base64::decode(envelope.get(PROPERTY_NONCE)?.as_str()?)?;
            let nonce: [u8; 12] = nonce.try_into().ok()?;
            let sealed = base64::decode(&join_payload(envelope)?)?;
            let plain = chacha20poly1305::open(&PROTECTION_KEY, &nonce, &sealed)?;
            serde_json::from_slice(&plain).ok()
        }
        _ => None,
    }
}

/// Splits a Base64 payload over the lanes of the envelope.
///
/// Every lane is written after a run of empty members, so the payload of the
/// file is an array of a size that tells nothing about the payload.
fn split_payload(payload: &str) -> Vec<Value> {
    let lane_length = payload.len() / OBFUSCATED_LANE_COUNT;
    let mut lanes = Vec::with_capacity(WRITTEN_OBFUSCATED_PAYLOAD_SIZE);
    for lane in 0..OBFUSCATED_LANE_COUNT {
        let start = lane * lane_length;
        let text = &payload[start..start + lane_length];
        lanes.extend(std::iter::repeat_n(Value::Null, LANE_PADDING_COUNT));
        lanes.push(Value::String(text.to_string()));
    }
    lanes
}

/// Joins the lanes of an envelope back into a Base64 payload.
fn join_payload(envelope: &Map<String, Value>) -> Option<String> {
    let lanes = envelope.get(PROPERTY_PAYLOAD)?.as_array()?;
    if lanes.len() < OBFUSCATED_LANE_COUNT {
        return None;
    }

    // The array of the launcher of Java is written with a size that is a power
    // of two, so the lanes are read from the window of that size.
    let window = highest_power_of_two(lanes.len());
    let mut payload = String::new();
    for lane in 0..OBFUSCATED_LANE_COUNT {
        let segment = window / OBFUSCATED_LANE_COUNT;
        let index = (lane + 1) * segment - 1;
        payload.push_str(lanes.get(index)?.as_str()?);
    }
    Some(payload)
}

/// Returns the highest power of two that is not larger than a number.
fn highest_power_of_two(number: usize) -> usize {
    if number == 0 {
        return 0;
    }
    1 << (usize::BITS - 1 - number.leading_zeros())
}

/// Returns a nonce for an envelope.
///
/// The nonce keeps a save from producing the same file as the save before it,
/// so a reader cannot tell that two saves hold the same credentials.
fn random_nonce() -> [u8; 12] {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);

    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&nanos.to_le_bytes());
    nonce[8..].copy_from_slice(&counter.to_le_bytes()[..4]);
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the identifier of an account.
    fn account_id(index: u128) -> AccountId {
        let uuid = crate::auth::Uuid::from_bytes(index.to_be_bytes());
        AccountId::new(uuid)
    }

    /// Returns the credentials of an account.
    fn credentials(token: &str) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert("accessToken".to_string(), Value::String(token.to_string()));
        data
    }

    #[test]
    fn the_store_uses_the_schema_of_the_java_launcher() {
        let store = AccountMetadataStore::new();
        assert_eq!(
            store.schema,
            "https://schemas.glavo.site/hmcl/accounts/1.0.0"
        );
        assert!(store.accounts.is_empty());
    }

    #[test]
    fn the_public_data_is_read_and_written_the_way_java_writes_it() {
        let mut record = Map::new();
        record.insert(
            "accountID".to_string(),
            Value::String("account-0199a3b4-1111-7222-8333-444444444444".to_string()),
        );
        record.insert("type".to_string(), Value::String("offline".to_string()));
        record.insert(
            "profileName".to_string(),
            Value::String("Steve".to_string()),
        );

        let store = AccountMetadataStore::from_records(vec![record]);
        let json = serde_json::to_string(&store).expect("the store should be written");
        assert!(json.contains("\"$schema\":\"https://schemas.glavo.site/hmcl/accounts/1.0.0\""));
        assert!(json.contains("\"accounts\":[{"));

        let read: AccountMetadataStore =
            serde_json::from_str(&json).expect("the store should be read");
        assert_eq!(read, store);
        assert_eq!(
            read.account_ids(),
            vec![
                AccountId::parse("account-0199a3b4-1111-7222-8333-444444444444")
                    .expect("the identifier should be read")
            ]
        );
    }

    #[test]
    fn a_record_without_a_readable_identifier_names_no_account() {
        let mut record = Map::new();
        record.insert("type".to_string(), Value::String("offline".to_string()));
        let store = AccountMetadataStore::from_records(vec![record]);
        assert!(store.account_ids().is_empty());
        assert_eq!(AccountMetadataStore::account_id(&store.accounts[0]), None);
    }

    #[test]
    fn the_payload_of_the_credentials_is_the_list_of_java() {
        let mut store = AccountPrivateData::new();
        store.put(account_id(1), credentials("first"));
        store.put(account_id(2), credentials("second"));

        let payload = store.payload();
        let entries = payload.as_array().expect("the payload should be a list");
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].get("accountID").and_then(Value::as_str),
            Some("account-00000000-0000-0000-0000-000000000001")
        );
        assert_eq!(
            entries[0]
                .get("privateData")
                .and_then(|data| data.get("accessToken"))
                .and_then(Value::as_str),
            Some("first")
        );

        assert_eq!(AccountPrivateData::read_payload(&payload), store);
    }

    #[test]
    fn the_credentials_are_stored_apart_from_the_public_data() {
        let mut store = AccountPrivateData::new();
        store.put(account_id(1), credentials("secret"));
        assert!(store.contains(account_id(1)));
        assert_eq!(
            store
                .get(account_id(1))
                .and_then(|data| data.get("accessToken"))
                .and_then(Value::as_str),
            Some("secret")
        );

        // An account without credentials leaves no entry behind.
        store.put(account_id(2), Map::new());
        assert!(!store.contains(account_id(2)));
        assert!(store.remove(account_id(1)));
        assert!(store.is_empty());
        assert!(!store.remove(account_id(1)));
    }

    #[test]
    fn a_payload_that_is_not_a_list_reads_as_no_credentials() {
        assert!(AccountPrivateData::read_payload(&Value::String("no".to_string())).is_empty());
        assert!(AccountPrivateData::read_payload(&serde_json::json!([1, "two"])).is_empty());
    }

    #[test]
    fn an_entry_with_a_broken_identifier_is_skipped() {
        let payload = serde_json::json!([
            {"accountID": "not-an-account", "privateData": {"accessToken": "lost"}},
            {"accountID": "account-00000000-0000-0000-0000-000000000001", "privateData": {"accessToken": "kept"}},
            {"accountID": "account-00000000-0000-0000-0000-000000000002", "privateData": "not-an-object"}
        ]);

        let store = AccountPrivateData::read_payload(&payload);
        assert_eq!(store.len(), 1);
        assert!(store.contains(account_id(1)));
    }

    #[test]
    fn the_envelope_is_protected_the_way_java_protects_it() {
        let mut store = AccountPrivateData::new();
        store.put(account_id(1), credentials("secret-token"));

        let mut envelope = Map::new();
        protect(
            &mut envelope,
            &store.payload(),
            Some([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        );

        // The envelope names the protection, the nonce and the lanes, and the
        // credentials are not readable in it.
        assert_eq!(
            envelope.get(PROPERTY_PROTECTION).and_then(Value::as_str),
            Some(PROTECTION_OBFUSCATED_V1)
        );
        assert_eq!(
            envelope.get(PROPERTY_NONCE).and_then(Value::as_str),
            Some("AQIDBAUGBwgJCgsM")
        );
        let lanes = envelope
            .get(PROPERTY_PAYLOAD)
            .and_then(Value::as_array)
            .expect("the payload should be a list of lanes");
        assert_eq!(lanes.len(), WRITTEN_OBFUSCATED_PAYLOAD_SIZE);
        assert!(!serde_json::to_string(&lanes)
            .expect("the lanes should be written")
            .contains("secret-token"));

        // The payload is the one the launcher of Java reads.
        let revealed = reveal(&envelope).expect("the payload should be revealed");
        assert_eq!(AccountPrivateData::read_payload(&revealed), store);
    }

    #[test]
    fn an_envelope_of_java_is_read() {
        // The envelope of the launcher of Java, with the payload
        // `[]` encrypted under the nonce of the test above.
        let mut envelope = Map::new();
        envelope.insert(
            PROPERTY_PROTECTION.to_string(),
            Value::String(PROTECTION_OBFUSCATED_V1.to_string()),
        );
        envelope.insert(
            PROPERTY_NONCE.to_string(),
            Value::String("AQIDBAUGBwgJCgsM".to_string()),
        );

        let mut lanes = vec![Value::Null; WRITTEN_OBFUSCATED_PAYLOAD_SIZE];
        let encoded = "Lz3+uwwqf51YAEsFKR4V29Vr";
        let lane_length = encoded.len() / OBFUSCATED_LANE_COUNT;
        for lane in 0..OBFUSCATED_LANE_COUNT {
            let segment = WRITTEN_OBFUSCATED_PAYLOAD_SIZE / OBFUSCATED_LANE_COUNT;
            lanes[(lane + 1) * segment - 1] =
                Value::String(encoded[lane * lane_length..(lane + 1) * lane_length].to_string());
        }
        envelope.insert(PROPERTY_PAYLOAD.to_string(), Value::Array(lanes));

        assert_eq!(reveal(&envelope), Some(serde_json::json!([])));
    }

    #[test]
    fn a_plain_envelope_is_read() {
        let mut envelope = Map::new();
        envelope.insert(
            PROPERTY_PROTECTION.to_string(),
            Value::String(PROTECTION_PLAIN.to_string()),
        );
        envelope.insert(
            PROPERTY_PAYLOAD.to_string(),
            serde_json::json!([{"accountID": "account-00000000-0000-0000-0000-000000000001", "privateData": {"accessToken": "plain"}}]),
        );

        let store = AccountPrivateData::read_payload(&reveal(&envelope).expect("revealed"));
        assert!(store.contains(account_id(1)));
    }

    #[test]
    fn a_broken_envelope_is_refused() {
        // An envelope without a protection cannot be read.
        assert_eq!(reveal(&Map::new()), None);

        // An unknown protection is refused rather than guessed.
        let mut envelope = Map::new();
        envelope.insert(
            PROPERTY_PROTECTION.to_string(),
            Value::String("future".to_string()),
        );
        assert_eq!(reveal(&envelope), None);

        // A payload that was changed does not reveal.
        let mut store = AccountPrivateData::new();
        store.put(account_id(1), credentials("secret"));
        let mut envelope = Map::new();
        protect(&mut envelope, &store.payload(), Some([0u8; 12]));
        let lanes = envelope
            .get_mut(PROPERTY_PAYLOAD)
            .and_then(Value::as_array_mut)
            .expect("the payload should be a list");
        lanes[LANE_PADDING_COUNT] = Value::String("AAAAAAAAAAAAAAAAAAAA".to_string());
        assert_eq!(reveal(&envelope), None);
    }

    #[test]
    fn the_lanes_of_the_credentials_are_split_evenly() {
        let payload = "abcdefghijklmnopqrstuvwx";
        let lanes = split_payload(payload);

        // Every lane is written after the padding, so the lanes are at the
        // end of each quarter of the array.
        let segment = WRITTEN_OBFUSCATED_PAYLOAD_SIZE / OBFUSCATED_LANE_COUNT;
        let mut joined = String::new();
        for lane in 0..OBFUSCATED_LANE_COUNT {
            joined.push_str(
                lanes[(lane + 1) * segment - 1]
                    .as_str()
                    .expect("the lane should be a string"),
            );
        }
        assert_eq!(joined, payload);
    }

    #[test]
    fn the_credentials_are_written_and_read_again() {
        let home = std::env::temp_dir().join(format!(
            "hmcl-core-account-private-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);

        let mut store = AccountPrivateData::new();
        store.put(account_id(1), credentials("first"));
        store.put(account_id(2), credentials("second"));
        store
            .save(&home)
            .expect("the credentials should be written");

        let read = AccountPrivateData::load(&home).expect("the credentials should be read");
        assert_eq!(read, store);

        // The credentials are not written as readable text.
        let text = std::fs::read_to_string(account_private_data_path(&home))
            .expect("the file should be read");
        assert!(!text.contains("first"));
        assert!(!text.contains("second"));
        assert!(text.contains("hmcl-obfuscated-v1"));

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_launcher_without_credentials_reads_an_empty_store() {
        let home = std::env::temp_dir().join(format!(
            "hmcl-core-account-private-empty-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).expect("the directory should be created");

        assert!(AccountPrivateData::load(&home)
            .expect("the store should be read")
            .is_empty());

        let _ = std::fs::remove_dir_all(&home);
    }
}
