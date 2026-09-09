use std::{
    io,
    path::{Path, PathBuf},
};

use super::App;
use crate::FileSystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperationMode {
    #[default]
    Immediate,
    Deferred,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOperation {
    Copy {
        sources: Vec<PathBuf>,
        destination: PathBuf,
        overwrite: bool,
    },
    Move {
        sources: Vec<PathBuf>,
        destination: PathBuf,
        overwrite: bool,
    },
    Delete {
        paths: Vec<PathBuf>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRequest {
    pub id: OperationId,
    pub operation: FileOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationProgress {
    pub id: OperationId,
    pub completed: usize,
    pub total: usize,
    pub current_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationFailure {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationResult {
    pub id: OperationId,
    pub succeeded: Vec<PathBuf>,
    pub failures: Vec<OperationFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationApplyError {
    pub expected: Option<OperationId>,
    pub received: OperationId,
}

impl std::fmt::Display for OperationApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "operation result {:?} does not match pending operation {:?}",
            self.received, self.expected
        )
    }
}

impl std::error::Error for OperationApplyError {}

fn copy_one(
    filesystem: &dyn FileSystem,
    source: &Path,
    destination: &Path,
    overwrite: bool,
) -> io::Result<PathBuf> {
    let name = source
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "source has no filename"))?;
    let target = destination.join(name);
    if filesystem.exists(&target) && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", target.display()),
        ));
    }
    if filesystem.is_dir(source) {
        filesystem.copy_dir(source, &target)?;
    } else {
        filesystem.copy_file(source, &target)?;
    }
    Ok(target)
}

/// Execute an operation synchronously, reporting item-level progress.
///
/// Embedders can run this helper on a worker thread, send progress back to the
/// UI thread, then call [`App::apply_operation_result`].
pub fn execute_operation_with_progress(
    request: &OperationRequest,
    on_progress: impl FnMut(OperationProgress),
) -> OperationResult {
    execute_operation_with(crate::std_filesystem().as_ref(), request, on_progress)
}

/// Execute an operation against a custom filesystem backend.
pub fn execute_operation_with(
    filesystem: &dyn FileSystem,
    request: &OperationRequest,
    mut on_progress: impl FnMut(OperationProgress),
) -> OperationResult {
    let (paths, total) = match &request.operation {
        FileOperation::Copy { sources, .. } | FileOperation::Move { sources, .. } => {
            (sources.clone(), sources.len())
        }
        FileOperation::Delete { paths } => (paths.clone(), paths.len()),
    };

    let mut succeeded = Vec::new();
    let mut failures = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        on_progress(OperationProgress {
            id: request.id,
            completed: index,
            total,
            current_path: Some(path.clone()),
        });

        let result = match &request.operation {
            FileOperation::Copy {
                destination,
                overwrite,
                ..
            } => copy_one(filesystem, path, destination, *overwrite),
            FileOperation::Move {
                destination,
                overwrite,
                ..
            } => copy_one(filesystem, path, destination, *overwrite).and_then(|target| {
                if filesystem.is_dir(path) {
                    filesystem.remove_dir_all(path)?;
                } else {
                    filesystem.remove_file(path)?;
                }
                Ok(target)
            }),
            FileOperation::Delete { .. } => {
                let changed = path.clone();
                if filesystem.is_dir(path) {
                    filesystem.remove_dir_all(path).map(|()| changed)
                } else {
                    filesystem.remove_file(path).map(|()| changed)
                }
            }
        };

        match result {
            Ok(path) => succeeded.push(path),
            Err(error) => failures.push(OperationFailure {
                path: path.clone(),
                error: error.to_string(),
            }),
        }
    }

    on_progress(OperationProgress {
        id: request.id,
        completed: total,
        total,
        current_path: None,
    });

    OperationResult {
        id: request.id,
        succeeded,
        failures,
    }
}

/// Execute an operation synchronously without progress callbacks.
pub fn execute_operation(request: &OperationRequest) -> OperationResult {
    execute_operation_with_progress(request, |_| {})
}

impl App {
    pub fn operation_mode(&self) -> OperationMode {
        self.operation_mode
    }

    /// Filesystem backend associated with the active operation destination.
    pub fn operation_filesystem(&self) -> crate::SharedFileSystem {
        self.active_pane().filesystem().clone()
    }

    pub fn set_operation_mode(&mut self, mode: OperationMode) {
        self.operation_mode = mode;
    }

    pub(crate) fn queue_operation(&mut self, operation: FileOperation) -> OperationRequest {
        if let Some(request) = self.pending_operation.clone() {
            self.notify_error("A filesystem operation is already in progress");
            return request;
        }
        let request = OperationRequest {
            id: OperationId(self.next_operation_id),
            operation,
        };
        self.next_operation_id = self.next_operation_id.wrapping_add(1);
        self.pending_operation = Some(request.clone());
        self.queued_operation = Some(request.clone());
        self.status_msg = "Filesystem operation queued.".into();
        request
    }

    /// Take a deferred request when using the legacy `handle_key` API.
    pub fn take_operation_request(&mut self) -> Option<OperationRequest> {
        self.queued_operation.take()
    }

    pub fn pending_operation(&self) -> Option<&OperationRequest> {
        self.pending_operation.as_ref()
    }

    /// Cancel a queued/pending operation. Worker cancellation itself remains
    /// host-owned; late results are rejected by operation ID.
    pub fn cancel_operation(&mut self, id: OperationId) -> Result<(), OperationApplyError> {
        let expected = self.pending_operation.as_ref().map(|request| request.id);
        if expected != Some(id) {
            return Err(OperationApplyError {
                expected,
                received: id,
            });
        }
        self.pending_operation = None;
        self.queued_operation = None;
        self.copy_progress = None;
        self.status_msg = "Filesystem operation cancelled.".into();
        self.event_queue
            .push_back(super::AppEvent::OperationCancelled { id });
        Ok(())
    }

    pub fn apply_operation_progress(
        &mut self,
        progress: OperationProgress,
    ) -> Result<(), OperationApplyError> {
        let expected = self.pending_operation.as_ref().map(|request| request.id);
        if expected != Some(progress.id) {
            return Err(OperationApplyError {
                expected,
                received: progress.id,
            });
        }
        self.copy_progress = Some(super::CopyProgress {
            label: format!("Processing {} item(s)…", progress.total),
            done: progress.completed,
            total: progress.total,
            current_item: progress
                .current_path
                .as_ref()
                .and_then(|path| path.file_name())
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        });
        self.event_queue
            .push_back(super::AppEvent::OperationProgress {
                id: progress.id,
                completed: progress.completed,
                total: progress.total,
            });
        Ok(())
    }

    pub fn apply_operation_result(
        &mut self,
        result: OperationResult,
    ) -> Result<Vec<PathBuf>, OperationApplyError> {
        let expected = self.pending_operation.as_ref().map(|request| request.id);
        if expected != Some(result.id) {
            return Err(OperationApplyError {
                expected,
                received: result.id,
            });
        }
        let succeeded_count = result.succeeded.len();
        let failure_count = result.failures.len();
        let was_move = self
            .pending_operation
            .as_ref()
            .is_some_and(|request| matches!(request.operation, FileOperation::Move { .. }));
        self.pending_operation = None;
        self.queued_operation = None;
        self.copy_progress = None;

        if was_move && result.failures.is_empty() {
            self.clipboard = None;
        }
        for pane in &mut self.panes {
            pane.clear_marks();
            pane.reload();
            pane.clear_dir_size_cache();
        }
        self.preview_state.invalidate();

        if result.failures.is_empty() {
            let message = format!("Completed {} item(s).", result.succeeded.len());
            self.status_msg = message.clone();
            self.notify(message);
        } else {
            let details = result
                .failures
                .iter()
                .map(|failure| format!("'{}': {}", failure.path.display(), failure.error))
                .collect::<Vec<_>>()
                .join("; ");
            let message = format!(
                "Completed {}, {} error(s): {details}",
                result.succeeded.len(),
                result.failures.len()
            );
            self.status_msg = format!("Error: {message}");
            self.notify_error(message);
        }
        self.event_queue
            .push_back(super::AppEvent::OperationCompleted {
                id: result.id,
                succeeded: succeeded_count,
                failed: failure_count,
            });
        Ok(result.succeeded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::{AppOptions, AppOutcome, ClipOp, ClipboardItem};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tempfile::tempdir;

    #[test]
    fn deferred_paste_returns_request_without_touching_destination() {
        let source_dir = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let source = source_dir.path().join("file.txt");
        fs::write(&source, b"payload").unwrap();
        let mut app = App::new(AppOptions {
            pane_dirs: vec![destination.path().to_path_buf()],
            ..AppOptions::default()
        });
        app.clipboard = Some(ClipboardItem {
            paths: vec![source.clone()],
            op: ClipOp::Copy,
        });
        app.set_operation_mode(OperationMode::Deferred);

        let outcome = app
            .dispatch_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE))
            .unwrap();
        let AppOutcome::OperationRequested(request) = outcome else {
            panic!("expected deferred operation");
        };
        assert!(matches!(request.operation, FileOperation::Copy { .. }));
        assert!(!destination.path().join("file.txt").exists());
    }

    #[test]
    fn execute_and_apply_deferred_copy() {
        let source_dir = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let source = source_dir.path().join("file.txt");
        fs::write(&source, b"payload").unwrap();
        let mut app = App::new(AppOptions {
            pane_dirs: vec![destination.path().to_path_buf()],
            ..AppOptions::default()
        });
        app.set_operation_mode(OperationMode::Deferred);
        let request = app.queue_operation(FileOperation::Copy {
            sources: vec![source],
            destination: destination.path().to_path_buf(),
            overwrite: false,
        });
        let result = execute_operation(&request);
        let changed = app.apply_operation_result(result).unwrap();
        assert_eq!(changed, vec![destination.path().join("file.txt")]);
        assert!(destination.path().join("file.txt").exists());
        assert!(app.pending_operation().is_none());
    }

    #[test]
    fn stale_result_is_rejected() {
        let mut app = App::new(AppOptions::default());
        app.set_operation_mode(OperationMode::Deferred);
        app.queue_operation(FileOperation::Delete { paths: Vec::new() });
        let error = app
            .apply_operation_result(OperationResult {
                id: OperationId(999),
                succeeded: Vec::new(),
                failures: Vec::new(),
            })
            .unwrap_err();
        assert_eq!(error.received, OperationId(999));
    }
}
