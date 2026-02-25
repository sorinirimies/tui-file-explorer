# rules.md — Development Guidelines for `tui-file-explorer`

This file is the single source of truth for conventions, patterns, and
decisions made in this codebase. Read it before opening a PR.

---

## 1. Project Layout

```
tui-file-explorer/
├── src/
│   ├── lib.rs        # Crate root — docs, module declarations, public re-exports only
│   ├── types.rs      # Plain data types: FsEntry, ExplorerOutcome
│   ├── palette.rs    # Colour constants (all pub so callers can reference them)
│   ├── explorer.rs   # FileExplorer state machine + filesystem helpers + unit tests
│   └── render.rs     # All ratatui Frame rendering (render / render_header / render_list / render_footer)
├── scripts/
│   ├── bump_version.sh   # Interactive version bump — runs checks before tagging
│   └── check_publish.sh  # Pre-publish gate — fmt, clippy, tests, doc, dry-run
├── .github/workflows/
│   ├── ci.yml        # fmt + clippy + doc + test on every PR / main push
│   └── release.yml   # Full release pipeline on v* tags → crates.io
└── justfile          # All developer tasks (see: just --list)
```

**Rules:**
- `lib.rs` must contain only `pub mod`, `pub use`, and crate-level doc comments.
  No logic lives there.
- Each module has one clear responsibility. If a module starts doing two things,
  split it.
- New public types go in `types.rs`; new colour tokens go in `palette.rs`.

---

## 2. Code Style

### Formatting
- `rustfmt` is mandatory. Config: `rustfmt.toml` (`max_width = 100`).
- Run before every commit: `just fmt` (or `cargo fmt`).
- CI will reject unformatted code (`just fmt-check`).

### Naming
| Thing | Convention | Example |
|---|---|---|
| Types / Traits | `UpperCamelCase` | `FileExplorer`, `ExplorerOutcome` |
| Functions / methods | `snake_case` | `handle_key`, `load_entries` |
| Constants | `SCREAMING_SNAKE_CASE` | `C_BRAND`, `C_ACCENT` |
| Modules | `snake_case` | `file_explorer`, `render` |
| Enum variants | `UpperCamelCase` | `Selected(PathBuf)`, `Dismissed` |

### Clippy
- Target: **zero warnings** with `-D warnings`.
- Run: `just clippy` (or `cargo clippy -- -D warnings`).
- Never suppress a lint with `#[allow(...)]` without a comment explaining why.

### Comments
- Use `//` line comments for implementation notes.
- Use `//!` at the top of each module file for the module doc.
- Use `///` on every `pub` item — types, functions, fields, variants.
- Section dividers use this style (80 chars):
  ```rust
  // ── Section title ──────────────────────────────────────────────────────────
  ```

---

## 3. Error Handling

- This crate has **no `Result`-returning public API** — `FileExplorer` methods
  are infallible. Errors (e.g. unreadable directories) degrade gracefully to an
  empty entries list or a status-bar message.
- Do not add `anyhow` or `thiserror` to `[dependencies]` — they are application
  concerns, not library concerns.
- IO errors from `fs::read_dir` are silently swallowed into an empty `Vec`.
  If richer error reporting is ever needed, add an `ExplorerOutcome::Error(String)`
  variant rather than propagating `io::Error`.

---

## 4. Public API Contract

- **Backwards-compat is king.** This crate follows SemVer strictly.
  - Adding a public item → minor bump.
  - Removing or changing a public item → major bump.
- The public surface is intentionally narrow:
  - Types: `FileExplorer`, `FsEntry`, `ExplorerOutcome`
  - Functions: `render`
  - Module: `palette` (constants only)
- `pub(crate)` is used for anything shared between modules that is not part of
  the external API (e.g. `is_selectable`, `entry_icon`, `fmt_size`).
- Do not expose `scroll_offset` or `status` fields publicly — they are rendering
  implementation details.

---

## 5. Ratatui Patterns

- `render(explorer, frame, area)` is the **only** public rendering entry-point.
  It owns the layout split; callers never pass pre-split areas.
- All widget construction is local to the `render_*` helpers — no widgets are
  stored in `FileExplorer`.
- Scroll state is managed manually via `scroll_offset`; `ListState::select` is
  used only to drive ratatui's internal highlight — it is always set to the
  *visible* index (`cursor - scroll_offset`), not the absolute index.
- Avoid allocating inside the `draw` closure hot-path where possible.
  Prefer `format!` only when the string genuinely changes per frame.
- The palette in `palette.rs` is the single source of colour truth.
  Never hardcode `Color::Rgb(...)` anywhere except `palette.rs`.

---

## 6. Testing

### What to test
- Every public method on `FileExplorer` must have at least one test.
- Every `ExplorerOutcome` variant must be covered.
- Edge cases: empty directory, cursor at boundaries, filesystem root ascent.

### Conventions
- Tests live in a `#[cfg(test)] mod tests` block at the **bottom** of the file
  that owns the code under test (currently `explorer.rs`).
- Use `tempfile::tempdir()` for all filesystem tests. Never rely on the real
  filesystem layout.
- Helper `fn temp_dir_with_files() -> TempDir` creates a canonical test fixture.
  Add to it rather than duplicating setup.
- Test function names follow `verb_condition_expectation`:
  ```rust
  fn move_down_clamps_at_last()
  fn handle_key_enter_on_dir_descends()
  ```
- No `#[ignore]` tests. If a test is flaky, fix it.

### Running
```bash
just test          # cargo test
just test-all      # cargo test --all-features --all-targets
```

---

## 7. Documentation

- Every `pub` item needs a `///` doc comment.
- Doc comments use imperative mood: *"Return the highlighted entry."* not
  *"Returns the highlighted entry."*
- Code examples in doc comments must compile. Use `no_run` only when a real
  terminal is required (i.e. ratatui draw closures).
- `lib.rs` must have a module-layout table so users can orient themselves
  without reading every file.
- Run `just doc` to verify docs build without warnings before a release.

---

## 8. Dependencies

- **Minimise dependencies.** The only `[dependencies]` should be `ratatui` and
  `crossterm`. Any new dependency requires justification in the PR description.
- `clap` is optional behind the `cli` feature (for the `tfe` binary).
- `tempfile` is the only allowed `[dev-dependencies]` entry.
- Pin to minor versions (`"0.30"`), not patch (`"0.30.0"`), to allow compatible
  updates without a PR.
- Run `just update` periodically. Review `Cargo.lock` diffs in PRs that touch
  dependencies.

---

## 9. Features

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

## 10. Versioning & Release Workflow

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
| `BREAKING CHANGE:` | Footer or `!` suffix → major bump |

### Release steps (maintainers)
```bash
# 1. Bump, run all checks, commit, tag:
just bump 0.2.0

# 2. Push branch + tag to GitHub (triggers CI + crates.io publish):
just release 0.2.0

# 3. (Optional) Mirror to Gitea:
just release-all 0.2.0
```

The `release.yml` GitHub Actions workflow automatically:
1. Runs fmt / clippy / tests
2. Generates changelog via git-cliff
3. Creates a GitHub Release with release notes
4. Publishes to crates.io (requires `CRATES_IO_TOKEN` in repo secrets)

### Pre-publish manual check
```bash
just release-check   # runs scripts/check_publish.sh
just publish-dry     # cargo publish --dry-run
```

---

## 11. Git Hygiene

- Commits on `main` must pass `just check-all` (fmt + clippy + test).
- PRs should be squash-merged with a conventional commit message.
- Do not commit `target/`, `*.rs.bk`, `.DS_Store`, or `.zed/` — all covered
  by `.gitignore`.
- Tag format: `v<semver>` (e.g. `v0.2.0`). Tags are created by `bump_version.sh`
  and should never be moved after push.
- The `Cargo.lock` is committed (it's a binary in `.gitattributes`) so
  reproducible builds are guaranteed for contributors.

---

## 12. Performance Hints

- `load_entries` allocates two `Vec<FsEntry>` per directory read — acceptable
  since it only runs on navigation events, never in the hot draw loop.
- `render_list` iterates only the visible window (`skip(scroll_offset).take(visible_height)`).
  Do not change this to iterate the full entries list.
- `fmt_size` is called once per visible entry per frame. It is intentionally
  simple — no caching needed at current scale.
- If the entry list grows beyond ~10 000 items, consider lazy loading or
  virtual scrolling, but do not over-engineer for the common case.

---

## 13. Checklist Before Opening a PR

- [ ] `just check-all` passes locally (fmt + clippy + test)
- [ ] New public items have `///` doc comments
- [ ] New logic has tests in the appropriate `mod tests` block
- [ ] `Cargo.toml` version is **not** bumped in the PR (the release workflow owns that)
- [ ] No new dependencies added without discussion
- [ ] Commit messages follow Conventional Commits
