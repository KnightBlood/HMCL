//! The process of the game and the output it writes.
//!
//! The launcher reads the output of the game while it runs and waits for the
//! process to stop; the way the game stopped tells the player whether the game
//! ran into an error.

use std::fmt;
use std::io::{self, BufRead};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

/// The way the game stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitType {
    /// The game stopped without an error.
    Normal,
    /// The Java runtime could not start the game.
    JvmError,
    /// The game stopped with an error, or the game wrote that it crashed.
    ApplicationError,
    /// The system killed the game, which the launcher sees as the exit code
    /// 137 on Linux and BSD.
    SignalKilled,
    /// The launcher stopped waiting for the game.
    Interrupted,
}

impl fmt::Display for ExitType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Normal => "the game stopped",
            Self::JvmError => "the Java runtime could not start the game",
            Self::ApplicationError => "the game stopped with an error",
            Self::SignalKilled => "the system killed the game",
            Self::Interrupted => "the launcher stopped waiting for the game",
        })
    }
}

/// The lines the Java runtime writes when it cannot start the game.
const JVM_ERROR_MARKERS: [&str; 3] = [
    "Could not create the Java Virtual Machine.",
    "Error occurred during initialization of VM",
    "A fatal exception has occurred. Program will exit.",
];

/// The lines the game writes when it stops with an error.
const APPLICATION_ERROR_MARKERS: [&str; 5] = [
    "Crash report saved to",
    "Could not save crash report to",
    "This crash report has been saved to:",
    "Unable to launch",
    "An exception was thrown, the game will display an error screen and halt.",
];

/// A line the game wrote to its output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameLine {
    /// The text of the line, without the line ending.
    pub text: String,
    /// Whether the game wrote the line to the standard error.
    pub error: bool,
}

/// The way the game stopped, with its exit code and the output it wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameExit {
    /// The exit code of the process.
    ///
    /// The code of a process the system killed is the number of the signal
    /// plus 128, as the systems of Unix report it.
    pub code: i32,
    /// The way the game stopped.
    pub kind: ExitType,
    /// The lines the game wrote while it ran.
    pub lines: Vec<GameLine>,
}

/// The error of starting the game or reading its output.
#[derive(Debug)]
pub enum ProcessError {
    /// The output of the game could not be read, or the system failed to
    /// start the game.
    Io(io::Error),
    /// The launcher may not run the command of the game.
    PermissionDenied,
}

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "The game cannot be started: {error}"),
            Self::PermissionDenied => {
                formatter.write_str("The launcher may not run the command of the game")
            }
        }
    }
}

impl std::error::Error for ProcessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::PermissionDenied => None,
        }
    }
}

/// The process of the game, with the lines it writes.
pub struct GameProcess {
    /// The process of the game.
    child: Child,
    /// The thread that reads the standard output of the game.
    stdout: thread::JoinHandle<()>,
    /// The thread that reads the standard error of the game.
    stderr: thread::JoinHandle<()>,
    /// The lines the game wrote, in the order the launcher read them.
    lines: Arc<CollectedLines>,
}

impl GameProcess {
    /// Starts the game and reads the lines of its output.
    pub fn spawn(command: Command) -> Result<Self, ProcessError> {
        Self::spawn_with(command, |_| {})
    }

    /// Starts the game and hands every line to the callback as the launcher
    /// reads it.
    ///
    /// The callback runs on the threads that read the output, so the launcher
    /// shows the log of the game while it runs. The lines are kept for
    /// [`GameProcess::wait`] as well.
    pub fn spawn_with(
        mut command: Command,
        callback: impl Fn(&GameLine) + Send + Sync + 'static,
    ) -> Result<Self, ProcessError> {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| match error.kind() {
            io::ErrorKind::PermissionDenied => ProcessError::PermissionDenied,
            _ => ProcessError::Io(error),
        })?;

        let lines = Arc::new(CollectedLines::default());
        let callback: Arc<dyn Fn(&GameLine) + Send + Sync> = Arc::new(callback);

        let stdout = child
            .stdout
            .take()
            .map(|stream| pump(stream, false, Arc::clone(&lines), Arc::clone(&callback)))
            .ok_or_else(|| ProcessError::Io(io::Error::other("the output is not piped")))?;
        let stderr = child
            .stderr
            .take()
            .map(|stream| pump(stream, true, Arc::clone(&lines), callback))
            .ok_or_else(|| ProcessError::Io(io::Error::other("the error output is not piped")))?;

        Ok(Self {
            child,
            stdout,
            stderr,
            lines,
        })
    }

    /// Returns the identifier of the process.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Stops the game with the system.
    ///
    /// The launcher waits for the game to stop with [`GameProcess::wait`]
    /// after it stops the process.
    pub fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }

    /// Waits for the game to stop and returns the way it stopped.
    ///
    /// The launcher reads the rest of the output of the game before it returns,
    /// so the lines of the game are complete even when the game stops while it
    /// writes.
    pub fn wait(mut self) -> Result<GameExit, ProcessError> {
        let status = self.child.wait().map_err(ProcessError::Io)?;

        let _ = self.stdout.join();
        let _ = self.stderr.join();

        let lines = Arc::try_unwrap(self.lines)
            .map(CollectedLines::take)
            .unwrap_or_default();
        let code = exit_code(status);
        let kind = classify_exit(code, &lines);

        Ok(GameExit { code, kind, lines })
    }
}

/// Reads the lines of a stream and hands every line to the callback.
///
/// A line is read as the text between the line endings; the line endings are
/// not part of the text. The text of a line that is not valid UTF-8 is read
/// with the characters that replace the invalid bytes, so one broken line does
/// not stop the launcher from reading the rest of the output.
pub fn pump_lines(reader: impl BufRead, mut callback: impl FnMut(&str)) -> io::Result<()> {
    let mut reader = reader;
    let mut buffer = Vec::new();

    loop {
        buffer.clear();
        if reader.read_until(b'\n', &mut buffer)? == 0 {
            break;
        }

        if buffer.last() == Some(&b'\n') {
            buffer.pop();
            if buffer.last() == Some(&b'\r') {
                buffer.pop();
            }
        }

        callback(&String::from_utf8_lossy(&buffer));
    }

    Ok(())
}

/// Returns the way a process stopped.
///
/// The Java runtime writes a line of its own when it cannot start the game, but
/// the mod loaders catch the exceptions of the game and stop with the code 0,
/// so the lines of the game are read as well as the exit code.
pub fn classify_exit(code: i32, lines: &[GameLine]) -> ExitType {
    if code != 0 && contains_one(lines, &JVM_ERROR_MARKERS) {
        ExitType::JvmError
    } else if code != 0 || contains_one(lines, &APPLICATION_ERROR_MARKERS) {
        if code == 137 && is_linux_or_bsd() {
            ExitType::SignalKilled
        } else {
            ExitType::ApplicationError
        }
    } else {
        ExitType::Normal
    }
}

/// Returns whether the output of the game holds one of the lines.
fn contains_one(lines: &[GameLine], markers: &[&str]) -> bool {
    lines
        .iter()
        .any(|line| markers.iter().any(|marker| line.text.contains(marker)))
}

/// Returns the exit code of a process.
fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }

    -1
}

/// Returns whether the launcher runs on Linux or BSD.
fn is_linux_or_bsd() -> bool {
    cfg!(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))
}

/// Reads the lines of one stream of the game.
fn pump(
    stream: impl io::Read + Send + 'static,
    error: bool,
    lines: Arc<CollectedLines>,
    callback: Arc<dyn Fn(&GameLine) + Send + Sync>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let reader = io::BufReader::new(stream);
        let _ = pump_lines(reader, |text| {
            let line = GameLine {
                text: text.to_string(),
                error,
            };
            callback(&line);
            lines.push(line);
        });
    })
}

/// The lines the game wrote, shared with the threads that read the output.
#[derive(Default)]
struct CollectedLines {
    /// The lines of the output.
    lines: Mutex<Vec<GameLine>>,
}

impl CollectedLines {
    /// Keeps a line of the output.
    fn push(&self, line: GameLine) {
        if let Ok(mut lines) = self.lines.lock() {
            lines.push(line);
        }
    }

    /// Returns the lines of the output.
    fn take(self) -> Vec<GameLine> {
        self.lines.into_inner().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Returns lines of the output with the error flag the game writes them by.
    fn lines(texts: &[&str]) -> Vec<GameLine> {
        texts
            .iter()
            .map(|text| GameLine {
                text: (*text).to_string(),
                error: true,
            })
            .collect()
    }

    #[test]
    fn the_lines_of_a_stream_are_read() {
        let mut read = Vec::new();
        pump_lines(Cursor::new(b"alpha\r\nbeta\n\ngamma\n"), |line| {
            read.push(line.to_string());
        })
        .expect("the stream should be read");

        assert_eq!(read, ["alpha", "beta", "", "gamma"]);
    }

    #[test]
    fn a_line_that_is_not_valid_text_is_read_with_replacement_characters() {
        let mut read = Vec::new();
        pump_lines(Cursor::new(b"ab\xffcd\n"), |line| {
            read.push(line.to_string());
        })
        .expect("the stream should be read");

        assert_eq!(read.len(), 1);
        assert!(read[0].starts_with("ab"));
        assert!(read[0].ends_with("cd"));
    }

    #[test]
    fn a_game_that_stopped_without_an_error_is_normal() {
        assert_eq!(classify_exit(0, &[]), ExitType::Normal);
    }

    #[test]
    fn a_game_that_stopped_with_an_error_is_an_application_error() {
        assert_eq!(classify_exit(1, &[]), ExitType::ApplicationError);
        assert_eq!(
            classify_exit(0, &lines(&["Crash report saved to logs"])),
            ExitType::ApplicationError
        );
        assert_eq!(
            classify_exit(0, &lines(&["Unable to launch the game"])),
            ExitType::ApplicationError
        );
    }

    #[test]
    fn the_errors_of_the_java_runtime_are_read() {
        assert_eq!(
            classify_exit(1, &lines(&["Could not create the Java Virtual Machine."])),
            ExitType::JvmError
        );
        assert_eq!(
            classify_exit(1, &lines(&["Error occurred during initialization of VM"])),
            ExitType::JvmError
        );

        // The game may stop with the code 0 after the Java runtime wrote its
        // message, which the launcher reads as a normal stop.
        assert_eq!(
            classify_exit(0, &lines(&["Could not create the Java Virtual Machine."])),
            ExitType::Normal
        );
    }

    #[test]
    fn the_killed_game_is_read_as_killed_on_the_systems_of_unix() {
        let kind = classify_exit(137, &[]);

        if is_linux_or_bsd() {
            assert_eq!(kind, ExitType::SignalKilled);
        } else {
            assert_eq!(kind, ExitType::ApplicationError);
        }
    }

    #[test]
    fn the_output_of_a_process_is_read() {
        #[cfg(windows)]
        let command = {
            let mut command = Command::new("cmd");
            command.args(["/C", "echo hello& (echo oops) 1>&2"]);
            command
        };
        #[cfg(not(windows))]
        let command = {
            let mut command = Command::new("sh");
            command.args(["-c", "echo hello; echo oops 1>&2"]);
            command
        };

        let process = GameProcess::spawn(command).expect("the process should be started");
        let exit = process.wait().expect("the process should stop");

        assert_eq!(exit.code, 0);
        assert_eq!(exit.kind, ExitType::Normal);
        assert!(exit.lines.iter().any(|line| line.text == "hello"));
        assert!(exit
            .lines
            .iter()
            .any(|line| line.text == "oops" && line.error));
    }
}
