//! # tui-file-explorer
//!
//! A self-contained, keyboard-driven file-browser widget for
//! [Ratatui](https://ratatui.rs).
//!
//! ## Design goals
//!
//! * **Zero application-specific dependencies** — only `ratatui`, `crossterm`,
//!   and the standard library are required.
//! * **Narrow public surface** — the public API is intentionally small so the
//!   crate can evolve without breaking changes.
//! * **Extension filtering** — pass a list of allowed extensions so that only
//!   relevant files are selectable (e.g. `["iso", "img"]`); directories are
//!   always navigable.
//! * **Keyboard-driven** — `↑`/`↓`/`←`/`→` scroll the list, `Enter` / `l`
//!   to descend or confirm, `Backspace` / `h` to ascend, `/` to search,
//!   `s` to cycle sort, `n` to create a new folder, `N` to create a new file,
//!   `Esc` / `q` to dismiss.
//! * **Searchable** — press `/` to enter incremental search; entries are
//!   filtered live as you type.  `Esc` clears the query; a second `Esc`
//!   dismisses the explorer.
//! * **Sortable** — press `s` to cycle through `Name`, `Size ↓`, and
//!   `Extension` sort modes, or set one programmatically via
//!   [`FileExplorer::set_sort_mode`].
//! * **Themeable** — every colour is overridable via [`Theme`] and
//!   [`render_themed`].
//! * **Dual-pane** — [`DualPane`] owns two independent [`FileExplorer`]s,
//!   manages focus switching (`Tab`), and supports a single-pane toggle (`w`).
//!
//! ## Quick start
//!
//! ```no_run
//! use tui_file_explorer::{FileExplorer, ExplorerOutcome, SortMode, render};
//! use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
//! # use ratatui::{Terminal, backend::TestBackend};
//! # let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
//!
//! // 1. Create once (e.g. in your App::new).
//! let mut explorer = FileExplorer::builder(std::env::current_dir().unwrap())
//!     .allow_extension("iso")
//!     .allow_extension("img")
//!     .sort_mode(SortMode::SizeDesc)
//!     .build();
//!
//! // 2. In your Terminal::draw closure:
//! //    render(&mut explorer, frame, frame.area());
//!
//! // 3. In your key-handler:
//! # let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
//! match explorer.handle_key(key) {
//!     ExplorerOutcome::Selected(path) => println!("chosen: {}", path.display()),
//!     ExplorerOutcome::Dismissed      => { /* close the overlay */ }
//!     _                               => {}
//! }
//! ```
//!
//! ## Builder / configuration
//!
//! Use [`FileExplorer::builder`] for a more ergonomic construction API:
//!
//! ```no_run
//! use tui_file_explorer::{FileExplorer, SortMode};
//!
//! let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
//!     .allow_extension("rs")
//!     .allow_extension("toml")
//!     .show_hidden(true)
//!     .sort_mode(SortMode::Extension)
//!     .build();
//! ```
//!
//! ## Incremental search
//!
//! Press `/` to activate search mode.  Subsequent keystrokes append to the
//! query and the entry list is filtered live (case-insensitive substring
//! match on the file name).  `Backspace` removes the last character; an
//! extra `Backspace` on an empty query deactivates search.  `Esc` clears
//! the query and deactivates search without dismissing the explorer; a
//! second `Esc` (when search is already inactive) dismisses it.
//!
//! Search state is also accessible programmatically:
//!
//! ```no_run
//! use tui_file_explorer::FileExplorer;
//!
//! let explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
//! println!("searching: {}", explorer.is_searching());
//! println!("query    : {}", explorer.search_query());
//! ```
//!
//! ## Sort modes
//!
//! Press `s` to cycle through the three sort modes, or set one directly:
//!
//! ```no_run
//! use tui_file_explorer::{FileExplorer, SortMode};
//!
//! let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
//! explorer.set_sort_mode(SortMode::SizeDesc); // largest files first
//!
//! println!("{}", explorer.sort_mode().label()); // "size ↓"
//! ```
//!
//! ## Theming
//!
//! Every colour is customisable via [`Theme`] and [`render_themed`]:
//!
//! ```no_run
//! use tui_file_explorer::{FileExplorer, Theme, render_themed};
//! use ratatui::style::Color;
//!
//! let theme = Theme::default()
//!     .brand(Color::Magenta)
//!     .accent(Color::Cyan)
//!     .dir(Color::LightYellow);
//!
//! // terminal.draw(|frame| {
//! //     render_themed(&mut explorer, frame, frame.area(), &theme);
//! // });
//! ```
//!
//! ## Named presets
//!
//! Twenty well-known editor / terminal colour schemes are available as
//! associated constructors on [`Theme`], mirroring the catalogue found in
//! [Iced](https://docs.rs/iced/latest/iced/theme/enum.Theme.html):
//!
//! ```
//! use tui_file_explorer::Theme;
//!
//! let t = Theme::dracula();
//! let t = Theme::nord();
//! let t = Theme::catppuccin_mocha();
//! let t = Theme::tokyo_night();
//! let t = Theme::gruvbox_dark();
//! let t = Theme::kanagawa_wave();
//! let t = Theme::oxocarbon();
//!
//! // Iterate the full catalogue (name, description, theme):
//! for (name, desc, _theme) in Theme::all_presets() {
//!     println!("{name} — {desc}");
//! }
//! ```
//!
//! The complete list: `Default`, `Dracula`, `Nord`, `Solarized Dark`,
//! `Solarized Light`, `Gruvbox Dark`, `Gruvbox Light`, `Catppuccin Latte`,
//! `Catppuccin Frappé`, `Catppuccin Macchiato`, `Catppuccin Mocha`,
//! `Tokyo Night`, `Tokyo Night Storm`, `Tokyo Night Light`, `Kanagawa Wave`,
//! `Kanagawa Dragon`, `Kanagawa Lotus`, `Moonfly`, `Nightfly`, `Oxocarbon`.
//!
//! ## Key bindings reference
//!
//! | Key | Action |
//! |-----|--------|
//! | `↑` / `k` | Move cursor up |
//! | `↓` / `j` | Move cursor down |
//! | `PgUp` / `PgDn` | Jump 10 entries |
//! | `Home` / `g` | Jump to top |
//! | `End` / `G` | Jump to bottom |
//! | `→` / `l` / `Enter` | Descend into directory; on a file `→` moves cursor down, `l`/`Enter` confirm |
//! | `←` / `h` / `Backspace` | Ascend to parent directory |
//! | `Space` | Toggle space-mark on current entry and advance cursor |
//! | `/` | Activate incremental search |
//! | `s` | Cycle sort mode (`Name` → `Size ↓` → `Extension`) |
//! | `.` | Toggle hidden (dot-file) entries |
//! | `n` | Create a new folder (type name → `Enter` confirm, `Esc` cancel) |
//! | `N` | Create a new file   (type name → `Enter` confirm, `Esc` cancel) |
//! | `r` | Rename current entry (pre-filled with current name → `Enter` confirm, `Esc` cancel) |
//! | `Esc` | Clear search / cancel mkdir / cancel touch / cancel rename (if active), then dismiss |
//! | `q` | Dismiss (when search / mkdir / touch / rename is not active) |
//!
//! ## Dual-pane quick start
//!
//! ```no_run
//! use tui_file_explorer::{DualPane, DualPaneOutcome, render_dual_pane_themed, Theme};
//! use crossterm::event::{Event, KeyCode, KeyModifiers, self};
//! # use ratatui::{Terminal, backend::TestBackend};
//! # let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
//!
//! // 1. Create once — left pane defaults to cwd; right pane mirrors it.
//! let mut dual = DualPane::builder(std::env::current_dir().unwrap())
//!     .right_dir(std::path::PathBuf::from("/tmp"))
//!     .build();
//!
//! let theme = Theme::default();
//!
//! // 2. In your Terminal::draw closure:
//! // terminal.draw(|frame| {
//! //     render_dual_pane_themed(&mut dual, frame, frame.area(), &theme);
//! // }).unwrap();
//!
//! // 3. In your event loop:
//! # let Event::Key(key) = event::read().unwrap() else { return; };
//! match dual.handle_key(key) {
//!     DualPaneOutcome::Selected(path) => println!("chosen: {}", path.display()),
//!     DualPaneOutcome::Dismissed      => { /* close the overlay */ }
//!     _                               => {}
//! }
//! ```
//!
//! ### Extra key bindings provided by `DualPane`
//!
//! | Key     | Action                                    |
//! |---------|-------------------------------------------|
//! | `Tab`   | Switch focus between left and right pane  |
//! | `w`     | Toggle single-pane / dual-pane mode       |
//! | `Space` | Mark current entry (forwarded to active pane) |
//!
//! All standard [`FileExplorer`] bindings continue to work on whichever pane
//! is currently active.
//!
//! ## Folder and file creation
//!
//! ### New folder — `n`
//!
//! Press `n` to enter mkdir mode.  Type the new folder name, then press
//! `Enter` to create it (using `fs::create_dir_all`, so nested paths like
//! `a/b/c` work) or `Esc` to cancel without creating anything.  On success
//! [`ExplorerOutcome::MkdirCreated`] is returned and the cursor moves to the
//! new directory.
//!
//! ### New file — `N`
//!
//! Press `N` (Shift+N) to enter touch mode.  Type the new file name (including
//! extension), then press `Enter` to create it or `Esc` to cancel.  Parent
//! directories are created automatically when the name contains `/`.  An
//! existing file at the target path is left untouched (no truncation).  On
//! success [`ExplorerOutcome::TouchCreated`] is returned and the cursor moves
//! to the new file.
//!
//! Both modes are also accessible programmatically:
//!
//! ```no_run
//! use tui_file_explorer::FileExplorer;
//!
//! let explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
//! println!("mkdir active : {}", explorer.is_mkdir_active());
//! println!("mkdir input  : {}", explorer.mkdir_input());
//! println!("touch active : {}", explorer.is_touch_active());
//! println!("touch input  : {}", explorer.touch_input());
//! ```
//!
//! ## Module layout
//!
//! | Module      | Contents                                                                                                                                        |
//! |-------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
//! | `types`     | [`FsEntry`], [`ExplorerOutcome`], [`SortMode`]                                                                                                  |
//! | `palette`   | Palette constants (all `pub`) + [`Theme`] builder + 27 named presets                                                                           |
//! | `explorer`  | [`FileExplorer`], [`FileExplorerBuilder`], [`entry_icon`], [`fmt_size`], `load_entries`                                                         |
//! | `dual_pane` | [`DualPane`], [`DualPaneBuilder`], [`DualPaneActive`], [`DualPaneOutcome`]                                                                      |
//! | `render`    | [`render`], [`render_themed`], [`render_dual_pane`], [`render_dual_pane_themed`] — pure rendering, no I/O                                       |

pub mod dual_pane;
pub mod explorer;
pub mod palette;
pub mod render;
pub mod types;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use dual_pane::{DualPane, DualPaneActive, DualPaneBuilder, DualPaneOutcome};
pub use explorer::{entry_icon, fmt_size, FileExplorer, FileExplorerBuilder};
pub use palette::Theme;
pub use render::{render, render_dual_pane, render_dual_pane_themed, render_themed};
pub use types::{ExplorerOutcome, FsEntry, SortMode};
