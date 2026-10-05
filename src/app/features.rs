//! Application-level features built on the panes: undo, bookmarks, the fuzzy
//! finder, bulk rename, archive extraction and the git column.
//!
//! Everything slow runs on a worker thread and is collected by
//! [`App::poll_background`], which `draw` calls once per frame.

use std::collections::VecDeque;

use super::*;
use crate::{
    bulk_rename,
    filesystem::FileSystem,
    picker::{Picker, PickerAction, PickerItem, PickerKind},
    worker::Task,
};

/// Maximum number of undoable steps kept.
const UNDO_LIMIT: usize = 100;

// ── Undo ──────────────────────────────────────────────────────────────────────

/// One reversible user action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoEntry {
    /// `from` was renamed to `to`.
    Rename { from: PathBuf, to: PathBuf },
    /// A file or folder was created at `path`.
    Create { path: PathBuf, is_dir: bool },
    /// Items were moved: each `(from, to)`.
    Move { pairs: Vec<(PathBuf, PathBuf)> },
    /// Items were copied: each `(source, new_copy)`.
    Copy { pairs: Vec<(PathBuf, PathBuf)> },
    /// Items were deleted *to the trash* (restorable only on some platforms).
    Delete { paths: Vec<PathBuf> },
    /// `archive` was extracted into the new folder `dest`.
    Extract { archive: PathBuf, dest: PathBuf },
    /// Several entries undone together (bulk rename).
    Batch(Vec<UndoEntry>),
}

impl UndoEntry {
    /// Short description for the status line.
    #[must_use]
    pub fn describe(&self) -> String {
        let name = |p: &Path| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };
        match self {
            Self::Rename { from, to } => format!("rename {} → {}", name(from), name(to)),
            Self::Create { path, .. } => format!("create {}", name(path)),
            Self::Move { pairs } => format!("move of {} item(s)", pairs.len()),
            Self::Copy { pairs } => format!("copy of {} item(s)", pairs.len()),
            Self::Delete { paths } => format!("delete of {} item(s)", paths.len()),
            Self::Extract { dest, .. } => format!("extract into {}", name(dest)),
            Self::Batch(items) => format!("{} change(s)", items.len()),
        }
    }
}

/// Reverse `entry` on `fs`. Returns the number of items restored.
fn apply_undo(fs: &dyn FileSystem, entry: &UndoEntry) -> Result<usize, String> {
    let remove = |p: &Path| {
        if fs.is_dir(p) {
            fs.remove_dir_all(p)
        } else {
            fs.remove_file(p)
        }
    };
    match entry {
        UndoEntry::Rename { from, to } => {
            if !fs.exists(to) {
                return Err(format!("'{}' no longer exists", to.display()));
            }
            if fs.exists(from) {
                return Err(format!("'{}' already exists", from.display()));
            }
            fs.rename(to, from).map_err(|e| e.to_string())?;
            Ok(1)
        }
        UndoEntry::Create { path, is_dir } => {
            if !fs.exists(path) {
                return Err(format!("'{}' no longer exists", path.display()));
            }
            // Only remove what is still the empty thing we created.
            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            let empty = if *is_dir {
                fs.read_dir(path).map_err(|e| e.to_string())?.is_empty()
            } else {
                fs.read_dir(parent)
                    .map_err(|e| e.to_string())?
                    .iter()
                    .any(|e| e.path == *path && e.size == Some(0))
            };
            if !empty {
                return Err(format!(
                    "'{}' is no longer empty — left in place",
                    path.display()
                ));
            }
            remove(path).map_err(|e| e.to_string())?;
            Ok(1)
        }
        UndoEntry::Move { pairs } => {
            let mut done = 0;
            for (from, to) in pairs.iter().rev() {
                if !fs.exists(to) {
                    return Err(format!("'{}' no longer exists", to.display()));
                }
                if fs.exists(from) {
                    return Err(format!("'{}' already exists", from.display()));
                }
                let copied = if fs.is_dir(to) {
                    fs.copy_dir(to, from)
                } else {
                    fs.copy_file(to, from)
                };
                copied.map_err(|e| e.to_string())?;
                remove(to).map_err(|e| e.to_string())?;
                done += 1;
            }
            Ok(done)
        }
        UndoEntry::Copy { pairs } => {
            let mut done = 0;
            for (_, p) in pairs.iter().rev() {
                if fs.exists(p) {
                    remove(p).map_err(|e| e.to_string())?;
                    done += 1;
                }
            }
            Ok(done)
        }
        UndoEntry::Delete { paths } => {
            let mut done = 0;
            for p in paths {
                fs.restore_deleted(p)
                    .map_err(|e| format!("cannot restore '{}': {e}", p.display()))?;
                done += 1;
            }
            Ok(done)
        }
        UndoEntry::Extract { dest, .. } => {
            if fs.exists(dest) {
                remove(dest).map_err(|e| e.to_string())?;
            }
            Ok(1)
        }
        UndoEntry::Batch(items) => {
            // A batch of renames may contain swaps/chains: undo it with the
            // same two-phase strategy used to apply it.
            let renames: Option<Vec<bulk_rename::RenameStep>> = items
                .iter()
                .map(|e| match e {
                    UndoEntry::Rename { from, to } => Some(bulk_rename::RenameStep {
                        from: to.clone(),
                        to: from.clone(),
                    }),
                    _ => None,
                })
                .collect();
            if let Some(steps) = renames {
                if let Some(missing) = steps.iter().find(|s| !fs.exists(&s.from)) {
                    return Err(format!("'{}' no longer exists", missing.from.display()));
                }
                // Targets must be free unless they are themselves moving.
                let moving: Vec<&Path> = steps.iter().map(|s| s.from.as_path()).collect();
                if let Some(taken) = steps
                    .iter()
                    .find(|s| fs.exists(&s.to) && !moving.contains(&s.to.as_path()))
                {
                    return Err(format!("'{}' already exists", taken.to.display()));
                }
                return bulk_rename::apply(fs, &steps)
                    .map(|done| done.len())
                    .map_err(|e| e.to_string());
            }
            let mut done = 0;
            for e in items.iter().rev() {
                done += apply_undo(fs, e)?;
            }
            Ok(done)
        }
    }
}

/// What re-doing an entry asks of the caller.
#[derive(Debug, PartialEq, Eq)]
enum Redone {
    /// Done; this many items were affected.
    Items(usize),
    /// The extraction must be re-run (in the background) by the caller.
    Extract { archive: PathBuf, dest: PathBuf },
}

/// Re-apply `entry` (the inverse of [`apply_undo`]) on `fs`.
fn apply_redo(fs: &dyn FileSystem, entry: &UndoEntry) -> Result<Redone, String> {
    let remove = |p: &Path| {
        if fs.is_dir(p) {
            fs.remove_dir_all(p)
        } else {
            fs.remove_file(p)
        }
    };
    let copy = |from: &Path, to: &Path| {
        if fs.is_dir(from) {
            fs.copy_dir(from, to)
        } else {
            fs.copy_file(from, to)
        }
    };
    let need = |p: &Path| -> Result<(), String> {
        if fs.exists(p) {
            Ok(())
        } else {
            Err(format!("'{}' no longer exists", p.display()))
        }
    };
    let free = |p: &Path| -> Result<(), String> {
        if fs.exists(p) {
            Err(format!("'{}' already exists", p.display()))
        } else {
            Ok(())
        }
    };
    match entry {
        UndoEntry::Rename { from, to } => {
            need(from)?;
            free(to)?;
            fs.rename(from, to).map_err(|e| e.to_string())?;
            Ok(Redone::Items(1))
        }
        UndoEntry::Create { path, is_dir } => {
            free(path)?;
            let made = if *is_dir {
                fs.create_dir_all(path)
            } else {
                fs.create_file(path)
            };
            made.map_err(|e| e.to_string())?;
            Ok(Redone::Items(1))
        }
        UndoEntry::Move { pairs } => {
            for (from, to) in pairs {
                need(from)?;
                free(to)?;
                copy(from, to).map_err(|e| e.to_string())?;
                remove(from).map_err(|e| e.to_string())?;
            }
            Ok(Redone::Items(pairs.len()))
        }
        UndoEntry::Copy { pairs } => {
            for (from, to) in pairs {
                need(from)?;
                free(to)?;
                copy(from, to).map_err(|e| e.to_string())?;
            }
            Ok(Redone::Items(pairs.len()))
        }
        UndoEntry::Delete { paths } => {
            for p in paths {
                need(p)?;
                remove(p).map_err(|e| e.to_string())?;
            }
            Ok(Redone::Items(paths.len()))
        }
        UndoEntry::Extract { archive, dest } => {
            need(archive)?;
            free(dest)?;
            Ok(Redone::Extract {
                archive: archive.clone(),
                dest: dest.clone(),
            })
        }
        UndoEntry::Batch(items) => {
            let renames: Option<Vec<bulk_rename::RenameStep>> = items
                .iter()
                .map(|e| match e {
                    UndoEntry::Rename { from, to } => Some(bulk_rename::RenameStep {
                        from: from.clone(),
                        to: to.clone(),
                    }),
                    _ => None,
                })
                .collect();
            if let Some(steps) = renames {
                for s in &steps {
                    need(&s.from)?;
                }
                let moving: Vec<&Path> = steps.iter().map(|s| s.from.as_path()).collect();
                if let Some(taken) = steps
                    .iter()
                    .find(|s| fs.exists(&s.to) && !moving.contains(&s.to.as_path()))
                {
                    return Err(format!("'{}' already exists", taken.to.display()));
                }
                return bulk_rename::apply(fs, &steps)
                    .map(|done| Redone::Items(done.len()))
                    .map_err(|e| e.to_string());
            }
            let mut n = 0;
            for e in items {
                match apply_redo(fs, e)? {
                    Redone::Items(k) => n += k,
                    Redone::Extract { .. } => {
                        return Err("cannot redo an extraction inside a batch".into())
                    }
                }
            }
            Ok(Redone::Items(n))
        }
    }
}

// ── Bulk rename session ───────────────────────────────────────────────────────

/// An in-progress bulk rename: the temp file being edited and the entries it
/// stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRenameSession {
    pub file: PathBuf,
    pub sources: Vec<PathBuf>,
}

// ── Background results ────────────────────────────────────────────────────────

/// A running archive extraction.
#[cfg(feature = "archive")]
pub(crate) struct ExtractJob {
    archive: PathBuf,
    dest: PathBuf,
    is_redo: bool,
    task: Task<Result<usize, String>>,
}

/// Messages from the fuzzy-finder tree walk.
#[derive(Debug)]
pub(crate) enum WalkMsg {
    Items(Vec<PickerItem>),
    Truncated,
}

/// Cap on files visited by the fuzzy finder.
const WALK_MAX_ENTRIES: usize = 200_000;
/// Wall-clock cap for the fuzzy finder walk.
const WALK_MAX_DURATION: Duration = Duration::from_secs(8);

/// Breadth-first walk of `root`, streaming batches of picker rows (paths
/// relative to `root`). Skips VCS metadata always, and `node_modules` /
/// `target` unless hidden entries are shown. Returns `true` when a cap cut
/// the walk short.
pub(crate) fn walk_tree(
    fs: &dyn FileSystem,
    root: &Path,
    show_hidden: bool,
    emit: &mut dyn FnMut(Vec<PickerItem>) -> bool,
) -> bool {
    let started = Instant::now();
    let mut queue: VecDeque<PathBuf> = VecDeque::from([root.to_path_buf()]);
    let mut batch: Vec<PickerItem> = Vec::new();
    let mut visited = 0usize;

    while let Some(dir) = queue.pop_front() {
        let Ok(mut entries) = fs.read_dir(&dir) else {
            continue;
        };
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        for e in entries {
            let hidden = e.name.starts_with('.');
            if hidden && !show_hidden {
                continue;
            }
            if e.is_dir
                && (matches!(e.name.as_str(), ".git" | ".hg" | ".svn")
                    || (!show_hidden && matches!(e.name.as_str(), "node_modules" | "target")))
            {
                continue;
            }
            visited += 1;
            let rel = e.path.strip_prefix(root).unwrap_or(&e.path);
            let mut label = rel.to_string_lossy().replace('\\', "/");
            if e.is_dir {
                label.push('/');
                queue.push_back(e.path.clone());
            }
            batch.push(PickerItem {
                label,
                detail: String::new(),
                path: e.path,
                is_dir: e.is_dir,
            });
            if batch.len() >= 512 && !emit(std::mem::take(&mut batch)) {
                return false;
            }
            if visited >= WALK_MAX_ENTRIES || started.elapsed() >= WALK_MAX_DURATION {
                let _ = emit(std::mem::take(&mut batch));
                return true;
            }
        }
    }
    let _ = emit(batch);
    false
}

impl App {
    // ── Undo ──────────────────────────────────────────────────────────────

    /// Remember `entry` so [`undo`](Self::undo) can reverse it.
    ///
    /// A new action invalidates anything that could have been redone.
    pub fn record_undo(&mut self, entry: UndoEntry) {
        self.redo_stack.clear();
        self.push_undo(entry);
    }

    fn push_undo(&mut self, entry: UndoEntry) {
        self.undo_stack.push(entry);
        if self.undo_stack.len() > UNDO_LIMIT {
            self.undo_stack.remove(0);
        }
    }

    /// Reverse the most recent undoable action.
    pub fn undo(&mut self) {
        let Some(entry) = self.undo_stack.pop() else {
            self.notify("Nothing to undo");
            return;
        };
        let fs = self.active_pane().filesystem.clone();
        match apply_undo(fs.as_ref(), &entry) {
            Ok(_) => {
                let msg = format!("Undid {}", entry.describe());
                self.redo_stack.push(entry);
                for p in &mut self.panes {
                    p.reload();
                    p.clear_dir_size_cache();
                }
                self.preview_state.invalidate();
                self.status_msg = msg.clone();
                self.notify(msg);
            }
            Err(e) => {
                let msg = format!("Undo failed: {e}");
                self.status_msg = format!("Error: {msg}");
                self.notify_error(msg);
            }
        }
    }

    /// Re-apply the action most recently reversed by [`undo`](Self::undo).
    pub fn redo(&mut self) {
        let Some(entry) = self.redo_stack.pop() else {
            self.notify("Nothing to redo");
            return;
        };
        let fs = self.active_pane().filesystem.clone();
        match apply_redo(fs.as_ref(), &entry) {
            Ok(Redone::Items(_)) => {
                let msg = format!("Redid {}", entry.describe());
                self.push_undo(entry);
                for p in &mut self.panes {
                    p.reload();
                    p.clear_dir_size_cache();
                }
                self.preview_state.invalidate();
                self.status_msg = msg.clone();
                self.notify(msg);
            }
            #[cfg(feature = "archive")]
            Ok(Redone::Extract { archive, dest }) => {
                // Runs in the background; the finished extraction goes back
                // on the undo stack (without discarding the rest of redo).
                self.start_extract(archive, dest, true);
            }
            #[cfg(not(feature = "archive"))]
            Ok(Redone::Extract { .. }) => {
                self.notify_error("Redo failed: archive support disabled")
            }
            Err(e) => {
                let msg = format!("Redo failed: {e}");
                self.status_msg = format!("Error: {msg}");
                self.notify_error(msg);
            }
        }
    }

    // ── Bookmarks ─────────────────────────────────────────────────────────

    /// Add the active pane's directory to the bookmarks, or remove it if it
    /// is already bookmarked.
    pub fn toggle_bookmark(&mut self) {
        let dir = self.active_pane().current_dir.clone();
        if let Some(i) = self.bookmarks.iter().position(|b| *b == dir) {
            self.bookmarks.remove(i);
            self.notify(format!("Removed bookmark '{}'", dir.display()));
        } else {
            self.notify(format!("Bookmarked '{}'", dir.display()));
            self.bookmarks.push(dir);
        }
    }

    /// Open the bookmark list.
    pub fn open_bookmarks(&mut self) {
        if self.bookmarks.is_empty() {
            self.notify("No bookmarks yet — press b to bookmark this folder");
            return;
        }
        let items = self
            .bookmarks
            .iter()
            .map(|p| PickerItem {
                label: p.display().to_string(),
                detail: String::new(),
                path: p.clone(),
                is_dir: true,
            })
            .collect();
        self.picker_task = None;
        self.picker = Some(Picker::new(PickerKind::Bookmarks, "Bookmarks", items));
    }

    // ── Fuzzy finder ──────────────────────────────────────────────────────

    /// Open the recursive fuzzy finder rooted at the active pane's directory.
    pub fn open_fuzzy_find(&mut self) {
        let pane = self.active_pane();
        let root = pane.current_dir.clone();
        let fs = pane.filesystem.clone();
        let show_hidden = pane.show_hidden;

        let mut picker = Picker::new(
            PickerKind::Fuzzy,
            format!("Find in {}", root.display()),
            Vec::new(),
        );
        picker.loading = true;
        self.picker = Some(picker);
        self.picker_task = Some(Task::spawn(move |emit| {
            let truncated = walk_tree(fs.as_ref(), &root, show_hidden, &mut |batch| {
                emit.send(WalkMsg::Items(batch))
            });
            if truncated {
                emit.send(WalkMsg::Truncated);
            }
        }));
    }

    /// Route a key to the open picker. Returns `true` when the app should
    /// exit (never, currently) — kept for symmetry with `handle_key`.
    pub(super) fn handle_picker_key(&mut self, key: crossterm::event::KeyEvent) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        match picker.handle_key(key) {
            PickerAction::Pending => {}
            PickerAction::Cancel => self.close_picker(),
            PickerAction::Choose(item) => {
                self.close_picker();
                self.jump_to(&item.path);
            }
            PickerAction::Delete(item) => {
                self.bookmarks.retain(|b| *b != item.path);
                if let Some(p) = self.picker.as_mut() {
                    p.remove(&item);
                }
                if self.bookmarks.is_empty() {
                    self.close_picker();
                }
            }
        }
    }

    fn close_picker(&mut self) {
        self.picker = None;
        self.picker_task = None;
    }

    /// Navigate the active pane to `path` (a folder, or a file's folder with
    /// the file selected).
    pub fn jump_to(&mut self, path: &Path) {
        if !self.active_pane().filesystem.exists(path) {
            self.notify_error(format!("'{}' no longer exists", path.display()));
            return;
        }
        self.active_pane_mut().go_to(path);
        self.preview_state.invalidate();
        self.status_msg = format!("Jumped to '{}'", path.display());
    }

    // ── Bulk rename ───────────────────────────────────────────────────────

    /// Start a bulk rename of the marked entries (or every listed entry when
    /// nothing is marked): the names are opened in the configured editor —
    /// or the built-in one — one per line.
    pub fn start_bulk_rename(&mut self) {
        let pane = self.active_pane();
        let sources: Vec<PathBuf> = if pane.marked.is_empty() {
            pane.entries.iter().map(|e| e.path.clone()).collect()
        } else {
            pane.entries
                .iter()
                .filter(|e| pane.marked.contains(&e.path))
                .map(|e| e.path.clone())
                .collect()
        };
        if sources.is_empty() {
            self.notify_error("Nothing to rename");
            return;
        }
        let content = match bulk_rename::names_file_content(&sources) {
            Ok(c) => c,
            Err(name) => {
                self.notify_error(format!(
                    "Cannot bulk-rename '{}': newline in name",
                    name.escape_debug()
                ));
                return;
            }
        };
        // Unique per session so concurrent instances (and tests) never collide.
        static SESSION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = SESSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let file =
            std::env::temp_dir().join(format!("tfe-bulk-rename-{}-{n}.txt", std::process::id()));
        if let Err(e) = std::fs::write(&file, content) {
            self.notify_error(format!("Cannot write temp file: {e}"));
            return;
        }
        self.bulk_rename = Some(BulkRenameSession {
            file: file.clone(),
            sources,
        });
        if self.editor == Editor::None {
            match InlineEditor::open(&file) {
                Ok(ed) => self.inline_editor = Some(ed),
                Err(e) => {
                    self.bulk_rename = None;
                    self.notify_error(format!("Cannot edit: {e}"));
                }
            }
        } else {
            self.open_with_editor = Some(file);
        }
        self.status_msg = "Edit the names, save, and quit the editor to apply.".into();
    }

    /// Tell the app an external editor session on `path` has ended.
    ///
    /// Hosts that handle [`AppOutcome::OpenEditor`](super::AppOutcome) (or
    /// [`App::take_editor_request`]) themselves must call this afterwards so
    /// a pending bulk rename is applied.
    pub fn editor_finished(&mut self, path: &Path) {
        if self.bulk_rename.as_ref().is_some_and(|s| s.file == path) {
            self.finish_bulk_rename();
        }
    }

    /// Apply the edited name list of the pending bulk rename, if any.
    pub(super) fn finish_bulk_rename(&mut self) {
        let Some(session) = self.bulk_rename.take() else {
            return;
        };
        let edited = std::fs::read_to_string(&session.file);
        let _ = std::fs::remove_file(&session.file);
        let edited = match edited {
            Ok(t) => t,
            Err(e) => {
                self.notify_error(format!("Bulk rename aborted: {e}"));
                return;
            }
        };
        let fs = self.active_pane().filesystem.clone();
        let steps = match bulk_rename::plan(&session.sources, &edited, |p| fs.exists(p)) {
            Ok(s) => s,
            Err(e) => {
                self.notify_error(format!("Bulk rename aborted: {e}"));
                return;
            }
        };
        if steps.is_empty() {
            self.notify("Bulk rename: no changes");
            return;
        }
        match bulk_rename::apply(fs.as_ref(), &steps) {
            Ok(done) => {
                let n = done.len();
                self.record_undo(UndoEntry::Batch(
                    done.into_iter()
                        .map(|s| UndoEntry::Rename {
                            from: s.from,
                            to: s.to,
                        })
                        .collect(),
                ));
                for p in &mut self.panes {
                    p.clear_marks();
                    p.reload();
                }
                self.preview_state.invalidate();
                self.notify(format!("Renamed {n} item(s)"));
            }
            Err(e) => self.notify_error(format!("Bulk rename failed (rolled back): {e}")),
        }
    }

    // ── Archives ──────────────────────────────────────────────────────────

    /// Extract the highlighted archive into a sibling folder named after it.
    #[cfg(feature = "archive")]
    pub fn extract_current_archive(&mut self) {
        let pane = self.active_pane();
        let Some(entry) = pane.current_entry() else {
            return;
        };
        let path = entry.path.clone();
        if entry.is_dir || crate::archive::ArchiveFormat::detect(&path).is_none() {
            self.notify_error("Not a supported archive (.zip, .tar, .tar.gz, .tgz)");
            return;
        }
        if !pane.filesystem.is_local() {
            self.notify_error("Extraction needs a local filesystem");
            return;
        }
        let dest = crate::archive::default_destination(&path);
        if dest.exists() {
            self.notify_error(format!("'{}' already exists", dest.display()));
            return;
        }
        self.start_extract(path, dest, false);
    }

    /// Run an extraction on a worker thread. `is_redo` keeps the redo stack.
    #[cfg(feature = "archive")]
    fn start_extract(&mut self, archive: PathBuf, dest: PathBuf, is_redo: bool) {
        self.status_msg = format!("Extracting '{}'…", archive.display());
        let target = dest.clone();
        let source = archive.clone();
        self.extract_task = Some(ExtractJob {
            archive,
            dest,
            is_redo,
            task: Task::spawn(move |emit| {
                let result = crate::archive::extract(&source, &target).map_err(|e| e.to_string());
                if result.is_err() {
                    // Don't leave a half-extracted folder behind.
                    let _ = std::fs::remove_dir_all(&target);
                }
                emit.send(result);
            }),
        });
    }

    // ── Git ───────────────────────────────────────────────────────────────

    /// Show or hide git change markers in every pane.
    pub fn toggle_git(&mut self) {
        let show = !self.active_pane().show_git;
        for p in &mut self.panes {
            p.set_show_git(show);
        }
        self.notify(format!("git markers: {}", if show { "on" } else { "off" }));
    }

    // ── Background polling ────────────────────────────────────────────────

    /// Collect finished background work. Returns `true` when the view should
    /// be redrawn. Called by [`draw`](crate::draw) once per frame.
    pub fn poll_background(&mut self) -> bool {
        let mut changed = false;
        for p in &mut self.panes {
            changed |= p.poll_background();
        }

        if let Some(task) = &mut self.picker_task {
            for msg in task.poll() {
                if let Some(picker) = &mut self.picker {
                    match msg {
                        WalkMsg::Items(items) => picker.extend(items),
                        WalkMsg::Truncated => picker.truncated = true,
                    }
                    changed = true;
                }
            }
            if task.is_done() {
                self.picker_task = None;
                if let Some(picker) = &mut self.picker {
                    picker.loading = false;
                }
                changed = true;
            }
        }

        #[cfg(feature = "archive")]
        if let Some(job) = &mut self.extract_task {
            let results = job.task.poll();
            let done = job.task.is_done();
            let (archive, dest, is_redo) = (job.archive.clone(), job.dest.clone(), job.is_redo);
            for result in results {
                match result {
                    Ok(n) => {
                        let entry = UndoEntry::Extract {
                            archive: archive.clone(),
                            dest: dest.clone(),
                        };
                        if is_redo {
                            self.push_undo(entry);
                        } else {
                            self.record_undo(entry);
                        }
                        for p in &mut self.panes {
                            p.reload();
                        }
                        self.notify(format!("Extracted {n} item(s) into '{}'", dest.display()));
                    }
                    Err(e) => self.notify_error(format!("Extract failed: {e}")),
                }
                changed = true;
            }
            if done {
                self.extract_task = None;
            }
        }

        if self.preview_state.poll() {
            changed = true;
        }
        changed
    }

    /// `true` while anything is still computing in the background — the
    /// event loop should wake periodically to redraw.
    #[must_use]
    pub fn has_background_work(&self) -> bool {
        #[cfg(feature = "archive")]
        if self.extract_task.is_some() {
            return true;
        }
        self.picker_task.is_some()
            || self.preview_state.is_loading()
            || self.panes.iter().any(FileExplorer::has_pending_work)
    }
}

#[cfg(test)]
mod tests;
