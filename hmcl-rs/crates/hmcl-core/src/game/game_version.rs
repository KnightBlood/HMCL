//! Reading the version of the game from the file of the game.
//!
//! An instance of the game is named by the user, so the name of the instance
//! does not tell the version of the game it runs. The launcher reads the file
//! of the game instead: the file of the modern versions tells the version, and
//! the classes of the older versions keep the version as a text.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use crate::util::zip::{self, ZipError};

/// Reads the version of the game from the file of the game.
///
/// The version is read from the `version.json` of the file, the constants of
/// the client class, or the constants of the server class, in this order.
///
/// The very old versions that tell no version are read by the digest of the
/// file by the version of the launcher written in Java; this launcher does not
/// read that table yet, so those versions return nothing.
///
/// A file that is not there or not an archive returns nothing rather than an
/// error: a game directory may hold an instance whose file was not downloaded
/// yet.
pub fn minecraft_version(jar: &Path) -> Result<Option<String>, ZipError> {
    if !jar.is_file() {
        return Ok(None);
    }

    let mut file = File::open(jar)?;

    if let Some(contents) = zip::read_entry_from(&mut file, "version.json")? {
        if let Some(version) = version_from_json(&contents) {
            return Ok(Some(version));
        }
    }

    if let Some(contents) = zip::read_entry_from(&mut file, "net/minecraft/client/Minecraft.class")?
    {
        if let Some(version) = version_from_client_class(&contents) {
            return Ok(Some(version));
        }
    }

    if let Some(contents) =
        zip::read_entry_from(&mut file, "net/minecraft/server/MinecraftServer.class")?
    {
        if let Some(version) = version_from_server_class(&contents) {
            return Ok(Some(version));
        }
    }

    Ok(None)
}

/// Reads the version from the `version.json` of the game.
///
/// The identifier of a version that names two versions, such as
/// `1.20.1 / 1.20.2`, names the first one as the version of the game.
fn version_from_json(contents: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(contents).ok()?;
    let id = value.get("id")?.as_str()?;

    Some(id.split(" / ").next().unwrap_or(id).to_string())
}

/// Reads the version from the constants of the client class of the game.
///
/// The class keeps the version as the text `Minecraft Minecraft <version>`;
/// the versions before the release name the version as `Beta <version>` or
/// `Alpha v<version>`, which the launcher writes as `b<version>` and
/// `a<version>`.
///
/// The release candidates keep `RC1`/`RC2` rather than the version, so the
/// launcher reads the version of those files from the server class.
fn version_from_client_class(contents: &[u8]) -> Option<String> {
    const PREFIX: &str = "Minecraft Minecraft ";

    let strings = class_strings(contents)?;
    let version = strings.iter().find_map(|text| text.strip_prefix(PREFIX))?;

    match version {
        "RC1" | "RC2" => None,
        version if version.starts_with("Beta ") => Some(format!("b{}", &version[5..])),
        version if version.starts_with("Alpha v") => Some(format!("a{}", &version[7..])),
        version => Some(version.to_string()),
    }
}

/// Reads the version from the constants of the server class of the game.
///
/// The server class keeps the version before the text of the message it writes
/// when the server cannot keep up with the game.
fn version_from_server_class(contents: &[u8]) -> Option<String> {
    let strings = class_strings(contents)?;
    let index = strings
        .iter()
        .position(|text| text.starts_with("Can't keep up!"))?;

    strings[..index]
        .iter()
        .rev()
        .find(|text| text.chars().any(|character| character.is_ascii_digit()))
        .cloned()
}

/// Returns the texts of the constants of a class.
///
/// The launcher reads the texts the class keeps as string constants, in the
/// order the class keeps them.
fn class_strings(contents: &[u8]) -> Option<Vec<String>> {
    const MAGIC: [u8; 4] = [0xCA, 0xFE, 0xBA, 0xBE];

    if contents.get(..4)? != MAGIC {
        return None;
    }

    // The magic number, the minor version and the major version.
    let mut index = 8;
    let count = usize::from(read_u16(contents, &mut index)?);
    let mut texts: HashMap<u16, String> = HashMap::new();
    let mut references: Vec<u16> = Vec::new();
    let mut slot = 1u16;

    while usize::from(slot) < count {
        let tag = *contents.get(index)?;
        index += 1;

        match tag {
            // The text of a name, a descriptor or a string.
            1 => {
                let length = usize::from(read_u16(contents, &mut index)?);
                let bytes = contents.get(index..index + length)?;
                index += length;
                texts.insert(slot, String::from_utf8_lossy(bytes).into_owned());
            }
            // An integer and a float.
            3 | 4 => index += 4,
            // A long and a double take two entries of the table.
            5 | 6 => {
                index += 8;
                slot = slot.checked_add(1)?;
            }
            // A class, a method type, a module and a package.
            7 | 16 | 19 | 20 => index += 2,
            // A string, which refers to a text.
            8 => references.push(read_u16(contents, &mut index)?),
            // A field, a method, an interface method, a name and a type, a
            // dynamic constant and an invokedynamic constant.
            9 | 10 | 11 | 12 | 17 | 18 => index += 4,
            // A method handle.
            15 => index += 3,
            _ => return None,
        }

        slot = slot.checked_add(1)?;
    }

    Some(
        references
            .into_iter()
            .filter_map(|reference| texts.get(&reference).cloned())
            .collect(),
    )
}

/// Reads a number of two bytes that the class stores from the most important
/// byte.
fn read_u16(contents: &[u8], index: &mut usize) -> Option<u16> {
    let bytes = contents.get(*index..*index + 2)?;
    *index += 2;

    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the bytes of a class that keeps the texts as string constants.
    fn class(texts: &[&str]) -> Vec<u8> {
        let mut contents = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52];
        contents.extend_from_slice(&((texts.len() as u16) * 2 + 1).to_be_bytes());

        for (index, text) in texts.iter().enumerate() {
            // The text, which the string constant refers to.
            contents.push(1);
            contents.extend_from_slice(&(text.len() as u16).to_be_bytes());
            contents.extend_from_slice(text.as_bytes());

            // The string constant.
            contents.push(8);
            contents.extend_from_slice(&((index as u16) * 2 + 1).to_be_bytes());
        }

        contents
    }

    /// Builds an archive that stores the files without compressing them.
    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut directory = Vec::new();

        for (name, contents) in files {
            let offset = data.len() as u32;

            data.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            data.extend_from_slice(&20u16.to_le_bytes());
            data.extend_from_slice(&[0; 8]);
            data.extend_from_slice(&0u32.to_le_bytes());
            data.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            data.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            data.extend_from_slice(&(name.len() as u16).to_le_bytes());
            data.extend_from_slice(&0u16.to_le_bytes());
            data.extend_from_slice(name.as_bytes());
            data.extend_from_slice(contents);

            directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&[0; 8]);
            directory.extend_from_slice(&0u32.to_le_bytes());
            directory.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&0u16.to_le_bytes());
            directory.extend_from_slice(&(0o100644u32 << 16).to_le_bytes());
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(name.as_bytes());
        }

        let offset = data.len() as u32;
        let size = directory.len() as u32;
        data.extend_from_slice(&directory);
        data.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&(files.len() as u16).to_le_bytes());
        data.extend_from_slice(&(files.len() as u16).to_le_bytes());
        data.extend_from_slice(&size.to_le_bytes());
        data.extend_from_slice(&offset.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());

        data
    }

    #[test]
    fn the_version_of_json_is_read() {
        assert_eq!(
            version_from_json(br#"{"id": "1.20.1"}"#),
            Some("1.20.1".to_string())
        );
        assert_eq!(
            version_from_json(br#"{"id": "1.20.1 / 1.20.2"}"#),
            Some("1.20.1".to_string())
        );
        assert_eq!(version_from_json(br#"{"id": 17}"#), None);
        assert_eq!(version_from_json(b"not json"), None);
    }

    #[test]
    fn the_version_of_the_client_class_is_read() {
        assert_eq!(
            version_from_client_class(&class(&["Minecraft Minecraft 1.12.2"])),
            Some("1.12.2".to_string())
        );
        assert_eq!(
            version_from_client_class(&class(&["Minecraft Minecraft Beta 1.7.3"])),
            Some("b1.7.3".to_string())
        );
        assert_eq!(
            version_from_client_class(&class(&["Minecraft Minecraft Alpha v1.2.6"])),
            Some("a1.2.6".to_string())
        );
        assert_eq!(
            version_from_client_class(&class(&["Minecraft Minecraft RC1"])),
            None
        );
        assert_eq!(version_from_client_class(&class(&["1.12.2"])), None);
        assert_eq!(version_from_client_class(b"not a class"), None);
    }

    #[test]
    fn the_version_of_the_server_class_is_read() {
        assert_eq!(
            version_from_server_class(&class(&["1.2.5", "Can't keep up!"])),
            Some("1.2.5".to_string())
        );
        assert_eq!(version_from_server_class(&class(&["Can't keep up!"])), None);
    }

    #[test]
    fn the_version_of_the_file_of_the_game_is_read() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-game-version-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");

        let with_json = directory.join("with-json.jar");
        std::fs::write(
            &with_json,
            archive(&[("version.json", br#"{"id": "1.20.1"}"#)]),
        )
        .expect("the archive should be written");
        assert_eq!(
            minecraft_version(&with_json).expect("the archive should be read"),
            Some("1.20.1".to_string())
        );

        let with_class = directory.join("with-class.jar");
        std::fs::write(
            &with_class,
            archive(&[(
                "net/minecraft/client/Minecraft.class",
                &class(&["Minecraft Minecraft 1.12.2"]),
            )]),
        )
        .expect("the archive should be written");
        assert_eq!(
            minecraft_version(&with_class).expect("the archive should be read"),
            Some("1.12.2".to_string())
        );

        let without_version = directory.join("without-version.jar");
        std::fs::write(&without_version, archive(&[("a.txt", b"alpha")]))
            .expect("the archive should be written");
        assert_eq!(
            minecraft_version(&without_version).expect("the archive should be read"),
            None
        );

        assert_eq!(
            minecraft_version(&directory.join("missing.jar")).expect("the file is not there"),
            None
        );

        let _ = std::fs::remove_dir_all(&directory);
    }
}
