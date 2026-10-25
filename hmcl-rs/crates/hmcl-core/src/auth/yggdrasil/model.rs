//! The values the servers of the game answer with.
//!
//! A server of Yggdrasil answers the profile of a player as an identifier and
//! a name, and the properties of the profile as a list of pairs, because a
//! property may be repeated and the list keeps the order. The skin of the
//! player is one of those properties: it is a text of the own, encoded in
//! Base64, which names the files of the skin and of the cape.
//!
//! The identifiers of the players are written by the servers without the
//! hyphens, so [`Uuid::parse`] reads both forms and the answers are written
//! back without the hyphens.

use std::collections::BTreeMap;

use serde::de::Error as DeserializeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::auth::{AuthenticationError, Uuid};
use crate::util::base64;

/// The identifier and the name of a player.
///
/// A login answers the profile of the player it logged in, but not the
/// properties of the profile: the properties are read from another endpoint,
/// which is why the profile of a login is told apart from
/// [`CompleteGameProfile`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameProfile {
    /// The identifier of the player.
    id: Uuid,
    /// The name of the player.
    name: String,
}

impl GameProfile {
    /// Creates the profile of a player.
    pub fn new(id: Uuid, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }

    /// Returns the identifier of the player.
    pub const fn id(&self) -> Uuid {
        self.id
    }

    /// Returns the name of the player.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The fields a profile is read from.
///
/// The fields are read as options so an answer that names no identifier or no
/// name is reported as a malformed answer rather than as the default value of
/// the field.
#[derive(Deserialize)]
struct ProfileFields {
    /// The identifier of the player, written with or without the hyphens.
    #[serde(default)]
    id: Option<String>,
    /// The name of the player.
    #[serde(default)]
    name: Option<String>,
}

/// Reads the identifier and the name a profile is named with.
fn read_profile_fields(
    fields: ProfileFields,
    missing: &str,
) -> Result<GameProfile, serde_json::Error> {
    let id = match fields.id {
        Some(id) => Uuid::parse(&id).map_err(|error| {
            DeserializeError::custom(format!(
                "the identifier of the profile is malformed: {error}"
            ))
        })?,
        None => {
            return Err(DeserializeError::custom(format!(
                "the profile names no {missing} identifier"
            )))
        }
    };
    let name = fields
        .name
        .ok_or_else(|| DeserializeError::custom("the profile names no name of the player"))?;

    Ok(GameProfile { id, name })
}

impl<'de> Deserialize<'de> for GameProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        read_profile_fields(ProfileFields::deserialize(deserializer)?, "player").map_err(|error| {
            DeserializeError::custom(format!("the profile of the player is malformed: {error}"))
        })
    }
}

impl Serialize for GameProfile {
    /// Writes the profile the way the servers of the game write it.
    ///
    /// The identifier is written without the hyphens, which is the form the
    /// servers read.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut object = Map::new();
        object.insert("id".to_string(), Value::String(self.id.compact()));
        object.insert("name".to_string(), Value::String(self.name.clone()));
        object.serialize(serializer)
    }
}

/// The profile of a player with the properties the servers know of it.
///
/// The properties of a profile are the signed texts the game reads, of which
/// `textures` names the skin and the cape of the player. The profile of a
/// login holds no properties, so the launcher reads the complete profile from
/// the server before it shows the skin of the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteGameProfile {
    /// The identifier and the name of the player.
    profile: GameProfile,
    /// The properties of the profile, in the order the server named them.
    properties: BTreeMap<String, String>,
}

impl CompleteGameProfile {
    /// Creates a complete profile.
    pub fn new(id: Uuid, name: impl Into<String>, properties: BTreeMap<String, String>) -> Self {
        Self {
            profile: GameProfile::new(id, name),
            properties,
        }
    }

    /// Creates a complete profile out of the profile of a login.
    ///
    /// The properties of a login hold no skin, so a launcher that read the
    /// properties from its own settings keeps them with the profile of the
    /// login until the server is asked again.
    pub fn of(profile: GameProfile, properties: BTreeMap<String, String>) -> Self {
        Self {
            profile,
            properties,
        }
    }

    /// Returns the identifier of the player.
    pub const fn id(&self) -> Uuid {
        self.profile.id()
    }

    /// Returns the name of the player.
    pub fn name(&self) -> &str {
        self.profile.name()
    }

    /// Returns the identifier and the name of the player.
    pub fn profile(&self) -> &GameProfile {
        &self.profile
    }

    /// Returns the properties of the profile.
    pub fn properties(&self) -> &BTreeMap<String, String> {
        &self.properties
    }

    /// Returns the value of one property, if the profile has it.
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(String::as_str)
    }

    /// Reads the skin and the cape of the player out of the properties.
    ///
    /// The property `textures` holds a Base64 text which is a JSON object that
    /// names the files of the skin and of the cape. A profile without the
    /// property has no textures, and a property that is not the text of such an
    /// object is a failure of the server rather than a profile without
    /// textures.
    pub fn textures(&self) -> Result<Option<BTreeMap<TextureType, Texture>>, AuthenticationError> {
        let Some(encoded) = self.properties.get("textures") else {
            return Ok(None);
        };

        let decoded = base64::decode(encoded).ok_or_else(|| {
            AuthenticationError::malformed("the textures of the profile are not Base64")
        })?;
        let payload: TexturePayload = serde_json::from_slice(&decoded).map_err(|error| {
            AuthenticationError::malformed(format!(
                "the textures of the profile are not JSON: {error}"
            ))
        })?;

        Ok(payload.textures.map(|textures| {
            let mut known = BTreeMap::new();
            for (name, texture) in textures {
                // A server may name a texture the game does not draw, such as
                // the elytra of the versions after the one the launcher knows;
                // those textures are left out rather than refused.
                if let Some(kind) = TextureType::of(&name) {
                    known.insert(kind, texture);
                }
            }
            known
        }))
    }
}

impl<'de> Deserialize<'de> for CompleteGameProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// The fields a complete profile is read from.
        #[derive(Deserialize)]
        struct Fields {
            /// The identifier and the name of the player.
            #[serde(flatten)]
            profile: ProfileFields,
            /// The properties of the profile.
            #[serde(default)]
            properties: Option<Vec<Value>>,
        }

        let fields = Fields::deserialize(deserializer)?;
        let profile = read_profile_fields(fields.profile, "player").map_err(|error| {
            DeserializeError::custom(format!("the profile of the player is malformed: {error}"))
        })?;
        let properties = fields.properties.ok_or_else(|| {
            DeserializeError::custom("the profile names no properties of the player")
        })?;

        Ok(CompleteGameProfile {
            profile,
            properties: read_properties(&properties),
        })
    }
}

impl Serialize for CompleteGameProfile {
    /// Writes the profile the way the servers of the game write it.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut object = Map::new();
        object.insert("id".to_string(), Value::String(self.profile.id.compact()));
        object.insert("name".to_string(), Value::String(self.profile.name.clone()));
        object.insert("properties".to_string(), properties_value(&self.properties));
        object.serialize(serializer)
    }
}

/// Reads the properties of a profile out of the list the server answered.
///
/// An entry that names no name or no value is left out, because a server that
/// answers such an entry gives the launcher no property it could use; the
/// launcher of Java fails on the entry instead, which keeps the player from
/// logging in over a property the game does not read.
pub fn read_properties(entries: &[Value]) -> BTreeMap<String, String> {
    let mut properties = BTreeMap::new();
    for entry in entries {
        let (Some(name), Some(value)) = (
            entry.get("name").and_then(Value::as_str),
            entry.get("value").and_then(Value::as_str),
        ) else {
            continue;
        };
        properties.insert(name.to_string(), value.to_string());
    }
    properties
}

/// Writes the properties of a profile as the list the servers write.
pub fn properties_value(properties: &BTreeMap<String, String>) -> Value {
    Value::Array(
        properties
            .iter()
            .map(|(name, value)| {
                let mut entry = Map::new();
                entry.insert("name".to_string(), Value::String(name.clone()));
                entry.insert("value".to_string(), Value::String(value.clone()));
                Value::Object(entry)
            })
            .collect(),
    )
}

/// Reads the properties a launcher wrote as an object of name and value.
///
/// The launcher keeps the properties it read in the protected data of the
/// account, where an object is easier to read than the list of the servers. A
/// value that is not a text is refused, because the properties of a profile
/// are texts.
pub fn read_properties_object(
    object: &Map<String, Value>,
) -> Result<BTreeMap<String, String>, AuthenticationError> {
    let mut properties = BTreeMap::new();
    for (name, value) in object {
        let value = value.as_str().ok_or_else(|| {
            AuthenticationError::invalid_stored(format!(
                "the property {name} of the stored profile is not a text"
            ))
        })?;
        properties.insert(name.clone(), value.to_string());
    }
    Ok(properties)
}

/// Writes the properties of a profile as the object the launcher keeps.
pub fn properties_object(properties: &BTreeMap<String, String>) -> Value {
    Value::Object(
        properties
            .iter()
            .map(|(name, value)| (name.clone(), Value::String(value.clone())))
            .collect(),
    )
}

/// The kind of a texture of a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextureType {
    /// The skin of the player.
    Skin,
    /// The cape of the player.
    Cape,
}

impl TextureType {
    /// Returns the name of the kind, as the servers write it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skin => "SKIN",
            Self::Cape => "CAPE",
        }
    }

    /// Returns the kind with the name the servers write, if it is one the game
    /// draws.
    pub fn of(name: &str) -> Option<Self> {
        match name {
            "SKIN" => Some(Self::Skin),
            "CAPE" => Some(Self::Cape),
            _ => None,
        }
    }
}

/// How the skin of a player is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureModel {
    /// The arms of the skin are as wide as the legs.
    Wide,
    /// The arms of the skin are thin.
    Slim,
}

impl TextureModel {
    /// Returns the name of the model, as the servers write it.
    pub const fn model_name(self) -> &'static str {
        match self {
            Self::Wide => "default",
            Self::Slim => "slim",
        }
    }

    /// Returns the model with the name the servers write.
    ///
    /// A skin without a model is drawn with the wide arms, which is the model
    /// the game draws when the property names none.
    pub fn of(name: &str) -> Self {
        match name {
            "slim" => Self::Slim,
            _ => Self::Wide,
        }
    }
}

/// The file of a texture and the way it is drawn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Texture {
    /// The address of the file of the texture.
    url: Option<String>,
    /// The values the texture is drawn with, such as its model.
    metadata: Option<BTreeMap<String, String>>,
}

impl Texture {
    /// Creates a texture.
    pub fn new(url: Option<String>, metadata: Option<BTreeMap<String, String>>) -> Self {
        Self { url, metadata }
    }

    /// Returns the address of the file of the texture.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the values the texture is drawn with.
    pub fn metadata(&self) -> Option<&BTreeMap<String, String>> {
        self.metadata.as_ref()
    }

    /// Returns the model the skin is drawn with, if the texture names one.
    pub fn model(&self) -> Option<TextureModel> {
        self.metadata
            .as_ref()
            .and_then(|metadata| metadata.get("model"))
            .map(|model| TextureModel::of(model))
    }
}

/// Reads a texture out of the answer of the server.
///
/// The fields of a texture are read as options, because the servers answer a
/// texture without a metadata for the skins that name no model.
#[derive(Deserialize)]
struct TextureFields {
    /// The address of the file of the texture.
    #[serde(default)]
    url: Option<String>,
    /// The values the texture is drawn with.
    #[serde(default)]
    metadata: Option<BTreeMap<String, Value>>,
}

impl<'de> Deserialize<'de> for Texture {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = TextureFields::deserialize(deserializer)?;
        let metadata = fields.metadata.map(|metadata| {
            metadata
                .into_iter()
                .filter_map(|(name, value)| value.as_str().map(|value| (name, value.to_string())))
                .collect()
        });

        Ok(Texture {
            url: fields.url,
            metadata,
        })
    }
}

/// The object the property `textures` holds.
#[derive(Deserialize)]
struct TexturePayload {
    /// The textures of the player, named the way the servers name them.
    #[serde(default)]
    textures: Option<BTreeMap<String, Texture>>,
}

/// The user of the account a login answers.
///
/// The user is not the player: the servers name the properties of the user of
/// the account, such as the options of the privacy of the account, which the
/// game reads as the properties of the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// The identifier of the user.
    id: String,
    /// The properties of the user.
    properties: Option<BTreeMap<String, String>>,
}

impl User {
    /// Creates a user.
    pub fn new(id: impl Into<String>, properties: Option<BTreeMap<String, String>>) -> Self {
        Self {
            id: id.into(),
            properties,
        }
    }

    /// Returns the identifier of the user.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the properties of the user.
    pub fn properties(&self) -> Option<&BTreeMap<String, String>> {
        self.properties.as_ref()
    }
}

impl<'de> Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// The fields a user is read from.
        #[derive(Deserialize)]
        struct Fields {
            /// The identifier of the user.
            #[serde(default)]
            id: Option<String>,
            /// The properties of the user.
            #[serde(default)]
            properties: Option<Vec<Value>>,
        }

        let fields = Fields::deserialize(deserializer)?;
        let id = fields
            .id
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| DeserializeError::custom("the user names no identifier"))?;

        Ok(User {
            id,
            properties: fields.properties.map(|entries| read_properties(&entries)),
        })
    }
}
