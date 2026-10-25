//! The reader of the property files of the launcher.
//!
//! The translations of the launcher are stored in the property files of Java.
//! The reader understands the parts of that format the launcher uses: the
//! comments, the escaped characters, the lines that continue on the next
//! line and the separators between the keys and the values.

use std::collections::BTreeMap;

/// The translations of one language.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bundle {
    /// The translations, by their key.
    entries: BTreeMap<String, String>,
}

impl Bundle {
    /// Reads the translations of a property file.
    pub fn parse(content: &str) -> Self {
        let mut entries = BTreeMap::new();

        for line in logical_lines(content) {
            let line = line.trim_start();
            if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
                continue;
            }

            let (key, value) = match split_entry(line) {
                Some(entry) => entry,
                None => continue,
            };

            if key.is_empty() {
                continue;
            }

            entries.insert(key, value);
        }

        Self { entries }
    }

    /// Returns the translation of a key, if the language has one.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Returns whether the language has a translation of the key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Returns the number of the translations of the language.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the language has no translation at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Adds the translations of another bundle.
    ///
    /// The translations of the bundle that is added win, so the texts of the
    /// pages of the C# version can be put over the texts of the Java
    /// version.
    pub fn merge(&mut self, other: &Bundle) {
        for (key, value) in &other.entries {
            self.entries.insert(key.clone(), value.clone());
        }
    }

    /// Returns the translations of the language, by their key.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}

/// Returns the lines of the file, with the continued lines joined.
///
/// A line that ends with an odd number of backslashes continues on the next
/// line; the white space at the beginning of the next line is dropped.
fn logical_lines(content: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        current.push_str(line.trim_start());

        if ends_with_continuation(&current) {
            // The backslash that continues the line is dropped, but the
            // backslashes that are escaped stay.
            let length = current.len();
            current.truncate(length - 1);
            continue;
        }

        lines.push(std::mem::take(&mut current));
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

/// Returns whether the line ends with a backslash that continues it.
fn ends_with_continuation(line: &str) -> bool {
    let mut backslashes = 0;
    for character in line.chars().rev() {
        if character == '\\' {
            backslashes += 1;
        } else {
            break;
        }
    }
    backslashes % 2 == 1
}

/// Splits a line into its key and its value.
///
/// The key and the value are separated by an equals sign, a colon or white
/// space; the white space around the separator is dropped.
fn split_entry(line: &str) -> Option<(String, String)> {
    let characters: Vec<char> = line.chars().collect();
    let mut index = 0;
    let mut key = String::new();
    let mut separator_consumed = false;

    while index < characters.len() {
        let character = characters[index];
        match character {
            '\\' => {
                // The escape is kept, so the key is unescaped as a whole.
                key.push(character);
                index += 1;
                if index < characters.len() {
                    key.push(characters[index]);
                    index += 1;
                }
            }
            '=' | ':' => {
                index += 1;
                separator_consumed = true;
                break;
            }
            ' ' | '\t' | '\u{c}' => {
                // The white space separates the key from the value.
                break;
            }
            _ => {
                key.push(character);
                index += 1;
            }
        }
    }

    if !separator_consumed {
        // The white space between the key and the value is dropped.
        while index < characters.len() && matches!(characters[index], ' ' | '\t' | '\u{c}') {
            index += 1;
        }

        if index < characters.len() && (characters[index] == '=' || characters[index] == ':') {
            index += 1;

            while index < characters.len() && matches!(characters[index], ' ' | '\t' | '\u{c}') {
                index += 1;
            }
        }
    }

    let value: String = characters[index.min(characters.len())..].iter().collect();
    Some((unescape(key.trim()), unescape(value.trim())))
}

/// Replaces the escaped characters of a value with the characters they stand
/// for.
fn unescape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }

        match characters.next() {
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('f') => result.push('\u{c}'),
            Some('u') => {
                let mut code = 0u32;
                for _ in 0..4 {
                    match characters.next().and_then(|digit| digit.to_digit(16)) {
                        Some(digit) => code = code * 16 + digit,
                        None => break,
                    }
                }
                if let Some(character) = char::from_u32(code) {
                    result.push(character);
                }
            }
            Some(other) => result.push(other),
            None => result.push('\\'),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entries_are_read() {
        let bundle = Bundle::parse(
            "# a comment\n\
             ! another comment\n\
             about=About\n\
             launch=Launch\n",
        );

        assert_eq!(bundle.get("about"), Some("About"));
        assert_eq!(bundle.get("launch"), Some("Launch"));
        assert!(!bundle.contains_key("about.copyright"));
        assert_eq!(bundle.len(), 2);
    }

    #[test]
    fn the_separators_are_understood() {
        let bundle = Bundle::parse("a=1\nb:2\nc 3\nd = 4\n");
        assert_eq!(bundle.get("a"), Some("1"));
        assert_eq!(bundle.get("b"), Some("2"));
        assert_eq!(bundle.get("c"), Some("3"));
        assert_eq!(bundle.get("d"), Some("4"));
    }

    #[test]
    fn the_escaped_characters_are_replaced() {
        let bundle = Bundle::parse("a=line\\nbreak\nb=tab\\there\nc=\\u4e2d\\u6587\nd=a\\:b\n");
        assert_eq!(bundle.get("a"), Some("line\nbreak"));
        assert_eq!(bundle.get("b"), Some("tab\there"));
        assert_eq!(bundle.get("c"), Some("中文"));
        assert_eq!(bundle.get("d"), Some("a:b"));
    }

    #[test]
    fn the_continued_lines_are_joined() {
        let bundle = Bundle::parse("a=first \\\n    second\nb=other\n");
        assert_eq!(bundle.get("a"), Some("first second"));
        assert_eq!(bundle.get("b"), Some("other"));
    }

    #[test]
    fn the_empty_lines_are_skipped() {
        let bundle = Bundle::parse("\n\na=value\n\n");
        assert_eq!(bundle.len(), 1);
        assert_eq!(bundle.get("a"), Some("value"));
    }

    #[test]
    fn the_last_entry_of_a_key_wins() {
        let bundle = Bundle::parse("a=first\na=second\n");
        assert_eq!(bundle.get("a"), Some("second"));
    }
}
