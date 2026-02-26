# tui-file-explorer

[![Crates.io](https://img.shields.io/crates/v/tui-file-explorer)](https://crates.io/crates/tui-file-explorer)
[![Documentation](https://docs.rs/tui-file-explorer/badge.svg)](https://docs.rs/tui-file-explorer)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Release](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/release.yml/badge.svg)](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/release.yml)
[![CI](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/ci.yml/badge.svg)](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/tui-file-explorer)](https://crates.io/crates/tui-file-explorer)

A keyboard-driven, two-pane file manager widget for [Ratatui](https://ratatui.rs).  
Use it as an **embeddable library widget** or run it as the **standalone `tfe` CLI tool**.

---

## Preview

<table>
  <tr>
    <td align="center">
      <img src="examples/vhs/generated/basic.gif" alt="Navigation, search and sort"/>
      <br/><sub><b>Navigation &middot; Search &middot; Sort</b></sub>
    </td>
    <td align="center">
      <img src="examples/vhs/generated/search.gif" alt="Incremental search"/>
      <br/><sub><b>Incremental Search</b></sub>
    </td>
    <td align="center">
      <img src="examples/vhs/generated/sort.gif" alt="Sort modes"/>
      <br/><sub><b>Sort Modes</b></sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="examples/vhs/generated/filter.gif" alt="Extension filter"/>
      <br/><sub><b>Extension Filter</b></sub>
    </td>
    <td align="center">
      <img src="examples/vhs/generated/file_ops.gif" alt="Copy, Cut, Paste, Delete"/>
      <br/><sub><b>File Operations</b></sub>
    </td>
    <td align="center">
      <img src="examples/vhs/generated/theme_switcher.gif" alt="27 live themes"/>
      <br/><sub><b>27 Live Themes</b></sub>
    </td>
  </tr>
</table>

---

## Features

- 🗂️ **Two-pane layout** — independent left and right explorer panes, `Tab` to switch focus
- 📋 **File operations** — copy (`y`), cut (`x`), paste (`p`), and delete (`d`) between panes
- 🔍 **Incremental search** — press `/` to filter entries live as you type
- 🔃 **Sort modes** — cycle `Name → Size ↓ → Extension` with `s`
- 🎛️ **Extension filter** — only matching files are selectable; dirs are always navigable
- 👁️ Toggle hidden dot-file visibility with `.`
- ⌨️ Full keyboard navigation: arrow keys, vim keys (`h/j/k/l`), `PgUp/PgDn`, `g/G`
- 🎨 **27 named themes** — Catppuccin, Dracula, Nord, Tokyo Night, Kanagawa, Gruvbox, and more
- 🎛️ **Live theme panel** — press `t` to open a side panel, `[`/`]` to cycle themes
- 🔧 Fluent builder API for ergonomic embedding
- 🖥️ Standalone `tfe` binary with full shell-pipeline integration
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
| Public API surface | 6 types, 4 free functions |
| Unit tests | 93 |

---

## Installation

### As a library

```toml
[dependencies]
tui-file-explorer = "0.1"
ratatui = "0.30"
```

Library-only (no `clap`-powered CLI binary):

```toml
[dependencies]
tui-file-explorer = { version = "0.1", default-features = false }
```

### As a CLI tool

```bash
cargo install tui-file-explorer
```

Installs the `tfe` binary onto your `PATH`.

---

## Quick Start

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

// 3. Inside your key-handler:
let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
match explorer.handle_key(key) {
    ExplorerOutcome::Selected(path) => println!("chosen: {}", path.display()),
    ExplorerOutcome::Dismissed      => { /* close the overlay */ }
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
| `Enter` / `→` / `l` | Descend into directory or confirm file |
| `Backspace` / `←` / `h` | Ascend to parent directory |
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
| `y` | **Yank** — mark highlighted entry for copy |
| `x` | **Cut** — mark highlighted entry for move |
| `p` | **Paste** — copy/move clipboard into the *other* pane's directory |
| `d` | **Delete** — remove highlighted entry (asks for confirmation) |

### Layout & theme controls

| Key | Action |
|-----|--------|
| `w` | Toggle two-pane ↔ single-pane layout |
| `t` | Next theme |
| `T` | Toggle theme panel (right sidebar) |
| `[` | Previous theme |

### Search mode (after pressing `/`)

| Key | Action |
|-----|--------|
| Any character | Append to query — list filters live |
| `Backspace` | Remove last character; empty query exits search |
| `Esc` | Clear query and exit search |
| `↑` / `↓` / `Enter` | Navigate the filtered results normally |

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

### `theme_switcher`

[`examples/theme_switcher.rs`](examples/theme_switcher.rs) — live theme switching without restarting, with a sidebar showing the full theme catalogue.

| Key | Action |
|-----|--------|
| `Tab` | Next theme |
| `Shift+Tab` | Previous theme |
| `↑/↓/j/k` | Navigate file list |
| `Enter` | Descend / select |
| `Backspace` | Ascend |
| `.` | Toggle hidden files |
| `/` | Search |
| `s` | Cycle sort mode |
| `Esc` / `q` | Quit |

```bash
cargo run --example theme_switcher
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

## Demo Quick Reference

| Demo | Command | Highlights |
|------|---------|------------|
| Navigation + Search | `cargo run --example basic` | All key bindings, search, sort, selection |
| Extension filter | `cargo run --example basic -- rs toml` | Dimmed non-matching files, footer status |
| Incremental search | `cargo run --example basic` → `/` | Live filtering, backspace, Esc behaviour |
| Sort modes | `cargo run --example basic` → `s` | Three modes, combined with search |
| File operations | `cargo run --bin tfe` | Copy, cut, paste, delete, overwrite modal |
| Theme switcher | `cargo run --example theme_switcher` | 27 live themes, sidebar catalogue |
| Pane toggle | `cargo run --bin tfe` | Tab focus-switch, `w` single/two-pane, `T` theme panel |

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
| `--print-dir` | Print the selected file's **parent directory** instead of the full path |
| `-0, --null` | Terminate output with a NUL byte (for `xargs -0`) |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | File selected — path printed to stdout |
| `1` | Dismissed (`Esc` / `q`) without selecting |
| `2` | Bad arguments or I/O error |

### Shell integration

```bash
# Open the selected file in $EDITOR
tfe | xargs -r $EDITOR

# cd into the directory containing the selected file
cd "$(tfe --print-dir)"

# Select a Rust source file and edit it
tfe -e rs | xargs -r nvim

# Start with the Catppuccin Mocha theme and the theme panel open
tfe --theme catppuccin-mocha --show-themes

# Start in single-pane mode (useful for narrow terminals or shell pipelines)
tfe --single-pane

# List all available themes
tfe --list-themes

# NUL-delimited output (safe for filenames with spaces or newlines)
tfe -0 | xargs -0 wc -l
```

> **Theme names** are case-insensitive and hyphens/spaces are interchangeable:  
> `catppuccin-mocha`, `Catppuccin Mocha`, and `catppuccin mocha` all resolve to the same preset.

---

## Public API

The public surface is intentionally narrow for stability:

| Item | Kind | Description |
|------|------|-------------|
| `FileExplorer` | `struct` | Core state machine — cursor, entries, search, sort state |
| `FileExplorerBuilder` | `struct` | Fluent builder for `FileExplorer` |
| `ExplorerOutcome` | `enum` | Result of `handle_key` — `Selected`, `Dismissed`, `Pending`, `Unhandled` |
| `FsEntry` | `struct` | A single directory entry (name, path, size, extension, is_dir) |
| `SortMode` | `enum` | `Name` \| `SizeDesc` \| `Extension` |
| `Theme` | `struct` | Colour palette with builder methods and 27 named presets |
| `render` | `fn` | Render using the default theme |
| `render_themed` | `fn` | Render with a custom `Theme` |
| `entry_icon` | `fn` | Map an `FsEntry` to its Unicode icon |
| `fmt_size` | `fn` | Format a byte count as a human-readable string (`1.5 KB`) |

---

## Module Layout

### Library (`src/lib.rs` re-exports)

| Module | Contents |
|--------|----------|
| `types` | `FsEntry`, `ExplorerOutcome`, `SortMode` — data types only, no I/O |
| `palette` | Palette constants + `Theme` builder + 27 named presets |
| `explorer` | `FileExplorer`, `FileExplorerBuilder`, `entry_icon`, `fmt_size` |
| `render` | `render`, `render_themed` — pure rendering, no state |

Because rendering is fully decoupled from state, you can slot the explorer into any Ratatui layout, render it conditionally as an overlay, or build a completely custom renderer by reading `FileExplorer`'s public fields directly.

### Binary (`tfe` CLI, not part of the public library API)

| Module | Contents |
|--------|----------|
| `main` | `Cli` struct (argument parsing), `run()`, `run_loop()` — thin entry-point only |
| `app` | `App` state, `Pane`, `ClipOp`, `ClipboardItem`, `Modal`, `handle_event` |
| `ui` | `draw()`, `render_theme_panel()`, `render_action_bar()`, `render_modal()` |
| `fs` | `copy_dir_all()`, `emit_path()`, `resolve_output_path()` |
| `persistence` | `AppState`, `load_state()`, `save_state()`, `resolve_theme_idx()` |

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
```

GIFs are written to `examples/vhs/generated/` and tracked with **Git LFS**.

---

## Development

### Prerequisites

- Rust 1.74.0 or later
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

just bump 0.2.0   # interactive version bump + tag
just release 0.2.0 # non-interactive: bump + commit + tag + push (triggers CI release)

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