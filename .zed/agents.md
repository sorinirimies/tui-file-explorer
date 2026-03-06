# agents.md — AI Agent Instructions for `tui-file-explorer`

This file instructs AI agents (Claude, Copilot, etc.) working in this
repository. Read `rules.md` first — this file extends it with agent-specific
operating procedures.

---

## 0. Prime Directive

You are working on a production Rust TUI crate. Every change you make must
leave the repository in a **shippable state**: green tests, zero clippy
warnings, committed, tagged, and pushed. Never leave work half-done.

---

## 1. Mandatory Workflow

For **every** task, follow this sequence exactly. Do not skip steps.

```
1. Read           — understand the code you will touch before touching it
2. Implement      — make the change
3. Test           — write or update tests for every changed behaviour
4. Clippy         — cargo clippy --all-targets -- -D warnings  (zero warnings)
5. Test suite     — cargo test  (zero failures)
6. Commit         — git commit with a Conventional Commit message
7. Release        — just release <next-version>
8. Verify         — confirm the tag and push succeeded
```

If step 4 or 5 fails, fix it before proceeding. Never commit broken code.
Never skip step 7 — an untagged, unpushed change is incomplete work.

---

## 2. Before You Write a Single Line

Read the files you are about to touch:

```bash
# Always read before editing
cat src/app.rs          # App state, Editor enum, handle_event
cat src/main.rs         # CLI, run_loop, TUI teardown/restore
cat src/ui.rs           # draw, render_options_panel, render_action_bar_spans
cat src/persistence.rs  # load_state_from, save_state_to, AppState
cat src/shell_init.rs   # auto_install_with, rc_path_with, detect_shell
```

Use the outline first (tool: `read_file` without line numbers), then read the
specific sections relevant to your change. Do not guess at structure.

---

## 3. Choosing the Next Version

| Change type | Example | Bump |
|---|---|---|
| Bug fix, test fix, doc update | Fix clippy lint, fix failing test | patch |
| New user-visible feature | New key binding, new CLI flag, new UI element | minor |
| New public library API | New type/function on `FileExplorer` | minor |
| Breaking public API change | Remove or rename a public item | major |

When in doubt, **patch**. Always prefer releasing often over batching.

Current version is always in `Cargo.toml`. Run `just version` to print it.

---

## 4. Commit Message Format

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <short imperative summary>

<optional body — what changed and why, not how>
```

Types: `feat`, `fix`, `refactor`, `perf`, `doc`, `chore`, `test`

**Good examples:**
```
fix: make auto_install_with shell-independent for hermetic tests
feat: add Editor enum and e-key file-open binding
chore: bump version to 0.4.3
```

**Bad examples:**
```
fixed stuff
WIP
update
```

---

## 5. Key Architecture Rules

### App / run_loop split
- `App::handle_event` sets flags and state — it never touches the terminal.
- `run_loop` in `main.rs` owns terminal teardown and restore. It checks
  `app.open_with_editor` after every event and handles the full
  suspend → spawn → restore → reload cycle.
- Do not call `disable_raw_mode`, `enable_raw_mode`, `LeaveAlternateScreen`, or
  `EnterAlternateScreen` from anywhere inside `app.rs`.

### Editor enum
- `Editor::None` is the default — the feature is fully opt-in.
- `binary()` returns `Option<&str>`. Always unwrap with `if let Some(b) = ...`.
- Cycle order: `None → Helix → Neovim → Vim → Nano → Micro → None → …`
- Persistence key strings: `none`, `helix`, `nvim`, `vim`, `nano`, `micro`,
  `custom:<binary>`.
- `Custom` variants jump back to `None` on cycle.

### Hermetic tests for shell/env code
- Every function that reads `$SHELL`, `$HOME`, `$ZDOTDIR`, or `$XDG_CONFIG_HOME`
  must have a `pub(crate) fn foo_with(shell: Option<Shell>, home: Option<&Path>, …)`
  twin that accepts explicit overrides.
- The `shell` parameter must be `Option<Shell>`: `None` = detect, `Some(s)` = use directly.
- Tests that are shell-specific must pass `Some(Shell::Zsh)` / `Some(Shell::Bash)`.
  **Never rely on `$SHELL` being a particular value in a test.**
- All path arguments in tests must come from `tempfile::tempdir()`.

### Persistence
- `AppState` fields are all `Option<T>` — absent keys load as `None`, not a default.
- Every new persisted field needs: a `const KEY_*` string, a match arm in
  `load_state_from`, a write line in `save_state_to`, and a field on `AppState`.
- The `full_state_round_trips` test must include every field. Update it when you
  add a field.

### UI
- The options panel has three zones: header (3 lines) | boolean toggles | editor section.
  Each zone is a fixed-height `Constraint::Length`. Update the constraints if you
  add rows.
- The action bar spans list has a stable count tested in `ui.rs`. When you add a
  span pair (`key` + `description`), update the count assertion and the `key_labels`
  arrays in all four affected tests.
- Never hardcode `Color::Rgb(…)` outside `palette.rs`.

---

## 6. Testing Rules

- **Every changed behaviour needs a test.** No exceptions.
- Tests live in `#[cfg(test)] mod tests` at the bottom of the file they test.
- Test names: `verb_condition_expectation` — e.g. `move_down_clamps_at_last`.
- Use `tempfile::tempdir()` for all filesystem tests.
- Use `make_app(dir.path().to_path_buf())` for all `App`-level tests.
- No `#[ignore]`. Fix flaky tests rather than ignoring them.
- The full suite must be **zero failures** — partial green is not acceptable.

### Test coverage requirements
| Area | Requirement |
|---|---|
| `Editor` enum | `binary`, `label`, `cycle`, `to_key`, `from_key`, full cycle loop |
| `AppOptions` / `App` defaults | Every new field has a default-value test |
| `AppState` persistence | `full_state_round_trips` includes every field |
| `handle_event` key bindings | Every new key has at least one test |
| `render_action_bar_spans` | Count, key spans bold+accent, description spans dim |
| `auto_install_with` variants | Each shell-specific test passes `Some(Shell::X)` |

---

## 7. Clippy Rules

- Target: zero warnings with `-D warnings`. No exceptions.
- Never use `#[allow(...)]` without a comment explaining exactly why.
- Common pitfalls in this codebase:
  - `clippy::empty_line_after_doc_comments` — do not put a blank line between
    a `/// …` doc comment and the item it documents. Put section banners
    (`// ── Foo ──`) *before* the doc comment, not after.
  - `clippy::derivable_impls` — use `#[derive(Default)]` + `#[default]` on the
    variant instead of a manual `impl Default`.
  - `clippy::enum_variant_names` — `Modal` variants must not share a common suffix.

---

## 8. Release Command

Always use the `just` recipe — never run the steps manually:

```bash
just release <version>   # runs check-all, bump_version.sh --yes, git push
```

This single command:
1. Runs `cargo fmt --check`
2. Runs `cargo clippy -- -D warnings`
3. Runs `cargo test`
4. Updates `Cargo.toml`, `Cargo.lock`, `README.md` badges
5. Generates `CHANGELOG.md` via `git-cliff`
6. Commits all changes
7. Creates an annotated `v<version>` tag
8. Pushes branch + tag to `origin` (GitHub Actions then publishes to crates.io)

---

## 9. What You Must Never Do

| Never | Why |
|---|---|
| Skip `just release` after a fix | Untagged changes are unreleased — treat as incomplete |
| Commit broken tests or clippy warnings | Every commit on `main` must be shippable |
| Use `#[allow(...)]` without explanation | Hides real problems |
| Call `disable_raw_mode` from `app.rs` | Terminal lifecycle belongs to `run_loop` |
| Read `$SHELL` / `$HOME` directly in a test | Use the `_with` twin with explicit paths |
| Add a dependency without justification | Minimal dependency surface is a project goal |
| Hardcode `Color::Rgb(…)` outside `palette.rs` | All colours must come from the theme |
| Leave `open_with_editor` unguarded in `run_loop` | Always use `if let Some(b) = app.editor.binary()` |
| Expose `Editor` through the public library API | It is binary-only, lives in `app.rs` |
| Batch unrelated changes into one commit | One logical change = one commit = one release |

---

## 10. Quick Reference

```bash
just --list              # show all available tasks
just version             # print current version
just check-all           # fmt-check + clippy + test
just test                # cargo test
just clippy              # cargo clippy -- -D warnings
just fmt                 # cargo fmt
just release 0.4.3       # full release: checks → bump → tag → push
just release-all 0.4.3   # release to both GitHub and Gitea
just changelog-preview   # preview what git-cliff would write
just doc                 # build and open rustdoc
```

### Module responsibilities (one-line)
| File | Owns |
|---|---|
| `app.rs` | `App`, `Editor`, `AppOptions`, `handle_event`, file ops, modal logic |
| `main.rs` | CLI (`Cli`), `run_loop`, TUI setup/teardown, editor spawn, persistence calls |
| `ui.rs` | `draw`, `render_options_panel`, `render_action_bar_spans`, `render_modal` |
| `persistence.rs` | `AppState`, `load_state_from`, `save_state_to`, key constants |
| `shell_init.rs` | `auto_install_with`, `rc_path_with`, `snippet`, `is_installed` |
| `explorer.rs` | `FileExplorer`, `handle_key`, `load_entries`, navigation, search |
| `render.rs` | Library rendering: `render`, `render_themed`, `render_dual_pane_themed` |
| `palette.rs` | All colour constants and `Theme::all_presets()` |
| `types.rs` | `FsEntry`, `ExplorerOutcome`, `SortMode` |
| `fs.rs` | `copy_dir_all`, `resolve_output_path` |
```