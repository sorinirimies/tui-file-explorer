# Changelog
All notable changes to this project will be documented in this file.

## [0.9.5] - 2026-03-30

### Miscellaneous
- Bump version to 0.9.5

## [0.9.4] - 2026-03-30

### Miscellaneous
- Bump version to 0.9.4

## [0.9.3] - 2026-03-30

### Miscellaneous
- Bump version to 0.9.3

## [0.9.2] - 2026-03-30

### Miscellaneous
- Bump version to 0.9.2

## [0.9.1] - 2026-03-25

### Bug Fixes
- Detect and correct --editor <file> misparse

### Miscellaneous
- Bump version to 0.9.1

## [0.9.0] - 2026-03-25

### Documentation
- Document three API tiers in README — widget, dual-pane, full app

### Features
- Tfe <file> opens directly in configured editor

### Miscellaneous
- Bump version to 0.9.0

## [0.8.8] - 2026-03-24

### Miscellaneous
- Bump version to 0.8.8, add weekly deps-update CI workflows

## [0.8.7] - 2026-03-24

### Miscellaneous
- Bump version to 0.8.7

## [0.8.6] - 2026-03-24

### Miscellaneous
- Bump version to 0.8.6

### Refactor
- Remove 'full' feature — library always exposes complete API

## [0.8.5] - 2026-03-24

### Bug Fixes
- Paste feedback and symlink handling
- Clippy — remove unused Color import and redundant f64 cast

### Features
- Copy/move progress overlay using tui-slider

## [0.8.4] - 2026-03-24

### Bug Fixes
- Yank checks both panes for marks — handles dual-pane mark-then-tab-then-y workflow

### Miscellaneous
- Bump version to 0.8.4

## [0.8.3] - 2026-03-24

### Bug Fixes
- Multi-selection yank — y/x now copies/cuts all marked entries

### Miscellaneous
- Bump version to 0.8.3

## [0.8.2] - 2026-03-23

### Miscellaneous
- Bump version to 0.8.2

## [0.8.1] - 2026-03-23

### Miscellaneous
- Bump version to 0.8.1

## [0.8.0] - 2026-03-23

### Miscellaneous
- Bump version to 0.8.0

## [0.7.5] - 2026-03-23

### Miscellaneous
- Bump version to 0.7.5

## [0.7.4] - 2026-03-23

### Miscellaneous
- Bump version to 0.7.4

## [0.7.3] - 2026-03-23

### Miscellaneous
- Bump version to 0.7.3

## [0.7.2] - 2026-03-23

### Miscellaneous
- Bump version to 0.7.2

## [0.7.1] - 2026-03-20

### Miscellaneous
- Bump version to 0.7.1

## [0.7.0] - 2026-03-19

### Miscellaneous
- Bump version to 0.7.0

## [0.6.9] - 2026-03-19

### Miscellaneous
- Bump version to 0.6.9

## [0.6.8] - 2026-03-19

### Miscellaneous
- Bump version to 0.6.8

## [0.6.7] - 2026-03-19

### Miscellaneous
- Bump version to 0.6.7

## [0.6.6] - 2026-03-17

### Miscellaneous
- Bump version to 0.6.6

## [0.6.5] - 2026-03-17

### Miscellaneous
- Bump version to 0.6.5

## [0.6.4] - 2026-03-17

### Miscellaneous
- Bump version to 0.6.4

## [0.6.3] - 2026-03-17

### Miscellaneous
- Bump version to 0.6.3

## [0.6.2] - 2026-03-17

### Miscellaneous
- Bump version to 0.6.2

## [0.6.1] - 2026-03-13

### Miscellaneous
- Bump version to 0.6.1

## [0.6.0] - 2026-03-13

### Miscellaneous
- Bump version to 0.6.0

## [0.5.1] - 2026-03-12

### Miscellaneous
- Bump version to 0.5.1

## [0.5.0] - 2026-03-11

### Miscellaneous
- Bump version to 0.5.0

## [0.4.8] - 2026-03-10

### Bug Fixes
- Rename options panel editor label from 'cycle' to 'open with'

### Miscellaneous
- Bump version to 0.4.8

## [0.4.7] - 2026-03-10

### Miscellaneous
- Bump version to 0.4.7

## [0.4.6] - 2026-03-06

### Bug Fixes
- Cross-platform editor launch (Windows .cmd shims, Custom arg splitting)

### Miscellaneous
- Bump version to 0.4.6

## [0.4.5] - 2026-03-06

### Bug Fixes
- Enter/l on a file opens editor instead of exiting TUI when editor is configured

### Miscellaneous
- Bump version to 0.4.5

## [0.4.4] - 2026-03-06

### Documentation
- Add agents.md and expand rules.md with Editor and hermetic-test patterns

### Miscellaneous
- Bump version to 0.4.4

## [0.4.3] - 2026-03-06

### Bug Fixes
- Make auto_install_with shell-independent for hermetic tests

### Miscellaneous
- Bump version to 0.4.3

## [0.4.2] - 2026-03-06

### Features
- Show version label in header bottom-right of active pane

### Miscellaneous
- Bump version to 0.4.2

## [0.4.1] - 2026-03-06

### Bug Fixes
- Default cd_on_exit to true now that wrapper is auto-installed

### Miscellaneous
- Bump version to 0.4.1

## [0.4.0] - 2026-03-06

### Features
- Auto-install shell wrapper on first run

### Miscellaneous
- Bump version to 0.4.0

## [0.3.9] - 2026-03-06

### Bug Fixes
- Detect hand-written tfe wrappers to prevent duplicate installs
- Resolve zsh rc file from ZDOTDIR, .zshrc, and .zshenv fallback

### Miscellaneous
- Bump version to 0.3.9

## [0.3.8] - 2026-03-06

### Bug Fixes
- Silence clippy empty_line_after_doc_comments and derivable_impls

### Documentation
- Update README/lib docs and add dual_pane.tape

### Features
- Editor launch via e key (v0.3.7)

### Miscellaneous
- Bump version to 0.3.8

## [0.3.6] - 2026-03-05

### Miscellaneous
- Cleanup dead code, add 310 new tests, bump to v0.3.6

## [0.3.5] - 2026-03-05

### Features
- Make cd-on-exit opt-in via --cd / --no-cd flags

### Miscellaneous
- Bump version to 0.3.5

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

