use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::host::HostContext;

/// A handle to a scheduled task allowing cancellation and status inspection.
#[derive(Clone)]
pub struct TaskHandle {
    pub id: u64,
    pub(crate) cancelled: Arc<AtomicBool>,
    pub(crate) host: Arc<dyn HostContext>,
}

impl std::fmt::Debug for TaskHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskHandle")
            .field("id", &self.id)
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

impl TaskHandle {
    pub fn new(id: u64, host: Arc<dyn HostContext>) -> Self {
        Self {
            id,
            cancelled: Arc::new(AtomicBool::new(false)),
            host,
        }
    }

    /// Cancels the scheduled task. If the task is already running or completed, this is a no-op.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.host.cancel_task(self.id);
    }

    /// Checks if this task was marked as cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// The scheduler provides facilities to run asynchronous, delayed, or repeating background tasks.
#[derive(Clone)]
pub struct Scheduler {
    host: Arc<dyn HostContext>,
}

impl Scheduler {
    pub fn new(host: Arc<dyn HostContext>) -> Self {
        Self { host }
    }

    /// Dispatches an immediate task to the server runtime.
    pub fn run_task<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.host.run_task(Box::new(task));
    }

    /// Schedules a task to be executed after the specified delay.
    pub fn run_task_later<F>(&self, delay: Duration, task: F) -> TaskHandle
    where
        F: FnOnce() + Send + 'static,
    {
        let handle = TaskHandle::new(0, self.host.clone());
        let cancelled = handle.cancelled.clone();
        let wrapped_task = Box::new(move || {
            if !cancelled.load(Ordering::Acquire) {
                task();
            }
        });

        let id = self.host.run_task_later(delay.as_millis() as u64, wrapped_task);
        let mut handle = handle;
        handle.id = id;
        handle
    }

    /// Schedules a task to run periodically with a fixed interval.
    pub fn run_task_repeating<F>(&self, initial_delay: Duration, period: Duration, mut task: F) -> TaskHandle
    where
        F: FnMut() + Send + 'static,
    {
        let handle = TaskHandle::new(0, self.host.clone());
        let cancelled = handle.cancelled.clone();
        let wrapped_task = Box::new(move || {
            if !cancelled.load(Ordering::Acquire) {
                task();
            }
        });

        let id = self.host.run_task_repeating(
            initial_delay.as_millis() as u64,
            period.as_millis() as u64,
            wrapped_task,
        );
        let mut handle = handle;
        handle.id = id;
        handle
    }
}
