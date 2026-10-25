//! The resources of the launcher.
//!
//! The resources are the texts of the launcher in the language the user
//! selected and the colors of the panels, which follow the theme.

use std::rc::Rc;

use ferroui::base::controls::ResourceDictionary;
use ferroui::base::media::{Color, SolidColorBrush};
use ferroui::base::styling::ThemeVariant;
use ferroui::base::{BoxedValue, Ref};
use hmcl_core::i18n::I18n;

/// The name of the resource of the background of the panels.
pub const PANEL_BACKGROUND: &str = "HMCL.PanelBackground";

/// The name of the resource of the background of the cards.
pub const CARD_BACKGROUND: &str = "HMCL.CardBackground";

/// The name of the resource of the background of the pages.
pub const PAGE_BACKGROUND: &str = "HMCL.PageBackground";

/// The name of the resource of the background of the side bar.
pub const SIDEBAR_BACKGROUND: &str = "HMCL.SidebarBackground";

/// The name of the resource of the borders.
pub const BORDER_BRUSH: &str = "HMCL.BorderBrush";

/// The name of the resource of the texts that are not important.
pub const MUTED_TEXT: &str = "HMCL.MutedText";

/// The name of the resource of the background of the bar of the title.
pub const TITLE_BAR_BACKGROUND: &str = "HMCL.TitleBarBackground";

/// The name of the resource of the texts of the bar of the title.
pub const TITLE_BAR_FOREGROUND: &str = "HMCL.TitleBarForeground";

/// The name of the resource of the background of the buttons that stand out.
pub const ACCENT_BACKGROUND: &str = "HMCL.AccentBackground";

/// The name of the resource of the texts of the buttons that stand out.
pub const ACCENT_FOREGROUND: &str = "HMCL.AccentForeground";

/// The name of the resource of the texts of the pages.
pub const TEXT_FOREGROUND: &str = "HMCL.TextForeground";

/// The color of the background of the panels, in the light theme.
const PANEL_BACKGROUND_LIGHT: &str = "#FAFAFA";
/// The color of the background of the panels, in the dark theme.
const PANEL_BACKGROUND_DARK: &str = "#2B2B2B";
/// The color of the background of the cards, in the light theme.
const CARD_BACKGROUND_LIGHT: &str = "#FFFFFF";
/// The color of the background of the cards, in the dark theme.
const CARD_BACKGROUND_DARK: &str = "#333333";
/// The color of the borders, in the light theme.
const BORDER_BRUSH_LIGHT: &str = "#DDDDDD";
/// The color of the borders, in the dark theme.
const BORDER_BRUSH_DARK: &str = "#3F3F3F";
/// The color of the texts that are not important, in the light theme.
const MUTED_TEXT_LIGHT: &str = "#666666";
/// The color of the texts that are not important, in the dark theme.
const MUTED_TEXT_DARK: &str = "#AAAAAA";
/// The color of the texts, in the light theme.
const TEXT_FOREGROUND_LIGHT: &str = "#1F1F1F";
/// The color of the texts, in the dark theme.
const TEXT_FOREGROUND_DARK: &str = "#E6E6E6";
/// The color of the title bar and of the buttons that stand out, in the light
/// theme.
const ACCENT_BACKGROUND_LIGHT: &str = "#2B579A";
/// The color of the title bar and of the buttons that stand out, in the dark
/// theme.
const ACCENT_BACKGROUND_DARK: &str = "#1E3F6E";

/// Creates the resources of the launcher.
///
/// The resources hold the texts of the launcher in the given language and the
/// colors of the panels for the light and the dark theme.
pub fn create(i18n: &I18n) -> Ref<ResourceDictionary> {
    let resources = ResourceDictionary::new();

    // The colors of the panels follow the theme, so that the dark theme is
    // not broken by the colors written in the pages.
    let light = ResourceDictionary::new();
    light.add(PANEL_BACKGROUND, brush(PANEL_BACKGROUND_LIGHT));
    light.add(CARD_BACKGROUND, brush(CARD_BACKGROUND_LIGHT));
    // The pages are drawn over the wallpaper with a translucent surface, so
    // that the controls stay readable.
    light.add(PAGE_BACKGROUND, brush("#F2FAFAFA"));
    light.add(SIDEBAR_BACKGROUND, brush("#B3F5F5F5"));
    light.add(BORDER_BRUSH, brush(BORDER_BRUSH_LIGHT));
    light.add(MUTED_TEXT, brush(MUTED_TEXT_LIGHT));
    light.add(TEXT_FOREGROUND, brush(TEXT_FOREGROUND_LIGHT));
    // The bar of the title uses the primary color of the theme, like the
    // tool bar of the Java launcher.
    light.add(TITLE_BAR_BACKGROUND, brush(ACCENT_BACKGROUND_LIGHT));
    light.add(TITLE_BAR_FOREGROUND, brush("#FFFFFF"));
    light.add(ACCENT_BACKGROUND, brush(ACCENT_BACKGROUND_LIGHT));
    light.add(ACCENT_FOREGROUND, brush("#FFFFFF"));

    let dark = ResourceDictionary::new();
    dark.add(PANEL_BACKGROUND, brush(PANEL_BACKGROUND_DARK));
    dark.add(CARD_BACKGROUND, brush(CARD_BACKGROUND_DARK));
    dark.add(PAGE_BACKGROUND, brush("#F22B2B2B"));
    dark.add(SIDEBAR_BACKGROUND, brush("#B3252525"));
    dark.add(BORDER_BRUSH, brush(BORDER_BRUSH_DARK));
    dark.add(MUTED_TEXT, brush(MUTED_TEXT_DARK));
    dark.add(TEXT_FOREGROUND, brush(TEXT_FOREGROUND_DARK));
    dark.add(TITLE_BAR_BACKGROUND, brush(ACCENT_BACKGROUND_DARK));
    dark.add(TITLE_BAR_FOREGROUND, brush("#FFFFFF"));
    dark.add(ACCENT_BACKGROUND, brush(ACCENT_BACKGROUND_DARK));
    dark.add(ACCENT_FOREGROUND, brush("#FFFFFF"));

    resources.add_theme_dictionary(ThemeVariant::light(), light);
    resources.add_theme_dictionary(ThemeVariant::dark(), dark);

    // The texts of the launcher, in the language the user selected.
    let strings = ResourceDictionary::new();
    for (key, value) in i18n.bundle().entries() {
        strings.add(key, Some(Rc::new(value.to_string())));
    }
    resources.add_merged_dictionary(strings);

    // The converters the pages use; the C# version names them as static
    // fields, which the run-time loader does not know.
    resources.add(
        "BoolToTextConverter",
        Some(Rc::new(crate::converters::BoolToTextConverter::new())),
    );
    resources.add(
        "NullToBoolConverter",
        Some(Rc::new(crate::converters::NullToBoolConverter::new())),
    );
    resources.add(
        "NotNullToBoolConverter",
        Some(Rc::new(crate::converters::NotNullToBoolConverter::new())),
    );
    resources.add(
        "VersionTypeConverter",
        Some(Rc::new(crate::converters::VersionTypeConverter::new())),
    );

    resources
}

/// Creates the brush of a color, such as `#FAFAFA`.
fn brush(color: &str) -> Option<BoxedValue> {
    let brush =
        SolidColorBrush::with_color(Color::parse(color).unwrap_or(Color::from_argb(0, 0, 0, 0)));
    Some(Rc::new(brush))
}

/// Returns the colors of the page of the login, as a style sheet.
///
/// The page of the login is shown by the browser of the player and not by the
/// window of the launcher, so it cannot read the resources of the launcher and
/// the colors are written into the page. The page names the colors the way the
/// launcher of Java names them, with the variables of the theme the Java
/// launcher writes into the sheet of its scenes.
///
/// The launcher has no implementation of the theme of the system that names
/// the colors, so the colors of the panels of the window are used: the same
/// colors, but not the colors a theme pack of the player would name.
pub fn page_style_sheet(dark: bool) -> String {
    let (surface, container_high, on_surface, on_surface_variant, primary) = if dark {
        (
            PANEL_BACKGROUND_DARK,
            CARD_BACKGROUND_DARK,
            TEXT_FOREGROUND_DARK,
            MUTED_TEXT_DARK,
            ACCENT_BACKGROUND_DARK,
        )
    } else {
        (
            PANEL_BACKGROUND_LIGHT,
            CARD_BACKGROUND_LIGHT,
            TEXT_FOREGROUND_LIGHT,
            MUTED_TEXT_LIGHT,
            ACCENT_BACKGROUND_LIGHT,
        )
    };

    let mut sheet = String::from(":root {\n");
    for (name, color) in [
        ("--monet-surface", surface),
        ("--monet-surface-container-high", container_high),
        ("--monet-on-surface", on_surface),
        ("--monet-on-surface-variant", on_surface_variant),
        ("--monet-primary", primary),
    ] {
        sheet.push_str("    ");
        sheet.push_str(name);
        sheet.push_str(": ");
        sheet.push_str(color);
        sheet.push_str(";\n");
    }
    sheet.push('}');

    sheet
}
