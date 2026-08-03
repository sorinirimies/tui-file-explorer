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
    extension_filter: Vec<String>,
    show_hidden: bool,
    sort_mode: SortMode,
    page_size: usize,
    show_sizes: bool,
}

impl FileExplorerBuilder {
    /// Create a builder rooted at `initial_dir`.
    pub fn new(initial_dir: PathBuf) -> Self {
        Self {
            initial_dir,
            extension_filter: Vec::new(),
            show_hidden: false,
            sort_mode: SortMode::default(),
            page_size: PAGE_SIZE,
            show_sizes: true,
        }
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
    pub fn show_sizes(mut self, show: bool) -> Self {
        self.show_sizes = show;
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
    pub fn sort_mode(mut self, mode: SortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// Set the number of entries scrolled by Page Up / Page Down.
    ///
    /// Defaults to 10.
    pub fn page_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }

    /// Consume the builder and return a fully initialised [`FileExplorer`].
    pub fn build(self) -> FileExplorer {
        let mut explorer = FileExplorer {
            current_dir: self.initial_dir,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            extension_filter: self.extension_filter,
            show_hidden: self.show_hidden,
            status: String::new(),
            sort_mode: self.sort_mode,
            page_size: self.page_size,
            search_query: String::new(),
            search_active: false,
            marked: HashSet::new(),
            mkdir_active: false,
            mkdir_input: String::new(),
            touch_active: false,
            touch_input: String::new(),
            rename_active: false,
            rename_input: String::new(),
            theme_name: String::new(),
            editor_name: String::new(),
            disk_usage: None,
            dir_size_cache: std::collections::HashMap::new(),
            show_sizes: self.show_sizes,
        };
        explorer.reload();
        explorer
    }
}
