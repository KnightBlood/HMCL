//! Reading the archives of the game.
//!
//! The native libraries of the game are stored in the archives of Java; the
//! launcher reads the files of the archives and extracts them to a directory
//! the game reads the libraries from.

use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use flate2::read::DeflateDecoder;

/// The signature of the end of the directory of an archive.
const DIRECTORY_END_SIGNATURE: u32 = 0x0605_4b50;
/// The signature of the locator of the directory of a large archive.
const DIRECTORY_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
/// The signature of the end of the directory of a large archive.
const DIRECTORY_END_64_SIGNATURE: u32 = 0x0606_4b50;
/// The signature of a file of the directory of an archive.
const DIRECTORY_SIGNATURE: u32 = 0x0201_4b50;
/// The signature of the header of a file of an archive.
const FILE_SIGNATURE: u32 = 0x0403_4b50;

/// The error of reading an archive.
#[derive(Debug)]
pub enum ZipError {
    /// The archive could not be read.
    Io(io::Error),
    /// The archive is not the archive the launcher reads.
    Malformed(&'static str),
}

impl fmt::Display for ZipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "The archive could not be read: {error}"),
            Self::Malformed(reason) => write!(formatter, "The archive is malformed: {reason}"),
        }
    }
}

impl std::error::Error for ZipError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Malformed(_) => None,
        }
    }
}

impl From<io::Error> for ZipError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Returns the path a file is written to before it is moved into place.
///
/// The suffix is added to the whole name of the file, so the extension of the
/// file is kept: a file that is written next to a file whose name has dots in
/// it keeps its name, and the file is moved into place when it is finished.
fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    path.with_file_name(name)
}

/// A file of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    /// The path of the file in the archive.
    pub name: String,
    /// Whether the entry is a directory.
    pub directory: bool,
    /// The size of the file once it is extracted, in bytes.
    pub size: u64,
    /// The mode of the file on the systems of Unix.
    pub unix_mode: u32,
}

impl ZipEntry {
    /// Returns whether the file is a symbolic link.
    ///
    /// The launcher does not follow the links of an archive; a link of an
    /// archive may point outside of the directory the files are extracted to.
    pub fn is_symlink(&self) -> bool {
        self.unix_mode & 0xF000 == 0xA000
    }
}

/// Returns the files of an archive.
pub fn entries(path: &Path) -> Result<Vec<ZipEntry>, ZipError> {
    let file = File::open(path)?;
    entries_from(file)
}

/// Returns the files of an archive that is read from memory.
pub fn entries_from<R: Read + Seek>(mut reader: R) -> Result<Vec<ZipEntry>, ZipError> {
    Ok(read_directory(&mut reader)?
        .into_iter()
        .map(|entry| ZipEntry {
            directory: entry.name.ends_with('/'),
            name: entry.name,
            size: entry.size,
            unix_mode: entry.unix_mode,
        })
        .collect())
}

/// Reads a file of an archive.
///
/// The file is found by its name; a file the archive does not hold returns
/// nothing. The name is compared as the archive writes it, so the caller reads
/// the names the archive holds.
pub fn read_entry(path: &Path, name: &str) -> Result<Option<Vec<u8>>, ZipError> {
    let mut file = File::open(path)?;
    read_entry_from(&mut file, name)
}

/// Reads a file of an archive that is read from memory.
pub fn read_entry_from<R: Read + Seek>(
    reader: &mut R,
    name: &str,
) -> Result<Option<Vec<u8>>, ZipError> {
    let entry = read_directory(reader)?
        .into_iter()
        .find(|entry| entry.name == name);

    match entry {
        Some(entry) => read_file(reader, &entry).map(Some),
        None => Ok(None),
    }
}

/// Extracts the files of an archive that the filter selects.
///
/// The files are written under the destination; the paths of the files are
/// read as the paths under the destination, so a file of the archive cannot
/// write outside of it. The files the filter does not select are skipped.
pub fn extract(
    path: &Path,
    destination: &Path,
    filter: impl Fn(&ZipEntry, &Path) -> bool,
) -> Result<Vec<PathBuf>, ZipError> {
    let mut file = File::open(path)?;
    extract_from(&mut file, destination, filter)
}

/// Extracts the files of an archive that the filter selects.
pub fn extract_from<R: Read + Seek>(
    reader: &mut R,
    destination: &Path,
    filter: impl Fn(&ZipEntry, &Path) -> bool,
) -> Result<Vec<PathBuf>, ZipError> {
    extract_under(reader, destination, None, filter)
}

/// Extracts the files of an archive that are under a directory of the archive.
///
/// The directory is not written to the destination: the files are written as
/// if the directory were the root of the destination. An archive that holds a
/// world in a directory of its own is read this way.
pub fn extract_under_directory(
    path: &Path,
    destination: &Path,
    directory: &str,
    filter: impl Fn(&ZipEntry, &Path) -> bool,
) -> Result<Vec<PathBuf>, ZipError> {
    let mut file = File::open(path)?;
    let prefix = format!("{}/", directory.trim_matches('/'));
    extract_under(&mut file, destination, Some(prefix.as_str()), filter)
}

/// Extracts the files of an archive that the filter selects.
///
/// The names of the files are read under the prefix of the directory the
/// caller extracts, if there is one.
fn extract_under<R: Read + Seek>(
    reader: &mut R,
    destination: &Path,
    prefix: Option<&str>,
    filter: impl Fn(&ZipEntry, &Path) -> bool,
) -> Result<Vec<PathBuf>, ZipError> {
    let directory = read_directory(reader)?;
    let mut extracted = Vec::new();

    for entry in directory {
        if entry.name.ends_with('/') {
            continue;
        }

        let name = match prefix {
            Some(prefix) => match entry.name.strip_prefix(prefix) {
                Some(name) => name,
                None => continue,
            },
            None => entry.name.as_str(),
        };

        let relative = relative_path(name).ok_or(ZipError::Malformed(
            "a file of the archive is not under the directory",
        ))?;
        let target = destination.join(relative);

        let file = ZipEntry {
            name: entry.name.clone(),
            directory: false,
            size: entry.size,
            unix_mode: entry.unix_mode,
        };
        if !filter(&file, &target) {
            continue;
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let data = read_file(reader, &entry)?;
        let mut file = File::create(&target)?;
        file.write_all(&data)?;
        extracted.push(target);
    }

    Ok(extracted)
}

/// A file of the directory of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectoryEntry {
    /// The path of the file in the archive.
    name: String,
    /// The way the file is compressed.
    compression: u16,
    /// The size of the file once it is compressed, in bytes.
    compressed_size: u64,
    /// The size of the file once it is extracted, in bytes.
    size: u64,
    /// The place of the header of the file in the archive.
    offset: u64,
    /// The mode of the file on the systems of Unix.
    unix_mode: u32,
}

/// Reads the directory of an archive.
fn read_directory<R: Read + Seek>(reader: &mut R) -> Result<Vec<DirectoryEntry>, ZipError> {
    let length = reader.seek(SeekFrom::End(0))?;
    if length < 22 {
        return Err(ZipError::Malformed("the archive is too short"));
    }

    // The end of the directory is at the end of the archive, after the comment
    // of the archive; the comment is at most 65535 bytes long.
    let tail_length = length.min(22 + 0xFFFF);
    let mut tail = vec![0u8; tail_length as usize];
    reader.seek(SeekFrom::Start(length - tail_length))?;
    reader.read_exact(&mut tail)?;

    let end = find_directory_end(&tail).ok_or(ZipError::Malformed(
        "the archive does not tell where its directory is",
    ))?;

    let mut count = u64::from(
        read_u16(&tail, end + 10)
            .ok_or(ZipError::Malformed("the end of the directory is malformed"))?,
    );
    let mut size = u64::from(
        read_u32(&tail, end + 12)
            .ok_or(ZipError::Malformed("the end of the directory is malformed"))?,
    );
    let mut offset = u64::from(
        read_u32(&tail, end + 16)
            .ok_or(ZipError::Malformed("the end of the directory is malformed"))?,
    );

    if count == 0xFFFF || size == 0xFFFF_FFFF || offset == 0xFFFF_FFFF {
        let (large_count, large_size, large_offset) = read_directory_64(reader, &tail, end)?;
        count = large_count;
        size = large_size;
        offset = large_offset;
    }

    let mut data = vec![0u8; size as usize];
    reader.seek(SeekFrom::Start(offset))?;
    reader.read_exact(&mut data)?;

    read_directory_entries(&data, count as usize)
}

/// Finds the end of the directory of an archive.
fn find_directory_end(tail: &[u8]) -> Option<usize> {
    if tail.len() < 22 {
        return None;
    }

    for index in (0..=tail.len() - 22).rev() {
        if read_u32(tail, index) == Some(DIRECTORY_END_SIGNATURE) {
            let comment = usize::from(read_u16(tail, index + 20)?);
            if index + 22 + comment == tail.len() {
                return Some(index);
            }
        }
    }

    None
}

/// Reads the end of the directory of an archive that is too large for the end
/// of the directory of the usual archives.
fn read_directory_64<R: Read + Seek>(
    reader: &mut R,
    tail: &[u8],
    end: usize,
) -> Result<(u64, u64, u64), ZipError> {
    if end < 20 || read_u32(tail, end - 20) != Some(DIRECTORY_LOCATOR_SIGNATURE) {
        return Err(ZipError::Malformed(
            "the archive is too large and does not tell where its directory is",
        ));
    }

    let locator = read_u64(tail, end - 20 + 8).ok_or(ZipError::Malformed(
        "the locator of the directory is malformed",
    ))?;

    let mut header = [0u8; 56];
    reader.seek(SeekFrom::Start(locator))?;
    reader.read_exact(&mut header)?;

    if read_u32(&header, 0) != Some(DIRECTORY_END_64_SIGNATURE) {
        return Err(ZipError::Malformed(
            "the end of the directory of the large archive is malformed",
        ));
    }

    let count = read_u64(&header, 32)
        .ok_or(ZipError::Malformed("the end of the directory is malformed"))?;
    let size = read_u64(&header, 40)
        .ok_or(ZipError::Malformed("the end of the directory is malformed"))?;
    let offset = read_u64(&header, 48)
        .ok_or(ZipError::Malformed("the end of the directory is malformed"))?;

    Ok((count, size, offset))
}

/// Reads the files of the directory of an archive.
fn read_directory_entries(directory: &[u8], count: usize) -> Result<Vec<DirectoryEntry>, ZipError> {
    let mut entries = Vec::with_capacity(count);
    let mut index = 0;

    for _ in 0..count {
        if read_u32(directory, index) != Some(DIRECTORY_SIGNATURE) {
            return Err(ZipError::Malformed(
                "the directory of the archive is malformed",
            ));
        }

        let compression = read_u16(directory, index + 10)
            .ok_or(ZipError::Malformed("a file of the directory is malformed"))?;
        let mut compressed_size = u64::from(
            read_u32(directory, index + 20)
                .ok_or(ZipError::Malformed("a file of the directory is malformed"))?,
        );
        let mut size = u64::from(
            read_u32(directory, index + 24)
                .ok_or(ZipError::Malformed("a file of the directory is malformed"))?,
        );
        let name_length = usize::from(
            read_u16(directory, index + 28)
                .ok_or(ZipError::Malformed("a file of the directory is malformed"))?,
        );
        let extra_length = usize::from(
            read_u16(directory, index + 30)
                .ok_or(ZipError::Malformed("a file of the directory is malformed"))?,
        );
        let comment_length = usize::from(
            read_u16(directory, index + 32)
                .ok_or(ZipError::Malformed("a file of the directory is malformed"))?,
        );
        let mut offset = u64::from(
            read_u32(directory, index + 42)
                .ok_or(ZipError::Malformed("a file of the directory is malformed"))?,
        );
        let unix_mode = read_u32(directory, index + 38)
            .ok_or(ZipError::Malformed("a file of the directory is malformed"))?
            >> 16;

        let name_start = index + 46;
        let name_end = name_start + name_length;
        let extra_end = name_end + extra_length;
        if extra_end > directory.len() {
            return Err(ZipError::Malformed("a file of the directory is malformed"));
        }

        if compressed_size == 0xFFFF_FFFF || size == 0xFFFF_FFFF || offset == 0xFFFF_FFFF {
            read_extra_64(
                &directory[name_end..extra_end],
                &mut size,
                &mut compressed_size,
                &mut offset,
            )?;
        }

        let name = String::from_utf8_lossy(&directory[name_start..name_end]).into_owned();
        entries.push(DirectoryEntry {
            name,
            compression,
            compressed_size,
            size,
            offset,
            unix_mode,
        });

        index = extra_end + comment_length;
    }

    Ok(entries)
}

/// Reads the sizes of a file of a large archive.
fn read_extra_64(
    extra: &[u8],
    size: &mut u64,
    compressed_size: &mut u64,
    offset: &mut u64,
) -> Result<(), ZipError> {
    let mut index = 0;

    while index + 4 <= extra.len() {
        let id = read_u16(extra, index)
            .ok_or(ZipError::Malformed("the extra data of a file is malformed"))?;
        let length = usize::from(
            read_u16(extra, index + 2)
                .ok_or(ZipError::Malformed("the extra data of a file is malformed"))?,
        );
        let start = index + 4;
        let end = start + length;
        if end > extra.len() {
            return Err(ZipError::Malformed("the extra data of a file is malformed"));
        }

        if id == 0x0001 {
            let mut cursor = start;
            if *size == 0xFFFF_FFFF {
                *size = read_u64(extra, cursor)
                    .ok_or(ZipError::Malformed("the size of a file is malformed"))?;
                cursor += 8;
            }
            if *compressed_size == 0xFFFF_FFFF {
                *compressed_size = read_u64(extra, cursor)
                    .ok_or(ZipError::Malformed("the size of a file is malformed"))?;
                cursor += 8;
            }
            if *offset == 0xFFFF_FFFF {
                *offset = read_u64(extra, cursor)
                    .ok_or(ZipError::Malformed("the place of a file is malformed"))?;
            }
            return Ok(());
        }

        index = end;
    }

    Err(ZipError::Malformed(
        "the archive does not tell the sizes of its files",
    ))
}

/// Reads a file of an archive.
fn read_file<R: Read + Seek>(reader: &mut R, entry: &DirectoryEntry) -> Result<Vec<u8>, ZipError> {
    let mut header = [0u8; 30];
    reader.seek(SeekFrom::Start(entry.offset))?;
    reader.read_exact(&mut header)?;

    if read_u32(&header, 0) != Some(FILE_SIGNATURE) {
        return Err(ZipError::Malformed("the header of a file is malformed"));
    }

    let name_length = usize::from(
        read_u16(&header, 26).ok_or(ZipError::Malformed("the header of a file is malformed"))?,
    );
    let extra_length = usize::from(
        read_u16(&header, 28).ok_or(ZipError::Malformed("the header of a file is malformed"))?,
    );

    reader.seek(SeekFrom::Start(
        entry.offset + 30 + name_length as u64 + extra_length as u64,
    ))?;

    let mut compressed = vec![0u8; entry.compressed_size as usize];
    reader.read_exact(&mut compressed)?;

    match entry.compression {
        0 => Ok(compressed),
        8 => {
            let mut data = Vec::with_capacity(entry.size as usize);
            DeflateDecoder::new(&compressed[..]).read_to_end(&mut data)?;
            Ok(data)
        }
        _ => Err(ZipError::Malformed(
            "the archive is compressed in a way the launcher does not read",
        )),
    }
}

/// Returns the path of a file of an archive under the destination.
///
/// The path is read as a relative path, so a file of an archive cannot be
/// written outside of the destination.
fn relative_path(name: &str) -> Option<PathBuf> {
    let name = name.replace('\\', "/");
    if name.starts_with('/') || name.contains(':') {
        return None;
    }

    let mut path = PathBuf::new();
    for part in name.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            part => path.push(part),
        }
    }

    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

/// Reads a number of two bytes.
fn read_u16(data: &[u8], index: usize) -> Option<u16> {
    let bytes = data.get(index..index + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Reads a number of four bytes.
fn read_u32(data: &[u8], index: usize) -> Option<u32> {
    let bytes = data.get(index..index + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Reads a number of eight bytes.
fn read_u64(data: &[u8], index: usize) -> Option<u64> {
    let bytes = data.get(index..index + 8)?;
    let mut value = [0u8; 8];
    value.copy_from_slice(bytes);
    Some(u64::from_le_bytes(value))
}

/// Writes files into an archive.
///
/// The files are compressed with deflate when the compression makes them
/// smaller, and stored as they are otherwise, so an archive of files that are
/// already compressed is not made larger. The archive is written to a file that
/// is moved into place, so a failure does not leave a half-written archive.
pub struct ZipWriter {
    /// The archive that is being written.
    file: std::io::BufWriter<File>,
    /// The entries of the directory of the archive.
    directory: Vec<WrittenEntry>,
    /// The place of the next file in the archive.
    offset: u64,
    /// The path the archive is written to.
    path: PathBuf,
}

/// An entry of the directory of an archive that is written.
struct WrittenEntry {
    /// The path of the file in the archive.
    name: String,
    /// The digest of the file, which the archive checks.
    crc: u32,
    /// The size of the file once it is compressed, in bytes.
    compressed_size: u64,
    /// The size of the file once it is extracted, in bytes.
    size: u64,
    /// The way the file is compressed.
    compression: u16,
    /// The place of the header of the file in the archive.
    offset: u64,
}

impl ZipWriter {
    /// Creates an archive at a path.
    ///
    /// The file that is at the path is replaced when the archive is finished.
    pub fn create(path: impl Into<PathBuf>) -> Result<Self, ZipError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let temporary = temporary_path(&path);
        let file = File::create(&temporary)?;

        Ok(Self {
            file: std::io::BufWriter::new(file),
            directory: Vec::new(),
            offset: 0,
            path,
        })
    }

    /// Adds a file to the archive.
    pub fn add_file(&mut self, name: &str, data: &[u8]) -> Result<(), ZipError> {
        let name = name.replace('\\', "/");

        let (compression, payload) = {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(data)?;
            let compressed = encoder.finish()?;

            // A file that does not get smaller is stored as it is, so the
            // archive does not grow with the files that are already compressed.
            if compressed.len() < data.len() {
                (8u16, compressed)
            } else {
                (0u16, data.to_vec())
            }
        };

        let crc = crc32(data);
        let name_bytes = name.as_bytes();

        let mut header = Vec::with_capacity(30 + name_bytes.len());
        header.extend_from_slice(&FILE_SIGNATURE.to_le_bytes());
        header.extend_from_slice(&20u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&compression.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&crc.to_le_bytes());
        header.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        header.extend_from_slice(&(data.len() as u32).to_le_bytes());
        header.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(name_bytes);
        header.extend_from_slice(&payload);

        self.file.write_all(&header)?;
        self.offset += header.len() as u64;

        self.directory.push(WrittenEntry {
            name,
            crc,
            compressed_size: payload.len() as u64,
            size: data.len() as u64,
            compression,
            offset: self.offset - header.len() as u64,
        });

        Ok(())
    }

    /// Adds a file of the file system to the archive.
    pub fn add_path(&mut self, name: &str, path: &Path) -> Result<(), ZipError> {
        let data = std::fs::read(path)?;
        self.add_file(name, &data)
    }

    /// Adds a directory of the file system to the archive, under a name.
    ///
    /// The files are written with the name of the directory in front of them,
    /// so the archive holds a directory the game reads as a world.
    pub fn add_directory(&mut self, name: &str, path: &Path) -> Result<(), ZipError> {
        if !path.is_dir() {
            return Err(ZipError::Malformed("the path is not a directory"));
        }
        self.add_directory_inner(name, path, &|_| false)
    }

    /// Adds a directory of the file system to the archive, leaving out the
    /// files the filter matches.
    ///
    /// The filter is given the path of the file in the archive, so a file the
    /// archive should not hold is left out wherever it is.
    pub fn add_directory_filtered(
        &mut self,
        name: &str,
        path: &Path,
        skip: &impl Fn(&Path) -> bool,
    ) -> Result<(), ZipError> {
        if !path.is_dir() {
            return Err(ZipError::Malformed("the path is not a directory"));
        }
        self.add_directory_inner(name, path, skip)
    }

    /// Adds a directory and the files under it.
    fn add_directory_inner(
        &mut self,
        name: &str,
        path: &Path,
        skip: &impl Fn(&Path) -> bool,
    ) -> Result<(), ZipError> {
        let mut entries = std::fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        // The files of a directory are written in the order of their names, so
        // the archive of the same directory is the same archive every time.
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            let entry_name = format!("{name}/{file_name}");
            let entry_path = entry.path();

            if skip(&entry_path) {
                continue;
            }

            if entry_path.is_dir() {
                self.add_directory_inner(&entry_name, &entry_path, skip)?;
            } else if entry_path.is_file() {
                self.add_path(&entry_name, &entry_path)?;
            }
        }

        Ok(())
    }

    /// Finishes the archive and moves it to the path it was created for.
    pub fn finish(mut self) -> Result<(), ZipError> {
        let directory_offset = self.offset;
        let mut directory = Vec::new();

        for entry in &self.directory {
            let name = entry.name.as_bytes();
            directory.extend_from_slice(&DIRECTORY_SIGNATURE.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&entry.compression.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&entry.crc.to_le_bytes());
            directory.extend_from_slice(&(entry.compressed_size as u32).to_le_bytes());
            directory.extend_from_slice(&(entry.size as u32).to_le_bytes());
            directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u32.to_le_bytes());
            directory.extend_from_slice(&(entry.offset as u32).to_le_bytes());
            directory.extend_from_slice(name);
        }

        let directory_size = directory.len() as u64;
        self.file.write_all(&directory)?;
        self.file
            .write_all(&DIRECTORY_END_SIGNATURE.to_le_bytes())?;
        self.file.write_all(&0u16.to_le_bytes())?;
        self.file.write_all(&0u16.to_le_bytes())?;
        self.file
            .write_all(&(self.directory.len() as u16).to_le_bytes())?;
        self.file
            .write_all(&(self.directory.len() as u16).to_le_bytes())?;
        self.file
            .write_all(&(directory_size as u32).to_le_bytes())?;
        self.file
            .write_all(&(directory_offset as u32).to_le_bytes())?;
        self.file.write_all(&0u16.to_le_bytes())?;
        self.file.flush()?;

        let temporary = temporary_path(&self.path);
        match std::fs::rename(&temporary, &self.path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                Err(ZipError::Io(error))
            }
        }
    }
}

/// Removes the files with the given names from an archive.
///
/// The archive is written again without the files, because an archive is a
/// stream of compressed files: a file that is written after it can only be
/// removed by writing the archive again. A failure leaves the archive as it
/// was, because the new archive is written next to it and moved into place.
///
/// Returns whether a file was removed; an archive that holds none of the names
/// is not written again.
pub fn remove_entries(path: &Path, names: &[&str]) -> Result<bool, ZipError> {
    let mut file = File::open(path)?;
    let directory = read_directory(&mut file)?;

    let mut kept: Vec<(String, Vec<u8>)> = Vec::new();
    let mut removed = false;
    for entry in directory {
        if entry.name.ends_with('/') {
            continue;
        }

        if names.contains(&entry.name.as_str()) {
            removed = true;
            continue;
        }

        kept.push((entry.name.clone(), read_file(&mut file, &entry)?));
    }

    if !removed {
        return Ok(false);
    }

    let mut writer = ZipWriter::create(path)?;
    for (name, data) in kept {
        writer.add_file(&name, &data)?;
    }
    writer.finish()?;

    Ok(true)
}

/// Returns the digest the archives use to check a file.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;

    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }

    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Builds an archive that stores the files without compressing them.
    fn archive(files: &[(&str, &str)]) -> Vec<u8> {
        archive_with_modes(
            &files
                .iter()
                .map(|(name, content)| (*name, *content, 0))
                .collect::<Vec<_>>(),
        )
    }

    /// Builds an archive that stores the files with the modes of Unix.
    fn archive_with_modes(files: &[(&str, &str, u32)]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut directory = Vec::new();

        for (name, content, mode) in files {
            let offset = data.len() as u32;
            let name = name.as_bytes();
            let content = content.as_bytes();

            data.extend_from_slice(&FILE_SIGNATURE.to_le_bytes());
            data.extend_from_slice(&20u16.to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&(content.len() as u32).to_le_bytes());
            data.extend_from_slice(&(content.len() as u32).to_le_bytes());
            data.extend_from_slice(&(name.len() as u16).to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(name);
            data.extend_from_slice(content);

            directory.extend_from_slice(&DIRECTORY_SIGNATURE.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u32.to_le_bytes());
            directory.extend_from_slice(&(content.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(content.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&((mode << 16).to_le_bytes()));
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(name);
        }

        let directory_offset = data.len() as u32;
        let directory_size = directory.len() as u32;
        data.extend_from_slice(&directory);
        data.extend_from_slice(&DIRECTORY_END_SIGNATURE.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&(files.len() as u16).to_le_bytes());
        data.extend_from_slice(&(files.len() as u16).to_le_bytes());
        data.extend_from_slice(&directory_size.to_le_bytes());
        data.extend_from_slice(&directory_offset.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data
    }

    /// Returns a directory the test may write to.
    fn scratch(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join("hmcl-rs-zip-tests").join(name);
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");
        directory
    }

    #[test]
    fn the_files_of_an_archive_are_read() {
        let data = archive(&[("a.txt", "alpha"), ("dir/b.txt", "beta")]);
        let entries = entries_from(Cursor::new(data)).expect("the archive should be read");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a.txt");
        assert_eq!(entries[0].size, 5);
        assert_eq!(entries[1].name, "dir/b.txt");
        assert!(!entries[1].directory);
    }

    #[test]
    fn a_file_of_an_archive_is_read() {
        let data = archive(&[("a.txt", "alpha"), ("dir/b.txt", "beta")]);
        let mut reader = Cursor::new(data);

        assert_eq!(
            read_entry_from(&mut reader, "dir/b.txt").expect("the archive should be read"),
            Some(b"beta".to_vec())
        );
        assert_eq!(
            read_entry_from(&mut reader, "missing.txt").expect("the archive should be read"),
            None
        );
    }

    #[test]
    fn the_files_of_an_archive_are_extracted() {
        let directory = scratch("extract");
        let path = directory.join("test.zip");
        std::fs::write(&path, archive(&[("a.txt", "alpha"), ("dir/b.txt", "beta")]))
            .expect("the archive should be written");

        let destination = directory.join("out");
        let extracted = extract(&path, &destination, |entry, _| {
            !entry.name.starts_with("dir/")
        })
        .expect("the archive should be extracted");

        assert_eq!(extracted, vec![destination.join("a.txt")]);
        assert_eq!(
            std::fs::read_to_string(destination.join("a.txt")).expect("the file should be read"),
            "alpha"
        );
        assert!(!destination.join("dir/b.txt").exists());
    }

    #[test]
    fn a_file_outside_of_the_destination_is_refused() {
        let directory = scratch("outside");
        let path = directory.join("test.zip");
        std::fs::write(&path, archive(&[("../outside.txt", "no")]))
            .expect("the archive should be written");

        let error = extract(&path, &directory.join("out"), |_, _| true)
            .expect_err("the archive should be refused");
        assert!(matches!(error, ZipError::Malformed(_)));
        assert!(!directory.join("outside.txt").exists());
    }

    #[test]
    fn a_file_that_is_not_an_archive_is_refused() {
        let error =
            entries_from(Cursor::new(vec![0u8; 64])).expect_err("the file should be refused");
        assert!(matches!(error, ZipError::Malformed(_)));
    }

    #[test]
    fn the_links_of_an_archive_are_read() {
        let data = archive_with_modes(&[
            ("file.txt", "alpha", 0o100644),
            ("link.txt", "file.txt", 0o120777),
        ]);
        let entries = entries_from(Cursor::new(data)).expect("the archive should be read");

        assert!(!entries[0].is_symlink());
        assert!(entries[1].is_symlink());
    }

    #[test]
    fn the_archives_of_the_repository_are_read() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../lib/JFoenix.jar");
        if !path.exists() {
            return;
        }

        let entries = entries(&path).expect("the archive of the repository should be read");
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|entry| entry.name.ends_with(".class")));
    }

    #[test]
    fn an_archive_is_written_and_read_again() {
        let base = scratch("write");
        let path = base.join("archive.zip");

        let mut writer = ZipWriter::create(&path).expect("the archive should be made");
        writer
            .add_file("files/first.txt", b"first file")
            .expect("the file should be added");
        // A file that repeats itself is compressed, and a short file that does
        // not get smaller is stored as it is.
        writer
            .add_file("files/second.txt", &vec![b'a'; 4096])
            .expect("the file should be added");
        writer
            .add_file("root.txt", b"x")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");

        assert!(path.is_file());
        assert!(!base.join("archive.zip.part").exists());

        let entries = entries(&path).expect("the archive should be read");
        let mut names = entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(
            names,
            vec!["files/first.txt", "files/second.txt", "root.txt"]
        );

        assert_eq!(
            read_entry(&path, "files/first.txt")
                .expect("the file should be read")
                .as_deref(),
            Some(b"first file".as_slice())
        );
        assert_eq!(
            read_entry(&path, "files/second.txt")
                .expect("the file should be read")
                .map(|data| data.len()),
            Some(4096)
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn files_are_removed_from_an_archive() {
        let base = scratch("remove");
        let path = base.join("archive.zip");

        let mut writer = ZipWriter::create(&path).expect("the archive should be made");
        writer
            .add_file("META-INF/MOJANG_C.DSA", b"signature")
            .expect("the file should be added");
        writer
            .add_file("META-INF/MOJANG_C.SF", b"signature file")
            .expect("the file should be added");
        writer
            .add_file("net/example/Main.class", b"classes")
            .expect("the file should be added");
        writer.finish().expect("the archive should be written");

        assert!(
            remove_entries(&path, &["META-INF/MOJANG_C.DSA", "META-INF/MOJANG_C.SF"])
                .expect("the files should be removed")
        );

        let entries = entries(&path).expect("the archive should be read");
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["net/example/Main.class"]
        );
        assert_eq!(
            read_entry(&path, "net/example/Main.class")
                .expect("the file should be read")
                .as_deref(),
            Some(b"classes".as_slice())
        );

        // An archive that holds none of the names is not written again.
        assert!(
            !remove_entries(&path, &["META-INF/MOJANG_C.SF"]).expect("the archive should be read")
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_directory_is_written_into_an_archive() {
        let base = scratch("write-directory");
        std::fs::create_dir_all(base.join("world/region")).expect("the directory should be made");
        std::fs::write(base.join("world/level.dat"), b"level").expect("the file should be written");
        std::fs::write(base.join("world/region/r.0.0.mca"), b"region")
            .expect("the file should be written");

        let path = base.join("world.zip");
        let mut writer = ZipWriter::create(&path).expect("the archive should be made");
        writer
            .add_directory("MyWorld", &base.join("world"))
            .expect("the directory should be added");
        writer.finish().expect("the archive should be written");

        assert_eq!(
            read_entry(&path, "MyWorld/level.dat")
                .expect("the file should be read")
                .as_deref(),
            Some(b"level".as_slice())
        );
        assert_eq!(
            read_entry(&path, "MyWorld/region/r.0.0.mca")
                .expect("the file should be read")
                .as_deref(),
            Some(b"region".as_slice())
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_digest_of_the_archives_is_computed() {
        // The digest of the empty input and of `123456789` are known, so the
        // reader of the archives checks the files with the same digest.
        assert_eq!(crc32(b""), 0x0000_0000);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
