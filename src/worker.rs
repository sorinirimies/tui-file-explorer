//! Tiny background-task primitive used to keep slow filesystem work (folder
//! sizes, previews, tree walks, `git status`) off the UI thread.
//!
//! A [`Task`] owns a worker thread and the receiving end of its channel.
//! Dropping the task *cancels* it: the worker's [`Emit::is_cancelled`] turns
//! `true` and further [`Emit::send`] calls report failure, so abandoned work
//! stops at its next checkpoint.

use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError},
        Arc,
    },
    thread,
};

/// Sending half handed to the worker closure.
pub struct Emit<T> {
    tx: Sender<T>,
    cancel: Arc<AtomicBool>,
}

impl<T> Emit<T> {
    /// Send a result to the owner. Returns `false` once the task has been
    /// cancelled or dropped — the worker should then stop.
    pub fn send(&self, value: T) -> bool {
        !self.is_cancelled() && self.tx.send(value).is_ok()
    }

    /// Has the owner cancelled (or dropped) the task?
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

/// A running (or finished) background computation producing `T`s.
pub struct Task<T> {
    rx: Receiver<T>,
    cancel: Arc<AtomicBool>,
    done: bool,
}

impl<T> fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task").field("done", &self.done).finish()
    }
}

impl<T: Send + 'static> Task<T> {
    /// Run `work` on a new thread.
    pub fn spawn(work: impl FnOnce(&Emit<T>) + Send + 'static) -> Self {
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let emit = Emit {
            tx,
            cancel: Arc::clone(&cancel),
        };
        thread::spawn(move || work(&emit));
        Self {
            rx,
            cancel,
            done: false,
        }
    }

    /// Collect everything produced so far without blocking.
    pub fn poll(&mut self) -> Vec<T> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(v) => out.push(v),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.done = true;
                    break;
                }
            }
        }
        out
    }

    /// Wait up to `timeout` for the next value. Returns `None` on timeout or
    /// once the worker has finished.
    pub fn wait_next(&mut self, timeout: std::time::Duration) -> Option<T> {
        match self.rx.recv_timeout(timeout) {
            Ok(v) => Some(v),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                self.done = true;
                None
            }
        }
    }

    /// Has the worker finished *and* all its output been polled?
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn drain<T: Send + 'static>(task: &mut Task<T>) -> Vec<T> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut all = Vec::new();
        while !task.is_done() && Instant::now() < deadline {
            all.extend(task.poll());
            thread::sleep(Duration::from_millis(2));
        }
        all
    }

    #[test]
    fn delivers_results_then_reports_done() {
        let mut task = Task::spawn(|emit| {
            for i in 0..3 {
                emit.send(i);
            }
        });
        assert_eq!(drain(&mut task), vec![0, 1, 2]);
        assert!(task.is_done());
    }

    #[test]
    fn wait_next_returns_a_value_or_times_out() {
        let mut quick: Task<u8> = Task::spawn(|emit| {
            emit.send(7);
        });
        assert_eq!(quick.wait_next(Duration::from_secs(5)), Some(7));

        let mut slow: Task<u8> = Task::spawn(|emit| {
            thread::sleep(Duration::from_millis(300));
            emit.send(1);
        });
        assert_eq!(slow.wait_next(Duration::from_millis(5)), None);
        assert!(!slow.is_done(), "a timeout is not completion");
        assert_eq!(slow.wait_next(Duration::from_secs(5)), Some(1));
    }

    #[test]
    fn wait_next_after_the_worker_ends_marks_done() {
        let mut t: Task<u8> = Task::spawn(|_| {});
        assert_eq!(t.wait_next(Duration::from_secs(5)), None);
        assert!(t.is_done());
    }

    #[test]
    fn dropping_cancels_the_worker() {
        let (tx, rx) = channel();
        let task: Task<u8> = Task::spawn(move |emit| {
            while !emit.is_cancelled() {
                thread::sleep(Duration::from_millis(1));
            }
            tx.send(()).unwrap();
        });
        drop(task);
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    #[test]
    fn send_fails_after_cancel() {
        let (tx, rx) = channel();
        let task: Task<u8> = Task::spawn(move |emit| {
            while !emit.is_cancelled() {
                thread::sleep(Duration::from_millis(1));
            }
            tx.send(emit.send(1)).unwrap();
        });
        drop(task);
        assert!(!rx.recv_timeout(Duration::from_secs(5)).unwrap());
    }
}
