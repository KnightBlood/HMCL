//! The task abstraction the launcher runs its background work with.
//!
//! A task reports a display name, can be cancelled through a shared
//! [`CancelToken`] and returns an error that carries a message suitable for
//! the log and for a dialog.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A handle that asks a running task to stop.
///
/// The token is shared with the task; a task is expected to check
/// [`CancelToken::is_cancelled`] between steps and to return
/// [`TaskError::cancelled`] when it observes a cancellation.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    /// Creates a token that is not cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests the cancellation of the task that holds the token.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Returns whether a cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// The failure of a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskError {
    message: String,
    cancelled: bool,
}

impl TaskError {
    /// Creates an error with a message that is shown to the user.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cancelled: false,
        }
    }

    /// Creates the error a task returns when it stops because of a
    /// cancellation request.
    pub fn cancelled() -> Self {
        Self {
            message: "The task was cancelled".to_string(),
            cancelled: true,
        }
    }

    /// Returns the message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns whether the error reports a cancellation.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TaskError {}

/// A unit of work that can be run in the background.
pub trait Task {
    /// Returns the name shown for the task in the progress UI.
    fn name(&self) -> &str;

    /// Runs the task.
    ///
    /// Implementations should return [`TaskError::cancelled`] as soon as
    /// `token` reports a cancellation instead of finishing their work.
    fn run(&self, token: &CancelToken) -> Result<(), TaskError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingTask;

    impl Task for RecordingTask {
        fn name(&self) -> &str {
            "recording"
        }

        fn run(&self, token: &CancelToken) -> Result<(), TaskError> {
            if token.is_cancelled() {
                return Err(TaskError::cancelled());
            }
            Ok(())
        }
    }

    #[test]
    fn a_fresh_token_runs_the_task() {
        let task = RecordingTask;
        assert!(task.run(&CancelToken::new()).is_ok());
    }

    #[test]
    fn a_cancelled_token_stops_the_task() {
        let token = CancelToken::new();
        token.cancel();

        let error = RecordingTask.run(&token).unwrap_err();
        assert!(error.is_cancelled());
        assert_eq!(error, TaskError::cancelled());
    }
}
