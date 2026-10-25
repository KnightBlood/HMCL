//! The HMCL desktop application.
//!
//! `main` starts the FerroUI application and the classic desktop lifetime
//! ends it when the main window closes. The window is described in XAML and
//! loaded by [`ui`], so the application class stays free of layout code.

mod app;
mod context;
mod converters;
mod http_client;
mod http_fetcher;
mod models;
mod oauth_server;
mod resources;
mod types;
mod ui;
mod view_models;

use std::path::PathBuf;

use ferroui::prelude::*;
use hmcl_core::metadata;
use hmcl_core::util::logging;
use hmcl_core::util::operating_system;

use crate::context::{launcher, Launcher};

fn main() -> std::process::ExitCode {
    types::register();

    let launcher = launcher();

    logging::info(&format!(
        "*** {} ({}) ***",
        metadata::TITLE,
        metadata::VERSION
    ));
    logging::info(&format!(
        "Operating System: {}",
        operating_system::current_os()
    ));
    logging::info(&format!("Language: {}", launcher.locale()));

    if let Err(error) = start_log(launcher) {
        logging::warn(&format!("The log file cannot be created: {error}"));
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = AppBuilder::configure::<app::HmclApplication>()
        .use_platform_detect()
        .start_with_classic_desktop_lifetime(&args);

    logging::info(&format!(
        "The main loop ended with the exit code {exit_code}"
    ));

    if let Err(error) = launcher.save() {
        logging::warn(&format!("The settings cannot be written: {error}"));
    }

    if let Err(error) = logging::shutdown() {
        logging::warn(&format!("The log file cannot be closed: {error}"));
    }

    std::process::ExitCode::from(exit_code as u8)
}

/// Creates the log file of the launcher in the directory of the logs.
fn start_log(launcher: &Launcher) -> Result<(), hmcl_core::error::Error> {
    let folder: PathBuf = launcher.local_home().join("logs");
    let path = logging::start_log(&folder)?;

    // The user may choose how many of the old log files are kept.
    if let Some(retention) = launcher.user_settings().log_retention {
        logging::set_log_retention(retention as usize);
    }

    logging::info(&format!("The log file is {}", path.display()));
    Ok(())
}
