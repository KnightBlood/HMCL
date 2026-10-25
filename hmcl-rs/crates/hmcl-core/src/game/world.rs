//! The worlds a player has in a game directory.
//!
//! A world is kept in its own directory under `saves`: the directory holds the
//! `level.dat` of the world, which is a gzipped NBT compound. The launcher
//! reads the level to show the worlds and to start the game in one of them.

use std::path::{Path, PathBuf};

use crate::util::nbt::{read_compressed, write_compressed, Tag};

/// The error of reading or managing a world.
#[derive(Debug)]
pub enum WorldError {
    /// The path is not a world directory.
    NotAWorld {
        /// The path that is not a world.
        path: PathBuf,
    },
    /// The level of the world cannot be read or written.
    Level {
        /// The file that cannot be read or written.
        file: PathBuf,
        /// The reason the file cannot be read or written.
        error: crate::error::Error,
    },
    /// The world is locked by another process.
    WorldLocked {
        /// The directory of the locked world.
        path: PathBuf,
    },
    /// The world cannot be renamed on the disk.
    RenameFailed {
        /// The directory of the world.
        path: PathBuf,
        /// The reason the rename failed.
        error: std::io::Error,
    },
    /// The world cannot be copied.
    CopyFailed {
        /// The directory of the world.
        path: PathBuf,
        /// The reason the copy failed.
        error: std::io::Error,
    },
    /// The world cannot be deleted.
    DeleteFailed {
        /// The directory of the world.
        path: PathBuf,
        /// The reason the deletion failed.
        error: std::io::Error,
    },
    /// The archive of a world cannot be read or written.
    Archive {
        /// The archive of the world.
        path: PathBuf,
        /// The reason the archive cannot be read or written.
        error: crate::util::zip::ZipError,
    },
    /// The world does not record the value the caller wants to change.
    MissingField {
        /// The directory of the world.
        path: PathBuf,
        /// The name of the value the world does not record.
        field: &'static str,
    },
    /// The image is not an icon the game reads.
    InvalidIcon {
        /// The image that cannot be used as an icon.
        path: PathBuf,
        /// The reason the image cannot be used.
        reason: String,
    },
}

impl std::fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAWorld { path } => {
                write!(formatter, "{} is not a world directory", path.display())
            }
            Self::Level { file, error } => {
                write!(
                    formatter,
                    "the level {} cannot be read or written: {error}",
                    file.display()
                )
            }
            Self::WorldLocked { path } => {
                write!(formatter, "the world {} is locked", path.display())
            }
            Self::RenameFailed { path, error } => {
                write!(
                    formatter,
                    "the world {} cannot be renamed: {error}",
                    path.display()
                )
            }
            Self::CopyFailed { path, error } => {
                write!(
                    formatter,
                    "the world {} cannot be copied: {error}",
                    path.display()
                )
            }
            Self::DeleteFailed { path, error } => {
                write!(
                    formatter,
                    "the world {} cannot be deleted: {error}",
                    path.display()
                )
            }
            Self::Archive { path, error } => {
                write!(
                    formatter,
                    "the archive {} of a world cannot be read or written: {error}",
                    path.display()
                )
            }
            Self::MissingField { path, field } => {
                write!(
                    formatter,
                    "the world {} does not record {field}",
                    path.display()
                )
            }
            Self::InvalidIcon { path, reason } => {
                write!(
                    formatter,
                    "{} is not a usable world icon: {reason}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for WorldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Level { error, .. } => Some(error),
            Self::RenameFailed { error, .. }
            | Self::CopyFailed { error, .. }
            | Self::DeleteFailed { error, .. } => Some(error),
            Self::Archive { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// A dimension of a world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dimension {
    /// The overworld, where the player starts.
    Overworld,
    /// The nether.
    TheNether,
    /// The end.
    TheEnd,
    /// A dimension the launcher does not know, named by its identifier.
    Custom(String),
}

impl Dimension {
    /// Reads a dimension from the identifier the game writes.
    ///
    /// The game writes the identifier of the standard dimensions both with and
    /// without the namespace, so both are read. A dimension that is not one of
    /// the three is kept by its identifier.
    pub fn from_id(id: &str) -> Self {
        match id {
            "overworld" | "minecraft:overworld" => Self::Overworld,
            "the_nether" | "minecraft:the_nether" => Self::TheNether,
            "the_end" | "minecraft:the_end" => Self::TheEnd,
            _ => Self::Custom(id.to_string()),
        }
    }

    /// Reads a dimension from the number the game wrote before 1.16.
    ///
    /// Returns `None` for a number the game does not use, because the launcher
    /// cannot name the dimension the position is in.
    pub fn from_legacy_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Overworld),
            -1 => Some(Self::TheNether),
            1 => Some(Self::TheEnd),
            _ => None,
        }
    }

    /// Reads a dimension from a tag, if the tag holds one.
    pub fn from_tag(tag: &Tag) -> Option<Self> {
        match tag {
            Tag::String(id) => Some(Self::from_id(id)),
            Tag::Int(id) => Self::from_legacy_id(*id),
            Tag::Byte(id) => Self::from_legacy_id(i32::from(*id)),
            _ => None,
        }
    }

    /// Returns the identifier of the dimension.
    ///
    /// The standard dimensions are given with the namespace, and a dimension
    /// the launcher does not know keeps the identifier it was read with.
    pub fn id(&self) -> &str {
        match self {
            Self::Overworld => "minecraft:overworld",
            Self::TheNether => "minecraft:the_nether",
            Self::TheEnd => "minecraft:the_end",
            Self::Custom(id) => id,
        }
    }

    /// Returns the name the launcher shows for the dimension.
    ///
    /// The overworld has no name of its own, because the launcher shows its
    /// positions without naming the dimension; the nether and the end are
    /// shown with the names of their texts, and a dimension the launcher does
    /// not know is shown with its identifier.
    pub fn display_name(&self, i18n: &crate::i18n::I18n) -> String {
        match self {
            Self::Overworld => String::new(),
            Self::TheNether => i18n.i18n("world.info.dimension.the_nether"),
            Self::TheEnd => i18n.i18n("world.info.dimension.the_end"),
            Self::Custom(id) => id.clone(),
        }
    }

    /// Writes a position the way the launcher shows it.
    ///
    /// A position of the overworld is written as three numbers, and a
    /// position of another dimension is written with the name of the
    /// dimension before the numbers, which is what the launcher of Java does.
    pub fn format_position(&self, position: WorldPosition, i18n: &crate::i18n::I18n) -> String {
        let name = self.display_name(i18n);
        if name.is_empty() {
            format!("{position}")
        } else {
            format!("{name} {position}")
        }
    }
}

impl std::fmt::Display for Dimension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.id())
    }
}

/// A position in a world, as the game writes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldPosition {
    /// The position of a block, which the game writes as three whole numbers.
    Block {
        /// The position on the axis of the x.
        x: i32,
        /// The position on the axis of the y.
        y: i32,
        /// The position on the axis of the z.
        z: i32,
    },
    /// The exact position of the player, which the game writes as three
    /// numbers with a decimal point.
    Exact {
        /// The position on the axis of the x.
        x: f64,
        /// The position on the axis of the y.
        y: f64,
        /// The position on the axis of the z.
        z: f64,
    },
}

impl WorldPosition {
    /// Reads a position from a list of three numbers, if the tag holds one.
    pub fn from_list(tag: &Tag) -> Option<Self> {
        let Tag::List(values) = tag else {
            return None;
        };
        if values.len() != 3 {
            return None;
        }

        let x = values[0].as_f64()?;
        let y = values[1].as_f64()?;
        let z = values[2].as_f64()?;
        Some(Self::Exact { x, y, z })
    }

    /// Reads a position from a list of three whole numbers, if the tag holds
    /// one.
    pub fn from_int_array(tag: &Tag) -> Option<Self> {
        let Tag::IntArray(values) = tag else {
            return None;
        };
        if values.len() < 3 {
            return None;
        }

        Some(Self::Block {
            x: values[0],
            y: values[1],
            z: values[2],
        })
    }

    /// Reads a position from the three tags the game wrote before 1.21.9, if
    /// all of them hold a number.
    pub fn from_axes(x: Option<&Tag>, y: Option<&Tag>, z: Option<&Tag>) -> Option<Self> {
        Some(Self::Block {
            x: x?.as_int()?,
            y: y?.as_int()?,
            z: z?.as_int()?,
        })
    }
}

impl std::fmt::Display for WorldPosition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Block { x, y, z } => write!(formatter, "({x}, {y}, {z})"),
            Self::Exact { x, y, z } => write!(formatter, "({x:.2}, {y:.2}, {z:.2})"),
        }
    }
}

/// A position in a world, with the dimension it is in.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldLocation {
    /// The dimension the position is in.
    pub dimension: Dimension,
    /// The position in the dimension.
    pub position: WorldPosition,
}

impl std::fmt::Display for WorldLocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The overworld is left out, because the launcher shows the other
        // dimensions by their name.
        match self.dimension {
            Dimension::Overworld => write!(formatter, "{}", self.position),
            _ => write!(formatter, "{} {}", self.dimension.id(), self.position),
        }
    }
}

impl WorldLocation {
    /// Returns the location the way the launcher shows it.
    ///
    /// The position of the overworld is written without naming the dimension,
    /// and the position of another dimension is written with the name the
    /// launcher shows for the dimension, which is what the launcher of Java
    /// does.
    pub fn display(&self, i18n: &crate::i18n::I18n) -> String {
        self.dimension.format_position(self.position, i18n)
    }
}

/// The difficulty of a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    /// The player cannot be hurt and does not need food.
    Peaceful,
    /// The game is easier than normal.
    Easy,
    /// The game is played as designed.
    Normal,
    /// The game is harder than normal.
    Hard,
}

impl Difficulty {
    /// Returns the number the game writes before 26.1.
    pub const fn id(self) -> i32 {
        match self {
            Self::Peaceful => 0,
            Self::Easy => 1,
            Self::Normal => 2,
            Self::Hard => 3,
        }
    }

    /// Reads a difficulty from the number the game wrote, if it is one.
    pub const fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Peaceful),
            1 => Some(Self::Easy),
            2 => Some(Self::Normal),
            3 => Some(Self::Hard),
            _ => None,
        }
    }

    /// Reads a difficulty from the name the game writes after 26.1, if it is
    /// one.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "peaceful" => Some(Self::Peaceful),
            "easy" => Some(Self::Easy),
            "normal" => Some(Self::Normal),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }

    /// Returns the name the game writes after 26.1.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Peaceful => "peaceful",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }
}

/// The way the player plays a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    /// The player gathers the resources of the world.
    Survival,
    /// The player can fly and has every block.
    Creative,
    /// The player cannot change the world.
    Adventure,
    /// The player sees the world without taking part in it.
    Spectator,
    /// The player plays survival with one life.
    Hardcore,
}

impl GameMode {
    /// Returns the number the game writes for the mode.
    ///
    /// Hardcore is survival with the flag of the hardcore worlds set, so it is
    /// written as survival.
    pub const fn id(self) -> i32 {
        match self {
            Self::Survival | Self::Hardcore => 0,
            Self::Creative => 1,
            Self::Adventure => 2,
            Self::Spectator => 3,
        }
    }

    /// Returns whether the mode is the hardcore flag with survival.
    pub const fn is_hardcore(self) -> bool {
        matches!(self, Self::Hardcore)
    }

    /// Reads the mode from the number and the hardcore flag, if the game uses
    /// the number.
    pub const fn from_id(id: i32, hardcore: bool) -> Option<Self> {
        match id {
            0 if hardcore => Some(Self::Hardcore),
            0 => Some(Self::Survival),
            1 => Some(Self::Creative),
            2 => Some(Self::Adventure),
            3 => Some(Self::Spectator),
            _ => None,
        }
    }
}

/// A world of a player.
#[derive(Debug, Clone)]
pub struct World {
    /// The directory of the world.
    file: PathBuf,
    /// The name of the directory of the world.
    file_name: String,
    /// The file the level of the world is read from.
    level_file: PathBuf,
    /// The compound tag of the level of the world.
    level_data: Tag,
    /// The settings of the generation of the world that are kept in their own
    /// file, with the file they are written back to.
    world_gen_file: Option<(PathBuf, Tag)>,
    /// The data of the player that is kept in its own file, with the file it
    /// is written back to.
    player_file: Option<(PathBuf, Tag)>,
}

impl World {
    /// Reads a world from its directory.
    ///
    /// The directory has to hold a `level.dat`, or a `special_level.dat` for
    /// the snapshot that renamed the file. The level has to name the world and
    /// the time it was last played.
    pub fn open(directory: impl AsRef<Path>) -> Result<Self, WorldError> {
        let directory = directory.as_ref();
        if !directory.is_dir() {
            return Err(WorldError::NotAWorld {
                path: directory.to_path_buf(),
            });
        }

        let level_file = world_level_file(directory).ok_or_else(|| WorldError::NotAWorld {
            path: directory.to_path_buf(),
        })?;

        let level_data = read_compressed(&level_file).map_err(|error| WorldError::Level {
            file: level_file.clone(),
            error,
        })?;

        let data = level_data.get("Data").ok_or_else(|| WorldError::Level {
            file: level_file.clone(),
            error: crate::error::Error::invalid_format(&level_file, "level.dat missing Data"),
        })?;

        if data.get("LevelName").and_then(Tag::as_str).is_none() {
            return Err(WorldError::Level {
                file: level_file.clone(),
                error: crate::error::Error::invalid_format(
                    &level_file,
                    "level.dat missing LevelName",
                ),
            });
        }
        if data.get("LastPlayed").and_then(Tag::as_long).is_none() {
            return Err(WorldError::Level {
                file: level_file.clone(),
                error: crate::error::Error::invalid_format(
                    &level_file,
                    "level.dat missing LastPlayed",
                ),
            });
        }

        let file_name = directory
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        let world_gen_file = read_world_gen_file(directory);
        let player_file = read_player_file(directory, &level_data);

        Ok(Self {
            file: directory.to_path_buf(),
            file_name,
            level_file,
            level_data,
            world_gen_file,
            player_file,
        })
    }

    /// Returns the directory of the world.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the name of the directory of the world.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the compound tag of the level of the world.
    ///
    /// The tag is the one of `level.dat`, so it holds `Data`, `Version` and
    /// the other entries the game writes.
    pub fn level_data(&self) -> &Tag {
        &self.level_data
    }

    /// Returns the data of the world, which the launcher reads the settings of
    /// the world from.
    fn data(&self) -> Option<&Tag> {
        self.level_data.get("Data")
    }

    /// Returns the name of the world.
    pub fn world_name(&self) -> &str {
        self.data()
            .and_then(|data| data.get("LevelName"))
            .and_then(Tag::as_str)
            .unwrap_or("")
    }

    /// Returns when the world was last played, in milliseconds since the epoch.
    pub fn last_played(&self) -> i64 {
        self.data()
            .and_then(|data| data.get("LastPlayed"))
            .and_then(Tag::as_long)
            .unwrap_or(0)
    }

    /// Returns how long the world was played, in ticks of the game.
    ///
    /// The game counts twenty ticks in a second, so the launcher divides the
    /// number of ticks by twenty to tell how long the world was played.
    pub fn time_played(&self) -> Option<i64> {
        self.data()
            .and_then(|data| data.get("Time"))
            .and_then(Tag::as_long)
    }

    /// Returns the game version of the world, if it is known.
    pub fn game_version(&self) -> Option<crate::util::versioning::GameVersionNumber> {
        let data = self.data()?;
        let version = data.get("Version")?.as_compound()?;
        let name = version.get("Name")?.as_str()?;
        Some(crate::util::versioning::GameVersionNumber::as_game_version(
            name,
        ))
    }

    /// Returns the seed of the world, if it is known.
    ///
    /// The seed is read from the settings of the world after 1.16 and from the
    /// data of the world before it.
    pub fn seed(&self) -> Option<i64> {
        let data = self.data()?;
        if let Some(seed) = self
            .world_gen_settings()
            .and_then(|settings| settings.get("seed"))
            .and_then(Tag::as_long)
        {
            return Some(seed);
        }
        data.get("RandomSeed").and_then(Tag::as_long)
    }

    /// Returns the settings of the generation of the world, if the world keeps
    /// them.
    ///
    /// The game writes the settings into the level after 1.16, and into
    /// `data/minecraft/world_gen_settings.dat` on a dedicated server. The
    /// launcher reads both, because a world that was moved between the game
    /// and a server holds either.
    pub fn world_gen_settings(&self) -> Option<&Tag> {
        if let Some(settings) = self
            .data()
            .and_then(|data| data.get("WorldGenSettings"))
            .filter(|tag| tag.as_compound().is_some())
        {
            return Some(settings);
        }

        self.world_gen_file
            .as_ref()
            .and_then(|(_, tag)| tag.get("data"))
            .filter(|tag| tag.as_compound().is_some())
    }

    /// Returns the data of the player of the world, if the world records it.
    ///
    /// The player is written into the level when the world is played alone, and
    /// into `players/data/<uuid>.dat` when it is played on a server. The data
    /// the launcher returns is the compound of the player, not the file it is
    /// kept in.
    pub fn player_data(&self) -> Option<&Tag> {
        if let Some(player) = self
            .data()
            .and_then(|data| data.get("Player"))
            .filter(|tag| tag.as_compound().is_some())
        {
            return Some(player);
        }

        self.player_file.as_ref().map(|(_, tag)| tag)
    }

    /// Returns the mode the player plays the world in, if the world records it.
    ///
    /// The hardcore flag is written next to the mode, and a world with the flag
    /// set is survival with one life, which the launcher shows as its own mode.
    pub fn game_mode(&self) -> Option<GameMode> {
        let player = self.player_data()?;
        let mode = player.get("playerGameType")?.as_int()?;
        let hardcore = self.hardcore().unwrap_or(false);
        GameMode::from_id(mode, hardcore)
    }

    /// Sets the mode the player plays the world in.
    ///
    /// Hardcore is written as survival with the hardcore flag set, like the
    /// game does. The level and the player data are written to the disk, so the
    /// game reads the new mode the next time it starts.
    pub fn set_game_mode(&mut self, mode: GameMode) -> Result<(), WorldError> {
        let hardcore = mode.is_hardcore();
        let path = self.file.clone();
        let player = self.player_data_mut().ok_or(WorldError::MissingField {
            path,
            field: "Player",
        })?;
        player.insert("playerGameType".to_string(), Tag::Int(mode.id()));

        self.set_hardcore(hardcore)?;
        self.write_world_data()
    }

    /// Returns whether the world is played with one life.
    pub fn hardcore(&self) -> Option<bool> {
        let data = self.data()?;
        if let Some(hardcore) = data.get("hardcore").and_then(Tag::as_long) {
            return Some(hardcore == 1);
        }
        data.get("difficulty_settings")
            .and_then(Tag::as_compound)
            .and_then(|settings| settings.get("hardcore"))
            .and_then(Tag::as_long)
            .map(|hardcore| hardcore == 1)
    }

    /// Sets whether the world is played with one life.
    ///
    /// The flag is written where the world keeps it: the game wrote it next to
    /// the difficulty before 26.1 and into the difficulty settings after it.
    /// A world that records the flag in neither place gets it next to the
    /// difficulty, which is the place the game has always read.
    fn set_hardcore(&mut self, hardcore: bool) -> Result<(), WorldError> {
        let value = if hardcore { 1 } else { 0 };
        let data = self.data_mut()?;

        let in_settings = data
            .get("difficulty_settings")
            .and_then(Tag::as_compound)
            .is_some_and(|settings| settings.get("hardcore").is_some());

        if in_settings {
            if let Some(settings) = data
                .get_mut("difficulty_settings")
                .and_then(Tag::as_compound_mut)
            {
                settings.insert("hardcore".to_string(), Tag::Byte(value));
            }
        } else {
            data.insert("hardcore".to_string(), Tag::Byte(value));
        }

        Ok(())
    }

    /// Returns the difficulty of the world, if it is recorded.
    ///
    /// The difficulty is a number before 26.1 and a name after it, so the
    /// launcher reads both.
    pub fn difficulty(&self) -> Option<Difficulty> {
        let data = self.data()?;
        if let Some(difficulty) = data.get("Difficulty").and_then(Tag::as_long) {
            return Difficulty::from_id(difficulty as i32);
        }
        data.get("difficulty_settings")
            .and_then(Tag::as_compound)
            .and_then(|settings| settings.get("difficulty"))
            .and_then(Tag::as_str)
            .and_then(Difficulty::from_name)
    }

    /// Sets the difficulty of the world.
    ///
    /// The difficulty is written in the form the world keeps it in, and the
    /// level is written to the disk.
    pub fn set_difficulty(&mut self, difficulty: Difficulty) -> Result<(), WorldError> {
        let data = self.data_mut()?;

        let in_settings = data
            .get("difficulty_settings")
            .and_then(Tag::as_compound)
            .is_some_and(|settings| settings.get("difficulty").is_some());

        if in_settings {
            if let Some(settings) = data
                .get_mut("difficulty_settings")
                .and_then(Tag::as_compound_mut)
            {
                settings.insert(
                    "difficulty".to_string(),
                    Tag::String(difficulty.name().to_string()),
                );
            }
        } else {
            data.insert("Difficulty".to_string(), Tag::Byte(difficulty.id() as i8));
        }

        self.write_level_data()
    }

    /// Returns whether the difficulty of the world cannot be changed in it.
    pub fn is_difficulty_locked(&self) -> Option<bool> {
        let data = self.data()?;
        if let Some(locked) = data.get("DifficultyLocked").and_then(Tag::as_long) {
            return Some(locked == 1);
        }
        data.get("difficulty_settings")
            .and_then(Tag::as_compound)
            .and_then(|settings| settings.get("locked"))
            .and_then(Tag::as_long)
            .map(|locked| locked == 1)
    }

    /// Locks or unlocks the difficulty of the world.
    pub fn set_difficulty_locked(&mut self, locked: bool) -> Result<(), WorldError> {
        let value = if locked { 1 } else { 0 };
        let data = self.data_mut()?;

        let in_settings = data
            .get("difficulty_settings")
            .and_then(Tag::as_compound)
            .is_some_and(|settings| settings.get("locked").is_some());

        if in_settings {
            if let Some(settings) = data
                .get_mut("difficulty_settings")
                .and_then(Tag::as_compound_mut)
            {
                settings.insert("locked".to_string(), Tag::Byte(value));
            }
        } else {
            data.insert("DifficultyLocked".to_string(), Tag::Byte(value));
        }

        self.write_level_data()
    }

    /// Returns whether the world allows the commands of the game.
    pub fn allow_commands(&self) -> Option<bool> {
        self.data()?
            .get("allowCommands")
            .and_then(Tag::as_long)
            .map(|value| value == 1)
    }

    /// Allows or refuses the commands of the game in the world.
    pub fn set_allow_commands(&mut self, allow: bool) -> Result<(), WorldError> {
        let value = if allow { 1 } else { 0 };
        self.data_mut()?
            .insert("allowCommands".to_string(), Tag::Byte(value));
        self.write_level_data()
    }

    /// Returns whether the world generates the structures of the game, if it is
    /// recorded.
    ///
    /// The flag was written into the level before 1.16 and into the settings of
    /// the generation after it; the name of the setting changed in 26.1.
    pub fn generate_features(&self) -> Option<bool> {
        if let Some(generate) = self.data()?.get("MapFeatures").and_then(Tag::as_long) {
            return Some(generate == 1);
        }

        let settings = self.world_gen_settings()?;
        settings
            .get("generate_features")
            .or_else(|| settings.get("generate_structures"))
            .and_then(Tag::as_long)
            .map(|generate| generate == 1)
    }

    /// Generates or leaves out the structures of the game in the world.
    ///
    /// The flag is written where the world keeps it, and the file it is kept in
    /// is written to the disk.
    pub fn set_generate_features(&mut self, generate: bool) -> Result<(), WorldError> {
        let value = if generate { 1 } else { 0 };

        // The flag before 1.16 is kept in the level and the flag after it in
        // the settings, so the flag is written where the world records it.
        let in_level = self
            .data()
            .is_some_and(|data| data.get("MapFeatures").is_some());
        if in_level {
            self.data_mut()?
                .insert("MapFeatures".to_string(), Tag::Byte(value));
            return self.write_level_data();
        }

        let in_settings = self.world_gen_settings().is_some_and(|settings| {
            settings.get("generate_features").is_some()
                || settings.get("generate_structures").is_some()
        });
        if !in_settings {
            return Err(WorldError::MissingField {
                path: self.file.clone(),
                field: "generate_features",
            });
        }

        // The setting is renamed after 26.1, so the name the world holds is
        // the one that is changed.
        if let Some(settings) = self.world_gen_settings_mut() {
            if settings.contains_key("generate_features") {
                settings.insert("generate_features".to_string(), Tag::Byte(value));
            } else {
                settings.insert("generate_structures".to_string(), Tag::Byte(value));
            }
        }

        self.write_world_data()
    }

    /// Returns where the player starts in the world, if it is recorded.
    ///
    /// The game wrote the spawn as three numbers before 1.21.9 and as a
    /// position with its dimension after it.
    pub fn spawn_point(&self) -> Option<WorldLocation> {
        let data = self.data()?;

        if let Some(spawn) = data.get("spawn").and_then(Tag::as_compound) {
            let position = spawn.get("pos").and_then(WorldPosition::from_int_array)?;
            let dimension = spawn
                .get("dimension")
                .and_then(Dimension::from_tag)
                .unwrap_or(Dimension::Overworld);
            return Some(WorldLocation {
                dimension,
                position,
            });
        }

        let position =
            WorldPosition::from_axes(data.get("SpawnX"), data.get("SpawnY"), data.get("SpawnZ"))?;
        Some(WorldLocation {
            dimension: Dimension::Overworld,
            position,
        })
    }

    /// Returns the position of the player in the world, if it is recorded.
    pub fn player_location(&self) -> Option<WorldLocation> {
        let player = self.player_data()?;
        let dimension = player.get("Dimension").and_then(Dimension::from_tag)?;
        let position = player.get("Pos").and_then(WorldPosition::from_list)?;
        Some(WorldLocation {
            dimension,
            position,
        })
    }

    /// Returns where the player died the last time, if the world records it.
    pub fn last_death_location(&self) -> Option<WorldLocation> {
        let death = self
            .player_data()?
            .get("LastDeathLocation")
            .and_then(Tag::as_compound)?;
        let dimension = death.get("dimension").and_then(Dimension::from_tag)?;
        let position = death.get("pos").and_then(WorldPosition::from_int_array)?;
        Some(WorldLocation {
            dimension,
            position,
        })
    }

    /// Returns where the player comes back after dying, if it is recorded.
    ///
    /// The game wrote the point as three numbers before 25w07a and as a
    /// position with its dimension after it. The dimension was not recorded
    /// before 20w12a, and the player came back to the overworld then.
    pub fn player_spawn_point(&self) -> Option<WorldLocation> {
        let player = self.player_data()?;

        if let Some(respawn) = player.get("respawn").and_then(Tag::as_compound) {
            let dimension = respawn.get("dimension").and_then(Dimension::from_tag)?;
            let position = respawn.get("pos").and_then(WorldPosition::from_int_array)?;
            return Some(WorldLocation {
                dimension,
                position,
            });
        }

        let position = WorldPosition::from_axes(
            player.get("SpawnX"),
            player.get("SpawnY"),
            player.get("SpawnZ"),
        )?;
        let dimension = player
            .get("SpawnDimension")
            .and_then(Dimension::from_tag)
            .unwrap_or(Dimension::Overworld);
        Some(WorldLocation {
            dimension,
            position,
        })
    }

    /// Returns the health of the player, if it is recorded.
    pub fn player_health(&self) -> Option<f32> {
        match self.player_data()?.get("Health")? {
            Tag::Float(value) => Some(*value),
            Tag::Double(value) => Some(*value as f32),
            _ => None,
        }
    }

    /// Returns the food level of the player, if it is recorded.
    pub fn player_food_level(&self) -> Option<i32> {
        self.player_data()?.get("foodLevel")?.as_int()
    }

    /// Returns the saturation of the food of the player, if it is recorded.
    pub fn player_food_saturation(&self) -> Option<f32> {
        match self.player_data()?.get("foodSaturationLevel")? {
            Tag::Float(value) => Some(*value),
            Tag::Double(value) => Some(*value as f32),
            _ => None,
        }
    }

    /// Returns the level of the experience of the player, if it is recorded.
    pub fn player_experience_level(&self) -> Option<i32> {
        self.player_data()?.get("XpLevel")?.as_int()
    }

    /// Sets the health of the player.
    pub fn set_player_health(&mut self, health: f32) -> Result<(), WorldError> {
        self.player_data_mut_or_missing()?
            .insert("Health".to_string(), Tag::Float(health));
        self.write_world_data()
    }

    /// Sets the food level of the player.
    pub fn set_player_food_level(&mut self, level: i32) -> Result<(), WorldError> {
        self.player_data_mut_or_missing()?
            .insert("foodLevel".to_string(), Tag::Int(level));
        self.write_world_data()
    }

    /// Sets the saturation of the food of the player.
    pub fn set_player_food_saturation(&mut self, saturation: f32) -> Result<(), WorldError> {
        self.player_data_mut_or_missing()?
            .insert("foodSaturationLevel".to_string(), Tag::Float(saturation));
        self.write_world_data()
    }

    /// Sets the level of the experience of the player.
    pub fn set_player_experience_level(&mut self, level: i32) -> Result<(), WorldError> {
        self.player_data_mut_or_missing()?
            .insert("XpLevel".to_string(), Tag::Int(level));
        self.write_world_data()
    }

    /// Writes the level of the world to the disk.
    ///
    /// The level is written to the file it was read from, so a snapshot with
    /// `special_level.dat` keeps its name.
    pub fn write_level_data(&self) -> Result<(), WorldError> {
        write_compressed(&self.level_file, &self.level_data).map_err(|error| WorldError::Level {
            file: self.level_file.clone(),
            error,
        })
    }

    /// Writes the level, the settings of the generation and the player to the
    /// disk.
    ///
    /// The files are written in the order the Java launcher writes them, so a
    /// failure leaves the world as close to the state the game expects as it
    /// can be.
    pub fn write_world_data(&self) -> Result<(), WorldError> {
        self.write_level_data()?;

        if let Some((path, tag)) = &self.world_gen_file {
            write_compressed(path, tag).map_err(|error| WorldError::Level {
                file: path.clone(),
                error,
            })?;
        }

        if let Some((path, tag)) = &self.player_file {
            write_compressed(path, tag).map_err(|error| WorldError::Level {
                file: path.clone(),
                error,
            })?;
        }

        Ok(())
    }

    /// Replaces the icon of the world with the image of a file.
    ///
    /// The game only reads an icon that is 64 pixels wide and 64 pixels high,
    /// so an image of another size is refused before anything is written. The
    /// image is copied as it is, because the game reads it as a PNG file.
    pub fn set_icon(&self, image: impl AsRef<Path>) -> Result<(), WorldError> {
        let image = image.as_ref();
        let bytes = std::fs::read(image).map_err(|error| WorldError::InvalidIcon {
            path: image.to_path_buf(),
            reason: error.to_string(),
        })?;

        let (width, height) = png_size(&bytes).ok_or_else(|| WorldError::InvalidIcon {
            path: image.to_path_buf(),
            reason: "the image is not a PNG file".to_string(),
        })?;

        if width != ICON_SIZE || height != ICON_SIZE {
            return Err(WorldError::InvalidIcon {
                path: image.to_path_buf(),
                reason: format!("the image is {width}x{height} instead of {ICON_SIZE}x{ICON_SIZE}"),
            });
        }

        std::fs::write(self.file.join("icon.png"), &bytes).map_err(|error| {
            WorldError::InvalidIcon {
                path: self.file.join("icon.png"),
                reason: error.to_string(),
            }
        })
    }

    /// Removes the icon of the world, so the game shows the default one.
    ///
    /// The icon of a world that has none is not an error, so the launcher can
    /// reset the icon of a world that was reset already.
    pub fn clear_icon(&self) -> Result<(), WorldError> {
        match std::fs::remove_file(self.file.join("icon.png")) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(WorldError::InvalidIcon {
                path: self.file.join("icon.png"),
                reason: error.to_string(),
            }),
        }
    }

    /// Returns the data of the world that can be changed.
    fn data_mut(&mut self) -> Result<&mut crate::util::nbt::Compound, WorldError> {
        let file = self.level_file.clone();
        self.level_data
            .as_compound_mut()
            .and_then(|level| level.get_mut("Data"))
            .and_then(Tag::as_compound_mut)
            .ok_or(WorldError::Level {
                file,
                error: crate::error::Error::invalid_format("level.dat", "level.dat missing Data"),
            })
    }

    /// Returns the settings of the generation that can be changed, if the world
    /// keeps them.
    ///
    /// The settings are kept in the level when the world is played alone, and
    /// in their own file on a server; the settings of the file are returned
    /// under `data`, which the file holds them in.
    fn world_gen_settings_mut(&mut self) -> Option<&mut crate::util::nbt::Compound> {
        if self
            .level_data
            .as_compound()
            .and_then(|level| level.get("Data"))
            .and_then(Tag::as_compound)
            .is_some_and(|data| data.contains_key("WorldGenSettings"))
        {
            return self
                .level_data
                .as_compound_mut()
                .and_then(|level| level.get_mut("Data"))
                .and_then(Tag::as_compound_mut)
                .and_then(|data| data.get_mut("WorldGenSettings"))
                .and_then(Tag::as_compound_mut);
        }

        self.world_gen_file
            .as_mut()
            .and_then(|(_, tag)| tag.as_compound_mut())
            .and_then(|file| file.get_mut("data"))
            .and_then(Tag::as_compound_mut)
    }

    /// Returns the data of the player that can be changed, if the world records
    /// it.
    fn player_data_mut(&mut self) -> Option<&mut crate::util::nbt::Compound> {
        let in_level = self
            .level_data
            .as_compound()
            .and_then(|level| level.get("Data"))
            .and_then(Tag::as_compound)
            .is_some_and(|data| data.contains_key("Player"));
        if in_level {
            return self
                .level_data
                .as_compound_mut()
                .and_then(|level| level.get_mut("Data"))
                .and_then(Tag::as_compound_mut)
                .and_then(|data| data.get_mut("Player"))
                .and_then(Tag::as_compound_mut);
        }

        self.player_file
            .as_mut()
            .and_then(|(_, tag)| tag.as_compound_mut())
    }

    /// Returns the data of the player that can be changed, or the error that
    /// the world does not record the player.
    fn player_data_mut_or_missing(
        &mut self,
    ) -> Result<&mut crate::util::nbt::Compound, WorldError> {
        let path = self.file.clone();
        self.player_data_mut().ok_or(WorldError::MissingField {
            path,
            field: "Player",
        })
    }

    /// Returns whether the world is locked by another process.
    pub fn is_locked(&self) -> bool {
        is_locked(&self.file.join("session.lock"))
    }

    /// Returns whether the world supports data packs.
    pub fn support_data_packs(&self) -> bool {
        self.game_version()
            .map(|version| version.is_at_least("1.13", "17w43a", false))
            .unwrap_or(false)
    }

    /// Returns whether the world supports quick play.
    pub fn support_quick_play(&self) -> bool {
        self.game_version()
            .map(|version| version.is_at_least("1.20", "23w14a", false))
            .unwrap_or(false)
    }

    /// Returns whether the world uses large biomes.
    ///
    /// The launcher reads the generator name before 1.16 and the settings of
    /// the overworld after it.
    pub fn is_large_biomes(&self) -> bool {
        let Some(data) = self.data() else {
            return false;
        };

        // Valid before 1.16.
        if let Some(Tag::String(name)) = data.get("generatorName") {
            if name == "largeBiomes" {
                return true;
            }
        }

        // Unified handling after 1.16.
        let Some(generator) = self
            .world_gen_settings()
            .and_then(|settings| settings.get("dimensions"))
            .and_then(Tag::as_compound)
            .and_then(|dimensions| dimensions.get("minecraft:overworld"))
            .and_then(Tag::as_compound)
            .and_then(|overworld| overworld.get("generator"))
            .and_then(Tag::as_compound)
        else {
            return false;
        };

        // Valid between 1.16 and 1.18.
        if let Some(Tag::Byte(value)) = generator
            .get("biome_source")
            .and_then(Tag::as_compound)
            .and_then(|source| source.get("large_biomes"))
        {
            if *value == 1 {
                return true;
            }
        }

        // Valid after 1.18.
        matches!(
            generator.get("settings"),
            Some(Tag::String(settings)) if settings == "minecraft:large_biomes"
        )
    }

    /// Renames the world.
    ///
    /// The name recorded in the level is changed and the level is written
    /// back before the directory is renamed on the disk, like the Java
    /// launcher does. The world must not be locked.
    pub fn rename(&mut self, new_name: &str) -> Result<(), WorldError> {
        if self.is_locked() {
            return Err(WorldError::WorldLocked {
                path: self.file.clone(),
            });
        }

        self.set_world_name(new_name)?;

        let new_path = self
            .file
            .parent()
            .map(|parent| parent.join(new_name))
            .unwrap_or_else(|| PathBuf::from(new_name));

        std::fs::rename(&self.file, &new_path).map_err(|error| WorldError::RenameFailed {
            path: self.file.clone(),
            error,
        })?;

        // The level file moved with the directory, so the path the world
        // writes the level to follows it.
        self.level_file = new_path.join(
            self.level_file
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("level.dat")),
        );
        if let Some((path, tag)) = self.world_gen_file.take() {
            self.world_gen_file = Some((new_path.join(relative_to_world(&path, &self.file)), tag));
        }
        if let Some((path, tag)) = self.player_file.take() {
            self.player_file = Some((new_path.join(relative_to_world(&path, &self.file)), tag));
        }
        self.file = new_path;
        self.file_name = new_name.to_string();

        Ok(())
    }

    /// Copies the world to a new directory with the name.
    ///
    /// The lock file of the world is not copied, because the copy is not the
    /// world the game runs. The world must not be locked, and no world may
    /// have the name yet.
    pub fn copy(&self, new_name: &str) -> Result<(), WorldError> {
        if self.is_locked() {
            return Err(WorldError::WorldLocked {
                path: self.file.clone(),
            });
        }

        let new_path = self
            .file
            .parent()
            .map(|parent| parent.join(new_name))
            .unwrap_or_else(|| PathBuf::from(new_name));

        if new_path.exists() {
            return Err(WorldError::CopyFailed {
                path: self.file.clone(),
                error: std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "the world already exists",
                ),
            });
        }

        copy_directory(&self.file, &new_path, |path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "session.lock")
        })
        .map_err(|error| WorldError::CopyFailed {
            path: self.file.clone(),
            error,
        })?;

        // The copy is named after its new directory, so the level of the copy
        // is rewritten. The source world is left as it is.
        let mut copied = World::open(&new_path).map_err(|error| {
            let _ = std::fs::remove_dir_all(&new_path);
            WorldError::CopyFailed {
                path: self.file.clone(),
                error: std::io::Error::other(error.to_string()),
            }
        })?;
        copied.set_world_name(new_name).inspect_err(|_| {
            let _ = std::fs::remove_dir_all(&new_path);
        })?;

        Ok(())
    }

    /// Writes the name of the world into the level and to the disk.
    ///
    /// The level is left untouched in memory when the file cannot be written,
    /// so the world still tells the name the game wrote.
    fn set_world_name(&mut self, new_name: &str) -> Result<(), WorldError> {
        let data = self
            .level_data
            .as_compound_mut()
            .and_then(|level| level.get_mut("Data"))
            .and_then(Tag::as_compound_mut)
            .ok_or_else(|| WorldError::Level {
                file: self.level_file.clone(),
                error: crate::error::Error::invalid_format(
                    &self.level_file,
                    "level.dat missing Data",
                ),
            })?;

        let previous = data.insert("LevelName".to_string(), Tag::String(new_name.to_string()));

        if let Err(error) = write_compressed(&self.level_file, &self.level_data) {
            // The level on the disk still holds the previous name, so the
            // level in memory is put back as well.
            if let Some(data) = self
                .level_data
                .as_compound_mut()
                .and_then(|level| level.get_mut("Data"))
                .and_then(Tag::as_compound_mut)
            {
                match previous {
                    Some(previous) => {
                        data.insert("LevelName".to_string(), previous);
                    }
                    None => {
                        data.remove("LevelName");
                    }
                }
            }
            return Err(WorldError::Level {
                file: self.level_file.clone(),
                error,
            });
        }

        Ok(())
    }

    /// Deletes the world.
    ///
    /// The world must not be locked.
    pub fn delete(&self) -> Result<(), WorldError> {
        if self.is_locked() {
            return Err(WorldError::WorldLocked {
                path: self.file.clone(),
            });
        }

        std::fs::remove_dir_all(&self.file).map_err(|error| WorldError::DeleteFailed {
            path: self.file.clone(),
            error,
        })
    }

    /// Writes the world into an archive, under the name of the world.
    ///
    /// The files of the world are stored in a directory named after the world,
    /// so the game and the launcher read the archive as a world. The lock file
    /// is left out, because the archive is not the world the game runs.
    pub fn export(&self, archive: impl AsRef<Path>, world_name: &str) -> Result<(), WorldError> {
        if !self.file.is_dir() {
            return Err(WorldError::NotAWorld {
                path: self.file.clone(),
            });
        }

        let archive = archive.as_ref();
        // An empty name would put the files in the root of the archive, where
        // they would be read as an archive of files instead of a world.
        let name = if world_name.trim().is_empty() {
            self.file_name.as_str()
        } else {
            world_name
        };

        let mut writer =
            crate::util::zip::ZipWriter::create(archive).map_err(|error| WorldError::Archive {
                path: archive.to_path_buf(),
                error,
            })?;

        writer
            .add_directory_filtered(name, &self.file, &|path: &Path| {
                path.file_name()
                    .and_then(|file_name| file_name.to_str())
                    .is_some_and(|file_name| file_name == "session.lock")
            })
            .map_err(|error| WorldError::Archive {
                path: archive.to_path_buf(),
                error,
            })?;

        writer.finish().map_err(|error| WorldError::Archive {
            path: archive.to_path_buf(),
            error,
        })
    }

    /// Installs a world into a saves directory, from a directory or an archive.
    ///
    /// An archive either holds the level of the world at its root or in one
    /// directory, which the launcher accepts so the archives the game and the
    /// launchers write are both read. The world is renamed to the name after it
    /// is installed, so the name the game shows matches the directory. A world
    /// that already has the name is not replaced.
    pub fn install(
        source: impl AsRef<Path>,
        saves_dir: &Path,
        name: &str,
    ) -> Result<(), WorldError> {
        let source = source.as_ref();
        let target = saves_dir.join(name);

        if target.is_dir() {
            return Err(WorldError::CopyFailed {
                path: target,
                error: std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "the world already exists",
                ),
            });
        }

        if source.is_dir() {
            copy_directory(source, &target, |path| {
                path.file_name()
                    .and_then(|file_name| file_name.to_str())
                    .is_some_and(|file_name| file_name == "session.lock")
            })
            .map_err(|error| WorldError::CopyFailed {
                path: source.to_path_buf(),
                error,
            })?;
        } else if source.is_file() {
            if let Err(error) = install_archive(source, &target) {
                let _ = std::fs::remove_dir_all(&target);
                return Err(error);
            }
        } else {
            return Err(WorldError::NotAWorld {
                path: source.to_path_buf(),
            });
        }

        // The name of the world is the name of the directory it is installed
        // as, so the level is rewritten before the caller reads the world.
        let mut world = match World::open(&target) {
            Ok(world) => world,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&target);
                return Err(error);
            }
        };

        world.set_world_name(name).inspect_err(|_| {
            let _ = std::fs::remove_dir_all(&target);
        })
    }

    /// Reads the icon of the world, which the game writes next to the level.
    pub fn icon(&self) -> Option<Vec<u8>> {
        std::fs::read(self.file.join("icon.png")).ok()
    }
}

/// Extracts a world archive into a directory.
///
/// The archive holds the level of the world either at its root or in one
/// directory of its own. The launcher reads both, because the archives written
/// by the game hold the files at the root and the archives of other launchers
/// hold them in a directory.
fn install_archive(archive: &Path, target: &Path) -> Result<(), WorldError> {
    let archive_error = |error| WorldError::Archive {
        path: archive.to_path_buf(),
        error,
    };

    let entries = crate::util::zip::entries(archive).map_err(archive_error)?;
    if entries.iter().any(|entry| entry.name == "level.dat") {
        crate::util::zip::extract(archive, target, |entry, _| !entry.is_symlink())
            .map_err(archive_error)?;
        return Ok(());
    }

    // The root of the archive holds one directory; the files of the world are
    // read under it, so the directory is not part of the world that is written.
    let directories = entries
        .iter()
        .filter_map(|entry| entry.name.split('/').next())
        .filter(|name| !name.is_empty())
        .collect::<std::collections::BTreeSet<_>>();

    let Some(directory) = directories
        .iter()
        .next()
        .copied()
        .filter(|_| directories.len() == 1)
    else {
        return Err(archive_error(crate::util::zip::ZipError::Malformed(
            "the archive of the world holds no level",
        )));
    };

    crate::util::zip::extract_under_directory(archive, target, directory, |entry, _| {
        !entry.is_symlink()
    })
    .map_err(archive_error)?;

    Ok(())
}

/// Returns the worlds in a saves directory.
///
/// A directory that is not a world is left out, because the launcher shows the
/// worlds the player has and a directory the game cannot read is not one.
pub fn get_worlds(saves_dir: &Path) -> Vec<World> {
    let Ok(entries) = std::fs::read_dir(saves_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| World::open(entry.path()).ok())
        .collect()
}

/// The side of the icon the game reads, in pixels.
pub const ICON_SIZE: u32 = 64;

/// Reads the settings of the generation a server keeps in their own file.
///
/// The launcher reads the file when the level does not hold the settings, which
/// is how a world moved from a server is written. A file that cannot be read is
/// left out, because the level still holds the data the launcher shows.
fn read_world_gen_file(directory: &Path) -> Option<(PathBuf, Tag)> {
    let path = directory.join("data/minecraft/world_gen_settings.dat");
    if !path.is_file() {
        return None;
    }

    let tag = read_compressed(&path).ok()?;
    // The file holds the settings under `data`; a file without it is not one
    // the launcher can read.
    tag.get("data")?.as_compound()?;
    Some((path, tag))
}

/// Reads the data of the player the game keeps in its own file.
///
/// The level names the player of a world played alone; when the world was
/// played on a server, the level names the player and the game keeps the data
/// in `players/data/<uuid>.dat`.
fn read_player_file(directory: &Path, level_data: &Tag) -> Option<(PathBuf, Tag)> {
    // The player of a world played alone is in the level, which is read first.
    if level_data
        .get("Data")
        .and_then(|data| data.get("Player"))
        .is_some_and(|player| player.as_compound().is_some())
    {
        return None;
    }

    let uuid = level_data
        .get("Data")
        .and_then(|data| data.get("singleplayer_uuid"))
        .and_then(|tag| match tag {
            Tag::IntArray(values) if values.len() == 4 => Some(uuid_from_int_array(values)),
            _ => None,
        })?;

    let path = directory.join("players/data").join(format!("{uuid}.dat"));
    if !path.is_file() {
        return None;
    }

    Some((path.clone(), read_compressed(&path).ok()?))
}

/// Reads the identifier the game writes as four whole numbers.
///
/// The game writes the identifier of the player as the four parts of its 128
/// bits, most significant first.
fn uuid_from_int_array(values: &[i32]) -> crate::auth::Uuid {
    let mut bytes = [0u8; 16];
    for (index, value) in values.iter().take(4).enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    crate::auth::Uuid::from_bytes(bytes)
}

/// Returns the path of a file of a world relative to the world, so it can be
/// joined to the new directory of a renamed world.
fn relative_to_world(file: &Path, world: &Path) -> PathBuf {
    file.strip_prefix(world)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| {
            file.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| file.to_path_buf())
        })
}

/// Returns the size of a PNG image, if the bytes are one.
///
/// The launcher only reads the header, because the game refuses an icon of
/// another size anyway.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.len() < 24 || bytes[..8] != SIGNATURE {
        return None;
    }

    // The width and the height are the first two numbers of the header of the
    // image, which follows the signature and the length and name of the chunk.
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

/// Returns the file of the level of a world directory.
///
/// The snapshot 20w14infinite names the file `special_level.dat`, so the
/// launcher reads it when the usual name is missing.
fn world_level_file(directory: &Path) -> Option<PathBuf> {
    let level = directory.join("level.dat");
    if level.is_file() {
        return Some(level);
    }

    let special = directory.join("special_level.dat");
    special.is_file().then_some(special)
}

/// Copies a directory, leaving out the paths the filter matches.
fn copy_directory(
    from: &Path,
    to: &Path,
    skip: impl Fn(&Path) -> bool + Copy,
) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;

    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        if skip(&source) {
            continue;
        }

        let destination = to.join(entry.file_name());
        if source.is_dir() {
            copy_directory(&source, &destination, skip)?;
        } else {
            std::fs::copy(&source, &destination)?;
        }
    }

    Ok(())
}

/// Returns whether the lock file of a world is held by another process.
///
/// The game keeps the lock file open for its whole run and takes an exclusive
/// lock on it, so a launcher that cannot open the file for writing sees the
/// world as locked. The launcher does not take a lock itself, because the game
/// is the one that holds it.
fn is_locked(session_lock_file: &Path) -> bool {
    match std::fs::OpenOptions::new()
        .write(true)
        .create(false)
        .open(session_lock_file)
    {
        Ok(_) => false,
        Err(error) => matches!(
            error.kind(),
            std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::WouldBlock
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::nbt::Compound;

    /// Writes a world directory the tests work in.
    fn world(directory: &Path, name: &str) {
        std::fs::create_dir_all(directory).expect("the directory should be made");

        let mut data = Compound::new();
        data.insert("LevelName".to_string(), Tag::String(name.to_string()));
        data.insert("LastPlayed".to_string(), Tag::Long(1_700_000_000_000));
        data.insert("RandomSeed".to_string(), Tag::Long(1234));

        let mut version = Compound::new();
        version.insert("Name".to_string(), Tag::String("1.20.1".to_string()));
        data.insert("Version".to_string(), Tag::Compound(version));

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));

        write_compressed(&directory.join("level.dat"), &Tag::Compound(level))
            .expect("the level should be written");
    }

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-world-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    #[test]
    fn the_world_is_read() {
        let base = directory("read");
        world(&base.join("saves/My World"), "My World");

        let world = World::open(base.join("saves/My World")).expect("the world should be read");

        assert_eq!(world.file_name(), "My World");
        assert_eq!(world.world_name(), "My World");
        assert_eq!(world.last_played(), 1_700_000_000_000);
        assert_eq!(world.seed(), Some(1234));
        assert_eq!(
            world.game_version().map(|version| version.to_string()),
            Some("1.20.1".to_string())
        );
        assert!(world.support_data_packs());
        assert!(world.support_quick_play());
        assert!(!world.is_large_biomes());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_seed_is_read_from_the_settings_after_1_16() {
        let base = directory("seed");
        std::fs::create_dir_all(base.join("saves").join("world")).expect("dir");

        let mut settings = Compound::new();
        settings.insert("seed".to_string(), Tag::Long(-42));

        let mut data = Compound::new();
        data.insert("LevelName".to_string(), Tag::String("world".to_string()));
        data.insert("LastPlayed".to_string(), Tag::Long(0));
        data.insert("WorldGenSettings".to_string(), Tag::Compound(settings));

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));
        write_compressed(&base.join("saves/world/level.dat"), &Tag::Compound(level))
            .expect("the level should be written");

        let world = World::open(base.join("saves/world")).expect("the world should be read");
        assert_eq!(world.seed(), Some(-42));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_large_biomes_are_read_after_1_18() {
        let base = directory("large-biomes");
        std::fs::create_dir_all(base.join("saves").join("world")).expect("dir");

        let mut generator = Compound::new();
        generator.insert(
            "settings".to_string(),
            Tag::String("minecraft:large_biomes".to_string()),
        );

        let mut overworld = Compound::new();
        overworld.insert("generator".to_string(), Tag::Compound(generator));

        let mut dimensions = Compound::new();
        dimensions.insert("minecraft:overworld".to_string(), Tag::Compound(overworld));

        let mut settings = Compound::new();
        settings.insert("dimensions".to_string(), Tag::Compound(dimensions));

        let mut data = Compound::new();
        data.insert("LevelName".to_string(), Tag::String("world".to_string()));
        data.insert("LastPlayed".to_string(), Tag::Long(0));
        data.insert("WorldGenSettings".to_string(), Tag::Compound(settings));

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));
        write_compressed(&base.join("saves/world/level.dat"), &Tag::Compound(level))
            .expect("the level should be written");

        let world = World::open(base.join("saves/world")).expect("the world should be read");
        assert!(world.is_large_biomes());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_worlds_of_a_directory_are_read() {
        let base = directory("list");
        world(&base.join("saves/first"), "First");
        world(&base.join("saves/second"), "Second");
        std::fs::create_dir_all(base.join("saves/not-a-world")).expect("dir");

        let worlds = get_worlds(&base.join("saves"));
        let mut names = worlds
            .iter()
            .map(|world| world.world_name().to_string())
            .collect::<Vec<_>>();
        names.sort();

        assert_eq!(names, vec!["First", "Second"]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_missing_directory_holds_no_world() {
        let base = directory("missing");
        assert!(get_worlds(&base.join("saves")).is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_non_world_directory_is_refused() {
        let base = directory("not");
        assert!(World::open(&base).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_world_without_a_name_is_refused() {
        let base = directory("nameless");
        let directory = base.join("saves/world");
        std::fs::create_dir_all(&directory).expect("dir");

        let mut data = Compound::new();
        data.insert("LastPlayed".to_string(), Tag::Long(0));

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));
        write_compressed(&directory.join("level.dat"), &Tag::Compound(level))
            .expect("the level should be written");

        assert!(World::open(&directory).is_err());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_world_is_renamed() {
        let base = directory("rename");
        world(&base.join("saves/old"), "Old");

        let mut world = World::open(base.join("saves/old")).expect("the world should be read");
        world.rename("New").expect("the world should be renamed");

        assert_eq!(world.file_name(), "New");
        assert_eq!(world.world_name(), "New");
        assert!(base.join("saves/New").is_dir());
        assert!(!base.join("saves/old").exists());

        // The name is written to the level of the renamed world.
        let reopened = World::open(base.join("saves/New")).expect("the world should be read");
        assert_eq!(reopened.world_name(), "New");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_world_is_renamed_twice() {
        let base = directory("rename-twice");
        world(&base.join("saves/first"), "First");

        let mut world = World::open(base.join("saves/first")).expect("the world should be read");
        world.rename("second").expect("the world should be renamed");
        world
            .rename("third")
            .expect("the world should be renamed again");

        assert_eq!(world.file_name(), "third");
        assert_eq!(world.world_name(), "third");
        assert!(base.join("saves/third").is_dir());
        assert!(!base.join("saves/second").exists());

        // The level is written to the renamed directory, not the old one.
        let reopened = World::open(base.join("saves/third")).expect("the world should be read");
        assert_eq!(reopened.world_name(), "third");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_world_is_copied() {
        let base = directory("copy");
        world(&base.join("saves/source"), "Source");
        std::fs::write(base.join("saves/source/session.lock"), b"snowman")
            .expect("the lock should be written");

        let world = World::open(base.join("saves/source")).expect("the world should be read");
        world.copy("Copy").expect("the world should be copied");

        assert!(base.join("saves/source").is_dir());
        assert!(base.join("saves/Copy").is_dir());
        // The lock of the source world is not copied.
        assert!(!base.join("saves/Copy/session.lock").exists());

        let copied = World::open(base.join("saves/Copy")).expect("the copy should be read");
        assert_eq!(copied.world_name(), "Copy");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_world_is_not_copied_over_another() {
        let base = directory("copy-exists");
        world(&base.join("saves/source"), "Source");
        world(&base.join("saves/taken"), "Taken");

        let world = World::open(base.join("saves/source")).expect("the world should be read");
        let error = world.copy("taken").expect_err("the copy should fail");
        assert!(matches!(error, WorldError::CopyFailed { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_world_is_deleted() {
        let base = directory("delete");
        world(&base.join("saves/doomed"), "Doomed");

        let world = World::open(base.join("saves/doomed")).expect("the world should be read");
        world.delete().expect("the world should be deleted");

        assert!(!base.join("saves/doomed").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_world_is_exported_and_installed() {
        let base = directory("export");
        world(&base.join("saves/source"), "Source");
        std::fs::create_dir_all(base.join("saves/source/region")).expect("dir");
        std::fs::write(base.join("saves/source/region/r.0.0.mca"), b"region")
            .expect("the region should be written");
        std::fs::write(base.join("saves/source/session.lock"), b"snowman")
            .expect("the lock should be written");

        let world = World::open(base.join("saves/source")).expect("the world should be read");
        let archive = base.join("export/Source.zip");
        world
            .export(&archive, "Source")
            .expect("the world should be exported");

        // The archive holds the world in a directory of its name and leaves out
        // the lock of the world.
        let entries = crate::util::zip::entries(&archive).expect("the archive should be read");
        let mut names = entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, vec!["Source/level.dat", "Source/region/r.0.0.mca",]);

        let saves = base.join("installed");
        World::install(&archive, &saves, "Installed").expect("the world should be installed");

        let installed =
            World::open(saves.join("Installed")).expect("the installed world should be read");
        // The level is renamed to the name the world is installed as.
        assert_eq!(installed.world_name(), "Installed");
        assert_eq!(
            std::fs::read(saves.join("Installed/region/r.0.0.mca")).expect("the region"),
            b"region"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_archive_with_the_level_at_the_root_is_installed() {
        let base = directory("install-root");
        world(&base.join("source"), "Source");

        // The game writes the level at the root of the archive, so the launcher
        // reads an archive without a directory as a world as well.
        let archive = base.join("root.zip");
        let mut writer =
            crate::util::zip::ZipWriter::create(&archive).expect("the archive should be made");
        writer
            .add_path("level.dat", &base.join("source/level.dat"))
            .expect("the level should be added");
        writer.finish().expect("the archive should be written");

        let saves = base.join("saves");
        World::install(&archive, &saves, "Root").expect("the world should be installed");

        let installed = World::open(saves.join("Root")).expect("the world should be read");
        assert_eq!(installed.world_name(), "Root");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_directory_is_installed_as_a_world() {
        let base = directory("install-directory");
        world(&base.join("source"), "Source");
        std::fs::write(base.join("source/session.lock"), b"snowman").expect("the lock");

        let saves = base.join("saves");
        World::install(base.join("source"), &saves, "Copied")
            .expect("the world should be installed");

        assert!(saves.join("Copied/level.dat").is_file());
        assert!(!saves.join("Copied/session.lock").exists());
        let installed = World::open(saves.join("Copied")).expect("the world should be read");
        assert_eq!(installed.world_name(), "Copied");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_world_is_not_installed_over_another() {
        let base = directory("install-exists");
        world(&base.join("source"), "Source");
        world(&base.join("saves/taken"), "Taken");

        let error = World::install(base.join("source"), &base.join("saves"), "taken")
            .expect_err("the world should not be installed");
        assert!(matches!(error, WorldError::CopyFailed { .. }));

        let taken = World::open(base.join("saves/taken")).expect("the world should be read");
        assert_eq!(taken.world_name(), "Taken");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_archive_without_a_level_is_refused() {
        let base = directory("install-malformed");
        let archive = base.join("bad.zip");
        let mut writer =
            crate::util::zip::ZipWriter::create(&archive).expect("the archive should be made");
        // Two directories, so the launcher cannot tell which one holds the
        // level of the world.
        writer
            .add_file("first/file.txt", b"first")
            .expect("the file should be added");
        writer
            .add_file("second/file.txt", b"second")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");

        let saves = base.join("saves");
        let error =
            World::install(&archive, &saves, "Bad").expect_err("the archive should be refused");
        assert!(matches!(error, WorldError::Archive { .. }));
        assert!(!saves.join("Bad").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_icon_of_the_world_is_read() {
        let base = directory("icon");
        world(&base.join("saves/world"), "World");

        let world = World::open(base.join("saves/world")).expect("the world should be read");
        assert_eq!(world.icon(), None);

        std::fs::write(base.join("saves/world/icon.png"), b"png").expect("the icon");
        assert_eq!(world.icon().as_deref(), Some(b"png".as_slice()));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Returns the bytes of the header of a PNG image of the given size.
    ///
    /// The launcher only reads the header when it checks an icon, so the rest
    /// of the image is left out.
    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    /// Writes a world that records the settings of the information page.
    fn world_with_player(directory: &Path) {
        std::fs::create_dir_all(directory).expect("the directory should be made");

        let mut player = Compound::new();
        player.insert("playerGameType".to_string(), Tag::Int(0));
        player.insert(
            "Dimension".to_string(),
            Tag::String("minecraft:the_nether".to_string()),
        );
        player.insert(
            "Pos".to_string(),
            Tag::List(vec![
                Tag::Double(1.5),
                Tag::Double(64.0),
                Tag::Double(-2.25),
            ]),
        );
        player.insert("SpawnX".to_string(), Tag::Int(10));
        player.insert("SpawnY".to_string(), Tag::Int(70));
        player.insert("SpawnZ".to_string(), Tag::Int(-30));
        player.insert(
            "SpawnDimension".to_string(),
            Tag::String("minecraft:the_end".to_string()),
        );
        player.insert("Health".to_string(), Tag::Float(20.0));
        player.insert("foodLevel".to_string(), Tag::Int(18));
        player.insert("foodSaturationLevel".to_string(), Tag::Float(4.5));
        player.insert("XpLevel".to_string(), Tag::Int(7));

        let mut death = Compound::new();
        death.insert(
            "dimension".to_string(),
            Tag::String("minecraft:overworld".to_string()),
        );
        death.insert("pos".to_string(), Tag::IntArray(vec![1, 2, 3]));
        player.insert("LastDeathLocation".to_string(), Tag::Compound(death));

        let mut spawn = Compound::new();
        spawn.insert(
            "dimension".to_string(),
            Tag::String("minecraft:the_nether".to_string()),
        );
        spawn.insert("pos".to_string(), Tag::IntArray(vec![0, 32, 0]));

        let mut data = Compound::new();
        data.insert("LevelName".to_string(), Tag::String("World".to_string()));
        data.insert("LastPlayed".to_string(), Tag::Long(0));
        data.insert("Time".to_string(), Tag::Long(120_000));
        data.insert("Player".to_string(), Tag::Compound(player));
        data.insert("spawn".to_string(), Tag::Compound(spawn));
        data.insert("Difficulty".to_string(), Tag::Byte(2));
        data.insert("DifficultyLocked".to_string(), Tag::Byte(0));
        data.insert("allowCommands".to_string(), Tag::Byte(1));
        data.insert("MapFeatures".to_string(), Tag::Byte(1));
        data.insert("hardcore".to_string(), Tag::Byte(0));

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));
        write_compressed(&directory.join("level.dat"), &Tag::Compound(level))
            .expect("the level should be written");
    }

    #[test]
    fn the_information_of_the_world_is_read() {
        let base = directory("info");
        world_with_player(&base.join("saves/world"));

        let world = World::open(base.join("saves/world")).expect("the world should be read");

        assert_eq!(world.difficulty(), Some(Difficulty::Normal));
        assert_eq!(world.is_difficulty_locked(), Some(false));
        assert_eq!(world.allow_commands(), Some(true));
        assert_eq!(world.generate_features(), Some(true));
        assert_eq!(world.hardcore(), Some(false));
        assert_eq!(world.game_mode(), Some(GameMode::Survival));

        let spawn = world.spawn_point().expect("the spawn should be read");
        assert_eq!(spawn.dimension, Dimension::TheNether);
        assert_eq!(spawn.position, WorldPosition::Block { x: 0, y: 32, z: 0 });
        assert_eq!(spawn.to_string(), "minecraft:the_nether (0, 32, 0)");

        let location = world
            .player_location()
            .expect("the location should be read");
        assert_eq!(location.dimension, Dimension::TheNether);
        assert_eq!(
            location.to_string(),
            "minecraft:the_nether (1.50, 64.00, -2.25)"
        );

        let death = world
            .last_death_location()
            .expect("the death location should be read");
        assert_eq!(death.dimension, Dimension::Overworld);
        // The overworld is not named, like the launcher shows it.
        assert_eq!(death.to_string(), "(1, 2, 3)");

        let respawn = world
            .player_spawn_point()
            .expect("the spawn of the player should be read");
        assert_eq!(respawn.dimension, Dimension::TheEnd);

        assert_eq!(world.player_health(), Some(20.0));
        assert_eq!(world.player_food_level(), Some(18));
        assert_eq!(world.player_food_saturation(), Some(4.5));
        assert_eq!(world.player_experience_level(), Some(7));

        // The game counts twenty ticks in a second, so 120000 ticks are
        // 100 minutes of play.
        assert_eq!(world.time_played(), Some(120_000));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_information_of_the_world_is_written() {
        let base = directory("info-write");
        world_with_player(&base.join("saves/world"));

        let mut world = World::open(base.join("saves/world")).expect("the world should be read");
        world
            .set_game_mode(GameMode::Hardcore)
            .expect("the mode should be written");
        world
            .set_difficulty(Difficulty::Hard)
            .expect("the difficulty should be written");
        world
            .set_difficulty_locked(true)
            .expect("the lock should be written");
        world
            .set_allow_commands(false)
            .expect("the commands should be refused");
        world
            .set_generate_features(false)
            .expect("the structures should be left out");
        world
            .set_player_health(12.5)
            .expect("the health should be written");

        // The values are written to the level, so a world read again holds
        // them.
        let reread = World::open(base.join("saves/world")).expect("the world should be read");
        assert_eq!(reread.game_mode(), Some(GameMode::Hardcore));
        assert_eq!(reread.hardcore(), Some(true));
        assert_eq!(reread.difficulty(), Some(Difficulty::Hard));
        assert_eq!(reread.is_difficulty_locked(), Some(true));
        assert_eq!(reread.allow_commands(), Some(false));
        assert_eq!(reread.generate_features(), Some(false));
        assert_eq!(reread.player_health(), Some(12.5));

        // Hardcore is survival with the flag of the hardcore worlds set.
        let data = reread.data().expect("the data of the level");
        assert_eq!(
            reread
                .player_data()
                .and_then(|player| player.get("playerGameType"))
                .and_then(Tag::as_int),
            Some(0)
        );
        assert_eq!(data.get("hardcore").and_then(Tag::as_long), Some(1));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_difficulty_is_written_in_the_form_the_world_keeps_it_in() {
        let base = directory("difficulty-form");
        std::fs::create_dir_all(base.join("saves/world")).expect("dir");

        let mut settings = Compound::new();
        settings.insert("difficulty".to_string(), Tag::String("easy".to_string()));
        settings.insert("locked".to_string(), Tag::Byte(0));
        settings.insert("hardcore".to_string(), Tag::Byte(0));

        let mut data = Compound::new();
        data.insert("LevelName".to_string(), Tag::String("world".to_string()));
        data.insert("LastPlayed".to_string(), Tag::Long(0));
        data.insert("difficulty_settings".to_string(), Tag::Compound(settings));

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));
        write_compressed(&base.join("saves/world/level.dat"), &Tag::Compound(level))
            .expect("the level should be written");

        let mut world = World::open(base.join("saves/world")).expect("the world should be read");
        assert_eq!(world.difficulty(), Some(Difficulty::Easy));

        world
            .set_difficulty(Difficulty::Peaceful)
            .expect("the difficulty should be written");
        world
            .set_difficulty_locked(true)
            .expect("the lock should be written");
        world
            .set_game_mode(GameMode::Creative)
            .expect_err("a world without a player cannot hold the mode");

        let reread = World::open(base.join("saves/world")).expect("the world should be read");
        assert_eq!(reread.difficulty(), Some(Difficulty::Peaceful));
        assert_eq!(reread.is_difficulty_locked(), Some(true));
        // The difficulty is a name after 26.1, so the number is not written.
        let data = reread.data().expect("the data of the level");
        assert!(data.get("Difficulty").is_none());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_player_of_a_server_world_is_read() {
        let base = directory("player-file");
        let directory = base.join("saves/world");
        std::fs::create_dir_all(directory.join("players/data")).expect("dir");

        let uuid = crate::auth::Uuid::parse("123e4567-e89b-12d3-a456-426614174000")
            .expect("the identifier should be read");
        let parts = [
            i32::from_be_bytes(uuid.as_bytes()[0..4].try_into().expect("four bytes")),
            i32::from_be_bytes(uuid.as_bytes()[4..8].try_into().expect("four bytes")),
            i32::from_be_bytes(uuid.as_bytes()[8..12].try_into().expect("four bytes")),
            i32::from_be_bytes(uuid.as_bytes()[12..16].try_into().expect("four bytes")),
        ];

        let mut data = Compound::new();
        data.insert("LevelName".to_string(), Tag::String("world".to_string()));
        data.insert("LastPlayed".to_string(), Tag::Long(0));
        data.insert(
            "singleplayer_uuid".to_string(),
            Tag::IntArray(parts.to_vec()),
        );

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));
        write_compressed(&directory.join("level.dat"), &Tag::Compound(level))
            .expect("the level should be written");

        let mut player = Compound::new();
        player.insert("playerGameType".to_string(), Tag::Int(1));
        player.insert("XpLevel".to_string(), Tag::Int(30));
        write_compressed(
            &directory.join(format!("players/data/{uuid}.dat")),
            &Tag::Compound(player),
        )
        .expect("the player should be written");

        let mut world = World::open(&directory).expect("the world should be read");
        assert_eq!(world.player_experience_level(), Some(30));
        assert_eq!(world.game_mode(), Some(GameMode::Creative));

        world
            .set_player_experience_level(31)
            .expect("the level should be written");

        // The player is written to the file it was read from, not the level.
        let reread = World::open(&directory).expect("the world should be read");
        assert_eq!(reread.player_experience_level(), Some(31));
        let level = read_compressed(&directory.join("level.dat")).expect("the level");
        assert!(level
            .get("Data")
            .and_then(|data| data.get("Player"))
            .is_none());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_settings_of_the_generation_of_a_server_world_are_read() {
        let base = directory("world-gen-file");
        let directory = base.join("saves/world");
        std::fs::create_dir_all(directory.join("data/minecraft")).expect("dir");

        let mut data = Compound::new();
        data.insert("LevelName".to_string(), Tag::String("world".to_string()));
        data.insert("LastPlayed".to_string(), Tag::Long(0));

        let mut level = Compound::new();
        level.insert("Data".to_string(), Tag::Compound(data));
        write_compressed(&directory.join("level.dat"), &Tag::Compound(level))
            .expect("the level should be written");

        let mut settings = Compound::new();
        settings.insert("seed".to_string(), Tag::Long(99));
        settings.insert("generate_features".to_string(), Tag::Byte(1));

        let mut file = Compound::new();
        file.insert("data".to_string(), Tag::Compound(settings));
        write_compressed(
            &directory.join("data/minecraft/world_gen_settings.dat"),
            &Tag::Compound(file),
        )
        .expect("the settings should be written");

        let mut world = World::open(&directory).expect("the world should be read");
        assert_eq!(world.seed(), Some(99));
        assert_eq!(world.generate_features(), Some(true));

        world
            .set_generate_features(false)
            .expect("the structures should be left out");

        // The setting is written back to its own file, not into the level.
        let file = read_compressed(&directory.join("data/minecraft/world_gen_settings.dat"))
            .expect("the settings should be read");
        assert_eq!(
            file.get("data")
                .and_then(|data| data.get("generate_features"))
                .and_then(Tag::as_long),
            Some(0)
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_icon_of_the_world_is_changed_and_cleared() {
        let base = directory("icon-edit");
        world(&base.join("saves/world"), "World");

        let world = World::open(base.join("saves/world")).expect("the world should be read");

        let icon = base.join("icon.png");
        std::fs::write(&icon, png(64, 64)).expect("the image should be written");
        world.set_icon(&icon).expect("the icon should be set");
        assert_eq!(
            world.icon().as_deref(),
            Some(std::fs::read(&icon).expect("the image").as_slice())
        );

        // An image of another size is refused, and the old icon is kept.
        let wrong = base.join("wrong.png");
        std::fs::write(&wrong, png(32, 32)).expect("the image should be written");
        let error = world
            .set_icon(&wrong)
            .expect_err("the image should be refused");
        assert!(matches!(error, WorldError::InvalidIcon { .. }));
        assert!(world.icon().is_some());

        // An image that is not a PNG file is refused as well.
        let text = base.join("text.png");
        std::fs::write(&text, b"not an image").expect("the file should be written");
        assert!(world.set_icon(&text).is_err());

        world.clear_icon().expect("the icon should be cleared");
        assert_eq!(world.icon(), None);
        // Clearing an icon the world does not have is not an error.
        world
            .clear_icon()
            .expect("the icon should be cleared again");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_dimensions_are_named_the_way_the_launcher_shows_them() {
        let i18n = crate::i18n::I18n::new(crate::i18n::Locale::English);

        // The overworld has no name of its own.
        assert!(Dimension::Overworld.display_name(&i18n).is_empty());
        assert_eq!(Dimension::TheNether.display_name(&i18n), "The Nether");
        assert_eq!(Dimension::TheEnd.display_name(&i18n), "The End");
        assert_eq!(
            Dimension::Custom("mymod:sky".to_string()).display_name(&i18n),
            "mymod:sky"
        );
    }

    #[test]
    fn a_position_is_written_with_the_name_of_its_dimension() {
        let i18n = crate::i18n::I18n::new(crate::i18n::Locale::English);

        let block = WorldPosition::Block { x: 1, y: 2, z: 3 };
        let exact = WorldPosition::Exact {
            x: 1.5,
            y: 2.25,
            z: -3.75,
        };

        // The overworld is written without a name, like the launcher shows it.
        assert_eq!(
            Dimension::Overworld.format_position(block, &i18n),
            "(1, 2, 3)"
        );
        assert_eq!(
            Dimension::TheNether.format_position(exact, &i18n),
            "The Nether (1.50, 2.25, -3.75)"
        );
        assert_eq!(
            Dimension::Custom("mymod:sky".to_string()).format_position(block, &i18n),
            "mymod:sky (1, 2, 3)"
        );

        // The location is written the same way.
        let location = WorldLocation {
            dimension: Dimension::TheEnd,
            position: block,
        };
        assert_eq!(location.display(&i18n), "The End (1, 2, 3)");
    }
}
