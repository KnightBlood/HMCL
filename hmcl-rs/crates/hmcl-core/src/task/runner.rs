//! The runner of the tasks.
//!
//! A task that runs in the background is started with
//! [`RunningTask::spawn`], which runs the task in a thread of its own. The
//! progress the task reports is collected by the runner, so the user
//! interface can show it.

use std::sync::mpsc::{channel, Receiver};
use std::thread::JoinHandle;

use super::{CancelToken, Progress, Task, TaskContext, TaskError};

/// A task that runs in the background.
#[derive(Debug)]
pub struct RunningTask {
    /// The name of the task.
    name: String,
    /// The token that asks the task to stop.
    token: CancelToken,
    /// The thread the task runs in.
    handle: Option<JoinHandle<Result<(), TaskError>>>,
    /// The channel the progress of the task is sent through.
    progress: Receiver<Progress>,
}

impl RunningTask {
    /// Starts a task in a thread of its own.
    pub fn spawn<T: Task + Send + 'static>(task: T) -> Self {
        let name = task.name().to_string();
        let token = CancelToken::new();
        let (sender, receiver) = channel();

        let task_token = token.clone();
        let handle = std::thread::spawn(move || {
            let context = TaskContext::with_progress(&task_token, &sender);
            task.run(&context)
        });

        Self {
            name,
            token,
            handle: Some(handle),
            progress: receiver,
        }
    }

    /// Returns the name of the task.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the token that asks the task to stop.
    pub fn token(&self) -> CancelToken {
        self.token.clone()
    }

    /// Asks the task to stop.
    ///
    /// The task stops when it notices the cancellation, so it may still be
    /// running for a while.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Returns whether the task asked to be stopped.
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Returns the next progress of the task, if there is one.
    ///
    /// The method does not wait, so it is meant to be called from the loop
    /// that draws the user interface.
    pub fn try_next_progress(&self) -> Option<Progress> {
        self.progress.try_recv().ok()
    }

    /// Returns the next progress of the task, waiting for it.
    ///
    /// The method returns `None` when the task is done and reported
    /// everything it had to report.
    pub fn next_progress(&self) -> Option<Progress> {
        self.progress.recv().ok()
    }

    /// Waits for the task to finish and returns what it reported.
    pub fn join(mut self) -> Result<(), TaskError> {
        match self.handle.take() {
            Some(handle) => handle
                .join()
                .unwrap_or_else(|_| Err(TaskError::new("The task stopped unexpectedly"))),
            None => Err(TaskError::new("The task was already joined")),
        }
    }
}

impl Drop for RunningTask {
    fn drop(&mut self) {
        // The task should not keep running when nobody waits for it.
        self.token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A task that reports its progress.
    struct ReportingTask {
        steps: usize,
        reported: Arc<AtomicUsize>,
    }

    impl Task for ReportingTask {
        fn name(&self) -> &str {
            "reporting"
        }

        fn run(&self, context: &TaskContext<'_>) -> Result<(), TaskError> {
            for step in 0..self.steps {
                if context.is_cancelled() {
                    return Err(TaskError::cancelled());
                }
                self.reported.fetch_add(1, Ordering::SeqCst);
                context.report(Progress::of("Working", step as u64 + 1, self.steps as u64));
            }
            Ok(())
        }
    }

    /// A task that fails.
    struct FailingTask;

    impl Task for FailingTask {
        fn name(&self) -> &str {
            "failing"
        }

        fn run(&self, _context: &TaskContext<'_>) -> Result<(), TaskError> {
            Err(TaskError::new("the task failed"))
        }
    }

    /// A task that waits for a cancellation.
    struct WaitingTask;

    impl Task for WaitingTask {
        fn name(&self) -> &str {
            "waiting"
        }

        fn run(&self, context: &TaskContext<'_>) -> Result<(), TaskError> {
            while !context.is_cancelled() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(TaskError::cancelled())
        }
    }

    #[test]
    fn the_progress_of_a_task_is_collected() {
        let reported = Arc::new(AtomicUsize::new(0));
        let task = RunningTask::spawn(ReportingTask {
            steps: 3,
            reported: Arc::clone(&reported),
        });

        assert_eq!(task.name(), "reporting");

        let mut progress = Vec::new();
        while let Some(step) = task.next_progress() {
            progress.push(step);
        }

        assert!(task.join().is_ok());
        assert_eq!(reported.load(Ordering::SeqCst), 3);
        assert_eq!(progress.len(), 3);
        assert_eq!(progress[0].stage, "Working");
        assert_eq!(progress[2].progress, 3);
    }

    #[test]
    fn the_failure_of_a_task_is_reported() {
        let task = RunningTask::spawn(FailingTask);
        let error = task.join().unwrap_err();

        assert_eq!(error.message(), "the task failed");
        assert!(!error.is_cancelled());
    }

    #[test]
    fn a_task_can_be_stopped() {
        let task = RunningTask::spawn(WaitingTask);
        task.cancel();

        let error = task.join().unwrap_err();
        assert!(error.is_cancelled());
    }
}
