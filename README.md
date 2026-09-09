# tui-file-explorer

[![Crates.io](https://img.shields.io/crates/v/tui-file-explorer?color=orange)](https://crates.io/crates/tui-file-explorer)
[![Documentation](https://docs.rs/tui-file-explorer/badge.svg)](https://docs.rs/tui-file-explorer)
[![CI](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/ci.yml/badge.svg)](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/ci.yml)
[![Release](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/release.yml/badge.svg)](https://github.com/sorinirimies/tui-file-explorer/actions/workflows/release.yml)
[![Downloads](https://img.shields.io/crates/d/tui-file-explorer)](https://crates.io/crates/tui-file-explorer)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Keyboard-driven, multi-pane file manager for [Ratatui](https://ratatui.rs). Use it as an embeddable widget or install the standalone `tfe` CLI.

## Preview

All recordings use the same 1600×900 canvas and render at 800×450 below.

### Dynamic multi-pane layout

<img src="examples/vhs/generated/multi_pane.gif" alt="Dynamic multi-pane demo showing pane creation, focus cycling, and pane closing" width="800" height="450"/>

### Multi-pane file operations

<img src="examples/vhs/generated/multi_pane_file_ops.gif" alt="Multi-pane file operations demo copying marked files across three panes" width="800" height="450"/>

<details>
<summary><strong>More demos</strong></summary>

### Preview and inline editor
<img src="examples/vhs/generated/preview.gif" alt="Preview and inline editor demo" width="800" height="450"/>

### Picture preview
<img src="examples/vhs/generated/picture_preview.gif" alt="Picture preview demo" width="800" height="450"/>

### Basic navigation
<img src="examples/vhs/generated/basic.gif" alt="Basic navigation demo" width="800" height="450"/>

### Search
<img src="examples/vhs/generated/search.gif" alt="Incremental search demo" width="800" height="450"/>

### Sort modes
<img src="examples/vhs/generated/sort.gif" alt="Sort modes demo" width="800" height="450"/>

### Extension filter
<img src="examples/vhs/generated/filter.gif" alt="Extension filter demo" width="800" height="450"/>

### File operations
<img src="examples/vhs/generated/file_ops.gif" alt="File operations demo" width="800" height="450"/>

### Theme switcher
<img src="examples/vhs/generated/theme_switcher.gif" alt="Theme switcher demo" width="800" height="450"/>

### Pane toggle
<img src="examples/vhs/generated/pane_toggle.gif" alt="Pane toggle demo" width="800" height="450"/>

### Dual-pane widget
<img src="examples/vhs/generated/dual_pane.gif" alt="Dual-pane widget demo" width="800" height="450"/>

### Options panel
<img src="examples/vhs/generated/options.gif" alt="Options panel demo" width="800" height="450"/>

### Editor picker
<img src="examples/vhs/generated/editor_picker.gif" alt="Editor picker demo" width="800" height="450"/>

### Create entries
<img src="examples/vhs/generated/create_entries.gif" alt="Create entries demo" width="800" height="450"/>

</details>

## Features

- **Dynamic multi-pane layout** — `Tab`/`Shift+Tab` cycle focus, `Ctrl+T` opens a pane, `Ctrl+W` closes one, and `w` toggles focused single-pane mode.
- **File operations** — mark multiple entries, copy, move, recursively delete, create, rename, and confirm overwrites.
- **Fast navigation** — arrows, Vim keys, paging, top/bottom jumps, incremental search, extension filters, and three sort modes.
- **Preview and editing** — text with line numbers, images, binary hex dumps, directory summaries, external editors, and a built-in editor.
- **Terminal-native images** — Kitty, Sixel, iTerm2, and halfblock fallback through [`ratatui-image`](https://github.com/ratatui/ratatui-image).
- **43 themes** — live theme picker plus fully customizable `Theme` values.
- **Persistent CLI state** — pane directories, active pane, theme, sort mode, visibility settings, editor, and cd-on-exit.
- **Cross-platform shell integration** — bash, zsh, fish, PowerShell, and Nushell on macOS, Linux, and Windows.
- **Three library tiers** — `FileExplorer`, `DualPane`, or full `App`.

## Installation

### Library

```toml
[dependencies]
tui-file-explorer = "2"
ratatui = "0.30"
```

Disable the CLI feature when only the library is needed:

```toml
[dependencies]
tui-file-explorer = { version = "2", default-features = false }
```

### CLI

```bash
cargo install tui-file-explorer
tfe
```

## Library quick start

| API | Use when | Provides |
|---|---|---|
| `FileExplorer` | You want one browser widget | Navigation, filtering, sorting, marks, create, rename |
| `DualPane` | You want a focused two-pane widget | Two independent `FileExplorer`s, focus switching, single/dual toggle |
| `App` + `draw` | You want the complete `tfe` experience | Arbitrary panes, file operations, panels, preview, editors, persistence-ready state |

### Full app

```rust,no_run
use std::path::PathBuf;
use tui_file_explorer::{draw, App, AppOptions};

let mut app = App::new(AppOptions {
    pane_dirs: vec![
        std::env::current_dir()?,
        PathBuf::from("/tmp"),
    ],
    ..AppOptions::default()
});

loop {
    terminal.draw(|frame| draw(&mut app, frame))?;
    if app.handle_event()? {
        break;
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

If your application already reads terminal events, call `App::handle_key` or `App::handle_raw_event` instead of `handle_event`.

### Single-pane widget

```rust,no_run
use tui_file_explorer::{render_themed, ExplorerOutcome, FileExplorer, SortMode, Theme};

let mut explorer = FileExplorer::builder(std::env::current_dir()?)
    .allow_extension("rs")
    .allow_extension("toml")
    .show_hidden(false)
    .show_sizes(true)
    .sort_mode(SortMode::Name)
    .build();

terminal.draw(|frame| {
    render_themed(&mut explorer, frame, frame.area(), &Theme::default());
})?;

match explorer.handle_key(key) {
    ExplorerOutcome::Selected(path) => println!("{}", path.display()),
    ExplorerOutcome::Dismissed => {}
    _ => {}
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Dual-pane widget

```rust,no_run
use std::path::PathBuf;
use tui_file_explorer::{render_dual_pane_themed, DualPane, DualPaneOutcome, Theme};

let mut dual = DualPane::builder(std::env::current_dir()?)
    .right_dir(PathBuf::from("/tmp"))
    .show_hidden(false)
    .build();

terminal.draw(|frame| {
    render_dual_pane_themed(&mut dual, frame, frame.area(), &Theme::default());
})?;

match dual.handle_key(key) {
    DualPaneOutcome::Selected(path) => println!("{}", path.display()),
    DualPaneOutcome::Dismissed => {}
    _ => {}
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

See [`examples/`](examples/) and the complete API on [docs.rs](https://docs.rs/tui-file-explorer).

## Keyboard controls

| Area | Keys | Action |
|---|---|---|
| Navigation | `↑`/`k`, `↓`/`j` | Move cursor |
| Navigation | `←`/`h`/`Backspace` | Ascend to parent |
| Navigation | `→` | Descend into directory; move down when entry is a file |
| Navigation | `Enter`/`l` | Descend or confirm file |
| Navigation | `PgUp`/`PgDn`, `g`/`G` | Page, top, bottom |
| Panes | `Tab`/`Shift+Tab` | Next/previous pane |
| Panes | `Ctrl+T`/`Ctrl+W` | Open/close pane |
| Panes | `w` | Toggle focused single-pane/multi-pane layout |
| Search | `/`, `Backspace`, `Esc` | Start, edit, or cancel incremental search |
| View | `s`, `.`, `z` | Cycle sort, toggle hidden entries, toggle sizes |
| Selection | `Space` | Mark/unmark current entry |
| File operations | `x`, `p`, `d` | Cut, paste, delete |
| File operations | `n`, `N`, `r` | New directory, new file, rename |
| Preview | `P`, `Ctrl+J`/`Ctrl+K` | Toggle and scroll preview |
| Editors | `e`, `i`, `Shift+E` | External editor, inline editor, editor picker |
| Panels | `T`, `Shift+O` | Theme picker, options panel |
| Theme | `t`, `[` | Next/previous theme |
| Shell | `Shift+C` | Toggle cd-on-exit |
| Exit | `Esc`/`q`, `Ctrl+C` | Dismiss or exit |

Search and text-input modes consume printable keys before global shortcuts. Delete and overwrite operations require confirmation.

## CLI usage

```text
tfe [OPTIONS] [PATH]
```

Common options:

| Option | Description |
|---|---|
| `-e, --ext <EXT>` | Restrict selectable files by extension; repeatable |
| `-H, --hidden` | Show dotfiles on startup |
| `-t, --theme <THEME>` | Select persisted or named theme |
| `--list-themes` | Print all theme names |
| `--show-themes` | Open theme picker at startup |
| `--single-pane` | Start with only focused pane visible |
| `--editor <EDITOR>` | Select external editor or command |
| `--cd` / `--no-cd` | Enable/disable persisted cd-on-exit |
| `--init <SHELL>` | Install shell wrapper |
| `--print-dir` | Print selected file's parent directory |
| `-0, --null` | Use NUL-delimited output |
| `--info` / `--doctor` | Show environment info or run diagnostics |
| `-v, --verbose` | Enable startup logs, log file, and debug panel |

Run `tfe --help` for full option documentation.

### Shell integration

`tfe` renders on stderr and reserves stdout for selected paths, cd-on-exit, and one-time shell setup. Enable directory changes and install the wrapper once:

```bash
tfe --cd
tfe --init zsh       # or bash, fish, powershell, nushell
```

Source the file reported by `--init`, or open a new terminal. Afterwards, dismissing with `Esc`/`q` changes the parent shell to the active pane's directory.

Use `command tfe` to bypass the wrapper when piping output:

```bash
command tfe -e rs | xargs -r nvim
command tfe -0 | xargs -0 wc -l
```

Exit codes:

| Code | Meaning |
|---|---|
| `0` | Selection or successful cd-on-exit output |
| `1` | Dismissed without cd-on-exit output |
| `2` | Invalid arguments or I/O failure |

## Main capabilities

### Multi-pane workflow

A fresh CLI session starts with two panes; later sessions restore the persisted pane layout. Each pane keeps its own directory, cursor, marks, search, sort, and size-display state. New panes clone the active pane's directory and display settings; at least one pane always remains open.

Typical source-to-destination copy:

1. Mark one or more entries with `Space`.
2. Move to another pane with `Tab`, or create one with `Ctrl+T`.
3. Navigate to destination.
4. Press `p` to copy. Use `x` before switching panes for a move.

### Search, filtering, and sorting

- `/` starts case-insensitive incremental filename search.
- `Esc` clears active search before dismissing explorer.
- Extension filters dim non-matching files while leaving directories navigable.
- `s` cycles `Name → Size ↓ → Extension`.
- Directories remain grouped before files.

### Preview and editors

`P` opens preview for text, supported images (PNG, JPEG, GIF, BMP, WebP), binary files, and directories. `i` opens built-in text editor; `Ctrl+S` saves and `Esc` returns. `e` launches configured external editor.

### Themes

Use `t`/`[` to cycle or `T` to open picker. Forty-three presets include Catppuccin, Dracula, Nord, Solarized, Gruvbox, Tokyo Night, Kanagawa, Rosé Pine, Flexoki, and others. Customize every color with `Theme` builder methods.

## Examples

| Example | Command | Purpose |
|---|---|---|
| Basic widget | `cargo run --example basic` | Navigation, search, sorting, extension filters |
| Dual pane | `cargo run --example dual_pane -- /tmp` | `DualPane` library API and independent directories |
| Theme switcher | `cargo run --example theme_switcher` | Live preset switching |
| Options | `cargo run --example options` | Options and editor panels |
| Editor picker | `cargo run --example editor_picker` | Terminal and GUI editor selection |
| Open file | `cargo run --example open_file -- "code --wait"` | Suspend TUI, edit, resume |
| Create entries | `cargo run --example create_entries` | Create directories/files and rename entries |
| Full app | `cargo run --example full` | Complete `App` integration |

## Public API

Main re-exports:

| Group | Items |
|---|---|
| Single pane | `FileExplorer`, `FileExplorerBuilder`, `ExplorerOutcome`, `FsEntry`, `SortMode`, `render`, `render_themed` |
| Dual pane | `DualPane`, `DualPaneBuilder`, `DualPaneActive`, `DualPaneOutcome`, `render_dual_pane`, `render_dual_pane_themed` |
| Full app | `App`, `AppOptions`, `ClipOp`, `ClipboardItem`, `Modal`, `Editor`, `Snackbar`, `CopyProgress`, `draw` |
| Preview/editor | `PreviewState`, `PreviewContent`, `InlineEditor`, `EditorAction`, `render_preview`, `render_inline_editor` |
| Theme/utilities | `Theme`, `entry_icon`, `fmt_size`, `copy_dir_all`, `AppState`, `load_state`, `save_state` |

Modules and detailed method docs: [docs.rs/tui-file-explorer](https://docs.rs/tui-file-explorer).

## Demo recordings

Install [VHS](https://github.com/charmbracelet/vhs), then run:

```bash
just vhs multi_pane  # one tape
just vhs-all         # every tape
```

Tapes live in [`examples/vhs/`](examples/vhs/). Generated 1600×900 GIFs live in [`examples/vhs/generated/`](examples/vhs/generated/) and are tracked with Git LFS.

| Tape | Demonstrates |
|---|---|
| `multi_pane` | Dynamic pane creation/closing, focus cycling, single-pane focus |
| `multi_pane_file_ops` | Copying marked files between three panes |
| `basic`, `search`, `sort`, `filter` | Core navigation and discovery |
| `file_ops`, `create_entries` | Copy/move/delete and create/rename workflows |
| `preview`, `picture_preview` | Text, binary, directory, and image preview |
| `theme_switcher`, `options`, `editor_picker` | Themes and configuration panels |
| `pane_toggle`, `dual_pane` | CLI layout and `DualPane` library widget |

## Troubleshooting

```bash
command tfe --info     # environment, paths, terminal, shell
command tfe --doctor   # pass/warn/fail diagnostics with fixes
command tfe --verbose  # debug panel and $TMPDIR/tfe-debug.log
```

Use `command tfe` when diagnostics or pipelines must bypass installed shell wrapper.

## Development

Requires Rust 1.88 or newer. Optional tooling: [`just`](https://github.com/casey/just), [`git-cliff`](https://github.com/orhun/git-cliff), and [`vhs`](https://github.com/charmbracelet/vhs).

```bash
just check-all   # format check, clippy with denied warnings, tests
just test-all    # all features and targets
just doc         # build and open API docs
just --list      # all project tasks
```

Contributions are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

Built for the [Ratatui](https://github.com/ratatui/ratatui) ecosystem.
