//! The NBT files the launcher shows in its NBT editor.
//!
//! The editor opens the files the game writes: the compressed files, such as
//! `level.dat` and the schematics, and the region files of the worlds, which
//! hold the chunks of a 32 by 32 group of chunks. The kind of a file is told by
//! its extension, which is the way the Java version of the launcher decides
//! how to read it.

use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::util::compression;
use crate::util::nbt::Tag;

/// The side of a region file in chunks.
pub const REGION_SIDE: i32 = 32;

/// The number of the chunks a region file holds.
pub const REGION_CHUNKS: usize = (REGION_SIDE * REGION_SIDE) as usize;

/// The size of the header of a region file.
///
/// The header holds the location and the timestamp of every chunk, which are
/// four bytes each.
const REGION_HEADER_SIZE: u64 = 4096;

/// The size of a sector of a region file.
const REGION_SECTOR_SIZE: usize = 4096;

/// The kind of an NBT file the launcher can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NbtFileType {
    /// A tag compressed with gzip, such as `level.dat` or a schematic.
    ///
    /// The game writes the tag itself when the archive uses the zlib format
    /// instead, so the reader accepts both.
    Compressed,
    /// A region file that holds the chunks of a world.
    Anvil,
    /// A region file of the versions before the Anvil format.
    Region,
}

impl NbtFileType {
    /// Returns the kind of the file of the path, if the launcher can read it.
    ///
    /// The extension is compared without the case, so a file the user renamed
    /// on a case insensitive file system is still read.
    pub fn of_file(path: &Path) -> Option<Self> {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())?
            .to_ascii_lowercase();

        match extension.as_str() {
            "dat" | "dat_old" | "litematic" | "nbt" | "schematic" | "schem" => {
                Some(Self::Compressed)
            }
            "mca" => Some(Self::Anvil),
            "mcr" => Some(Self::Region),
            _ => None,
        }
    }

    /// Returns whether the launcher can read the file of the path.
    pub fn is_nbt_file(path: &Path) -> bool {
        Self::of_file(path).is_some()
    }
}

/// The content of an NBT file the launcher shows.
#[derive(Debug, Clone, PartialEq)]
pub enum NbtContent {
    /// The tag of a compressed file.
    Tag(Tag),
    /// The chunks of a region file.
    Region(NbtRegion),
}

/// A chunk of a region file.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionChunk {
    /// The position of the chunk in the region, from `0` to `31`.
    pub local_x: i32,
    /// The position of the chunk in the region, from `0` to `31`.
    pub local_z: i32,
    /// The moment the chunk was last written, in seconds since the epoch.
    ///
    /// The value is `0` when the writer of the file did not record a time.
    pub timestamp: u32,
    /// The tag of the chunk.
    pub data: Tag,
}

impl RegionChunk {
    /// Returns the name the editor shows for the chunk.
    pub fn display_name(&self) -> String {
        format!("Chunk ({}, {})", self.local_x, self.local_z)
    }
}

/// The chunks of a region file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NbtRegion {
    /// The chunks the file holds, in the order of the file.
    chunks: Vec<RegionChunk>,
}

impl NbtRegion {
    /// Reads the chunks of a region file.
    ///
    /// A location that points outside of the file, a sector count that does
    /// not cover the chunk and a compression the launcher does not support are
    /// refused, so the editor reports the file rather than showing a value the
    /// launcher invented.
    pub fn read(path: &Path) -> Result<Self, Error> {
        let mut file = File::open(path).map_err(|error| Error::io(path, error))?;
        let length = file
            .metadata()
            .map_err(|error| Error::io(path, error))?
            .len();

        if length < REGION_HEADER_SIZE {
            return Err(Error::invalid_format(
                path,
                "the region file is shorter than its header",
            ));
        }

        let mut header = vec![0u8; REGION_CHUNKS * 4];
        file.read_exact(&mut header)
            .map_err(|error| Error::io(path, error))?;

        let mut timestamps = vec![0u8; REGION_CHUNKS * 4];
        file.read_exact(&mut timestamps)
            .map_err(|error| Error::io(path, error))?;

        let mut region = Self::default();
        for index in 0..REGION_CHUNKS {
            let location = &header[index * 4..index * 4 + 4];
            let offset = u32::from_be_bytes([0, location[0], location[1], location[2]]) as u64;
            let sectors = location[3] as u64;
            if offset == 0 && sectors == 0 {
                continue;
            }

            let timestamp = u32::from_be_bytes([
                timestamps[index * 4],
                timestamps[index * 4 + 1],
                timestamps[index * 4 + 2],
                timestamps[index * 4 + 3],
            ]);

            let start = offset * REGION_SECTOR_SIZE as u64;
            if start + 5 > length {
                return Err(Error::invalid_format(
                    path,
                    format!("the chunk {index} starts outside of the region file"),
                ));
            }
            let end = start + sectors * REGION_SECTOR_SIZE as u64;
            if end > length {
                return Err(Error::invalid_format(
                    path,
                    format!("the chunk {index} ends outside of the region file"),
                ));
            }

            let data = read_chunk(&mut file, path, start)?;
            region.chunks.push(RegionChunk {
                local_x: (index as i32) % REGION_SIDE,
                local_z: (index as i32) / REGION_SIDE,
                timestamp,
                data,
            });
        }

        Ok(region)
    }

    /// Returns the chunks of the file.
    pub fn chunks(&self) -> &[RegionChunk] {
        &self.chunks
    }

    /// Returns how many chunks the file holds.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Returns whether the file holds no chunk.
    ///
    /// An empty region file is valid: the game writes the header of a region
    /// when it creates the world and stores the chunks later.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Returns the chunk at a position of the region.
    pub fn chunk(&self, local_x: i32, local_z: i32) -> Option<&RegionChunk> {
        self.chunks
            .iter()
            .find(|chunk| chunk.local_x == local_x && chunk.local_z == local_z)
    }
}

/// Reads one chunk of a region file.
fn read_chunk(file: &mut File, path: &Path, start: u64) -> Result<Tag, Error> {
    file.seek(SeekFrom::Start(start))
        .map_err(|error| Error::io(path, error))?;

    let mut length = [0u8; 4];
    file.read_exact(&mut length)
        .map_err(|error| Error::io(path, error))?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 {
        return Err(Error::invalid_format(path, "the chunk is empty"));
    }

    let mut compression = [0u8; 1];
    file.read_exact(&mut compression)
        .map_err(|error| Error::io(path, error))?;

    let mut data = vec![0u8; length.saturating_sub(1)];
    file.read_exact(&mut data)
        .map_err(|error| Error::io(path, error))?;

    let data = match compression[0] {
        1 => compression::decompress_gzip(&data)?,
        2 => compression::decompress(&data)?,
        3 => data,
        4 => {
            return Err(Error::unsupported(
                "the chunks the region file stores with LZ4",
            ))
        }
        kind => {
            return Err(Error::invalid_format(
                path,
                format!("the compression {kind} of the chunk is not known"),
            ))
        }
    };

    let mut reader = data.as_slice();
    Tag::read(&mut reader)
}

/// Reads an NBT file the launcher can show.
///
/// The kind of the file decides how it is read: a compressed file holds one
/// tag, and a region file holds the chunks of the world.
pub fn read(path: &Path) -> Result<(NbtFileType, NbtContent), Error> {
    let kind = NbtFileType::of_file(path).ok_or_else(|| {
        Error::invalid_format(
            path,
            format!(
                "the file {} is not an NBT file",
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
        )
    })?;

    let content = match kind {
        NbtFileType::Compressed => NbtContent::Tag(crate::util::nbt::read_compressed(path)?),
        NbtFileType::Anvil | NbtFileType::Region => NbtContent::Region(NbtRegion::read(path)?),
    };

    Ok((kind, content))
}

/// Writes the content of an NBT file back to its path.
///
/// The compressed files are written with gzip, which is the format the game
/// reads; the region files are written as well. The launcher writes the file
/// through a temporary file, so a failure does not leave a half written world
/// behind.
pub fn write(path: &Path, content: &NbtContent) -> Result<(), Error> {
    match content {
        NbtContent::Tag(tag) => crate::util::nbt::write_compressed(path, tag),
        NbtContent::Region(region) => write_region(path, region),
    }
}

/// Writes the chunks of a region file.
///
/// The chunks are written in the order of their position, so the file of a
/// world the launcher did not change is written the same way again.
pub fn write_region(path: &Path, region: &NbtRegion) -> Result<(), Error> {
    let mut chunks: Vec<&RegionChunk> = region.chunks.iter().collect();
    chunks.sort_by_key(|chunk| (chunk.local_z * REGION_SIDE + chunk.local_x) as usize);

    let mut header = vec![0u8; REGION_CHUNKS * 4];
    let mut timestamps = vec![0u8; REGION_CHUNKS * 4];
    let mut body: Vec<u8> = Vec::new();
    let mut next_sector = 2u32;

    for chunk in chunks {
        let index = (chunk.local_z * REGION_SIDE + chunk.local_x).max(0) as usize;
        if index >= REGION_CHUNKS {
            return Err(Error::invalid_format(
                path,
                format!(
                    "the chunk {} {} is outside of the region",
                    chunk.local_x, chunk.local_z
                ),
            ));
        }

        let mut data = Vec::new();
        chunk.data.write(&mut data)?;
        let compressed = compression::compress(&data)?;

        let length = (compressed.len() + 1) as u32;
        let sectors = (length as usize + 4).div_ceil(REGION_SECTOR_SIZE) as u32;

        // The location is three bytes of the sector and one byte of the
        // number of the sectors the chunk uses.
        header[index * 4] = (next_sector >> 16) as u8;
        header[index * 4 + 1] = (next_sector >> 8) as u8;
        header[index * 4 + 2] = next_sector as u8;
        header[index * 4 + 3] = sectors as u8;
        timestamps[index * 4..index * 4 + 4].copy_from_slice(&chunk.timestamp.to_be_bytes());

        body.extend_from_slice(&length.to_be_bytes());
        body.push(2);
        body.extend_from_slice(&compressed);
        body.resize(
            body.len() + (sectors as usize * REGION_SECTOR_SIZE) - (length as usize + 4),
            0,
        );

        next_sector += sectors;
    }

    let temporary = temporary_path(path);
    let mut contents = Vec::with_capacity(REGION_SECTOR_SIZE * 2 + body.len());
    contents.extend_from_slice(&header);
    contents.extend_from_slice(&timestamps);
    contents.extend_from_slice(&body);

    std::fs::write(&temporary, &contents).map_err(|error| Error::io(&temporary, error))?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        Error::io(path, error)
    })
}

/// Returns the file the launcher writes before it replaces the target.
fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.tmp"))
}

impl fmt::Display for NbtFileType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Compressed => "compressed NBT",
            Self::Anvil => "Anvil region",
            Self::Region => "region",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::nbt::Compound;

    /// Creates a directory the tests work in.
    ///
    /// The name carries a counter, so the tests that run in parallel do not
    /// read the files of each other.
    fn directory(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let number = COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "hmcl-rs-nbt-file-{name}-{}-{number}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    /// Returns the tag of a chunk that names its position.
    fn chunk_tag(x: i32, z: i32) -> Tag {
        let mut compound = Compound::new();
        compound.insert("xPos".to_string(), Tag::Int(x));
        compound.insert("zPos".to_string(), Tag::Int(z));
        Tag::Compound(compound)
    }

    /// Builds a region file that holds the chunks.
    fn region_file(chunks: &[(i32, i32)]) -> Vec<u8> {
        let mut region = NbtRegion::default();
        for (x, z) in chunks {
            region.chunks.push(RegionChunk {
                local_x: *x,
                local_z: *z,
                timestamp: 1234,
                data: chunk_tag(*x, *z),
            });
        }

        let directory = directory("write-region");
        let path = directory.join("r.0.0.mca");
        write_region(&path, &region).expect("the region should be written");
        let contents = std::fs::read(&path).expect("the region should be read");
        let _ = std::fs::remove_dir_all(&directory);
        contents
    }

    #[test]
    fn the_values_of_a_tag_are_described_for_the_editor() {
        let mut compound = Compound::new();
        compound.insert("name".to_string(), Tag::String("world".to_string()));
        compound.insert("seed".to_string(), Tag::Long(-12));

        let tag = Tag::Compound(compound);
        assert_eq!(tag.child_count(), Some(2));
        assert!(!tag.is_leaf());
        assert_eq!(tag.display_value(), None);

        let children = tag.children();
        assert_eq!(children[0].0, "name");
        assert_eq!(children[0].1.display_value(), Some("world".to_string()));

        // The values of a list and of an array are numbered.
        let list = Tag::List(vec![Tag::Int(4), Tag::Int(5)]);
        assert_eq!(list.child_count(), Some(2));
        assert_eq!(
            list.children()
                .iter()
                .map(|(name, value)| (name.as_str(), value.display_value()))
                .collect::<Vec<_>>(),
            [("0", Some("4".to_string())), ("1", Some("5".to_string()))]
        );

        let array = Tag::IntArray(vec![7, 8]);
        assert_eq!(
            array.children()[0].1,
            Tag::Int(7),
            "the numbers of an array are shown as numbers of four bytes"
        );

        // A value and an empty compound are leaves.
        assert!(Tag::Int(1).is_leaf());
        assert!(Tag::Compound(Compound::new()).is_leaf());
        assert_eq!(Tag::String("text".to_string()).child_count(), None);
    }

    #[test]
    fn the_kind_of_a_file_is_told_by_its_extension() {
        assert_eq!(
            NbtFileType::of_file(Path::new("level.dat")),
            Some(NbtFileType::Compressed)
        );
        assert_eq!(
            NbtFileType::of_file(Path::new("level.DAT")),
            Some(NbtFileType::Compressed)
        );
        assert_eq!(
            NbtFileType::of_file(Path::new("r.0.0.mca")),
            Some(NbtFileType::Anvil)
        );
        assert_eq!(
            NbtFileType::of_file(Path::new("r.0.0.mcr")),
            Some(NbtFileType::Region)
        );
        assert_eq!(NbtFileType::of_file(Path::new("options.txt")), None);
        assert!(NbtFileType::is_nbt_file(Path::new("test.schematic")));
        assert!(NbtFileType::is_nbt_file(Path::new("test.litematic")));
        assert!(!NbtFileType::is_nbt_file(Path::new("test.jar")));
    }

    #[test]
    fn a_compressed_file_is_read_through_the_editor() {
        let directory = directory("read-compressed");
        let path = directory.join("level.dat");

        let mut compound = Compound::new();
        compound.insert("LevelName".to_string(), Tag::String("world".to_string()));
        let tag = Tag::Compound(compound);

        crate::util::nbt::write_compressed(&path, &tag).expect("the tag should be written");
        let (kind, content) = read(&path).expect("the file should be read");
        assert_eq!(kind, NbtFileType::Compressed);
        assert_eq!(content, NbtContent::Tag(tag));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_chunks_of_a_region_file_are_read() {
        let directory = directory("read-region");
        let path = directory.join("r.0.0.mca");
        std::fs::write(&path, region_file(&[(0, 0), (5, 3), (31, 31)]))
            .expect("the region should be written");

        let (kind, content) = read(&path).expect("the file should be read");
        assert_eq!(kind, NbtFileType::Anvil);

        let NbtContent::Region(region) = content else {
            panic!("the region file should hold a region");
        };
        assert_eq!(region.len(), 3);
        assert_eq!(
            region.chunk(5, 3).map(|chunk| chunk.data.clone()),
            Some(chunk_tag(5, 3))
        );
        assert_eq!(
            region
                .chunk(31, 31)
                .map(|chunk| (chunk.local_x, chunk.local_z, chunk.timestamp)),
            Some((31, 31, 1234))
        );
        assert_eq!(
            region.chunk(0, 0).map(RegionChunk::display_name),
            Some("Chunk (0, 0)".to_string())
        );
        assert_eq!(region.chunk(1, 1), None);

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_region_file_is_written_and_read_again() {
        let directory = directory("round-trip");
        let path = directory.join("r.0.0.mca");
        std::fs::write(&path, region_file(&[(2, 1), (0, 0)]))
            .expect("the region should be written");

        let mut region = NbtRegion::read(&path).expect("the region should be read");
        let mut second = Compound::new();
        second.insert("xPos".to_string(), Tag::Int(9));
        second.insert("zPos".to_string(), Tag::Int(9));
        region.chunks.push(RegionChunk {
            local_x: 9,
            local_z: 9,
            timestamp: 7,
            data: Tag::Compound(second),
        });
        write_region(&path, &region).expect("the region should be written");

        let read = NbtRegion::read(&path).expect("the region should be read again");
        assert_eq!(read.len(), 3);
        assert_eq!(read.chunk(9, 9).map(|chunk| chunk.timestamp), Some(7));
        // The launcher removes the temporary file it writes.
        assert!(!directory.join(".r.0.0.mca.tmp").exists());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_file_that_is_not_a_region_is_refused() {
        let directory = directory("short");
        let path = directory.join("r.0.0.mca");
        std::fs::write(&path, b"short").expect("the file should be written");

        assert!(NbtRegion::read(&path).is_err());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_chunk_outside_of_the_file_is_refused() {
        let directory = directory("outside");
        let path = directory.join("r.0.0.mca");

        // A location whose sector is past the end of the file.
        let mut contents = vec![0u8; REGION_SECTOR_SIZE * 2];
        contents[0] = 0;
        contents[1] = 0;
        contents[2] = 0xff;
        contents[3] = 1;
        std::fs::write(&path, &contents).expect("the file should be written");

        assert!(NbtRegion::read(&path).is_err());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_file_that_the_launcher_cannot_show_is_refused() {
        let directory = directory("unknown");
        let path = directory.join("options.txt");
        std::fs::write(&path, b"fov: 0").expect("the file should be written");

        assert!(read(&path).is_err());

        let _ = std::fs::remove_dir_all(&directory);
    }
}
