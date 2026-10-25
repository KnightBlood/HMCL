//! The NBT format of Minecraft.
//!
//! The launcher reads the NBT of the worlds, of the schematics and of the
//! crash reports. The format is the one of Minecraft: the values are written
//! with the big endian order and the texts use the modified UTF-8 of Java.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::util::compression;

/// The kind of a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagType {
    /// The end of a compound.
    End,
    /// A number of a byte.
    Byte,
    /// A number of two bytes.
    Short,
    /// A number of four bytes.
    Int,
    /// A number of eight bytes.
    Long,
    /// A number of four bytes with a decimal point.
    Float,
    /// A number of eight bytes with a decimal point.
    Double,
    /// A list of numbers of a byte.
    ByteArray,
    /// A text.
    String,
    /// A list of the values of a kind.
    List,
    /// A list of the values that are named.
    Compound,
    /// A list of numbers of four bytes.
    IntArray,
    /// A list of numbers of eight bytes.
    LongArray,
}

impl TagType {
    /// Returns the number the kind of the tag is written with.
    pub const fn id(self) -> u8 {
        match self {
            Self::End => 0,
            Self::Byte => 1,
            Self::Short => 2,
            Self::Int => 3,
            Self::Long => 4,
            Self::Float => 5,
            Self::Double => 6,
            Self::ByteArray => 7,
            Self::String => 8,
            Self::List => 9,
            Self::Compound => 10,
            Self::IntArray => 11,
            Self::LongArray => 12,
        }
    }

    /// Returns the kind of the tag of the number.
    pub const fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::End),
            1 => Some(Self::Byte),
            2 => Some(Self::Short),
            3 => Some(Self::Int),
            4 => Some(Self::Long),
            5 => Some(Self::Float),
            6 => Some(Self::Double),
            7 => Some(Self::ByteArray),
            8 => Some(Self::String),
            9 => Some(Self::List),
            10 => Some(Self::Compound),
            11 => Some(Self::IntArray),
            12 => Some(Self::LongArray),
            _ => None,
        }
    }
}

/// The values of a tag that are named.
pub type Compound = BTreeMap<String, Tag>;

/// A value of the NBT format.
#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    /// A number of a byte.
    Byte(i8),
    /// A number of two bytes.
    Short(i16),
    /// A number of four bytes.
    Int(i32),
    /// A number of eight bytes.
    Long(i64),
    /// A number of four bytes with a decimal point.
    Float(f32),
    /// A number of eight bytes with a decimal point.
    Double(f64),
    /// A list of numbers of a byte.
    ByteArray(Vec<i8>),
    /// A text.
    String(String),
    /// A list of the values of a kind.
    List(Vec<Tag>),
    /// A list of the values that are named.
    Compound(Compound),
    /// A list of numbers of four bytes.
    IntArray(Vec<i32>),
    /// A list of numbers of eight bytes.
    LongArray(Vec<i64>),
}

impl Tag {
    /// Returns the kind of the tag.
    pub fn tag_type(&self) -> TagType {
        match self {
            Self::Byte(_) => TagType::Byte,
            Self::Short(_) => TagType::Short,
            Self::Int(_) => TagType::Int,
            Self::Long(_) => TagType::Long,
            Self::Float(_) => TagType::Float,
            Self::Double(_) => TagType::Double,
            Self::ByteArray(_) => TagType::ByteArray,
            Self::String(_) => TagType::String,
            Self::List(_) => TagType::List,
            Self::Compound(_) => TagType::Compound,
            Self::IntArray(_) => TagType::IntArray,
            Self::LongArray(_) => TagType::LongArray,
        }
    }

    /// Returns the value of the tag as a number, if it is one.
    pub fn as_long(&self) -> Option<i64> {
        match self {
            Self::Byte(value) => Some(*value as i64),
            Self::Short(value) => Some(*value as i64),
            Self::Int(value) => Some(*value as i64),
            Self::Long(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the value of the tag as a whole number of four bytes, if it is
    /// one.
    ///
    /// The game writes some of its values as a byte or a short, so the caller
    /// reads the number with [`Tag::as_long`] when it accepts every size.
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the value of the tag as a number with a decimal point, if it is
    /// one of the two sizes the game writes.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value as f64),
            Self::Double(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the value of the tag as a text, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value of the tag as a list of the values that are named,
    /// if it is one.
    pub fn as_compound(&self) -> Option<&Compound> {
        match self {
            Self::Compound(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value of the tag as a mutable list of the values that are
    /// named, if it is one.
    pub fn as_compound_mut(&mut self) -> Option<&mut Compound> {
        match self {
            Self::Compound(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the value of the tag with the name, if the tag is a list of
    /// the values that are named and holds it.
    pub fn get(&self, name: &str) -> Option<&Tag> {
        self.as_compound().and_then(|compound| compound.get(name))
    }

    /// Returns the text the launcher shows for the value of the tag.
    ///
    /// A value that the launcher shows is a number or a text; the tags that
    /// hold other values show how many values they hold instead, so the editor
    /// writes `3 entries` for a list. The method returns `None` for a tag that
    /// holds other values.
    pub fn display_value(&self) -> Option<String> {
        match self {
            Self::Byte(value) => Some(value.to_string()),
            Self::Short(value) => Some(value.to_string()),
            Self::Int(value) => Some(value.to_string()),
            Self::Long(value) => Some(value.to_string()),
            Self::Float(value) => Some(value.to_string()),
            Self::Double(value) => Some(value.to_string()),
            Self::String(value) => Some(value.clone()),
            Self::ByteArray(_)
            | Self::List(_)
            | Self::Compound(_)
            | Self::IntArray(_)
            | Self::LongArray(_) => None,
        }
    }

    /// Returns how many values the tag holds, if it holds other values.
    ///
    /// The arrays and the lists count every element, and the compound counts
    /// its named values. The method returns `None` for a number or a text,
    /// which hold no other value.
    pub fn child_count(&self) -> Option<usize> {
        match self {
            Self::ByteArray(values) => Some(values.len()),
            Self::List(values) => Some(values.len()),
            Self::Compound(values) => Some(values.len()),
            Self::IntArray(values) => Some(values.len()),
            Self::LongArray(values) => Some(values.len()),
            Self::Byte(_)
            | Self::Short(_)
            | Self::Int(_)
            | Self::Long(_)
            | Self::Float(_)
            | Self::Double(_)
            | Self::String(_) => None,
        }
    }

    /// Returns whether the tag holds no other value.
    ///
    /// The values the launcher shows as a single line, such as a number or a
    /// text, are leaves; an empty compound or list is a leaf as well, because
    /// the editor has nothing to expand.
    pub fn is_leaf(&self) -> bool {
        self.child_count().is_none_or(|count| count == 0)
    }

    /// Returns the values the tag holds, with the name the editor shows for
    /// each of them.
    ///
    /// The values of a compound are named with their keys and the values of a
    /// list with their position, which is the way the editor numbers the
    /// elements of a list. The numbers of an array are numbered as well,
    /// because the format gives them no name. A tag that holds no other value
    /// has no children.
    ///
    /// The elements of an array are copied into tags, because the format
    /// stores them as plain numbers; the editor asks for them only when the
    /// user expands the array.
    pub fn children(&self) -> Vec<(String, Tag)> {
        match self {
            Self::Compound(values) => values
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            Self::List(values) => values
                .iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), value.clone()))
                .collect(),
            Self::ByteArray(values) => values
                .iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), Self::Byte(*value)))
                .collect(),
            Self::IntArray(values) => values
                .iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), Self::Int(*value)))
                .collect(),
            Self::LongArray(values) => values
                .iter()
                .enumerate()
                .map(|(index, value)| (index.to_string(), Self::Long(*value)))
                .collect(),
            Self::Byte(_)
            | Self::Short(_)
            | Self::Int(_)
            | Self::Long(_)
            | Self::Float(_)
            | Self::Double(_)
            | Self::String(_) => Vec::new(),
        }
    }

    /// Returns the value of the tag with the name that can be changed, if the
    /// tag is a list of the values that are named and holds it.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Tag> {
        self.as_compound_mut()
            .and_then(|compound| compound.get_mut(name))
    }

    /// Reads a tag of the reader.
    ///
    /// The format of Minecraft writes the kind of the tag, the name of the tag
    /// and the value. The root tag of a file has an empty name, which the game
    /// still writes, so the reader consumes it; the launcher does not show the
    /// name of the root of a file, because the editor shows the name of the
    /// file instead.
    pub fn read(reader: &mut impl Read) -> Result<Self, Error> {
        let tag_type = read_u8(reader)?;
        let tag_type = TagType::from_id(tag_type).ok_or_else(|| {
            Error::invalid_format("", format!("the tag of the kind {tag_type} is not known"))
        })?;
        if tag_type == TagType::End {
            return Err(Error::invalid_format(
                "",
                "the tag of the kind End has no value",
            ));
        }

        read_string(reader)?;
        Self::read_payload(tag_type, reader)
    }

    /// Reads the value of a tag of the given kind.
    fn read_payload(tag_type: TagType, reader: &mut impl Read) -> Result<Self, Error> {
        Ok(match tag_type {
            TagType::End => {
                return Err(Error::invalid_format(
                    "",
                    "the tag of the kind End has no value",
                ))
            }
            TagType::Byte => Self::Byte(read_i8(reader)?),
            TagType::Short => Self::Short(read_i16(reader)?),
            TagType::Int => Self::Int(read_i32(reader)?),
            TagType::Long => Self::Long(read_i64(reader)?),
            TagType::Float => Self::Float(f32::from_bits(read_i32(reader)? as u32)),
            TagType::Double => Self::Double(f64::from_bits(read_i64(reader)? as u64)),
            TagType::ByteArray => {
                let length = read_i32(reader)?;
                let mut buffer = vec![0u8; length.max(0) as usize];
                reader
                    .read_exact(&mut buffer)
                    .map_err(Error::io_without_path)?;
                Self::ByteArray(buffer.into_iter().map(|byte| byte as i8).collect())
            }
            TagType::String => Self::String(read_string(reader)?),
            TagType::List => {
                let element_type = TagType::from_id(read_u8(reader)?).ok_or_else(|| {
                    Error::invalid_format("", "the kind of the values of the list is not known")
                })?;
                let length = read_i32(reader)?;
                let mut values = Vec::with_capacity(length.max(0) as usize);
                for _ in 0..length.max(0) {
                    values.push(Self::read_payload(element_type, reader)?);
                }
                Self::List(values)
            }
            TagType::Compound => {
                let mut compound = Compound::new();
                loop {
                    let tag_type = TagType::from_id(read_u8(reader)?).ok_or_else(|| {
                        Error::invalid_format("", "the kind of the tag is not known")
                    })?;
                    if tag_type == TagType::End {
                        break;
                    }

                    let name = read_string(reader)?;
                    compound.insert(name, Self::read_payload(tag_type, reader)?);
                }
                Self::Compound(compound)
            }
            TagType::IntArray => {
                let length = read_i32(reader)?;
                let mut values = vec![0i32; length.max(0) as usize];
                for value in &mut values {
                    *value = read_i32(reader)?;
                }
                Self::IntArray(values)
            }
            TagType::LongArray => {
                let length = read_i32(reader)?;
                let mut values = vec![0i64; length.max(0) as usize];
                for value in &mut values {
                    *value = read_i64(reader)?;
                }
                Self::LongArray(values)
            }
        })
    }

    /// Writes the tag to the writer.
    ///
    /// The name of the root tag is empty, which is the name the game writes for
    /// the root of the files it stores.
    pub fn write(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer
            .write_all(&[self.tag_type().id()])
            .map_err(Error::io_without_path)?;
        write_string(writer, "")?;
        self.write_payload(writer)
    }

    /// Writes the value of the tag.
    fn write_payload(&self, writer: &mut impl Write) -> Result<(), Error> {
        match self {
            Self::Byte(value) => writer
                .write_all(&value.to_be_bytes())
                .map_err(Error::io_without_path),
            Self::Short(value) => writer
                .write_all(&value.to_be_bytes())
                .map_err(Error::io_without_path),
            Self::Int(value) => writer
                .write_all(&value.to_be_bytes())
                .map_err(Error::io_without_path),
            Self::Long(value) => writer
                .write_all(&value.to_be_bytes())
                .map_err(Error::io_without_path),
            Self::Float(value) => writer
                .write_all(&value.to_bits().to_be_bytes())
                .map_err(Error::io_without_path),
            Self::Double(value) => writer
                .write_all(&value.to_bits().to_be_bytes())
                .map_err(Error::io_without_path),
            Self::ByteArray(values) => {
                writer
                    .write_all(&(values.len() as i32).to_be_bytes())
                    .map_err(Error::io_without_path)?;
                for value in values {
                    writer
                        .write_all(&value.to_be_bytes())
                        .map_err(Error::io_without_path)?;
                }
                Ok(())
            }
            Self::String(value) => write_string(writer, value),
            Self::List(values) => {
                let element_type = values.first().map(Tag::tag_type).unwrap_or(TagType::End);
                writer
                    .write_all(&[element_type.id()])
                    .map_err(Error::io_without_path)?;
                writer
                    .write_all(&(values.len() as i32).to_be_bytes())
                    .map_err(Error::io_without_path)?;
                for value in values {
                    value.write_payload(writer)?;
                }
                Ok(())
            }
            Self::Compound(compound) => {
                for (name, value) in compound {
                    writer
                        .write_all(&[value.tag_type().id()])
                        .map_err(Error::io_without_path)?;
                    write_string(writer, name)?;
                    value.write_payload(writer)?;
                }
                writer
                    .write_all(&[TagType::End.id()])
                    .map_err(Error::io_without_path)
            }
            Self::IntArray(values) => {
                writer
                    .write_all(&(values.len() as i32).to_be_bytes())
                    .map_err(Error::io_without_path)?;
                for value in values {
                    writer
                        .write_all(&value.to_be_bytes())
                        .map_err(Error::io_without_path)?;
                }
                Ok(())
            }
            Self::LongArray(values) => {
                writer
                    .write_all(&(values.len() as i32).to_be_bytes())
                    .map_err(Error::io_without_path)?;
                for value in values {
                    writer
                        .write_all(&value.to_be_bytes())
                        .map_err(Error::io_without_path)?;
                }
                Ok(())
            }
        }
    }
}

/// Reads the file of a tag.
///
/// The game compresses most of the files it stores with gzip and some of them,
/// such as the files a user edited by hand, with zlib or not at all. The header
/// of the file tells the compression, so the launcher reads which one it is.
pub fn read_compressed(path: &Path) -> Result<Tag, Error> {
    let file = File::open(path).map_err(|error| Error::io(path, error))?;
    let mut reader = BufReader::new(file);

    let mut data = Vec::new();
    reader
        .read_to_end(&mut data)
        .map_err(|error| Error::io(path, error))?;

    let data = match data.as_slice() {
        [0x1f, 0x8b, ..] => compression::decompress_gzip(&data)?,
        [0x78, ..] => compression::decompress(&data)?,
        _ => data,
    };
    let mut reader = data.as_slice();

    Tag::read(&mut reader)
}

/// Writes a tag to a file, compressed with gzip.
///
/// The file is written next to the target and moved into place, so a failure
/// while writing does not leave a half-written file behind.
pub fn write_compressed(path: &Path, tag: &Tag) -> Result<(), Error> {
    let mut data = Vec::new();
    tag.write(&mut data)?;

    let compressed = compression::compress_gzip(&data)?;

    let temporary = temporary_path(path);
    std::fs::write(&temporary, &compressed).map_err(|error| Error::io(&temporary, error))?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        Error::io(path, error)
    })
}

/// Returns the file the launcher writes before it replaces the target.
///
/// The name is the name of the target with `.tmp` appended, so two files whose
/// names differ only in their extension, such as `level.dat` and
/// `level.dat_old`, do not share a temporary file.
fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.tmp"))
}

/// Reads a number of a byte.
fn read_u8(reader: &mut impl Read) -> Result<u8, Error> {
    let mut buffer = [0u8; 1];
    reader
        .read_exact(&mut buffer)
        .map_err(Error::io_without_path)?;
    Ok(buffer[0])
}

/// Reads a number of a byte.
fn read_i8(reader: &mut impl Read) -> Result<i8, Error> {
    Ok(read_u8(reader)? as i8)
}

/// Reads a number of two bytes.
fn read_i16(reader: &mut impl Read) -> Result<i16, Error> {
    let mut buffer = [0u8; 2];
    reader
        .read_exact(&mut buffer)
        .map_err(Error::io_without_path)?;
    Ok(i16::from_be_bytes(buffer))
}

/// Reads a number of four bytes.
fn read_i32(reader: &mut impl Read) -> Result<i32, Error> {
    let mut buffer = [0u8; 4];
    reader
        .read_exact(&mut buffer)
        .map_err(Error::io_without_path)?;
    Ok(i32::from_be_bytes(buffer))
}

/// Reads a number of eight bytes.
fn read_i64(reader: &mut impl Read) -> Result<i64, Error> {
    let mut buffer = [0u8; 8];
    reader
        .read_exact(&mut buffer)
        .map_err(Error::io_without_path)?;
    Ok(i64::from_be_bytes(buffer))
}

/// Reads a text.
///
/// The length of the text is the number of its bytes, not the number of its
/// characters, so the text is read as the modified UTF-8 of Java.
fn read_string(reader: &mut impl Read) -> Result<String, Error> {
    let length = read_u16(reader)? as usize;

    let mut bytes = vec![0u8; length];
    reader
        .read_exact(&mut bytes)
        .map_err(Error::io_without_path)?;

    Ok(decode_modified_utf8(&bytes))
}

/// Reads a number of two bytes.
fn read_u16(reader: &mut impl Read) -> Result<u16, Error> {
    let mut buffer = [0u8; 2];
    reader
        .read_exact(&mut buffer)
        .map_err(Error::io_without_path)?;
    Ok(u16::from_be_bytes(buffer))
}

/// Writes a text.
///
/// The length of the text is the number of the characters of the text, not
/// the number of its bytes, so the text is written as the modified UTF-8 of
/// Java.
fn write_string(writer: &mut impl Write, value: &str) -> Result<(), Error> {
    let (encoded, length) = encode_modified_utf8(value);
    writer
        .write_all(&length.to_be_bytes())
        .map_err(Error::io_without_path)?;
    writer.write_all(&encoded).map_err(Error::io_without_path)
}

/// Reads the text of the modified UTF-8 of Java.
///
/// The characters outside the basic plane are written as the two characters
/// of a pair, so they are joined again here.
fn decode_modified_utf8(bytes: &[u8]) -> String {
    let mut units = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        let value = if byte < 0x80 {
            index += 1;
            byte as u32
        } else if byte & 0xe0 == 0xc0 && index + 1 < bytes.len() {
            let value = ((byte as u32 & 0x1f) << 6) | (bytes[index + 1] as u32 & 0x3f);
            index += 2;
            value
        } else if byte & 0xf0 == 0xe0 && index + 2 < bytes.len() {
            let value = ((byte as u32 & 0x0f) << 12)
                | ((bytes[index + 1] as u32 & 0x3f) << 6)
                | (bytes[index + 2] as u32 & 0x3f);
            index += 3;
            value
        } else {
            index += 1;
            0xfffd
        };

        units.push(value);
    }

    let mut result = String::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        let value = units[index];

        if (0xd800..0xdc00).contains(&value) && index + 1 < units.len() {
            let low = units[index + 1];
            if (0xdc00..0xe000).contains(&low) {
                let code = 0x10000 + ((value - 0xd800) << 10) + (low - 0xdc00);
                result.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                index += 2;
                continue;
            }
        }

        result.push(char::from_u32(value).unwrap_or('\u{fffd}'));
        index += 1;
    }

    result
}

/// Writes the text as the modified UTF-8 of Java.
///
/// The method returns the bytes of the text and the number of the bytes, which
/// is the length the format of Minecraft writes before the text.
fn encode_modified_utf8(value: &str) -> (Vec<u8>, u16) {
    let mut result = Vec::with_capacity(value.len());

    for character in value.chars() {
        let code = character as u32;
        if code == 0 {
            // Java writes the null character with two bytes, so the zero byte
            // can never appear inside a text.
            result.push(0xc0);
            result.push(0x80);
        } else if code < 0x80 {
            result.push(code as u8);
        } else if code < 0x800 {
            result.push(0xc0 | (code >> 6) as u8);
            result.push(0x80 | (code & 0x3f) as u8);
        } else if code < 0x10000 {
            result.push(0xe0 | (code >> 12) as u8);
            result.push(0x80 | ((code >> 6) & 0x3f) as u8);
            result.push(0x80 | (code & 0x3f) as u8);
        } else {
            // The characters outside the basic plane are written as the two
            // characters of a pair, like Java does.
            let adjusted = code - 0x10000;
            let high = 0xd800 + (adjusted >> 10);
            let low = 0xdc00 + (adjusted & 0x3ff);

            for surrogate in [high, low] {
                result.push(0xe0 | (surrogate >> 12) as u8);
                result.push(0x80 | ((surrogate >> 6) & 0x3f) as u8);
                result.push(0x80 | (surrogate & 0x3f) as u8);
            }
        }
    }

    let length = result.len() as u16;
    (result, length)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the bytes of a tag.
    fn write(tag: &Tag) -> Vec<u8> {
        let mut buffer = Vec::new();
        tag.write(&mut buffer).expect("the tag should be written");
        buffer
    }

    /// Reads a tag of the bytes.
    fn read(bytes: &[u8]) -> Tag {
        let mut reader = bytes;
        Tag::read(&mut reader).expect("the tag should be read")
    }

    /// Reads a tag of the bytes, keeping the failure.
    fn try_read(bytes: &[u8]) -> Result<Tag, Error> {
        let mut reader = bytes;
        Tag::read(&mut reader)
    }

    #[test]
    fn the_numbers_are_read_and_written_again() {
        for tag in [
            Tag::Byte(-5),
            Tag::Short(-300),
            Tag::Int(-70000),
            Tag::Long(-5_000_000_000),
            Tag::Float(1.5),
            Tag::Double(-2.25),
        ] {
            assert_eq!(read(&write(&tag)), tag);
        }
    }

    #[test]
    fn the_lists_are_read_and_written_again() {
        for tag in [
            Tag::ByteArray(vec![1, -2, 3]),
            Tag::IntArray(vec![1, -2, 3]),
            Tag::LongArray(vec![1, -2, 3]),
            Tag::List(vec![Tag::Int(1), Tag::Int(2)]),
            Tag::List(Vec::new()),
        ] {
            assert_eq!(read(&write(&tag)), tag);
        }
    }

    #[test]
    fn the_texts_are_read_and_written_again() {
        for tag in [
            Tag::String(String::new()),
            Tag::String("Hello".to_string()),
            Tag::String("你好".to_string()),
            Tag::String("🎮".to_string()),
        ] {
            assert_eq!(read(&write(&tag)), tag);
        }
    }

    #[test]
    fn the_length_of_a_text_counts_its_bytes() {
        // The format writes the number of the bytes of the text, so a text of
        // three characters that are written with three bytes each is prefixed
        // with nine. Files of the game name their regions in Chinese, so the
        // difference matters.
        let tag = Tag::Compound({
            let mut compound = Compound::new();
            compound.insert("名".to_string(), Tag::String("你好".to_string()));
            compound
        });

        let bytes = write(&tag);
        // The kind of the compound, the empty name of the root, the kind of the
        // text, the length of the name and the name itself.
        assert_eq!(&bytes[..7], &[10, 0, 0, 8, 0, 3, 0xe5]);
        // The length of the value is the number of its bytes, not of its
        // characters.
        let value_length = u16::from_be_bytes([bytes[9], bytes[10]]);
        assert_eq!(value_length, 6);
        assert_eq!(bytes.len(), 11 + 6 + 1);

        assert_eq!(read(&bytes), tag);
    }

    #[test]
    fn the_values_that_are_named_are_read_and_written_again() {
        let mut compound = Compound::new();
        compound.insert("name".to_string(), Tag::String("world".to_string()));
        compound.insert("time".to_string(), Tag::Long(1234));
        compound.insert(
            "position".to_string(),
            Tag::List(vec![Tag::Double(1.0), Tag::Double(2.0), Tag::Double(3.0)]),
        );

        let tag = Tag::Compound(compound);
        let read = read(&write(&tag));

        assert_eq!(read, tag);
        assert_eq!(read.get("name").and_then(Tag::as_str), Some("world"));
        assert_eq!(read.get("time").and_then(Tag::as_long), Some(1234));
    }

    #[test]
    fn the_kinds_of_the_tags_are_told_apart() {
        assert_eq!(TagType::from_id(10), Some(TagType::Compound));
        assert_eq!(TagType::from_id(0), Some(TagType::End));
        assert_eq!(TagType::from_id(13), None);
        assert_eq!(Tag::Int(1).tag_type().id(), 3);
    }

    #[test]
    fn the_name_of_the_root_of_a_file_is_read_and_written() {
        let tag = Tag::Compound(Compound::new());
        let bytes = write(&tag);

        // The kind of the tag, the empty name of the root and the end of the
        // compound, which is the way the game writes the root of a file.
        assert_eq!(bytes, vec![10, 0, 0, 0]);
        assert_eq!(read(&bytes), tag);

        // A file whose root is named is read as well: the launcher reads the
        // files the game writes, which name the root of some of them.
        let mut named = vec![10, 0, 5];
        named.extend_from_slice(b"Level");
        named.push(8);
        named.extend_from_slice(&1u16.to_be_bytes());
        named.extend_from_slice(b"a");
        named.extend_from_slice(&1u16.to_be_bytes());
        named.extend_from_slice(b"x");
        named.push(0);

        let read = read(&named);
        assert_eq!(read.get("a").and_then(Tag::as_str), Some("x"));

        // A file whose first kind is the end of a compound holds no tag.
        assert!(try_read(&[0, 0, 0]).is_err());
    }

    #[test]
    fn a_file_with_the_zlib_format_is_read() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-nbt-zlib-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        let path = directory.join("level.dat");

        let tag = Tag::Compound(Compound::new());
        let mut data = Vec::new();
        tag.write(&mut data).expect("the tag should be written");
        let compressed = compression::compress(&data).expect("the data should be compressed");
        std::fs::write(&path, &compressed).expect("the file should be written");

        assert_eq!(
            read_compressed(&path).expect("the file should be read"),
            tag
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_data_that_is_not_a_tag_is_refused() {
        assert!(try_read(&[200]).is_err());
        assert!(try_read(&[]).is_err());
    }

    #[test]
    fn a_compressed_file_is_read_and_written_again() {
        let directory = std::env::temp_dir().join(format!("hmcl-rs-nbt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        let path = directory.join("level.dat");

        let mut compound = Compound::new();
        compound.insert("name".to_string(), Tag::String("world".to_string()));
        compound.insert("seed".to_string(), Tag::Long(-1234));
        let tag = Tag::Compound(compound);

        write_compressed(&path, &tag).expect("the tag should be written");
        let read = read_compressed(&path).expect("the tag should be read");
        assert_eq!(read, tag);

        let _ = std::fs::remove_dir_all(&directory);
    }
}
