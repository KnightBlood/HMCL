//! The task abstraction the launcher runs its background work with.
//!
//! A task reports a display name, can be cancelled through a shared
//! [`CancelToken`] and returns an error that carries a message suitable for
//! the log and for a dialog.
//!
//! A task that runs in the background is started with
//! [`RunningTask::spawn`], which runs the task in a thread of its own and
//! collects the progress the task reports.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod runner;

pub use runner::RunningTask;

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

/// The progress of a task.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Progress {
    /// The name of the step the task is in.
    pub stage: String,
    /// The amount of the work that is done.
    pub progress: u64,
    /// The amount of the work the task has to do.
    ///
    /// A total of zero means that the amount of the work is not known yet.
    pub total: u64,
}

impl Progress {
    /// Creates the progress of a task that is in a step.
    pub fn stage(stage: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            progress: 0,
            total: 0,
        }
    }

    /// Creates the progress of a task that did a part of its work.
    pub fn of(stage: impl Into<String>, progress: u64, total: u64) -> Self {
        Self {
            stage: stage.into(),
            progress,
            total,
        }
    }

    /// Returns the part of the work that is done, from zero to one.
    ///
    /// A task that does not know how much work it has to do reports no
    /// progress at all.
    pub fn fraction(&self) -> Option<f64> {
        if self.total == 0 {
            return None;
        }
        Some(self.progress as f64 / self.total as f64)
    }
}

/// The context a task runs in.
///
/// The context carries the token that asks the task to stop and the channel
/// the progress of the task is sent through.
pub struct TaskContext<'a> {
    /// The token that asks the task to stop.
    token: &'a CancelToken,
    /// The channel the progress of the task is sent through.
    progress: Option<&'a std::sync::mpsc::Sender<Progress>>,
}

impl<'a> TaskContext<'a> {
    /// Creates a context for a task that runs on its own.
    pub fn detached(token: &'a CancelToken) -> Self {
        Self {
            token,
            progress: None,
        }
    }

    /// Creates a context for a task that reports its progress.
    pub fn with_progress(
        token: &'a CancelToken,
        progress: &'a std::sync::mpsc::Sender<Progress>,
    ) -> Self {
        Self {
            token,
            progress: Some(progress),
        }
    }

    /// Returns the token that asks the task to stop.
    pub fn token(&self) -> &CancelToken {
        self.token
    }

    /// Returns whether a cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Reports the progress of the task.
    ///
    /// The progress is dropped when nobody listens to it, so a task does not
    /// have to know whether the launcher shows its progress.
    pub fn report(&self, progress: Progress) {
        if let Some(sender) = self.progress {
            let _ = sender.send(progress);
        }
    }
}

/// A unit of work that can be run in the background.
pub trait Task {
    /// Returns the name shown for the task in the progress UI.
    fn name(&self) -> &str;

    /// Runs the task.
    ///
    /// Implementations should return [`TaskError::cancelled`] as soon as the
    /// context reports a cancellation instead of finishing their work.
    fn run(&self, context: &TaskContext<'_>) -> Result<(), TaskError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingTask;

    impl Task for RecordingTask {
        fn name(&self) -> &str {
            "recording"
        }

        fn run(&self, context: &TaskContext<'_>) -> Result<(), TaskError> {
            if context.is_cancelled() {
                return Err(TaskError::cancelled());
            }
            Ok(())
        }
    }

    #[test]
    fn a_fresh_token_runs_the_task() {
        let token = CancelToken::new();
        let task = RecordingTask;
        assert!(task.run(&TaskContext::detached(&token)).is_ok());
    }

    #[test]
    fn a_cancelled_token_stops_the_task() {
        let token = CancelToken::new();
        token.cancel();

        let error = RecordingTask
            .run(&TaskContext::detached(&token))
            .unwrap_err();
        assert!(error.is_cancelled());
        assert_eq!(error, TaskError::cancelled());
    }

    #[test]
    fn the_progress_of_a_task_is_told_apart() {
        let progress = Progress::stage("Downloading");
        assert_eq!(progress.stage, "Downloading");
        assert_eq!(progress.fraction(), None);

        let progress = Progress::of("Downloading", 5, 10);
        assert_eq!(progress.fraction(), Some(0.5));
    }
}
