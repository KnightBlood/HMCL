//! The translations of the launcher.
//!
//! The launcher shows its texts in the language the user selected. The
//! translations are kept in the property files of the Java launcher, so the
//! two launchers show the same texts.
//!
//! Every language falls back to English, so a translation that is missing in
//! one language is still shown in English.

pub mod locale_utils;
pub mod localized_text;
pub mod properties;

use std::fmt;

pub use locale_utils::{
    find_all_localized_files, find_all_localized_files_by_extension, get_by_candidate_locales,
    normalize_language, parent_language, preferred_language, root_language, LanguageTag,
    TextDirection, DEFAULT_LANGUAGE_KEY,
};
pub use localized_text::LocalizedText;
pub use properties::Bundle;

/// The English translations, which every other language falls back to.
const ENGLISH: &str = include_str!("../../resources/lang/I18N.properties");

/// The English texts of the pages that were ported from the C# version.
///
/// The pages of the C# version name their texts with the keys of an older
/// version of the Java launcher, so the texts of those pages are kept apart.
const LEGACY_ENGLISH: &str = include_str!("../../resources/lang/legacy.properties");

/// The Chinese texts of the pages that were ported from the C# version.
const LEGACY_CHINESE: &str = include_str!("../../resources/lang/legacy_zh_Hans.properties");

/// The Arabic translations.
const ARABIC: &str = include_str!("../../resources/lang/I18N_ar.properties");

/// The German translations.
const GERMAN: &str = include_str!("../../resources/lang/I18N_de.properties");

/// The Spanish translations.
const SPANISH: &str = include_str!("../../resources/lang/I18N_es.properties");

/// The Japanese translations.
const JAPANESE: &str = include_str!("../../resources/lang/I18N_ja.properties");

/// The translations of the classical Chinese.
const LZH: &str = include_str!("../../resources/lang/I18N_lzh.properties");

/// The Russian translations.
const RUSSIAN: &str = include_str!("../../resources/lang/I18N_ru.properties");

/// The Ukrainian translations.
const UKRAINIAN: &str = include_str!("../../resources/lang/I18N_uk.properties");

/// The Simplified Chinese translations.
const SIMPLIFIED_CHINESE: &str = include_str!("../../resources/lang/I18N_zh_Hans.properties");

/// The Traditional Chinese translations.
const TRADITIONAL_CHINESE: &str = include_str!("../../resources/lang/I18N_zh_Hant.properties");

/// The languages the launcher shows its texts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    /// English, the language every other language falls back to.
    #[default]
    English,
    /// Arabic.
    Arabic,
    /// German.
    German,
    /// Spanish.
    Spanish,
    /// Japanese.
    Japanese,
    /// Classical Chinese.
    Lzh,
    /// Russian.
    Russian,
    /// Ukrainian.
    Ukrainian,
    /// Simplified Chinese.
    SimplifiedChinese,
    /// Traditional Chinese.
    TraditionalChinese,
}

impl Locale {
    /// Returns every language the launcher supports.
    pub const ALL: [Locale; 10] = [
        Locale::English,
        Locale::Arabic,
        Locale::German,
        Locale::Spanish,
        Locale::Japanese,
        Locale::Lzh,
        Locale::Russian,
        Locale::Ukrainian,
        Locale::SimplifiedChinese,
        Locale::TraditionalChinese,
    ];

    /// Returns the code of the language, such as `zh_Hans`.
    pub const fn code(self) -> &'static str {
        match self {
            Locale::English => "en",
            Locale::Arabic => "ar",
            Locale::German => "de",
            Locale::Spanish => "es",
            Locale::Japanese => "ja",
            Locale::Lzh => "lzh",
            Locale::Russian => "ru",
            Locale::Ukrainian => "uk",
            Locale::SimplifiedChinese => "zh_Hans",
            Locale::TraditionalChinese => "zh_Hant",
        }
    }

    /// Returns the language of a code, such as `zh_Hans`.
    ///
    /// The code of a language that the launcher does not know names the
    /// English language, which is the language every other language falls
    /// back to.
    pub fn from_code(code: &str) -> Self {
        let normalized = code.replace('-', "_");

        match normalized.as_str() {
            "ar" => Locale::Arabic,
            "de" => Locale::German,
            "es" => Locale::Spanish,
            "ja" => Locale::Japanese,
            "lzh" => Locale::Lzh,
            "ru" => Locale::Russian,
            "uk" => Locale::Ukrainian,
            "zh" | "zh_CN" | "zh_Hans" | "zh_Hans_CN" => Locale::SimplifiedChinese,
            "zh_TW" | "zh_HK" | "zh_Hant" | "zh_Hant_TW" | "zh_Hant_HK" => {
                Locale::TraditionalChinese
            }
            _ => Locale::English,
        }
    }

    /// Returns the translations of the language.
    fn bundle(self) -> Bundle {
        let (content, legacy) = match self {
            Locale::English => (ENGLISH, LEGACY_ENGLISH),
            Locale::Arabic => (ARABIC, LEGACY_ENGLISH),
            Locale::German => (GERMAN, LEGACY_ENGLISH),
            Locale::Spanish => (SPANISH, LEGACY_ENGLISH),
            Locale::Japanese => (JAPANESE, LEGACY_ENGLISH),
            Locale::Lzh => (LZH, LEGACY_ENGLISH),
            Locale::Russian => (RUSSIAN, LEGACY_ENGLISH),
            Locale::Ukrainian => (UKRAINIAN, LEGACY_ENGLISH),
            Locale::SimplifiedChinese => (SIMPLIFIED_CHINESE, LEGACY_CHINESE),
            Locale::TraditionalChinese => (TRADITIONAL_CHINESE, LEGACY_CHINESE),
        };

        let mut bundle = Bundle::parse(content);
        bundle.merge(&Bundle::parse(legacy));
        bundle
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

/// The texts of the launcher, in the language the user selected.
#[derive(Debug, Clone)]
pub struct I18n {
    /// The language the user selected.
    locale: Locale,
    /// The translations of the language.
    bundle: Bundle,
    /// The English translations, which the other languages fall back to.
    fallback: Bundle,
}

impl I18n {
    /// Creates the texts of the launcher in a language.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            bundle: locale.bundle(),
            fallback: Locale::English.bundle(),
        }
    }

    /// Returns the language the user selected.
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Returns the translations of the launcher, by their key.
    pub fn bundle(&self) -> &Bundle {
        &self.bundle
    }

    /// Returns whether the launcher has a text for the key.
    pub fn has_key(&self, key: &str) -> bool {
        self.bundle.contains_key(key) || self.fallback.contains_key(key)
    }

    /// Returns the text of the key, if the launcher has one.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.bundle.get(key).or_else(|| self.fallback.get(key))
    }

    /// Returns the text of the key.
    ///
    /// A key the launcher does not know is shown as it is, so the missing
    /// text is easy to find.
    pub fn i18n(&self, key: &str) -> String {
        self.get(key).unwrap_or(key).to_string()
    }

    /// Returns the text of the key with the arguments in it.
    ///
    /// The arguments replace the placeholders of the text, which are written
    /// as `%s`, `%1$s` and so on, like in the Java launcher.
    pub fn format(&self, key: &str, arguments: &[&str]) -> String {
        format_text(&self.i18n(key), arguments)
    }
}

/// Puts the arguments into the placeholders of a text.
///
/// The placeholders are written as `%s` and `%1$s`; a placeholder that
/// cannot be read is kept as it is.
fn format_text(text: &str, arguments: &[&str]) -> String {
    let mut result = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    let mut next = 0usize;

    while let Some(character) = characters.next() {
        if character != '%' {
            result.push(character);
            continue;
        }

        match characters.peek() {
            Some('%') => {
                characters.next();
                result.push('%');
                continue;
            }
            Some('n') => {
                characters.next();
                result.push('\n');
                continue;
            }
            None => {
                result.push('%');
                break;
            }
            _ => {}
        }

        // The index of the argument, if the placeholder names one.
        let mut index = None;
        let mut digits = String::new();
        while let Some(digit) = characters.peek() {
            if digit.is_ascii_digit() {
                digits.push(*digit);
                characters.next();
            } else {
                break;
            }
        }
        if !digits.is_empty() && characters.peek() == Some(&'$') {
            characters.next();
            index = digits.parse::<usize>().ok();
        }

        // The flags, the width and the precision of the placeholder.
        let mut specification = digits;
        while let Some(character) = characters.peek() {
            if character.is_ascii_digit() || matches!(character, '-' | '+' | ' ' | ',' | '.' | '#')
            {
                specification.push(*character);
                characters.next();
            } else {
                break;
            }
        }

        let conversion = match characters.next() {
            Some(conversion) => conversion,
            None => {
                result.push('%');
                break;
            }
        };

        if !matches!(conversion, 's' | 'S' | 'd' | 'f' | 'x' | 'X' | 'o' | 'b') {
            // The placeholder is not one the launcher knows, so it is kept.
            result.push('%');
            result.push_str(&specification);
            result.push(conversion);
            continue;
        }

        let position = match index {
            // The index of a placeholder starts at one, like in Java.
            Some(index) => index.saturating_sub(1),
            None => next,
        };
        if index.is_none() {
            next += 1;
        }

        match arguments.get(position) {
            Some(argument) => result.push_str(argument),
            None => {
                result.push('%');
                result.push_str(&specification);
                result.push(conversion);
            }
        }
    }

    result
}

impl Default for I18n {
    fn default() -> Self {
        Self::new(Locale::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_languages_are_told_apart() {
        assert_eq!(Locale::from_code("zh_Hans"), Locale::SimplifiedChinese);
        assert_eq!(Locale::from_code("zh-Hant"), Locale::TraditionalChinese);
        assert_eq!(Locale::from_code("zh"), Locale::SimplifiedChinese);
        assert_eq!(Locale::from_code("ru"), Locale::Russian);
        assert_eq!(Locale::from_code("xx"), Locale::English);
        assert_eq!(Locale::SimplifiedChinese.code(), "zh_Hans");
    }

    #[test]
    fn the_translations_are_read() {
        let i18n = I18n::new(Locale::English);

        assert!(i18n.has_key("instance.launch"));
        assert!(!i18n.i18n("instance.launch").is_empty());
        assert_eq!(i18n.locale(), Locale::English);
    }

    #[test]
    fn the_other_languages_fall_back_to_english() {
        let i18n = I18n::new(Locale::SimplifiedChinese);

        assert!(i18n.has_key("instance.launch"));
        assert!(!i18n.i18n("instance.launch").is_empty());
        assert_ne!(
            i18n.i18n("instance.launch"),
            I18n::new(Locale::English).i18n("instance.launch")
        );
    }

    #[test]
    fn a_missing_text_is_shown_as_the_key() {
        let i18n = I18n::new(Locale::English);
        assert_eq!(
            i18n.i18n("a.key.the.launcher.does.not.have"),
            "a.key.the.launcher.does.not.have"
        );
        assert!(!i18n.has_key("a.key.the.launcher.does.not.have"));
    }

    #[test]
    fn the_texts_of_the_pages_of_the_csharp_version_are_there() {
        let i18n = I18n::new(Locale::English);
        assert_eq!(i18n.i18n("hmcl.tasklistpage.468_"), "Task list");
        assert_eq!(
            i18n.i18n("hmcl.tasklistpage.472_"),
            "Clear the finished ones"
        );

        let i18n = I18n::new(Locale::SimplifiedChinese);
        assert_eq!(i18n.i18n("hmcl.tasklistpage.468_"), "任务列表");
    }

    #[test]
    fn the_arguments_are_put_in_the_text() {
        let i18n = I18n::new(Locale::English);
        let text = i18n.format("install.installer.install", &["Forge"]);
        assert!(text.contains("Forge"));
    }

    #[test]
    fn the_placeholders_of_the_text_are_filled_in() {
        assert_eq!(format_text("Installing %s", &["Forge"]), "Installing Forge");
        assert_eq!(
            format_text("Installing %1$s and %1$s", &["Forge"]),
            "Installing Forge and Forge"
        );
        assert_eq!(
            format_text("Installing %2$s with %1$s", &["Forge", "Fabric"]),
            "Installing Fabric with Forge"
        );
        assert_eq!(format_text("100%%", &[]), "100%");
        assert_eq!(format_text("no placeholder", &[]), "no placeholder");
        assert_eq!(format_text("%s", &[]), "%s");
    }

    #[test]
    fn every_language_has_the_same_keys() {
        let english = I18n::new(Locale::English);
        for locale in Locale::ALL {
            let i18n = I18n::new(locale);
            for key in [
                "instance.launch",
                "settings.advanced",
                "install.installer.install",
                "game.crash.title",
            ] {
                assert!(
                    i18n.has_key(key),
                    "the language {locale} should have the text {key}"
                );
            }
            assert_eq!(english.locale(), Locale::English);
        }
    }
}
