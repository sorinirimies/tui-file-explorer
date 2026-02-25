# tui-file-explorer

[![Crates.io](https://img.shields.io/crates/v/tui-file-explorer)](https://crates.io/crates/tui-file-explorer)
[![Documentation](https://docs.rs/tui-file-explorer/badge.svg)](https://docs.rs/tui-file-explorer)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Release](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/release.yml/badge.svg)](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/release.yml)
[![CI](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/ci.yml/badge.svg)](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/tui-file-explorer)](https://crates.io/crates/tui-file-explorer)

A self-contained, keyboard-driven file-browser widget for [Ratatui](https://ratatui.rs).  
Works both as an **embeddable library widget** and as a **standalone CLI tool** (`tfe`).

---

## Preview

### Navigation & Search
![Basic navigation and incremental search](examples/vhs/generated/basic.gif)

### Incremental Search (`/`)
![Live incremental search](examples/vhs/generated/search.gif)

### Sort Modes (`s`)
![Sort mode cycling](examples/vhs/generated/sort.gif)

### Extension Filter
![Extension filter](examples/vhs/generated/filter.gif)

### Theme Switcher
![Live theme switching](examples/vhs/generated/theme_switcher.gif)

> **Generating the GIFs** — install [VHS](https://github.com/charmbracelet/vhs) then run:
> ```bash
> just vhs-all
> # or individually:
> vhs examples/vhs/basic.tape
> vhs examples/vhs/search.tape
> vhs examples/vhs/sort.tape
> vhs examples/vhs/filter.tape
> vhs examples/vhs/theme_switcher.tape
> ```

---

## Features

- 📁 Directories-first listing with case-insensitive alphabetical sorting
- 🔍 **Incremental search** — press `/` to filter entries live as you type
- 🔃 **Sort modes** — cycle `Name → Size ↓ → Extension` with `s`
- 🎛️ **Extension filter** — only matching files are selectable; dirs always navigable
- 👁 Toggle hidden (dot-file) visibility with `.`
- ⌨️ Full keyboard navigation: arrow keys, vim keys (`h/j/k/l`), `PgUp/PgDn`, `g/G`
- 🎨 Fully themeable palette — 27 named presets + custom `Theme` builder
- 🔧 Fluent builder API for ergonomic configuration
- 🖥️ Standalone CLI binary (`tfe`) with shell integration
- ✅ Lean library — only `ratatui` + `crossterm` required (`clap` is opt-out)

---

## Stats

| Metric | Value |
|--------|-------|
| Dependencies (library) | 2 (`ratatui`, `crossterm`) |
| Named colour themes | 27 |
| Sort modes | 3 (`Name`, `Size ↓`, `Extension`) |
| Key bindings | 15+ |
| File-type icons | 50+ extensions mapped |
| Public API surface | 5 types, 2 free functions |
| Test coverage | 32 unit tests + 28 doc-tests |

---

## Installation

### As a library

```toml
[dependencies]
tui-file-explorer = "0.1"
```

Library-only (without the `clap`-powered CLI binary):

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
| `/` | **Activate incremental search** |
| `s` | **Cycle sort mode** (`Name → Size ↓ → Extension`) |
| `.` | Toggle hidden (dot-file) entries |
| `Esc` | Clear search (if active), then dismiss |
| `q` | Dismiss (when search is not active) |

### Search mode (`/` activated)

| Key | Action |
|-----|--------|
| Any character | Append to search query (live filter) |
| `Backspace` | Remove last character; empty query → exit search |
| `Esc` | Clear query and exit search (returns `Pending`, not `Dismissed`) |
| `↑` / `↓` / `Enter` | Navigate the filtered results as normal |

---

## Incremental Search

Press `/` to activate search mode. The footer transforms into a live input bar
showing your query. Entries are filtered in real-time using a case-insensitive
substring match on the file name.

```rust
// Inspect search state programmatically:
println!("searching: {}", explorer.is_searching());
println!("query    : {}", explorer.search_query());
```

**Behaviour details:**
- `Backspace` on a non-empty query pops the last character.
- `Backspace` on an empty query deactivates search without dismissing.
- `Esc` clears the query and deactivates search — a second `Esc` (when search is already inactive) dismisses the explorer entirely.
- Search is automatically cleared when descending into a subdirectory or ascending to a parent, keeping the filter scoped to one directory at a time.

---

## Sort Modes

Press `s` to cycle through three sort modes, or set one directly:

```rust
use tui_file_explorer::{FileExplorer, SortMode};

let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);

explorer.set_sort_mode(SortMode::SizeDesc);   // largest files first
println!("{}", explorer.sort_mode().label()); // "size ↓"
```

| Mode | Key | Description |
|------|-----|-------------|
| `SortMode::Name` | `s` (1st) | Alphabetical A → Z — the default |
| `SortMode::SizeDesc` | `s` (2nd) | Largest files first |
| `SortMode::Extension` | `s` (3rd) | Grouped by extension, then name |

Directories always sort alphabetically among themselves regardless of the active mode.

The current sort mode is shown in the footer status panel at all times.

---

## Extension Filtering

Only files whose extension matches the filter are selectable.  
Directories are always shown and always navigable, regardless of the filter.

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

Non-matching files are shown dimmed in the list so the directory structure
remains visible. Attempting to confirm a non-matching file shows a status
message in the footer.

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

The classic `FileExplorer::new(dir, filter)` constructor is still available
and fully backwards-compatible.

---

## Theming

Every colour used by the widget is overridable through the `Theme` struct.
Pass a `Theme` to `render_themed` instead of `render`:

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
let t = Theme::tokyo_night();
let t = Theme::gruvbox_dark();
let t = Theme::kanagawa_wave();
let t = Theme::oxocarbon();
let t = Theme::grape();
let t = Theme::ocean();
let t = Theme::neon();

// Iterate the full catalogue (name, description, theme):
for (name, desc, _theme) in Theme::all_presets() {
    println!("{name} — {desc}");
}
```

### Palette constants

The default colours are exported as `pub const` values in `palette` for use
alongside complementary widgets:

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

- Builder API
- Full event loop (raw mode, alternate screen)
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

[`examples/theme_switcher.rs`](examples/theme_switcher.rs) — **live theme switching** without restarting.  
Press `Tab` / `Shift+Tab` to cycle through eight built-in named themes.

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

![Basic navigation](examples/vhs/generated/basic.gif)

Demonstrates directory navigation, incremental search (`/`), sort mode cycling (`s`), hidden-file toggle, and file selection.

---

### Incremental Search

**Run:** `cargo run --example basic` then press `/`

![Incremental search](examples/vhs/generated/search.gif)

Shows the full search lifecycle: activate with `/`, type to filter live, use `Backspace` to narrow/clear, and `Esc` to cancel without dismissing.

---

### Sort Modes

**Run:** `cargo run --example basic` then press `s`

![Sort modes](examples/vhs/generated/sort.gif)

Demonstrates `Name → Size ↓ → Extension → Name` cycling, combined with search, and sort persistence across directory navigation.

---

### Extension Filter

**Run:** `cargo run --example basic -- rs toml`

![Extension filter](examples/vhs/generated/filter.gif)

Shows only `.rs` and `.toml` files as selectable; all other files appear dimmed. The footer reflects the active filter at all times.

---

### Theme Switcher

**Run:** `cargo run --example theme_switcher`

![Theme switcher](examples/vhs/generated/theme_switcher.gif)

Live theme cycling through eight named palettes — Default, Grape, Ocean, Sunset, Forest, Rose, Mono, and Neon.

---

## Demo Quick Reference

| Demo | Command | Features |
|------|---------|----------|
| Navigation + Search | `cargo run --example basic` | All key bindings, search, sort |
| Extension filter | `cargo run --example basic -- rs toml` | Dimmed non-matching files |
| Theme switcher | `cargo run --example theme_switcher` | 8 live themes, `Tab` to cycle |

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
| `--print-dir` | Print the selected file's **parent directory** instead of the full path |
| `-0, --null` | Terminate output with a NUL byte (for `xargs -0`) |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | File selected — path printed to stdout |
| `1` | Dismissed (Esc / q) without selecting |
| `2` | Bad arguments or I/O error |

### Shell integration

```bash
# Open the selected file in $EDITOR
tfe | xargs -r $EDITOR

# cd into the directory containing the selected file
cd "$(tfe --print-dir)"

# Select a Rust source file, then edit it
tfe -e rs | xargs -r nvim

# Use the Catppuccin Mocha theme
tfe --theme catppuccin-mocha

# List all available themes
tfe --list-themes

# NUL-delimited output (safe for filenames with spaces or newlines)
tfe -0 | xargs -0 wc -l
```

> Theme names are case-insensitive and hyphens/spaces are interchangeable:  
> `catppuccin-mocha`, `Catppuccin Mocha`, and `catppuccin mocha` all resolve to the same theme.

---

## Public API

The public surface is intentionally narrow for stability:

| Item | Kind | Description |
|------|------|-------------|
| `FileExplorer` | `struct` | Core state machine — holds cursor, entries, search, sort state |
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

| Module | Contents |
|--------|----------|
| `types` | `FsEntry`, `ExplorerOutcome`, `SortMode` — data types only, no I/O |
| `palette` | Palette constants + `Theme` builder + 27 named presets |
| `explorer` | `FileExplorer`, `FileExplorerBuilder`, `entry_icon`, `fmt_size` |
| `render` | `render`, `render_themed` — pure rendering, no state |

Because rendering is fully decoupled from state, you can slot the explorer into
any Ratatui layout, render it conditionally as an overlay, or build a completely
custom renderer by reading `FileExplorer`'s public fields directly.

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
vhs examples/vhs/theme_switcher.tape
```

GIFs are stored under `examples/vhs/generated/` and tracked with **Git LFS**.

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
```

```bash
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