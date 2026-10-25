//! The languages and the ways their texts are written.
//!
//! The launcher reads a language from a tag of BCP 47, which names the
//! language, the way its text is written, the place it is spoken in and the
//! kind of that place. The tags are read the way the launcher of Java reads
//! them, so the two launchers pick the same translation of a file.
//!
//! A language is written in more than one way: Chinese is written with the
//! characters of the mainland and with the characters of Taiwan, and the
//! launcher tells them apart because the translation of one is not the
//! translation of the other. A language that has no way of writing named in
//! its tag gets the way the launcher knows, and a language that is spoken in
//! more than one place gets the place of its tag.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The languages that are written from the right to the left.
///
/// The scripts of these languages are the ones the launcher knows write their
/// text from the right to the left.
const RIGHT_TO_LEFT_SCRIPTS: [&str; 3] = ["Qabs", "Arab", "Hebr"];

/// The places where Chinese is written with the traditional characters.
const CHINESE_TRADITIONAL_REGIONS: [&str; 3] = ["TW", "HK", "MO"];

/// The languages that are the most used of the languages that hold more than
/// one of them.
///
/// Chinese is spoken as Mandarin, Cantonese and more, and a player who writes
/// `zh` usually means Mandarin, so the launcher reads the tag of the language
/// of the mainland first.
const PREFERRED_LANGUAGES: [(&str, &str); 1] = [("zh", "cmn")];

/// The key of the language that has no tag of its own.
///
/// A file that has no language in its name is the file of every language, so
/// it is read when no other file is.
pub const DEFAULT_LANGUAGE_KEY: &str = "default";

/// The way the text of a language is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextDirection {
    /// The text is written from the left to the right, like the text of
    /// English.
    #[default]
    LeftToRight,
    /// The text is written from the right to the left, like the text of
    /// Arabic.
    RightToLeft,
}

/// A language, the way its text is written, the place it is spoken in and the
/// kind of that place.
///
/// The tag is written the way BCP 47 writes it, which is the way the launcher
/// of Java writes it, so a file named after a tag is found by both launchers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct LanguageTag {
    /// The language, such as `en` or `zh`.
    pub language: String,
    /// The way the text of the language is written, such as `Hans`.
    pub script: String,
    /// The place the language is spoken in, such as `CN`.
    pub region: String,
    /// The kind of the place, such as `1996`.
    pub variant: String,
}

impl LanguageTag {
    /// Returns the tag of no language at all.
    ///
    /// The tag is the one the launcher reads after every other language, so a
    /// text that is written in no language is read in it.
    pub fn root() -> Self {
        Self::default()
    }

    /// Reads a tag the way BCP 47 writes it.
    ///
    /// A tag that cannot be read is kept as a language, so the launcher does
    /// not lose the name the player gave.
    pub fn parse(tag: &str) -> Self {
        let mut result = Self::default();
        let mut parts = tag.split(['-', '_']).filter(|part| !part.is_empty());

        if let Some(language) = parts.next() {
            result.language = language.to_lowercase();
        }

        for part in parts {
            // The first part of four letters is the way the text is written,
            // the part of two letters or three numbers is the place, and the
            // parts that are left are the kinds of the place.
            if result.script.is_empty()
                && part.len() == 4
                && part.chars().all(|c| c.is_ascii_alphabetic())
            {
                result.script = uppercase_first(part);
            } else if result.region.is_empty()
                && (part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic())
                    || part.len() == 3 && part.chars().all(|c| c.is_ascii_digit()))
            {
                result.region = part.to_uppercase();
            } else if result.variant.is_empty() {
                result.variant = part.to_lowercase();
            } else {
                result.variant.push('-');
                result.variant.push_str(&part.to_lowercase());
            }
        }

        result
    }

    /// Returns the tag of the language, the way the launcher names the files
    /// that are written in it.
    ///
    /// The tag of a language that has no language at all is
    /// [`DEFAULT_LANGUAGE_KEY`], which is the key of the files that are read
    /// by every language.
    pub fn language_key(&self) -> String {
        if self.language.is_empty() {
            return DEFAULT_LANGUAGE_KEY.to_string();
        }

        let mut key = self.language.clone();
        if !self.script.is_empty() {
            key.push('-');
            key.push_str(&self.script);
        }
        if !self.region.is_empty() {
            key.push('-');
            key.push_str(&self.region);
        }
        if !self.variant.is_empty() {
            key.push('-');
            key.push_str(&self.variant);
        }
        key
    }

    /// Returns whether the language is the one of the English texts.
    pub fn is_english(&self) -> bool {
        root_language(&self.language) == "en"
    }

    /// Returns whether the language is the one of the Chinese texts.
    pub fn is_chinese(&self) -> bool {
        root_language(&self.language) == "zh"
    }

    /// Returns the way the text of the language is written.
    ///
    /// The way of writing of a tag that names none is the way the launcher
    /// knows for the language, and a Chinese tag is read with the characters
    /// of the place it names. The way of writing is empty when the launcher
    /// knows none.
    pub fn script(&self) -> String {
        if !self.script.is_empty() {
            return self.script.clone();
        }

        // A kind of place tells the way of writing by itself, for the place
        // of a kind is written in one way.
        if !self.variant.is_empty() {
            if let Some(script) = default_scripts().get(&self.variant) {
                return script.clone();
            }
        }

        // The place of the upside down text is written upside down.
        if self.region == "UD" {
            return "Qabs".to_string();
        }

        if let Some(script) = default_scripts().get(&normalize_language(&self.language)) {
            return script.clone();
        }

        if self.is_chinese() {
            return if CHINESE_TRADITIONAL_REGIONS.contains(&self.region.as_str()) {
                "Hant".to_string()
            } else {
                "Hans".to_string()
            };
        }

        String::new()
    }

    /// Returns the way the text of the language is written in.
    pub fn text_direction(&self) -> TextDirection {
        if RIGHT_TO_LEFT_SCRIPTS.contains(&self.script().as_str()) {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        }
    }

    /// Returns the languages the launcher looks for a text in, the one it
    /// looks for first first.
    ///
    /// A player who reads the launcher in the Chinese of Taiwan can read the
    /// texts of the Chinese of the mainland as well, so the launcher looks for
    /// the text of the place before the text of the language. The list ends
    /// with the tag of no language at all, which is the text of every player.
    ///
    /// A language that is spoken in more than one way is looked for by every
    /// way and by the language that holds them, so a text of Mandarin is read
    /// by a player who reads Chinese.
    pub fn candidate_locales(&self) -> Vec<Self> {
        let language = preferred_language(&self.language);
        let script = self.script();
        let region = self.region.clone();
        let variants: Vec<String> = if self.variant.is_empty() {
            Vec::new()
        } else {
            self.variant
                .split(['-', '_'])
                .map(|part| part.to_string())
                .collect()
        };

        let mut result = Vec::new();
        let mut language = Some(language);
        while let Some(name) = language {
            add_candidate_locales(&mut result, &name, &script, &region, &variants);
            language = parent_language(&name).map(str::to_string);
        }

        result.push(Self::root());
        result
    }
}

impl fmt::Display for LanguageTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.language_key())
    }
}

impl From<&str> for LanguageTag {
    /// Reads a tag, and keeps it as a language when it cannot be read.
    fn from(tag: &str) -> Self {
        Self::parse(tag)
    }
}

/// Returns the language of the code, the way the names of the languages are
/// written.
///
/// A language that is written with three letters is the language of the names
/// the machines use, so the launcher reads the code of two letters it is known
/// by. An empty language is read as English, which is the language every other
/// language falls back to.
pub fn normalize_language(language: &str) -> String {
    if language.is_empty() {
        return "en".to_string();
    }

    language_aliases()
        .get(language)
        .cloned()
        .unwrap_or_else(|| language.to_string())
}

/// Returns the language that holds the language, if it holds more than one of
/// them.
pub fn parent_language(language: &str) -> Option<&'static str> {
    sub_languages().get(language).map(String::as_str)
}

/// Returns the language the texts of the launcher fall back to.
///
/// A language that is spoken in more than one way is read as the language that
/// holds them, and a language that is written with three letters is read as
/// the language of two letters it is known by.
pub fn root_language(language: &str) -> String {
    let language = normalize_language(language);
    parent_language(&language)
        .map(str::to_string)
        .unwrap_or(language)
}

/// Returns the language that is spoken the most, if the language holds more
/// than one of them.
pub fn preferred_language(language: &str) -> String {
    let language = normalize_language(language);
    PREFERRED_LANGUAGES
        .iter()
        .find(|(name, _)| *name == language)
        .map(|(_, preferred)| (*preferred).to_string())
        .unwrap_or(language)
}

/// Returns the text of the key in the language the tag names, if there is
/// one.
///
/// The languages are looked for in the order the launcher looks for them, so
/// a text of the place is read before a text of the language.
pub fn get_by_candidate_locales<'a, T>(
    entries: &'a [(String, T)],
    candidates: &[LanguageTag],
) -> Option<&'a T> {
    for candidate in candidates {
        let key = candidate.language_key();
        if let Some((_, value)) = entries.iter().find(|(name, _)| *name == key) {
            return Some(value);
        }
    }

    None
}

/// Returns the files of a directory that are written in a language, by the
/// key of the language they are written in.
///
/// A file of the name `base.ext` is the file of every language, and a file of
/// the name `base_language.ext` is the file of the language its name names.
/// The files are kept in the order the directory names them, so the launcher
/// reads them the way the launcher of Java reads them.
pub fn find_all_localized_files(
    directory: &Path,
    base_name: &str,
    extension: &str,
) -> Vec<(String, PathBuf)> {
    let mut result = Vec::new();

    if !directory.is_dir() {
        return result;
    }

    let Ok(entries) = std::fs::read_dir(directory) else {
        return result;
    };

    let suffix = format!(".{extension}");
    let default_name = format!("{base_name}{suffix}");
    let prefixed = format!("{base_name}_");

    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }

        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };

        if file_name == default_name {
            result.push((DEFAULT_LANGUAGE_KEY.to_string(), entry.path()));
        } else if file_name.starts_with(&prefixed) && file_name.ends_with(&suffix) {
            let key = file_name[prefixed.len()..file_name.len() - suffix.len()].replace('_', "-");
            if !key.is_empty() {
                result.push((key, entry.path()));
            }
        }
    }

    result
}

/// Returns the files of a directory that are written in a language, by the
/// key of the language and the kind of the file.
///
/// A file of the name `base.ext` is the file of every language, and a file of
/// the name `base_language.ext` is the file of the language its name names.
/// The languages are kept in the order the directory names them, so the
/// launcher reads them the way the launcher of Java reads them.
pub fn find_all_localized_files_by_extension(
    directory: &Path,
    base_name: &str,
    extensions: &[&str],
) -> Vec<(String, Vec<(String, PathBuf)>)> {
    let mut result: Vec<(String, Vec<(String, PathBuf)>)> = Vec::new();

    if !directory.is_dir() {
        return result;
    }

    let Ok(entries) = std::fs::read_dir(directory) else {
        return result;
    };

    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }

        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };

        // The extension of the file is the part of its name that follows the
        // last dot; a name that has no dot names no extension at all.
        let Some(extension) = file_name.rfind('.') else {
            continue;
        };
        let extension = &file_name[extension + 1..];
        if !extensions.contains(&extension) {
            continue;
        }

        let key = if file_name.len() == base_name.len() + extension.len() + 1 {
            // The name of the file names no language, so the file is the one
            // of every language.
            DEFAULT_LANGUAGE_KEY.to_string()
        } else if file_name.len() > base_name.len() + extension.len() + 2
            && file_name.starts_with(base_name)
            && file_name.as_bytes().get(base_name.len()) == Some(&b'_')
        {
            file_name[base_name.len() + 1..file_name.len() - extension.len() - 1].replace('_', "-")
        } else {
            continue;
        };

        if key.is_empty() {
            continue;
        }

        // A language may hold a file of more than one kind, so the files of a
        // language are kept together.
        match result.iter_mut().find(|(name, _)| *name == key) {
            Some((_, files)) => files.push((extension.to_string(), entry.path())),
            None => result.push((key, vec![(extension.to_string(), entry.path())])),
        }
    }

    result
}

/// Adds the languages a tag is read as, the way the launcher of Java adds
/// them.
///
/// The tag of the whole language is added last, so a text of the language is
/// read before a text of the place. A Chinese tag is read with the characters
/// of the mainland and the characters of Taiwan as well, for a player who
/// writes `zh` means one of them.
fn add_candidate_locales(
    list: &mut Vec<LanguageTag>,
    language: &str,
    script: &str,
    region: &str,
    variants: &[String],
) {
    let of = |script: &str, region: &str, variant: &str| LanguageTag {
        language: language.to_string(),
        script: script.to_string(),
        region: region.to_string(),
        variant: variant.to_string(),
    };

    if !variants.is_empty() {
        for variant in variants {
            list.push(of(script, region, variant));
        }
    }
    if !region.is_empty() {
        list.push(of(script, region, ""));
    }
    if !script.is_empty() {
        list.push(of(script, "", ""));
        if !variants.is_empty() {
            for variant in variants {
                list.push(of("", region, variant));
            }
        }
        if !region.is_empty() {
            list.push(of("", region, ""));
        }
    }

    list.push(of("", "", ""));

    if language == "zh" {
        // A player who writes the Chinese of Taiwan reads the traditional
        // characters, and a player who writes no place at all reads the
        // characters of the mainland.
        let chinese = of("", "", "");
        let traditional = of("", "TW", "");
        let simplified = of("", "CN", "");

        let has_hant = list.iter().any(|tag| tag.script == "Hant");
        if has_hant && !list.contains(&traditional) {
            if let Some(index) = list.iter().position(|tag| *tag == chinese) {
                list.insert(index, traditional);
            }
        }

        if !list.contains(&simplified) {
            if let Some(index) = list.iter().position(|tag| *tag == chinese) {
                let index = if list.iter().any(|tag| tag.script == "Hans") {
                    index
                } else {
                    index + 1
                };
                list.insert(index, simplified);
            }
        }
    }
}

/// Returns the text with its first letter written as a capital.
fn uppercase_first(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

/// Returns the languages that are written with three letters, by the codes of
/// two letters they are known by.
fn language_aliases() -> &'static BTreeMap<String, String> {
    static ALIASES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    ALIASES.get_or_init(|| read_pairs(include_str!("../../resources/lang/language_aliases.csv")))
}

/// Returns the languages that hold more than one language, by the languages
/// they hold.
fn sub_languages() -> &'static BTreeMap<String, String> {
    static LANGUAGES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    LANGUAGES.get_or_init(|| read_pairs(include_str!("../../resources/lang/sublanguages.csv")))
}

/// Returns the ways of writing the languages are written in, by the languages
/// that are written in them.
fn default_scripts() -> &'static BTreeMap<String, String> {
    static SCRIPTS: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    SCRIPTS.get_or_init(|| read_pairs(include_str!("../../resources/lang/default_script.csv")))
}

/// Reads the pairs of a file that holds a name and the names that belong to
/// it.
///
/// The first name of a line is the one the others belong to, so a line of
/// `parent,child,child` gives the pairs of the children to the parent. A line
/// that begins with `#` and a line that holds nothing are left out.
fn read_pairs(text: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let mut items = line.split(',');
        let Some(parent) = items.next() else {
            continue;
        };

        for item in items {
            result.insert(item.to_string(), parent.to_string());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_read_by_the_parts_of_it() {
        let tag = LanguageTag::parse("zh-Hans-CN");
        assert_eq!(tag.language, "zh");
        assert_eq!(tag.script, "Hans");
        assert_eq!(tag.region, "CN");
        assert!(tag.variant.is_empty());

        // A language that is written with an underscore is read the way the
        // launcher of Java writes it.
        let tag = LanguageTag::parse("zh_TW");
        assert_eq!(tag.language, "zh");
        assert_eq!(tag.region, "TW");

        let tag = LanguageTag::parse("sl-rozaj-biske");
        assert_eq!(tag.language, "sl");
        assert_eq!(tag.region, "");
        assert_eq!(tag.variant, "rozaj-biske");

        // A tag the launcher does not know is kept as it is, so the name
        // the player gave is not lost.
        let tag = LanguageTag::parse("xyzzy");
        assert_eq!(tag.language, "xyzzy");
    }

    #[test]
    fn the_key_of_a_language_is_written_the_way_the_files_are_named() {
        assert_eq!(LanguageTag::parse("en").language_key(), "en");
        assert_eq!(
            LanguageTag::parse("zh-Hans-CN").language_key(),
            "zh-Hans-CN"
        );
        // A tag of no language is the file of every language.
        assert_eq!(LanguageTag::root().language_key(), DEFAULT_LANGUAGE_KEY);
    }

    #[test]
    fn a_language_of_three_letters_is_read_as_the_language_of_two_letters() {
        assert_eq!(normalize_language("zho"), "zh");
        assert_eq!(normalize_language("cmn"), "cmn");
        // An empty language is read as English.
        assert_eq!(normalize_language(""), "en");
    }

    #[test]
    fn a_language_is_read_as_the_language_that_holds_it() {
        assert_eq!(parent_language("cmn"), Some("zh"));
        assert_eq!(parent_language("id"), Some("ms"));
        assert_eq!(root_language("yue"), "zh");
        // A language that is written with three letters and holds more than
        // one language is read as the language that holds it.
        assert_eq!(root_language("zho"), "zh");
        // The way the text of a language is written is no language of its own.
        assert_eq!(LanguageTag::parse("yue-Hant").language, "yue");
        assert_eq!(
            root_language(&LanguageTag::parse("yue-Hant").language),
            "zh"
        );
        assert_eq!(root_language("fr"), "fr");
    }

    #[test]
    fn a_language_that_holds_more_than_one_is_read_as_the_one_spoken_the_most() {
        assert_eq!(preferred_language("zho"), "cmn");
        assert_eq!(preferred_language("zh"), "cmn");
        assert_eq!(preferred_language("fr"), "fr");
    }

    #[test]
    fn the_way_of_writing_a_language_is_read() {
        assert_eq!(LanguageTag::parse("en").script(), "Latn");
        assert_eq!(LanguageTag::parse("ja").script(), "Jpan");
        // A tag that names the way of writing keeps it.
        assert_eq!(LanguageTag::parse("zh-Hans").script(), "Hans");
        // A kind of place tells the way of writing by itself.
        assert_eq!(LanguageTag::parse("lzh").script(), "Hant");
        // A Chinese tag is read with the characters of the place it names.
        assert_eq!(LanguageTag::parse("zh-TW").script(), "Hant");
        assert_eq!(LanguageTag::parse("zh-CN").script(), "Hans");
        assert_eq!(LanguageTag::parse("zh").script(), "Hans");
        // The place of the upside down text is written upside down.
        assert_eq!(LanguageTag::parse("en-UD").script(), "Qabs");
        // A language the launcher knows no way of writing for is read as it
        // is.
        assert_eq!(LanguageTag::parse("xyz").script(), "");
    }

    #[test]
    fn the_way_the_text_of_a_language_is_written_in_is_read() {
        assert_eq!(
            LanguageTag::parse("en").text_direction(),
            TextDirection::LeftToRight
        );
        assert_eq!(
            LanguageTag::parse("ar").text_direction(),
            TextDirection::RightToLeft
        );
        assert_eq!(
            LanguageTag::parse("he").text_direction(),
            TextDirection::RightToLeft
        );
        assert_eq!(
            LanguageTag::parse("en-UD").text_direction(),
            TextDirection::RightToLeft
        );
    }

    #[test]
    fn the_languages_a_tag_is_read_as_are_ordered() {
        // A tag of the whole language is read before the tags of its parts.
        let candidates = LanguageTag::parse("zh-Hans-CN").candidate_locales();
        let keys: Vec<String> = candidates.iter().map(LanguageTag::language_key).collect();

        assert_eq!(
            keys,
            vec![
                "cmn-Hans-CN",
                "cmn-Hans",
                "cmn-CN",
                "cmn",
                "zh-Hans-CN",
                "zh-Hans",
                "zh-CN",
                "zh",
                "default",
            ]
        );

        // A language that holds more than one is read by the language that
        // holds it as well.
        let candidates = LanguageTag::parse("ja").candidate_locales();
        let keys: Vec<String> = candidates.iter().map(LanguageTag::language_key).collect();
        assert_eq!(keys, vec!["ja-Jpan", "ja", "default"]);

        // The Chinese characters of the other place are read as well.
        let candidates = LanguageTag::parse("zh-TW").candidate_locales();
        let keys: Vec<String> = candidates.iter().map(LanguageTag::language_key).collect();
        assert!(keys.contains(&"zh-CN".to_string()));
        assert!(keys.contains(&"zh-Hant-TW".to_string()));
    }

    #[test]
    fn the_text_of_the_language_the_launcher_looks_for_first_is_read() {
        let entries = vec![
            ("zh-Hans".to_string(), "simplified"),
            ("en".to_string(), "english"),
            (DEFAULT_LANGUAGE_KEY.to_string(), "every language"),
        ];

        let candidates = LanguageTag::parse("zh-Hans-CN").candidate_locales();
        assert_eq!(
            get_by_candidate_locales(&entries, &candidates),
            Some(&"simplified")
        );

        // A language the launcher has no text of is read as the text of every
        // language, which is the last key the launcher looks for.
        let candidates = LanguageTag::parse("de").candidate_locales();
        assert_eq!(
            get_by_candidate_locales(&entries, &candidates),
            Some(&"every language")
        );

        // A list of no language at all looks for no key.
        assert_eq!(get_by_candidate_locales(&entries, &[]), None);
    }

    #[test]
    fn the_files_of_a_directory_that_are_written_in_a_language_are_found() {
        let base = std::env::temp_dir().join(format!(
            "hmcl-rs-locale-files-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("the directory should be made");

        std::fs::write(base.join("icon.png"), b"default").expect("write");
        std::fs::write(base.join("icon_zh_Hans.png"), b"simplified").expect("write");
        std::fs::write(base.join("icon_ja.png"), b"japanese").expect("write");
        std::fs::write(base.join("icon.txt"), b"other").expect("write");
        std::fs::write(base.join("other_zh_Hans.png"), b"other name").expect("write");
        std::fs::create_dir_all(base.join("icon_ru.png")).expect("dir");

        let files = find_all_localized_files(&base, "icon", "png");
        let keys: Vec<&str> = files.iter().map(|(key, _)| key.as_str()).collect();

        assert!(keys.contains(&DEFAULT_LANGUAGE_KEY));
        assert!(keys.contains(&"zh-Hans"));
        assert!(keys.contains(&"ja"));
        // The files of the other kinds, the other names and the directories
        // are left out.
        assert!(!keys.contains(&"txt"));
        assert_eq!(files.len(), 3);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_files_of_a_directory_are_found_by_the_kind_of_the_file() {
        let base = std::env::temp_dir().join(format!(
            "hmcl-rs-locale-kinds-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("the directory should be made");

        std::fs::write(base.join("texture.png"), b"default").expect("write");
        std::fs::write(base.join("texture_zh_Hans.png"), b"simplified").expect("write");
        std::fs::write(base.join("texture_zh_Hans.json"), b"meta").expect("write");

        let files = find_all_localized_files_by_extension(&base, "texture", &["png", "json"]);
        assert_eq!(files.len(), 2);

        let (key, default) = &files[0];
        assert_eq!(key, DEFAULT_LANGUAGE_KEY);
        assert_eq!(default.len(), 1);
        assert_eq!(default[0].0, "png");

        // A language holds the files of every kind it names.
        let (key, chinese) = files
            .iter()
            .find(|(key, _)| key == "zh-Hans")
            .expect("the Chinese files should be found");
        assert_eq!(key, "zh-Hans");
        assert_eq!(chinese.len(), 2);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_directory_that_cannot_be_read_holds_no_file() {
        let base = std::env::temp_dir().join(format!(
            "hmcl-rs-locale-missing-{}-{}",
            std::process::id(),
            line!()
        ));

        assert!(find_all_localized_files(&base, "icon", "png").is_empty());
        assert!(find_all_localized_files_by_extension(&base, "icon", &["png"]).is_empty());
    }
}
