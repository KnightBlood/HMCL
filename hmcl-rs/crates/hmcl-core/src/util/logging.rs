//! Logging to the standard error stream.
//!
//! The launcher writes its diagnostics to `stderr` so that the standard output
//! of a child process stays readable. A later version can route the same
//! records to a log file without changing the call sites.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Severity of a log record, in increasing order of importance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Diagnostic detail, disabled unless the launcher is started with
    /// verbosity.
    Debug,
    /// Normal progress information.
    Info,
    /// A recoverable problem.
    Warn,
    /// A failure the user should notice.
    Error,
}

static VERBOSE: OnceLock<bool> = OnceLock::new();

/// Enables or disables the debug records.
///
/// Later calls are ignored; the first call decides for the lifetime of the
/// process.
pub fn set_verbose(enabled: bool) {
    let _ = VERBOSE.set(enabled);
}

/// Returns whether debug records are enabled.
pub fn verbose() -> bool {
    *VERBOSE.get().unwrap_or(&false)
}

/// Writes a record with `level`, if the level is enabled.
pub fn log(level: Level, message: &str) {
    if level == Level::Debug && !verbose() {
        return;
    }

    eprintln!("[{level}] {} {message}", timestamp());
}

/// Writes an informational record.
pub fn info(message: &str) {
    log(Level::Info, message);
}

/// Writes a warning record.
pub fn warn(message: &str) {
    log(Level::Warn, message);
}

/// Writes an error record.
pub fn error(message: &str) {
    log(Level::Error, message);
}

/// Writes a debug record.
pub fn debug(message: &str) {
    log(Level::Debug, message);
}

/// Returns the current wall clock time as `HH:MM:SS`.
fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    let seconds_of_day = seconds % 86_400;
    format!(
        "{:02}:{:02}:{:02}",
        seconds_of_day / 3_600,
        (seconds_of_day % 3_600) / 60,
        seconds_of_day % 60
    )
}

impl std::fmt::Display for Level {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        };
        formatter.write_str(name)
    }
}
