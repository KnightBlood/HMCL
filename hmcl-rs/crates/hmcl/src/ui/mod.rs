//! The user interface of the launcher.
//!
//! The layout lives in XAML (`MainWindow.xaml`), as in the Avalonia version of
//! HMCL; this crate embeds the document and loads it with the run-time XAML
//! loader of FerroUI. The application class (`app.rs`) installs the theme, so
//! the document only describes the window.

use ferroui::markup_xaml::XamlLoadException;
use ferroui::markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui::prelude::*;
use hmcl_core::util::logging;

/// The XAML document of the main window.
///
/// It is embedded in the binary, so the launcher starts without reading files
/// next to the executable and a missing document is a build error.
const MAIN_WINDOW_XAML: &str = include_str!("../ui/MainWindow.xaml");

/// Loads the main window from its XAML document.
///
/// When the document cannot be loaded, the error is logged and the returned
/// window shows the message instead, so the launcher reports the problem in a
/// window rather than on a console that a desktop user does not see.
pub fn create_main_window() -> Ref<Window> {
    match load_main_window() {
        Ok(window) => window,
        Err(error) => {
            logging::error(&describe(&error));
            error_window(&describe(&error))
        }
    }
}

/// Loads `MainWindow.xaml` with the run-time loader.
fn load_main_window() -> Result<Ref<Window>, XamlLoadException> {
    FerroRuntimeXamlLoader::parse_as::<Ref<Window>>(MAIN_WINDOW_XAML, None)
}

/// Builds the window shown when the document cannot be loaded.
fn error_window(message: &str) -> Ref<Window> {
    let text = TextBlock::new();
    text.set_text(Some(message));
    text.set_font_size(14.0);

    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    panel.set_margin(Thickness::uniform(24.0));
    panel.children().add(text);

    let window = Window::new();
    window.set_title(Some("HMCL.RS".to_string()));
    window.set_width(720.0);
    window.set_height(360.0);
    window.set_content(Some(Control::boxed(panel)));
    window
}

/// Returns the message of a load failure with the error of the compiler, when
/// there is one.
fn describe(error: &XamlLoadException) -> String {
    match error.inner_exception() {
        Some(inner) => format!("{}\n ---> {inner}", error.message()),
        None => error.message().to_string(),
    }
}
