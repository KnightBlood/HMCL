//! The helpers of the texts.
//!
//! The launcher shows the texts of the servers and of the add-ons, which
//! are written in many ways, so the launcher has to look at them in many
//! ways as well.

/// Returns whether the text is empty or holds only white space.
pub fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// Returns whether the text holds something else than white space.
pub fn is_not_blank(text: &str) -> bool {
    !is_blank(text)
}

/// Returns the text with the first character in upper case.
pub fn capitalize_first(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
    }
}

/// Returns the text with the first character of every word in upper case.
///
/// The words are separated by a space, a dash or an underscore.
pub fn capitalize_words(text: &str) -> String {
    text.replace(['-', '_'], " ")
        .split(' ')
        .map(capitalize_first)
        .collect::<Vec<String>>()
        .join(" ")
}

/// Returns whether one of the patterns holds one of the targets.
///
/// The patterns and the targets are compared without the case, like in the
/// Java launcher.
pub fn contains_one<'a>(
    patterns: impl IntoIterator<Item = &'a str>,
    targets: impl IntoIterator<Item = &'a str>,
) -> bool {
    let targets: Vec<String> = targets.into_iter().map(|t| t.to_lowercase()).collect();

    patterns.into_iter().any(|pattern| {
        let pattern = pattern.to_lowercase();
        targets
            .iter()
            .any(|target| pattern.contains(target.as_str()))
    })
}

/// Returns whether the text ends with one of the suffixes.
pub fn ends_with_any<'a>(text: &str, suffixes: impl IntoIterator<Item = &'a str>) -> bool {
    suffixes.into_iter().any(|suffix| text.ends_with(suffix))
}

/// Returns whether the text starts with one of the prefixes.
pub fn starts_with_any<'a>(text: &str, prefixes: impl IntoIterator<Item = &'a str>) -> bool {
    prefixes.into_iter().any(|prefix| text.starts_with(prefix))
}

/// Returns whether the text starts with the prefix, without the case.
pub fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix)
}

/// Returns the part of the text that follows the delimiter.
///
/// The text is empty when the delimiter is not in it.
pub fn substring_after(text: &str, delimiter: char) -> &str {
    match text.find(delimiter) {
        Some(index) => &text[index + delimiter.len_utf8()..],
        None => "",
    }
}

/// Returns the part of the text that follows the last delimiter.
pub fn substring_after_last(text: &str, delimiter: char) -> &str {
    match text.rfind(delimiter) {
        Some(index) => &text[index + delimiter.len_utf8()..],
        None => "",
    }
}

/// Returns the part of the text that comes before the delimiter.
///
/// The text is returned as it is when the delimiter is not in it.
pub fn substring_before(text: &str, delimiter: char) -> &str {
    match text.find(delimiter) {
        Some(index) => &text[..index],
        None => text,
    }
}

/// Returns the part of the text that comes before the last delimiter.
pub fn substring_before_last(text: &str, delimiter: char) -> &str {
    match text.rfind(delimiter) {
        Some(index) => &text[..index],
        None => text,
    }
}

/// Returns the text without the prefix.
pub fn remove_prefix<'a>(text: &'a str, prefix: &str) -> &'a str {
    text.strip_prefix(prefix).unwrap_or(text)
}

/// Returns the text without the suffix.
pub fn remove_suffix<'a>(text: &'a str, suffix: &str) -> &'a str {
    text.strip_suffix(suffix).unwrap_or(text)
}

/// Returns the text with the white space at the beginning and at the end
/// removed and the runs of white space replaced by a single space.
pub fn normalize_whitespaces(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut space = false;

    for character in text.chars() {
        if character.is_whitespace() {
            space = true;
            continue;
        }

        if space && !result.is_empty() {
            result.push(' ');
        }
        space = false;
        result.push(character);
    }

    result
}

/// Returns whether the text holds only the characters of the ASCII.
pub fn is_ascii(text: &str) -> bool {
    text.is_ascii()
}

/// Returns whether the text holds only the letters and the numbers.
pub fn is_alphabetic_or_number(text: &str) -> bool {
    text.chars()
        .all(|character| character.is_alphabetic() || character.is_numeric())
}

/// Returns whether the text holds a number.
pub fn contains_digit(text: &str) -> bool {
    text.chars().any(|character| character.is_ascii_digit())
}

/// Returns whether the text holds a character of the Chinese.
pub fn contains_chinese(text: &str) -> bool {
    text.chars()
        .any(|character| ('\u{4e00}'..='\u{9fa5}').contains(&character))
}

/// Returns the character as often as the number tells.
pub fn repeats(character: char, repeat: usize) -> String {
    std::iter::repeat_n(character, repeat).collect()
}

/// Returns the text, shortened to the limit.
///
/// The text is shortened at both ends, so the beginning and the end of it
/// stay.
pub fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }

    let half = limit.saturating_sub(5) / 2;
    let characters: Vec<char> = text.chars().collect();
    let length = characters.len();

    let start: String = characters[..half].iter().collect();
    let end: String = characters[length - half..].iter().collect();
    format!("{start} ... {end}")
}

/// Returns the number the text holds, if it holds one.
pub fn to_int(text: &str) -> Option<i32> {
    if is_blank(text) {
        return None;
    }
    text.trim().parse().ok()
}

/// Returns whether the text begins with the prefix and ends with the suffix.
pub fn is_surrounded(text: &str, prefix: &str, suffix: &str) -> bool {
    text.starts_with(prefix) && text.ends_with(suffix)
}

/// Returns the text without the delimiters at its beginning and at its end.
pub fn remove_surrounding<'a>(text: &'a str, prefix: &str, suffix: &str) -> &'a str {
    if is_surrounded(text, prefix, suffix) && text.len() >= prefix.len() + suffix.len() {
        &text[prefix.len()..text.len() - suffix.len()]
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_empty_texts_are_told_apart() {
        assert!(is_blank(""));
        assert!(is_blank("  \t "));
        assert!(!is_blank(" a "));
        assert!(is_not_blank("a"));
    }

    #[test]
    fn the_texts_are_capitalized() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_words("hello world"), "Hello World");
        assert_eq!(
            capitalize_words("hello-world_and_more"),
            "Hello World And More"
        );
    }

    #[test]
    fn the_parts_of_the_texts_are_found() {
        assert_eq!(substring_after("a.b.c", '.'), "b.c");
        assert_eq!(substring_after("abc", '.'), "");
        assert_eq!(substring_after_last("a.b.c", '.'), "c");
        assert_eq!(substring_before("a.b.c", '.'), "a");
        assert_eq!(substring_before("abc", '.'), "abc");
        assert_eq!(substring_before_last("a.b.c", '.'), "a.b");
    }

    #[test]
    fn the_prefixes_and_the_suffixes_are_removed() {
        assert_eq!(remove_prefix("prefix-text", "prefix-"), "text");
        assert_eq!(remove_prefix("text", "prefix-"), "text");
        assert_eq!(remove_suffix("text-suffix", "-suffix"), "text");
        assert_eq!(remove_suffix("text", "-suffix"), "text");
    }

    #[test]
    fn the_white_space_is_normalized() {
        assert_eq!(normalize_whitespaces("  a   b  "), "a b");
        assert_eq!(normalize_whitespaces("a\t\nb"), "a b");
        assert_eq!(normalize_whitespaces("   "), "");
        assert_eq!(normalize_whitespaces("a b"), "a b");
    }

    #[test]
    fn the_texts_are_told_apart() {
        assert!(is_ascii("hello"));
        assert!(!is_ascii("你好"));
        assert!(is_alphabetic_or_number("abc123"));
        assert!(!is_alphabetic_or_number("abc 123"));
        assert!(contains_digit("abc123"));
        assert!(!contains_digit("abc"));
        assert!(contains_chinese("你好"));
        assert!(!contains_chinese("hello"));
        assert!(starts_with_ignore_case("Hello", "he"));
        assert!(!starts_with_ignore_case("Hello", "lo"));
        assert!(starts_with_any("prefix-text", ["a", "prefix"]));
        assert!(ends_with_any("text-suffix", ["suffix", "x"]));
        assert!(contains_one(["Hello World"], ["world"]));
        assert!(!contains_one(["Hello World"], ["worlds"]));
    }

    #[test]
    fn the_texts_are_shortened() {
        assert_eq!(truncate("hello", 10), "hello");
        // The text is shortened at both ends, like in the Java launcher.
        assert_eq!(truncate("hello world", 9), "he ... ld");
        assert_eq!(truncate("hello world", 11), "hello world");
        assert_eq!(truncate("hello world", 10), "he ... ld");
    }

    #[test]
    fn the_numbers_are_read() {
        assert_eq!(to_int("42"), Some(42));
        assert_eq!(to_int(" -42 "), Some(-42));
        assert_eq!(to_int(""), None);
        assert_eq!(to_int("abc"), None);
    }

    #[test]
    fn the_surroundings_are_removed() {
        assert!(is_surrounded("[text]", "[", "]"));
        assert_eq!(remove_surrounding("[text]", "[", "]"), "text");
        assert_eq!(remove_surrounding("text", "[", "]"), "text");
    }

    #[test]
    fn the_characters_are_repeated() {
        assert_eq!(repeats('-', 3), "---");
        assert_eq!(repeats('-', 0), "");
    }
}
