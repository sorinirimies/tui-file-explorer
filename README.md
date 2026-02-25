# tui-file-explorer

A self-contained, keyboard-driven file-browser widget for [Ratatui](https://ratatui.rs).

## Demo

| Basic navigation | Extension filter | Theme switcher |
|:---:|:---:|:---:|
| ![basic demo](examples/vhs/generated/basic.gif) | ![filter demo](examples/vhs/generated/filter.gif) | ![theme switcher demo](examples/vhs/generated/theme_switcher.gif) |

> **Generating the GIFs** — install [VHS](https://github.com/charmbracelet/vhs) and run:
> ```text
> vhs examples/vhs/basic.tape
> vhs examples/vhs/filter.tape
> vhs examples/vhs/theme_switcher.tape
> ```

---

## Features

- 📁 Directories-first listing with case-insensitive alphabetical sorting
- 🔍 Optional extension filter — only matching files are selectable
- 👁 Toggle hidden (dot-file) visibility with `.`
- ⌨️ Full keyboard navigation: arrow keys, vim keys (`h/j/k/l`), `PgUp/PgDn`, `Home/End`
- 🎨 Fully themeable palette via the `Theme` builder — 27 named presets included
- 🔧 Builder API for ergonomic configuration
- 🖥️ Works as both a **library widget** and a **standalone CLI** (`tfe`)
- ✅ Lean library — only `ratatui` + `crossterm` (CLI binary adds `clap`, opt-out supported)

---

## Installation

### As a library

```toml
[dependencies]
tui-file-explorer = "0.1"
```

If you only want the widget and don't want to pull in `clap`, opt out of the
default `cli` feature:

```toml
[dependencies]
tui-file-explorer = { version = "0.1", default-features = false }
```

### As a CLI tool

```text
cargo install tui-file-explorer
```

This installs the `tfe` binary onto your `PATH`.

---

## CLI usage

```text
tfe [OPTIONS] [PATH]
```

| Flag | Description |
|------|-------------|
| `[PATH]` | Starting directory (default: current directory) |
| `-e, --ext <EXT>` | Only select files with this extension — repeatable |
| `-H, --hidden` | Show hidden dot-files on startup |
| `-t, --theme <THEME>` | Colour theme (see `--list-themes`) |
| `--list-themes` | Print all 27 available themes and exit |
| `--print-dir` | Print the selected file's **parent directory** instead of the full path |
| `-0, --null` | Terminate output with a NUL byte (for `xargs -0`) |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | A file was selected — path printed to stdout |
| `1` | Dismissed (Esc / q) without selecting anything |
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

# Show all available themes
tfe --list-themes

# NUL-delimited output (safe for filenames with spaces)
tfe -0 | xargs -0 wc -l
```

> Theme names are case-insensitive and hyphens/spaces are interchangeable:
> `catppuccin-mocha`, `Catppuccin Mocha`, and `catppuccin mocha` all work.

---

## Quick start

```rust
use tui_file_explorer::{FileExplorer, ExplorerOutcome, render};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Create once (e.g. in App::new)
let mut explorer = FileExplorer::new(
    std::env::current_dir().unwrap(),
    vec!["iso".into(), "img".into()],   // pass vec![] to allow all files
);

// Inside Terminal::draw:
// render(&mut explorer, frame, frame.area());

// Inside your key-handler:
let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
match explorer.handle_key(key) {
    ExplorerOutcome::Selected(path) => println!("chosen: {}", path.display()),
    ExplorerOutcome::Dismissed      => { /* close the overlay */ }
    _                               => {}
}
```

---

## Example

### `basic`

[`examples/basic.rs`](examples/basic.rs) — a fully self-contained Ratatui app. It shows:

- Building the explorer with the **builder API**
- Embedding it in a real Ratatui event loop (raw mode, alternate screen)
- Handling all `ExplorerOutcome` variants
- Applying a custom **`Theme`**
- Accepting optional extension-filter arguments from the CLI

```text
# browse everything
cargo run --example basic

# only .rs and .toml files are selectable
cargo run --example basic -- rs toml md
```

### `theme_switcher`

[`examples/theme_switcher.rs`](examples/theme_switcher.rs) — demonstrates **live theme switching**. Eight named colour presets are bundled; press `Tab` / `Shift+Tab` to cycle through them without restarting the app. A sidebar shows the theme list and highlights the active one.

| Theme | Character |
|-------|-----------|
| **Default** | Orange title, cyan borders, yellow dirs |
| **Grape** | Deep violet & soft blue |
| **Ocean** | Teal & aquamarine |
| **Sunset** | Warm amber & rose |
| **Forest** | Earthy greens & bark browns |
| **Rose** | Pinks & corals |
| **Mono** | Greyscale only |
| **Neon** | Electric brights — synthwave / retro |

```text
cargo run --example theme_switcher
```

| Key | Action |
|-----|--------|
| `Tab` | Next theme |
| `Shift+Tab` | Previous theme |
| `↑/↓/j/k` | Navigate file list |
| `Enter` | Descend / select |
| `Backspace` | Ascend |
| `.` | Toggle hidden files |
| `Esc` / `q` | Quit |

---

## Configuration

### Builder API

`FileExplorer::builder` gives you a fluent, chainable construction API:

```rust
use tui_file_explorer::FileExplorer;

let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    // restrict selectable files to Rust sources and manifests
    .allow_extension("rs")
    .allow_extension("toml")
    // start with hidden dot-files visible
    .show_hidden(true)
    .build();
```

Alternatively, pass the full filter list at once:

```rust
let explorer = FileExplorer::builder(std::env::current_dir().unwrap())
    .extension_filter(vec!["iso".into(), "img".into()])
    .build();
```

The classic `FileExplorer::new(dir, filter)` constructor is still available and
forwards to the same internal logic.

### Public fields

After construction every configuration field is publicly accessible, so you can
mutate the explorer at any time:

```rust
explorer.show_hidden = true;
explorer.extension_filter.push("log".into());
explorer.reload(); // re-scan the current directory
```

---

## Theming

Every colour used by the widget is exposed through the `Theme` struct.
Pass a `Theme` to `render_themed` instead of `render` to override any or all
colours:

```rust
use tui_file_explorer::{FileExplorer, Theme, render_themed};
use ratatui::style::Color;

// Start from the defaults and tweak what you need.
let theme = Theme::default()
    .brand(Color::Magenta)               // widget title
    .accent(Color::Cyan)                 // borders & path
    .dir(Color::LightYellow)             // directory names
    .sel_bg(Color::Rgb(50, 40, 80))      // highlighted row background
    .success(Color::Rgb(100, 230, 140))  // status bar & selectable files
    .match_file(Color::Rgb(100, 230, 140));

terminal.draw(|frame| {
    render_themed(&mut explorer, frame, frame.area(), &theme);
})?;
```

`Theme` also implements `Default`, so you can construct one field-by-field:

```rust
let mut theme = Theme::default();
theme.brand  = Color::Rgb(255, 80, 120);
theme.accent = Color::Rgb(80, 180, 255);
```

### Palette constants

The eight default colours are also exported as plain `pub const` values in the
`palette` module, in case you want to reference them when building complementary
widgets:

| Constant     | Default value          | Used for                          |
|--------------|------------------------|-----------------------------------|
| `C_BRAND`    | `Rgb(255, 100, 30)`    | Widget title                      |
| `C_ACCENT`   | `Rgb(80, 200, 255)`    | Borders, current-path text        |
| `C_SUCCESS`  | `Rgb(80, 220, 120)`    | Selectable files, status bar      |
| `C_DIM`      | `Rgb(120, 120, 130)`   | Hints, non-matching files         |
| `C_FG`       | `White`                | Default foreground                |
| `C_SEL_BG`   | `Rgb(40, 60, 80)`      | Selected-row background           |
| `C_DIR`      | `Rgb(255, 210, 80)`    | Directory names & icons           |
| `C_MATCH`    | `Rgb(80, 220, 120)`    | Extension-matched file names      |

---

## Keyboard bindings

| Key | Action |
|---|---|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `PgUp` | Jump up 10 rows |
| `PgDn` | Jump down 10 rows |
| `Home` / `g` | Jump to top |
| `End` / `G` | Jump to bottom |
| `Enter` / `→` / `l` | Descend into directory / confirm file |
| `Backspace` / `←` / `h` | Ascend to parent directory |
| `.` | Toggle hidden files |
| `Esc` / `q` | Dismiss explorer |

---

## Modularity

The crate is split into four focused modules that you can use independently:

| Module    | Contents                                                   |
|-----------|------------------------------------------------------------|
| `types`   | `FsEntry`, `ExplorerOutcome` — data types only, no I/O     |
| `palette` | Palette constants + `Theme` builder — no widget logic      |
| `explorer`| `FileExplorer` state machine + `FileExplorerBuilder`       |
| `render`  | `render` and `render_themed` — pure rendering, no state    |

Because the render functions are decoupled from state, you can slot the explorer
into any Ratatui layout, render it conditionally as an overlay, or replace the
renderer entirely by reading `FileExplorer`'s public fields and painting your
own widget.

---

## License

MIT