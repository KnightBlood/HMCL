//! Comparison of version numbers.
//!
//! The rules follow the version order specification of Apache Maven, so that
//! `1.0` is equal to `1`, `1.1` is greater than `1` and `1.1` is greater than
//! `1-sp`. The launcher uses the same rules for the early access versions of
//! Minecraft, whose numbers are compared with the numbers of the releases.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

/// The characters that separate the parts of a version number.
const SEPARATORS: &str = "!\"#$%&'()*+,-/:;<=>?@[\\]^_`{|}~";

/// The qualifiers that mark a version as an early access version.
const PRE_QUALIFIERS: [&str; 5] = ["alpha", "beta", "pre", "rc", "experimental"];

/// The number of digits that are stored as an integer; longer numbers are
/// compared as decimal strings.
const MAX_LONG_ITEM_LENGTH: usize = 18;

/// A part of a version number.
#[derive(Debug, Clone)]
enum Item {
    /// An integer that fits into a 64 bit signed integer.
    Long(i64),
    /// An integer that is too large for [`Item::Long`], kept as digits.
    BigInt(String),
    /// A qualifier, such as `pre` or `rc`.
    Text {
        /// The text of the qualifier.
        value: String,
        /// Whether the qualifier marks an early access version.
        pre: bool,
    },
    /// A nested list of items, introduced by a separator.
    List {
        /// The character that introduced the list.
        separator: Option<char>,
        /// The items of the list.
        items: Vec<Item>,
    },
}

impl Item {
    /// Returns whether the item does not contribute to the version.
    fn is_null(&self) -> bool {
        match self {
            Self::Long(value) => *value == 0,
            Self::BigInt(_) => false,
            Self::Text { value, .. } => value.is_empty(),
            Self::List { items, .. } => items.is_empty(),
        }
    }

    /// Compares the item with another item.
    ///
    /// A missing item stands for the end of a list, which is how a shorter
    /// version is compared with a longer one.
    fn compare(&self, other: Option<&Item>) -> Ordering {
        match other {
            None => match self {
                Self::Long(value) => {
                    if *value == 0 {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                }
                Self::BigInt(_) => Ordering::Greater,
                Self::Text { pre, .. } => {
                    if *pre {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    }
                }
                Self::List { items, .. } => match items.first() {
                    None => Ordering::Equal,
                    Some(first) => first.compare(None),
                },
            },
            Some(other) => match (self, other) {
                (Self::Long(left), Self::Long(right)) => left.cmp(right),
                (Self::Long(_), Self::BigInt(_)) => Ordering::Less,
                (Self::Long(_), Self::Text { .. }) => Ordering::Greater,
                (Self::Long(_), Self::List { .. }) => Ordering::Greater,
                (Self::BigInt(left), Self::BigInt(right)) => compare_big(left, right),
                (Self::BigInt(_), Self::Long(_)) => Ordering::Greater,
                (Self::BigInt(_), Self::Text { .. }) => Ordering::Greater,
                (Self::BigInt(_), Self::List { .. }) => Ordering::Greater,
                (Self::Text { value: left, .. }, Self::Text { value: right, .. }) => {
                    left.cmp(right)
                }
                (Self::Text { .. }, Self::Long(_)) | (Self::Text { .. }, Self::BigInt(_)) => {
                    Ordering::Less
                }
                (Self::Text { .. }, Self::List { .. }) => Ordering::Less,
                (Self::List { .. }, Self::Long(_)) | (Self::List { .. }, Self::BigInt(_)) => {
                    Ordering::Less
                }
                (Self::List { .. }, Self::Text { .. }) => Ordering::Greater,
                (Self::List { items: left, .. }, Self::List { items: right, .. }) => {
                    compare_lists(left, right)
                }
            },
        }
    }

    /// Appends the canonical form of the item to the buffer.
    fn append_to(&self, buffer: &mut String) {
        match self {
            Self::Long(value) => buffer.push_str(&value.to_string()),
            Self::BigInt(value) => buffer.push_str(value),
            Self::Text { value, .. } => buffer.push_str(value),
            Self::List { separator, items } => {
                if let Some(separator) = separator {
                    buffer.push(*separator);
                }

                append_list_to(items, buffer);
            }
        }
    }
}

/// Appends the canonical form of a list of items to the buffer.
///
/// The items of a list are separated by a dot, but the nested lists are
/// separated by the character that introduced them.
fn append_list_to(items: &[Item], buffer: &mut String) {
    let initial_length = buffer.len();
    for item in items {
        if buffer.len() > initial_length && !matches!(item, Item::List { .. }) {
            buffer.push('.');
        }
        item.append_to(buffer);
    }
}

/// Compares two lists of items item by item.
fn compare_lists(left: &[Item], right: &[Item]) -> Ordering {
    let mut left = left.iter();
    let mut right = right.iter();

    loop {
        let current = left.next();
        let other = right.next();

        if current.is_none() && other.is_none() {
            return Ordering::Equal;
        }

        let result = match current {
            None => match other {
                None => Ordering::Equal,
                Some(other) => other.compare(None).reverse(),
            },
            Some(current) => current.compare(other),
        };

        if result != Ordering::Equal {
            return result;
        }
    }
}

/// Compares two decimal numbers that are too large for an integer.
fn compare_big(left: &str, right: &str) -> Ordering {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');

    match left.len().cmp(&right.len()) {
        Ordering::Equal => left.cmp(right),
        other => other,
    }
}

/// Returns whether the text starts with one of the early access qualifiers.
fn is_pre_qualifier(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    PRE_QUALIFIERS
        .iter()
        .any(|qualifier| text.starts_with(qualifier))
}

/// Parses a part of a version number into an item.
fn parse_item(text: &str) -> Item {
    let mut number_length = 0usize;
    let mut leading_zero = true;

    for character in text.chars() {
        if character.is_ascii_digit() {
            if character != '0' {
                leading_zero = false;
            }
            if !leading_zero {
                number_length += 1;
            }
        } else {
            return Item::Text {
                value: text.to_string(),
                pre: is_pre_qualifier(text),
            };
        }
    }

    if number_length == 0 {
        Item::Long(0)
    } else if number_length <= MAX_LONG_ITEM_LENGTH {
        // The number has at most eighteen significant digits, so it always
        // fits into a 64 bit signed integer.
        Item::Long(text.parse::<i64>().unwrap_or(0))
    } else {
        // The leading zeros are dropped, like the Java BigInteger does.
        Item::BigInt(text.trim_start_matches('0').to_string())
    }
}

/// Returns whether the character separates the parts of a version number.
fn is_separator(character: char) -> bool {
    SEPARATORS.contains(character)
}

/// A list of items that is being parsed.
#[derive(Debug)]
struct ListFrame {
    /// The character that introduced the list.
    separator: Option<char>,
    /// The items of the list.
    items: Vec<Item>,
    /// The index of the list in the items of the parent list.
    parent_index: usize,
}

/// A version number that can be compared with another one.
///
/// The number is split into its parts once, when it is created; the parts
/// are then compared without parsing the text again.
#[derive(Debug, Clone)]
pub struct VersionNumber {
    /// The text the number was created from.
    value: String,
    /// The parts of the number.
    items: Vec<Item>,
    /// The canonical form of the number, used for equality.
    canonical: String,
}

impl VersionNumber {
    /// Creates a version number from its text.
    ///
    /// Every text is a valid version number; the parts that are not numbers
    /// are kept as qualifiers.
    pub fn parse(version: &str) -> Self {
        let mut stack: Vec<ListFrame> = vec![ListFrame {
            separator: None,
            items: Vec::new(),
            parent_index: 0,
        }];

        let mut is_digit = false;
        let mut start_index = 0usize;

        for (index, character) in version.char_indices() {
            if character == '.' {
                push_item(&mut stack, &version[start_index..index]);
                start_index = index + character.len_utf8();
            } else if is_separator(character) {
                push_item(&mut stack, &version[start_index..index]);
                start_index = index + character.len_utf8();

                push_list(&mut stack, Some(character));
            } else if character.is_ascii_digit() {
                if !is_digit && index > start_index {
                    push_item(&mut stack, &version[start_index..index]);
                    start_index = index;

                    push_list(&mut stack, None);
                }
                is_digit = true;
            } else {
                if is_digit && index > start_index {
                    push_item(&mut stack, &version[start_index..index]);
                    start_index = index;

                    push_list(&mut stack, None);
                }
                is_digit = false;
            }
        }

        if version.len() > start_index {
            push_item(&mut stack, &version[start_index..]);
        }

        let mut items = Vec::new();
        while let Some(frame) = stack.pop() {
            let mut frame = frame;
            normalize(&mut frame.items);

            match stack.last_mut() {
                Some(parent) => {
                    parent.items[frame.parent_index] = Item::List {
                        separator: frame.separator,
                        items: frame.items,
                    };
                }
                None => items = frame.items,
            }
        }

        let mut canonical = String::new();
        append_list_to(&items, &mut canonical);

        Self {
            value: version.to_string(),
            items,
            canonical,
        }
    }

    /// Returns the text the number was created from.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the canonical form of the number.
    ///
    /// Two numbers with the same canonical form are equal, so `1.0` and `1`
    /// are the same version.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// Returns whether the text is a version number that consists of numbers
    /// only, such as `1.20.1`.
    pub fn is_int_version_number(version: &str) -> bool {
        if version.is_empty() {
            return false;
        }

        let mut index = 0usize;
        loop {
            let dot_index = version[index..].find('.').map(|offset| index + offset);
            let (end_index, has_more) = match dot_index {
                Some(dot_index) => {
                    if dot_index == index || dot_index == version.len() - 1 {
                        return false;
                    }
                    (dot_index, true)
                }
                None => (version.len(), false),
            };

            if end_index - index > 9 {
                // Numbers that are larger than 10^10 cannot be stored as an
                // integer.
                return false;
            }

            for character in version[index..end_index].chars() {
                if !character.is_ascii_digit() {
                    return false;
                }
            }

            if !has_more {
                break;
            }
            index = end_index + 1;
        }

        true
    }
}

/// Adds a part to the list that is being parsed.
fn push_item(stack: &mut [ListFrame], text: &str) {
    let item = if text.is_empty() {
        Item::Long(0)
    } else {
        parse_item(text)
    };

    if let Some(frame) = stack.last_mut() {
        frame.items.push(item);
    }
}

/// Starts a nested list in the list that is being parsed.
fn push_list(stack: &mut Vec<ListFrame>, separator: Option<char>) {
    let parent_index = match stack.last_mut() {
        Some(parent) => {
            parent.items.push(Item::List {
                separator,
                items: Vec::new(),
            });
            parent.items.len() - 1
        }
        None => 0,
    };

    stack.push(ListFrame {
        separator,
        items: Vec::new(),
        parent_index,
    });
}

/// Removes the parts that do not contribute to the version from the end of
/// the list.
fn normalize(items: &mut Vec<Item>) {
    let mut index = items.len();
    while index > 0 {
        index -= 1;
        match &items[index] {
            item if item.is_null() => {
                items.remove(index);
            }
            Item::List { .. } => continue,
            _ => break,
        }
    }
}

impl fmt::Display for VersionNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)
    }
}

impl PartialEq for VersionNumber {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
    }
}

impl Eq for VersionNumber {}

impl Hash for VersionNumber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical.hash(state);
    }
}

impl PartialOrd for VersionNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_lists(&self.items, &other.items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compares two version numbers.
    fn compare(left: &str, right: &str) -> Ordering {
        VersionNumber::parse(left).cmp(&VersionNumber::parse(right))
    }

    /// Returns whether the first version number is smaller than the second.
    fn assert_less_than(left: &str, right: &str) {
        assert_eq!(
            compare(left, right),
            Ordering::Less,
            "{left} should be less than {right}"
        );
        assert_eq!(
            compare(right, left),
            Ordering::Greater,
            "{right} should be greater than {left}"
        );
    }

    /// Returns the canonical form of a version number.
    fn normalize(version: &str) -> String {
        VersionNumber::parse(version).canonical().to_string()
    }

    #[test]
    fn the_canonical_form_drops_the_parts_that_do_not_matter() {
        assert_eq!(normalize("3.2.0.0"), "3.2");
        assert_eq!(normalize("3.2.0.0-5"), "3.2-5");
        assert_eq!(normalize("3.2.0.0-0"), "3.2");
        assert_eq!(normalize("3.2--------"), "3.2");
        assert_eq!(normalize("3.0002"), "3.2");
        assert_eq!(
            normalize("1.7.2$%%^@&snapshot-3.1.1"),
            "1.7.2$%%^@&snapshot-3.1.1"
        );
        assert_eq!(
            normalize("1.99999999999999999999"),
            "1.99999999999999999999"
        );
        assert_eq!(
            normalize("1.0099999999999999999999"),
            "1.99999999999999999999"
        );
        assert_eq!(
            normalize("1.99999999999999999999.0"),
            "1.99999999999999999999"
        );
        assert_eq!(
            normalize("1.99999999999999999999--------"),
            "1.99999999999999999999"
        );
    }

    #[test]
    fn the_integer_version_numbers_are_detected() {
        for version in [
            "",
            " ",
            ".",
            "1.",
            ".1",
            ".1.",
            "1..8",
            "1.8.",
            ".1.8",
            "1.7.10forge1614_FTBInfinity",
            "3.2-5",
            "1.9999999999",
        ] {
            assert!(
                !VersionNumber::is_int_version_number(version),
                "{version} should not be an integer version number"
            );
        }

        for version in [
            "0",
            "1",
            "0.1",
            "0.1.0",
            "1.8",
            "1.12.2",
            "1.13.1",
            "1.999999999",
            "999999999.0",
        ] {
            assert!(
                VersionNumber::is_int_version_number(version),
                "{version} should be an integer version number"
            );
        }
    }

    #[test]
    fn the_version_numbers_are_compared() {
        assert_less_than("1.7.10forge1614_FTBInfinity", "1.12.2");
        assert_less_than("1.8.0_51", "1.8.0.51");
        assert_less_than("1.8.0_77", "1.8.0_151");
        assert_less_than("1.6.0_22", "1.8.0_11");
        assert_less_than("1.7.0_22", "1.7.99");
        assert_less_than("1.12.2-14.23.4.2739", "1.12.2-14.23.5.2760");
        assert_less_than("1.9", "1.99999999999999999999");
        assert_less_than("1.99999999999999999999", "1.199999999999999999999");
        assert_less_than("1.99999999999999999999", "2");
        assert_less_than("1.99999999999999999999", "2.0");
        assert_less_than("1.0", "1.0-zzz");
        assert_less_than("1.0-beta.1", "1.0");
        assert_less_than("1.0-alpha.1", "1.0-beta.1");
        assert_less_than("3.6.15", "3.6.15.289");
        assert_less_than("3.6.15.289", "3.6.16");
    }

    #[test]
    fn the_equal_version_numbers_are_equal() {
        assert_eq!(VersionNumber::parse("1"), VersionNumber::parse("1.0"));
        assert_eq!(VersionNumber::parse("1.0.0"), VersionNumber::parse("1"));
        assert_eq!(VersionNumber::parse("1.0.0"), VersionNumber::parse("1"));
    }

    #[test]
    fn the_version_numbers_are_sorted() {
        let mut versions = vec![
            "0",
            "0.10.0",
            "1.6.4",
            "1.6.4-Forge9.11.1.1345",
            "1.7.10",
            "1.7.10Agrarian_Skies_2",
            "1.7.10-F1614-L",
            "1.7.10-FL1614_04",
            "1.7.10-Forge10.13.4.1614-1.7.10",
            "1.7.10-Forge1614",
            "1.7.10Forge1614_FTBInfinity-2.6.0",
            "1.7.10Forge1614_FTBInfinity-3.0.1",
            "1.7.10-Forge1614.1",
            "1.7.10forge1614_ATlauncher",
            "1.7.10forge1614_FTBInfinity",
            "1.7.10forge1614_FTBInfinity_server",
            "1.7.10forge1614test",
            "1.7.10-1614",
            "1.7.10-1614-test",
            "1.8",
            "1.8-forge1577",
            "1.8.9",
            "1.8.9-forge1902",
            "1.9",
            "1.10-alpha.2",
            "1.10-beta.1",
            "1.10-beta.2",
            "1.10",
            "1.10.2",
            "1.10.2-AOE",
            "1.10.2-AOE-1.1.5",
            "1.10.2-All the Mods",
            "1.10.2-FTB_Beyond",
            "1.10.2-LiteLoader1.10.2",
            "1.10.2-forge2511-AOE-1.1.2",
            "1.10.2-forge2511-ATM-E",
            "1.10.2-forge2511-Age_of_Progression",
            "1.10.2-forge2511_Farming_Valley",
            "1.10.2-forge2511_bxztest",
            "1.10.2-forge2511-simple_life_2",
            "1.12.2",
            "1.12.2_Modern_Skyblock-3.4.2",
            "1.13.1",
            "1.99999999999999999999",
            "2",
            "2.0",
            "2.1",
        ];

        versions.sort_by(|left, right| {
            VersionNumber::parse(left)
                .cmp(&VersionNumber::parse(right))
                .then_with(|| left.cmp(right))
        });

        let expected = vec![
            "0",
            "0.10.0",
            "1.6.4",
            "1.6.4-Forge9.11.1.1345",
            "1.7.10",
            "1.7.10Agrarian_Skies_2",
            "1.7.10-F1614-L",
            "1.7.10-FL1614_04",
            "1.7.10-Forge10.13.4.1614-1.7.10",
            "1.7.10-Forge1614",
            "1.7.10Forge1614_FTBInfinity-2.6.0",
            "1.7.10Forge1614_FTBInfinity-3.0.1",
            "1.7.10-Forge1614.1",
            "1.7.10forge1614_ATlauncher",
            "1.7.10forge1614_FTBInfinity",
            "1.7.10forge1614_FTBInfinity_server",
            "1.7.10forge1614test",
            "1.7.10-1614",
            "1.7.10-1614-test",
            "1.8",
            "1.8-forge1577",
            "1.8.9",
            "1.8.9-forge1902",
            "1.9",
            "1.10-alpha.2",
            "1.10-beta.1",
            "1.10-beta.2",
            "1.10",
            "1.10.2",
            "1.10.2-AOE",
            "1.10.2-AOE-1.1.5",
            "1.10.2-All the Mods",
            "1.10.2-FTB_Beyond",
            "1.10.2-LiteLoader1.10.2",
            "1.10.2-forge2511-AOE-1.1.2",
            "1.10.2-forge2511-ATM-E",
            "1.10.2-forge2511-Age_of_Progression",
            "1.10.2-forge2511_Farming_Valley",
            "1.10.2-forge2511_bxztest",
            "1.10.2-forge2511-simple_life_2",
            "1.12.2",
            "1.12.2_Modern_Skyblock-3.4.2",
            "1.13.1",
            "1.99999999999999999999",
            "2",
            "2.0",
            "2.1",
        ];

        assert_eq!(versions, expected);
    }

    #[test]
    fn the_value_is_kept() {
        let version = VersionNumber::parse("1.20.1");
        assert_eq!(version.value(), "1.20.1");
        assert_eq!(version.to_string(), "1.20.1");
    }
}
