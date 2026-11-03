use std::{fmt, io, path::PathBuf, time::Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::{App, AppOptions, Editor, OperationRequest};
use crate::{AppState, FileExplorer, SelectionMode, SortMode, Theme};

/// Result of dispatching input through the full application API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppOutcome {
    /// Non-key or key-release event; no application action was requested.
    Ignored,
    /// Input was processed without a separately classified state transition.
    Continue,
    /// One or more observable application properties changed.
    Changed(Vec<AppEvent>),
    /// User selected a path and requested exit.
    Selected(PathBuf),
    /// User dismissed the application.
    Dismissed,
    /// Host should suspend rendering and open `path` with `editor`.
    OpenEditor {
        /// The file to open.
        path: PathBuf,
        /// The editor to open it with.
        editor: Editor,
    },
    /// Host should execute a filesystem operation outside the UI thread.
    OperationRequested(OperationRequest),
}

/// Observable state transition emitted by host-driven dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    /// A pane was opened.
    PaneAdded {
        /// Number of panes after the change.
        pane_count: usize,
    },
    /// A pane was closed.
    PaneClosed {
        /// Number of panes after the change.
        pane_count: usize,
    },
    /// Keyboard focus moved to another pane.
    ActivePaneChanged {
        /// Index of the newly focused pane.
        index: usize,
    },
    /// A pane navigated to another directory.
    DirectoryChanged {
        /// Index of the pane that navigated.
        pane: usize,
        /// The pane's new current directory.
        path: PathBuf,
    },
    /// The active theme changed.
    ThemeChanged {
        /// Index of the new theme in the theme catalogue.
        index: usize,
    },
    /// Single-pane mode was turned on or off.
    LayoutChanged {
        /// `true` when only the active pane is shown.
        single_pane: bool,
    },
    /// The preview panel was shown or hidden.
    PreviewChanged {
        /// `true` when the preview panel is shown.
        visible: bool,
    },
    /// A running file operation made progress.
    OperationProgress {
        /// The operation this update belongs to.
        id: super::OperationId,
        /// Items processed so far.
        completed: usize,
        /// Total number of items in the operation.
        total: usize,
    },
    /// A file operation finished.
    OperationCompleted {
        /// The operation that finished.
        id: super::OperationId,
        /// Items processed without error.
        succeeded: usize,
        /// Items that failed.
        failed: usize,
    },
    /// A file operation was cancelled before it finished.
    OperationCancelled {
        /// The operation that was cancelled.
        id: super::OperationId,
    },
}

command_enum! {
    /// Semantic command accepted by [`App::dispatch_command`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum AppCommand {
        /// Move the cursor up one row.
        MoveUp => "move_up",
        /// Move the cursor down one row.
        MoveDown => "move_down",
        /// Move the cursor up one page.
        PageUp => "page_up",
        /// Move the cursor down one page.
        PageDown => "page_down",
        /// Jump to the first entry.
        Top => "top",
        /// Jump to the last entry.
        Bottom => "bottom",
        /// Go to the parent directory.
        Ascend => "ascend",
        /// Enter the directory under the cursor.
        Open => "open",
        /// Select the entry under the cursor, or enter it when directories can't be selected.
        Confirm => "confirm",
        /// Focus the next pane.
        NextPane => "next_pane",
        /// Focus the previous pane.
        PreviousPane => "previous_pane",
        /// Open a new pane.
        OpenPane => "open_pane",
        /// Close the active pane.
        ClosePane => "close_pane",
        /// Show only the active pane, or all panes again.
        ToggleSinglePane => "toggle_single_pane",
        /// Start incremental search.
        Search => "search",
        /// Switch to the next sort order.
        CycleSort => "cycle_sort",
        /// Show or hide dotfiles.
        ToggleHidden => "toggle_hidden",
        /// Show or hide file sizes.
        ToggleSizes => "toggle_sizes",
        /// Show or hide directory sizes.
        ToggleDirSizes => "toggle_dir_sizes",
        /// Mark or unmark the entry under the cursor.
        ToggleMark => "toggle_mark",
        /// Cut the marked entries, or the one under the cursor, to the clipboard.
        Cut => "cut",
        /// Paste the clipboard into the active pane's directory.
        Paste => "paste",
        /// Delete the marked entries, or the one under the cursor.
        Delete => "delete",
        /// Create a directory in the current directory.
        NewDirectory => "new_directory",
        /// Create an empty file in the current directory.
        NewFile => "new_file",
        /// Rename the entry under the cursor.
        Rename => "rename",
        /// Show or hide the preview panel.
        TogglePreview => "toggle_preview",
        /// Scroll the preview down.
        PreviewDown => "preview_down",
        /// Scroll the preview up.
        PreviewUp => "preview_up",
        /// Open the file under the cursor in the external editor.
        OpenExternalEditor => "open_external_editor",
        /// Open the file under the cursor in the built-in editor.
        OpenInlineEditor => "open_inline_editor",
        /// Show or hide the theme picker.
        ToggleThemePanel => "toggle_theme_panel",
        /// Show or hide the options panel.
        ToggleOptionsPanel => "toggle_options_panel",
        /// Show or hide the editor picker.
        ToggleEditorPanel => "toggle_editor_panel",
        /// Switch to the next theme.
        NextTheme => "next_theme",
        /// Switch to the previous theme.
        PreviousTheme => "previous_theme",
        /// Turn cd-on-exit on or off.
        ToggleCdOnExit => "toggle_cd_on_exit",
        /// Close the explorer without selecting anything.
        Dismiss => "dismiss",
        /// Quit the application.
        Quit => "quit",
        /// Undo the last file action.
        Undo => "undo",
        /// Redo the last undone action.
        Redo => "redo",
        /// Rename the marked entries together in a text editor.
        BulkRename => "bulk_rename",
        /// Bookmark the current directory, or remove its bookmark.
        ToggleBookmark => "toggle_bookmark",
        /// Show the bookmark list.
        OpenBookmarks => "open_bookmarks",
        /// Find a file below the current directory by fuzzy name search.
        FuzzyFind => "fuzzy_find",
        /// Show or hide git status.
        ToggleGit => "toggle_git",
        /// Extract the archive under the cursor.
        ExtractArchive => "extract_archive",
    }
}

impl AppCommand {
    /// The built-in key that triggers this command.
    #[must_use]
    pub fn key_event(self) -> KeyEvent {
        let (code, modifiers) = match self {
            Self::MoveUp => (KeyCode::Up, KeyModifiers::NONE),
            Self::MoveDown => (KeyCode::Down, KeyModifiers::NONE),
            Self::PageUp => (KeyCode::PageUp, KeyModifiers::NONE),
            Self::PageDown => (KeyCode::PageDown, KeyModifiers::NONE),
            Self::Top => (KeyCode::Home, KeyModifiers::NONE),
            Self::Bottom => (KeyCode::End, KeyModifiers::NONE),
            Self::Ascend => (KeyCode::Left, KeyModifiers::NONE),
            Self::Open => (KeyCode::Right, KeyModifiers::NONE),
            Self::Confirm => (KeyCode::Enter, KeyModifiers::NONE),
            Self::NextPane => (KeyCode::Tab, KeyModifiers::NONE),
            Self::PreviousPane => (KeyCode::BackTab, KeyModifiers::SHIFT),
            Self::OpenPane => (KeyCode::Char('t'), KeyModifiers::CONTROL),
            Self::ClosePane => (KeyCode::Char('w'), KeyModifiers::CONTROL),
            Self::ToggleSinglePane => (KeyCode::Char('w'), KeyModifiers::NONE),
            Self::Search => (KeyCode::Char('/'), KeyModifiers::NONE),
            Self::CycleSort => (KeyCode::Char('s'), KeyModifiers::NONE),
            Self::ToggleHidden => (KeyCode::Char('.'), KeyModifiers::NONE),
            Self::ToggleSizes => (KeyCode::Char('z'), KeyModifiers::NONE),
            Self::ToggleDirSizes => (KeyCode::Char('Z'), KeyModifiers::SHIFT),
            Self::ToggleMark => (KeyCode::Char(' '), KeyModifiers::NONE),
            Self::Cut => (KeyCode::Char('x'), KeyModifiers::NONE),
            Self::Paste => (KeyCode::Char('p'), KeyModifiers::NONE),
            Self::Delete => (KeyCode::Char('d'), KeyModifiers::NONE),
            Self::NewDirectory => (KeyCode::Char('n'), KeyModifiers::NONE),
            Self::NewFile => (KeyCode::Char('N'), KeyModifiers::NONE),
            Self::Rename => (KeyCode::Char('r'), KeyModifiers::NONE),
            Self::TogglePreview => (KeyCode::Char('P'), KeyModifiers::SHIFT),
            Self::PreviewDown => (KeyCode::Char('j'), KeyModifiers::CONTROL),
            Self::PreviewUp => (KeyCode::Char('k'), KeyModifiers::CONTROL),
            Self::OpenExternalEditor => (KeyCode::Char('e'), KeyModifiers::NONE),
            Self::OpenInlineEditor => (KeyCode::Char('i'), KeyModifiers::NONE),
            Self::ToggleThemePanel => (KeyCode::Char('T'), KeyModifiers::SHIFT),
            Self::ToggleOptionsPanel => (KeyCode::Char('O'), KeyModifiers::SHIFT),
            Self::ToggleEditorPanel => (KeyCode::Char('E'), KeyModifiers::SHIFT),
            Self::NextTheme => (KeyCode::Char('t'), KeyModifiers::NONE),
            Self::PreviousTheme => (KeyCode::Char('['), KeyModifiers::NONE),
            Self::ToggleCdOnExit => (KeyCode::Char('C'), KeyModifiers::SHIFT),
            Self::Dismiss => (KeyCode::Esc, KeyModifiers::NONE),
            Self::Quit => (KeyCode::Char('q'), KeyModifiers::NONE),
            Self::Undo => (KeyCode::Char('u'), KeyModifiers::NONE),
            Self::Redo => (KeyCode::Char('r'), KeyModifiers::CONTROL),
            Self::BulkRename => (KeyCode::Char('R'), KeyModifiers::SHIFT),
            Self::ToggleBookmark => (KeyCode::Char('b'), KeyModifiers::NONE),
            Self::OpenBookmarks => (KeyCode::Char('B'), KeyModifiers::SHIFT),
            Self::FuzzyFind => (KeyCode::Char('f'), KeyModifiers::NONE),
            Self::ToggleGit => (KeyCode::Char('v'), KeyModifiers::NONE),
            Self::ExtractArchive => (KeyCode::Char('X'), KeyModifiers::SHIFT),
        };
        KeyEvent::new(code, modifiers)
    }
}

/// One custom key-to-command mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBinding {
    /// The key that triggers the command.
    pub code: KeyCode,
    /// Modifiers that must be held with [`code`](Self::code).
    pub modifiers: KeyModifiers,
    /// The command the key runs.
    pub command: AppCommand,
}

/// Custom key bindings applied by [`App::dispatch_key`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyBindings {
    bindings: Vec<KeyBinding>,
    /// Commands whose built-in key has been replaced via [`Self::remap`].
    remapped: Vec<AppCommand>,
}

impl KeyBindings {
    /// An empty set: every command keeps its built-in key.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace one key mapping.
    #[must_use]
    pub fn bind(mut self, code: KeyCode, modifiers: KeyModifiers, command: AppCommand) -> Self {
        self.bindings
            .retain(|binding| binding.code != code || binding.modifiers != modifiers);
        self.bindings.push(KeyBinding {
            code,
            modifiers,
            command,
        });
        self
    }

    /// Remove the mapping for this key, if any.
    #[must_use]
    pub fn unbind(mut self, code: KeyCode, modifiers: KeyModifiers) -> Self {
        self.bindings
            .retain(|binding| binding.code != code || binding.modifiers != modifiers);
        self
    }

    /// Replace the keys of `command` with `chords`.
    ///
    /// Unlike [`bind`](Self::bind), this also *disables the built-in key* of
    /// the command (unless a chord reuses it), so `x` stops cutting once
    /// `cut` is moved elsewhere.
    #[must_use]
    pub fn remap(mut self, command: AppCommand, chords: &[crate::KeyChord]) -> Self {
        self.bindings.retain(|b| b.command != command);
        if !self.remapped.contains(&command) {
            self.remapped.push(command);
        }
        for chord in chords {
            let event = chord.to_event();
            // Uppercase letters arrive with or without SHIFT depending on the
            // terminal; accept both.
            let mut mods = vec![event.modifiers];
            if matches!(chord.mods, crate::ChordMods::Plain | crate::ChordMods::Any)
                && let KeyCode::Char(c) = event.code
                && c.is_uppercase()
            {
                mods.push(KeyModifiers::NONE);
            }
            for m in mods {
                self = self.bind(event.code, m, command);
            }
        }
        self
    }

    /// Translate a physical key into the event the built-in handler expects.
    ///
    /// * custom binding  → that command's built-in key,
    /// * a remapped command's old built-in key → `None` (swallowed),
    /// * anything else → the key unchanged.
    #[must_use]
    pub fn translate(&self, key: KeyEvent) -> Option<KeyEvent> {
        if let Some(command) = self.resolve(key) {
            return Some(command.key_event());
        }
        let swallowed = self.remapped.iter().any(|c| {
            let built_in = c.key_event();
            built_in.code == key.code && built_in.modifiers == key.modifiers
        });
        (!swallowed).then_some(key)
    }

    /// All custom mappings, in the order they were added.
    pub fn bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }

    /// The command custom-bound to `key`, if any.
    #[must_use]
    pub fn resolve(&self, key: KeyEvent) -> Option<AppCommand> {
        self.bindings
            .iter()
            .find(|binding| binding.code == key.code && binding.modifiers == key.modifiers)
            .map(|binding| binding.command)
    }

    /// `true` when there are no custom mappings and no remapped commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty() && self.remapped.is_empty()
    }
}

/// Per-pane construction options used by [`AppBuilder`].
#[derive(Debug, Clone)]
pub struct PaneOptions {
    /// Directory the pane opens in.
    pub directory: PathBuf,
    /// Filesystem the pane browses.
    pub filesystem: crate::SharedFileSystem,
    /// File extensions to show; empty shows every file.
    pub extensions: Vec<String>,
    /// Show dotfiles.
    pub show_hidden: bool,
    /// Show file sizes.
    pub show_sizes: bool,
    /// Show directory sizes.
    pub show_dir_sizes: bool,
    /// Navigation keys for this pane.
    pub keymap: crate::Keymap,
    /// Read directories on a background thread.
    pub async_loading: bool,
    /// Initial sort order.
    pub sort_mode: SortMode,
    /// Rows moved by page up and page down.
    pub page_size: usize,
    /// Whether the pane selects files, directories or both.
    pub selection_mode: SelectionMode,
}

impl PaneOptions {
    /// Options for a pane in `directory`, with the defaults: real filesystem, all
    /// files, hidden files off, sizes on, synchronous loading.
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            filesystem: crate::std_filesystem(),
            extensions: Vec::new(),
            show_hidden: false,
            show_sizes: true,
            show_dir_sizes: true,
            keymap: crate::Keymap::default(),
            async_loading: false,
            sort_mode: SortMode::default(),
            page_size: crate::PAGE_SIZE,
            selection_mode: SelectionMode::default(),
        }
    }

    /// Browse `filesystem` instead of the real one.
    #[must_use]
    pub fn filesystem(mut self, filesystem: crate::SharedFileSystem) -> Self {
        self.filesystem = filesystem;
        self
    }

    /// Also show files with this extension (without the dot).
    #[must_use]
    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        self.extensions.push(extension.into());
        self
    }

    /// Show or hide dotfiles.
    #[must_use]
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// Show or hide file sizes.
    #[must_use]
    pub fn show_sizes(mut self, show: bool) -> Self {
        self.show_sizes = show;
        self
    }

    /// Show or hide directory sizes.
    #[must_use]
    pub fn show_dir_sizes(mut self, show: bool) -> Self {
        self.show_dir_sizes = show;
        self
    }

    /// Use `keymap` for navigation in this pane.
    #[must_use]
    pub fn keymap(mut self, keymap: crate::Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Read directories on a background thread.
    pub fn async_loading(mut self, enabled: bool) -> Self {
        self.async_loading = enabled;
        self
    }

    /// Set the initial sort order.
    pub fn sort_mode(mut self, sort_mode: SortMode) -> Self {
        self.sort_mode = sort_mode;
        self
    }

    /// Set how many rows page up and page down move.
    #[must_use]
    pub fn page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size.max(1);
        self
    }

    /// Set whether the pane selects files, directories or both.
    #[must_use]
    pub fn selection_mode(mut self, selection_mode: SelectionMode) -> Self {
        self.selection_mode = selection_mode;
        self
    }

    fn build(self) -> FileExplorer {
        FileExplorer::builder(self.directory)
            .filesystem(self.filesystem)
            .extension_filter(self.extensions)
            .show_hidden(self.show_hidden)
            .show_sizes(self.show_sizes)
            .show_dir_sizes(self.show_dir_sizes)
            .keymap(self.keymap)
            .async_loading(self.async_loading)
            .sort_mode(self.sort_mode)
            .page_size(self.page_size)
            .selection_mode(self.selection_mode)
            .build()
    }
}

/// Error returned by [`AppBuilder::build`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppBuildError {
    /// The application needs at least one pane.
    NoPanes,
    /// The theme index is past the end of the theme catalogue.
    InvalidThemeIndex {
        /// The requested index.
        index: usize,
        /// Number of themes available.
        theme_count: usize,
    },
    /// The pane index is past the last pane.
    InvalidPaneIndex {
        /// The requested index.
        index: usize,
        /// Number of panes open.
        pane_count: usize,
    },
}

impl fmt::Display for AppBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPanes => write!(f, "an application must contain at least one pane"),
            Self::InvalidThemeIndex { index, theme_count } => {
                write!(f, "theme index {index} is outside 0..{theme_count}")
            }
            Self::InvalidPaneIndex { index, pane_count } => {
                write!(f, "pane index {index} is outside 0..{pane_count}")
            }
        }
    }
}

impl std::error::Error for AppBuildError {}

/// Builder for the complete embeddable application.
#[derive(Debug, Clone)]
pub struct AppBuilder {
    panes: Vec<PaneOptions>,
    options: AppOptions,
    key_bindings: KeyBindings,
    theme_override: Option<Theme>,
    operation_mode: super::OperationMode,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            panes: Vec::new(),
            options: AppOptions::default(),
            key_bindings: KeyBindings::default(),
            theme_override: None,
            operation_mode: super::OperationMode::Immediate,
        }
    }
}

impl AppBuilder {
    /// A builder with default options and no panes yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pane. Without any, [`build`](Self::build) adds one for `.`.
    #[must_use]
    pub fn pane(mut self, pane: PaneOptions) -> Self {
        self.panes.push(pane);
        self
    }

    /// Start with the theme at `theme_index` in the theme catalogue.
    #[must_use]
    pub fn theme_index(mut self, theme_index: usize) -> Self {
        self.options.theme_idx = theme_index;
        self
    }

    /// Replace the selected preset's palette with a host-defined theme.
    #[must_use]
    pub fn theme(mut self, theme: impl Into<Theme>) -> Self {
        self.theme_override = Some(theme.into());
        self
    }

    /// Editor that opens files on `e`.
    #[must_use]
    pub fn editor(mut self, editor: Editor) -> Self {
        self.options.editor = editor;
        self
    }

    /// Start with only the active pane shown.
    #[must_use]
    pub fn single_pane(mut self, enabled: bool) -> Self {
        self.options.single_pane = enabled;
        self
    }

    /// Print the final directory on dismiss, for the shell wrapper.
    #[must_use]
    pub fn cd_on_exit(mut self, enabled: bool) -> Self {
        self.options.cd_on_exit = enabled;
        self
    }

    /// Show the debug log panel and write logs to a file.
    #[must_use]
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.options.verbose = enabled;
        self
    }

    /// Use custom key bindings.
    #[must_use]
    pub fn key_bindings(mut self, key_bindings: KeyBindings) -> Self {
        self.key_bindings = key_bindings;
        self
    }

    /// Run file operations at once, or hand them to the host.
    #[must_use]
    pub fn operation_mode(mut self, operation_mode: super::OperationMode) -> Self {
        self.operation_mode = operation_mode;
        self
    }

    /// Build the [`App`].
    ///
    /// # Errors
    ///
    /// [`AppBuildError::InvalidThemeIndex`] when the theme index is out of range.
    pub fn build(mut self) -> Result<App, AppBuildError> {
        let theme_count = crate::Theme::presets().len();
        if self.options.theme_idx >= theme_count {
            return Err(AppBuildError::InvalidThemeIndex {
                index: self.options.theme_idx,
                theme_count,
            });
        }

        if self.panes.is_empty() {
            self.panes.push(PaneOptions::new("."));
        }
        self.options.pane_dirs = self
            .panes
            .iter()
            .map(|pane| pane.directory.clone())
            .collect();
        let pane_options = self.panes;
        let mut app = App::new(self.options);
        app.panes = pane_options.into_iter().map(PaneOptions::build).collect();
        app.key_bindings = self.key_bindings;
        app.operation_mode = self.operation_mode;
        if let Some(theme) = self.theme_override {
            app.themes[app.theme_idx].1 = theme;
        }
        Ok(app)
    }
}

impl App {
    /// Start building an application.
    #[must_use]
    pub fn builder() -> AppBuilder {
        AppBuilder::new()
    }

    /// All panes, in left-to-right order.
    #[must_use]
    pub fn panes(&self) -> &[FileExplorer] {
        &self.panes
    }

    /// All panes, mutable.
    pub fn panes_mut(&mut self) -> &mut [FileExplorer] {
        &mut self.panes
    }

    /// Replace every pane. The focus moves to the last pane if it was further right.
    ///
    /// # Errors
    ///
    /// [`AppBuildError::NoPanes`] when `panes` is empty.
    pub fn replace_panes(&mut self, panes: Vec<FileExplorer>) -> Result<(), AppBuildError> {
        if panes.is_empty() {
            return Err(AppBuildError::NoPanes);
        }
        self.panes = panes;
        self.active_idx = self.active_idx.min(self.panes.len() - 1);
        Ok(())
    }

    /// Index of the pane with keyboard focus.
    #[must_use]
    pub fn active_pane_index(&self) -> usize {
        self.active_idx
    }

    /// Move keyboard focus to the pane at `index`.
    ///
    /// # Errors
    ///
    /// [`AppBuildError::InvalidPaneIndex`] when there is no such pane.
    pub fn set_active_pane_index(&mut self, index: usize) -> Result<(), AppBuildError> {
        if index >= self.panes.len() {
            return Err(AppBuildError::InvalidPaneIndex {
                index,
                pane_count: self.panes.len(),
            });
        }
        self.active_idx = index;
        Ok(())
    }

    /// Switch to the theme at `index` in the theme catalogue.
    ///
    /// # Errors
    ///
    /// [`AppBuildError::InvalidThemeIndex`] when there is no such theme.
    pub fn set_theme_index(&mut self, index: usize) -> Result<(), AppBuildError> {
        if index >= self.themes.len() {
            return Err(AppBuildError::InvalidThemeIndex {
                index,
                theme_count: self.themes.len(),
            });
        }
        self.theme_idx = index;
        Ok(())
    }

    /// Use `keymap` for explorer navigation in every pane (and new ones).
    pub fn set_keymap(&mut self, keymap: &crate::Keymap) {
        for pane in &mut self.panes {
            pane.keymap = keymap.clone();
        }
    }

    /// Apply a parsed `keymap.toml`: explorer keys on every pane, app-level
    /// remaps via [`set_key_bindings`](Self::set_key_bindings).
    pub fn apply_keymap_config(&mut self, config: &crate::KeymapConfig) {
        self.set_keymap(&config.explorer);
        self.key_bindings = config.app.clone();
    }

    /// The custom key bindings in use.
    #[must_use]
    pub fn key_bindings(&self) -> &KeyBindings {
        &self.key_bindings
    }

    /// Replace the custom key bindings.
    pub fn set_key_bindings(&mut self, key_bindings: KeyBindings) {
        self.key_bindings = key_bindings;
    }

    /// Dispatch a host-owned terminal event.
    pub fn dispatch_event(&mut self, event: Event) -> io::Result<AppOutcome> {
        match event {
            Event::Key(key) => self.dispatch_key(key),
            Event::Resize(_, _) => Ok(AppOutcome::Continue),
            Event::Mouse(_) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {
                Ok(AppOutcome::Ignored)
            }
        }
    }

    /// Dispatch a semantic command without synthesizing input in host code.
    pub fn dispatch_command(&mut self, command: AppCommand) -> io::Result<AppOutcome> {
        self.dispatch_key_event(command.key_event())
    }

    /// Dispatch a key using custom bindings, preserving modal/editor priority.
    pub fn dispatch_key(&mut self, key: KeyEvent) -> io::Result<AppOutcome> {
        press_only!(key, Ok(AppOutcome::Ignored));
        let pane_captures_text =
            self.active_pane().search_active || self.active_pane().has_prompt();
        let mapped = if self.inline_editor.is_some() || self.modal.is_some() || pane_captures_text {
            Some(key)
        } else {
            self.key_bindings.translate(key)
        };
        match mapped {
            Some(mapped) => self.dispatch_key_event(mapped),
            None => Ok(AppOutcome::Continue),
        }
    }

    fn dispatch_key_event(&mut self, key: KeyEvent) -> io::Result<AppOutcome> {
        let previous_editor_request = self.open_with_editor.clone();
        let previous_pane_count = self.panes.len();
        let previous_active = self.active_idx;
        let previous_directories: Vec<PathBuf> = self
            .panes
            .iter()
            .map(|pane| pane.current_dir.clone())
            .collect();
        let previous_theme = self.theme_idx;
        let previous_single_pane = self.single_pane;
        let previous_preview = self.show_preview;
        let should_exit = self.handle_key(key)?;

        let mut events = Vec::new();
        match self.panes.len().cmp(&previous_pane_count) {
            std::cmp::Ordering::Greater => events.push(AppEvent::PaneAdded {
                pane_count: self.panes.len(),
            }),
            std::cmp::Ordering::Less => events.push(AppEvent::PaneClosed {
                pane_count: self.panes.len(),
            }),
            std::cmp::Ordering::Equal => {}
        }
        if self.active_idx != previous_active {
            events.push(AppEvent::ActivePaneChanged {
                index: self.active_idx,
            });
        }
        for (index, pane) in self.panes.iter().enumerate() {
            if previous_directories.get(index) != Some(&pane.current_dir) {
                events.push(AppEvent::DirectoryChanged {
                    pane: index,
                    path: pane.current_dir.clone(),
                });
            }
        }
        if self.theme_idx != previous_theme {
            events.push(AppEvent::ThemeChanged {
                index: self.theme_idx,
            });
        }
        if self.single_pane != previous_single_pane {
            events.push(AppEvent::LayoutChanged {
                single_pane: self.single_pane,
            });
        }
        if self.show_preview != previous_preview {
            events.push(AppEvent::PreviewChanged {
                visible: self.show_preview,
            });
        }
        self.event_queue.extend(events.iter().cloned());

        if let Some(request) = self.queued_operation.take() {
            return Ok(AppOutcome::OperationRequested(request));
        }
        if should_exit {
            return Ok(self
                .selected
                .clone()
                .map_or(AppOutcome::Dismissed, AppOutcome::Selected));
        }
        if self.open_with_editor != previous_editor_request
            && let Some(path) = self.open_with_editor.clone()
        {
            return Ok(AppOutcome::OpenEditor {
                path,
                editor: self.editor.clone(),
            });
        }
        if events.is_empty() {
            Ok(AppOutcome::Continue)
        } else {
            Ok(AppOutcome::Changed(events))
        }
    }

    /// Drain observable transitions accumulated by dispatch calls.
    pub fn drain_events(&mut self) -> impl Iterator<Item = AppEvent> + '_ {
        self.event_queue.drain(..)
    }

    /// Advance time-based UI state without rendering.
    pub fn tick(&mut self, now: Instant) {
        if self
            .snackbar
            .as_ref()
            .is_some_and(|snackbar| now >= snackbar.expires_at)
        {
            self.snackbar = None;
        }
    }

    /// Refresh preview content for a host-provided viewport.
    pub fn update_preview(&mut self, width: u16, height: u16) {
        if !self.show_preview {
            return;
        }
        let current_path = self
            .active_pane()
            .current_entry()
            .map(|entry| entry.path.clone());
        self.preview_state
            .update(current_path.as_deref(), width, height);
    }

    /// Take and clear the external-editor request used by the legacy run loop.
    pub fn take_editor_request(&mut self) -> Option<(PathBuf, Editor)> {
        self.open_with_editor
            .take()
            .map(|path| (path, self.editor.clone()))
    }

    /// Capture host-storable application state without writing global files.
    #[must_use]
    pub fn snapshot(&self) -> AppState {
        let active = self.active_pane();
        AppState {
            theme: Some(self.theme_name().to_string()),
            last_dir: self.panes.first().map(|pane| pane.current_dir.clone()),
            last_dir_right: self.panes.get(1).map(|pane| pane.current_dir.clone()),
            sort_mode: Some(active.sort_mode),
            show_hidden: Some(active.show_hidden),
            show_sizes: Some(active.show_sizes),
            show_dir_sizes: Some(active.show_dir_sizes),
            use_trash: Some(self.use_trash),
            show_git: Some(active.show_git),
            bookmarks: Some(self.bookmarks.clone()),
            single_pane: Some(self.single_pane),
            cd_on_exit: Some(self.cd_on_exit),
            editor: Some(self.editor.to_key()),
            active_pane: Some(
                if self.active_idx == 1 {
                    "right"
                } else {
                    "left"
                }
                .into(),
            ),
            pane_dirs: Some(
                self.panes
                    .iter()
                    .map(|pane| pane.current_dir.clone())
                    .collect(),
            ),
            active_pane_idx: Some(self.active_idx),
        }
    }

    /// Apply host-loaded state while preserving runtime-only UI state.
    pub fn restore(&mut self, state: &AppState) {
        let pane_dirs = state
            .pane_dirs
            .clone()
            .filter(|dirs| !dirs.is_empty())
            .or_else(|| {
                state.last_dir.clone().map(|left| {
                    let mut dirs = vec![left];
                    if let Some(right) = state.last_dir_right.clone() {
                        dirs.push(right);
                    }
                    dirs
                })
            });
        if let Some(dirs) = pane_dirs {
            let show_hidden = state.show_hidden.unwrap_or(false);
            let show_sizes = state.show_sizes.unwrap_or(true);
            let show_dir_sizes = state.show_dir_sizes.unwrap_or(true);
            let show_git = state.show_git.unwrap_or(false);
            let sort_mode = state.sort_mode.unwrap_or_default();
            let filesystem = self
                .panes
                .first()
                .map_or_else(crate::std_filesystem, |pane| pane.filesystem().clone());
            let keymap = self
                .panes
                .first()
                .map(|pane| pane.keymap.clone())
                .unwrap_or_default();
            let async_loading = self.panes.first().is_some_and(|pane| pane.async_loading);
            self.panes = dirs
                .into_iter()
                .map(|directory| {
                    FileExplorer::builder(directory)
                        .filesystem(filesystem.clone())
                        .show_hidden(show_hidden)
                        .show_sizes(show_sizes)
                        .show_dir_sizes(show_dir_sizes)
                        .show_git(show_git)
                        .async_loading(async_loading)
                        .keymap(keymap.clone())
                        .sort_mode(sort_mode)
                        .build()
                })
                .collect();
        }
        if let Some(bookmarks) = &state.bookmarks {
            self.bookmarks.clone_from(bookmarks);
        }
        if let Some(index) = state.active_pane_idx {
            self.active_idx = index.min(self.panes.len().saturating_sub(1));
        }
        if let Some(theme) = &state.theme {
            self.theme_idx = crate::resolve_theme_idx(theme, &self.themes);
        }
        if let Some(single_pane) = state.single_pane {
            self.single_pane = single_pane;
        }
        if let Some(cd_on_exit) = state.cd_on_exit {
            self.cd_on_exit = cd_on_exit;
        }
        if let Some(editor) = state.editor.as_deref().and_then(Editor::from_key) {
            self.editor = editor;
        }
    }
}

#[cfg(feature = "theme")]
impl crate::nuances::Themed for AppBuilder {
    fn apply_theme(self, theme: &crate::nuances::NuancesTheme) -> Self {
        self.theme(theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tempfile::tempdir;

    #[test]
    fn builder_supports_per_pane_options() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        let app = App::builder()
            .pane(PaneOptions::new(left.path()).show_hidden(true))
            .pane(PaneOptions::new(right.path()).show_sizes(false))
            .build()
            .unwrap();
        assert_eq!(app.panes().len(), 2);
        assert!(app.panes()[0].show_hidden);
        assert!(!app.panes()[1].show_sizes);
    }

    #[test]
    fn builder_rejects_invalid_theme() {
        let result = App::builder().theme_index(usize::MAX).build();
        assert!(matches!(
            result,
            Err(AppBuildError::InvalidThemeIndex { .. })
        ));
    }

    #[test]
    fn custom_binding_dispatches_command() {
        let dir = tempdir().unwrap();
        let mut app = App::builder()
            .pane(PaneOptions::new(dir.path()))
            .key_bindings(KeyBindings::new().bind(
                KeyCode::F(2),
                KeyModifiers::NONE,
                AppCommand::ToggleSinglePane,
            ))
            .build()
            .unwrap();
        assert!(!app.single_pane);
        let outcome = app
            .dispatch_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(
            outcome,
            AppOutcome::Changed(vec![AppEvent::LayoutChanged { single_pane: true }])
        );
        assert!(app.single_pane);
        assert_eq!(
            app.drain_events().collect::<Vec<_>>(),
            vec![AppEvent::LayoutChanged { single_pane: true }]
        );
    }

    #[test]
    fn custom_bindings_do_not_override_inline_editor_input() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("note.txt"), b"").unwrap();
        let mut app = App::builder()
            .pane(PaneOptions::new(dir.path()))
            .key_bindings(KeyBindings::new().bind(
                KeyCode::Char('p'),
                KeyModifiers::NONE,
                AppCommand::ToggleSinglePane,
            ))
            .build()
            .unwrap();
        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
            .unwrap();
        app.dispatch_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE))
            .unwrap();
        assert!(!app.single_pane);
        assert_eq!(app.inline_editor.as_ref().unwrap().lines(), &["p"]);
    }

    #[test]
    fn custom_bindings_do_not_override_search_input() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("paper.txt"), b"").unwrap();
        let mut app = App::builder()
            .pane(PaneOptions::new(dir.path()))
            .key_bindings(KeyBindings::new().bind(
                KeyCode::Char('p'),
                KeyModifiers::NONE,
                AppCommand::ToggleSinglePane,
            ))
            .build()
            .unwrap();
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE))
            .unwrap();
        app.dispatch_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE))
            .unwrap();
        assert!(!app.single_pane);
        assert_eq!(app.active_pane().search_query(), "p");
    }

    #[test]
    fn snapshot_round_trips_host_owned_state() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        let mut app = App::builder()
            .pane(PaneOptions::new(left.path()))
            .pane(PaneOptions::new(right.path()))
            .build()
            .unwrap();
        app.set_active_pane_index(1).unwrap();
        app.single_pane = true;
        let snapshot = app.snapshot();

        let mut restored = App::new(AppOptions::default());
        restored.restore(&snapshot);
        assert_eq!(restored.panes().len(), 2);
        assert_eq!(restored.active_pane_index(), 1);
        assert!(restored.single_pane);
    }
}
