//! The user interface of the launcher.
//!
//! The layout lives in XAML (`ui/MainWindow.xaml` and the pages in
//! `ui/pages`), as in the Avalonia version of HMCL; this crate embeds the
//! documents and loads them with the run-time XAML loader of FerroUI. The
//! application class (`app.rs`) installs the theme and the resources, so the
//! documents only describe the pages.
//!
//! The texts of the window come from the resources of the launcher, so the
//! window shows them in the language the user selected. The elements that
//! hold a text are named in the documents and are filled in here.

use std::path::Path;
use std::rc::Rc;

use ferroui::base::controls::NameScopeExtensions;
use ferroui::base::{BoxedValue, FerroObject, PixelPoint, Size};
use ferroui::controls::{
    as_clickable_control, ContentControl, ItemsSource, ListBoxItem, TextBlock, UserControl,
    WindowState,
};
use ferroui::markup_xaml::XamlLoadException;
use ferroui::markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui::prelude::*;
use hmcl_core::setting::{initial_content_bounds, ContentBounds, ScreenBounds};
use hmcl_core::util::logging;

use crate::context::Launcher;
use crate::view_models::export_wizard::ModpackExportWizardViewModel;
use crate::view_models::TaskListViewModel;

/// The XAML document of the main window.
///
/// It is embedded in the binary, so the launcher starts without reading files
/// next to the executable and a missing document is a build error.
const MAIN_WINDOW_XAML: &str = include_str!("../../ui/MainWindow.xaml");

/// The XAML document of the page that shows the tasks of the launcher.
///
/// The page is converted from the XAML of the C# version by
/// `tools/convert_avalonia_xaml.py`.
const TASK_LIST_PAGE_XAML: &str = include_str!("../../ui/pages/TaskListPage.axaml");

/// The XAML document of the page that exports an instance as a modpack.
///
/// The page is the export wizard of MP-04: the type of the pack, the
/// information of its manifest and the files of the instance it carries.
#[allow(dead_code)]
const MODPACK_EXPORT_PAGE_XAML: &str = include_str!("../../ui/pages/ModpackExportPage.axaml");

/// Loads the main window from its XAML document.
///
/// When the document cannot be loaded, the error is logged and the returned
/// window shows the message instead, so the launcher reports the problem in a
/// window rather than on a console that a desktop user does not see.
pub fn create_main_window(launcher: &Launcher) -> Ref<Window> {
    match load_main_window() {
        Ok(window) => {
            fill_texts(&window, launcher);

            // The page of the tasks is the page the launcher starts with; the
            // other pages take its place when the user asks for them.
            if let Some(page) = create_task_list_page(launcher) {
                set_content(&window, &page);
            }

            restore_window_bounds(&window, launcher);
            save_window_bounds_on_close(&window);

            window
        }
        Err(error) => {
            logging::error(&describe(&error));
            error_window(&describe(&error))
        }
    }
}

/// Puts the window in the rectangle the player left it in last.
///
/// The rectangle is the one of the state of the launcher (APP-07): a window
/// the player closed on another screen is put where the player looks now,
/// and a window that is not on any screen is put in the middle of the primary
/// one. A window that the player never moved is put in the middle too.
fn restore_window_bounds(window: &Ref<Window>, launcher: &Launcher) {
    let screens = window.screens();

    // The primary screen is the one the launcher puts the window in the
    // middle of; a platform that names none leaves the window where the
    // platform put it.
    let Some(primary) = screens.primary() else {
        return;
    };

    let primary_bounds = primary.bounds();
    let primary_size = (
        f64::from(primary_bounds.width),
        f64::from(primary_bounds.height),
    );
    let all_screens: Vec<ScreenBounds> = screens
        .all()
        .iter()
        .map(|screen| {
            let bounds = screen.bounds();
            ScreenBounds {
                min_x: f64::from(bounds.x),
                min_y: f64::from(bounds.y),
                max_x: f64::from(bounds.x + bounds.width),
                max_y: f64::from(bounds.y + bounds.height),
            }
        })
        .collect();

    let state = launcher.launcher_state();
    let bounds = initial_content_bounds(&state, primary_size, &all_screens);
    drop(state);

    // The rectangle of the state is in the logical units of the window, and
    // the position of the window is in the physical pixels of the screen.
    let scaling = window.desktop_scaling();
    window.set_client_size(Size::new(bounds.width, bounds.height));
    window.set_position(PixelPoint::new(
        (bounds.x * scaling).round() as i32,
        (bounds.y * scaling).round() as i32,
    ));
}

/// Keeps the rectangle the window is closed in.
///
/// The rectangle is only kept when the window is not maximized: the player
/// wants the window to come back the size it was before it was maximized,
/// which is the way the launcher of Java treats the state.
fn save_window_bounds_on_close(window: &Ref<Window>) {
    let Some(primary) = window.screens().primary() else {
        return;
    };
    let primary_bounds = primary.bounds();
    let primary_size = (
        f64::from(primary_bounds.width),
        f64::from(primary_bounds.height),
    );

    // The handler outlives this function, so it names the window weakly and
    // asks the launcher of the process for the state rather than keeping a
    // reference to it.
    let weak = window.downgrade();
    window.closing(move |_| {
        let Some(window) = weak.upgrade() else {
            return;
        };

        // A window that is maximized has the size of the screen, which the
        // launcher does not keep: the player would find the window in the
        // middle of the screen the next time, not maximized.
        if window.window_state() == WindowState::Maximized {
            return;
        }

        let size = window.client_size();
        let position = window.position();
        let scaling = window.desktop_scaling();
        let bounds = ContentBounds {
            x: f64::from(position.x) / scaling,
            y: f64::from(position.y) / scaling,
            width: size.width,
            height: size.height,
        };

        let launcher = crate::context::launcher();
        launcher.update_launcher_state(|state| state.record_content_bounds(&bounds, primary_size));
        if let Err(error) = launcher.save_state() {
            logging::warn(&format!(
                "The state of the launcher cannot be written: {error}"
            ));
        }
    });
}

/// Loads `MainWindow.xaml` with the run-time loader.
fn load_main_window() -> Result<Ref<Window>, XamlLoadException> {
    FerroRuntimeXamlLoader::parse_as::<Ref<Window>>(MAIN_WINDOW_XAML, None)
}

/// Loads the page that shows the tasks of the launcher.
///
/// The page is the one of the C# version, converted to the XAML of FerroUI;
/// it is filled with the view model of the page.
pub fn create_task_list_page(_launcher: &Launcher) -> Option<Ref<UserControl>> {
    let page = match FerroRuntimeXamlLoader::parse_as::<Ref<UserControl>>(TASK_LIST_PAGE_XAML, None)
    {
        Ok(page) => page,
        Err(error) => {
            logging::error(&describe(&error));
            return None;
        }
    };

    let view_model = TaskListViewModel::new();
    view_model.set_tasks(Some(ItemsSource::from(Vec::<BoxedValue>::new())));
    page.set_data_context(Some(data_context(&view_model)));

    Some(page)
}

/// Creates the page that exports an instance as a modpack.
///
/// The page shows the three steps of the wizard --- the type of the pack, the
/// information of its manifest and the files of the instance it carries ---
/// all bound to [`ModpackExportWizardViewModel`]. Finishing writes the archive
/// with the writer of the core. `game_version` and `components` are empty here
/// because the page is not yet attached to an instance; a wizard host fills
/// them in before it calls `write_export`.
#[allow(dead_code)]
pub fn create_export_page(run_directory: &Path, launcher: &Launcher) -> Option<Ref<UserControl>> {
    let page = match FerroRuntimeXamlLoader::parse_as::<Ref<UserControl>>(
        MODPACK_EXPORT_PAGE_XAML,
        None,
    ) {
        Ok(page) => page,
        Err(error) => {
            logging::error(&describe(&error));
            return None;
        }
    };

    let view_model = ModpackExportWizardViewModel::new();
    page.set_data_context(Some(data_context_for(&view_model)));

    // The page is loaded before it is put into the window, so the resources
    // of the launcher are given to it directly; otherwise the texts of the
    // page would be empty.
    page.set_resources(crate::resources::create(launcher.i18n()));

    // The type the player chose decides the format the archive is written in.
    for (name, format) in [
        ("TypeMcbbs", "mcbbs"),
        ("TypeServer", "server"),
        ("TypeModrinth", "modrinth"),
    ] {
        if let Some(radio) = NameScopeExtensions::find::<Button>(&page, name) {
            if let Some(clickable) = as_clickable_control(&radio.upcast::<FerroObject>()) {
                let vm = view_model.clone();
                clickable.click(Rc::new(move |_| {
                    vm.set_type_name(format);
                }));
            }
        }
    }

    // The player checks or unchecks every entry of the instance at once.
    for (name, included) in [("SelectAllButton", true), ("SelectNoneButton", false)] {
        if let Some(button) = NameScopeExtensions::find::<Button>(&page, name) {
            if let Some(clickable) = as_clickable_control(&button.upcast::<FerroObject>()) {
                let vm = view_model.clone();
                clickable.click(Rc::new(move |_| {
                    vm.set_all_included(included);
                }));
            }
        }
    }

    // Finishing writes the archive: the player named the files and the path,
    // so the writer of the core only needs them. The run directory is owned by
    // the closure, because a click handler may outlive this function.
    let run_directory = run_directory.to_path_buf();
    if let Some(button) = NameScopeExtensions::find::<Button>(&page, "FinishButton") {
        if let Some(clickable) = as_clickable_control(&button.upcast::<FerroObject>()) {
            let vm = view_model.clone();
            let status = NameScopeExtensions::find::<TextBlock>(&page, "FinishStatus");
            clickable.click(Rc::new(move |_| {
                let outcome = vm.write_export(&run_directory, "", &[]);
                if let Some(status) = &status {
                    match outcome {
                        Ok(outcome) => status.set_text(Some(&format!(
                            "wrote {} files to {}",
                            outcome.files.len(),
                            outcome.archive.display()
                        ))),
                        Err(error) => status.set_text(Some(&format!("cannot export: {error}"))),
                    }
                }
            }));
        }
    }

    Some(page)
}

/// Puts the view model into the value the bindings read.
#[allow(dead_code)]
fn data_context_for(view_model: &Ref<ModpackExportWizardViewModel>) -> BoxedValue {
    Rc::new(view_model.clone().upcast::<FerroObject>())
}

/// Puts the texts of the launcher into the elements of the window.
///
/// The elements that are not in the document are left alone, so the window
/// still shows something when the document changes.
fn fill_texts(window: &Ref<Window>, launcher: &Launcher) {
    let i18n = launcher.i18n();

    set_text(window, "Subtitle", hmcl_core::metadata::TITLE);
    set_text(window, "Status", &status(launcher));
    set_text(window, "Welcome", &i18n.i18n("game"));
    set_text(window, "Description", &i18n.i18n("instance.launch"));
    set_text(window, "Hint", &i18n.i18n("help"));

    set_item_text(window, "NavGames", &i18n.i18n("game"));
    set_item_text(window, "NavDownload", &i18n.i18n("download"));
    set_item_text(window, "NavAccounts", &i18n.i18n("account"));
    set_item_text(window, "NavSettings", &i18n.i18n("settings"));
    set_item_text(window, "NavAbout", &i18n.i18n("about"));
}

/// Returns the text of the line that tells the state of the launcher.
fn status(launcher: &Launcher) -> String {
    let language = launcher
        .launcher_settings()
        .language
        .clone()
        .unwrap_or_else(|| launcher.locale().code().to_string());

    format!(
        "{} · {language} · {}",
        hmcl_core::metadata::VERSION,
        launcher.user_home().display()
    )
}

/// Puts the view model into the value the bindings read.
fn data_context(view_model: &Ref<TaskListViewModel>) -> BoxedValue {
    Rc::new(view_model.clone().upcast::<FerroObject>())
}

/// Sets the text of an element of the window.
fn set_text(window: &Ref<Window>, name: &str, text: &str) {
    if let Some(element) = NameScopeExtensions::find::<TextBlock>(window, name) {
        element.set_text(Some(text));
    }
}

/// Puts a page into the window.
fn set_content(window: &Ref<Window>, page: &Ref<UserControl>) {
    if let Some(element) = NameScopeExtensions::find::<ContentControl>(window, "Content") {
        element.set_content(Some(Control::boxed(page.clone())));
    }
}

/// Sets the text of an item of the list of the window.
fn set_item_text(window: &Ref<Window>, name: &str, text: &str) {
    if let Some(element) = NameScopeExtensions::find::<ListBoxItem>(window, name) {
        element.set_content(Some(Rc::new(text.to_string())));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui::controls::UserControl;

    /// The page whose view models the Rust version has.
    const PORTED: [&str; 1] = [include_str!("../../ui/pages/TaskListPage.axaml")];

    /// The pages that are converted from the XAML of the C# version.
    const PAGES: [&str; 5] = [
        include_str!("../../ui/pages/TaskListPage.axaml"),
        include_str!("../../ui/pages/InstallerListPage.axaml"),
        include_str!("../../ui/pages/BlueprintListPage.axaml"),
        include_str!("../../ui/pages/GameInstancePage.axaml"),
        include_str!("../../ui/pages/InstanceFoldersPage.axaml"),
    ];

    /// Returns the errors of the message of the loader.
    ///
    /// The errors are written in parentheses, one after the other.
    fn errors(message: &str) -> Vec<&str> {
        let mut errors = Vec::new();
        let mut rest = message;

        while let Some(index) = rest.find('(') {
            let end = rest[index..].find(')').map(|end| index + end);
            let error = match end {
                Some(end) => &rest[index + 1..end],
                None => &rest[index + 1..],
            };
            errors.push(error);
            rest = match end {
                Some(end) => &rest[end + 1..],
                None => "",
            };
        }

        errors
    }

    #[test]
    fn the_types_of_the_crate_are_declared_to_the_loader() {
        crate::types::register();

        assert!(ferroui::base::TypeInfo::find("HMCL.ViewModels", "TaskListViewModel").is_some());
        assert!(ferroui::base::TypeInfo::find("HMCL.ViewModels", "TaskItemViewModel").is_some());
        assert!(
            ferroui::base::TypeInfo::find("HMCL.ViewModels", "ModpackExportWizardViewModel")
                .is_some()
        );
        assert!(
            ferroui::base::TypeInfo::find("HMCL.ViewModels", "ModpackFileEntryViewModel").is_some()
        );
    }

    #[test]
    fn the_converted_pages_are_loaded() {
        crate::types::register();

        for page in PORTED {
            if let Err(error) = FerroRuntimeXamlLoader::parse_as::<Ref<UserControl>>(page, None) {
                panic!("the page cannot be loaded: {}", describe(&error));
            }
        }
    }

    #[test]
    fn the_pages_that_are_not_ported_yet_only_name_the_types_of_the_csharp_version() {
        for page in PAGES {
            if let Err(error) = FerroRuntimeXamlLoader::parse_as::<Ref<UserControl>>(page, None) {
                let message = describe(&error);
                let errors = errors(&message);

                assert!(!errors.is_empty(), "the page cannot be loaded: {message}");

                for error in errors {
                    // The pages of the C# version name the types of the C#
                    // version, which the Rust version does not have yet, and
                    // the handlers of the events of the code behind, which
                    // the Rust version sets in code. Every other failure is a
                    // failure of the conversion.
                    assert!(
                        error.starts_with("Unable to resolve type ")
                            || error.starts_with(
                                "Unable to find suitable setter or adder for property "
                            ),
                        "the page cannot be loaded: {error}"
                    );
                }
            }
        }
    }
}
