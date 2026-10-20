//! The HMCL desktop application.
//!
//! `main` starts the FerroUI application and the classic desktop lifetime
//! ends it when the main window closes. The window is described in XAML and
//! loaded by [`ui`], so the application class stays free of layout code.

mod app;
mod ui;

use ferroui::prelude::*;
use hmcl_core::metadata;
use hmcl_core::util::logging;
use hmcl_core::util::operating_system;

fn main() -> std::process::ExitCode {
    logging::info(&format!(
        "*** {} ({}) ***",
        metadata::TITLE,
        metadata::VERSION
    ));
    logging::info(&format!(
        "Operating System: {}",
        operating_system::current_os()
    ));

    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = AppBuilder::configure::<app::HmclApplication>()
        .use_platform_detect()
        .start_with_classic_desktop_lifetime(&args);

    logging::info(&format!(
        "The main loop ended with the exit code {exit_code}"
    ));
    std::process::ExitCode::from(exit_code as u8)
}
