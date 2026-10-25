//! Dual-pane file-explorer widget.
//!
//! [`DualPane`] owns two independent [`FileExplorer`] instances — a left pane
//! and a right pane — and manages which one has keyboard focus.  It also
//! supports a **single-pane mode** where only the active pane is rendered,
//! matching the `tfe` binary's `w` toggle.
//!
//! ## Quick start
//!
//! ```
//! use tui_file_explorer::{DualPane, DualPaneOutcome, render_dual_pane_themed, Theme};
//! use crossterm::event::{Event, KeyCode, KeyModifiers, self};
//! # use ratatui::{Terminal, backend::TestBackend};
//! # let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
//!
//! let mut dual = DualPane::builder(std::env::current_dir().unwrap()).build();
//! let theme    = Theme::default();
//!
//! // Inside your Terminal::draw closure:
//! // terminal.draw(|frame| {
//! //     render_dual_pane_themed(&mut dual, frame, frame.area(), &theme);
//! // }).unwrap();
//!
//! // Inside your event loop:
//! # let key = crossterm::event::KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
//! match dual.handle_key(key) {
//!     DualPaneOutcome::Selected(path) => println!("chosen: {}", path.display()),
//!     DualPaneOutcome::Dismissed      => { /* close the overlay */ }
//!     _                               => {}
//! }
//! ```
//!
//! ## Key bindings added by `DualPane`
//!
//! All standard [`FileExplorer`] bindings work on the active pane.
//! `DualPane` intercepts the following additional keys **before** forwarding
//! to the active pane:
//!
//! | Key | Action |
//! |-----|--------|
//! | `Tab` | Switch focus between left and right pane |
//! | `w` | Toggle single-pane / dual-pane mode |
//!
//! ## Rendering
//!
//! Use [`crate::render_dual_pane`] (default theme) or
//! [`crate::render_dual_pane_themed`] (custom theme).  In single-pane mode
//! the full area is given to the active pane; in dual-pane mode the area is
//! split evenly in two.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{ExplorerCommand, ExplorerOutcome, FileExplorer, Keymap, SortMode};

// ── DualPaneActive ────────────────────────────────────────────────────────────

/// Which pane of a [`DualPane`] currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DualPaneActive {
    /// The left-hand pane has focus. This is the default.
    #[default]
    Left,
    /// The right-hand pane has focus.
    Right,
}

impl DualPaneActive {
    /// Return the opposite pane.
    ///
    /// ```
    /// use tui_file_explorer::DualPaneActive;
    ///
    /// assert_eq!(DualPaneActive::Left.other(),  DualPaneActive::Right);
    /// assert_eq!(DualPaneActive::Right.other(), DualPaneActive::Left);
    /// ```
    #[must_use]
    pub fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

// ── DualPaneOutcome ───────────────────────────────────────────────────────────

/// Outcome returned by [`DualPane::handle_key`].
///
/// Mirrors [`ExplorerOutcome`] but also carries which pane produced the event
/// so callers can act differently depending on the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualPaneOutcome {
    /// The user confirmed a file selection in the given pane.
    Selected(PathBuf),
    /// The user dismissed the explorer (`Esc` / `q`) from the given pane.
    Dismissed,
    /// A key was consumed but produced no navigational outcome yet.
    Pending,
    /// The key was not recognised by either the `DualPane` layer or the
    /// active [`FileExplorer`].
    Unhandled,
    /// A new directory was successfully created at the given path.
    MkdirCreated(PathBuf),
    /// A new empty file was successfully created at the given path.
    TouchCreated(PathBuf),
    /// An entry was successfully renamed; contains the new path.
    RenameCompleted { from: PathBuf, to: PathBuf },
}

impl From<ExplorerOutcome> for DualPaneOutcome {
    /// Filesystem-mutation outcomes keep their paths so callers can react
    /// (reload the inactive pane, show a status message, move focus, …).
    fn from(outcome: ExplorerOutcome) -> Self {
        match outcome {
            ExplorerOutcome::Selected(p) => Self::Selected(p),
            ExplorerOutcome::Dismissed => Self::Dismissed,
            ExplorerOutcome::Pending => Self::Pending,
            ExplorerOutcome::Unhandled => Self::Unhandled,
            ExplorerOutcome::MkdirCreated(p) => Self::MkdirCreated(p),
            ExplorerOutcome::TouchCreated(p) => Self::TouchCreated(p),
            ExplorerOutcome::RenameCompleted { from, to } => Self::RenameCompleted { from, to },
        }
    }
}

/// Semantic command accepted by [`DualPane::handle_command`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DualPaneCommand {
    Explorer(ExplorerCommand),
    NextPane,
    PreviousPane,
    FocusLeft,
    FocusRight,
    ToggleSinglePane,
}

// ── DualPane ─────────────────────────────────────────────────────────────────

/// A dual-pane file-explorer widget.
///
/// Owns two independent [`FileExplorer`] instances.  Use
/// [`DualPane::builder`] to construct one, [`DualPane::handle_key`] to drive
/// it from your event loop, and [`crate::render_dual_pane_themed`] to draw it.
///
/// # Example
///
/// ```
/// use tui_file_explorer::{DualPane, SortMode};
///
/// let mut dual = DualPane::builder(std::env::current_dir().unwrap())
///     .right_dir(std::path::PathBuf::from("/tmp"))
///     .show_hidden(true)
///     .sort_mode(SortMode::SizeDesc)
///     .single_pane(false)
///     .build();
///
/// // Focus starts on the left pane.
/// assert_eq!(dual.active().current_dir, dual.left.current_dir);
/// ```
#[derive(Debug)]
pub struct DualPane {
    /// The left-hand explorer pane.
    pub left: FileExplorer,
    /// The right-hand explorer pane.
    pub right: FileExplorer,
    /// Which pane currently has keyboard focus.
    pub active_side: DualPaneActive,
    /// When `true` only the active pane is rendered (full-width).
    /// Toggle at runtime with the `w` key.
    pub single_pane: bool,
}

impl DualPane {
    // ── Construction ─────────────────────────────────────────────────────────

    /// Return a [`DualPaneBuilder`] rooted at `left_dir`.
    ///
    /// The right pane mirrors `left_dir` unless you call
    /// [`DualPaneBuilder::right_dir`].
    ///
    /// ```
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap()).build();
    /// ```
    #[must_use]
    pub fn builder(left_dir: PathBuf) -> DualPaneBuilder {
        DualPaneBuilder::new(left_dir)
    }

    // ── Pane accessors ────────────────────────────────────────────────────────

    /// Return a shared reference to the currently active pane.
    #[must_use]
    pub fn active(&self) -> &FileExplorer {
        match self.active_side {
            DualPaneActive::Left => &self.left,
            DualPaneActive::Right => &self.right,
        }
    }

    /// Return a mutable reference to the currently active pane.
    pub fn active_mut(&mut self) -> &mut FileExplorer {
        match self.active_side {
            DualPaneActive::Left => &mut self.left,
            DualPaneActive::Right => &mut self.right,
        }
    }

    /// Return a shared reference to the inactive (background) pane.
    #[must_use]
    pub fn inactive(&self) -> &FileExplorer {
        match self.active_side {
            DualPaneActive::Left => &self.right,
            DualPaneActive::Right => &self.left,
        }
    }

    // ── Key handling ─────────────────────────────────────────────────────────

    /// Dispatch a semantic command without coupling host code to key bindings.
    pub fn handle_command(&mut self, command: DualPaneCommand) -> DualPaneOutcome {
        match command {
            DualPaneCommand::Explorer(command) => self.active_mut().handle_command(command).into(),
            DualPaneCommand::NextPane | DualPaneCommand::PreviousPane => {
                self.active_side = self.active_side.other();
                DualPaneOutcome::Pending
            }
            DualPaneCommand::FocusLeft => {
                self.focus_left();
                DualPaneOutcome::Pending
            }
            DualPaneCommand::FocusRight => {
                self.focus_right();
                DualPaneOutcome::Pending
            }
            DualPaneCommand::ToggleSinglePane => {
                self.toggle_single_pane();
                DualPaneOutcome::Pending
            }
        }
    }

    /// Process a single keyboard event and return a [`DualPaneOutcome`].
    ///
    /// `DualPane` intercepts `Tab` (switch pane) and `w` (toggle single/dual
    /// mode) before forwarding everything else to the active [`FileExplorer`].
    ///
    /// ```
    /// use tui_file_explorer::{DualPane, DualPaneOutcome};
    /// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    ///
    /// let mut dual = DualPane::builder(std::env::current_dir().unwrap()).build();
    /// let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    ///
    /// // Tab switches focus — outcome is Pending.
    /// assert_eq!(dual.handle_key(key), DualPaneOutcome::Pending);
    /// ```
    pub fn handle_key(&mut self, key: KeyEvent) -> DualPaneOutcome {
        press_only!(key, DualPaneOutcome::Pending);

        // ── DualPane-level keys (handled before the active pane sees them) ────
        match key.code {
            // Switch active pane.
            KeyCode::Tab if key.modifiers.is_empty() => {
                self.active_side = self.active_side.other();
                return DualPaneOutcome::Pending;
            }
            // Toggle single / dual-pane mode.
            KeyCode::Char('w') if key.modifiers.is_empty() => {
                self.single_pane = !self.single_pane;
                return DualPaneOutcome::Pending;
            }
            _ => {}
        }

        // Delegate everything else to the active pane.
        self.active_mut().handle_key(key).into()
    }

    // ── Convenience mutators ─────────────────────────────────────────────────

    /// Switch focus to the left pane.
    pub fn focus_left(&mut self) {
        self.active_side = DualPaneActive::Left;
    }

    /// Switch focus to the right pane.
    pub fn focus_right(&mut self) {
        self.active_side = DualPaneActive::Right;
    }

    /// Toggle between single-pane and dual-pane mode.
    pub fn toggle_single_pane(&mut self) {
        self.single_pane = !self.single_pane;
    }

    /// Reload both panes from the filesystem.
    ///
    /// Call this after external filesystem mutations that may affect either
    /// pane's directory listing (e.g. after a copy, move, or delete).
    pub fn reload_both(&mut self) {
        self.left.reload();
        self.right.reload();
    }
}

// ── DualPaneBuilder ───────────────────────────────────────────────────────────

/// Builder for [`DualPane`].
///
/// Construct via [`DualPane::builder`].
///
/// # Example
///
/// ```
/// use tui_file_explorer::{DualPane, SortMode};
/// use std::path::PathBuf;
///
/// let dual = DualPane::builder(PathBuf::from("/home/user"))
///     .right_dir(PathBuf::from("/tmp"))
///     .show_hidden(true)
///     .sort_mode(SortMode::Extension)
///     .single_pane(false)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct DualPaneBuilder {
    left_dir: PathBuf,
    right_dir: Option<PathBuf>,
    extensions: Vec<String>,
    show_hidden: bool,
    show_sizes: bool,
    show_dir_sizes: bool,
    keymap: Keymap,
    sort_mode: SortMode,
    selection_mode: crate::SelectionMode,
    filesystem: crate::SharedFileSystem,
    single_pane: bool,
    page_size: usize,
}

impl DualPaneBuilder {
    /// Create a new builder with `left_dir` as the starting directory for the
    /// left pane.  The right pane mirrors `left_dir` until
    /// [`right_dir`](Self::right_dir) is called.
    #[must_use]
    pub fn new(left_dir: PathBuf) -> Self {
        Self {
            left_dir,
            right_dir: None,
            extensions: Vec::new(),
            show_hidden: false,
            show_sizes: true,
            show_dir_sizes: true,
            keymap: Keymap::default(),
            sort_mode: SortMode::default(),
            selection_mode: crate::SelectionMode::default(),
            filesystem: crate::std_filesystem(),
            single_pane: false,
            page_size: 10,
        }
    }

    /// Set an independent starting directory for the right pane.
    ///
    /// If not called, the right pane starts in the same directory as the left.
    ///
    /// ```
    /// use tui_file_explorer::DualPane;
    /// use std::path::PathBuf;
    ///
    /// let dual = DualPane::builder(PathBuf::from("/home"))
    ///     .right_dir(PathBuf::from("/tmp"))
    ///     .build();
    ///
    /// assert_eq!(dual.right.current_dir, PathBuf::from("/tmp"));
    /// ```
    #[must_use]
    pub fn right_dir(mut self, dir: PathBuf) -> Self {
        self.right_dir = Some(dir);
        self
    }

    /// Set the file-extension filter applied to both panes.
    ///
    /// Only files whose extension is in the list are selectable; directories
    /// are always navigable.  An empty list means all files are selectable
    /// (the default).
    ///
    /// ```
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .extension_filter(vec!["rs".into(), "toml".into()])
    ///     .build();
    /// ```
    #[must_use]
    pub fn extension_filter(mut self, filter: Vec<String>) -> Self {
        self.extensions = filter;
        self
    }

    /// Append a single allowed extension (can be called multiple times).
    ///
    /// ```
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .allow_extension("rs")
    ///     .allow_extension("toml")
    ///     .build();
    /// ```
    #[must_use]
    pub fn allow_extension(mut self, ext: impl Into<String>) -> Self {
        self.extensions.push(ext.into());
        self
    }

    /// Set whether hidden (dot-file) entries are visible on startup in both
    /// panes.
    ///
    /// ```
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .show_hidden(true)
    ///     .build();
    ///
    /// assert!(dual.left.show_hidden);
    /// assert!(dual.right.show_hidden);
    /// ```
    #[must_use]
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// Set whether file and directory sizes are rendered in both panes.
    #[must_use]
    pub fn show_sizes(mut self, show: bool) -> Self {
        self.show_sizes = show;
        self
    }

    /// Use `keymap` in both panes.
    #[must_use]
    pub fn keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Set whether directories show recursive sizes (vs. item counts) in both panes.
    #[must_use]
    pub fn show_dir_sizes(mut self, show: bool) -> Self {
        self.show_dir_sizes = show;
        self
    }

    /// Set which entry kinds can be confirmed in both panes.
    #[must_use]
    pub fn selection_mode(mut self, mode: crate::SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Use a custom filesystem backend for both panes.
    #[must_use]
    pub fn filesystem(mut self, filesystem: crate::SharedFileSystem) -> Self {
        self.filesystem = filesystem;
        self
    }

    /// Set the initial sort mode for both panes.
    ///
    /// ```
    /// use tui_file_explorer::{DualPane, SortMode};
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .sort_mode(SortMode::SizeDesc)
    ///     .build();
    ///
    /// assert_eq!(dual.left.sort_mode, SortMode::SizeDesc);
    /// assert_eq!(dual.right.sort_mode, SortMode::SizeDesc);
    /// ```
    #[must_use]
    pub fn sort_mode(mut self, mode: SortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// Start in single-pane mode (default: `false`).
    ///
    /// When `true`, only the active pane is rendered.  The `w` key toggles
    /// this at runtime.
    ///
    /// ```
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .single_pane(true)
    ///     .build();
    ///
    /// assert!(dual.single_pane);
    /// ```
    #[must_use]
    pub fn single_pane(mut self, enabled: bool) -> Self {
        self.single_pane = enabled;
        self
    }

    /// Set the number of entries scrolled by Page Up / Page Down in each pane.
    ///
    /// Defaults to 10.
    #[must_use]
    pub fn page_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }

    /// Consume the builder and return a fully initialised [`DualPane`].
    #[must_use]
    pub fn build(self) -> DualPane {
        let right_dir = self.right_dir.unwrap_or_else(|| self.left_dir.clone());

        let left = FileExplorer::builder(self.left_dir)
            .filesystem(self.filesystem.clone())
            .extension_filter(self.extensions.clone())
            .show_hidden(self.show_hidden)
            .show_sizes(self.show_sizes)
            .show_dir_sizes(self.show_dir_sizes)
            .keymap(self.keymap.clone())
            .selection_mode(self.selection_mode)
            .sort_mode(self.sort_mode)
            .page_size(self.page_size)
            .build();

        let right = FileExplorer::builder(right_dir)
            .filesystem(self.filesystem)
            .extension_filter(self.extensions)
            .show_hidden(self.show_hidden)
            .show_sizes(self.show_sizes)
            .show_dir_sizes(self.show_dir_sizes)
            .keymap(self.keymap)
            .selection_mode(self.selection_mode)
            .sort_mode(self.sort_mode)
            .page_size(self.page_size)
            .build();

        DualPane {
            left,
            right,
            active_side: DualPaneActive::Left,
            single_pane: self.single_pane,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
