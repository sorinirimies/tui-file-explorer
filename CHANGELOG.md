# Changelog
All notable changes to this project will be documented in this file.

## [0.1.0] - 2025-01-01

### Features
- Self-contained file-browser widget for Ratatui
- Keyboard-driven navigation (arrow keys, vim keys, PgUp/PgDn, Home/End)
- Optional extension filter — only matching files are selectable
- Toggle hidden (dot-file) visibility with `.`
- Directories-first listing with case-insensitive alphabetical sorting
- Scrollable list with position indicator
- Header with left-truncated current path display
- Footer with key hints and filter/status info
- Zero application-specific dependencies (only `ratatui` + `crossterm`)
- 17 unit tests + 3 doc-tests
