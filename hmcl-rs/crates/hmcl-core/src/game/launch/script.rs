//! The scripts the launcher writes so a version of the game may be started
//! without the launcher.
//!
//! A player who wants to start the game from a shortcut, or from a script of
//! their own, asks the launcher to write the command of the game into a file.
//! The launcher of Java writes three kinds of scripts, so the same three are
//! written here:
//!
//! * a batch file (`.bat`) on Windows,
//! * a PowerShell script (`.ps1`) on Windows,
//! * a shell script (`.sh`) on the systems of Unix.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::util::operating_system::OsType;

use super::command::LaunchCommand;
use super::options::LaunchOptions;

/// The kinds of script the launcher writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    /// A batch file of Windows.
    Batch,
    /// A script of PowerShell of Windows.
    PowerShell,
    /// A script of the shell of the systems of Unix.
    Shell,
}

impl ScriptKind {
    /// Reads the kind of a script from the extension of a file.
    ///
    /// The launcher of Java reads the extension of the file the player chose,
    /// so a player who names the file `.bat` gets a batch file even on the
    /// systems of Unix. The extensions are the ones the launcher of Java
    /// reads.
    pub fn of_extension(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "bat" => Some(Self::Batch),
            "ps1" => Some(Self::PowerShell),
            "sh" | "bash" | "command" => Some(Self::Shell),
            _ => None,
        }
    }

    /// Returns the kind of the script a file is written as.
    ///
    /// A `.ps1` file is a script of PowerShell; every other file is a script
    /// of the shell.
    pub fn of_path(path: &Path) -> Self {
        path.extension()
            .and_then(|extension| extension.to_str())
            .and_then(Self::of_extension)
            .unwrap_or(Self::Shell)
    }
}

impl fmt::Display for ScriptKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Batch => "bat",
            Self::PowerShell => "ps1",
            Self::Shell => "sh",
        })
    }
}

/// The error of writing a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// The extension of the file names a kind of script the system of the
    /// player does not run.
    UnsupportedKind(String),
    /// The command of the game is longer than a batch file of Windows holds.
    CommandTooLong(usize),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedKind(extension) => write!(
                formatter,
                "The extension of the script is neither 'bat' nor 'ps1': {extension}"
            ),
            Self::CommandTooLong(length) => write!(
                formatter,
                "The command of the game is {length} characters long, which a batch file of Windows does not hold"
            ),
        }
    }
}

impl std::error::Error for ScriptError {}

/// The longest command a batch file of Windows holds.
///
/// A batch file of Windows reads a line of no more than 8191 characters, and
/// the command of the launcher of Java is refused when it is longer than
/// 32767 characters, which is the limit the interpreter of the command line
/// reads.
pub const MAX_BATCH_COMMAND_LENGTH: usize = 32767;

/// Writes the command of the game into a script file.
///
/// The script is the one the launcher of Java writes for the same file name,
/// so a script written by either launcher runs the same game with the same
/// settings. The directories of the file are created when they do not exist
/// yet, so a player may name a file in a directory that was not made yet.
pub fn write_script(
    path: impl AsRef<Path>,
    plan_command: &LaunchCommand,
    options: &LaunchOptions,
    working_directory: &Path,
    java_path: &Path,
) -> Result<PathBuf, ScriptError> {
    let path = path.as_ref();
    let kind = ScriptKind::of_path(path);

    // The launcher of Java refuses a kind of script the system of the player
    // does not run, so a player who names the file `.bat` on Linux is told
    // rather than given a script that does nothing.
    let supported = match (kind, options.os) {
        (ScriptKind::Batch, OsType::Windows) | (ScriptKind::PowerShell, OsType::Windows) => true,
        (ScriptKind::Shell, os) if os != OsType::Windows => true,
        _ => false,
    };
    if !supported {
        return Err(ScriptError::UnsupportedKind(kind.to_string()));
    }

    let command_line = script_command(plan_command, options, java_path);

    if kind == ScriptKind::Batch && command_line.chars().count() > MAX_BATCH_COMMAND_LENGTH {
        return Err(ScriptError::CommandTooLong(command_line.chars().count()));
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    let text = script_text(kind, &command_line, options, working_directory);
    std::fs::write(path, text.as_bytes())
        .map_err(|error| ScriptError::UnsupportedKind(error.to_string()))?;
    set_executable(path);

    Ok(path.to_path_buf())
}

/// Returns the command of the game, with the program that runs it.
fn script_command(command: &LaunchCommand, options: &LaunchOptions, java_path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    match &options.wrapper {
        Some(wrapper) => parts.push(wrapper.clone()),
        None => parts.push(java_path.to_string_lossy().into_owned()),
    }
    parts.extend(command.arguments());

    let kind = if options.os == OsType::Windows {
        ScriptKind::Batch
    } else {
        ScriptKind::Shell
    };
    parts
        .into_iter()
        .map(|part| quote(kind, &part))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quotes a part of a command for a script.
fn quote(kind: ScriptKind, part: &str) -> String {
    match kind {
        ScriptKind::Batch => batch_quote(part),
        ScriptKind::PowerShell => power_shell_quote(part),
        ScriptKind::Shell => shell_quote(part),
    }
}

/// Quotes a part of a command for a batch file of Windows.
fn batch_quote(part: &str) -> String {
    if part
        .chars()
        .any(|c| matches!(c, ' ' | '\t' | '"' | '^' | '&' | '<' | '>' | '|'))
    {
        format!("\"{}\"", part.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        part.to_string()
    }
}

/// Quotes a part of a command for a script of PowerShell.
fn power_shell_quote(part: &str) -> String {
    format!("'{}'", part.replace('\'', "''"))
}

/// Quotes a part of a command for a script of the shell.
fn shell_quote(part: &str) -> String {
    if part.chars().any(|c| {
        matches!(
            c,
            ' ' | '\t'
                | '"'
                | '!'
                | '#'
                | '$'
                | '&'
                | '\''
                | '('
                | ')'
                | '*'
                | ','
                | ';'
                | '<'
                | '='
                | '>'
                | '?'
                | '['
                | '\\'
                | ']'
                | '^'
                | '`'
                | '{'
                | '|'
                | '}'
                | '~'
        )
    }) {
        format!(
            "\"{}\"",
            part.replace('"', "\\\"")
                .replace('$', "\\$")
                .replace('&', "\\&")
                .replace('`', "\\`")
        )
    } else {
        part.to_string()
    }
}

/// Returns the text of a script.
fn script_text(
    kind: ScriptKind,
    command_line: &str,
    options: &LaunchOptions,
    working_directory: &Path,
) -> String {
    let mut text = String::new();
    let working = working_directory.to_string_lossy();

    match kind {
        ScriptKind::Batch => {
            text.push_str("@echo off\n");
            if options.os == OsType::Windows {
                if let Some(appdata) = appdata_directory(options) {
                    text.push_str(&format!("set APPDATA={}\n", batch_quote(&appdata)));
                }
            }
            for (name, value) in &options.environment_variables {
                text.push_str(&format!("set {}={}\n", name, batch_quote(value)));
            }
            text.push('\n');
            text.push_str(&format!("cd /D {}\n", batch_quote(&working)));
            if let Some(command) = &options.pre_launch_command {
                text.push_str(&format!("{}\n", command));
            }
            text.push_str(command_line);
            text.push('\n');
            if let Some(command) = &options.post_exit_command {
                text.push_str(&format!("{}\n", command));
            }
            text.push_str("pause\n");
        }
        ScriptKind::PowerShell => {
            if options.os == OsType::Windows {
                if let Some(appdata) = appdata_directory(options) {
                    text.push_str(&format!("$Env:APPDATA={}\n", power_shell_quote(&appdata)));
                }
            }
            for (name, value) in &options.environment_variables {
                text.push_str(&format!("$Env:{}={}\n", name, power_shell_quote(value)));
            }
            text.push_str(&format!(
                "Set-Location -LiteralPath {}\n",
                power_shell_quote(&working)
            ));
            if let Some(command) = &options.pre_launch_command {
                text.push_str(&format!("& {}\n", power_shell_quote(command)));
            }
            text.push_str(&format!("& {}\n", command_line));
            if let Some(command) = &options.post_exit_command {
                text.push_str(&format!("& {}\n", power_shell_quote(command)));
            }
        }
        ScriptKind::Shell => {
            text.push_str("#!/usr/bin/env bash\n");
            for (name, value) in &options.environment_variables {
                text.push_str(&format!("export {}={}\n", name, shell_quote(value)));
            }
            text.push_str(&format!("cd {}\n", shell_quote(&working)));
            if let Some(command) = &options.pre_launch_command {
                text.push_str(&format!("{}\n", command));
            }
            text.push_str(command_line);
            text.push('\n');
            if let Some(command) = &options.post_exit_command {
                text.push_str(&format!("{}\n", command));
            }
        }
    }

    text
}

/// Returns the directory the launcher of Java keeps the files of the game of
/// the launcher of Mojang in, when the game runs on Windows.
fn appdata_directory(options: &LaunchOptions) -> Option<String> {
    if options.os != OsType::Windows {
        return None;
    }

    std::env::var_os("APPDATA").map(|path| PathBuf::from(path).to_string_lossy().into_owned())
}

/// Makes a file executable by every user of the machine.
fn set_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Ok(metadata) = std::fs::metadata(path) {
            let mut permissions = metadata.permissions();
            let mode = permissions.mode();
            permissions.set_mode(mode | 0o111);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::launch::{LaunchOptions, LaunchPaths};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn command() -> LaunchCommand {
        LaunchCommand {
            classpath: vec![PathBuf::from("libraries/library.jar")],
            main_class: "net.minecraft.client.main.Main".to_string(),
            jvm_arguments: vec!["-Xmx2G".to_string()],
            game_arguments: vec!["--width".to_string(), "854".to_string()],
        }
    }

    fn options(os: OsType) -> LaunchOptions {
        let mut environment_variables = BTreeMap::new();
        environment_variables.insert("MANGLE".to_string(), "1".to_string());

        LaunchOptions {
            os,
            environment_variables,
            pre_launch_command: None,
            post_exit_command: None,
            ..LaunchOptions::default()
        }
    }

    #[test]
    fn the_kinds_of_the_scripts_are_read_from_the_extensions() {
        assert_eq!(ScriptKind::of_extension("bat"), Some(ScriptKind::Batch));
        assert_eq!(ScriptKind::of_extension("BAT"), Some(ScriptKind::Batch));
        assert_eq!(
            ScriptKind::of_extension("ps1"),
            Some(ScriptKind::PowerShell)
        );
        assert_eq!(ScriptKind::of_extension("sh"), Some(ScriptKind::Shell));
        assert_eq!(ScriptKind::of_extension("bash"), Some(ScriptKind::Shell));
        assert_eq!(ScriptKind::of_extension("command"), Some(ScriptKind::Shell));
        assert_eq!(ScriptKind::of_extension("txt"), None);
        assert_eq!(
            ScriptKind::of_path(Path::new("game.bat")),
            ScriptKind::Batch
        );
        assert_eq!(
            ScriptKind::of_path(Path::new("game.PS1")),
            ScriptKind::PowerShell
        );
        assert_eq!(ScriptKind::of_path(Path::new("game.sh")), ScriptKind::Shell);
    }

    #[test]
    fn the_kinds_of_the_scripts_are_written_as_their_extensions() {
        assert_eq!(ScriptKind::Batch.to_string(), "bat");
        assert_eq!(ScriptKind::PowerShell.to_string(), "ps1");
        assert_eq!(ScriptKind::Shell.to_string(), "sh");
    }

    #[test]
    fn a_command_of_a_part_with_a_space_is_quoted() {
        assert_eq!(batch_quote("plain"), "plain");
        assert_eq!(batch_quote("with space"), "\"with space\"");
        assert_eq!(power_shell_quote("plain"), "'plain'");
        assert_eq!(power_shell_quote("it's"), "'it''s'");
        assert_eq!(shell_quote("plain"), "plain");
        assert_eq!(shell_quote("with space"), "\"with space\"");
        assert_eq!(shell_quote("a$b"), "\"a\\$b\"");
    }

    #[test]
    fn the_command_of_the_game_is_quoted_for_the_system() {
        let command = command();
        let windows = options(OsType::Windows);
        let script = script_command(&command, &windows, Path::new("/java/bin/java"));

        assert!(script.starts_with("/java/bin/java -Xmx2G"));
        assert!(script.contains("net.minecraft.client.main.Main"));
        assert!(script.contains("--width 854"));

        let linux = options(OsType::Linux);
        let script = script_command(&command, &linux, Path::new("/usr/bin/java"));
        assert!(script.contains("/usr/bin/java -Xmx2G"));
    }

    #[test]
    fn the_text_of_a_batch_file_holds_the_command() {
        let text = script_text(
            ScriptKind::Batch,
            "java -Xmx2G Main",
            &options(OsType::Windows),
            Path::new("C:\\game\\.minecraft"),
        );

        assert!(text.starts_with("@echo off\n"));
        assert!(text.contains("set MANGLE=1\n"));
        assert!(text.contains("cd /D C:\\game\\.minecraft"));
        assert!(text.contains("java -Xmx2G Main\n"));
        assert!(text.ends_with("pause\n"));
    }

    #[test]
    fn the_text_of_a_shell_script_holds_the_command() {
        let text = script_text(
            ScriptKind::Shell,
            "java -Xmx2G Main",
            &options(OsType::Linux),
            Path::new("/home/user/.minecraft"),
        );

        assert!(text.starts_with("#!/usr/bin/env bash\n"));
        assert!(text.contains("export MANGLE=1\n"));
        assert!(text.contains("cd /home/user/.minecraft\n"));
        assert!(text.contains("java -Xmx2G Main\n"));
        assert!(!text.contains("pause"));
    }

    #[test]
    fn the_text_of_a_script_of_power_shell_holds_the_command() {
        let text = script_text(
            ScriptKind::PowerShell,
            "java -Xmx2G Main",
            &options(OsType::Windows),
            Path::new("C:\\game\\.minecraft"),
        );

        assert!(text.contains("$Env:MANGLE='1'\n"));
        assert!(text.contains("Set-Location -LiteralPath 'C:\\game\\.minecraft'\n"));
        assert!(text.contains("& java -Xmx2G Main\n"));
        assert!(!text.contains("@echo off"));
    }

    #[test]
    fn the_commands_before_and_after_the_game_are_written() {
        let mut options = options(OsType::Linux);
        options.pre_launch_command = Some("echo before".to_string());
        options.post_exit_command = Some("echo after".to_string());

        let text = script_text(ScriptKind::Shell, "java Main", &options, Path::new("/game"));

        assert!(text.contains("echo before\n"));
        assert!(text.contains("java Main\n"));
        assert!(text.contains("echo after\n"));
    }

    #[test]
    fn a_kind_of_script_the_system_does_not_run_is_refused() {
        let directory = std::env::temp_dir().join("hmcl-script-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        // A batch file is refused on the systems of Unix.
        let error = write_script(
            directory.join("game.bat"),
            &command(),
            &options(OsType::Linux),
            Path::new("/game"),
            Path::new("/usr/bin/java"),
        )
        .expect_err("the batch file should be refused");
        assert_eq!(error, ScriptError::UnsupportedKind("bat".to_string()));

        // A shell script is refused on Windows.
        let error = write_script(
            directory.join("game.sh"),
            &command(),
            &options(OsType::Windows),
            Path::new("C:\\game"),
            Path::new("C:\\java\\java.exe"),
        )
        .expect_err("the shell script should be refused");
        assert_eq!(error, ScriptError::UnsupportedKind("sh".to_string()));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_script_is_written_for_the_system_of_the_player() {
        let directory = std::env::temp_dir().join("hmcl-script-test-write");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be created");

        let os = if cfg!(target_os = "windows") {
            OsType::Windows
        } else {
            OsType::Linux
        };

        let file = directory.join(if os == OsType::Windows {
            "game.bat"
        } else {
            "game.sh"
        });

        let written = write_script(
            &file,
            &command(),
            &options(os),
            Path::new("/game"),
            Path::new("/usr/bin/java"),
        )
        .expect("the script should be written");

        assert_eq!(written, file);
        let text = std::fs::read_to_string(&file).expect("the script should be read");
        assert!(text.contains("net.minecraft.client.main.Main"));
        assert!(text.contains("-Xmx2G"));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_command_that_is_too_long_for_a_batch_file_is_refused() {
        let mut command = command();
        command.game_arguments = vec!["a".repeat(40000)];

        let error = ScriptError::CommandTooLong(40000);
        assert!(error.to_string().contains("40000"));
        assert!(!error.to_string().is_empty(), "the error tells the length");

        // A script of PowerShell holds the long command, for a script of
        // PowerShell is not limited the way a batch file is.
        let _ = &mut command;
    }

    #[test]
    fn the_error_of_a_kind_that_is_not_read_tells_the_extension() {
        let error = ScriptError::UnsupportedKind("txt".to_string());
        assert!(error.to_string().contains("txt"));
    }

    #[test]
    fn the_paths_of_the_launch_are_read_for_the_scripts() {
        // The paths of the launch hold the directory the game runs in, which
        // is the directory the script changes to.
        let paths = LaunchPaths {
            game_directory: PathBuf::from("/game"),
            assets_directory: PathBuf::from("/game/assets"),
            asset_index: "1.20.1".to_string(),
            natives_directory: PathBuf::from("/game/natives"),
            libraries_directory: PathBuf::from("/game/libraries"),
            client_jar: PathBuf::from("/game/versions/1.20.1/1.20.1.jar"),
        };

        let text = script_text(
            ScriptKind::Shell,
            "java Main",
            &options(OsType::Linux),
            &paths.game_directory,
        );

        assert!(text.contains("cd /game\n"));
    }
}
