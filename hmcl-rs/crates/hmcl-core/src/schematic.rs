//! The schematics the player keeps in a game directory.
//!
//! A schematic is a building the player saved with a mod, so the launcher only
//! reads the files to show them and to let the player open them; the game and
//! the mods write them. Three formats are read: the files of Litematica, the
//! files of WorldEdit and Sponge, and the structure files of the game.

use std::path::{Path, PathBuf};

use crate::util::nbt::{read_compressed, Tag};
use crate::util::point3::Point3I;
use crate::util::string::{is_blank, substring_after_last};

/// The format of a schematic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchematicType {
    /// The format of Litematica, which holds the regions of the building.
    Litematic,
    /// The structure file of the game, which the structure blocks write.
    NbtStructure,
    /// The format of WorldEdit and of the specification of Sponge.
    Schem,
}

impl SchematicType {
    /// Returns the types of the schematics the launcher reads.
    pub const ALL: [Self; 3] = [Self::Litematic, Self::NbtStructure, Self::Schem];

    /// Returns the extensions the files of the format are named with.
    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Litematic => &["litematic"],
            Self::NbtStructure => &["nbt"],
            Self::Schem => &["schematic", "schem"],
        }
    }

    /// Returns the type of a file, by its extension.
    ///
    /// The extension is compared without regard to the case, because the files
    /// of the players come from the systems that write both cases.
    pub fn of(path: &Path) -> Option<Self> {
        let extension = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| substring_after_last(name, '.').to_ascii_lowercase())?;

        Self::ALL
            .into_iter()
            .find(|kind| kind.extensions().contains(&extension.as_str()))
    }
}

/// The error of reading a schematic.
#[derive(Debug)]
pub enum SchematicError {
    /// The file is not one of the formats the launcher reads.
    Unsupported {
        /// The file that is not a schematic.
        path: PathBuf,
    },
    /// The file cannot be read as the format its extension promises.
    Invalid {
        /// The file that cannot be read.
        path: PathBuf,
        /// The reason the file is rejected.
        reason: String,
    },
}

impl std::fmt::Display for SchematicError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { path } => {
                write!(formatter, "{} is not a schematic", path.display())
            }
            Self::Invalid { path, reason } => {
                write!(
                    formatter,
                    "the schematic {} cannot be read: {reason}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for SchematicError {}

/// A schematic of a game directory.
///
/// The values the format does not hold are empty, and the launcher shows the
/// ones that are there, so a schematic of an old format is read as well.
#[derive(Debug, Clone)]
pub struct Schematic {
    /// The file the schematic is read from.
    file: PathBuf,
    /// The format of the file.
    kind: SchematicType,
    /// The version of the data of the game, which tells the version the
    /// schematic is made for.
    data_version: i32,
    /// The size of the box the schematics fits in.
    enclosing_size: Option<Point3I>,
    /// The version of the format, if the format holds one.
    version: Option<i32>,
    /// The version of the format that follows the main version.
    sub_version: Option<i32>,
    /// The name the file holds, or the name of the file without its extension.
    name: String,
    /// The author the file names, if it names one.
    author: Option<String>,
    /// The description the file holds, if it holds one.
    description: Option<String>,
    /// The moment the schematic was created, in milliseconds since the first
    /// of January 1970.
    time_created: Option<i64>,
    /// The moment the schematic was written last, in milliseconds since the
    /// first of January 1970.
    time_modified: Option<i64>,
    /// The number of the regions of the file, if the format holds them.
    region_count: Option<i32>,
    /// The number of the blocks of the schematic, if the file holds it.
    total_blocks: Option<i32>,
    /// The volume of the schematic, if the file holds it.
    total_volume: Option<i32>,
    /// The materials of the legacy format of WorldEdit.
    materials: Option<String>,
    /// The preview image of the file, as the pixels of a square.
    preview_image_data: Option<Vec<i32>>,
}

impl Schematic {
    /// Returns whether a file is a schematic the launcher reads.
    pub fn is_file_schematic(path: &Path) -> bool {
        path.is_file() && SchematicType::of(path).is_some()
    }

    /// Reads a schematic of any of the formats the launcher reads.
    ///
    /// The format is chosen by the extension of the file, so a file that is
    /// named after another format is refused instead of being read as one.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SchematicError> {
        let path = path.as_ref();
        let kind = SchematicType::of(path).ok_or_else(|| SchematicError::Unsupported {
            path: path.to_path_buf(),
        })?;

        let root = read_compressed(path).map_err(|error| SchematicError::Invalid {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;

        match kind {
            SchematicType::Litematic => Self::read_litematic(path, &root),
            SchematicType::NbtStructure => Self::read_structure(path, &root),
            SchematicType::Schem => Self::read_schem(path, &root),
        }
    }

    /// Reads a schematic of the format of Litematica.
    fn read_litematic(path: &Path, root: &Tag) -> Result<Self, SchematicError> {
        let invalid = |reason: &str| SchematicError::Invalid {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        };

        let version = root
            .get("Version")
            .ok_or_else(|| invalid("the version of the format is missing"))?;
        let version = int_of(version).ok_or_else(|| invalid("the version is not a number"))?;

        let metadata = root
            .get("Metadata")
            .and_then(Tag::as_compound)
            .ok_or_else(|| invalid("the metadata is missing"))?;

        let region_count = root
            .get("Regions")
            .and_then(Tag::as_compound)
            .map(|regions| regions.len() as i32);

        let enclosing_size = metadata
            .get("EnclosingSize")
            .and_then(Tag::as_compound)
            .map(|size| {
                Point3I::new(
                    size.get("x").and_then(int_of).unwrap_or(0),
                    size.get("y").and_then(int_of).unwrap_or(0),
                    size.get("z").and_then(int_of).unwrap_or(0),
                )
            })
            .filter(|size| size.is_positive());

        let sub_version = root.get("SubVersion").and_then(int_of);
        let data_version = root
            .get("MinecraftDataVersion")
            .and_then(int_of)
            .unwrap_or(0);

        let preview_image_data = metadata.get("PreviewImageData").and_then(|tag| match tag {
            Tag::IntArray(pixels) => Some(pixels.clone()),
            _ => None,
        });

        let name = metadata
            .get("Name")
            .and_then(Tag::as_str)
            .filter(|name| !is_blank(name))
            .map(str::to_string)
            .unwrap_or_else(|| file_stem(path));

        Ok(Self {
            file: path.to_path_buf(),
            kind: SchematicType::Litematic,
            data_version,
            enclosing_size,
            version: Some(version),
            sub_version,
            name,
            author: metadata
                .get("Author")
                .and_then(Tag::as_str)
                .map(str::to_string),
            description: metadata
                .get("Description")
                .and_then(Tag::as_str)
                .map(str::to_string),
            time_created: metadata.get("TimeCreated").and_then(long_of),
            time_modified: metadata.get("TimeModified").and_then(long_of),
            region_count: region_count.filter(|count| *count >= 1),
            total_blocks: metadata
                .get("TotalBlocks")
                .and_then(int_of)
                .filter(|blocks| *blocks >= 1),
            total_volume: metadata
                .get("TotalVolume")
                .and_then(int_of)
                .filter(|volume| *volume >= 1),
            materials: None,
            preview_image_data,
        })
    }

    /// Reads a schematic of the format of WorldEdit or of Sponge.
    ///
    /// The legacy format of WorldEdit names the materials with a tag, and the
    /// formats of Sponge name the version of the format instead.
    fn read_schem(path: &Path, root: &Tag) -> Result<Self, SchematicError> {
        if root.get("Materials").is_some() {
            return Self::read_schem_legacy(path, root);
        }
        if root.get("Version").is_some() {
            return Self::read_schem_sponge(path, root);
        }
        if let Some(sponge) = root
            .get("Schematic")
            .and_then(Tag::as_compound)
            .filter(|tag| tag.get("Version").is_some())
        {
            return Self::read_schem_sponge(path, &Tag::Compound(sponge.clone()));
        }

        Err(SchematicError::Invalid {
            path: path.to_path_buf(),
            reason: "the materials and the version of the format are missing".to_string(),
        })
    }

    /// Reads a schematic of the legacy format of WorldEdit.
    fn read_schem_legacy(path: &Path, root: &Tag) -> Result<Self, SchematicError> {
        let materials =
            root.get("Materials")
                .and_then(Tag::as_str)
                .ok_or_else(|| SchematicError::Invalid {
                    path: path.to_path_buf(),
                    reason: "the materials are not a text".to_string(),
                })?;

        let enclosing_size = enclosing_size_of(root, short_of, true);

        Ok(Self {
            file: path.to_path_buf(),
            kind: SchematicType::Schem,
            // The legacy format holds no version of the data of the game.
            data_version: 0,
            enclosing_size,
            version: None,
            sub_version: None,
            name: file_stem(path),
            author: None,
            description: None,
            time_created: None,
            time_modified: None,
            region_count: None,
            total_blocks: None,
            total_volume: None,
            materials: Some(materials.to_string()),
            preview_image_data: None,
        })
    }

    /// Reads a schematic of the formats of Sponge.
    fn read_schem_sponge(path: &Path, root: &Tag) -> Result<Self, SchematicError> {
        let version =
            root.get("Version")
                .and_then(int_of)
                .ok_or_else(|| SchematicError::Invalid {
                    path: path.to_path_buf(),
                    reason: "the version is not a number".to_string(),
                })?;

        // The first format of Sponge holds no version of the data of the game;
        // the version of 1.13.2 is the one its blocks are read with.
        let data_version = root
            .get("DataVersion")
            .map(|tag| int_of(tag).unwrap_or(0))
            .unwrap_or(1_631);

        // The sizes of the format are written as short numbers, so a side of an
        // unsigned short is read back as the number it was written as.
        let enclosing_size = enclosing_size_of(root, short_of, true);

        let metadata = root.get("Metadata").and_then(Tag::as_compound);
        let name = metadata
            .and_then(|metadata| metadata.get("Name"))
            .and_then(Tag::as_str)
            .filter(|name| !is_blank(name))
            .map(str::to_string)
            .unwrap_or_else(|| file_stem(path));

        Ok(Self {
            file: path.to_path_buf(),
            kind: SchematicType::Schem,
            data_version,
            enclosing_size,
            version: Some(version).filter(|version| *version > 0),
            sub_version: None,
            name,
            author: metadata
                .and_then(|metadata| metadata.get("Author"))
                .and_then(Tag::as_str)
                .map(str::to_string),
            description: None,
            time_created: None,
            time_modified: None,
            region_count: None,
            total_blocks: None,
            total_volume: None,
            materials: None,
            preview_image_data: None,
        })
    }

    /// Reads a structure file of the game.
    fn read_structure(path: &Path, root: &Tag) -> Result<Self, SchematicError> {
        let invalid = |reason: &str| SchematicError::Invalid {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        };

        if !matches!(root.get("blocks"), Some(Tag::List(_))) {
            return Err(invalid("the blocks are missing"));
        }
        if !matches!(root.get("entities"), Some(Tag::List(_))) {
            return Err(invalid("the entities are missing"));
        }

        let size = match root.get("size") {
            None => return Err(invalid("the size is missing")),
            Some(Tag::List(size)) => size,
            Some(_) => return Err(invalid("the size is not a list")),
        };
        if size.len() != 3 {
            return Err(invalid("the size does not hold three numbers"));
        }

        let enclosing_size = Point3I::new(
            size.first().and_then(int_of).unwrap_or(0),
            size.get(1).and_then(int_of).unwrap_or(0),
            size.get(2).and_then(int_of).unwrap_or(0),
        );
        let enclosing_size = enclosing_size.is_positive().then_some(enclosing_size);

        Ok(Self {
            file: path.to_path_buf(),
            kind: SchematicType::NbtStructure,
            // A structure file without a version is made by 1.10, which is the
            // version of the data the structure blocks of 1.10 write.
            data_version: root.get("DataVersion").and_then(int_of).unwrap_or(500),
            enclosing_size,
            version: None,
            sub_version: None,
            name: file_stem(path),
            author: root.get("author").and_then(Tag::as_str).map(str::to_string),
            description: None,
            time_created: None,
            time_modified: None,
            region_count: None,
            total_blocks: None,
            total_volume: None,
            materials: None,
            preview_image_data: None,
        })
    }

    /// Returns the file the schematic is read from.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the format of the schematic.
    pub fn kind(&self) -> SchematicType {
        self.kind
    }

    /// Returns the version of the format, if the format holds one.
    pub fn version(&self) -> Option<i32> {
        self.version
    }

    /// Returns the version of the format that follows the main version.
    pub fn sub_version(&self) -> Option<i32> {
        self.sub_version
    }

    /// Returns the version of the data of the game the schematic is made for.
    ///
    /// The versions below one hundred are the ones of the files that hold no
    /// version of the data, so they are left out.
    pub fn minecraft_data_version(&self) -> Option<i32> {
        (self.data_version >= 100).then_some(self.data_version)
    }

    /// Returns the version of the game the schematic is made for.
    ///
    /// The legacy format of WorldEdit names the materials of the blocks
    /// instead of the version of the game, so the caller shows the text.
    pub fn minecraft_version(&self) -> Option<String> {
        match &self.materials {
            Some(materials) => Some(materials.clone()),
            None => self
                .minecraft_data_version()
                .map(|version| version.to_string()),
        }
    }

    /// Returns the name of the schematic.
    ///
    /// The name the file holds is used, and the name of the file without its
    /// extension when the file holds none.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the author the file names, if it names one.
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// Returns the description the file holds, if it holds one.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the moment the schematic was created.
    pub fn time_created(&self) -> Option<i64> {
        self.time_created
    }

    /// Returns the moment the schematic was written last.
    pub fn time_modified(&self) -> Option<i64> {
        self.time_modified
    }

    /// Returns the number of the regions of the schematic.
    pub fn region_count(&self) -> Option<i32> {
        self.region_count
    }

    /// Returns the number of the blocks of the schematic.
    pub fn total_blocks(&self) -> Option<i32> {
        self.total_blocks
    }

    /// Returns the size of the box the schematic fits in.
    pub fn enclosing_size(&self) -> Option<Point3I> {
        self.enclosing_size
    }

    /// Returns the volume of the schematic.
    ///
    /// The volume the file holds is used, and the volume of the box when the
    /// file holds none.
    pub fn total_volume(&self) -> Option<i64> {
        match self.total_volume {
            Some(volume) => Some(i64::from(volume)),
            None => self.enclosing_size.map(|size| size.volume()),
        }
    }

    /// Returns the pixels of the preview image of the file, if it holds one.
    pub fn preview_image_data(&self) -> Option<&[i32]> {
        self.preview_image_data.as_deref()
    }

    /// Returns the size of the square of the preview image, if the pixels make
    /// a square.
    ///
    /// The formats store the image as a square of pixels, so a number of the
    /// pixels that is not a square is not an image.
    pub fn preview_image_size(&self) -> Option<usize> {
        let pixels = self.preview_image_data.as_ref()?;
        if pixels.is_empty() {
            return None;
        }

        let size = (pixels.len() as f64).sqrt() as usize;
        (size * size == pixels.len()).then_some(size)
    }
}

/// Reads the size of a schematic from the tags of a width, a height and a
/// length.
///
/// The size is only read when every side is positive, because a box without a
/// side holds nothing.
fn enclosing_size_of(
    root: &Tag,
    number: impl Fn(&Tag) -> Option<i64>,
    unsigned: bool,
) -> Option<Point3I> {
    let side = |name: &str| {
        let value = root.get(name).and_then(&number)?;
        Some(if unsigned {
            // The sides of the formats of Sponge are written as unsigned
            // shorts, so a side above 32767 is read as a negative short.
            value & 0xFFFF
        } else {
            value
        })
    };

    let size = Point3I::new(
        side("Width")? as i32,
        side("Height")? as i32,
        side("Length")? as i32,
    );

    size.is_positive().then_some(size)
}

/// Returns the number of a tag, if it is one and it fits.
fn int_of(tag: &Tag) -> Option<i32> {
    let value = tag.as_long()?;
    i32::try_from(value).ok()
}

/// Returns the number of a tag as a whole number of two bytes, if it is one.
fn short_of(tag: &Tag) -> Option<i64> {
    let value = tag.as_long()?;
    i16::try_from(value).ok().map(i64::from)
}

/// Returns the number of a tag as a whole number of eight bytes, if it is one.
fn long_of(tag: &Tag) -> Option<i64> {
    tag.as_long()
}

/// Returns the name of a file without its extension.
fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string()
}

/// The directory the schematics of a game directory are kept in.
pub const SCHEMATICS_DIRECTORY: &str = "schematics";

/// The configuration of Litematica, which may move the schematics.
const LITEMATICA_CONFIG: &str = "config/litematica.json";

/// Returns the directory the schematics of a game directory are kept in.
///
/// Litematica reads the schematics from a directory the player may move, which
/// it records in its configuration. A relative directory is read from the game
/// directory, like the mod does, and a configuration that cannot be read leaves
/// the directory where it usually is.
pub fn schematics_directory(run_directory: &Path) -> PathBuf {
    let default = run_directory.join(SCHEMATICS_DIRECTORY);

    let Ok(text) = std::fs::read_to_string(run_directory.join(LITEMATICA_CONFIG)) else {
        return default;
    };
    let Ok(config) = serde_json::from_str::<serde_json::Value>(&text) else {
        return default;
    };

    let Some(generic) = config.get("Generic") else {
        return default;
    };
    if generic
        .get("customSchematicBaseDirectoryEnabled")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return default;
    }

    let Some(directory) = generic
        .get("customSchematicBaseDirectory")
        .and_then(serde_json::Value::as_str)
        .filter(|directory| !is_blank(directory))
    else {
        return default;
    };

    let directory = Path::new(directory);
    if directory.is_absolute() {
        normalize(directory)
    } else {
        normalize(&run_directory.join(directory))
    }
}

/// Returns a path without the parts that do not name a file.
///
/// The launcher keeps the path as the player wrote it, so a path with a part
/// that goes up is read as the directory it names.
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            part => result.push(part),
        }
    }

    result
}

/// An item of the list of the schematics of a directory.
#[derive(Debug, Clone)]
pub enum SchematicEntry {
    /// A directory that holds the schematics, which the player opens.
    Directory {
        /// The directory.
        path: PathBuf,
        /// The number of the directories and the schematics it holds.
        size: usize,
    },
    /// A schematic file.
    ///
    /// The schematic is held behind a pointer, because it is much larger than
    /// a directory of the list and the list holds many of them.
    File(Box<Schematic>),
}

impl SchematicEntry {
    /// Returns the path of the directory or the file.
    pub fn path(&self) -> &Path {
        match self {
            Self::Directory { path, .. } => path,
            Self::File(schematic) => schematic.file(),
        }
    }

    /// Returns the name the launcher shows.
    pub fn name(&self) -> String {
        match self {
            Self::Directory { path, .. } => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string(),
            Self::File(schematic) => schematic.name().to_string(),
        }
    }

    /// Returns whether the item is a directory.
    pub fn is_directory(&self) -> bool {
        matches!(self, Self::Directory { .. })
    }
}

/// Returns the directories and the schematics of a directory.
///
/// The directories come first, so the player can walk into them, and the
/// schematics follow in the order of their names. A directory that cannot be
/// read holds nothing.
pub fn list_schematics(directory: &Path) -> Vec<SchematicEntry> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            let size = std::fs::read_dir(&path)
                .map(|children| {
                    children
                        .flatten()
                        .filter(|child| {
                            let child = child.path();
                            child.is_dir() || Schematic::is_file_schematic(&child)
                        })
                        .count()
                })
                .unwrap_or(0);
            items.push(SchematicEntry::Directory { path, size });
        } else if let Ok(schematic) = Schematic::open(&path) {
            items.push(SchematicEntry::File(Box::new(schematic)));
        }
    }

    items.sort_by(|left, right| {
        right
            .is_directory()
            .cmp(&left.is_directory())
            .then_with(|| left.name().cmp(&right.name()))
    });
    items
}

/// Copies a schematic into a directory.
///
/// A file that already has the name is not replaced, so a schematic the player
/// has is not lost when they add one with the same name.
pub fn import_schematic(directory: &Path, path: &Path) -> Result<PathBuf, SchematicError> {
    if !Schematic::is_file_schematic(path) {
        return Err(SchematicError::Unsupported {
            path: path.to_path_buf(),
        });
    }

    std::fs::create_dir_all(directory).map_err(|error| SchematicError::Invalid {
        path: directory.to_path_buf(),
        reason: error.to_string(),
    })?;

    let target = directory.join(path.file_name().ok_or_else(|| SchematicError::Invalid {
        path: path.to_path_buf(),
        reason: "the schematic has no name".to_string(),
    })?);

    if target.exists() {
        return Err(SchematicError::Invalid {
            path: target,
            reason: "the schematic already exists".to_string(),
        });
    }

    std::fs::copy(path, &target).map_err(|error| SchematicError::Invalid {
        path: target.clone(),
        reason: error.to_string(),
    })?;

    Ok(target)
}

/// Creates a directory for the schematics.
///
/// The name is checked, because a name the file system cannot hold would make
/// the directory impossible to open.
pub fn create_schematic_directory(parent: &Path, name: &str) -> std::io::Result<PathBuf> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the name of the directory is not valid",
        ));
    }

    let target = parent.join(name);
    if target.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "the directory already exists",
        ));
    }

    std::fs::create_dir_all(&target)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::nbt::{write_compressed, Compound};

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-schematic-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    /// Writes a schematic of the format of Litematica.
    fn litematic(path: &Path, name: &str) {
        let mut metadata = Compound::new();
        metadata.insert("Name".to_string(), Tag::String(name.to_string()));
        metadata.insert("Author".to_string(), Tag::String("Glavo".to_string()));
        metadata.insert(
            "Description".to_string(),
            Tag::String("A house".to_string()),
        );
        metadata.insert("TimeCreated".to_string(), Tag::Long(1_700_000_000_000));
        metadata.insert("TimeModified".to_string(), Tag::Long(1_700_000_001_000));
        metadata.insert("TotalBlocks".to_string(), Tag::Int(64));
        metadata.insert("TotalVolume".to_string(), Tag::Int(128));
        metadata.insert(
            "EnclosingSize".to_string(),
            Tag::Compound({
                let mut size = Compound::new();
                size.insert("x".to_string(), Tag::Int(4));
                size.insert("y".to_string(), Tag::Int(4));
                size.insert("z".to_string(), Tag::Int(4));
                size
            }),
        );
        metadata.insert(
            "PreviewImageData".to_string(),
            Tag::IntArray(vec![0xFF00_00FFu32 as i32; 4]),
        );

        let mut regions = Compound::new();
        regions.insert("Main".to_string(), Tag::Compound(Compound::new()));

        let mut root = Compound::new();
        root.insert("Version".to_string(), Tag::Int(6));
        root.insert("SubVersion".to_string(), Tag::Int(1));
        root.insert("MinecraftDataVersion".to_string(), Tag::Int(3_465));
        root.insert("Metadata".to_string(), Tag::Compound(metadata));
        root.insert("Regions".to_string(), Tag::Compound(regions));

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the directory should be made");
        }
        write_compressed(path, &Tag::Compound(root)).expect("the schematic should be written");
    }

    #[test]
    fn the_types_of_the_files_are_read_from_the_extensions() {
        assert_eq!(
            SchematicType::of(Path::new("a.litematic")),
            Some(SchematicType::Litematic)
        );
        assert_eq!(
            SchematicType::of(Path::new("a.NBT")),
            Some(SchematicType::NbtStructure)
        );
        assert_eq!(
            SchematicType::of(Path::new("a.schem")),
            Some(SchematicType::Schem)
        );
        assert_eq!(
            SchematicType::of(Path::new("a.schematic")),
            Some(SchematicType::Schem)
        );
        assert_eq!(SchematicType::of(Path::new("a.jar")), None);
        assert_eq!(SchematicType::of(Path::new("a")), None);
    }

    #[test]
    fn the_litematic_is_read() {
        let base = directory("litematic");
        let path = base.join("House.litematic");
        litematic(&path, "My House");

        assert!(Schematic::is_file_schematic(&path));

        let schematic = Schematic::open(&path).expect("the schematic should be read");
        assert_eq!(schematic.kind(), SchematicType::Litematic);
        assert_eq!(schematic.name(), "My House");
        assert_eq!(schematic.author(), Some("Glavo"));
        assert_eq!(schematic.description(), Some("A house"));
        assert_eq!(schematic.version(), Some(6));
        assert_eq!(schematic.sub_version(), Some(1));
        assert_eq!(schematic.minecraft_data_version(), Some(3_465));
        assert_eq!(schematic.minecraft_version().as_deref(), Some("3465"));
        assert_eq!(schematic.region_count(), Some(1));
        assert_eq!(schematic.total_blocks(), Some(64));
        assert_eq!(schematic.enclosing_size(), Some(Point3I::new(4, 4, 4)));
        // The volume the file holds is used instead of the box.
        assert_eq!(schematic.total_volume(), Some(128));
        assert_eq!(schematic.time_created(), Some(1_700_000_000_000));
        assert_eq!(schematic.time_modified(), Some(1_700_000_001_000));
        assert_eq!(schematic.preview_image_size(), Some(2));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_litematic_without_a_name_uses_the_file_name() {
        let base = directory("litematic-name");
        let path = base.join("Tower.litematic");
        litematic(&path, "  ");

        let schematic = Schematic::open(&path).expect("the schematic should be read");
        assert_eq!(schematic.name(), "Tower");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_litematic_without_a_version_is_refused() {
        let base = directory("litematic-version");
        let path = base.join("Bad.litematic");

        let mut root = Compound::new();
        root.insert("Metadata".to_string(), Tag::Compound(Compound::new()));
        write_compressed(&path, &Tag::Compound(root)).expect("the file should be written");

        let error = Schematic::open(&path).expect_err("the file should be refused");
        assert!(matches!(error, SchematicError::Invalid { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_schematic_of_the_legacy_format_is_read() {
        let base = directory("schem-legacy");
        let path = base.join("Old.schematic");

        let mut root = Compound::new();
        root.insert("Materials".to_string(), Tag::String("Alpha".to_string()));
        root.insert("Width".to_string(), Tag::Short(3));
        root.insert("Height".to_string(), Tag::Short(5));
        root.insert("Length".to_string(), Tag::Short(7));
        write_compressed(&path, &Tag::Compound(root)).expect("the file should be written");

        let schematic = Schematic::open(&path).expect("the schematic should be read");
        assert_eq!(schematic.kind(), SchematicType::Schem);
        assert_eq!(schematic.name(), "Old");
        assert_eq!(schematic.version(), None);
        assert_eq!(schematic.minecraft_data_version(), None);
        // The legacy format names the materials instead of the version.
        assert_eq!(schematic.minecraft_version().as_deref(), Some("Alpha"));
        assert_eq!(schematic.enclosing_size(), Some(Point3I::new(3, 5, 7)));
        assert_eq!(schematic.total_volume(), Some(105));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_schematic_of_sponge_is_read() {
        let base = directory("schem-sponge");
        let path = base.join("Sponge.schem");

        let mut metadata = Compound::new();
        metadata.insert("Name".to_string(), Tag::String("Sponge House".to_string()));
        metadata.insert("Author".to_string(), Tag::String("Calboot".to_string()));

        let mut root = Compound::new();
        root.insert("Version".to_string(), Tag::Int(2));
        root.insert("DataVersion".to_string(), Tag::Int(3_465));
        // A side above 32767 is written as a negative short.
        root.insert("Width".to_string(), Tag::Short(-32_768));
        root.insert("Height".to_string(), Tag::Short(2));
        root.insert("Length".to_string(), Tag::Short(3));
        root.insert("Metadata".to_string(), Tag::Compound(metadata));
        write_compressed(&path, &Tag::Compound(root)).expect("the file should be written");

        let schematic = Schematic::open(&path).expect("the schematic should be read");
        assert_eq!(schematic.name(), "Sponge House");
        assert_eq!(schematic.author(), Some("Calboot"));
        assert_eq!(schematic.version(), Some(2));
        assert_eq!(schematic.minecraft_data_version(), Some(3_465));
        assert_eq!(schematic.enclosing_size(), Some(Point3I::new(32_768, 2, 3)));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_schematic_of_sponge_without_a_version_of_the_data_is_read() {
        let base = directory("schem-sponge-old");
        let path = base.join("Old.schem");

        let mut root = Compound::new();
        root.insert("Version".to_string(), Tag::Int(1));
        root.insert("Width".to_string(), Tag::Short(1));
        root.insert("Height".to_string(), Tag::Short(1));
        root.insert("Length".to_string(), Tag::Short(1));
        write_compressed(&path, &Tag::Compound(root)).expect("the file should be written");

        let schematic = Schematic::open(&path).expect("the schematic should be read");
        // The blocks of the first format of Sponge are read with 1.13.2.
        assert_eq!(schematic.minecraft_data_version(), Some(1_631));
        assert_eq!(schematic.name(), "Old");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_structure_file_is_read() {
        let base = directory("structure");
        let path = base.join("House.nbt");

        let mut root = Compound::new();
        root.insert("blocks".to_string(), Tag::List(Vec::new()));
        root.insert("entities".to_string(), Tag::List(Vec::new()));
        root.insert(
            "size".to_string(),
            Tag::List(vec![Tag::Int(2), Tag::Int(3), Tag::Int(4)]),
        );
        root.insert("DataVersion".to_string(), Tag::Int(3_465));
        root.insert("author".to_string(), Tag::String("Glavo".to_string()));
        write_compressed(&path, &Tag::Compound(root)).expect("the file should be written");

        let schematic = Schematic::open(&path).expect("the schematic should be read");
        assert_eq!(schematic.kind(), SchematicType::NbtStructure);
        assert_eq!(schematic.name(), "House");
        assert_eq!(schematic.author(), Some("Glavo"));
        assert_eq!(schematic.enclosing_size(), Some(Point3I::new(2, 3, 4)));
        assert_eq!(schematic.total_volume(), Some(24));
        assert_eq!(schematic.minecraft_data_version(), Some(3_465));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_structure_file_without_blocks_is_refused() {
        let base = directory("structure-blocks");
        let path = base.join("Bad.nbt");

        let mut root = Compound::new();
        root.insert("entities".to_string(), Tag::List(Vec::new()));
        root.insert(
            "size".to_string(),
            Tag::List(vec![Tag::Int(1), Tag::Int(1), Tag::Int(1)]),
        );
        write_compressed(&path, &Tag::Compound(root)).expect("the file should be written");

        let error = Schematic::open(&path).expect_err("the file should be refused");
        assert!(matches!(error, SchematicError::Invalid { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_schematics_of_the_repository_are_read() {
        // The files the Java version of the launcher tests itself with hold
        // the root names the game writes, so reading them checks the format.
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../HMCLCore/src/test/resources/schematics");
        if !base.is_dir() {
            return;
        }

        let litematic = Schematic::open(base.join("test.litematic"))
            .expect("the litematic of the repository should be read");
        assert_eq!(litematic.kind(), SchematicType::Litematic);
        assert_eq!(litematic.region_count(), Some(1));
        assert_eq!(litematic.total_blocks(), Some(1_334));
        assert_eq!(litematic.total_volume(), Some(5_746));
        assert_eq!(litematic.author(), Some("hsds"));
        assert_eq!(litematic.minecraft_data_version(), Some(4_325));
        // The file names its region in Chinese, so the text of the file is
        // read with the length of its bytes.
        assert!(litematic.name().chars().count() > 8);

        let structure = Schematic::open(base.join("test.nbt"))
            .expect("the structure of the repository should be read");
        assert_eq!(structure.kind(), SchematicType::NbtStructure);
        assert_eq!(structure.enclosing_size(), Some(Point3I::new(9, 11, 13)));
        assert_eq!(structure.total_volume(), Some(1_287));
        assert_eq!(structure.minecraft_data_version(), Some(3_465));

        let legacy = Schematic::open(base.join("test.schem"))
            .expect("the schematic of the repository should be read");
        assert_eq!(legacy.kind(), SchematicType::Schem);
        assert_eq!(legacy.enclosing_size(), Some(Point3I::new(9, 5, 9)));
        assert_eq!(legacy.minecraft_version().as_deref(), Some("3465"));

        let sponge = Schematic::open(base.join("test.schematic"))
            .expect("the sponge schematic of the repository should be read");
        assert_eq!(sponge.kind(), SchematicType::Schem);
        assert_eq!(sponge.enclosing_size(), Some(Point3I::new(28, 35, 18)));
        assert_eq!(sponge.minecraft_version().as_deref(), Some("Alpha"));
    }

    #[test]
    fn a_file_of_another_format_is_refused() {
        let base = directory("unsupported");
        let path = base.join("mod.jar");
        std::fs::write(&path, b"jar").expect("the file should be written");

        let error = Schematic::open(&path).expect_err("the file should be refused");
        assert!(matches!(error, SchematicError::Unsupported { .. }));
        assert!(!Schematic::is_file_schematic(&path));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_schematics_directory_is_read_from_the_configuration() {
        let base = directory("config");
        // Without a configuration the mod reads the directory of the game.
        assert_eq!(schematics_directory(&base), base.join("schematics"));

        std::fs::create_dir_all(base.join("config")).expect("dir");
        std::fs::write(
            base.join("config/litematica.json"),
            r#"{"Generic":{"customSchematicBaseDirectoryEnabled":true,"customSchematicBaseDirectory":"my_schematics"}}"#,
        )
        .expect("write");
        // A relative directory is read from the game directory.
        assert_eq!(schematics_directory(&base), base.join("my_schematics"));

        // A path with a part that goes up is read as the directory it names.
        std::fs::write(
            base.join("config/litematica.json"),
            r#"{"Generic":{"customSchematicBaseDirectoryEnabled":true,"customSchematicBaseDirectory":"a/../schematics"}}"#,
        )
        .expect("write");
        assert_eq!(schematics_directory(&base), base.join("schematics"));

        // A configuration that leaves the directory alone is ignored.
        std::fs::write(
            base.join("config/litematica.json"),
            r#"{"Generic":{"customSchematicBaseDirectoryEnabled":false,"customSchematicBaseDirectory":"other"}}"#,
        )
        .expect("write");
        assert_eq!(schematics_directory(&base), base.join("schematics"));

        // A configuration that cannot be read leaves the default directory.
        std::fs::write(base.join("config/litematica.json"), "{").expect("write");
        assert_eq!(schematics_directory(&base), base.join("schematics"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_directories_and_the_schematics_of_a_directory_are_listed() {
        let base = directory("list");
        std::fs::create_dir_all(base.join("Builds")).expect("dir");
        litematic(&base.join("Tower.litematic"), "Tower");
        litematic(&base.join("House.litematic"), "House");
        std::fs::write(base.join("notes.txt"), b"text").expect("write");

        let items = list_schematics(&base);
        let names = items.iter().map(SchematicEntry::name).collect::<Vec<_>>();
        // The directories come first, then the schematics by their names.
        assert_eq!(names, vec!["Builds", "House", "Tower"]);
        assert!(items[0].is_directory());
        assert!(!items[1].is_directory());
        assert_eq!(items[1].path(), base.join("House.litematic"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_number_of_the_items_of_a_directory_is_counted() {
        let base = directory("count");
        std::fs::create_dir_all(base.join("Builds/Inner")).expect("dir");
        litematic(&base.join("Builds/One.litematic"), "One");
        std::fs::write(base.join("Builds/notes.txt"), b"text").expect("write");

        let items = list_schematics(&base);
        let SchematicEntry::Directory { size, .. } = &items[0] else {
            panic!("the directory should come first");
        };
        // The inner directory and the schematic are counted, the notes are not.
        assert_eq!(*size, 2);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_schematic_is_imported() {
        let base = directory("import");
        litematic(&base.join("source/Tower.litematic"), "Tower");

        let target = import_schematic(
            &base.join("schematics"),
            &base.join("source/Tower.litematic"),
        )
        .expect("the schematic should be imported");
        assert_eq!(target, base.join("schematics/Tower.litematic"));
        assert!(target.is_file());

        let error = import_schematic(
            &base.join("schematics"),
            &base.join("source/Tower.litematic"),
        )
        .expect_err("the schematic should not be replaced");
        assert!(matches!(error, SchematicError::Invalid { .. }));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_of_another_format_is_not_imported() {
        let base = directory("import-invalid");
        std::fs::write(base.join("notes.txt"), b"text").expect("write");

        let error = import_schematic(&base.join("schematics"), &base.join("notes.txt"))
            .expect_err("the file should be refused");
        assert!(matches!(error, SchematicError::Unsupported { .. }));
        assert!(!base.join("schematics/notes.txt").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_directory_is_created_for_the_schematics() {
        let base = directory("create");

        let created = create_schematic_directory(&base, "Builds").expect("the directory");
        assert!(created.is_dir());

        let error = create_schematic_directory(&base, "Builds")
            .expect_err("the directory should not be created twice");
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);

        // A name that the file system cannot hold is refused.
        for name in [
            "", ".", "..", "a/b", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b",
        ] {
            assert!(
                create_schematic_directory(&base, name).is_err(),
                "the name {name:?} should be refused"
            );
        }

        let _ = std::fs::remove_dir_all(&base);
    }
}
