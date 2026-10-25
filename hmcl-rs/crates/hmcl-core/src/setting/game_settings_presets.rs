//! The store of the game settings presets.
//!
//! The presets are kept in `config/game-settings.json`, apart from the other
//! settings of the launcher, so a broken preset does not break the settings of
//! the launcher.

use serde::{Deserialize, Serialize};

use crate::setting::schema::{Schema, SchemaSettings, SchemaVersion};
use crate::setting::storage::SettingsFile;

use super::game_settings::{Effective, GameSettingsPresetId, InstanceSettings, Preset};

/// The name of the file that holds the presets.
pub const GAME_SETTINGS_FILE: &str = "game-settings.json";

/// The address of the schema of the file that holds the presets.
pub const GAME_SETTINGS_SCHEMA: &str = "https://schemas.glavo.site/hmcl/game-settings/1.0.1";

/// Returns the schema of the file that holds the presets.
pub fn game_settings_schema() -> Schema {
    Schema::new("game-settings", SchemaVersion::new(1, 0, 1))
}

/// The reusable sets of game settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GameSettingsPresets {
    /// The address of the schema of the file.
    #[serde(rename = "$schema")]
    pub schema: String,
    /// The presets, in the order the launcher shows them.
    pub presets: Vec<Preset>,
}

impl Default for GameSettingsPresets {
    fn default() -> Self {
        Self {
            schema: GAME_SETTINGS_SCHEMA.to_string(),
            presets: Vec::new(),
        }
    }
}

impl GameSettingsPresets {
    /// Creates a store without presets.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an identifier that no preset of the store uses yet.
    pub fn new_preset_id(&self) -> GameSettingsPresetId {
        loop {
            let id = GameSettingsPresetId::generate();
            if self.get_preset(id).is_none() {
                return id;
            }
        }
    }

    /// Returns the first number the launcher can use to name a new preset.
    pub fn new_preset_auto_name_number(&self) -> u32 {
        let mut number = 1;
        loop {
            if !self
                .presets
                .iter()
                .any(|preset| preset.auto_name_number == Some(number))
            {
                return number;
            }
            number += 1;
        }
    }

    /// Returns the preset with the given identifier.
    pub fn get_preset(&self, id: GameSettingsPresetId) -> Option<&Preset> {
        self.presets.iter().find(|preset| preset.id == id)
    }

    /// Returns the preset with the given identifier, so it can be changed.
    pub fn get_preset_mut(&mut self, id: GameSettingsPresetId) -> Option<&mut Preset> {
        self.presets.iter_mut().find(|preset| preset.id == id)
    }

    /// Returns the preset the launcher uses by default, creating one when the
    /// store is empty.
    ///
    /// The preset with the preferred identifier is used when it exists;
    /// otherwise the first preset is used. A store without presets gets a new
    /// preset with a new identifier.
    pub fn default_preset_or_create(&mut self, preferred: Option<GameSettingsPresetId>) -> &Preset {
        let index = match preferred.and_then(|id| self.index_of(id)) {
            Some(index) => index,
            None => {
                if self.presets.is_empty() {
                    let id = self.new_id_for_new_preset();
                    self.presets.push(Preset::new(id));
                }
                0
            }
        };

        &self.presets[index]
    }

    /// Returns the identifier of the default preset, creating the preset when
    /// the store is empty.
    pub fn default_preset_id_or_create(
        &mut self,
        preferred: Option<GameSettingsPresetId>,
    ) -> GameSettingsPresetId {
        self.default_preset_or_create(preferred).id
    }

    /// Returns the preset an instance inherits from.
    ///
    /// The parent the instance names is used when it exists; otherwise the
    /// default preset is used.
    pub fn parent_of(
        &self,
        instance: Option<&InstanceSettings>,
        preferred: Option<GameSettingsPresetId>,
    ) -> Option<&Preset> {
        if let Some(parent) = instance.and_then(|instance| instance.parent) {
            if let Some(preset) = self.get_preset(parent) {
                return Some(preset);
            }
        }

        preferred
            .and_then(|id| self.get_preset(id))
            .or_else(|| self.presets.first())
    }

    /// Resolves the settings of an instance and the preset it inherits from.
    pub fn resolve<'a>(
        &'a self,
        instance: Option<&'a InstanceSettings>,
        preferred: Option<GameSettingsPresetId>,
    ) -> Option<Effective<'a>> {
        self.parent_of(instance, preferred)
            .map(|preset| preset.resolve(instance))
    }

    /// Returns the position of a preset in the store.
    fn index_of(&self, id: GameSettingsPresetId) -> Option<usize> {
        self.presets.iter().position(|preset| preset.id == id)
    }

    /// Creates an identifier for a preset that is about to be added.
    fn new_id_for_new_preset(&self) -> GameSettingsPresetId {
        loop {
            let id = GameSettingsPresetId::generate();
            if self.get_preset(id).is_none() {
                return id;
            }
        }
    }

    /// Reads the presets from `config/game-settings.json` of the launcher
    /// directory.
    pub fn load(home: &std::path::Path) -> Result<SettingsFile<Self>, crate::error::Error> {
        SettingsFile::load_with_schema(
            crate::setting::game_settings_path(home),
            &game_settings_schema(),
        )
    }

    /// Writes the presets to `config/game-settings.json` of the launcher
    /// directory.
    pub fn save(home: &std::path::Path, presets: &Self) -> Result<(), crate::error::Error> {
        SettingsFile::new(crate::setting::game_settings_path(home), presets.clone()).save()
    }
}

impl SchemaSettings for GameSettingsPresets {
    fn schema(&self) -> &str {
        &self.schema
    }

    fn set_schema(&mut self, schema: &str) {
        self.schema = schema.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::LocalizedText;

    /// Returns a preset with the identifier used by the tests.
    fn preset(id: &str) -> Preset {
        Preset::new(GameSettingsPresetId::parse(id).expect("the identifier should be read"))
    }

    #[test]
    fn the_store_uses_the_schema_of_the_java_launcher() {
        let store = GameSettingsPresets::new();
        let json = serde_json::to_value(&store).expect("the store should be written");

        assert_eq!(json["$schema"], GAME_SETTINGS_SCHEMA);
        assert_eq!(json["presets"], serde_json::json!([]));

        let read: GameSettingsPresets = serde_json::from_value(serde_json::json!({"presets": []}))
            .expect("the store should be read");
        assert_eq!(read.schema, GAME_SETTINGS_SCHEMA);
    }

    #[test]
    fn the_presets_are_found_by_their_identifier() {
        let first = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174000");
        let second = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174001");
        let store = GameSettingsPresets {
            schema: GAME_SETTINGS_SCHEMA.to_string(),
            presets: vec![first.clone(), second.clone()],
        };

        assert_eq!(store.get_preset(first.id), Some(&first));
        assert_eq!(store.get_preset(second.id), Some(&second));
        assert_eq!(store.get_preset(GameSettingsPresetId::NIL), None);
    }

    #[test]
    fn the_default_preset_is_the_preferred_one_or_the_first() {
        let first = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174000");
        let second = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174001");
        let mut store = GameSettingsPresets {
            schema: GAME_SETTINGS_SCHEMA.to_string(),
            presets: vec![first.clone(), second.clone()],
        };

        assert_eq!(
            store.default_preset_or_create(Some(second.id)).id,
            second.id
        );
        assert_eq!(store.default_preset_or_create(None).id, first.id);
        assert_eq!(
            store
                .default_preset_or_create(Some(GameSettingsPresetId::NIL))
                .id,
            first.id
        );
    }

    #[test]
    fn an_empty_store_creates_a_preset() {
        let mut store = GameSettingsPresets::new();
        let id = store.default_preset_id_or_create(None);

        assert_eq!(store.presets.len(), 1);
        assert_eq!(store.presets[0].id, id);
        assert!(!id.is_nil());
    }

    #[test]
    fn a_new_preset_gets_an_unused_identifier() {
        let store = GameSettingsPresets::new();
        let first = store.new_preset_id();
        let second = store.new_preset_id();

        assert_ne!(first, second);
        assert!(!first.is_nil());
    }

    #[test]
    fn the_automatic_names_skip_the_numbers_in_use() {
        let mut store = GameSettingsPresets::new();
        let mut first = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174000");
        first.auto_name_number = Some(1);
        let mut second = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174001");
        second.auto_name_number = Some(3);
        store.presets = vec![first, second];

        assert_eq!(store.new_preset_auto_name_number(), 2);
    }

    #[test]
    fn a_named_preset_and_an_automatic_number_are_stored_apart() {
        let mut named = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174000");
        named.name = Some(LocalizedText::plain("Custom"));
        let json = serde_json::to_value(&named).expect("the preset should be written");
        assert_eq!(json["name"], "Custom");
        assert!(json.get("autoNameNumber").is_none());

        let automatic = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174001");
        let mut automatic = automatic;
        automatic.auto_name_number = Some(4);
        let json = serde_json::to_value(&automatic).expect("the preset should be written");
        assert_eq!(json["autoNameNumber"], 4);
        assert!(json.get("name").is_none());
    }

    #[test]
    fn an_instance_uses_the_preset_it_names() {
        let first = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174000");
        let second = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174001");
        let store = GameSettingsPresets {
            schema: GAME_SETTINGS_SCHEMA.to_string(),
            presets: vec![first.clone(), second.clone()],
        };

        let mut instance = InstanceSettings::new();
        instance.parent = Some(second.id);

        assert_eq!(
            store
                .parent_of(Some(&instance), Some(first.id))
                .map(|p| p.id),
            Some(second.id)
        );

        // A parent that does not exist falls back to the default preset.
        instance.parent = Some(GameSettingsPresetId::NIL);
        assert_eq!(
            store
                .parent_of(Some(&instance), Some(second.id))
                .map(|p| p.id),
            Some(second.id)
        );
    }

    #[test]
    fn the_parent_of_an_instance_without_a_parent_is_the_default_preset() {
        let first = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174000");
        let second = preset("game-settings-preset:123e4567-e89b-12d3-a456-426614174001");
        let store = GameSettingsPresets {
            schema: GAME_SETTINGS_SCHEMA.to_string(),
            presets: vec![first.clone(), second.clone()],
        };

        assert_eq!(
            store.parent_of(None, Some(second.id)).map(|p| p.id),
            Some(second.id)
        );
        assert_eq!(store.parent_of(None, None).map(|p| p.id), Some(first.id));
    }

    #[test]
    fn the_store_is_written_and_read_back() {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-core-game-settings-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        let mut store = GameSettingsPresets::new();
        let id = store.default_preset_id_or_create(None);
        store.presets[0].settings.max_memory = Some(4096);

        GameSettingsPresets::save(&directory, &store).expect("the presets should be saved");
        let read = GameSettingsPresets::load(&directory).expect("the presets should be loaded");

        assert_eq!(read.settings().presets.len(), 1);
        assert_eq!(read.settings().presets[0].id, id);
        assert_eq!(read.settings().presets[0].settings.max_memory, Some(4096));
        assert_eq!(read.path(), crate::setting::game_settings_path(&directory));

        let _ = std::fs::remove_dir_all(&directory);
    }
}
