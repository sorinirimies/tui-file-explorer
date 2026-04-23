# Changelog
All notable changes to this project will be documented in this file.

## [1.0.5] - 2026-04-21

### Miscellaneous
- Bump version to 1.0.5

## [1.0.4] - 2026-04-21

### Bug Fixes
- Repair version-bump and release-workflow trigger pipeline
- Make `just release <version>` fully non-interactive
- Preserve last_dir_right when exiting in single-pane mode
- Prevent crashes at navigation boundaries
- Arrow keys scroll the list; only Enter/l descend or confirm
- Render TUI on /dev/tty so shell wrapper $() capture works
- Cross-platform TUI rendering and shell integration
- Silence clippy empty_line_after_doc_comments and derivable_impls
- Detect hand-written tfe wrappers to prevent duplicate installs
- Resolve zsh rc file from ZDOTDIR, .zshrc, and .zshenv fallback
- Default cd_on_exit to true now that wrapper is auto-installed
- Make auto_install_with shell-independent for hermetic tests
- Enter/l on a file opens editor instead of exiting TUI when editor is configured
- Cross-platform editor launch (Windows .cmd shims, Custom arg splitting)
- Rename options panel editor label from 'cycle' to 'open with'
- Multi-selection yank — y/x now copies/cuts all marked entries
- Yank checks both panes for marks — handles dual-pane mark-then-tab-then-y workflow
- Paste feedback and symlink handling
- Clippy — remove unused Color import and redundant f64 cast
- Detect and correct --editor <file> misparse
- Use sort_by_key to satisfy clippy 1.95 unnecessary_sort_by lint

### Documentation
- Overhaul README, add VHS demo GIFs via Git LFS
- Mandate commit + release after every implementation and test cycle
- Update README for v0.2.0 DualPane API
- Update README key bindings for Miller-columns navigation
- Update README/lib docs and add dual_pane.tape
- Add agents.md and expand rules.md with Editor and hermetic-test patterns
- Document three API tiers in README — widget, dual-pane, full app

### Features
- Incremental search, sort modes, and expanded icon set
- Add file_ops demo GIF, tape, and README wiring
- Add DualPane widget to the library
- Miller-columns navigation and split action bar
- Cd to browsed directory on dismiss
- --init <shell> installs cd-on-exit wrapper automatically
- Make cd-on-exit opt-in via --cd / --no-cd flags
- Editor launch via e key (v0.3.7)
- Auto-install shell wrapper on first run
- Show version label in header bottom-right of active pane
- Copy/move progress overlay using tui-slider
- Tfe <file> opens directly in configured editor

### Miscellaneous
- Bump version to 0.1.2
- Bump version to 0.1.2
- Bump version to 0.1.2
- Bump version to 0.1.3
- Bump version to 0.1.4
- Bump version to 0.1.5
- Bump version to 0.1.6
- Bump version to 0.1.7
- Bump version to 0.1.8
- Bump version to 0.1.9
- Bump version to 0.1.10
- Bump version to 0.2.0
- Bump version to 0.2.1
- Bump version to 0.2.2
- Bump version to 0.2.3
- Bump version to 0.3.0
- Bump version to 0.3.1
- Bump version to 0.3.2
- Bump version to 0.3.3
- Bump version to 0.3.4
- Bump version to 0.3.5
- Cleanup dead code, add 310 new tests, bump to v0.3.6
- Bump version to 0.3.8
- Bump version to 0.3.9
- Bump version to 0.4.0
- Bump version to 0.4.1
- Bump version to 0.4.2
- Bump version to 0.4.3
- Bump version to 0.4.4
- Bump version to 0.4.5
- Bump version to 0.4.6
- Bump version to 0.4.7
- Bump version to 0.4.8
- Bump version to 0.5.0
- Bump version to 0.5.1
- Bump version to 0.6.0
- Bump version to 0.6.1
- Bump version to 0.6.2
- Bump version to 0.6.3
- Bump version to 0.6.4
- Bump version to 0.6.5
- Bump version to 0.6.6
- Bump version to 0.6.7
- Bump version to 0.6.8
- Bump version to 0.6.9
- Bump version to 0.7.0
- Bump version to 0.7.1
- Bump version to 0.7.2
- Bump version to 0.7.3
- Bump version to 0.7.4
- Bump version to 0.7.5
- Bump version to 0.8.0
- Bump version to 0.8.1
- Bump version to 0.8.2
- Bump version to 0.8.3
- Bump version to 0.8.4
- Bump version to 0.8.6
- Bump version to 0.8.7
- Bump version to 0.8.8, add weekly deps-update CI workflows
- Bump version to 0.9.0
- Bump version to 0.9.1
- Bump version to 0.9.2
- Bump version to 0.9.3
- Bump version to 0.9.4
- Bump version to 0.9.5
- Bump version to 0.9.6
- Bump version to 0.9.8
- Bump version to 1.0.0
- Bump version to 1.0.1
- Bump version to 1.0.2
- Bump version to 1.0.4

### Refactor
- Introduce AppOptions struct to fix too_many_arguments lint
- Remove 'full' feature — library always exposes complete API

### Style
- Rewrite all VHS tapes to minimal single-line comment style with Catppuccin Mocha theme

