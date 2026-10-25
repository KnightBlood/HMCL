//! A small reader of the TOML documents the launcher reads.
//!
//! The launcher reads the metadata of the mods of the game, which Forge and
//! NeoForge describe in TOML. The reader keeps the values of the document — the
//! texts, the numbers, the truth values, the arrays and the tables — and
//! rejects a document it cannot read.
//!
//! The reader covers the part of TOML the metadata of the mods uses: tables,
//! lists of tables, dotted keys, basic and literal strings (single line and
//! multiple lines), integers, decimal numbers, truth values, arrays and inline
//! tables. Dates are kept as the text they are written with.

use std::collections::BTreeMap;
use std::fmt;

/// A table of a document, which holds the values by their keys.
pub type Table = BTreeMap<String, Value>;

/// A value of a document.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A text.
    String(String),
    /// A whole number.
    Integer(i64),
    /// A decimal number.
    Float(f64),
    /// A truth value.
    Boolean(bool),
    /// A list of the values.
    Array(Vec<Value>),
    /// A table of the values by their keys.
    Table(Table),
}

impl Value {
    /// Returns the text of the value, if it is a text.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// Returns the table of the value, if it is a table.
    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Self::Table(table) => Some(table),
            _ => None,
        }
    }

    /// Returns the list of the value, if it is a list.
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Returns the whole number of the value, if it is a whole number.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(number) => Some(*number),
            _ => None,
        }
    }

    /// Returns the truth value of the value, if it is a truth value.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the name of the kind of the value, as the errors write it.
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::String(_) => "a text",
            Self::Integer(_) => "a whole number",
            Self::Float(_) => "a decimal number",
            Self::Boolean(_) => "a truth value",
            Self::Array(_) => "a list",
            Self::Table(_) => "a table",
        }
    }
}

/// The reason a document cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TomlError {
    /// The line of the document the reader stopped at.
    line: usize,
    /// The reason the document was rejected.
    reason: String,
}

impl TomlError {
    /// Creates the reason a document is rejected at a line.
    fn new(line: usize, reason: impl Into<String>) -> Self {
        Self {
            line,
            reason: reason.into(),
        }
    }

    /// Returns the line of the document the reader stopped at.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the reason the document was rejected.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for TomlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.reason)
    }
}

impl std::error::Error for TomlError {}

/// Reads a document.
pub fn parse(text: &str) -> Result<Table, TomlError> {
    let mut parser = Parser::new(text);
    let mut root = Table::new();
    let mut path: Vec<String> = Vec::new();

    loop {
        parser.skip_blank();
        if parser.is_empty() {
            break;
        }

        if parser.peek() == Some(b'[') {
            path = parser.parse_header(&mut root)?;
        } else {
            let (key, value) = parser.parse_key_value()?;
            let target = descend(&mut root, &path);
            assign(target, &key, value, parser.line())?;
        }
    }

    Ok(root)
}

/// A reader over the text of a document.
struct Parser<'a> {
    /// The text of the document.
    source: &'a str,
    /// The offset of the next byte to read.
    position: usize,
}

impl<'a> Parser<'a> {
    /// Creates a reader over a document.
    fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    /// Returns the line the reader is at, counted from one.
    fn line(&self) -> usize {
        let end = self.position.min(self.source.len());
        self.source.as_bytes()[..end]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
            + 1
    }

    /// Returns whether the reader has read the whole document.
    fn is_empty(&self) -> bool {
        self.position >= self.source.len()
    }

    /// Returns the byte at an offset from the reader, if there is one.
    fn byte_at(&self, offset: usize) -> Option<u8> {
        self.source.as_bytes().get(self.position + offset).copied()
    }

    /// Returns the next byte, if there is one.
    fn peek(&self) -> Option<u8> {
        self.byte_at(0)
    }

    /// Moves the reader past the next byte.
    fn advance(&mut self) {
        self.position += 1;
    }

    /// Reads the next character of the document.
    fn next_char(&mut self) -> Result<char, TomlError> {
        let rest = &self.source[self.position..];
        let character = rest
            .chars()
            .next()
            .ok_or_else(|| TomlError::new(self.line(), "the document ended"))?;
        self.position += character.len_utf8();
        Ok(character)
    }

    /// Reads the byte the document must hold next.
    fn expect(&mut self, byte: u8) -> Result<(), TomlError> {
        if self.peek() == Some(byte) {
            self.advance();
            Ok(())
        } else {
            Err(TomlError::new(
                self.line(),
                format!("expected `{}`", byte as char),
            ))
        }
    }

    /// Skips the spaces of a line.
    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(b' ') | Some(b'\t')) {
            self.advance();
        }
    }

    /// Skips a comment to the end of its line.
    fn skip_comment(&mut self) {
        if self.peek() != Some(b'#') {
            return;
        }
        while let Some(byte) = self.peek() {
            if byte == b'\n' {
                break;
            }
            self.advance();
        }
    }

    /// Skips the spaces, the newlines and the comments of the document.
    fn skip_blank(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => self.advance(),
                Some(b'#') => self.skip_comment(),
                _ => break,
            }
        }
    }

    /// Reads the end of a line after the value of the line.
    fn finish_line(&mut self) -> Result<(), TomlError> {
        self.skip_spaces();
        self.skip_comment();
        match self.peek() {
            None => Ok(()),
            Some(b'\n') => {
                self.advance();
                Ok(())
            }
            Some(b'\r') if self.byte_at(1) == Some(b'\n') => {
                self.position += 2;
                Ok(())
            }
            Some(byte) => Err(TomlError::new(
                self.line(),
                format!("unexpected `{}` after the value", byte as char),
            )),
        }
    }

    /// Reads the header of a table and returns the path it names.
    ///
    /// A header of a list of tables, `[[name]]`, appends a table to the list,
    /// so the keys after it are written into the table that was appended.
    fn parse_header(&mut self, root: &mut Table) -> Result<Vec<String>, TomlError> {
        let line = self.line();
        self.expect(b'[')?;
        let is_list = self.peek() == Some(b'[');
        if is_list {
            self.advance();
        }
        self.skip_spaces();
        let key = self.parse_dotted_key()?;
        self.skip_spaces();
        self.expect(b']')?;
        if is_list {
            self.expect(b']')?;
        }
        self.finish_line()?;

        if is_list {
            let (name, parents) = key
                .split_last()
                .ok_or_else(|| TomlError::new(line, "the header does not name a table"))?;
            let parent = descend(root, parents);
            match parent.get_mut(name) {
                Some(Value::Array(items)) => items.push(Value::Table(Table::new())),
                Some(_) => {
                    return Err(TomlError::new(
                        line,
                        format!("`{name}` is not a list of tables"),
                    ))
                }
                None => {
                    parent.insert(name.clone(), Value::Array(vec![Value::Table(Table::new())]));
                }
            }
        } else {
            descend(root, &key);
        }

        Ok(key)
    }

    /// Reads a key, which may name a table the value is written into.
    fn parse_dotted_key(&mut self) -> Result<Vec<String>, TomlError> {
        let mut key = Vec::new();
        loop {
            self.skip_spaces();
            let part = match self.peek() {
                Some(b'"') | Some(b'\'') => self.parse_string()?,
                _ => self.parse_bare_key()?,
            };
            key.push(part);
            self.skip_spaces();
            if self.peek() == Some(b'.') {
                self.advance();
            } else {
                break;
            }
        }
        Ok(key)
    }

    /// Reads a key that is not quoted.
    fn parse_bare_key(&mut self) -> Result<String, TomlError> {
        let start = self.position;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' {
                self.advance();
            } else {
                break;
            }
        }
        if start == self.position {
            return Err(TomlError::new(self.line(), "expected a key"));
        }
        Ok(self.source[start..self.position].to_string())
    }

    /// Reads a key and the value it is given.
    fn parse_key_value(&mut self) -> Result<(Vec<String>, Value), TomlError> {
        let key = self.parse_dotted_key()?;
        self.skip_spaces();
        self.expect(b'=')?;
        self.skip_spaces();
        let value = self.parse_value()?;
        self.finish_line()?;
        Ok((key, value))
    }

    /// Reads a value.
    fn parse_value(&mut self) -> Result<Value, TomlError> {
        match self.peek() {
            Some(b'"') | Some(b'\'') => Ok(Value::String(self.parse_string()?)),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_inline_table(),
            None => Err(TomlError::new(self.line(), "expected a value")),
            _ => self.parse_atom(),
        }
    }

    /// Reads a list of the values.
    fn parse_array(&mut self) -> Result<Value, TomlError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        loop {
            self.skip_blank();
            if self.peek() == Some(b']') {
                self.advance();
                return Ok(Value::Array(items));
            }
            items.push(self.parse_value()?);
            self.skip_blank();
            match self.peek() {
                Some(b',') => self.advance(),
                Some(b']') => {
                    self.advance();
                    return Ok(Value::Array(items));
                }
                _ => return Err(TomlError::new(self.line(), "expected `,` or `]`")),
            }
        }
    }

    /// Reads a table that is written on one line.
    fn parse_inline_table(&mut self) -> Result<Value, TomlError> {
        self.expect(b'{')?;
        let mut table = Table::new();
        loop {
            self.skip_blank();
            if self.peek() == Some(b'}') {
                self.advance();
                return Ok(Value::Table(table));
            }
            let line = self.line();
            let key = self.parse_dotted_key()?;
            self.skip_spaces();
            self.expect(b'=')?;
            self.skip_spaces();
            let value = self.parse_value()?;
            assign(&mut table, &key, value, line)?;
            self.skip_blank();
            match self.peek() {
                Some(b',') => self.advance(),
                Some(b'}') => {
                    self.advance();
                    return Ok(Value::Table(table));
                }
                _ => return Err(TomlError::new(self.line(), "expected `,` or `}`")),
            }
        }
    }

    /// Reads a value that is not a text, a list or a table.
    fn parse_atom(&mut self) -> Result<Value, TomlError> {
        let line = self.line();
        let start = self.position;
        while let Some(byte) = self.peek() {
            if matches!(
                byte,
                b' ' | b'\t' | b'\r' | b'\n' | b',' | b']' | b'}' | b'#'
            ) {
                break;
            }
            self.advance();
        }

        let token = &self.source[start..self.position];
        if token.is_empty() {
            return Err(TomlError::new(line, "expected a value"));
        }

        match token {
            "true" => return Ok(Value::Boolean(true)),
            "false" => return Ok(Value::Boolean(false)),
            "inf" | "+inf" => return Ok(Value::Float(f64::INFINITY)),
            "-inf" => return Ok(Value::Float(f64::NEG_INFINITY)),
            "nan" | "+nan" | "-nan" => return Ok(Value::Float(f64::NAN)),
            _ => {}
        }

        // A date or a time is kept as the text it is written with.
        if is_temporal(token) {
            return Ok(Value::String(token.to_string()));
        }

        parse_number(token)
            .map(Ok)
            .unwrap_or_else(|| Err(TomlError::new(line, format!("`{token}` is not a value"))))
    }

    /// Reads a text, whether it is quoted the basic way or the literal way.
    fn parse_string(&mut self) -> Result<String, TomlError> {
        match self.peek() {
            Some(b'"') => {
                if self.byte_at(1) == Some(b'"') && self.byte_at(2) == Some(b'"') {
                    self.position += 3;
                    self.parse_basic_multiline()
                } else {
                    self.advance();
                    self.parse_basic_single()
                }
            }
            Some(b'\'') => {
                if self.byte_at(1) == Some(b'\'') && self.byte_at(2) == Some(b'\'') {
                    self.position += 3;
                    self.parse_literal_multiline()
                } else {
                    self.advance();
                    self.parse_literal_single()
                }
            }
            _ => Err(TomlError::new(self.line(), "expected a text")),
        }
    }

    /// Reads a text of a line that is quoted with double quotes.
    fn parse_basic_single(&mut self) -> Result<String, TomlError> {
        let mut text = String::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    return Err(TomlError::new(self.line(), "the text is not closed"))
                }
                Some(b'"') => {
                    self.advance();
                    return Ok(text);
                }
                Some(b'\\') => {
                    self.advance();
                    self.parse_escape(&mut text)?;
                }
                Some(_) => text.push(self.next_char()?),
            }
        }
    }

    /// Reads a text of a line that is quoted with single quotes.
    fn parse_literal_single(&mut self) -> Result<String, TomlError> {
        let start = self.position;
        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    return Err(TomlError::new(self.line(), "the text is not closed"))
                }
                Some(b'\'') => {
                    let text = self.source[start..self.position].to_string();
                    self.advance();
                    return Ok(text);
                }
                Some(_) => self.advance(),
            }
        }
    }

    /// Reads a text over several lines that is quoted with double quotes.
    fn parse_basic_multiline(&mut self) -> Result<String, TomlError> {
        self.trim_first_newline();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Err(TomlError::new(self.line(), "the text is not closed")),
                Some(b'"') => {
                    if self.byte_at(1) == Some(b'"') && self.byte_at(2) == Some(b'"') {
                        self.position += 3;
                        // Up to two more quotes are the end of the text.
                        let mut extra = 0;
                        while extra < 2 && self.peek() == Some(b'"') {
                            self.advance();
                            text.push('"');
                            extra += 1;
                        }
                        return Ok(text);
                    }
                    self.advance();
                    text.push('"');
                }
                Some(b'\\') => {
                    self.advance();
                    self.parse_escape(&mut text)?;
                }
                Some(_) => text.push(self.next_char()?),
            }
        }
    }

    /// Reads a text over several lines that is quoted with single quotes.
    fn parse_literal_multiline(&mut self) -> Result<String, TomlError> {
        self.trim_first_newline();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Err(TomlError::new(self.line(), "the text is not closed")),
                Some(b'\'') => {
                    if self.byte_at(1) == Some(b'\'') && self.byte_at(2) == Some(b'\'') {
                        self.position += 3;
                        let mut extra = 0;
                        while extra < 2 && self.peek() == Some(b'\'') {
                            self.advance();
                            text.push('\'');
                            extra += 1;
                        }
                        return Ok(text);
                    }
                    self.advance();
                    text.push('\'');
                }
                Some(_) => text.push(self.next_char()?),
            }
        }
    }

    /// Skips the newline that follows the opening of a text over several lines.
    fn trim_first_newline(&mut self) {
        if self.peek() == Some(b'\r') && self.byte_at(1) == Some(b'\n') {
            self.position += 2;
        } else if self.peek() == Some(b'\n') {
            self.advance();
        }
    }

    /// Reads the character a backslash of a text introduces.
    fn parse_escape(&mut self, text: &mut String) -> Result<(), TomlError> {
        let line = self.line();
        let byte = self
            .peek()
            .ok_or_else(|| TomlError::new(line, "the text is not closed"))?;
        self.advance();

        match byte {
            b'b' => text.push('\u{8}'),
            b't' => text.push('\t'),
            b'n' => text.push('\n'),
            b'f' => text.push('\u{c}'),
            b'r' => text.push('\r'),
            b'"' => text.push('"'),
            b'\\' => text.push('\\'),
            b'u' => {
                let value = self.parse_hex_digits(4)?;
                let character = char::from_u32(value)
                    .ok_or_else(|| TomlError::new(line, "the escape is not a character"))?;
                text.push(character);
            }
            b'U' => {
                let value = self.parse_hex_digits(8)?;
                let character = char::from_u32(value)
                    .ok_or_else(|| TomlError::new(line, "the escape is not a character"))?;
                text.push(character);
            }
            b' ' | b'\t' | b'\r' | b'\n' => {
                // A backslash at the end of a line of a text over several lines
                // trims the whitespace up to the next text.
                while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                    self.advance();
                }
            }
            _ => return Err(TomlError::new(line, "the escape is not known")),
        }

        Ok(())
    }

    /// Reads the hexadecimal digits of an escape.
    fn parse_hex_digits(&mut self, count: usize) -> Result<u32, TomlError> {
        let line = self.line();
        let mut value = 0u32;
        for _ in 0..count {
            let byte = self
                .peek()
                .ok_or_else(|| TomlError::new(line, "the escape is not closed"))?;
            let digit = (byte as char)
                .to_digit(16)
                .ok_or_else(|| TomlError::new(line, "the escape is not a number"))?;
            value = value * 16 + digit;
            self.advance();
        }
        Ok(value)
    }
}

/// Returns the table a path names, making the tables that are missing.
///
/// A path that names a list of tables is followed into the table the list ends
/// with, so the keys under a header of a list of tables are written into the
/// table the header appended.
fn descend<'a>(table: &'a mut Table, path: &[String]) -> &'a mut Table {
    let mut current = table;

    for key in path {
        if !current.contains_key(key) {
            current.insert(key.clone(), Value::Table(Table::new()));
        }

        let entry = current
            .get_mut(key)
            .expect("the key was made just before it is read");

        let next = match entry {
            Value::Table(inner) => inner,
            Value::Array(items) => {
                if !matches!(items.last(), Some(Value::Table(_))) {
                    items.push(Value::Table(Table::new()));
                }
                match items.last_mut() {
                    Some(Value::Table(inner)) => inner,
                    _ => unreachable!("the list ends with a table"),
                }
            }
            other => {
                *other = Value::Table(Table::new());
                match other {
                    Value::Table(inner) => inner,
                    _ => unreachable!("the value was made a table"),
                }
            }
        };

        current = next;
    }

    current
}

/// Writes a value at the path of a key.
fn assign(table: &mut Table, key: &[String], value: Value, line: usize) -> Result<(), TomlError> {
    let (name, parents) = key
        .split_last()
        .ok_or_else(|| TomlError::new(line, "an empty key"))?;

    let target = descend(table, parents);
    if target.contains_key(name) {
        return Err(TomlError::new(line, format!("`{name}` is written twice")));
    }
    target.insert(name.clone(), value);
    Ok(())
}

/// Returns whether the text is a date or a time, which is kept as it is.
fn is_temporal(token: &str) -> bool {
    if token.contains(':') {
        return true;
    }

    let bytes = token.as_bytes();
    bytes.len() >= 10
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

/// Reads a number.
fn parse_number(token: &str) -> Option<Value> {
    let cleaned: String = token
        .chars()
        .filter(|character| *character != '_')
        .collect();

    let (negative, digits) = if let Some(rest) = cleaned.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = cleaned.strip_prefix('+') {
        (false, rest)
    } else {
        (false, cleaned.as_str())
    };

    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(rest) = digits.strip_prefix(prefix) {
            let magnitude = i64::from_str_radix(rest, radix).ok()?;
            return Some(Value::Integer(if negative {
                -magnitude
            } else {
                magnitude
            }));
        }
    }

    if digits.contains('.') || digits.contains('e') || digits.contains('E') {
        let number: f64 = cleaned.parse().ok()?;
        return Some(Value::Float(if negative { -number } else { number }));
    }

    let magnitude: i64 = digits.parse().ok()?;
    Some(Value::Integer(if negative {
        -magnitude
    } else {
        magnitude
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads a document the tests work in and returns its root table.
    fn read(text: &str) -> Table {
        parse(text).expect("the document should be read")
    }

    /// Returns the text at a path of a table.
    fn text_at<'a>(table: &'a Table, path: &[&str]) -> Option<&'a str> {
        let mut current = table;
        for (index, key) in path.iter().enumerate() {
            let value = current.get(*key)?;
            if index + 1 == path.len() {
                return value.as_str();
            }
            current = value.as_table()?;
        }
        None
    }

    #[test]
    fn the_values_of_the_metadata_are_read() {
        let document = r#"
modLoader = "javafml"
loaderVersion = "[47,)"
license = "MIT"
showAsResourcePack = false
issueTrackerURL = "https://example.com/issues"

[[mods]]
modId = "example"
version = "1.2.3"
displayName = "Example Mod"
authors = "Somebody, Someone"
description = '''
A mod that does things.
'''
logoFile = "logo.png"

[[dependencies.example]]
modId = "forge"
mandatory = true
versionRange = "[47,)"
ordering = "NONE"
side = "BOTH"
"#;

        let table = read(document);
        assert_eq!(text_at(&table, &["modLoader"]), Some("javafml"));
        assert_eq!(text_at(&table, &["license"]), Some("MIT"));
        assert_eq!(
            table.get("showAsResourcePack"),
            Some(&Value::Boolean(false))
        );

        let mods = table
            .get("mods")
            .and_then(Value::as_array)
            .expect("the mods should be a list");
        assert_eq!(mods.len(), 1);
        let first = mods[0].as_table().expect("a mod should be a table");
        assert_eq!(text_at(first, &["modId"]), Some("example"));
        assert_eq!(text_at(first, &["displayName"]), Some("Example Mod"));
        assert_eq!(
            text_at(first, &["description"]),
            Some("A mod that does things.\n")
        );

        let dependencies = table
            .get("dependencies")
            .and_then(Value::as_table)
            .and_then(|table| table.get("example"))
            .and_then(Value::as_array)
            .expect("the dependencies should be a list");
        assert_eq!(dependencies.len(), 1);
    }

    #[test]
    fn the_numbers_are_read() {
        let table = read("whole = 47\nnegative = -3\nhex = 0xFF\ndecimal = 1.5\n");

        assert_eq!(table.get("whole"), Some(&Value::Integer(47)));
        assert_eq!(table.get("negative"), Some(&Value::Integer(-3)));
        assert_eq!(table.get("hex"), Some(&Value::Integer(255)));
        assert_eq!(table.get("decimal"), Some(&Value::Float(1.5)));
    }

    #[test]
    fn the_texts_are_read() {
        let table = read(
            r#"basic = "a\tb"
literal = 'c:\path'
escaped = "quote \" and unicode \u00e9"
multiline = """
first
second"""
"#,
        );

        assert_eq!(table.get("basic"), Some(&Value::String("a\tb".to_string())));
        assert_eq!(
            table.get("literal"),
            Some(&Value::String("c:\\path".to_string()))
        );
        assert_eq!(
            table.get("escaped"),
            Some(&Value::String("quote \" and unicode é".to_string()))
        );
        assert_eq!(
            table.get("multiline"),
            Some(&Value::String("first\nsecond".to_string()))
        );
    }

    #[test]
    fn the_dotted_keys_write_into_the_tables() {
        let table = read("a.b.c = 1\n");

        let a = table.get("a").and_then(Value::as_table).expect("a");
        let b = a.get("b").and_then(Value::as_table).expect("b");
        assert_eq!(b.get("c"), Some(&Value::Integer(1)));
    }

    #[test]
    fn the_tables_are_written_under_their_header() {
        let table =
            read("[metadata]\nname = \"Example\"\n\n[metadata.contact]\nhomepage = \"x\"\n");

        assert_eq!(text_at(&table, &["metadata", "name"]), Some("Example"));
        assert_eq!(
            text_at(&table, &["metadata", "contact", "homepage"]),
            Some("x")
        );
    }

    #[test]
    fn the_inline_tables_are_read() {
        let table = read("point = { x = 1, y = 2 }\n");

        let point = table.get("point").and_then(Value::as_table).expect("point");
        assert_eq!(point.get("x"), Some(&Value::Integer(1)));
        assert_eq!(point.get("y"), Some(&Value::Integer(2)));
    }

    #[test]
    fn a_document_that_is_written_twice_is_refused() {
        assert!(parse("a = 1\na = 2\n").is_err());
        assert!(parse("a = 1\n").is_ok());
    }

    #[test]
    fn a_document_that_is_not_toml_is_refused() {
        assert!(parse("a = \n").is_err());
        assert!(parse("a\n").is_err());
        assert!(parse("= 1\n").is_err());
        assert!(parse("a = \"unclosed\n").is_err());
    }

    #[test]
    fn the_comments_and_the_blank_lines_are_skipped() {
        let table = read("# a comment\n\n   \na = 1 # trailing\n\n# tail\n");
        assert_eq!(table.get("a"), Some(&Value::Integer(1)));
    }
}
