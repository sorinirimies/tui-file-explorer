# Changelog
All notable changes to this project will be documented in this file.

## [0.3.7] - 2026-03-06

### Features
- Add `Editor` enum (`None`, `Helix`, `Neovim`, `Vim`, `Nano`, `Micro`, `Custom(String)`) to control which editor is launched when pressing `e` on a file
- `Editor::None` is the default — the feature is fully opt-in; pressing `e` is a silent no-op until an editor is selected
- Press `e` in the options panel (`O`) to cycle through editors: None → Helix → Neovim → Vim → Nano → Micro → None → …
- Press `e` on a file in the explorer to suspend the TUI, open the file in the configured editor, then automatically restore the TUI and reload both panes
- Add `--editor <EDITOR>` CLI flag to override the persisted editor for a session (e.g. `tfe --editor nvim`, `tfe --editor "code --wait"`)
- Persist the selected editor in the state file (`editor=helix`, `editor=nvim`, `editor=custom:code`, etc.)
- Options panel grows a new **Editor** section showing the active editor; label is dim when `none`, green+bold when an editor is selected
- Action bar gains `e edit` hint between `d del` and `[ / t theme`

### Testing
- 15 new unit tests covering `Editor` enum methods (`binary`, `label`, `cycle`, `to_key`, `from_key`), full cycle loop invariant, `AppOptions`/`App` default fields, and persistence round-trips

## [0.3.6] - 2026-03-05

### Bug Fixes
- Remove dead `Theme::new()` (identical to `Default::default()`, never called)
- Remove stray `theme_switcher.rs` from repository root (already present in `examples/`)
- Extract `render_nav_hints_spans` helper for testability, mirroring `render_action_bar_spans`

### Testing
- Add 310 new unit tests across all modules (475 total, up from 165)
- `types` — full coverage of `SortMode::next` cycle, `FsEntry` construction, `ExplorerOutcome` variants
- `palette` — palette constants match `Theme::default()`, all builder setters, `all_presets` catalogue invariants
- `explorer` — extended `entry_icon` coverage (22 extensions), `fmt_size` full boundary suite, `navigate_to` with `&str`/`&Path`, `is_searching` accessor, `status` cleared on reload, `load_entries` directly
- `dual_pane` — `DualPaneActive::default()`, focus round-trips, inactive accessor, `DualPaneOutcome` variants, `active_mut` for right pane, `toggle_single_pane` idempotency
- `persistence` — `sort_mode_to_key`/`sort_mode_from_key` internal helpers, `AppState::default` all-None invariant
- `ui` — `render_nav_hints_spans` content, bold/accent/dim style assertions, stable span count
- `app` — Tab pane switching, `themes` list non-empty, `theme_idx` from options, next/prev theme bounds, `do_paste` success status, `active_pane_mut`, `AppOptions::default` fields

## [0.3.5] - 2026-03-05

### Features
- Make cd-on-exit opt-in via --cd / --no-cd flags

## [0.3.4] - 2026-03-05

### Bug Fixes
- Cross-platform TUI rendering and shell integration

### Miscellaneous
- Bump version to 0.3.4

## [0.3.3] - 2026-03-05

### Bug Fixes
- Render TUI on /dev/tty so shell wrapper $() capture works

### Miscellaneous
- Bump version to 0.3.3

## [0.3.2] - 2026-03-05

### Features
- --init <shell> installs cd-on-exit wrapper automatically

### Miscellaneous
- Bump version to 0.3.2

## [0.3.1] - 2026-03-05

### Features
- Cd to browsed directory on dismiss

### Miscellaneous
- Bump version to 0.3.1

## [0.3.0] - 2026-03-05

### Documentation
- Update README key bindings for Miller-columns navigation

### Features
- Miller-columns navigation and split action bar

### Miscellaneous
- Bump version to 0.3.0

## [0.2.3] - 2026-03-05

### Bug Fixes
- Arrow keys scroll the list; only Enter/l descend or confirm

### Miscellaneous
- Bump version to 0.2.3

## [0.2.2] - 2026-03-05

### Bug Fixes
- Prevent crashes at navigation boundaries

### Documentation
- Update README for v0.2.0 DualPane API

### Miscellaneous
- Bump version to 0.2.2

## [0.2.1] - 2026-03-05

### Features
- Add DualPane widget to the library

### Miscellaneous
- Bump version to 0.2.1

## [0.2.0] - 2026-03-05

### Miscellaneous
- Bump version to 0.2.0

## [0.1.10] - 2026-03-04

### Documentation
- Mandate commit + release after every implementation and test cycle

### Miscellaneous
- Bump version to 0.1.10

## [0.1.9] - 2026-03-04

### Bug Fixes
- Preserve last_dir_right when exiting in single-pane mode

### Miscellaneous
- Bump version to 0.1.9

## [0.1.8] - 2026-03-04

### Miscellaneous
- Bump version to 0.1.8

### Refactor
- Introduce AppOptions struct to fix too_many_arguments lint

## [0.1.7] - 2026-02-28

### Miscellaneous
- Bump version to 0.1.7

## [0.1.6] - 2026-02-28

### Miscellaneous
- Bump version to 0.1.6

## [0.1.5] - 2026-02-26

### Miscellaneous
- Bump version to 0.1.5

## [0.1.4] - 2026-02-25

### Documentation
- Overhaul README, add VHS demo GIFs via Git LFS

### Features
- Incremental search, sort modes, and expanded icon set
- Add file_ops demo GIF, tape, and README wiring

### Miscellaneous
- Bump version to 0.1.4

### Style
- Rewrite all VHS tapes to minimal single-line comment style with Catppuccin Mocha theme

## [0.1.3] - 2026-02-25

### Bug Fixes
- Repair version-bump and release-workflow trigger pipeline
- Make `just release <version>` fully non-interactive

### Miscellaneous
- Bump version to 0.1.2
- Bump version to 0.1.2
- Bump version to 0.1.3

## [0.1.2] - 2026-02-25

### Miscellaneous
- Bump version to 0.1.2

