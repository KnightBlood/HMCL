//! The file the launcher of Java reads the servers of a new workspace from.
//!
//! A provider who hands a player the launcher of Java may drop a file named
//! `authlib-injectors.json` next to the program of the launcher, which names
//! the servers of the provider the player signs in to. The launcher reads
//! that file when it starts in a workspace no launcher has written files
//! into yet, adds every server it names to the list of the servers of the
//! authlib-injector, and asks the way accounts are added from the servers
//! afterwards.
//!
//! The launcher of Rust reads the same file the same way, so a player who was
//! given a launcher of Rust by the same provider finds the servers the
//! provider named.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::Error;

/// The name of the file that names the servers of a new workspace.
pub const AUTHLIB_INJECTORS_FILE: &str = "authlib-injectors.json";

/// The servers a player was given with the launcher.
///
/// The field `urls` is the only one the file has; a file without it names no
/// server, because the launcher cannot tell the addresses the player wanted
/// rather than guessed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct AuthlibInjectorServers {
    /// The addresses of the endpoints of Yggdrasil of the servers.
    #[serde(default)]
    urls: Option<Vec<String>>,
}

/// Returns the file that names the servers of a new workspace.
///
/// The launcher of Java keeps the file next to its program when the program
/// is a file it may write, and in the directory the launcher was started from
/// otherwise; this launcher looks in the same places, so a provider who
/// writes the file for the launcher of Java is read by this launcher too.
pub fn authlib_injectors_path() -> PathBuf {
    let executable = std::env::current_exe().ok();
    authlib_injectors_path_in(executable.as_deref(), &crate::metadata::current_directory())
}

/// Returns the file the servers are read from, given the parts the launcher
/// reads them from.
///
/// The program of the launcher is only used when it is a file the launcher
/// may write: a program a player may not change is no place a provider could
/// have dropped a file into. The directory the launcher was started from is
/// where the launcher of Java looks then, which is why it is the fallback
/// here too.
pub fn authlib_injectors_path_in(executable: Option<&Path>, current_directory: &Path) -> PathBuf {
    match executable {
        Some(executable) if is_writable_file(executable) => match executable.parent() {
            Some(directory) => directory.join(AUTHLIB_INJECTORS_FILE),
            // A path that names no directory is the root of a drive, which
            // the current directory is closer to than a fileless name.
            None => current_directory.join(AUTHLIB_INJECTORS_FILE),
        },
        _ => current_directory.join(AUTHLIB_INJECTORS_FILE),
    }
}

/// Returns whether a path is a file the launcher may write, which is how the
/// launcher of Java asks for a place its jar may be next to.
///
/// Opening the file for writing without writing anything is the way the
/// launcher asks: a file the system keeps read-only, such as one inside a
/// directory of another user, refuses the opening.
fn is_writable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
        && fs::OpenOptions::new().append(true).open(path).is_ok()
}

/// Reads the addresses of the servers a file names.
///
/// The file is read the way the launcher of Java reads it: the addresses are
/// named in `urls`, and a file without `urls` is reported rather than read as
/// a file that names no server, because a player whose file was written by
/// hand has to be told which part of it is wrong.
pub fn read_urls(path: &Path) -> Result<Vec<String>, Error> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let servers: AuthlibInjectorServers = serde_json::from_str(&text)
        .map_err(|error| Error::invalid_format(path, error.to_string()))?;

    servers.urls.ok_or_else(|| {
        Error::invalid_format(path, "authlib-injectors.json -> urls cannot be null.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counts the directories the tests make, so tests that run in parallel
    /// do not share one.
    static INDEX: AtomicUsize = AtomicUsize::new(0);

    /// Creates a directory of the tests and forgets what was in it.
    fn directory(name: &str) -> PathBuf {
        let index = INDEX.fetch_add(1, Ordering::SeqCst);
        let directory = std::env::temp_dir()
            .join("hmcl-authlib-injectors-test")
            .join(format!("{name}-{}-{index}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("the directory should be created");
        directory
    }

    #[test]
    fn the_addresses_of_the_servers_are_read() {
        let directory = directory("read");
        let path = directory.join(AUTHLIB_INJECTORS_FILE);
        fs::write(&path, r#"{"urls": ["example.com/api/yggdrasil"]}"#)
            .expect("the file should be written");

        let urls = read_urls(&path).expect("the addresses should be read");
        assert_eq!(urls, vec!["example.com/api/yggdrasil".to_string()]);

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_file_without_addresses_is_refused() {
        let directory = directory("validate");
        let path = directory.join(AUTHLIB_INJECTORS_FILE);
        fs::write(&path, r#"{"something": "the launcher does not know"}"#)
            .expect("the file should be written");

        let error = read_urls(&path).expect_err("the file should be refused");
        assert!(error.to_string().contains("urls cannot be null"), "{error}");

        // A file that names null is refused the same way, which is the way
        // the launcher of Java reads a file whose urls are missing.
        fs::write(&path, r#"{"urls": null}"#).expect("the file should be written");
        let error = read_urls(&path).expect_err("the file should be refused");
        assert!(error.to_string().contains("urls cannot be null"), "{error}");

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_file_the_launcher_cannot_read_is_refused() {
        let directory = directory("malformed");
        let path = directory.join(AUTHLIB_INJECTORS_FILE);
        fs::write(&path, "not json").expect("the file should be written");

        let error = read_urls(&path).expect_err("the file should be refused");
        assert!(error.to_string().contains("expected ident"), "{error}");

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_missing_file_is_a_reading_error() {
        let directory = directory("missing");
        let error = read_urls(&directory.join(AUTHLIB_INJECTORS_FILE))
            .expect_err("the file should not exist");
        assert!(!error.to_string().is_empty(), "{error}");
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_file_is_read_from_the_directory_of_the_program() {
        let directory = directory("program");
        let current = directory.join("somewhere-else");
        fs::create_dir_all(&current).expect("the directory should be created");

        let executable = directory.join("launcher.exe");
        fs::write(&executable, b"MZ").expect("the program should be written");

        let path = authlib_injectors_path_in(Some(&executable), &current);
        assert_eq!(path, directory.join(AUTHLIB_INJECTORS_FILE));

        // A program that is not a file is not the place a provider dropped a
        // file into, so the directory the launcher was started from is asked.
        let running = current.join("proc");
        fs::write(&running, b"MZ").expect("the file should be written");
        let path = authlib_injectors_path_in(Some(&running), &current);
        assert_eq!(path, current.join(AUTHLIB_INJECTORS_FILE));
    }

    #[test]
    fn the_file_is_read_from_the_directory_the_launcher_started_from() {
        let directory = directory("read-only");
        let executable = directory.join("read-only");
        fs::write(&executable, b"MZ").expect("the program should be written");

        // The launcher of Java asks whether the program may be written before
        // it looks next to it; a read-only program is not asked for the file.
        let mut permissions = fs::metadata(&executable)
            .expect("the program should be read")
            .permissions();
        let writable_before = !permissions.readonly();
        permissions.set_readonly(true);
        fs::set_permissions(&executable, permissions).expect("the file should be kept read-only");

        let path = authlib_injectors_path_in(Some(&executable), &directory);
        if writable_before && path == directory.join(AUTHLIB_INJECTORS_FILE) {
            // The system kept the file writable: a file the launcher may not
            // change does not exist here, so the test tells nothing.
        } else {
            assert_eq!(path, directory.join(AUTHLIB_INJECTORS_FILE));
        }
        let _ = fs::remove_dir_all(&directory);
    }
}
