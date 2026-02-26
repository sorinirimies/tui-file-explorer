# rules.md — Development Guidelines for `tui-file-explorer`

This file is the single source of truth for conventions, patterns, and
decisions made in this codebase. Read it before opening a PR or adding
a feature.

---

## 1. Project Layout

```
tui-file-explorer/
├── src/
│   ├── lib.rs        # Crate root — docs, module declarations, public re-exports only
│   ├── types.rs      # Plain data types: FsEntry, ExplorerOutcome, SortMode
│   ├── palette.rs    # Colour constants + Theme struct + all named presets
│   ├── explorer.rs   # FileExplorer state machine + filesystem helpers + unit tests
│   ├── render.rs     # All ratatui Frame rendering (render / render_themed)
│   ├── main.rs       # tfe binary: CLI (Cli struct), run(), run_loop() entry-point only
│   ├── app.rs        # tfe binary: App state, Pane, ClipOp, ClipboardItem, Modal, handle_event
│   ├── ui.rs         # tfe binary: draw(), render_theme_panel(), render_action_bar(), render_modal()
│   └── fs.rs         # tfe binary: copy_dir_all(), emit_path(), resolve_output_path()
├── scripts/
│   ├── bump_version.sh   # Interactive version bump — runs checks before tagging
│   └── check_publish.sh  # Pre-publish gate — fmt, clippy, tests, doc, dry-run
├── examples/
│   └── vhs/              # VHS tape files + generated demo GIFs (tracked in Git LFS)
├── .github/workflows/
│   ├── ci.yml        # fmt + clippy + doc + test on every PR / main push
│   └── release.yml   # Full release pipeline on v* tags → crates.io
├── justfile          # All developer tasks (see: just --list)
└── rules.md          # This file
```

**Rules:**
- `lib.rs` contains only `pub mod`, `pub use`, and crate-level doc comments.
  No logic lives there.
- Each module has one clear responsibility. If a module starts doing two
  things, split it.
- New public types go in `types.rs`; new colour tokens go in `palette.rs`.
- Binary-only code is split across four modules: `main.rs` (CLI + entry-point),
  `app.rs` (App state + event handling), `ui.rs` (TUI rendering), `fs.rs`
  (filesystem helpers). `persistence.rs` owns state serialisation.

---

## 2. Code Style

### Formatting
- `rustfmt` is mandatory. Config lives in `rustfmt.toml` (`max_width = 100`).
- Run before every commit: `just fmt` (or `cargo fmt`).
- CI rejects unformatted code (`just fmt-check`).

### Naming
| Thing | Convention | Example |
|---|---|---|
| Types / Traits | `UpperCamelCase` | `FileExplorer`, `ExplorerOutcome` |
| Functions / methods | `snake_case` | `handle_key`, `load_entries` |
| Constants | `SCREAMING_SNAKE_CASE` | `C_BRAND`, `C_ACCENT` |
| Modules | `snake_case` | `explorer`, `render` |
| Enum variants | `UpperCamelCase` | `Selected(PathBuf)`, `Dismissed` |

### Clippy
- Target: **zero warnings** with `-D warnings`.
- Run: `just clippy` (or `cargo clippy -- -D warnings`).
- Never suppress a lint with `#[allow(...)]` without a comment explaining why.

### Comments
- Use `//` for implementation notes.
- Use `//!` at the top of each module file for the module doc.
- Use `///` on every `pub` item — types, functions, fields, variants.
- Section dividers use this exact style (80 chars total):
  ```rust
  // ── Section title ──────────────────────────────────────────────────────────
  ```

---

## 3. The Golden Rule: Tests Come With the Feature

> **Every feature, fix, or refactor must ship with tests in the same commit.**

This is non-negotiable. A PR that adds behaviour without tests will not be
merged. A PR that fixes a bug without a regression test will not be merged.

### What "shipped together" means
1. Write the feature code.
2. Write the tests that cover it.
3. `just check-all` must pass before committing.
4. Both land in the same commit (or the same PR if squash-merged).

### Why
- Tests written after the fact are weaker — the author already knows the code
  works and tends to write confirming tests, not falsifying ones.
- Tests written alongside the code force the author to think about edge cases
  during design, which leads to better APIs.
- A green CI history is only meaningful if every feature has always had
  coverage.

---

## 4. Testing

### What to test
- Every public method on `FileExplorer` must have at least one test.
- Every `ExplorerOutcome` variant must be reachable through a test.
- Every persistence function (`save_theme_to`, `load_theme_from`) must be
  tested in isolation using explicit paths — never by mutating `$HOME` or
  `$XDG_CONFIG_HOME`.
- Edge cases: empty directory, cursor at boundaries, filesystem root ascent,
  missing/empty config file, whitespace-only config file.

### Test location
| Code under test | Where tests live |
|---|---|
| `FileExplorer`, `load_entries`, `entry_icon`, `fmt_size` | `mod tests` at the bottom of `explorer.rs` |
| `save_state_to`, `load_state_from`, `resolve_theme_idx` | `mod tests` at the bottom of `persistence.rs` |
| `App`, `Pane`, `ClipboardItem`, `Modal`, file operations | `mod tests` at the bottom of `app.rs` |
| `render_action_bar_spans` | `mod tests` at the bottom of `ui.rs` |
| `copy_dir_all`, `resolve_output_path` | `mod tests` at the bottom of `fs.rs` |
| New module added in future | `mod tests` at the bottom of that module |

Tests always live in a `#[cfg(test)] mod tests { ... }` block at the
**bottom** of the file that owns the code under test.

### Binary testing conventions
- Do **not** test `run()`, `run_loop()`, or `draw()` — these require a real
  terminal and are integration-level concerns covered by VHS recordings.
- Do **not** manipulate environment variables (`$HOME`, `$XDG_CONFIG_HOME`)
  inside tests — this is not thread-safe. Instead, pass explicit `&Path`
  arguments to the functions under test (`save_state_to`, `load_state_from`).
- `App::handle_event` is not directly testable without an event queue.
  Trust that it is covered by: (a) testing the functions it delegates to
  (`yank`, `paste`, `do_paste`, `confirm_delete`, etc.), and (b) VHS smoke
  tests for the full binary.
- `resolve_theme_idx` is a pure function — test it exhaustively: known names,
  unknown names, case variants, hyphen normalisation.
- `render_action_bar_spans` returns a plain `Vec<Span>` and is fully testable
  without a real terminal — test it for content, colour, and modifier correctness.

### Fixtures and helpers
- Use `tempfile::tempdir()` for all filesystem tests.
- A canonical fixture helper `fn temp_dir_with_files() -> TempDir` lives in
  `explorer.rs`. Add to it rather than duplicating setup.
- Canonical fixture helpers `fn tmp_state_path()` and `fn tmp_theme_path()`
  live in `persistence.rs` tests. Use them for all persistence tests.
- A `fn make_app(dir: PathBuf) -> App` helper in `app.rs` tests constructs a
  minimal `App` with sensible defaults. Use it rather than calling `App::new`
  directly in each test.
- Never rely on the real filesystem layout in tests.

### Test naming
Follow `verb_condition_expectation`:
```rust
fn save_theme_to_creates_parent_directories()
fn load_theme_from_empty_file_returns_none()
fn resolve_theme_idx_normalises_hyphens_to_spaces()
fn move_down_clamps_at_last()
```

### Rules
- No `#[ignore]` tests. If a test is flaky, fix it or delete it.
- No `unwrap()` in test helpers that can reasonably fail — use `expect("reason")`.
- Do not share mutable state between tests. Each test is fully self-contained.

### Running
```bash
just test           # cargo test
just test-all       # cargo test --all-features --all-targets
```

---

## 5. Error Handling

- The library (`FileExplorer`) has **no `Result`-returning public API**.
  Methods are infallible. Errors degrade gracefully to empty entry lists or
  status-bar messages.
- Do not add `anyhow` or `thiserror` to `[dependencies]` — they are
  application concerns, not library concerns.
- `fs::read_dir` errors are silently swallowed into an empty `Vec`. If richer
  error reporting is ever needed, add an `ExplorerOutcome::Error(String)`
  variant rather than propagating `io::Error`.
- In the binary, persistence errors (`save_theme`, `load_saved_theme`) are
  silently ignored with `let _ = ...`. The app is fully usable without a
  config file and must never crash because one cannot be written (e.g.
  read-only filesystem).

---

## 6. Public API Contract

- **Backwards-compat is king.** This crate follows SemVer strictly.
  - Adding a public item → minor bump.
  - Removing or changing a public item → major bump.
- The public surface is intentionally narrow:
  - Types: `FileExplorer`, `FileExplorerBuilder`, `FsEntry`, `ExplorerOutcome`, `SortMode`
  - Functions: `render`, `render_themed`, `entry_icon`, `fmt_size`
  - Struct: `Theme` (with builder methods and `all_presets`)
- `pub(crate)` is used for anything shared between modules that is not part of
  the external API.
- Do not expose `scroll_offset` or `status` fields publicly — they are
  rendering implementation details.

---

## 7. Ratatui Patterns

- `render(explorer, frame, area)` and `render_themed(...)` are the only public
  rendering entry-points. They own the layout split; callers never pass
  pre-split areas.
- All widget construction is local to `render_*` helpers — no widgets are
  stored in `FileExplorer`.
- Scroll state is managed manually via `scroll_offset`. `ListState::select` is
  used only to drive ratatui's highlight — always set to the *visible* index
  (`cursor - scroll_offset`), not the absolute index.
- Avoid allocating inside the `draw` closure hot-path where possible.
  Prefer `format!` only when the string genuinely changes per frame.
- The palette in `palette.rs` is the single source of colour truth.
  Never hardcode `Color::Rgb(...)` anywhere except `palette.rs`.

---

## 8. Persistence

The `tfe` binary persists the selected theme across sessions.

### Storage
- Path: `$XDG_CONFIG_HOME/tfe/theme` (falls back to `$HOME/.config/tfe/theme`).
- Format: a single UTF-8 line containing the theme name, no trailing newline
  required (the loader trims whitespace).
- The file and its parent directory are created automatically on first save.

### Rules
- Persistence must **never** crash the app. All save/load errors are silently
  ignored (`let _ = save_theme_to(...)`).
- CLI `--theme` always overrides the persisted value. Priority order:
  1. Explicit `--theme <name>` flag
  2. Saved `~/.config/tfe/theme`
  3. Built-in default (`"default"`)
- State is saved once on clean exit (when the user confirms a selection or
  dismisses the explorer). The full `AppState` — theme, last directory, sort
  mode, hidden-file flag, single-pane flag — is written atomically in a single
  `fs::write` call.
- Functions that interact with the filesystem (`save_state_to`,
  `load_state_from`) accept an explicit `&Path` argument so they can be
  tested without touching the real config directory.
- The two higher-level wrappers (`save_state`, `load_state`) resolve the
  config path at call time and are used by production code only.

---

## 9. Dependencies

- **Minimise dependencies.** The only `[dependencies]` are `ratatui` and
  `crossterm`. Any new dependency requires justification in the PR description.
- `clap` is optional behind the `cli` feature (for the `tfe` binary).
- `tempfile` is the only allowed `[dev-dependencies]` entry.
- Pin to minor versions (`"0.30"`), not patch (`"0.30.0"`), to allow
  compatible updates without a PR.
- Run `just update` periodically. Review `Cargo.lock` diffs in PRs that touch
  dependencies.
- Do **not** add `dirs`, `directories`, `xdg`, or similar crates to resolve
  config paths — the two-line `$XDG_CONFIG_HOME` / `$HOME` fallback in
  `config_path()` is sufficient and keeps the dependency count at zero.

---

## 10. Features

```toml
[features]
default = ["cli"]
cli     = ["dep:clap"]     # enables the `tfe` binary
```

- Library users who only want the widget should disable defaults:
  ```toml
  tui-file-explorer = { version = "0.1", default-features = false }
  ```
- Do not add features that change public API behaviour — that is a SemVer
  concern, not a feature-flag concern.

---

## 11. Documentation

- Every `pub` item needs a `///` doc comment.
- Doc comments use imperative mood: *"Return the highlighted entry."* not
  *"Returns the highlighted entry."*
- Code examples in doc comments must compile. Use `no_run` only when a real
  terminal is required (i.e. ratatui draw closures).
- `lib.rs` must have a module-layout table so users can orient themselves
  without reading every file.
- Run `just doc` to verify docs build without warnings before a release.

---

## 12. Versioning & Release Workflow

This project uses [Conventional Commits](https://www.conventionalcommits.org/)
and [git-cliff](https://git-cliff.org/) for automated changelogs.

### Commit prefixes
| Prefix | Meaning |
|---|---|
| `feat:` | New user-visible capability → minor bump |
| `fix:` | Bug fix → patch bump |
| `doc:` | Documentation only |
| `refactor:` | Internal restructure, no behaviour change |
| `perf:` | Performance improvement |
| `chore:` | Tooling, CI, dependency updates |
| `test:` | Adding or fixing tests only |
| `BREAKING CHANGE:` | Footer or `!` suffix → major bump |

### Release steps (maintainers)
```bash
just bump 0.2.0       # bump, checks, commit, tag (interactive)
just release 0.2.0    # bump + push to GitHub (triggers CI + crates.io)
just release-all 0.2.0 # bump + push to GitHub and Gitea
```

The `release.yml` GitHub Actions workflow automatically:
1. Runs fmt / clippy / tests
2. Generates changelog via git-cliff
3. Creates a GitHub Release with release notes
4. Publishes to crates.io (requires `CRATES_IO_TOKEN` in repo secrets)

---

## 13. Git Hygiene

- Commits on `main` must pass `just check-all` (fmt + clippy + test).
- PRs should be squash-merged with a conventional commit message.
- Do not commit `target/`, `*.rs.bk`, `.DS_Store`, or `.zed/`.
- Tag format: `v<semver>` (e.g. `v0.2.0`). Tags are created by
  `bump_version.sh` and must never be moved after push.
- `Cargo.lock` is committed (marked binary in `.gitattributes`) so
  reproducible builds are guaranteed for contributors.
- Large binary assets (GIFs, PNGs, videos) are tracked via **Git LFS**.
  The `.gitattributes` file defines the tracked patterns (`*.gif`, `*.mp4`,
  `*.webm`, `*.png`). Run `git lfs track` to verify before adding new assets.

---

## 14. Performance Hints

- `load_entries` allocates two `Vec<FsEntry>` per directory read — acceptable
  since it runs only on navigation events, never in the hot draw loop.
- `render_list` iterates only the visible window
  (`skip(scroll_offset).take(visible_height)`). Do not change this to iterate
  the full entries list.
- `fmt_size` is called once per visible entry per frame. No caching needed.
- If the entry list grows beyond ~10 000 items, consider lazy loading or
  virtual scrolling, but do not over-engineer for the common case.
- `save_theme` does a single `fs::write` on every `t` / `[` keypress. This is
  intentional (last-write-wins persistence) and fast enough at human typing
  speed. Do not debounce unless profiling shows it matters.

---

## 15. Best Practices

### General
- **Fail loudly in tests, silently in production.** Use `expect("reason")` in
  test helpers; use `let _ = ...` or `unwrap_or_default` in production code
  paths where failure is expected and recoverable.
- **Prefer explicit over implicit.** Functions that touch the filesystem should
  accept a `&Path` argument rather than resolving the path themselves. Reserve
  path-resolution wrappers for the top-level call site.
- **One concern per function.** If a function both resolves a path *and* reads
  a file, split it into two functions. This makes each half independently
  testable.
- **No global mutable state.** All app state lives in `App`. No `static mut`,
  no `lazy_static`, no `OnceCell` storing mutable data.
- **Do not over-engineer.** This is a small, focused tool. Resist the urge to
  add abstraction layers, trait objects, or generics unless there is a concrete
  second use-case in the same codebase.

### Rust-specific
- Prefer `Option` chaining (`?`, `and_then`, `unwrap_or_else`) over nested
  `if let` chains.
- Use `PathBuf::from` + `join` for path construction. Never concatenate path
  strings with `+` or `format!`.
- Derive `Debug` on every internal struct and enum — it costs nothing and pays
  off immediately during debugging.
- Avoid `clone()` in hot paths. In the draw loop, pass `&Theme` references
  rather than cloning the full struct each frame.
- Mark functions that take ownership of a `String` with `impl Into<String>` or
  `&str` to keep call sites clean.

### TUI-specific
- Always restore the terminal (raw mode, alternate screen) even on panic. The
  current `run_loop` does this via explicit cleanup after `run_loop` returns.
  Do not add early returns from `run()` that skip cleanup.
- Never block in the draw closure. Any work that could take > 1 ms belongs in
  a background thread or a pre-computed field on `App`.
- Keep the action bar and key hints in sync with the actual key bindings. If
  you add a new key, update `render_action_bar_spans` in the same commit.

---

## 16. What Still Needs Doing

This section is a living list of known gaps and planned improvements. Move
items to `CHANGELOG.md` when they are completed.

### High priority
- [ ] **Mouse support** — click to focus a pane, scroll wheel to move cursor,
      click a theme in the theme panel to select it. `EnableMouseCapture` is
      already set; the event loop just needs to handle `Event::Mouse`.
- [ ] **Rename / move** — add an `r` key to rename the highlighted entry
      in-place with a text input overlay, similar to the delete confirm modal.
- [ ] **Create file / directory** — add `n` (new file) and `N` (new directory)
      keys with an inline input modal.

### Medium priority
- [ ] **Bookmarks** — allow the user to mark directories with `b` and jump to
      them with a picker overlay, persisted to config.
- [ ] **Preview pane** — optional third column showing a file preview (text
      files) or metadata (size, permissions, modification time).
- [ ] **Symlink display** — visually distinguish symlinks from regular files
      and show their target path in the status bar when highlighted.
- [ ] **Bulk selection** — `Space` to toggle selection, `Y`/`X` to
      copy/cut all selected items at once.
- [ ] **Integration tests for the binary** — a lightweight test harness that
      drives the `tfe` binary via `std::process::Command` and asserts on exit
      codes and stdout, covering `--list-themes`, `--theme`, and `--print-dir`.

### Low priority / nice-to-have
- [ ] **Windows support** — `crossterm` and `ratatui` both support Windows;
      the main blocker is path handling (`\` vs `/`) and the lack of XDG.
      `config_path()` needs a `%APPDATA%` fallback.
- [ ] **Configurable key bindings** — read from config file, fall back to
      defaults. This is a larger refactor and a potential breaking change.
- [ ] **Plugin / hook system** — shell hooks on file selection (e.g. auto-open
      in `$EDITOR`) rather than requiring shell integration wrappers.
- [ ] **Accessibility** — expose an `--accessible` flag that replaces Unicode
      box-drawing and emoji with plain ASCII and text labels.
- [ ] **VHS recordings for new features** — `file_ops.gif` exists; add tapes
      for theme persistence, single-pane mode, and search.

---

## 17. Checklist Before Opening a PR

- [ ] `just check-all` passes locally (fmt + clippy + test)
- [ ] New feature has tests in the appropriate `mod tests` block (see §3 and §4)
- [ ] New public items have `///` doc comments
- [ ] Key hints in `render_action_bar_spans` match actual bindings
- [ ] No new `[dependencies]` added without discussion
- [ ] `Cargo.toml` version is **not** bumped in the PR (the release workflow owns that)
- [ ] Commit message follows Conventional Commits
- [ ] New binary assets added to Git LFS (not committed as plain blobs)