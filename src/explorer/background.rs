//! Off-thread work for the explorer: lazy item counts, recursive folder
//! sizes and `git status`.
//!
//! Rendering asks for what the *visible* rows need; slow answers are computed
//! on worker threads and picked up by [`FileExplorer::poll_background`] on a
//! later frame. Until then the row shows a cheap placeholder (an item count),
//! so the UI never blocks on a slow disk.

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    time::{Duration, Instant},
};

use super::*;
use crate::{git, worker::Task};

/// Folder-size walks allowed to run at once.
const MAX_CONCURRENT_DIR_SIZES: usize = 4;

/// Result of one folder-size walk: `(bytes, is_partial)`.
type DirSize = (u64, bool);

/// Cursor/selection to restore once a (possibly deferred) listing arrives.
#[derive(Debug, Default, Clone)]
pub(crate) struct AfterLoad {
    /// Put the cursor on this path if it is in the new listing.
    pub select: Option<PathBuf>,
    /// Otherwise keep the cursor on this row index (clamped).
    pub cursor: Option<usize>,
}

/// Outcome of reading a directory off-thread.
type LoadResult = std::io::Result<(Vec<FsEntry>, Option<crate::DiskUsage>)>;

/// A directory read running on a worker thread.
#[derive(Debug)]
pub(crate) struct LoadJob {
    task: Task<LoadResult>,
    after: AfterLoad,
}

/// How long [`FileExplorer::reload`] waits for a background read before
/// giving up the foreground and showing a "loading…" state. Fast reads (the
/// overwhelming majority) therefore complete synchronously with no flicker.
const LOAD_GRACE: Duration = Duration::from_millis(25);

/// In-flight and bookkeeping state; owned by [`FileExplorer`].
#[derive(Debug, Default)]
pub(crate) struct Background {
    /// Running folder-size walks, with the item count they were started at
    /// (the cache's staleness key).
    dir_size_jobs: HashMap<PathBuf, (Task<DirSize>, usize)>,
    /// Directories whose item count was already looked up since the last
    /// reload (so unreadable directories are not retried every frame).
    counted: HashSet<PathBuf>,
    git_task: Option<Task<Option<git::GitStatus>>>,
    /// Running directory read, when asynchronous loading is enabled.
    load: Option<LoadJob>,
}

impl Background {
    pub(crate) fn clear_counted(&mut self) {
        self.counted.clear();
    }

    pub(crate) fn clear_dir_sizes(&mut self) {
        self.dir_size_jobs.clear();
    }

    pub(crate) fn cancel_load(&mut self) {
        self.load = None;
    }
}

impl FileExplorer {
    /// Fill in [`FsEntry::item_count`] for the directories in `range`.
    ///
    /// One cheap `read_dir` per visible directory — instead of one per entry
    /// of a possibly enormous listing at load time.
    pub(crate) fn fill_item_counts(&mut self, range: Range<usize>) {
        for idx in range {
            let Some(entry) = self.entries.get(idx) else {
                break;
            };
            if !entry.is_dir || entry.item_count.is_some() {
                continue;
            }
            let path = entry.path.clone();
            if !self.bg.counted.insert(path.clone()) {
                continue;
            }
            let count = self.filesystem.item_count(&path);
            if let Some(entry) = self.entries.get_mut(idx) {
                entry.item_count = count;
            }
        }
    }

    /// Start recursive-size walks for uncached directories in `range`.
    ///
    /// Call [`fill_item_counts`](Self::fill_item_counts) first: the count is
    /// the cache's staleness key.
    pub(crate) fn request_dir_sizes(&mut self, range: Range<usize>) {
        for idx in range {
            if self.bg.dir_size_jobs.len() >= MAX_CONCURRENT_DIR_SIZES {
                break;
            }
            let Some(entry) = self.entries.get(idx) else {
                break;
            };
            let (true, Some(count)) = (entry.is_dir, entry.item_count) else {
                continue;
            };
            let path = &entry.path;
            if self.bg.dir_size_jobs.contains_key(path) {
                continue;
            }
            if self
                .dir_size_cache
                .get(path)
                .is_some_and(|&(_, _, cached)| cached == count)
            {
                continue;
            }
            let fs = std::sync::Arc::clone(&self.filesystem);
            let walk_path = path.clone();
            let task = Task::spawn(move |emit| {
                emit.send(fs.dir_size(&walk_path));
            });
            self.bg.dir_size_jobs.insert(path.clone(), (task, count));
        }
    }

    // ── Directory loading ─────────────────────────────────────────────────

    /// `true` while the directory listing is still being read in the
    /// background (see [`FileExplorer::async_loading`]).
    #[must_use]
    pub fn is_loading(&self) -> bool {
        self.bg.load.is_some()
    }

    /// Re-read the directory, then restore the cursor as described by
    /// `after` — immediately when the read is quick (or loading is
    /// synchronous), otherwise once it finishes.
    pub(crate) fn reload_with(&mut self, after: AfterLoad) {
        if !self.async_loading {
            self.reload_sync(&after);
            return;
        }

        let fs = std::sync::Arc::clone(&self.filesystem);
        let dir = self.current_dir.clone();
        let (show_hidden, filter, sort, query) = (
            self.show_hidden,
            self.extension_filter.clone(),
            self.sort_mode,
            self.search_query.clone(),
        );
        // Replacing the job drops (cancels) any read still in flight.
        let mut task = Task::spawn(move |emit| {
            let result = try_load_entries(fs.as_ref(), &dir, show_hidden, &filter, sort, &query)
                .map(|entries| (entries, fs.disk_usage(&dir)));
            emit.send(result);
        });
        self.status.clear();

        if let Some(result) = task.wait_next(LOAD_GRACE) {
            self.bg.load = None;
            self.apply_load(result, &after);
        } else {
            // Slow disk: don't block the UI. Show an empty "loading" list
            // (so no key can act on stale rows) and finish in `poll_background`.
            self.entries.clear();
            self.bg.load = Some(LoadJob { task, after });
        }
    }

    /// Synchronous `reload` body, shared with the non-async path.
    fn reload_sync(&mut self, after: &AfterLoad) {
        self.bg.cancel_load();
        self.status.clear();
        let result = self.read_dir_now();
        self.apply_load(result, after);
    }

    fn read_dir_now(&self) -> LoadResult {
        try_load_entries(
            self.filesystem.as_ref(),
            &self.current_dir,
            self.show_hidden,
            &self.extension_filter,
            self.sort_mode,
            &self.search_query,
        )
        .map(|entries| (entries, self.filesystem.disk_usage(&self.current_dir)))
    }

    /// Install a finished listing (or its error) and restore the cursor.
    fn apply_load(&mut self, result: LoadResult, after: &AfterLoad) {
        match result {
            Ok((entries, usage)) => {
                self.entries = entries;
                self.disk_usage = usage;
                self.link_cache.clear();
                self.bg.clear_counted();
                self.refresh_git();
                if let Some(row) = after.cursor {
                    self.cursor = row;
                }
                // The list may have shrunk (filter change, external deletion).
                self.clamp_cursor();
                if let Some(path) = &after.select {
                    self.select_path(path);
                }
            }
            Err(source) => {
                let error = ExplorerError {
                    path: self.current_dir.clone(),
                    source,
                };
                self.entries.clear();
                self.disk_usage = None;
                self.status = error.to_string();
                self.clamp_cursor();
            }
        }
    }

    /// Collect finished background results. Returns `true` when something
    /// changed and the view should be redrawn.
    pub fn poll_background(&mut self) -> bool {
        let mut changed = false;

        // The directory listing comes first: everything else is about it.
        if let Some(job) = &mut self.bg.load {
            let results = job.task.poll();
            if !results.is_empty() || job.task.is_done() {
                let job = self.bg.load.take().expect("checked above");
                for result in results {
                    self.apply_load(result, &job.after);
                }
                changed = true;
            }
        }

        let mut finished = Vec::new();
        for (path, (task, count)) in &mut self.bg.dir_size_jobs {
            for (bytes, partial) in task.poll() {
                self.dir_size_cache
                    .insert(path.clone(), (bytes, partial, *count));
                changed = true;
            }
            if task.is_done() {
                finished.push(path.clone());
            }
        }
        for path in finished {
            self.bg.dir_size_jobs.remove(&path);
        }

        if let Some(task) = &mut self.bg.git_task {
            for status in task.poll() {
                self.git = status;
                changed = true;
            }
            if task.is_done() {
                self.bg.git_task = None;
            }
        }
        changed
    }

    /// `true` while any background computation is still running.
    #[must_use]
    pub fn has_pending_work(&self) -> bool {
        self.bg.load.is_some() || !self.bg.dir_size_jobs.is_empty() || self.bg.git_task.is_some()
    }

    /// Block until background work finishes or `timeout` elapses.
    ///
    /// Intended for tests and headless callers; interactive UIs should call
    /// [`poll_background`](Self::poll_background) once per frame instead.
    pub fn wait_for_background(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            self.poll_background();
            if !self.has_pending_work() || Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    // ── git ───────────────────────────────────────────────────────────────

    /// (Re)start a `git status` for the current directory when git markers
    /// are enabled and the backend is a real filesystem.
    pub(crate) fn refresh_git(&mut self) {
        if !self.show_git || !self.filesystem.is_local() {
            self.bg.git_task = None;
            return;
        }
        let dir = self.current_dir.clone();
        self.bg.git_task = Some(Task::spawn(move |emit| {
            emit.send(git::status_for(&dir));
        }));
    }

    /// Turn the git change column on or off.
    pub fn set_show_git(&mut self, show: bool) {
        self.show_git = show;
        if show {
            self.refresh_git();
        } else {
            self.git = None;
            self.bg.git_task = None;
        }
    }

    /// The git marker for the entry at `path`, if any.
    #[must_use]
    pub fn git_mark(&self, path: &Path) -> Option<git::GitMark> {
        self.git.as_ref()?.mark(path)
    }
}
