//! The application class of HMCL.
//!
//! The theme is installed here; the window itself is described by the XAML
//! document in `ui/MainWindow.xaml` and loaded by the run-time loader, so this
//! class holds no layout code.

use ferroui::prelude::*;
use hmcl_core::metadata;

use crate::ui;

/// The FerroUI application of HMCL.
///
/// It installs the Fluent theme and creates the main window once the
/// framework has finished initialising. The state of the launcher lives in
/// `hmcl-core`; this class only owns the window.
#[repr(C)]
pub struct HmclApplication {
    base: Application,
}

ferro_class!(HmclApplication: Application);
ferro_impl_classes!(HmclApplication: FerroObjectImpl);

impl NewApplication for HmclApplication {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}

impl ApplicationImpl for HmclApplication {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.set_name(Some(metadata::TITLE.to_string()));
        this.styles().add(FluentTheme::new().as_style());
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime
            .as_ref()
            .and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
        {
            desktop.set_main_window(Some(ui::create_main_window()));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}
