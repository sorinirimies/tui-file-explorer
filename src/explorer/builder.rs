use super::*;

// ── FileExplorerBuilder ───────────────────────────────────────────────────────

/// Builder for [`FileExplorer`].
///
/// Obtain one via [`FileExplorer::builder`].
///
/// # Example
///
/// ```no_run
/// use tui_file_explorer::{FileExplorer, SortMode};
///
/// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
///     .allow_extension("iso")
///     .allow_extension("img")
///     .show_hidden(false)
///     .sort_mode(SortMode::SizeDesc)
///     .build();
/// ```
pub struct FileExplorerBuilder {
    initial_dir: PathBuf,
    filesystem: crate::SharedFileSystem,
    extension_filter: Vec<String>,
    show_hidden: bool,
    sort_mode: SortMode,
    page_size: usize,
    selection_mode: SelectionMode,
    show_sizes: bool,
    show_dir_sizes: bool,
    keymap: Keymap,
    show_git: bool,
    async_loading: bool,
}

impl FileExplorerBuilder {
    /// Create a builder rooted at `initial_dir`.
    #[must_use]
    pub fn new(initial_dir: PathBuf) -> Self {
        Self {
            initial_dir,
            filesystem: crate::std_filesystem(),
            extension_filter: Vec::new(),
            show_hidden: false,
            sort_mode: SortMode::default(),
            page_size: PAGE_SIZE,
            selection_mode: SelectionMode::default(),
            show_sizes: true,
            show_dir_sizes: true,
            keymap: Keymap::default(),
            show_git: false,
            async_loading: false,
        }
    }

    /// Use a custom filesystem backend.
    #[must_use]
    pub fn filesystem(mut self, filesystem: crate::SharedFileSystem) -> Self {
        self.filesystem = filesystem;
        self
    }

    /// Set the full extension filter list at once.
    ///
    /// Replaces any extensions added with [`allow_extension`](Self::allow_extension).
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .extension_filter(vec!["iso".into(), "img".into()])
    ///     .build();
    /// ```
    #[must_use]
    pub fn extension_filter(mut self, filter: Vec<String>) -> Self {
        self.extension_filter = filter;
        self
    }

    /// Append a single allowed extension.
    ///
    /// Call multiple times to build up the filter:
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .allow_extension("iso")
    ///     .allow_extension("img")
    ///     .build();
    /// ```
    #[must_use]
    pub fn allow_extension(mut self, ext: impl Into<String>) -> Self {
        self.extension_filter.push(ext.into());
        self
    }

    /// Set whether hidden (dot-file) entries are shown on startup.
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .show_hidden(true)
    ///     .build();
    /// ```
    #[must_use]
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// Set whether file/folder sizes are shown on startup.
    ///
    /// Defaults to `true`. Set to `false` for the snappiest possible
    /// browsing of huge directory trees, since it skips the recursive
    /// directory-size walk entirely.
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .show_sizes(false)
    ///     .build();
    /// ```
    #[must_use]
    pub fn show_sizes(mut self, show: bool) -> Self {
        self.show_sizes = show;
        self
    }

    /// Set whether directories show their recursive byte size on startup.
    ///
    /// Defaults to `true`. With `false`, files still show their size but
    /// directories show a shallow item count and the (potentially slow)
    /// recursive walk is skipped. Has no effect when
    /// [`show_sizes`](Self::show_sizes) is `false`.
    ///
    /// ```no_run
    /// use tui_file_explorer::FileExplorer;
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .show_dir_sizes(false)
    ///     .build();
    /// ```
    #[must_use]
    pub fn show_dir_sizes(mut self, show: bool) -> Self {
        self.show_dir_sizes = show;
        self
    }

    /// Read directories on a background thread (see
    /// [`FileExplorer::async_loading`]). Off by default.
    #[must_use]
    pub fn async_loading(mut self, enabled: bool) -> Self {
        self.async_loading = enabled;
        self
    }

    /// Show a git change marker (`M`, `A`, `?`, …) next to changed entries.
    ///
    /// Off by default; runs `git status` on a background thread.
    #[must_use]
    pub fn show_git(mut self, show: bool) -> Self {
        self.show_git = show;
        self
    }

    /// Replace the key bindings (defaults to [`Keymap::default`]).
    #[must_use]
    pub fn keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Set the initial sort mode.
    ///
    /// ```no_run
    /// use tui_file_explorer::{FileExplorer, SortMode};
    ///
    /// let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    ///     .sort_mode(SortMode::SizeDesc)
    ///     .build();
    /// ```
    #[must_use]
    pub fn sort_mode(mut self, mode: SortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// Configure which entry kinds can be confirmed as selections.
    #[must_use]
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Set the number of entries scrolled by Page Up / Page Down.
    ///
    /// Defaults to 10.
    #[must_use]
    pub fn page_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }

    /// Consume the builder and return a fully initialised [`FileExplorer`].
    ///
    /// Filesystem errors are retained as explorer status for backwards
    /// compatibility. Use [`Self::try_build`] when the host needs typed errors.
    #[must_use]
    pub fn build(self) -> FileExplorer {
        let mut explorer = self.build_unloaded();
        explorer.reload();
        explorer
    }

    /// Build an explorer and return an error when its initial directory cannot
    /// be read.
    pub fn try_build(self) -> Result<FileExplorer, ExplorerError> {
        let mut explorer = self.build_unloaded();
        explorer.try_reload()?;
        Ok(explorer)
    }

    fn build_unloaded(self) -> FileExplorer {
        FileExplorer {
            current_dir: self.initial_dir,
            filesystem: self.filesystem,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            extension_filter: self.extension_filter,
            show_hidden: self.show_hidden,
            status: String::new(),
            sort_mode: self.sort_mode,
            page_size: self.page_size,
            selection_mode: self.selection_mode,
            search_query: String::new(),
            search_active: false,
            marked: HashSet::new(),
            prompt: None,
            keymap: self.keymap,
            theme_name: String::new(),
            editor_name: String::new(),
            disk_usage: None,
            bg: Default::default(),
            show_git: self.show_git,
            async_loading: self.async_loading,
            git: None,
            link_cache: std::collections::HashMap::new(),
            dir_size_cache: std::collections::HashMap::new(),
            show_sizes: self.show_sizes,
            show_dir_sizes: self.show_dir_sizes,
        }
    }
}
