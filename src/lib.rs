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
//! * **Keyboard-driven** — arrow keys / vim keys, `Enter` to descend or
//!   confirm, `Backspace` / `h` to ascend, `Esc` / `q` to dismiss.
//! * **Themeable** — every colour is overridable via [`Theme`] and
//!   [`render_themed`].
//!
//! ## Quick start
//!
//! ```no_run
//! use tui_file_explorer::{FileExplorer, ExplorerOutcome, render};
//! use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
//!
//! // 1. Create once (e.g. in your App::new).
//! let mut explorer = FileExplorer::new(
//!     std::env::current_dir().unwrap(),
//!     vec!["iso".into(), "img".into()],
//! );
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
//! use tui_file_explorer::FileExplorer;
//!
//! let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
//!     .allow_extension("rs")
//!     .allow_extension("toml")
//!     .show_hidden(true)
//!     .build();
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
//! ## Module layout
//!
//! | Module      | Contents                                                      |
//! |-------------|---------------------------------------------------------------|
//! | `types`     | [`FsEntry`], [`ExplorerOutcome`]                              |
//! | `palette`   | Palette constants (all `pub`) + [`Theme`] + named presets     |
//! | `explorer`  | [`FileExplorer`] + [`FileExplorerBuilder`]                    |
//! | `render`    | [`render`], [`render_themed`]                                 |

pub mod explorer;
pub mod palette;
pub mod render;
pub mod types;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use explorer::{FileExplorer, FileExplorerBuilder};
pub use palette::Theme;
pub use render::{render, render_themed};
pub use types::{ExplorerOutcome, FsEntry};
