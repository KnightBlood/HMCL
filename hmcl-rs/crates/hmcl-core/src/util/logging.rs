//! Logging to the standard error stream and to a log file.
//!
//! The launcher writes its diagnostics to `stderr` so that the standard output
//! of a child process stays readable, and to a log file, so the user can
//! send the log to the developers when something goes wrong.
//!
//! The log file of every start of the launcher is a file of its own, so the
//! launcher never overwrites the log of an earlier start. The old files are
//! removed when there are more of them than the user allows.

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::error::Error;
use crate::util::datetime::DateTime;

/// The extension of the files that hold a log.
pub const LOG_EXTENSION: &str = "log";

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

/// The file the records are written to.
struct Sink {
    /// The path of the file.
    path: PathBuf,
    /// The writer of the file.
    writer: BufWriter<File>,
}

/// The state of the logger.
#[derive(Default)]
struct State {
    /// The file the records are written to, if there is one.
    sink: Option<Sink>,
    /// The number of the log files that are kept.
    retention: usize,
    /// The tokens that are hidden from the log.
    tokens: Vec<String>,
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();
static VERBOSE: OnceLock<bool> = OnceLock::new();

/// Returns the state of the logger.
fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State::default()))
}

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

/// Starts a log file in the folder.
///
/// The file is named after the moment the launcher was started, so the log of
/// an earlier start is never overwritten. The records are written to the file
/// from now on.
pub fn start_log(folder: &Path) -> Result<PathBuf, Error> {
    fs::create_dir_all(folder).map_err(|error| Error::io(folder, error))?;

    let moment = DateTime::now();

    for number in 0..1000 {
        let path = log_file_path(folder, &moment, number);

        // The file is created at once, so two loggers that start in the same
        // second do not use the same file.
        let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(Error::io(&path, error)),
        };

        let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
        state.sink = Some(Sink {
            path: path.clone(),
            writer: BufWriter::new(file),
        });

        return Ok(path);
    }

    Err(Error::io(
        folder,
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "there are too many log files in the folder",
        ),
    ))
}

/// Returns the path of the file of a log in the folder.
///
/// The name of the file holds the moment the launcher was started; a number
/// is added to the name when a file of that name is already there.
fn log_file_path(folder: &Path, moment: &DateTime, number: usize) -> PathBuf {
    let name = moment.file_name();
    let file_name = if number == 0 {
        format!("{name}.{LOG_EXTENSION}")
    } else {
        format!("{name}.{number}.{LOG_EXTENSION}")
    };

    folder.join(file_name)
}

/// Returns the path of the log file, if there is one.
pub fn log_file() -> Option<PathBuf> {
    let state = state().lock().unwrap_or_else(|error| error.into_inner());
    state.sink.as_ref().map(|sink| sink.path.clone())
}

/// Sets the number of the log files that are kept.
///
/// The old files are removed when the launcher stops. A number of zero means
/// that every file is kept.
pub fn set_log_retention(retention: usize) {
    let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
    state.retention = retention;
}

/// Removes the log files that are older than the ones the user allows.
///
/// The files are named after the moment they were written, so the name of a
/// file tells how old it is.
pub fn clean_old_logs(folder: &Path, retention: usize) -> Result<(), Error> {
    if retention == 0 {
        return Ok(());
    }

    let mut files = Vec::new();
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io(folder, error)),
    };

    for entry in entries {
        let entry = entry.map_err(|error| Error::io(folder, error))?;
        let path = entry.path();

        if !is_log_file(&path) {
            continue;
        }

        files.push(path);
    }

    if files.len() <= retention {
        return Ok(());
    }

    // The names of the files hold the moment they were written, so the
    // files are ordered by their names.
    files.sort();

    let current = log_file();
    for path in files.iter().take(files.len() - retention) {
        if current.as_ref() == Some(path) {
            continue;
        }
        fs::remove_file(path).map_err(|error| Error::io(path, error))?;
    }

    Ok(())
}

/// Returns whether the file holds a log.
fn is_log_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == LOG_EXTENSION)
        && path.file_stem().is_some_and(|stem| {
            stem.to_string_lossy()
                .starts_with(|c: char| c.is_ascii_digit())
        })
}

/// Copies the log to a file, so the user can send it to the developers.
///
/// When there is no log file, the records that were written to the standard
/// error stream are not kept, so the method fails.
pub fn export_log(destination: &Path) -> Result<(), Error> {
    let source = log_file()
        .ok_or_else(|| Error::invalid_format(destination, "there is no log file to export"))?;

    fs::copy(&source, destination).map_err(|error| Error::io(destination, error))?;
    Ok(())
}

/// Hides a token from the log.
///
/// The launcher tells the servers of the game the tokens of the accounts, so
/// the tokens are not written to the log.
pub fn register_access_token(token: &str) {
    if token.len() <= 1 {
        return;
    }

    let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
    if !state.tokens.iter().any(|known| known == token) {
        state.tokens.push(token.to_string());
    }
}

/// Removes the tokens that must not be seen from a text.
pub fn filter_forbidden_token(message: &str) -> String {
    let state = state().lock().unwrap_or_else(|error| error.into_inner());
    if state.tokens.is_empty() {
        return message.to_string();
    }

    let mut filtered = message.to_string();
    for token in &state.tokens {
        filtered = filtered.replace(token.as_str(), "<access token>");
    }
    filtered
}

/// Writes a record with `level`, if the level is enabled.
pub fn log(level: Level, message: &str) {
    log_with_caller(level, None, message);
}

/// Writes a record with `level` and the name of the place it was written in.
pub fn log_with_caller(level: Level, caller: Option<&str>, message: &str) {
    if level == Level::Debug && !verbose() {
        return;
    }

    let message = filter_forbidden_token(message);
    let record = format_record(level, caller, &message);

    eprintln!("{record}");
    write_to_file(&record);
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

/// Returns the text of a record, such as `[12:34:56] [INFO] The launcher
/// started`.
fn format_record(level: Level, caller: Option<&str>, message: &str) -> String {
    let moment = DateTime::now();
    let source = match caller {
        Some(caller) => format!("{caller}/{level}"),
        None => level.to_string(),
    };

    format!("[{}] [{source}] {message}", moment)
}

/// Writes a record to the log file.
///
/// The file is written at once, so a record is not lost when the launcher
/// is stopped without ending the program.
fn write_to_file(record: &str) {
    let mut state = state().lock().unwrap_or_else(|error| error.into_inner());

    if let Some(sink) = state.sink.as_mut() {
        if let Err(error) = writeln!(sink.writer, "{record}") {
            eprintln!("[ERROR] The log file cannot be written: {error}");
        } else if let Err(error) = sink.writer.flush() {
            eprintln!("[ERROR] The log file cannot be written: {error}");
        }
    }
}

/// Writes the records that are still in the memory to the log file.
pub fn flush() {
    let mut state = state().lock().unwrap_or_else(|error| error.into_inner());

    if let Some(sink) = state.sink.as_mut() {
        let _ = sink.writer.flush();
    }
}

/// Stops the log file and removes the log files that are too old.
///
/// The method is called when the launcher stops, so the log of the last start
/// is complete and the old files are gone.
pub fn shutdown() -> Result<(), Error> {
    let (folder, retention) = {
        let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
        let folder = state
            .sink
            .as_ref()
            .and_then(|sink| sink.path.parent())
            .map(Path::to_path_buf);
        let retention = state.retention;

        if let Some(sink) = state.sink.as_mut() {
            let _ = sink.writer.flush();
        }

        (folder, retention)
    };

    if let Some(folder) = folder {
        if retention > 0 {
            clean_old_logs(&folder, retention)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a directory that is removed when the test ends.
    fn temporary_directory(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("hmcl-core-logging-{name}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the test directory should be created");
        path
    }

    #[test]
    fn the_records_are_written_in_the_format_of_the_java_launcher() {
        let record = format_record(Level::Info, Some("org.jackhuang.hmcl.Main"), "Started");
        assert!(record.ends_with("[org.jackhuang.hmcl.Main/INFO] Started"));
        assert!(record.starts_with('['));

        let record = format_record(Level::Error, None, "Failed");
        assert!(record.ends_with("[ERROR] Failed"));
    }

    #[test]
    fn the_tokens_are_hidden_from_the_records() {
        register_access_token("secret-token");

        let filtered = filter_forbidden_token("the token is secret-token");
        assert_eq!(filtered, "the token is <access token>");
    }

    #[test]
    fn the_log_files_are_named_after_the_moment_they_were_written() {
        let folder = temporary_directory("names");
        let moment = DateTime::new(2026, 1, 2, 3, 4, 5);

        let path = log_file_path(&folder, &moment, 0);
        assert_eq!(path.file_name().unwrap(), "2026-01-02T03-04-05.log");

        // A second file of the same moment gets a number.
        let path = log_file_path(&folder, &moment, 1);
        assert_eq!(path.file_name().unwrap(), "2026-01-02T03-04-05.1.log");
    }

    #[test]
    fn the_log_file_is_created_and_written() {
        let folder = temporary_directory("write");

        let path = start_log(&folder).expect("the log file should be created");
        assert!(path.exists());
        assert_eq!(log_file(), Some(path.clone()));

        info("the launcher started");
        flush();

        let content = fs::read_to_string(&path).expect("the log file should be read");
        assert!(content.contains("the launcher started"));
        assert!(content.contains("INFO"));
    }

    #[test]
    fn the_old_log_files_are_removed() {
        let folder = temporary_directory("retention");

        for name in [
            "2026-01-01T00-00-00.log",
            "2026-01-02T00-00-00.log",
            "2026-01-03T00-00-00.log",
            "not-a-log.txt",
        ] {
            fs::write(folder.join(name), "record").expect("the file should be written");
        }

        clean_old_logs(&folder, 2).expect("the old files should be removed");

        assert!(!folder.join("2026-01-01T00-00-00.log").exists());
        assert!(folder.join("2026-01-02T00-00-00.log").exists());
        assert!(folder.join("2026-01-03T00-00-00.log").exists());
        assert!(folder.join("not-a-log.txt").exists());
    }
}
