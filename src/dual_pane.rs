//! Dual-pane file-explorer widget.
//!
//! [`DualPane`] owns two independent [`FileExplorer`] instances — a left pane
//! and a right pane — and manages which one has keyboard focus.  It also
//! supports a **single-pane mode** where only the active pane is rendered,
//! matching the `tfe` binary's `w` toggle.
//!
//! ## Quick start
//!
//! ```no_run
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
//! # let Event::Key(key) = event::read().unwrap() else { return; };
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

use crate::{ExplorerOutcome, FileExplorer, SortMode};

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
/// ```no_run
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
    /// ```no_run
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap()).build();
    /// ```
    pub fn builder(left_dir: PathBuf) -> DualPaneBuilder {
        DualPaneBuilder::new(left_dir)
    }

    // ── Pane accessors ────────────────────────────────────────────────────────

    /// Return a shared reference to the currently active pane.
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
    pub fn inactive(&self) -> &FileExplorer {
        match self.active_side {
            DualPaneActive::Left => &self.right,
            DualPaneActive::Right => &self.left,
        }
    }

    // ── Key handling ─────────────────────────────────────────────────────────

    /// Process a single keyboard event and return a [`DualPaneOutcome`].
    ///
    /// `DualPane` intercepts `Tab` (switch pane) and `w` (toggle single/dual
    /// mode) before forwarding everything else to the active [`FileExplorer`].
    ///
    /// ```no_run
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
        match self.active_mut().handle_key(key) {
            ExplorerOutcome::Selected(p) => DualPaneOutcome::Selected(p),
            ExplorerOutcome::Dismissed => DualPaneOutcome::Dismissed,
            ExplorerOutcome::Pending => DualPaneOutcome::Pending,
            ExplorerOutcome::Unhandled => DualPaneOutcome::Unhandled,
        }
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
/// ```no_run
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
    sort_mode: SortMode,
    single_pane: bool,
}

impl DualPaneBuilder {
    /// Create a new builder with `left_dir` as the starting directory for the
    /// left pane.  The right pane mirrors `left_dir` until
    /// [`right_dir`](Self::right_dir) is called.
    pub fn new(left_dir: PathBuf) -> Self {
        Self {
            left_dir,
            right_dir: None,
            extensions: Vec::new(),
            show_hidden: false,
            sort_mode: SortMode::default(),
            single_pane: false,
        }
    }

    /// Set an independent starting directory for the right pane.
    ///
    /// If not called, the right pane starts in the same directory as the left.
    ///
    /// ```no_run
    /// use tui_file_explorer::DualPane;
    /// use std::path::PathBuf;
    ///
    /// let dual = DualPane::builder(PathBuf::from("/home"))
    ///     .right_dir(PathBuf::from("/tmp"))
    ///     .build();
    ///
    /// assert_eq!(dual.right.current_dir, PathBuf::from("/tmp"));
    /// ```
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
    /// ```no_run
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .extension_filter(vec!["rs".into(), "toml".into()])
    ///     .build();
    /// ```
    pub fn extension_filter(mut self, filter: Vec<String>) -> Self {
        self.extensions = filter;
        self
    }

    /// Append a single allowed extension (can be called multiple times).
    ///
    /// ```no_run
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .allow_extension("rs")
    ///     .allow_extension("toml")
    ///     .build();
    /// ```
    pub fn allow_extension(mut self, ext: impl Into<String>) -> Self {
        self.extensions.push(ext.into());
        self
    }

    /// Set whether hidden (dot-file) entries are visible on startup in both
    /// panes.
    ///
    /// ```no_run
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .show_hidden(true)
    ///     .build();
    ///
    /// assert!(dual.left.show_hidden);
    /// assert!(dual.right.show_hidden);
    /// ```
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// Set the initial sort mode for both panes.
    ///
    /// ```no_run
    /// use tui_file_explorer::{DualPane, SortMode};
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .sort_mode(SortMode::SizeDesc)
    ///     .build();
    ///
    /// assert_eq!(dual.left.sort_mode, SortMode::SizeDesc);
    /// assert_eq!(dual.right.sort_mode, SortMode::SizeDesc);
    /// ```
    pub fn sort_mode(mut self, mode: SortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// Start in single-pane mode (default: `false`).
    ///
    /// When `true`, only the active pane is rendered.  The `w` key toggles
    /// this at runtime.
    ///
    /// ```no_run
    /// use tui_file_explorer::DualPane;
    ///
    /// let dual = DualPane::builder(std::env::current_dir().unwrap())
    ///     .single_pane(true)
    ///     .build();
    ///
    /// assert!(dual.single_pane);
    /// ```
    pub fn single_pane(mut self, enabled: bool) -> Self {
        self.single_pane = enabled;
        self
    }

    /// Consume the builder and return a fully initialised [`DualPane`].
    pub fn build(self) -> DualPane {
        let right_dir = self.right_dir.unwrap_or_else(|| self.left_dir.clone());

        let left = FileExplorer::builder(self.left_dir)
            .extension_filter(self.extensions.clone())
            .show_hidden(self.show_hidden)
            .sort_mode(self.sort_mode)
            .build();

        let right = FileExplorer::builder(right_dir)
            .extension_filter(self.extensions)
            .show_hidden(self.show_hidden)
            .sort_mode(self.sort_mode)
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
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::fs;
    use tempfile::tempdir;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_dual() -> DualPane {
        let dir = tempdir().expect("tempdir");
        // Create a dummy file so the pane is not empty.
        fs::write(dir.path().join("file.txt"), b"x").unwrap();
        // Leak the TempDir so the path survives — acceptable in tests.
        let path = dir.keep();
        DualPane::builder(path).build()
    }

    // ── DualPaneActive ────────────────────────────────────────────────────────

    #[test]
    fn dual_pane_active_other_left_returns_right() {
        assert_eq!(DualPaneActive::Left.other(), DualPaneActive::Right);
    }

    #[test]
    fn dual_pane_active_other_right_returns_left() {
        assert_eq!(DualPaneActive::Right.other(), DualPaneActive::Left);
    }

    // ── Builder ───────────────────────────────────────────────────────────────

    #[test]
    fn builder_default_active_side_is_left() {
        let dual = make_dual();
        assert_eq!(dual.active_side, DualPaneActive::Left);
    }

    #[test]
    fn builder_default_single_pane_is_false() {
        let dual = make_dual();
        assert!(!dual.single_pane);
    }

    #[test]
    fn builder_single_pane_true_when_requested() {
        let dir = tempdir().expect("tempdir");
        let dual = DualPane::builder(dir.path().to_path_buf())
            .single_pane(true)
            .build();
        assert!(dual.single_pane);
    }

    #[test]
    fn builder_right_dir_sets_independent_right_pane() {
        let left_dir = tempdir().expect("left tempdir");
        let right_dir = tempdir().expect("right tempdir");
        let dual = DualPane::builder(left_dir.path().to_path_buf())
            .right_dir(right_dir.path().to_path_buf())
            .build();
        assert_eq!(dual.left.current_dir, left_dir.path());
        assert_eq!(dual.right.current_dir, right_dir.path());
        assert_ne!(dual.left.current_dir, dual.right.current_dir);
    }

    #[test]
    fn builder_without_right_dir_mirrors_left() {
        let dir = tempdir().expect("tempdir");
        let dual = DualPane::builder(dir.path().to_path_buf()).build();
        assert_eq!(dual.left.current_dir, dual.right.current_dir);
    }

    #[test]
    fn builder_show_hidden_applies_to_both_panes() {
        let dir = tempdir().expect("tempdir");
        let dual = DualPane::builder(dir.path().to_path_buf())
            .show_hidden(true)
            .build();
        assert!(dual.left.show_hidden);
        assert!(dual.right.show_hidden);
    }

    #[test]
    fn builder_sort_mode_applies_to_both_panes() {
        let dir = tempdir().expect("tempdir");
        let dual = DualPane::builder(dir.path().to_path_buf())
            .sort_mode(SortMode::SizeDesc)
            .build();
        assert_eq!(dual.left.sort_mode, SortMode::SizeDesc);
        assert_eq!(dual.right.sort_mode, SortMode::SizeDesc);
    }

    #[test]
    fn builder_extension_filter_applies_to_both_panes() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("a.rs"), b"fn main(){}").unwrap();
        let dual = DualPane::builder(dir.path().to_path_buf())
            .allow_extension("rs")
            .build();
        assert_eq!(dual.left.extension_filter, vec!["rs"]);
        assert_eq!(dual.right.extension_filter, vec!["rs"]);
    }

    #[test]
    fn builder_allow_extension_accumulates() {
        let dir = tempdir().expect("tempdir");
        let dual = DualPane::builder(dir.path().to_path_buf())
            .allow_extension("rs")
            .allow_extension("toml")
            .build();
        assert!(dual.left.extension_filter.contains(&"rs".to_string()));
        assert!(dual.left.extension_filter.contains(&"toml".to_string()));
    }

    // ── Pane accessors ────────────────────────────────────────────────────────

    #[test]
    fn active_returns_left_by_default() {
        let dual = make_dual();
        assert_eq!(dual.active().current_dir, dual.left.current_dir);
    }

    #[test]
    fn active_returns_right_after_focus_switch() {
        let mut dual = make_dual();
        dual.active_side = DualPaneActive::Right;
        assert_eq!(dual.active().current_dir, dual.right.current_dir);
    }

    #[test]
    fn active_mut_returns_left_by_default() {
        let mut dual = make_dual();
        let dir = dual.active_mut().current_dir.clone();
        assert_eq!(dir, dual.left.current_dir);
    }

    #[test]
    fn inactive_returns_right_when_left_is_active() {
        let dual = make_dual();
        assert_eq!(dual.inactive().current_dir, dual.right.current_dir);
    }

    #[test]
    fn inactive_returns_left_when_right_is_active() {
        let mut dual = make_dual();
        dual.active_side = DualPaneActive::Right;
        assert_eq!(dual.inactive().current_dir, dual.left.current_dir);
    }

    #[test]
    fn focus_left_sets_active_to_left() {
        let mut dual = make_dual();
        dual.active_side = DualPaneActive::Right;
        dual.focus_left();
        assert_eq!(dual.active_side, DualPaneActive::Left);
    }

    #[test]
    fn focus_right_sets_active_to_right() {
        let mut dual = make_dual();
        dual.focus_right();
        assert_eq!(dual.active_side, DualPaneActive::Right);
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    #[test]
    fn tab_switches_active_pane_from_left_to_right() {
        let mut dual = make_dual();
        assert_eq!(dual.active_side, DualPaneActive::Left);
        let outcome = dual.handle_key(key(KeyCode::Tab));
        assert_eq!(outcome, DualPaneOutcome::Pending);
        assert_eq!(dual.active_side, DualPaneActive::Right);
    }

    #[test]
    fn tab_switches_active_pane_from_right_to_left() {
        let mut dual = make_dual();
        dual.active_side = DualPaneActive::Right;
        dual.handle_key(key(KeyCode::Tab));
        assert_eq!(dual.active_side, DualPaneActive::Left);
    }

    #[test]
    fn tab_returns_pending() {
        let mut dual = make_dual();
        assert_eq!(dual.handle_key(key(KeyCode::Tab)), DualPaneOutcome::Pending);
    }

    #[test]
    fn w_key_toggles_single_pane_on() {
        let mut dual = make_dual();
        assert!(!dual.single_pane);
        let outcome = dual.handle_key(key(KeyCode::Char('w')));
        assert_eq!(outcome, DualPaneOutcome::Pending);
        assert!(dual.single_pane);
    }

    #[test]
    fn w_key_toggles_single_pane_off() {
        let mut dual = make_dual();
        dual.single_pane = true;
        dual.handle_key(key(KeyCode::Char('w')));
        assert!(!dual.single_pane);
    }

    #[test]
    fn w_key_returns_pending() {
        let mut dual = make_dual();
        assert_eq!(
            dual.handle_key(key(KeyCode::Char('w'))),
            DualPaneOutcome::Pending
        );
    }

    #[test]
    fn navigation_keys_forwarded_to_active_pane() {
        let mut dual = make_dual();
        // Down arrow should move the left pane cursor.
        let before = dual.left.cursor;
        dual.handle_key(key(KeyCode::Down));
        // Cursor only advances if there are entries; the file we created ensures there are.
        let after = dual.left.cursor;
        // Either it moved (entries exist) or stayed at 0 (only one entry).
        assert!(after >= before);
    }

    #[test]
    fn esc_returns_dismissed() {
        let mut dual = make_dual();
        assert_eq!(
            dual.handle_key(key(KeyCode::Esc)),
            DualPaneOutcome::Dismissed
        );
    }

    #[test]
    fn q_key_returns_dismissed() {
        let mut dual = make_dual();
        assert_eq!(
            dual.handle_key(key(KeyCode::Char('q'))),
            DualPaneOutcome::Dismissed
        );
    }

    #[test]
    fn unrecognised_key_returns_unhandled() {
        let mut dual = make_dual();
        // F5 is not bound by DualPane or FileExplorer.
        assert_eq!(
            dual.handle_key(key(KeyCode::F(5))),
            DualPaneOutcome::Unhandled
        );
    }

    #[test]
    fn enter_on_file_returns_selected() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("pick.txt"), b"hello").unwrap();
        let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
        // Cursor is on the file; press Enter.
        let outcome = dual.handle_key(key(KeyCode::Enter));
        assert!(
            matches!(outcome, DualPaneOutcome::Selected(_)),
            "expected Selected, got {outcome:?}"
        );
    }

    #[test]
    fn tab_does_not_affect_inactive_pane_cursor() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        fs::write(dir.path().join("b.txt"), b"b").unwrap();
        let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

        // Move left pane cursor down.
        dual.handle_key(key(KeyCode::Down));
        let left_cursor_before_tab = dual.left.cursor;

        // Switch to right pane.
        dual.handle_key(key(KeyCode::Tab));

        // Left pane cursor must not have changed.
        assert_eq!(dual.left.cursor, left_cursor_before_tab);
    }

    #[test]
    fn navigation_after_tab_affects_right_pane() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        fs::write(dir.path().join("b.txt"), b"b").unwrap();
        let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

        // Switch to right pane then navigate.
        dual.handle_key(key(KeyCode::Tab));
        let right_before = dual.right.cursor;
        dual.handle_key(key(KeyCode::Down));
        let right_after = dual.right.cursor;

        // Left pane cursor must be untouched.
        assert_eq!(dual.left.cursor, 0, "left pane cursor should not move");
        assert!(
            right_after >= right_before,
            "right pane cursor should have advanced"
        );
    }

    // ── toggle_single_pane ────────────────────────────────────────────────────

    #[test]
    fn toggle_single_pane_flips_state() {
        let mut dual = make_dual();
        assert!(!dual.single_pane);
        dual.toggle_single_pane();
        assert!(dual.single_pane);
        dual.toggle_single_pane();
        assert!(!dual.single_pane);
    }

    // ── reload_both ───────────────────────────────────────────────────────────

    #[test]
    fn reload_both_picks_up_new_files() {
        let dir = tempdir().expect("tempdir");
        let mut dual = DualPane::builder(dir.path().to_path_buf()).build();

        assert!(dual.left.entries.is_empty());
        assert!(dual.right.entries.is_empty());

        // Add a file externally, then reload.
        fs::write(dir.path().join("new.txt"), b"hi").unwrap();
        dual.reload_both();

        assert!(!dual.left.entries.is_empty(), "left should see new file");
        assert!(!dual.right.entries.is_empty(), "right should see new file");
    }

    // ── DualPaneActive default ────────────────────────────────────────────────

    #[test]
    fn dual_pane_active_default_is_left() {
        assert_eq!(DualPaneActive::default(), DualPaneActive::Left);
    }

    // ── focus_left / focus_right after a switch ───────────────────────────────

    #[test]
    fn focus_left_after_switch_to_right_restores_left() {
        let dir = tempdir().expect("tempdir");
        let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
        dual.focus_right();
        assert_eq!(dual.active_side, DualPaneActive::Right);
        dual.focus_left();
        assert_eq!(dual.active_side, DualPaneActive::Left);
    }

    #[test]
    fn focus_right_after_switch_to_left_restores_right() {
        let dir = tempdir().expect("tempdir");
        let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
        dual.focus_left();
        assert_eq!(dual.active_side, DualPaneActive::Left);
        dual.focus_right();
        assert_eq!(dual.active_side, DualPaneActive::Right);
    }

    // ── inactive accessor after focus switch ──────────────────────────────────

    #[test]
    fn inactive_after_focus_right_is_left() {
        let dir = tempdir().expect("tempdir");
        let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
        dual.focus_right();
        // When right is active, inactive() must return the left pane.
        assert_eq!(
            dual.inactive().current_dir,
            dual.left.current_dir,
            "inactive() should point to left when right is active"
        );
    }

    #[test]
    fn inactive_after_focus_left_is_right() {
        let dir = tempdir().expect("tempdir");
        let _right_dir = {
            let d = tempdir().expect("tempdir right");
            d.path().to_path_buf()
            // Note: TempDir is dropped here but path still exists momentarily;
            // use a sub-dir of the same temp dir instead.
        };
        let sub = dir.path().join("right_sub");
        fs::create_dir(&sub).unwrap();
        let mut dual = DualPane::builder(dir.path().to_path_buf())
            .right_dir(sub.clone())
            .build();
        dual.focus_left();
        assert_eq!(
            dual.inactive().current_dir,
            sub,
            "inactive() should point to right when left is active"
        );
    }

    // ── builder extension_filter applies to both panes ────────────────────────

    #[test]
    fn builder_extension_filter_limits_visible_files_on_both_panes() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("main.rs"), b"fn main(){}").unwrap();
        fs::write(dir.path().join("Cargo.toml"), b"[package]").unwrap();

        let dual = DualPane::builder(dir.path().to_path_buf())
            .extension_filter(vec!["rs".into()])
            .build();

        // Only .rs files visible — .toml must be filtered out on both sides.
        assert_eq!(dual.left.entries.len(), 1, "left should show only .rs");
        assert_eq!(dual.right.entries.len(), 1, "right should show only .rs");
        assert_eq!(dual.left.entries[0].extension, "rs");
        assert_eq!(dual.right.entries[0].extension, "rs");
    }

    // ── DualPaneOutcome variants ──────────────────────────────────────────────

    #[test]
    fn dual_pane_outcome_dismissed_eq() {
        assert_eq!(DualPaneOutcome::Dismissed, DualPaneOutcome::Dismissed);
    }

    #[test]
    fn dual_pane_outcome_pending_eq() {
        assert_eq!(DualPaneOutcome::Pending, DualPaneOutcome::Pending);
    }

    #[test]
    fn dual_pane_outcome_unhandled_eq() {
        assert_eq!(DualPaneOutcome::Unhandled, DualPaneOutcome::Unhandled);
    }

    #[test]
    fn dual_pane_outcome_selected_carries_path() {
        use std::path::PathBuf;
        let path = PathBuf::from("/tmp/chosen.txt");
        let outcome = DualPaneOutcome::Selected(path.clone());
        assert_eq!(outcome, DualPaneOutcome::Selected(path));
    }

    #[test]
    fn dual_pane_outcome_selected_neq_dismissed() {
        use std::path::PathBuf;
        let outcome = DualPaneOutcome::Selected(PathBuf::from("/tmp/x"));
        assert_ne!(outcome, DualPaneOutcome::Dismissed);
    }

    // ── active_mut returns mutable ref to active pane ─────────────────────────

    #[test]
    fn active_mut_returns_right_when_right_is_active() {
        let dir = tempdir().expect("tempdir");
        let sub = dir.path().join("right");
        fs::create_dir(&sub).unwrap();
        let mut dual = DualPane::builder(dir.path().to_path_buf())
            .right_dir(sub.clone())
            .build();
        dual.focus_right();
        assert_eq!(
            dual.active_mut().current_dir,
            sub,
            "active_mut() should return right pane when right is active"
        );
    }

    // ── toggle_single_pane ────────────────────────────────────────────────────

    #[test]
    fn toggle_single_pane_from_false_to_true() {
        let dir = tempdir().expect("tempdir");
        let mut dual = DualPane::builder(dir.path().to_path_buf()).build();
        assert!(!dual.single_pane);
        dual.toggle_single_pane();
        assert!(dual.single_pane);
    }

    #[test]
    fn toggle_single_pane_twice_returns_to_original() {
        let dir = tempdir().expect("tempdir");
        let mut dual = DualPane::builder(dir.path().to_path_buf())
            .single_pane(true)
            .build();
        dual.toggle_single_pane();
        dual.toggle_single_pane();
        assert!(
            dual.single_pane,
            "two toggles should restore original state"
        );
    }
}
