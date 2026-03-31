# tui-file-explorer

[![Crates.io](https://img.shields.io/crates/v/tui-file-explorer?color=orange)](https://crates.io/crates/tui-file-explorer)
[![Documentation](https://docs.rs/tui-file-explorer/badge.svg)](https://docs.rs/tui-file-explorer)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Release](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/release.yml/badge.svg)](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/release.yml)
[![CI](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/ci.yml/badge.svg)](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/tui-file-explorer)](https://crates.io/crates/tui-file-explorer)

A keyboard-driven, two-pane file manager widget for [Ratatui](https://ratatui.rs).  
Use it as an **embeddable library widget** or run it as the **standalone `tfe` CLI tool**.

---

## Preview

### Basic navigation
![basic](examples/vhs/generated/basic.gif)

### Search
![search](examples/vhs/generated/search.gif)

### Sort modes
![sort](examples/vhs/generated/sort.gif)

### Extension filter
![filter](examples/vhs/generated/filter.gif)

### File operations
![file_ops](examples/vhs/generated/file_ops.gif)

### Theme switcher
![theme_switcher](examples/vhs/generated/theme_switcher.gif)

### Pane toggle
![pane_toggle](examples/vhs/generated/pane_toggle.gif)

### Dual pane
![dual_pane](examples/vhs/generated/dual_pane.gif)

### Options panel
![options](examples/vhs/generated/options.gif)

### Editor picker
![editor_picker](examples/vhs/generated/editor_picker.gif)

### Create entries
![create_entries](examples/vhs/generated/create_entries.gif)

---

## Features

- 🗂️ **Two-pane layout** — independent left and right explorer panes, `Tab` to switch focus
- 📋 **File operations** — copy (`y`), cut (`x`), paste (`p`), delete (`d`); `Space` to multi-select; `n`/`N` to create dirs/files; `r` to rename
- 🔍 **Incremental search** — press `/` to filter entries live as you type
- 🔃 **Sort modes** — cycle `Name → Size ↓ → Extension` with `s`
- 🎛️ **Extension filter** — only matching files are selectable; dirs are always navigable
- 👁️ Toggle hidden dot-file visibility with `.`
- ⌨️ Full keyboard navigation: Miller-columns `←`/`→` (ascend/descend), vim keys (`h/j/k/l`), `↑`/`↓`/`j`/`k`, `PgUp/PgDn`, `g/G` — `→` on a file moves down, never exits the TUI
- 🎨 **27 named themes** — Catppuccin, Dracula, Nord, Tokyo Night, Kanagawa, Gruvbox, and more
- 🎛️ **Live theme panel** — press `T` to open a sidebar, `t`/`[` to cycle themes; `↑`/`↓` navigate the list when the panel is open
- 📝 **Editor picker** — `Shift+E` opens a panel listing Terminal Editors and IDEs & GUI Editors; `↑`/`↓` navigate, `Enter` selects, `Esc` cancels; `e` opens the highlighted file in the configured editor
- ⚙️ **Options panel** — `Shift+O` opens a panel showing Toggles, Editor, and File Ops sections
- 🔧 Fluent builder API for ergonomic embedding — both `FileExplorer` and `DualPane`
- 📦 **`DualPane` library widget** — drop a full two-pane explorer into any Ratatui app with one struct
- 🖥️ **`cd` on exit** — dismiss with `Esc`/`q` and your terminal jumps to the directory you were browsing; one-time setup with `tfe --init <shell>` (bash, zsh, fish, powershell)
- ✅ Lean library — only `ratatui` + `crossterm` required (`clap` is opt-out)

---

## Stats

| Metric | Value |
|--------|-------|
| Library dependencies | 2 (`ratatui`, `crossterm`) |
| Named colour themes | 27 |
| Sort modes | 3 (`Name`, `Size ↓`, `Extension`) |
| File operations | 4 (copy, cut, paste, delete) |
| Key bindings | 20+ |
| File-type icons | 50+ extensions mapped |
| Public API surface | 10 types, 6 free functions |
| Unit tests | 475 |

---

## Installation

### As a library

```toml
[dependencies]
tui-file-explorer = "0.8"
ratatui = "0.30"
```

Library-only (no `clap`-powered CLI binary):

```toml
[dependencies]
tui-file-explorer = { version = "0.8", default-features = false }
```

### As a CLI tool

```bash
cargo install tui-file-explorer
```

Installs the `tfe` binary onto your `PATH`.

---

## Quick Start

The library exposes **three API tiers** — pick the level that matches your needs:

| Tier | Entry point | What you get | What you wire yourself |
|------|-------------|-------------|----------------------|
| **Widget** | `FileExplorer` | Single-pane navigation, search, sort, marks, mkdir, touch, rename | Everything else (rendering, clipboard, multi-pane, panels) |
| **Dual-pane widget** | `DualPane` | Two independent panes + `Tab` / `w` toggle | Clipboard, copy/paste, delete, theme/options/editor panels |
| **Full app** | `App` + `draw()` | **Everything** — identical to the `tfe` binary | Nothing — all features work via keyboard out of the box |

### Full app (batteries included)

The easiest way to embed the complete `tfe` experience in your own application.
All features — dual-pane, copy/cut/paste with progress, delete with confirmation,
theme picker, options panel, editor integration, snackbar notifications — work
through keyboard shortcuts automatically with **zero extra code**:

```rust
use tui_file_explorer::{App, AppOptions, draw};
use std::path::PathBuf;

// 1. Create once.
let mut app = App::new(AppOptions {
    left_dir: std::env::current_dir().unwrap(),
    right_dir: PathBuf::from("/tmp"),
    ..AppOptions::default()
});

// 2. Event loop — that's it:
loop {
    terminal.draw(|frame| draw(&mut app, frame))?;
    if app.handle_event()? {
        break;
    }
}
```

Every feature from the `tfe` binary is available without any manual wiring:

| Feature | Key | Automatic? |
|---------|-----|:----------:|
| Dual-pane | `Tab` | ✅ |
| Single/dual toggle | `w` | ✅ |
| Multi-select | `Space` | ✅ |
| Copy (yank) | `y` | ✅ |
| Cut | `x` | ✅ |
| Paste (with progress) | `p` | ✅ |
| Delete (with confirmation) | `d` | ✅ |
| Theme picker panel | `T` | ✅ |
| Theme cycling | `t` / `[` | ✅ |
| Options panel | `O` | ✅ |
| Editor picker panel | `E` | ✅ |
| Open file in editor | `e` | ✅ |
| cd-on-exit toggle | `C` | ✅ |
| Search, sort, mkdir, touch, rename | `/` `s` `n` `N` `r` | ✅ |

### Single-pane widget

Use `FileExplorer` when you only need a file-browser widget and want to handle
outcomes yourself:

```rust
use tui_file_explorer::{FileExplorer, ExplorerOutcome, SortMode, render};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// 1. Create once — e.g. in your App::new
let mut explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    .allow_extension("iso")
    .allow_extension("img")
    .sort_mode(SortMode::SizeDesc)   // largest files first
    .show_hidden(false)
    .build();

// 2. Inside Terminal::draw:
// render(&mut explorer, frame, frame.area());

// 3. Inside your key-handler — ↑/↓/←/→ scroll the list, Enter confirms:
let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
match explorer.handle_key(key) {
    ExplorerOutcome::Selected(path) => println!("chosen: {}", path.display()),
    ExplorerOutcome::Dismissed      => { /* close the overlay */ }
    _                               => {}
}
```

### Dual-pane widget

Use `DualPane` when you want two panes with focus switching but prefer to build
your own clipboard / file-ops layer:

```rust
use tui_file_explorer::{DualPane, DualPaneOutcome, render_dual_pane_themed, Theme};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

// 1. Create once — left pane defaults to cwd; right pane can differ.
let mut dual = DualPane::builder(std::env::current_dir().unwrap())
    .right_dir(PathBuf::from("/tmp"))
    .show_hidden(false)
    .build();

let theme = Theme::default();

// 2. Inside Terminal::draw:
// render_dual_pane_themed(&mut dual, frame, frame.area(), &theme);

// 3. Inside your key-handler:
let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
match dual.handle_key(key) {
    DualPaneOutcome::Selected(path) => println!("chosen: {}", path.display()),
    DualPaneOutcome::Dismissed      => { /* close the overlay */ }
    _                               => {}
}
```

---

## Key Bindings

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move cursor up |
| `↓` / `j` | Move cursor down |
| `PgUp` | Jump up 10 rows |
| `PgDn` | Jump down 10 rows |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `→` / `l` / `Enter` | Descend into directory; on a file `→` moves cursor down, `l`/`Enter` confirm (exits TUI) |
| `←` / `h` / `Backspace` | Ascend to parent directory |
| `Tab` | **Switch active pane** (left ↔ right) |
| `w` | **Toggle two-pane ↔ single-pane** layout |

### Explorer actions

| Key | Action |
|-----|--------|
| `/` | Activate incremental search |
| `s` | Cycle sort mode (`Name → Size ↓ → Extension`) |
| `.` | Toggle hidden (dot-file) entries |
| `Esc` | Clear search (if active), then dismiss |
| `q` | Dismiss when search is not active |

### File operations

| Key | Action |
|-----|--------|
| `Space` | **Mark** — toggle multi-select on the highlighted entry |
| `y` | **Yank** — mark highlighted entry for copy |
| `x` | **Cut** — mark highlighted entry for move |
| `p` | **Paste** — copy/move clipboard into the *other* pane's directory |
| `d` | **Delete** — remove highlighted entry (asks for confirmation) |
| `n` | **New folder** — prompt for a name and create the directory (`mkdir`) |
| `N` | **New file** — prompt for a name and create the file (`touch`) |
| `r` | **Rename** — prompt for a new name and rename the highlighted entry |
| `e` | **Open in editor** — open highlighted file in the configured editor; shows an error if no editor is set |

### Panels & layout controls

| Key | Action |
|-----|--------|
| `w` | Toggle two-pane ↔ single-pane layout |
| `t` | Next theme |
| `[` | Previous theme |
| `T` | Toggle theme panel (right sidebar); `↑`/`↓`/`j`/`k` navigate themes when open |
| `Shift+E` | Open **editor picker** panel; `↑`/`↓`/`j`/`k` navigate, `Enter` select, `Esc` cancel |
| `Shift+O` | Toggle **options panel** |
| `Shift+C` | Toggle `cd-on-exit` on/off |
| `Ctrl+↑` / `Ctrl+↓` | Scroll debug log panel (verbose mode only) |

### Search mode (after pressing `/`)

| Key | Action |
|-----|--------|
| Any character | Append to query — list filters live |
| `Backspace` | Remove last character; empty query exits search |
| `Esc` | Clear query and exit search |
| `↑` / `↓` / `j` / `k` | Scroll the filtered results |
| `→` / `Enter` / `l` | Descend into directory or confirm/navigate entry |
| `←` / `Backspace` / `h` | Ascend to parent directory |

---

## Two-Pane File Manager

The `tfe` binary opens a **split-screen file manager** with a left and right pane.

```
┌─── Left Pane (active) ──────────┬─── Right Pane ───────────────────┐
│ 📁 ~/projects/tui-file-explorer │ 📁 ~/projects/tui-file-explorer  │
├─────────────────────────────────┤──────────────────────────────────┤
│ ▶ 📁 src/                       │   📁 src/                        │
│   📁 examples/                  │   📁 examples/                   │
│   📄 Cargo.toml                 │   📄 Cargo.toml                  │
│   📄 README.md                  │   📄 README.md                   │
├─────────────────────────────────┴──────────────────────────────────┤
│ 📋 Copy: main.rs          Tab pane  y copy  x cut  p paste  d del  │
└────────────────────────────────────────────────────────────────────┘
```

- The **active pane** renders with your full theme accent; the **inactive pane** dims its borders so focus is always clear at a glance
- Press `Tab` to switch which pane has keyboard focus
- Arrow keys (`←`/`→`) scroll the cursor up/down **without** entering or exiting a directory — use `Enter` / `l` to descend and `Backspace` / `h` to ascend
- Press `w` to **collapse to a single pane** — the hidden pane keeps its state and reappears when you press `w` again
- Press `t` / `[` to cycle themes forward / backward; press `T` to open the theme panel
- Each pane navigates independently — scroll to different directories and use one as source, one as destination

### File Operations (Copy / Cut / Paste / Delete)

The classic **Midnight Commander** source-to-destination workflow:

1. **Navigate** to the file you want in the active pane
2. **`y`** to yank (copy) or **`x`** to cut it — the status bar confirms what is in the clipboard
3. **`Tab`** to switch to the other pane and navigate to the destination directory
4. **`p`** to paste — the file is copied (or moved for cut) into that pane's directory

```
Active pane: ~/projects/src/     Other pane: ~/backup/
  ▶ main.rs   ← press y                      ← press p here → main.rs appears
```

If the destination file already exists a **confirmation modal** appears asking whether to overwrite. Delete (`d`) also shows a modal before any data is removed.

All file operations support **directories** — copy and delete both recurse automatically.

---

## Incremental Search

Press `/` to activate search mode. The footer transforms into a live input bar showing your query. Entries are filtered in real-time using a case-insensitive substring match on the file name.

```rust
// Inspect search state programmatically:
println!("searching: {}", explorer.is_searching());
println!("query    : {}", explorer.search_query());
```

**Behaviour details:**
- `Backspace` on a non-empty query pops the last character
- `Backspace` on an empty query deactivates search without dismissing
- `Esc` clears the query and deactivates search — a second `Esc` (when search is already inactive) dismisses the explorer entirely
- Search is automatically cleared when navigating into a subdirectory or ascending to a parent

---

## Sort Modes

Press `s` to cycle through three sort modes, or set one directly:

```rust
use tui_file_explorer::{FileExplorer, SortMode};

let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);

explorer.set_sort_mode(SortMode::SizeDesc);   // largest files first
println!("{}", explorer.sort_mode().label()); // "size ↓"
```

| Mode | Trigger | Description |
|------|---------|-------------|
| `SortMode::Name` | `s` (1st press) | Alphabetical A → Z — the default |
| `SortMode::SizeDesc` | `s` (2nd press) | Largest files first |
| `SortMode::Extension` | `s` (3rd press) | Grouped by extension, then by name |

Directories always sort alphabetically among themselves regardless of the active mode. The current sort mode is shown in the footer status panel at all times.

---

## Extension Filtering

Only files whose extension matches the filter are selectable. Directories are always shown and always navigable, regardless of the filter.

```rust
use tui_file_explorer::FileExplorer;

// Builder API (preferred)
let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    .allow_extension("iso")
    .allow_extension("img")
    .build();

// Or replace the filter at runtime — reloads the listing immediately
explorer.set_extension_filter(["rs", "toml"]);

// Pass an empty filter to allow all files
explorer.set_extension_filter([] as [&str; 0]);
```

Non-matching files are shown dimmed so the directory structure remains visible. Attempting to confirm a non-matching file shows a status message in the footer.

---

## Builder API

### `FileExplorer::builder`

`FileExplorer::builder` gives a fluent, chainable construction API:

```rust
use tui_file_explorer::{FileExplorer, SortMode};

let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    .allow_extension("rs")          // add one extension at a time
    .allow_extension("toml")
    .show_hidden(true)              // show dot-files on startup
    .sort_mode(SortMode::Extension) // initial sort order
    .build();
```

Or pass the full filter list at once:

```rust
let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    .extension_filter(vec!["iso".into(), "img".into()])
    .build();
```

The classic `FileExplorer::new(dir, filter)` constructor is still available and fully backwards-compatible.

---

### `DualPane::builder`

`DualPane::builder` mirrors the single-pane builder and adds dual-pane-specific options:

```rust
use tui_file_explorer::{DualPane, SortMode};
use std::path::PathBuf;

let dual = DualPane::builder(std::env::current_dir().unwrap())
    .right_dir(PathBuf::from("/tmp")) // independent right-pane directory
    .allow_extension("rs")            // applied to both panes
    .allow_extension("toml")
    .show_hidden(false)               // both panes
    .sort_mode(SortMode::Name)        // both panes
    .single_pane(false)               // start in dual-pane mode (default)
    .build();
```

Once built, pane directories, sort mode, and hidden-file visibility can still be changed independently on `dual.left` and `dual.right` at runtime.

| Builder method | Effect |
|---|---|
| `.right_dir(path)` | Independent starting directory for the right pane |
| `.allow_extension(ext)` | Append one extension to the shared filter |
| `.extension_filter(vec)` | Replace the shared filter entirely |
| `.show_hidden(bool)` | Hidden-file visibility for both panes |
| `.sort_mode(mode)` | Initial sort order for both panes |
| `.single_pane(bool)` | Start in single-pane mode (default `false`) |

---

## Theming

Every colour used by the widget is overridable through the `Theme` struct. Pass a `Theme` to `render_themed` instead of `render`:

```rust
use tui_file_explorer::{FileExplorer, Theme, render_themed};
use ratatui::style::Color;

let theme = Theme::default()
    .brand(Color::Magenta)               // widget title
    .accent(Color::Cyan)                 // borders & current path
    .dir(Color::LightYellow)             // directory names & icons
    .sel_bg(Color::Rgb(50, 40, 80))      // highlighted-row background
    .success(Color::Rgb(100, 230, 140))  // status bar & selectable files
    .match_file(Color::Rgb(100, 230, 140));

terminal.draw(|frame| {
    render_themed(&mut explorer, frame, frame.area(), &theme);
})?;
```

### Named presets (27 included)

All presets are available as associated constructors on `Theme`:

```rust
use tui_file_explorer::Theme;

let t = Theme::dracula();
let t = Theme::nord();
let t = Theme::catppuccin_mocha();
let t = Theme::catppuccin_latte();
let t = Theme::tokyo_night();
let t = Theme::tokyo_night_storm();
let t = Theme::gruvbox_dark();
let t = Theme::kanagawa_wave();
let t = Theme::kanagawa_dragon();
let t = Theme::moonfly();
let t = Theme::oxocarbon();
let t = Theme::grape();
let t = Theme::ocean();
let t = Theme::neon();

// Iterate the full catalogue (name, description, theme):
for (name, desc, _theme) in Theme::all_presets() {
    println!("{name} — {desc}");
}
```

**Full preset list:**  
`Default` · `Dracula` · `Nord` · `Solarized Dark` · `Solarized Light` · `Gruvbox Dark` · `Gruvbox Light` · `Catppuccin Latte` · `Catppuccin Frappé` · `Catppuccin Macchiato` · `Catppuccin Mocha` · `Tokyo Night` · `Tokyo Night Storm` · `Tokyo Night Light` · `Kanagawa Wave` · `Kanagawa Dragon` · `Kanagawa Lotus` · `Moonfly` · `Nightfly` · `Oxocarbon` · `Grape` · `Ocean` · `Sunset` · `Forest` · `Rose` · `Mono` · `Neon`

### Palette constants

The default colours are exported as `pub const` values for use alongside complementary widgets:

| Constant | Default | Used for |
|----------|---------|----------|
| `C_BRAND` | `Rgb(255, 100, 30)` | Widget title |
| `C_ACCENT` | `Rgb(80, 200, 255)` | Borders, current-path text |
| `C_SUCCESS` | `Rgb(80, 220, 120)` | Selectable files, status bar |
| `C_DIM` | `Rgb(120, 120, 130)` | Hints, non-matching files |
| `C_FG` | `White` | Default foreground |
| `C_SEL_BG` | `Rgb(40, 60, 80)` | Selected-row background |
| `C_DIR` | `Rgb(255, 210, 80)` | Directory names & icons |
| `C_MATCH` | `Rgb(80, 220, 120)` | Extension-matched file names |

---

## Examples

### `basic`

[`examples/basic.rs`](examples/basic.rs) — a fully self-contained Ratatui app demonstrating:

- Builder API and full event loop
- All `ExplorerOutcome` variants
- Custom `Theme`
- Optional CLI extension-filter arguments

```bash
# Browse everything
cargo run --example basic

# Only .rs and .toml files are selectable
cargo run --example basic -- rs toml md
```

---

### `dual_pane`

[`examples/dual_pane.rs`](examples/dual_pane.rs) — a fully self-contained dual-pane Ratatui app built entirely on the **library API** (no binary code):

- `DualPane::builder` with an optional independent right-pane directory
- `render_dual_pane_themed` for rendering both panes in one call
- All `DualPaneOutcome` variants
- Status bar showing active pane and current layout mode

| Key | Action |
|-----|--------|
| `Tab` | Switch focus left ↔ right |
| `w` | Toggle single-pane / dual-pane mode |
| `↑` / `↓` / `j` / `k` | Move cursor up / down |
| `←` / `→` | Scroll cursor up / down (no navigation side-effects) |
| `Enter` / `l` | Descend into directory or confirm file |
| `Backspace` / `h` | Ascend to parent directory |
| `.` | Toggle hidden files |
| `/` | Incremental search |
| `s` | Cycle sort mode |
| `Esc` / `q` | Quit |

```bash
# Both panes start in the current directory
cargo run --example dual_pane

# Left pane starts in cwd, right pane starts in /tmp
cargo run --example dual_pane -- /tmp
```

---

### `theme_switcher`

[`examples/theme_switcher.rs`](examples/theme_switcher.rs) — live theme switching without restarting, with a sidebar showing the full theme catalogue.

| Key | Action |
|-----|--------|
| `Tab` | Next theme |
| `Shift+Tab` | Previous theme |
| `↑` / `↓` / `j` / `k` | Move cursor up / down |
| `←` / `→` | Scroll cursor up / down (no navigation side-effects) |
| `Enter` / `l` | Descend into directory or confirm file |
| `Backspace` / `h` | Ascend to parent directory |
| `.` | Toggle hidden files |
| `/` | Search |
| `s` | Cycle sort mode |
| `Esc` / `q` | Quit |

```bash
cargo run --example theme_switcher
```

---

### `options`

[`examples/options.rs`](examples/options.rs) — dual-pane explorer with a fully interactive options panel:

- **`Shift+O`** opens and closes the options side panel (toggles, editor row, file-ops reference)
- **`Shift+E`** opens the **editor picker** panel to select from terminal editors and IDEs
- Error snackbar when `e` is pressed with no editor configured
- Toggle `cd-on-exit`, single-pane mode, and hidden-file visibility from the panel
- Selecting a file with `Enter` stays in the TUI and shows a helpful message when no editor is set

| Key | Action |
|-----|--------|
| `Shift+O` | Toggle options panel |
| `Shift+E` | Open editor picker panel |
| `e` | Open current file in editor (shows error snackbar if none set) |
| `Shift+C` | Toggle cd-on-exit |
| `w` | Toggle single-pane mode |
| `h` | Toggle hidden files |
| `s` | Cycle sort mode |
| `t` | Cycle theme |
| `Tab` | Switch active pane |
| `Esc` / `q` | Quit |

```bash
cargo run --example options
```

---

### `editor_picker`

[`examples/editor_picker.rs`](examples/editor_picker.rs) — single-pane explorer demonstrating the full **editor picker panel**:

- Two bordered cells: **"Terminal Editors"** (`none`, `helix`, `nvim`, `vim`, `nano`, `micro`, `emacs`) and **"IDEs & GUI Editors"** (`sublime`, `vscode`, `zed`, `xcode`, `android-studio`, `rustrover`, `intellij`, `webstorm`, `pycharm`, `goland`, `clion`, `fleet`, `rubymine`, `phpstorm`, `rider`, `eclipse`)
- `↑`/`↓` (or `j`/`k`) navigate the list; highlighted entry is shown with a `▶` cursor
- `Enter` sets the selected editor and closes the panel; a checkmark (`✓`) shows the active editor
- Footer shows the binary name for the currently highlighted editor
- `Esc` closes without changing the selection
- `e` opens the highlighted file in the configured editor; shows an error if none is set

| Key | Action |
|-----|--------|
| `Shift+E` | Open / close the editor picker panel |
| `↑` / `k` | Move cursor up in the picker |
| `↓` / `j` | Move cursor down in the picker |
| `Enter` | Confirm the highlighted editor |
| `Esc` | Cancel — editor unchanged |
| `e` | Open current file in the configured editor |
| `Esc` / `q` | Quit (when panel is closed) |

```bash
cargo run --example editor_picker
```

---

### `open_file`

[`examples/open_file.rs`](examples/open_file.rs) — single-pane explorer that opens files in your editor and resumes browsing:

- Resolves editor from CLI arg → `$VISUAL` → `$EDITOR` → `vi`
- Tears down the TUI, launches the editor synchronously, then restores the TUI
- Status bar shows the last-opened filename and editor used
- Supports multi-word editor strings like `"code --wait"`

```bash
# Uses $VISUAL / $EDITOR
cargo run --example open_file

# Explicit editor
cargo run --example open_file -- hx
cargo run --example open_file -- "code --wait"
```

---

## Example Demos

### Navigation & Search

**Run:** `cargo run --example basic`

![Navigation and search](examples/vhs/generated/basic.gif)

Demonstrates directory navigation, incremental search (`/`), sort mode cycling (`s`), hidden-file toggle (`.`), directory descent and ascent, and file selection.

---

### Incremental Search

**Run:** `cargo run --example basic` then press `/`

![Incremental search](examples/vhs/generated/search.gif)

Shows the full search lifecycle: activate with `/`, type to filter live, use `Backspace` to narrow or clear, and `Esc` to cancel without dismissing the explorer.

---

### Sort Modes

**Run:** `cargo run --example basic` then press `s`

![Sort modes](examples/vhs/generated/sort.gif)

Demonstrates `Name → Size ↓ → Extension → Name` cycling, combined with search, and sort persistence across directory navigation.

---

### Extension Filter

**Run:** `cargo run --example basic -- rs toml`

![Extension filter](examples/vhs/generated/filter.gif)

Only `.rs` and `.toml` files are selectable; all other files appear dimmed. The footer reflects the active filter at all times.

---

### File Operations

**Run:** `cargo run --bin tfe`

![Copy, Cut, Paste, Delete](examples/vhs/generated/file_ops.gif)

Demonstrates the full **copy then paste** and **cut (move) then paste** workflows across both panes, followed by a **delete with confirmation modal**. The clipboard status bar updates live after each `y` / `x` / `p` keystroke, and an overwrite-prompt appears when the destination file already exists.

---

### Theme Switcher

**Run:** `cargo run --example theme_switcher`

![Theme switcher](examples/vhs/generated/theme_switcher.gif)

Live theme cycling through all 27 named palettes with a real-time sidebar showing the catalogue and the active theme's description.

---

### Pane Toggle

**Run:** `cargo run --bin tfe`

![Pane toggle](examples/vhs/generated/pane_toggle.gif)

Demonstrates the three layout controls in sequence:

- **`Tab`** — switch keyboard focus between the left and right pane; each pane navigates independently so you can be in different directories at the same time
- **`w`** — collapse to single-pane (the active pane expands to full width) and back to two-pane (the hidden pane reappears with its cursor position preserved)
- **`T`** — open and close the theme-picker sidebar; use `t` / `[` to cycle themes while the panel is open; both panes remain fully navigable with the panel visible

---

### Options Panel & Snackbar

**Run:** `cargo run --bin tfe` then press `Shift+O`

![Options panel and snackbar](examples/vhs/generated/options.gif)

Demonstrates the full options panel workflow:

- **`Shift+O`** — open and close the options side panel (shows Toggles, Editor, and File Ops cells)
- **`Shift+E`** — open the **editor picker** panel; select from Terminal Editors and IDEs & GUI Editors
- **`e`** (no editor set) — triggers the **error snackbar** floating above the action bar, auto-dismissing after 4 seconds; message reads _"No editor set — open Editor picker (Shift + E) to pick one"_
- **`Shift+C`** — toggle `cd-on-exit` on/off with live indicator
- **`w`** — toggle single-pane mode from inside the panel
- **`T`** — open the theme panel (closes the options panel); both panels cannot be open simultaneously

---

### Editor Picker Panel

**Run:** `cargo run --example editor_picker`

![Editor picker panel](examples/vhs/generated/editor_picker.gif)

Demonstrates the editor picker panel:

- **`Shift+E`** — open the floating editor picker; two bordered cells show **Terminal Editors** and **IDEs & GUI Editors**
- **`↑`/`↓`** (or `j`/`k`) — navigate the list; a `▶` cursor tracks the highlighted editor
- **`Enter`** — set the highlighted editor and close the panel; a `✓` marks the active selection
- **`Esc`** — close without changing the editor
- Footer displays the launch binary of the highlighted editor (e.g. `vscode  →  code`)

---

### DualPane Library Widget

**Run:** `cargo run --example dual_pane`

![DualPane library widget](examples/vhs/generated/dual_pane.gif)

A complete two-pane file manager built entirely on the **library API** — no binary code. Demonstrates:

- `DualPane::builder` constructing two independent panes
- `render_dual_pane_themed` rendering both panes in a single call
- **`Tab`** switching keyboard focus; each pane tracks its own cursor and directory
- **`w`** collapsing to single-pane mode (active pane fills the full width) and back
- Incremental search (`/`) and sort cycling (`s`) inside the active pane
- A live status bar showing the active pane label and current layout mode
- `DualPaneOutcome::Selected` — selecting a file prints its path to stdout and exits

---

### Create Entries

**Run:** `cargo run --example create_entries`

![Create entries](examples/vhs/generated/create_entries.gif)

Demonstrates in-place file and directory creation:

- **`n`** — new folder: opens an inline input bar, type a name (supports nested paths like `src/lib`), `Enter` to confirm or `Esc` to cancel
- **`N`** — new file: same input bar, creates an empty file with `touch`
- **`r`** — rename: prompts for the new name of the highlighted entry
- Cursor automatically jumps to the newly created entry after confirmation
- `Backspace` corrects typos mid-input

---

## Demo Quick Reference

| Demo | Command | Highlights |
|------|---------|------------|
| Navigation + Search | `cargo run --example basic` | All key bindings, search, sort, selection |
| Extension filter | `cargo run --example basic -- rs toml` | Dimmed non-matching files, footer status |
| Incremental search | `cargo run --example basic` → `/` | Live filtering, backspace, Esc behaviour |
| Sort modes | `cargo run --example basic` → `s` | Three modes, combined with search |
| **Dual-pane (library)** | `cargo run --example dual_pane` | `DualPane` widget, Tab focus, `w` toggle, status bar |
| **Dual-pane (right dir)** | `cargo run --example dual_pane -- /tmp` | Independent left/right starting directories |
| File operations | `cargo run --bin tfe` | Copy, cut, paste, delete, overwrite modal |
| Theme switcher | `cargo run --example theme_switcher` | 27 live themes, sidebar catalogue |
| Pane toggle | `cargo run --bin tfe` | Tab focus-switch, `w` single/two-pane, `T` theme panel |
| **Options panel** | `cargo run --bin tfe` then `Shift+O` | Options panel, Shift+E editor picker, error snackbar |
| **Editor picker** | `cargo run --example editor_picker` | Editor picker panel, Terminal Editors + IDEs |
| **Create entries** | `cargo run --example create_entries` | `n` mkdir, `N` touch, `r` rename, nested paths |
| **Dual-pane GIF** | `vhs examples/vhs/dual_pane.tape` | Full `dual_pane` example recorded end-to-end |

---

## CLI Usage

```bash
tfe [OPTIONS] [PATH]
```

| Flag | Description |
|------|-------------|
| `[PATH]` | Starting directory (default: current directory) |
| `-e, --ext <EXT>` | Only select files with this extension (repeatable) |
| `-H, --hidden` | Show hidden dot-files on startup |
| `-t, --theme <THEME>` | Colour theme — see `--list-themes` |
| `--list-themes` | Print all 27 available themes and exit |
| `--show-themes` | Open the theme panel on startup (`T` toggles it at runtime) |
| `--single-pane` | Start in single-pane mode (default is two-pane; toggle at runtime with `w`) |
| `--print-dir` | Print the **parent directory** of the selected file instead of the full path |
| `-0, --null` | Terminate output with a NUL byte (for `xargs -0`) |
| `--cd` | Enable cd-on-exit: on dismiss, print the active pane's directory to stdout (persisted) |
| `--no-cd` | Disable cd-on-exit (persisted) |
| `--init <SHELL>` | Install the shell wrapper for cd-on-exit and exit. Shells: `bash`, `zsh`, `fish`, `powershell`, `nushell` |
| `--info` | Print version, platform, environment info and exit |
| `--doctor` | Run diagnostic checks (environment, shell, config, terminal, editor) with pass/warn/fail indicators and exit |
| `-v, --verbose` | Enable verbose mode: startup diagnostics on stderr, debug log file at `$TMPDIR/tfe-debug.log`, and a debug panel inside the TUI |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | File selected — path printed to stdout |
| `0` | Dismissed with `--cd` enabled — active pane's directory printed to stdout |
| `1` | Dismissed without `--cd` — nothing printed |
| `2` | Bad arguments or I/O error |

### Shell integration (cd on exit)

The killer feature: press `Esc` or `q` to dismiss `tfe` and your terminal
**automatically `cd`s** to whichever directory you were browsing.
Works on **macOS, Linux, and Windows** with **bash, zsh, fish, PowerShell,
and Nushell**.

**cd-on-exit is enabled by default.**  Run `tfe --no-cd` to turn it off,
or `tfe --cd` to turn it back on.  The setting is persisted across sessions.

#### How setup works

**On the very first run**, `tfe` automatically:

1. Detects your shell (`$NU_VERSION` for Nushell, `$SHELL` on Unix/macOS,
   `$PSModulePath` for PowerShell on Windows).
2. Writes the shell wrapper function to your rc file (e.g. `~/.zshrc`,
   `~/.config/nushell/config.nu`).
3. Emits a special `source:` directive to stdout that the wrapper — now
   written but not yet active — cannot intercept yet.

Because the wrapper isn't active in the current session until the rc file is
sourced, you need to do **one of the following, once**:

```bash
source ~/.zshrc          # bash / zsh — activate in the current session
source ~/.config/fish/functions/tfe.fish  # fish
. $PROFILE               # PowerShell
source ~/.config/nushell/config.nu        # Nushell
```

Or simply **open a new terminal tab/window** — the wrapper will be active
automatically from that point on, forever.

> **Why only once?**  A child process (like `tfe`) cannot modify its parent
> shell's environment — that is a hard OS rule.  The shell wrapper exists
> precisely to work around this: once it is active, it intercepts `tfe`'s
> stdout, acts on `source:` directives and `cd` paths, and everything is
> fully automatic with no further user action required.

#### From the second session onward — fully automatic

Once the wrapper is active, using `tfe` is completely seamless:

- Dismiss with `Esc` or `q` → shell `cd`s to the directory you were browsing.
- Select a file with `Enter` → path is printed to stdout for piping.
- No flags, no commands, no thought required.

#### Manual installation (`--init`)

If you prefer to install the wrapper explicitly rather than waiting for the
auto-install on first run:

```bash
tfe --init bash        # writes to ~/.bashrc (or ~/.bash_profile if it exists)
tfe --init zsh         # writes to ~/.zshrc  (or $ZDOTDIR/.zshrc / ~/.zshenv)
tfe --init fish        # writes to ~/.config/fish/functions/tfe.fish
tfe --init powershell  # writes to $PROFILE  (Windows / cross-platform pwsh)
tfe --init nushell     # writes to <config-dir>/nushell/config.nu
```

`--init` is idempotent — running it twice will never duplicate the snippet.
It tells you exactly where it wrote and reminds you to source the file.

#### How it works under the hood

| Situation | stdout | exit code |
|---|---|---|
| File selected (`Enter` / `l`) | selected file path | `0` |
| Dismissed (`Esc` / `q`) + cd-on-exit enabled | active pane's directory | `0` |
| Dismissed + cd-on-exit disabled | *(nothing)* | `1` |
| First run — wrapper just installed | `source:<rc-path>` then directory | `0` |

The TUI renders on **stderr** so it is never swallowed by the shell's
`$()` capture.  The wrapper reads `tfe`'s stdout line by line: lines
beginning with `source:` are sourced in the current shell; any other
non-empty line is passed to `cd` / `Set-Location`.

#### Windows

- **PowerShell**: fully supported — `tfe --init powershell` or auto-detected.
- **Nushell**: fully supported on Windows — `tfe --init nushell`.
- **CMD**: not supported (CMD has no shell functions).  Use WSL and run
  `tfe --init bash` or `tfe --init zsh` inside WSL.

```bash
# Open the selected file in $EDITOR (bypasses the wrapper)
command tfe | xargs -r $EDITOR

# Select a Rust source file and edit it
command tfe -e rs | xargs -r nvim

# Start with the Catppuccin Mocha theme and the theme panel open
tfe --theme catppuccin-mocha --show-themes

# Start in single-pane mode (useful for narrow terminals)
tfe --single-pane

# List all available themes
tfe --list-themes

# NUL-delimited output (safe for filenames with spaces or newlines)
command tfe -0 | xargs -0 wc -l
```

> **Theme names** are case-insensitive and hyphens/spaces are interchangeable:  
> `catppuccin-mocha`, `Catppuccin Mocha`, and `catppuccin mocha` all resolve to the same preset.

---

## Troubleshooting

Three built-in tools help diagnose problems — especially on a fresh install
where the TUI fails to appear.

### Quick info (`--info`)

```bash
command tfe --info
```

Prints version, platform, environment variables, terminal capabilities,
detected shell, and config file paths to stderr, then exits.  Useful for
quickly checking which `tfe` binary is running and what it sees.

### Doctor (`--doctor`)

```bash
command tfe --doctor
```

Runs structured diagnostic checks and prints a pass / warn / fail report:

- **Platform** — os, arch
- **Binary** — executable location, `~/.cargo/bin` on `$PATH`
- **Environment** — `$HOME`, `$SHELL`, working directory
- **Terminal** — size, stderr tty status
- **Shell integration** — detected shell, rc file, wrapper installed
- **Config** — state file, persisted settings
- **Editor** — configured editor, binary on `$PATH`

Each failing check includes an actionable hint on how to fix it.

### Verbose mode (`-v` / `--verbose`)

```bash
command tfe -v
```

Enables detailed startup logging:
- **stderr** — each startup step is logged with timestamps (visible before
  the TUI takes over)
- **Log file** — all logs written to `$TMPDIR/tfe-debug.log` (survives the
  alternate screen).  Monitor from another terminal:
  ```bash
  tail -f /tmp/tfe-debug.log
  ```
- **Debug panel** — a scrollable log panel appears at the bottom of the TUI
  (use `Ctrl+↑` / `Ctrl+↓` to scroll)

> **Tip:** Use `command tfe` (not bare `tfe`) to bypass the shell wrapper
> when debugging, so the wrapper doesn't intercept diagnostic output.

---

## Public API

The library exposes three tiers — from a lightweight embeddable widget to the
full-featured app that powers the `tfe` binary.

### Single-pane widget

| Item | Kind | Description |
|------|------|-------------|
| `FileExplorer` | `struct` | Core state machine — cursor, entries, search, sort state |
| `FileExplorerBuilder` | `struct` | Fluent builder for `FileExplorer` |
| `ExplorerOutcome` | `enum` | Result of `handle_key` — `Selected`, `Dismissed`, `Pending`, `Unhandled`, `MkdirCreated`, `TouchCreated`, `RenameCompleted` |
| `FsEntry` | `struct` | A single directory entry (name, path, size, extension, is_dir) |
| `SortMode` | `enum` | `Name` \| `SizeDesc` \| `Extension` |
| `Theme` | `struct` | Colour palette with builder methods and 27 named presets |
| `render` | `fn` | Render one pane using the default theme |
| `render_themed` | `fn` | Render one pane with a custom `Theme` |
| `entry_icon` | `fn` | Map an `FsEntry` to its Unicode icon |
| `fmt_size` | `fn` | Format a byte count as a human-readable string (`1.5 KB`) |

### Dual-pane widget

| Item | Kind | Description |
|------|------|-------------|
| `DualPane` | `struct` | Owns `left` + `right: FileExplorer`; routes keys; manages focus and single-pane mode |
| `DualPaneBuilder` | `struct` | Fluent builder for `DualPane` — independent dirs, shared filter/sort/hidden |
| `DualPaneActive` | `enum` | `Left` \| `Right` — which pane has focus; `.other()` flips it |
| `DualPaneOutcome` | `enum` | Result of `DualPane::handle_key` — `Selected`, `Dismissed`, `Pending`, `Unhandled`, `MkdirCreated`, `TouchCreated`, `RenameCompleted` |
| `render_dual_pane` | `fn` | Render both panes using the default theme |
| `render_dual_pane_themed` | `fn` | Render both panes with a custom `Theme` |

### Full app (same feature set as the `tfe` binary)

All of these are re-exported from `lib.rs` — library consumers get the **exact
same functionality** as the CLI binary. Use `App::new()` + `app.handle_event()`
\+ `draw()` and every feature works through keyboard shortcuts automatically.

| Item | Kind | Description |
|------|------|-------------|
| `App` | `struct` | Top-level state: two panes, clipboard, modals, themes, editor, snackbar, progress |
| `AppOptions` | `struct` | Configuration for `App::new` — dirs, extensions, theme, editor, single-pane, cd-on-exit |
| `Pane` | `enum` | `Left` \| `Right` — which pane is active |
| `ClipOp` | `enum` | `Copy` \| `Cut` — clipboard operation type |
| `ClipboardItem` | `struct` | One or more yanked paths + their `ClipOp` |
| `CopyProgress` | `struct` | Tracks label, done/total count, and current item during paste |
| `Modal` | `enum` | `Delete` \| `MultiDelete` \| `Overwrite` — blocking confirmation dialogs |
| `Editor` | `enum` | `None`, `Helix`, `Neovim`, `Vim`, `Nano`, `Micro`, `Emacs`, `Sublime`, `VSCode`, … |
| `Snackbar` | `struct` | Auto-expiring notification (info or error) |
| `draw` | `fn` | Render the complete app UI (panes + action bar + panels + modals + snackbar + progress) |
| `render_theme_panel` | `fn` | Theme picker side-panel |
| `render_options_panel` | `fn` | Options / key-reference side-panel |
| `render_editor_panel` | `fn` | Editor picker side-panel |
| `render_modal` | `fn` | Delete / overwrite confirmation overlay |
| `render_snackbar` | `fn` | Floating notification overlay |
| `render_copy_progress` | `fn` | Copy/move progress bar overlay |
| `render_action_bar` | `fn` | Bottom status bar (clipboard, pane, editor info) |
| `render_nav_hints` | `fn` | Key-binding hint rows |
| `copy_dir_all` | `fn` | Recursive directory copy (skips symlinks) |
| `resolve_output_path` | `fn` | Resolve file → parent dir when `--print-dir` is active |
| `AppState` | `struct` | Persisted state (theme, dirs, sort, hidden, editor, cd-on-exit) |
| `load_state` / `save_state` | `fn` | Read/write `AppState` from/to `~/.config/tfe/state.json` |

---

## Module Layout

All modules are re-exported from `lib.rs` — library consumers have access to
the full feature set, from lightweight widgets to the complete app.

### Core widget modules

| Module | Contents |
|--------|----------|
| `types` | `FsEntry`, `ExplorerOutcome`, `SortMode` — data types only, no I/O |
| `palette` | Palette constants + `Theme` builder + 27 named presets |
| `explorer` | `FileExplorer`, `FileExplorerBuilder`, `entry_icon`, `fmt_size` |
| `dual_pane` | `DualPane`, `DualPaneBuilder`, `DualPaneActive`, `DualPaneOutcome` |
| `render` | `render`, `render_themed`, `render_dual_pane`, `render_dual_pane_themed` — pure rendering, no state |

Because rendering is fully decoupled from state, you can slot either widget into any Ratatui layout, render it conditionally as an overlay, or build a completely custom renderer by reading `FileExplorer`'s public fields directly.

### Full-app modules (also available to library consumers)

| Module | Contents |
|--------|----------|
| `app` | `App`, `AppOptions`, `Pane`, `ClipOp`, `ClipboardItem`, `Modal`, `CopyProgress`, `Editor`, `Snackbar`, `handle_key`, `handle_event` |
| `ui` | `draw()`, `render_theme_panel()`, `render_editor_panel()`, `render_options_panel()`, `render_nav_hints()`, `render_action_bar()`, `render_modal()`, `render_snackbar()`, `render_copy_progress()` |
| `fs` | `copy_dir_all()`, `resolve_output_path()` |
| `persistence` | `AppState`, `load_state()`, `save_state()`, `resolve_theme_idx()` |

### Binary-only (not re-exported)

| Module | Contents |
|--------|----------|
| `main` | `Cli` struct (argument parsing), `run()`, `run_loop()` — thin entry-point only |
| `shell_init` | `Shell`, `detect_shell()`, `snippet()`, `rc_path_with()`, `is_installed()`, `install()`, `install_or_print()` |

---

## Generating Demo GIFs

Install [VHS](https://github.com/charmbracelet/vhs) then run:

```bash
# Generate all GIFs at once (requires just)
just vhs-all

# Or individually
vhs examples/vhs/basic.tape
vhs examples/vhs/search.tape
vhs examples/vhs/sort.tape
vhs examples/vhs/filter.tape
vhs examples/vhs/file_ops.tape
vhs examples/vhs/theme_switcher.tape
vhs examples/vhs/pane_toggle.tape
vhs examples/vhs/dual_pane.tape
vhs examples/vhs/options.tape
vhs examples/vhs/editor_picker.tape
vhs examples/vhs/create_entries.tape
```

GIFs are written to `examples/vhs/generated/` and tracked with **Git LFS**.

| Tape | Demo | Command |
|------|------|---------|
| `basic.tape` | Navigation, search, sort, hidden-file toggle, selection | `cargo run --example basic` |
| `search.tape` | Full incremental search lifecycle | `cargo run --example basic` |
| `sort.tape` | Name → Size ↓ → Extension cycling | `cargo run --example basic` |
| `filter.tape` | Extension filter — selectable vs dimmed files | `cargo run --example basic -- rs toml` |
| `file_ops.tape` | Copy, cut, paste, delete, overwrite modal | `cargo run --bin tfe` |
| `theme_switcher.tape` | Live cycling of all 27 themes with sidebar | `cargo run --example theme_switcher` |
| `pane_toggle.tape` | Tab focus-switch, `w` single/dual, `T` theme panel | `cargo run --bin tfe` |
| `dual_pane.tape` | `DualPane` library widget — Tab, `w`, status bar | `cargo run --example dual_pane` |
| `options.tape` | Options panel, Shift+E editor picker, toggles, error snackbar | `cargo run --bin tfe` |
| `editor_picker.tape` | Editor picker panel — Terminal Editors and IDEs & GUI Editors | `cargo run --example editor_picker` |
| `create_entries.tape` | New folder (`n`), new file (`N`), rename (`r`), nested paths | `cargo run --example create_entries` |

---

## Development

### Prerequisites

- Rust 1.75.0 or later (MSRV)
- [`just`](https://github.com/casey/just) — task runner
- [`git-cliff`](https://github.com/orhun/git-cliff) — changelog generator
- [`vhs`](https://github.com/charmbracelet/vhs) — GIF recorder (optional, for demos)

```bash
just install-tools
```

### Common tasks

```bash
just fmt          # format code
just clippy       # run linter (zero warnings enforced)
just test         # run tests
just check-all    # fmt + clippy + test in one shot
just doc          # build and open docs

just bump 0.3.6   # interactive version bump + tag
just release 0.3.6 # non-interactive: bump + commit + tag + push (triggers CI release)

just vhs basic           # record a single demo GIF
just vhs-all             # record all demo GIFs

just --list       # see all available commands
```

---

## License

MIT — see [LICENSE](LICENSE) for details.

---

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## Acknowledgments

Built for the [Ratatui](https://github.com/ratatui/ratatui) ecosystem.  
Special thanks to the Ratatui team for an outstanding TUI framework.
