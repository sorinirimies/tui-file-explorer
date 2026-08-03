//! Application state for the `tfe` binary.
//!
//! This module owns all runtime state that is not part of the file-explorer
//! widget itself:
//!
//! * `panes` / `active_idx` — the list of open panes and which one is focused.
//! * [`ClipOp`]        — whether a yanked entry is being copied or cut.
//! * [`ClipboardItem`] — what is currently in the clipboard.
//! * [`Modal`]         — an optional blocking confirmation dialog.
//! * [`Editor`]        — which editor to launch when `e` is pressed on a file.
//! * [`App`]           — the top-level state struct that drives the event loop.

use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

// ── Editor ────────────────────────────────────────────────────────────────────

/// The editor that is launched when the user presses `e` on a file.
///
/// # Persistence
///
/// Serialised to/from a short key string in the `tfe` state file:
///
/// | Variant            | Key string        |
/// |--------------------|-------------------|
/// | `None`             | `none`            |
/// | `Helix`            | `helix`           |
/// | `Neovim`           | `nvim`            |
/// | `Vim`              | `vim`             |
/// | `Nano`             | `nano`            |
/// | `Micro`            | `micro`           |
/// | `Emacs`            | `emacs`           |
/// | `VSCode`           | `vscode`          |
/// | `Zed`              | `zed`             |
/// | `Xcode`            | `xcode`           |
/// | `AndroidStudio`    | `android-studio`  |
/// | `RustRover`        | `rustrover`       |
/// | `IntelliJIdea`     | `intellij`        |
/// | `WebStorm`         | `webstorm`        |
/// | `PyCharm`          | `pycharm`         |
/// | `GoLand`           | `goland`          |
/// | `CLion`            | `clion`           |
/// | `Fleet`            | `fleet`           |
/// | `Sublime`          | `sublime`         |
/// | `RubyMine`         | `rubymine`        |
/// | `PHPStorm`         | `phpstorm`        |
/// | `Rider`            | `rider`           |
/// | `Eclipse`          | `eclipse`         |
/// | `Custom(s)`        | `custom:<s>`      |
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Editor {
    /// No editor — pressing `e` on a file is a silent no-op.
    #[default]
    None,
    /// [Helix](https://helix-editor.com/) — `hx`
    Helix,
    /// [Neovim](https://neovim.io/) — `nvim`
    Neovim,
    /// [Vim](https://www.vim.org/) — `vim`
    Vim,
    /// [Nano](https://www.nano-editor.org/) — `nano`
    Nano,
    /// [Micro](https://micro-editor.github.io/) — `micro`
    Micro,
    /// [Emacs](https://www.gnu.org/software/emacs/) — `emacs`
    Emacs,
    /// [Visual Studio Code](https://code.visualstudio.com/) — `code`
    VSCode,
    /// [Zed](https://zed.dev/) — `zed`
    Zed,
    /// [Xcode](https://developer.apple.com/xcode/) — `xed`
    Xcode,
    /// [Android Studio](https://developer.android.com/studio) — `studio`
    AndroidStudio,
    /// [RustRover](https://www.jetbrains.com/rust/) — `rustrover`
    RustRover,
    /// [IntelliJ IDEA](https://www.jetbrains.com/idea/) — `idea`
    IntelliJIdea,
    /// [WebStorm](https://www.jetbrains.com/webstorm/) — `webstorm`
    WebStorm,
    /// [PyCharm](https://www.jetbrains.com/pycharm/) — `pycharm`
    PyCharm,
    /// [GoLand](https://www.jetbrains.com/go/) — `goland`
    GoLand,
    /// [CLion](https://www.jetbrains.com/clion/) — `clion`
    CLion,
    /// [Fleet](https://www.jetbrains.com/fleet/) — `fleet`
    Fleet,
    /// [Sublime Text](https://www.sublimetext.com/) — `subl`
    Sublime,
    /// [RubyMine](https://www.jetbrains.com/ruby/) — `rubymine`
    RubyMine,
    /// [PHPStorm](https://www.jetbrains.com/phpstorm/) — `phpstorm`
    PHPStorm,
    /// [Rider](https://www.jetbrains.com/rider/) — `rider`
    Rider,
    /// [Eclipse](https://www.eclipse.org/) — `eclipse`
    Eclipse,
    /// A user-supplied binary name or path.
    Custom(String),
}

impl Editor {
    /// Return the launch binary (and optional arguments) for this editor.
    ///
    /// Returns `None` for `Editor::None` — the caller should skip the launch.
    ///
    /// For `Custom` variants the returned string may contain embedded
    /// arguments (e.g. `"code --wait"`).  The caller is responsible for
    /// splitting on whitespace to separate the binary from its arguments
    /// before passing them to `std::process::Command`.
    ///
    /// For `Editor::Helix` the function probes `$PATH` at call time: it
    /// tries `hx` first (the name used by the official release binaries and
    /// Homebrew on macOS), then falls back to `helix` (the name used by most
    /// Linux package managers such as pacman, apt, and dnf).  Whichever is
    /// found first is returned; if neither is on `$PATH` the string `"hx"` is
    /// returned as a best-effort fallback so the error message names a real
    /// binary.
    pub fn binary(&self) -> Option<String> {
        match self {
            Editor::None => Option::None,
            Editor::Helix => Some(Self::resolve_helix()),
            Editor::Neovim => Some("nvim".to_string()),
            Editor::Vim => Some("vim".to_string()),
            Editor::Nano => Some("nano".to_string()),
            Editor::Micro => Some("micro".to_string()),
            Editor::Emacs => Some("emacs".to_string()),
            Editor::VSCode => Some("code".to_string()),
            Editor::Zed => Some("zed".to_string()),
            Editor::Xcode => Some("xed".to_string()),
            Editor::AndroidStudio => Some("studio".to_string()),
            Editor::RustRover => Some("rustrover".to_string()),
            Editor::IntelliJIdea => Some("idea".to_string()),
            Editor::WebStorm => Some("webstorm".to_string()),
            Editor::PyCharm => Some("pycharm".to_string()),
            Editor::GoLand => Some("goland".to_string()),
            Editor::CLion => Some("clion".to_string()),
            Editor::Fleet => Some("fleet".to_string()),
            Editor::Sublime => Some("subl".to_string()),
            Editor::RubyMine => Some("rubymine".to_string()),
            Editor::PHPStorm => Some("phpstorm".to_string()),
            Editor::Rider => Some("rider".to_string()),
            Editor::Eclipse => Some("eclipse".to_string()),
            Editor::Custom(s) => Some(s.clone()),
        }
    }

    /// Probe `$PATH` for the Helix binary name.
    ///
    /// Returns `"hx"` when found, then tries `"helix"`, and finally falls
    /// back to `"hx"` so callers always get a non-empty string.
    fn resolve_helix() -> String {
        for candidate in &["hx", "helix"] {
            if which_on_path(candidate) {
                return candidate.to_string();
            }
        }
        // Neither found — return "hx" so the error message is predictable.
        "hx".to_string()
    }

    /// Return a short human-readable label (shown in the options panel).
    pub fn label(&self) -> &str {
        match self {
            Editor::None => "none",
            Editor::Helix => "helix",
            Editor::Neovim => "nvim",
            Editor::Vim => "vim",
            Editor::Nano => "nano",
            Editor::Micro => "micro",
            Editor::Emacs => "emacs",
            Editor::VSCode => "vscode",
            Editor::Zed => "zed",
            Editor::Xcode => "xcode",
            Editor::AndroidStudio => "android-studio",
            Editor::RustRover => "rustrover",
            Editor::IntelliJIdea => "intellij",
            Editor::WebStorm => "webstorm",
            Editor::PyCharm => "pycharm",
            Editor::GoLand => "goland",
            Editor::CLion => "clion",
            Editor::Fleet => "fleet",
            Editor::Sublime => "sublime",
            Editor::RubyMine => "rubymine",
            Editor::PHPStorm => "phpstorm",
            Editor::Rider => "rider",
            Editor::Eclipse => "eclipse",
            Editor::Custom(s) => s.as_str(),
        }
    }

    /// Cycle to the next editor in the fixed rotation.
    ///
    /// Order: None → Helix → Neovim → Vim → Nano → Micro → None → …
    ///
    /// `Custom` variants skip back to `None` — the user must set them via
    /// `--editor` or direct persistence editing.
    #[allow(dead_code)]
    pub fn cycle(&self) -> Editor {
        match self {
            Editor::None => Editor::Helix,
            Editor::Helix => Editor::Neovim,
            Editor::Neovim => Editor::Vim,
            Editor::Vim => Editor::Nano,
            Editor::Nano => Editor::Micro,
            Editor::Micro => Editor::None,
            // New GUI/IDE editors and Custom all fall back to None in the legacy
            // cycle rotation.  The cycle() method is deprecated in favour of the
            // editor-picker panel (Shift + E); this fallback keeps it exhaustive.
            _ => Editor::None,
        }
    }

    /// Serialise to the on-disk key string.
    pub fn to_key(&self) -> String {
        match self {
            Editor::None => "none".to_string(),
            Editor::Helix => "helix".to_string(),
            Editor::Neovim => "nvim".to_string(),
            Editor::Vim => "vim".to_string(),
            Editor::Nano => "nano".to_string(),
            Editor::Micro => "micro".to_string(),
            Editor::Emacs => "emacs".to_string(),
            Editor::VSCode => "vscode".to_string(),
            Editor::Zed => "zed".to_string(),
            Editor::Xcode => "xcode".to_string(),
            Editor::AndroidStudio => "android-studio".to_string(),
            Editor::RustRover => "rustrover".to_string(),
            Editor::IntelliJIdea => "intellij".to_string(),
            Editor::WebStorm => "webstorm".to_string(),
            Editor::PyCharm => "pycharm".to_string(),
            Editor::GoLand => "goland".to_string(),
            Editor::CLion => "clion".to_string(),
            Editor::Fleet => "fleet".to_string(),
            Editor::Sublime => "sublime".to_string(),
            Editor::RubyMine => "rubymine".to_string(),
            Editor::PHPStorm => "phpstorm".to_string(),
            Editor::Rider => "rider".to_string(),
            Editor::Eclipse => "eclipse".to_string(),
            Editor::Custom(s) => format!("custom:{s}"),
        }
    }

    /// Deserialise from the on-disk key string.
    ///
    /// Returns `None` (the Rust `Option`) for an empty string; unknown values
    /// are treated as `Custom` so that third-party editors survive round-trips.
    pub fn from_key(s: &str) -> Option<Editor> {
        if s.is_empty() {
            return Option::None;
        }
        Some(match s {
            "none" => Editor::None,
            "helix" => Editor::Helix,
            "nvim" => Editor::Neovim,
            "vim" => Editor::Vim,
            "nano" => Editor::Nano,
            "micro" => Editor::Micro,
            "emacs" => Editor::Emacs,
            "vscode" => Editor::VSCode,
            "zed" => Editor::Zed,
            "xcode" => Editor::Xcode,
            "android-studio" => Editor::AndroidStudio,
            "rustrover" => Editor::RustRover,
            "intellij" => Editor::IntelliJIdea,
            "webstorm" => Editor::WebStorm,
            "pycharm" => Editor::PyCharm,
            "goland" => Editor::GoLand,
            "clion" => Editor::CLion,
            "fleet" => Editor::Fleet,
            "sublime" => Editor::Sublime,
            "rubymine" => Editor::RubyMine,
            "phpstorm" => Editor::PHPStorm,
            "rider" => Editor::Rider,
            "eclipse" => Editor::Eclipse,
            _ if s.starts_with("custom:") => Editor::Custom(s["custom:".len()..].to_string()),
            other => Editor::Custom(other.to_string()),
        })
    }
}

// ── PATH probe helper ─────────────────────────────────────────────────────────

/// Returns `true` when `name` resolves to an executable on `$PATH`.
///
/// This is intentionally minimal — it only walks `$PATH` entries and checks
/// for a regular (or symlinked) file with execute permission.  It does not
/// handle Windows `.cmd` shims or `PATHEXT`, but that is fine because Helix
/// does not ship as a `.cmd` wrapper.
fn which_on_path(name: &str) -> bool {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path_var).any(|dir| {
        let candidate = dir.join(name);
        // `metadata` follows symlinks, so a symlink to an executable is OK.
        candidate
            .metadata()
            .map(|m| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    m.is_file() && (m.permissions().mode() & 0o111 != 0)
                }
                #[cfg(not(unix))]
                {
                    m.is_file()
                }
            })
            .unwrap_or(false)
    })
}

// ── AppOptions ────────────────────────────────────────────────────────────────

/// Startup configuration passed to [`App::new`].
///
/// Grouping all constructor parameters into a single struct keeps the call
/// sites readable and avoids the `clippy::too_many_arguments` limit.
///
/// # Example
///
/// ```rust,ignore
/// let app = App::new(AppOptions {
///     pane_dirs: vec![PathBuf::from("/home/user"), PathBuf::from("/tmp")],
///     ..AppOptions::default()
/// });
/// ```
#[derive(Debug, Clone)]
pub struct AppOptions {
    /// Starting directories, one per pane (at least one is required).
    pub pane_dirs: Vec<PathBuf>,
    /// File-extension filter (empty = show all).
    pub extensions: Vec<String>,
    /// Show hidden (dot-prefixed) entries on startup.
    pub show_hidden: bool,
    /// Show file/folder sizes on startup. Defaults to `true`; set to `false`
    /// for the snappiest possible browsing of huge directory trees.
    pub show_sizes: bool,
    /// Index into the theme catalogue to use on startup.
    pub theme_idx: usize,
    /// Whether the theme-picker side-panel should be open on startup.
    pub show_theme_panel: bool,
    /// Whether to start in single-pane mode.
    pub single_pane: bool,
    /// Active sort mode.
    pub sort_mode: SortMode,
    /// Whether cd-on-exit is enabled.
    pub cd_on_exit: bool,
    /// Which editor to open when the user presses `e` on a file.
    pub editor: Editor,
    /// When `true`, show a debug log panel in the TUI and write logs to a
    /// file.  Activated by `--verbose` / `-v`.
    pub verbose: bool,
    /// Pre-App log lines collected during startup (before the App existed).
    /// These are drained into [`App::debug_log`] on construction.
    pub startup_log: Vec<String>,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            pane_dirs: vec![PathBuf::from(".")],
            extensions: vec![],
            show_hidden: false,
            show_sizes: true,
            theme_idx: 0,
            show_theme_panel: false,
            single_pane: false,
            sort_mode: SortMode::default(),
            cd_on_exit: false,
            editor: Editor::default(),
            verbose: false,
            startup_log: Vec::new(),
        }
    }
}

use crate::fs::copy_dir_all;
use crate::inline_editor::{EditorAction, InlineEditor};
use crate::preview::PreviewState;

use crate::{ExplorerOutcome, FileExplorer, SortMode, Theme};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

// ── ClipOp ───────────────────────────────────────────────────────────────────

/// Whether the clipboard item should be copied or moved on paste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipOp {
    Copy,
    Cut,
}

// ── ClipboardItem ─────────────────────────────────────────────────────────────

/// An entry (or entries) that have been yanked (copied or cut) and are waiting
/// to be pasted.  When the user space-marks multiple files before pressing
/// `p` (copy) or `x` (cut), all marked paths are stored here.
#[derive(Debug, Clone)]
pub struct ClipboardItem {
    /// One or more source paths waiting to be pasted.
    pub paths: Vec<PathBuf>,
    /// Whether this is a copy or a cut operation.
    pub op: ClipOp,
}

impl ClipboardItem {
    /// A small emoji that visually distinguishes copy from cut in the action bar.
    pub fn icon(&self) -> &'static str {
        match self.op {
            ClipOp::Copy => "\u{1F4CB}", // 📋
            ClipOp::Cut => "\u{2702} ",  // ✂
        }
    }

    /// A short human-readable label for the current operation.
    pub fn label(&self) -> &'static str {
        match self.op {
            ClipOp::Copy => "Copy",
            ClipOp::Cut => "Cut ",
        }
    }

    /// Number of paths in the clipboard.
    pub fn count(&self) -> usize {
        self.paths.len()
    }

    /// The first (or only) path — convenience accessor for single-item clipboard.
    pub fn first_path(&self) -> Option<&PathBuf> {
        self.paths.first()
    }
}

// ── Modal ─────────────────────────────────────────────────────────────────────

/// A blocking confirmation dialog that intercepts all keyboard input until
/// the user either confirms or cancels.
#[derive(Debug)]
pub enum Modal {
    /// Asks the user to confirm deletion of a file or directory.
    Delete {
        /// Absolute path of the entry to delete.
        path: PathBuf,
    },
    /// Asks the user to confirm deletion of multiple marked entries.
    MultiDelete {
        /// Absolute paths of all entries to delete.
        paths: Vec<PathBuf>,
    },
    /// Asks the user whether to overwrite an existing destination during paste.
    Overwrite {
        /// Absolute path of the source being pasted.
        src: PathBuf,
        /// Absolute path of the destination that already exists.
        dst: PathBuf,
        /// `true` if the original operation was a cut (move).
        is_cut: bool,
    },
}

// ── App ───────────────────────────────────────────────────────────────────────

// Top-level application state for the `tfe` binary.
//
// Owns both [`FileExplorer`] panes, the clipboard, the active modal, theme
// state, and the final selected path (set when the user confirms a file).
// ── Snackbar ──────────────────────────────────────────────────────────────────

/// A short-lived notification that floats over the UI and auto-expires.
pub struct Snackbar {
    /// The message to display.
    pub message: String,
    /// When the snackbar should stop being shown.
    pub expires_at: Instant,
    /// Whether this is an error (affects colour).
    pub is_error: bool,
}

impl Snackbar {
    /// Create a new info snackbar that lasts 3 seconds.
    #[allow(dead_code)]
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            expires_at: Instant::now() + Duration::from_secs(3),
            is_error: false,
        }
    }

    /// Create a new error snackbar that lasts 4 seconds.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            expires_at: Instant::now() + Duration::from_secs(4),
            is_error: true,
        }
    }

    /// Returns `true` if the snackbar's display window has passed.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Tracks the progress of an in-progress copy/move operation.
#[derive(Debug, Clone)]
pub struct CopyProgress {
    /// Human-readable label for the current operation (e.g. "Copying 3 items").
    pub label: String,
    /// Number of files/dirs successfully processed so far.
    pub done: usize,
    /// Total number of files/dirs to process.
    pub total: usize,
    /// Name of the item currently being copied.
    pub current_item: String,
}

impl CopyProgress {
    /// Create a new progress tracker.
    pub fn new(label: impl Into<String>, total: usize) -> Self {
        Self {
            label: label.into(),
            done: 0,
            total,
            current_item: String::new(),
        }
    }

    /// Returns the fraction complete as a value in `0.0..=1.0`.
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.done as f64 / self.total as f64
        }
    }

    /// Returns `true` when all items have been processed.
    pub fn is_complete(&self) -> bool {
        self.done >= self.total
    }
}

pub struct App {
    /// All explorer panes, in left-to-right display order. Always has at
    /// least one entry.
    pub panes: Vec<FileExplorer>,
    /// Index into `panes` of the pane that currently has keyboard focus.
    pub active_idx: usize,
    /// The most recently yanked entry, if any.
    pub clipboard: Option<ClipboardItem>,
    /// All available themes as `(name, description, Theme)` triples.
    pub themes: Vec<(&'static str, &'static str, Theme)>,
    /// Index into `themes` for the currently active theme.
    pub theme_idx: usize,
    /// Whether the theme-picker side-panel is visible.
    pub show_theme_panel: bool,
    /// Whether the options side-panel is visible.
    pub show_options_panel: bool,
    /// Whether only the active pane is shown (single-pane mode).
    pub single_pane: bool,
    /// The currently displayed confirmation modal, if any.
    pub modal: Option<Modal>,
    /// The path chosen by the user (set on `Enter` / `→` confirm).
    pub selected: Option<PathBuf>,
    /// One-line status text shown in the action bar.
    pub status_msg: String,
    /// Optional floating notification that auto-expires.
    pub snackbar: Option<Snackbar>,
    /// Progress of an ongoing copy/move operation, if any.
    pub copy_progress: Option<CopyProgress>,
    /// Whether cd-on-exit is enabled (dismiss prints cwd to stdout).
    pub cd_on_exit: bool,
    /// Which editor to open when the user presses `e` on a file.
    pub editor: Editor,
    /// When `Some`, the run-loop should suspend the TUI, open this path in
    /// `self.editor`, then restore the TUI.  Set by the `e` key handler;
    /// cleared by `run_loop` after the editor exits.
    pub open_with_editor: Option<PathBuf>,
    /// Whether the editor-picker side-panel is visible.
    pub show_editor_panel: bool,
    /// Highlighted row index in the editor-picker panel (cursor position).
    pub editor_panel_idx: usize,
    /// Whether the debug log panel is visible (`--verbose` / `-v`).
    pub verbose: bool,
    /// Accumulated debug log lines shown in the log panel.
    pub debug_log: Vec<String>,
    /// Scroll offset for the debug log panel (0 = pinned to bottom).
    pub debug_scroll: usize,
    /// Whether the preview panel is visible (toggled with `P`).
    pub show_preview: bool,
    /// Cached preview content for the currently highlighted file.
    pub preview_state: PreviewState,
    /// The active inline editor, if any (opened with `i`).
    pub inline_editor: Option<InlineEditor>,
}

impl App {
    /// Construct a new `App` from an [`AppOptions`] config struct.
    pub fn new(opts: AppOptions) -> Self {
        let dirs = if opts.pane_dirs.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            opts.pane_dirs
        };
        let panes: Vec<FileExplorer> = dirs
            .into_iter()
            .map(|dir| {
                FileExplorer::builder(dir)
                    .extension_filter(opts.extensions.clone())
                    .show_hidden(opts.show_hidden)
                    .show_sizes(opts.show_sizes)
                    .sort_mode(opts.sort_mode)
                    .build()
            })
            .collect();
        Self {
            panes,
            active_idx: 0,
            clipboard: None,
            themes: Theme::all_presets(),
            theme_idx: opts.theme_idx,
            show_theme_panel: opts.show_theme_panel,
            show_options_panel: false,
            single_pane: opts.single_pane,
            modal: None,
            selected: None,
            status_msg: String::new(),
            snackbar: None,
            copy_progress: None,
            cd_on_exit: opts.cd_on_exit,
            editor: opts.editor,
            open_with_editor: None,
            show_editor_panel: false,
            editor_panel_idx: 0,
            verbose: opts.verbose,
            debug_log: opts.startup_log,
            debug_scroll: 0,
            show_preview: false,
            preview_state: PreviewState::new(),
            inline_editor: None,
        }
    }

    /// Append a line to the debug log (visible in the log panel when
    /// `--verbose` is active).
    pub fn log(&mut self, msg: impl Into<String>) {
        if self.verbose {
            self.debug_log.push(msg.into());
        }
    }

    /// Index of the first IDE/GUI editor in the [`all_editors`] list.
    ///
    /// Everything before this index is a terminal editor; everything from
    /// this index onward is a GUI editor or IDE.  Used by the editor panel
    /// to render the two section headers.
    pub fn first_ide_idx() -> usize {
        // None, Helix, Neovim, Vim, Nano, Micro, Emacs  →  7 terminal entries
        7
    }

    /// Return every [`Editor`] variant in display order.
    ///
    /// Used by the editor-picker panel to populate the list and navigate it.
    /// Terminal editors come first, then GUI editors/IDEs.
    pub fn all_editors() -> Vec<Editor> {
        vec![
            // ── Terminal editors ──────────────────────────────────────────────
            Editor::None,
            Editor::Helix,
            Editor::Neovim,
            Editor::Vim,
            Editor::Nano,
            Editor::Micro,
            Editor::Emacs,
            // ── IDEs & GUI editors ────────────────────────────────────────────
            Editor::Sublime,
            Editor::VSCode,
            Editor::Zed,
            Editor::Xcode,
            Editor::AndroidStudio,
            Editor::RustRover,
            Editor::IntelliJIdea,
            Editor::WebStorm,
            Editor::PyCharm,
            Editor::GoLand,
            Editor::CLion,
            Editor::Fleet,
            Editor::RubyMine,
            Editor::PHPStorm,
            Editor::Rider,
            Editor::Eclipse,
        ]
    }

    /// Sync `editor_panel_idx` to point at the currently active `editor`.
    ///
    /// Called when the panel is opened so the cursor lands on the current
    /// selection.  Defaults to index `0` (`Editor::None`) if not found.
    pub fn sync_editor_panel_idx(&mut self) {
        let editors = Self::all_editors();
        self.editor_panel_idx = editors.iter().position(|e| e == &self.editor).unwrap_or(0);
    }

    // ── Snackbar helpers ──────────────────────────────────────────────────────

    /// Show an info snackbar with the given message (auto-expires after 3 s).
    #[allow(dead_code)]
    pub fn notify(&mut self, msg: impl Into<String>) {
        self.snackbar = Some(Snackbar::info(msg));
    }

    /// Reload both panes and show a notification with the entry name.
    fn reload_and_notify(&mut self, path: &std::path::Path, verb: &str) {
        for p in self.panes.iter_mut() {
            p.reload();
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.notify(format!("{verb} '{name}'"));
    }

    /// Show an error snackbar with the given message (auto-expires after 4 s).
    pub fn notify_error(&mut self, msg: impl Into<String>) {
        self.snackbar = Some(Snackbar::error(msg));
    }

    // ── Theme helpers ─────────────────────────────────────────────────────────

    /// Return a reference to the currently selected [`Theme`].
    pub fn theme(&self) -> &Theme {
        &self.themes[self.theme_idx].2
    }

    /// Return the name of the currently selected theme.
    pub fn theme_name(&self) -> &str {
        self.themes[self.theme_idx].0
    }

    /// Return the description of the currently selected theme.
    pub fn theme_desc(&self) -> &str {
        self.themes[self.theme_idx].1
    }

    /// Advance to the next theme, wrapping around at the end of the list.
    pub fn next_theme(&mut self) {
        self.theme_idx = (self.theme_idx + 1) % self.themes.len();
    }

    /// Retreat to the previous theme, wrapping around at the beginning.
    pub fn prev_theme(&mut self) {
        self.theme_idx = self
            .theme_idx
            .checked_sub(1)
            .unwrap_or(self.themes.len() - 1);
    }
}

mod clipboard;
mod keys;
mod pane;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
