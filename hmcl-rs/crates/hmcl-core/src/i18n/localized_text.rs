//! The texts the launcher shows in more than one language.

use std::fmt;

use serde::de::{Error as _, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A text that is either the same in every language or has one value for
/// each language.
///
/// The text is written as a JSON string when it is the same in every
/// language, and as a JSON object that maps language keys, such as `en`,
/// `zh-Hans` or `default`, to the text of that language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalizedText {
    /// A text that is the same in every language.
    Plain(String),
    /// The texts of the languages, in the order they were read.
    Localized(Vec<(String, String)>),
}

impl LocalizedText {
    /// Creates a text that is the same in every language.
    pub fn plain(value: impl Into<String>) -> Self {
        Self::Plain(value.into())
    }

    /// Creates a text from the values of the languages.
    ///
    /// The values are kept in the order they are given, which is the order
    /// the launcher falls back to when no language matches.
    ///
    /// # Panics
    ///
    /// Panics when no value is given, because a text without a value cannot
    /// be shown.
    pub fn localized(values: Vec<(String, String)>) -> Self {
        assert!(
            !values.is_empty(),
            "the localized text needs at least one value"
        );
        Self::Localized(values)
    }

    /// Returns whether the text is the same in every language.
    pub fn is_plain(&self) -> bool {
        matches!(self, Self::Plain(_))
    }

    /// Returns whether at least one value of the text is empty.
    pub fn may_be_empty(&self) -> bool {
        match self {
            Self::Plain(value) => value.is_empty(),
            Self::Localized(values) => values.iter().any(|(_, value)| value.is_empty()),
        }
    }

    /// Returns the text of one language, if it has one.
    pub fn get(&self, language: &str) -> Option<&str> {
        match self {
            Self::Plain(value) => Some(value),
            Self::Localized(values) => values
                .iter()
                .find(|(key, _)| key == language)
                .map(|(_, value)| value.as_str()),
        }
    }

    /// Returns the best text for the languages the launcher prefers.
    ///
    /// The languages are checked in the order they are given. When none of
    /// them matches, the text of the language `default` is used; a text
    /// without that language shows the first value it has.
    pub fn text(&self, languages: &[&str]) -> &str {
        match self {
            Self::Plain(value) => value,
            Self::Localized(values) => {
                for language in languages {
                    if let Some((_, value)) = values.iter().find(|(key, _)| key == language) {
                        return value;
                    }
                }

                if let Some((_, value)) = values.iter().find(|(key, _)| key == "default") {
                    return value;
                }

                &values[0].1
            }
        }
    }
}

impl fmt::Display for LocalizedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.text(&[]))
    }
}

impl Serialize for LocalizedText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Plain(value) => serializer.serialize_str(value),
            Self::Localized(values) => {
                use serde::ser::SerializeMap;

                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (language, value) in values {
                    map.serialize_entry(language, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for LocalizedText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TextVisitor;

        impl<'de> Visitor<'de> for TextVisitor {
            type Value = LocalizedText;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string or an object of strings")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(LocalizedText::Plain(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(LocalizedText::Plain(value))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some((language, value)) = map.next_entry::<String, String>()? {
                    values.push((language, value));
                }

                if values.is_empty() {
                    return Err(A::Error::custom(
                        "the localized text cannot be an empty object",
                    ));
                }

                Ok(LocalizedText::Localized(values))
            }
        }

        deserializer.deserialize_any(TextVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_text_is_written_as_a_string() {
        let text = LocalizedText::plain("Launcher");
        let json = serde_json::to_string(&text).expect("the text should be written");

        assert_eq!(json, "\"Launcher\"");
        assert_eq!(text.get("en"), Some("Launcher"));
        assert_eq!(text.text(&["zh-Hans"]), "Launcher");
    }

    #[test]
    fn a_localized_text_is_written_as_an_object() {
        let text = LocalizedText::localized(vec![
            ("zh-Hans".to_string(), "启动器".to_string()),
            ("default".to_string(), "Launcher".to_string()),
            ("en".to_string(), "Launcher".to_string()),
        ]);

        let json = serde_json::to_string(&text).expect("the text should be written");
        assert_eq!(
            json,
            r#"{"zh-Hans":"启动器","default":"Launcher","en":"Launcher"}"#
        );

        assert_eq!(text.get("zh-Hans"), Some("启动器"));
        assert_eq!(text.text(&["zh-Hans"]), "启动器");
        assert_eq!(text.text(&["fr"]), "Launcher");
        assert_eq!(text.text(&["en", "zh-Hans"]), "Launcher");
    }

    #[test]
    fn a_localized_text_without_a_default_shows_its_first_value() {
        let text = LocalizedText::localized(vec![
            ("en".to_string(), "Launcher".to_string()),
            ("zh-Hans".to_string(), "启动器".to_string()),
        ]);

        assert_eq!(text.text(&["fr"]), "Launcher");
    }

    #[test]
    fn an_empty_localized_text_is_refused() {
        assert!(serde_json::from_str::<LocalizedText>("{}").is_err());
        assert!(serde_json::from_str::<LocalizedText>("[]").is_err());
    }
}
