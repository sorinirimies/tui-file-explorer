# rules.md — Development Guidelines for `tui-file-explorer`

This file is the single source of truth for conventions, patterns, and
decisions made in this codebase. Read it before opening a PR.

---

## 1. Project Layout

```
tui-file-explorer/
├── src/
│   ├── lib.rs          # Crate root — docs, module declarations, public re-exports only
│   ├── types.rs        # Plain data types: FsEntry, ExplorerOutcome, SortMode
│   ├── palette.rs      # Colour constants (all pub so callers can reference them)
│   ├── explorer.rs     # FileExplorer state machine + filesystem helpers + unit tests
│   ├── render.rs       # All ratatui Frame rendering (render / render_header / render_list / render_footer)
│   │
│   │   ── binary-only (not part of the public library API) ──
│   ├── main.rs         # CLI entry-point (tfe binary): arg parsing, terminal setup, run loop
│   ├── app.rs          # App state: two-pane layout, clipboard, Modal, multi-delete, event handling
│   ├── ui.rs           # Binary-specific TUI drawing: action bar, theme panel, modal overlay
│   ├── fs.rs           # fs helpers for the binary: copy_dir_all, resolve_output_path, emit_path
│   └── persistence.rs  # State persistence: load/save last dir, theme, sort mode, hidden flag, editor
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
- Binary-only logic (two-pane app state, CLI flags, persistence) stays in the
  binary modules and must not leak into the library crate.
- `Editor` enum lives in `app.rs` (binary-only). It is never exposed through
  the library crate.
- `shell_init.rs` owns all shell-detection and rc-file writing logic.
  `main.rs` owns the post-TUI notice that tells the user to `source` their rc
  file; it must never be emitted from inside `shell_init.rs` during startup.

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
| Macros | `snake_case!` | `handle_input_mode!`, `render_input_footer!` |

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
  - Types: `FileExplorer`, `FsEntry`, `ExplorerOutcome`, `SortMode`
  - Functions: `render`, `render_themed`, `fmt_size`, `entry_icon`
  - Module: `palette` (constants only)
- Public fields on `FileExplorer`:
  - `current_dir`, `entries`, `cursor`, `extension_filter`, `show_hidden`,
    `sort_mode`, `search_query`, `search_active` — navigation/filter state
  - `marked: HashSet<PathBuf>` — paths space-marked for multi-item operations
- Public methods added for multi-delete support:
  - `marked_paths()` — shared reference to the marked set
  - `toggle_mark()` — toggle the mark on the current entry and advance cursor
  - `clear_marks()` — clear all marks (called after multi-delete or navigation)
- `pub(crate)` is used for anything shared between library modules that is not
  part of the external API (e.g. `load_entries`, `scroll_offset`, `status`).
- Do not expose `scroll_offset` or `status` fields publicly — they are rendering
  implementation details.
- **Macros are never public.** Both `handle_input_mode!` and
  `render_input_footer!` are module-private `macro_rules!` definitions. Do not
  add `#[macro_export]` to them.

---

## 5. Ratatui Patterns

- `render(explorer, frame, area)` is the **only** public rendering entry-point.
  It owns the layout split; callers never pass pre-split areas.
- All widget construction is local to the `render_*` helpers — no widgets are
  stored in `FileExplorer`.
- Marked entries are rendered with a `◆` leading marker and `theme.brand`
  colour. The list block title shows `◆ N marked` when marks are active.
- The `Modal::MultiDelete` variant uses a taller dynamic-height overlay that
  lists up to 6 file names and a `… and N more` overflow line.
- Scroll state is managed manually via `scroll_offset`; `ListState::select` is
  used only to drive ratatui's internal highlight — it is always set to the
  *visible* index (`cursor - scroll_offset`), not the absolute index.
- Avoid allocating inside the `draw` closure hot-path where possible.
  Prefer `format!` only when the string genuinely changes per frame.
- The palette in `palette.rs` is the single source of colour truth.
  Never hardcode `Color::Rgb(...)` anywhere except `palette.rs`.

---

## 6. Macros

### When to use macros

Use `macro_rules!` only when **three or more call sites** share identical
boilerplate that cannot be extracted into a regular function (e.g. because the
duplicated code references different field names by identifier, not by value).
Prefer functions, closures, or trait methods for everything else.

Current macros in the codebase:

| Macro | File | Purpose |
|---|---|---|
| `handle_input_mode!` | `explorer.rs` | De-duplicates the char-input guard block shared by `rename_active`, `touch_active`, and `mkdir_active` inside `handle_key`. |
| `render_input_footer!` | `render.rs` | De-duplicates the inline-input `Paragraph` render + early-return shared by all four active-mode footer arms in `render_footer`. |

### `handle_input_mode!` — signature and contract

```rust
handle_input_mode!(self, key, $active, $input, $on_enter);
```

| Parameter | Kind | Description |
|---|---|---|
| `$self` | ident | The `&mut self` receiver. |
| `$key` | ident | The `KeyEvent` local taken by value in `handle_key`. |
| `$active` | ident | Boolean field name on `FileExplorer` (e.g. `rename_active`). |
| `$input` | ident | `String` field name on `FileExplorer` (e.g. `rename_input`). |
| `$on_enter` | expr | Spliced into the `KeyCode::Enter` arm. Must set `$active = false`, clear `$input`, and `return` an `ExplorerOutcome`. |

The macro wraps the match in `if $self.$active { … }` so it is a complete no-op
when the mode is inactive. Arms covered by the macro (callers must not
re-implement them in `$on_enter`):
- `Char(c)` with empty or Shift modifiers → push, return `Pending`
- `Backspace` → pop, return `Pending`
- `Esc` → clear flag + input, return `Pending`
- `_` → return `Pending` (consumes unrecognised keys while in the mode)

### `render_input_footer!` — signature and contract

```rust
render_input_footer!(
    explorer, frame, area, theme,
    $active, $input_expr, $label, $colour, $hint
);
```

| Parameter | Kind | Description |
|---|---|---|
| `$explorer` | expr | `&FileExplorer` reference. |
| `$frame` | expr | `&mut Frame` reference. |
| `$area` | expr | `Rect` for the footer zone. |
| `$theme` | expr | `&Theme` reference. |
| `$active` | ident | Boolean field name on `$explorer`. |
| `$input_expr` | expr | Expression yielding `&str` / `String` for the typed input (e.g. `explorer.mkdir_input()` or `explorer.search_query.as_str()`). |
| `$label` | expr | String expression for the leading label span. |
| `$colour` | ident | Field name on `$theme` used for **both** the label colour and the border colour. |
| `$hint` | expr | String expression for the trailing hint span. |

The macro renders a rounded `Paragraph` with four spans (label · input · cursor
block `█` · hint) and calls `return` after rendering. It is a complete no-op
when `$explorer.$active` is false.

### Macro placement rules

- Define macros at **module level** (not inside `impl` blocks), placed in their
  own named section with the standard divider banner:
  ```rust
  // ── handle_input_mode! ────────────────────────────────────────────────────────
  ```
- Place the macro definition **before** any `impl` block or function that uses
  it, so the compiler sees it in order.
- Do **not** use `#[macro_export]`. All macros in this codebase are
  module-private by design.
- Add a block comment above each macro explaining: what it does, the parameter
  list, and what the caller must supply.

### Adding a new macro

Before reaching for `macro_rules!`:
1. Count the call sites — fewer than three is not enough.
2. Confirm a function cannot do the job (field-name metaprogramming is the
   usual reason it cannot).
3. Write the macro with a full doc-comment block (see existing examples).
4. Add tests that exercise each arm of the macro through real call sites.
   Test names: `<mode>_<arm>_via_macro` (e.g. `mkdir_mode_esc_cancels_via_macro`).

---

## 7. Testing

### What to test
- Every public method on `FileExplorer` must have at least one test.
- Every `ExplorerOutcome` variant must be covered.
- Every `Modal` variant (`Delete`, `MultiDelete`, `Overwrite`) must be covered
  in `app.rs` tests — both the confirm and cancel paths.
- Edge cases: empty directory, cursor at boundaries, filesystem root ascent,
  last entry marking (no cursor overflow), partial errors in multi-delete.
- Every macro arm must be exercised through its real call sites. The macro
  itself is not tested in isolation — it is tested via the modes that use it.

### Conventions
- Tests live in a `#[cfg(test)] mod tests` block at the **bottom** of the file
  that owns the code under test:
  - `explorer.rs` — widget-level unit tests (key handling, mark toggle, navigation, macro arms)
  - `render.rs` — rendering smoke tests (each active-mode footer arm, no-panic assertions)
  - `app.rs` — integration-level tests (prompt_delete, confirm_delete_many, paste, clipboard)
  - `ui.rs` — action-bar span structure tests
- Use `tempfile::tempdir()` for all filesystem tests. Never rely on the real
  filesystem layout.
- `explorer.rs` fixture: `fn temp_dir_with_files() -> TempDir` — creates one subdir
  and three files. Add to it rather than duplicating setup in individual tests.
- `app.rs` fixture: `fn make_app(dir: PathBuf) -> App` — constructs an `App` with
  sensible defaults. Use this for all binary-level tests.
- Test function names follow `verb_condition_expectation`:
  ```rust
  fn move_down_clamps_at_last()
  fn handle_key_enter_on_dir_descends()
  fn confirm_delete_many_clears_marks_on_both_panes()
  fn mkdir_mode_esc_cancels_via_macro()
  fn render_footer_mkdir_active_does_not_panic()
  ```
- No `#[ignore]` tests. If a test is flaky, fix it.

### Running
```bash
just test          # cargo test
just test-all      # cargo test --all-features --all-targets
```

### Key bindings to keep covered
Every key binding that produces a non-trivial state change needs a test:

| Key | Action | Test location |
|---|---|---|
| `Space` | Toggle mark on current entry, cursor advances | `explorer.rs` |
| `d` (no marks) | Raise `Modal::Delete` for current entry | `app.rs` |
| `d` (with marks) | Raise `Modal::MultiDelete` for all marked paths | `app.rs` |
| `y` in modal | Confirm delete / multi-delete / overwrite | `app.rs` |
| any other key in modal | Cancel, set status message | `app.rs` |
| `Backspace` / `h` / `←` | Ascend, clear marks | `explorer.rs` |
| `Enter` / `l` on dir | Descend, clear marks | `explorer.rs` |
| `Enter` / `l` on file (editor ≠ None) | Set `open_with_editor` — TUI suspends, editor opens, TUI resumes | `app.rs` |
| `Enter` / `l` on file (editor = None) | Set `selected`, exit TUI (classic behaviour) | `app.rs` |
| `→` on dir | Descend, clear search + marks (never exits TUI) | `explorer.rs` |
| `→` on file | Move cursor down, never exits TUI | `explorer.rs` |
| `e` on file (editor ≠ None) | Same as Enter on file with editor — sets `open_with_editor` | `app.rs` / `main.rs` |
| `e` on dir or editor = None | Silent no-op — no status message | `app.rs` |
| `e` in options panel | Cycle `Editor` variant, update status message | `app.rs` |
| `n` | Activate mkdir mode | `explorer.rs` |
| `N` | Activate touch mode | `explorer.rs` |
| `r` | Activate rename mode (pre-filled) | `explorer.rs` |
| `Char(c)` in any input mode | Append char, stay in mode | `explorer.rs` (via macro tests) |
| `Backspace` in any input mode | Pop last char, stay in mode | `explorer.rs` (via macro tests) |
| `Esc` in any input mode | Cancel + clear input, return `Pending` | `explorer.rs` (via macro tests) |

### Render smoke tests (`render.rs`)
`render.rs` owns a `mod tests` block with smoke tests using `TestBackend`. Each
active-mode footer arm and the all-inactive path must have a no-panic assertion:

| Test | What it checks |
|---|---|
| `render_footer_mkdir_active_does_not_panic` | `mkdir_active = true` renders cleanly |
| `render_footer_touch_active_does_not_panic` | `touch_active = true` renders cleanly |
| `render_footer_rename_active_does_not_panic` | `rename_active = true` renders cleanly |
| `render_footer_search_active_does_not_panic` | `search_active = true` renders cleanly |
| `render_footer_all_inactive_does_not_panic` | All modes inactive renders cleanly |

Use `Terminal::new(TestBackend::new(80, 24))` and call `render` inside
`terminal.draw(…)`.

---

## 8. Documentation

- Every `pub` item needs a `///` doc comment.
- Doc comments use imperative mood: *"Return the highlighted entry."* not
  *"Returns the highlighted entry."*
- Code examples in doc comments must compile. Use `no_run` only when a real
  terminal is required (i.e. ratatui draw closures).
- `lib.rs` must have a module-layout table so users can orient themselves
  without reading every file.
- Run `just doc` to verify docs build without warnings before a release.

### Modal enum naming
- `Modal` variants must **not** share a common postfix (Clippy `enum_variant_names`).
- Current variants: `Delete`, `MultiDelete`, `Overwrite` — do not add a `Confirm`
  suffix or any other suffix that would be uniform across all variants.

---

## 9. Editor Launch Pattern

The `Editor` enum (`None`, `Helix`, `Neovim`, `Vim`, `Nano`, `Micro`,
`Custom(String)`) controls which editor is opened when the user presses `e`
on a file. Key design decisions to preserve:

- **`Editor::None` is the default.** The feature is fully opt-in. A fresh
  install never launches an editor unless the user cycles to one or passes
  `--editor` on the CLI.
- **`binary()` returns `Option<&str>`.** `Editor::None` yields `Option::None`.
  Callers must unwrap with `if let Some(binary) = app.editor.binary()` — never
  `.unwrap()` directly.
- **TUI teardown/restore lives in `run_loop` (`main.rs`), not in
  `handle_event` (`app.rs`).** `handle_event` only sets
  `app.open_with_editor = Some(path)`. `run_loop` checks this field after every
  event, tears down the terminal, spawns the editor synchronously with
  `Command::new(binary).arg(path).status()`, then restores the terminal and
  reloads both panes. This keeps `App` free of `Terminal` / raw-mode concerns.
- **`Enter` / `l` on a file is intercepted by `handle_event`** before the
  outcome reaches the exit path. When `editor != Editor::None` and the path is
  not a directory, `open_with_editor` is set and the function returns
  `Ok(false)` (TUI stays running). When `editor == Editor::None` the original
  behaviour is preserved: `selected` is set and the function returns `Ok(true)`
  (TUI exits, path printed to stdout for the shell wrapper).
- **Cycle order:** `None → Helix → Neovim → Vim → Nano → Micro → None → …`
  `Custom` variants skip directly back to `None` when cycled — they can only
  be set via `--editor` or the state file.
- **Persistence key:** `editor=<key>` in the state file. Key strings:
  `none`, `helix`, `nvim`, `vim`, `nano`, `micro`, `custom:<binary>`.
- **`--editor <EDITOR>` CLI flag** overrides the persisted value for the
  session only. Unknown values are treated as `Custom(value)`.
- **Options panel** shows the editor name in `success` colour + bold when an
  editor is selected, dim when `None` — mirroring the on/off style of boolean
  toggles.
- **Action bar** shows `e edit` between `d del` and `[ / t theme`.

### Dos and Don'ts
- **Do** guard every `open_with_editor` branch in `run_loop` with
  `if let Some(b) = app.editor.binary()` to handle the `None` case
  defensively, even though `handle_event` already skips `None`.
- **Do not** call `disable_raw_mode` / `enable_raw_mode` from inside `App`
  methods — that is exclusively `run_loop`'s responsibility.
- **Do not** add async editor launch — editors are synchronous by nature and
  blocking the event loop for the duration is the correct behaviour.
- **Do not** let `ExplorerOutcome::Selected` exit the TUI for files when an
  editor is configured. The intercept in `handle_event` must check
  `editor != Editor::None && !path.is_dir()` before falling through to the
  `selected`/exit path.

---

## 10. Shell Integration — cd-on-exit

### The `source:` stdout protocol

The shell wrapper captures everything `tfe` writes to stdout.  `tfe` uses a
two-line protocol:

| Line format | Meaning |
|---|---|
| `source:<absolute-path>` | Wrapper must source that file in the parent shell |
| anything else non-empty | Wrapper must `cd` / `Set-Location` to that path |

This allows `tfe` to request a `source` from its parent shell on first install
— something a child process cannot do directly.  The directive is always
terminated with `\n` (never NUL, even when `--null` is passed) because it is a
control message consumed by the wrapper, not a data path consumed by the caller.

The prefix constant is `shell_init::SOURCE_DIRECTIVE_PREFIX = "source:"`.  It
is `pub` so `main.rs` and tests can reference it without hardcoding the string.

**Backwards compatibility**: old wrappers that pre-date this protocol will
attempt to `cd` to `source:<path>`, which will fail silently (the path does not
exist) and leave the shell in its current directory.  No data is lost.

### How the wrapper is installed

`auto_install()` is called at the start of `run()` in `main.rs`, **before**
the TUI enters the alternate screen.  It:

1. Detects the current shell via `detect_shell()`.
2. Checks all candidate rc files for the shell (`is_installed`).
3. If the wrapper is absent, calls `install_or_print_to` to write it and
   returns `InitOutcome::Installed(path)`.
4. Returns the `InitOutcome` to the caller — it does **not** `eprintln!`
   anything itself.  The caller (`run()` in `main.rs`) is responsible for
   emitting the `source:` directive and the notice after the TUI exits.

### Why the notice must be deferred

`run()` stores `auto_install_outcome` before `enable_raw_mode()` and
`EnterAlternateScreen`.  Any output emitted *after* those calls goes into
the alternate screen buffer and is never seen by the user.  After the TUI
exits (after `result?;`) the terminal is back in normal mode.  At that point
`run()` does two things:

1. **Emits the `source:` directive to stdout** via `emit_source_directive(rc_path)`.
   The shell wrapper reads this line and sources the rc file in the parent
   shell process — making the `tfe` function available in the current session
   immediately, with no restart required.

2. **Prints a brief notice to stderr** so the user knows what happened.

```rust
if let shell_init::InitOutcome::Installed(ref rc_path) = auto_install_outcome {
    shell_init::emit_source_directive(rc_path);
    eprintln!("tfe: shell integration installed to {}", rc_path.display());
    eprintln!("  The wrapper function has been sourced into this session automatically.");
    eprintln!("  cd-on-exit is now active — no restart needed.");
}
```

**Never move this block inside `auto_install_with` or anywhere before
`LeaveAlternateScreen`.**

### `emit_source_directive`

```rust
pub fn emit_source_directive(rc_path: &Path)
```

Writes `source:<rc_path>\n` to stdout.  Errors are silently swallowed — a
failure to emit the directive is not fatal; the user will still see the stderr
notice and can source manually if needed.  Always uses `\n`, never `\0`.

### Shell recognition strings

`Shell::from_str` accepts these values (case-insensitive):

| Input | Maps to |
|-------|---------|
| `bash` | `Shell::Bash` |
| `zsh` | `Shell::Zsh` |
| `fish` | `Shell::Fish` |
| `powershell`, `pwsh` | `Shell::Pwsh` |
| `nu`, `nushell` | `Shell::Nu` |

`Shell::name()` returns the canonical lowercase name used in sentinel comments
and display messages: `bash`, `zsh`, `fish`, `powershell`, `nushell`.

### Shell fallback order

`detect_shell()` reads `$SHELL` on Unix/macOS and `$PSModulePath` on Windows.
`rc_path_with` applies the following priority rules:

| Shell | Priority order |
|-------|---------------|
| **zsh** | `$ZDOTDIR/.zshrc` → `~/.zshrc` (if exists) → `~/.zshenv` (if exists) → `~/.zshrc` (create fresh) |
| **bash** | `~/.bash_profile` (if exists, common on macOS) → `~/.bashrc` |
| **fish** | `$XDG_CONFIG_HOME/fish/functions/tfe.fish` → `~/.config/fish/functions/tfe.fish` |
| **powershell** | `$PROFILE` (set by PowerShell) → `~/Documents/PowerShell/…ps1` (Windows) → `~/.config/powershell/…ps1` (Linux/macOS) |
| **nushell** | `$XDG_CONFIG_HOME/nushell/config.nu` → `~/Library/Application Support/nushell/config.nu` (macOS) → `~/.config/nushell/config.nu` (Linux) → `%APPDATA%\nushell\config.nu` (Windows) |

The duplicate-install guard checks **all** candidates for the detected shell
(e.g. both `.zshrc` and `.zshenv` for zsh) so that a manually-added wrapper
in any of them prevents a second install.

For Nushell, the only candidate is `<nu_config_dir>/config.nu`.  The config
dir is resolved by `nu_config_dir_default()` which respects `$XDG_CONFIG_HOME`
on all platforms before falling back to the OS-conventional path.

### Updated wrapper shapes

All five wrappers now loop over every output line from `^tfe` / `command tfe`
and dispatch on the `source:` prefix.  The key structural change per shell:

**bash / zsh** — process substitution loop:
```bash
tfe() {
    local line
    while IFS= read -r line; do
        case "$line" in
            source:*) source "${line#source:}" ;;
            ?*)       cd "$line" ;;
        esac
    done < <(command tfe "$@")
}
```

**fish** — `for` loop over lines:
```fish
function tfe
    for line in (command tfe $argv | string split0 | string split "\n")
        if string match -q 'source:*' -- $line
            source (string replace 'source:' '' -- $line)
        else if test -n "$line"
            cd $line
        end
    end
end
```

**PowerShell** — pipeline `ForEach-Object`:
```powershell
function tfe {
    & (Get-Command tfe -CommandType Application).Source @args | ForEach-Object {
        if ($_ -like 'source:*') { . ($_ -replace '^source:', '') }
        elseif ($_)              { Set-Location $_ }
    }
}
```

**Nushell** — `def --env --wrapped`, simple `str trim` form:
```nushell
def --env --wrapped tfe [...rest] {
    let dir = (^tfe ...$rest | str trim)
    if ($dir | is-not-empty) { cd $dir }
}
```

> **Why no `source:` handling in Nushell?**  `source` is a parse-time keyword
> in Nushell — it cannot accept a dynamically-computed string argument at
> runtime.  The `source:` protocol is therefore intentionally skipped for Nu.
> The one-time bootstrap still requires opening a new terminal or running
> `source <config.nu>` manually, exactly as for other shells.

> **Why `def --env`?**  In Nushell, environment mutations (including `cd`,
> which changes `$env.PWD`) are scoped to the command and discarded on return
> unless the command is declared with `--env`.  Without `--env` the `cd` call
> inside the wrapper takes effect only within the wrapper's own scope and the
> caller's directory is unchanged.

When no `source:` line is emitted (every run after the first install), the
bash/zsh/fish/PowerShell wrappers behave identically to the old single-line
versions — the loop body just hits the `cd` branch once.

### Nushell-specific details

- **Detection**: `detect_shell()` checks `$NU_VERSION` **first** on every
  platform (Nushell exports it to all child processes).  This takes priority
  over `$SHELL` and `$PSModulePath`, so running `tfe` from inside Nushell
  always identifies correctly as Nu regardless of OS.
- **Snippet**: Uses `def --env --wrapped tfe [...rest]`.
  - `--env` is **required** so that `cd` (which mutates `$env.PWD`) propagates
    to the caller's scope.  Without it the directory change is discarded when
    the command returns.
  - `--wrapped` forwards all flags and arguments to `^tfe` unchanged.
  - `^tfe` calls the external binary, bypassing any Nushell built-in shadowing.
  - `str trim` strips the trailing newline from stdout.
  - `is-not-empty` guards the `cd` so dismissing without cd-on-exit is a no-op.
  - `source` is **not** handled: it is a parse-time keyword in Nushell and
    cannot be called with a runtime string.  The `source:` protocol does not
    apply to Nushell.
  ```
  def --env --wrapped tfe [...rest] {
      let dir = (^tfe ...$rest | str trim)
      if ($dir | is-not-empty) { cd $dir }
  }
  ```
- **Config file**: written to `<nu_config_dir>/config.nu`.  The directory is
  created automatically by `install()` via `create_dir_all`.
- **`nu_config_dir` parameter**: `rc_path_with`, `auto_install_with`, and
  `install_or_print_to` all accept a `nu_config_dir: Option<&Path>` override.
  Pass `None` in production (resolved via `nu_config_dir_default()`); pass
  `Some(temp_dir)` in tests.  `rc_path_with` returns `None` when
  `nu_config_dir` is `None` — callers must always provide a value or let the
  production helpers resolve it.
- **`is_installed` slow path**: recognises hand-written Nushell wrappers by
  looking for `def --wrapped tfe` (with or without `--env`) followed by `^tfe`
  within 6 lines.

### Windows

On Windows:
- `detect_shell()` returns `Some(Shell::Nu)` when `$NU_VERSION` is set
  (Nushell running), `Some(Shell::Pwsh)` when `$PSModulePath` is set
  (PowerShell running), and `None` for CMD or unknown shells.
- `auto_install_with` runs normally for both Pwsh and Nu and installs into
  the appropriate config file.
- The `--init` flag accepts `powershell` (or `pwsh`) and `nushell` (or `nu`)
  on Windows.  Passing `--init bash/zsh/fish` on Windows prints an explicit
  error message directing the user to WSL.
- Editor launch uses `cmd /C <binary>` on Windows so that `.cmd`/`.bat`
  shims (e.g. `code.cmd`, `nvim.cmd`) resolve correctly.  On Unix, editors
  are spawned directly with `/dev/tty` as stdin/stdout/stderr.
- CMD users: there is no CMD equivalent of shell functions.  The cd-on-exit
  feature is PowerShell-only or Nushell-only on Windows.  WSL users should
  run the Linux `tfe` binary inside WSL and use `tfe --init bash/zsh`.

### `--init` error message

The error message for an unrecognised shell must list all five supported
shells:

```
tfe: unrecognised shell '…'. Supported: bash, zsh, fish, powershell, nushell
```

Never omit `powershell` or `nushell` from this list.

### Snippet test requirements

Every shell's snippet must be covered by at least these assertions:

| Test | What it checks |
|---|---|
| `snippet_<shell>_contains_function_body` | `command tfe` / `^tfe` present; `cd` present; `source:` handler present (bash/zsh/fish/pwsh only) |
| `snippet_<shell>_handles_source_directive` | snippet contains `"source:"` (bash/zsh/fish/pwsh only — not Nushell) |
| `snippet_nushell_contains_def_env` | Nushell snippet contains `--env` |
| `snippet_nushell_contains_def_wrapped` | Nushell snippet contains `def --env --wrapped tfe` |
| `snippet_nushell_does_not_use_each_closure` | Nushell snippet does **not** contain `each {` (cd must be at top level) |
| `snippet_nushell_uses_str_trim` | Nushell snippet contains `str trim` |
| `snippet_<shell>_differs_from_bash` | non-bash snippets are distinct (zsh is the exception — identical to bash) |

When modifying any snippet:
- For bash/zsh/fish/PowerShell: update both the structural assertion and the `source:` handler assertion.
- For Nushell: update `def --env --wrapped`, `^tfe`, `str trim`, `is-not-empty` assertions.
  **Never** add `source:` handling to the Nushell snippet — `source` is a parse-time
  keyword and cannot be called with a runtime string.

The loop variable in the non-Nu wrappers is `$line` (bash/zsh/fish) or `$_` (PowerShell).
The Nushell wrapper uses a plain `let dir` binding, not a loop.

### `nu_config_dir_default()`

This `pub(crate)` helper resolves the platform-default Nushell configuration
directory without requiring the caller to pass env vars.  Priority:

1. `$XDG_CONFIG_HOME/nushell` — overrides the platform default on all OSes.
2. `~/Library/Application Support/nushell` — macOS default (via `$HOME`).
3. `%APPDATA%\nushell` — Windows default.
4. `~/.config/nushell` — Linux / other Unix default.

Returns `None` when `$HOME` / `$APPDATA` is unset.  Tests must use
`auto_install_with(…, Some(tempdir))` rather than calling
`nu_config_dir_default()` directly so they stay hermetic.

### `auto_install` return type

`auto_install() -> InitOutcome` — the return value must be preserved by
callers.  Do not change it back to `()`.  `auto_install_with` likewise returns
`InitOutcome`.

When the wrapper is already installed everywhere, `auto_install_with` returns
`InitOutcome::AlreadyInstalled(path)` where `path` is the first candidate that
contains the wrapper.

---

## 11. Hermetic Test Pattern for Shell-Detection Code

Any function that reads environment variables (`$SHELL`, `$HOME`, `$ZDOTDIR`,
`$XDG_CONFIG_HOME`) **must** have a `_with(…)` twin that accepts explicit path
overrides so tests never depend on the real environment.

### Pattern
```rust
// Production entry-point — reads real env vars, calls the _with twin.
pub fn auto_install() {
    auto_install_with(
        None,                          // shell: None = detect from $SHELL
        home().as_deref(),
        xdg_config_home().as_deref(),
        zdotdir().as_deref(),
        bash_profile(home().as_deref()).as_deref(),
        zshenv(home().as_deref()).as_deref(),
    );
}

// Testable twin — every env-derived value is an explicit parameter.
pub(crate) fn auto_install_with(
    shell: Option<Shell>,              // None = detect; Some(s) = use directly
    home: Option<&Path>,
    xdg_config_home: Option<&Path>,
    zdotdir: Option<&Path>,
    bash_profile: Option<&Path>,
    zshenv: Option<&Path>,
) { … }
```

### Rules
- The `_with` twin is `pub(crate)` — never `pub`. Tests call it directly;
  external callers use the public wrapper.
- The `shell` parameter must come **first** and be `Option<Shell>`:
  `None` → fall back to `detect_shell()`; `Some(s)` → use `s` directly.
  This pattern is already used by `install_or_print_to`.
- Tests that exercise zsh-specific behaviour must pass `Some(Shell::Zsh)`.
  Tests that exercise bash-specific behaviour must pass `Some(Shell::Bash)`.
  **Never rely on `$SHELL` being a particular value in a test** — CI runners
  may have any shell set.
- Use `tempfile::tempdir()` for all path arguments. Never pass a real `$HOME`
  path into a `_with` function in tests.
- `auto_install_with` now returns `InitOutcome`.  Tests that call it must
  either assert on the return value or explicitly discard it with `let _ =`.
  Preferred: assert with `matches!(outcome, InitOutcome::Installed(_))` etc.
- Tests that call `auto_install_with(None, None, …)` must accept any of the
  four `InitOutcome` variants — the real `$SHELL` environment variable may be
  set on the CI host and cause paths other than `UnknownShell`.

### `auto_install_with` test coverage requirements

| Test | What it asserts |
|------|-----------------|
| `auto_install_returns_installed_on_first_run` | `Installed(_)` when no wrapper present |
| `auto_install_installed_outcome_carries_rc_path` | Path in `Installed` ends with the expected rc filename |
| `auto_install_returns_already_installed_on_second_run` | `AlreadyInstalled(_)` on second call |
| `auto_install_already_installed_outcome_carries_rc_path` | Path in `AlreadyInstalled` contains the wrapper |
| `auto_install_bash_returns_installed_on_first_run` | `Installed(_)` for bash; `.bashrc` contains wrapper |
| `auto_install_fish_returns_installed_on_first_run` | `Installed(_)` for fish; `tfe.fish` contains wrapper |
| `auto_install_zsh_already_installed_in_zshenv_returns_already_installed` | `AlreadyInstalled` when wrapper is in `.zshenv` |
| `auto_install_nushell_writes_config_nu_on_first_run` | `Installed(_)` for Nu; `config.nu` contains wrapper |
| `auto_install_nushell_does_not_duplicate_when_already_installed` | Content unchanged on second run |
| `auto_install_nushell_returns_already_installed_when_config_nu_has_sentinel` | `AlreadyInstalled(_)` on second run |
| `auto_install_nushell_creates_parent_directories_for_config_nu` | Deep nested path is created automatically |

### `emit_source_directive` test

`emit_source_directive_does_not_panic` — calls the function with a dummy path
and asserts no panic.  Capturing stdout in unit tests requires extra
dependencies (none allowed), so content correctness is verified indirectly
through the snippet tests and the `SOURCE_DIRECTIVE_PREFIX` constant test.

| Test | What it checks |
|---|---|
| `source_directive_prefix_constant_value` | `SOURCE_DIRECTIVE_PREFIX == "source:"` |
| `emit_source_directive_does_not_panic` | function completes without panicking |
| `snippet_bash_handles_source_directive` | bash snippet contains `"source:"` |
| `snippet_fish_handles_source_directive` | fish snippet contains `"source:"` |
| `snippet_pwsh_handles_source_directive` | PowerShell snippet contains `"source:"` |
| `snippet_nushell_handles_source_directive` | Nushell snippet contains `"source:"` |

### Nushell test helpers

Nushell tests use a local `fn auto_install_nu(nu_config_dir: &Path) -> InitOutcome`
helper (defined inside `mod tests`) that wires up the Nushell-specific argument
while leaving all POSIX arguments as `None`.  This keeps individual tests
concise.  There is no `install_or_print_nu` helper — use `install_or_print_to`
directly when needed, always passing `Some(nu_config_dir)` as the last argument.

---

## 11. Dependencies

- **Minimise dependencies.** The only `[dependencies]` should be `ratatui` and
  `crossterm`. Any new dependency requires justification in the PR description.
- `clap` is optional behind the `cli` feature (for the `tfe` binary).
- `tempfile` is the only allowed `[dev-dependencies]` entry.
- Pin to minor versions (`"0.30"`), not patch (`"0.30.0"`), to allow compatible
  updates without a PR.
- Run `just update` periodically. Review `Cargo.lock` diffs in PRs that touch
  dependencies.

---

## 12. Features

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

## 13. Commit & Release After Every Change

**Every completed implementation + test cycle must end with a commit and a new
release.** There is no such thing as "I'll release later" — if the code is
correct and the tests are green, ship it.

### Mandatory steps after any code change

```
implement  →  write / update tests  →  cargo clippy  →  cargo test
     →  git commit  →  just release <next-patch>  →  git push
```

1. **Implement** the feature or fix.
2. **Write or update tests** — every changed behaviour needs test coverage.
3. **`cargo clippy --all-targets -- -D warnings`** must be clean (zero warnings).
4. **`cargo test`** must be fully green (zero failures).
5. **`git commit`** with a Conventional Commit message describing the change.
6. **`just release <next-version>`** — runs all checks again, bumps
   `Cargo.toml`, updates `Cargo.lock`, regenerates `CHANGELOG.md`, creates the
   annotated git tag, and pushes branch + tag to GitHub.
7. **Never skip the release step.** A green test suite that is not tagged and
   pushed is an unreleased change — treat it as incomplete work.

### Choosing the version number
| Change type | Bump |
|---|---|
| Bug fix, internal refactor, doc update | patch (`0.1.x → 0.1.x+1`) |
| New user-visible feature, new public API | minor (`0.1.x → 0.2.0`) |
| Breaking public API change | major (`0.x.y → 1.0.0`) |

When in doubt, bump the patch. Releasing often is better than batching.

### One logical change = one commit = one release
Do not batch multiple unrelated fixes into a single release. Each logical
unit of work (feature, fix, refactor) gets its own commit and its own version
tag. This keeps the changelog readable and makes bisecting trivial.

### Quick reference
```bash
# After implementation + tests pass:
git add <changed files>
git commit -m "fix: <what was fixed>"    # or feat:/refactor:/chore: etc.
just release 0.1.X                       # bumps, tags, pushes
```

---

## 14. Versioning & Release Workflow

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

## 15. Git Hygiene

- Commits on `main` must pass `just check-all` (fmt + clippy + test).
- PRs should be squash-merged with a conventional commit message.
- Do not commit `target/`, `*.rs.bk`, `.DS_Store`, or `.zed/` — all covered
  by `.gitignore`.
- Tag format: `v<semver>` (e.g. `v0.2.0`). Tags are created by `bump_version.sh`
  and should never be moved after push.
- The `Cargo.lock` is committed (it's a binary in `.gitattributes`) so
  reproducible builds are guaranteed for contributors.

---

## 16. Performance Hints

- `load_entries` allocates two `Vec<FsEntry>` per directory read — acceptable
  since it only runs on navigation events, never in the hot draw loop.
- `render_list` iterates only the visible window (`skip(scroll_offset).take(visible_height)`).
  Do not change this to iterate the full entries list.
- `fmt_size` is called once per visible entry per frame. It is intentionally
  simple — no caching needed at current scale.
- The `handle_input_mode!` and `render_input_footer!` macros expand inline at
  each call site — there is no function-call overhead. Do not extract them into
  functions to "reduce overhead"; the compiler already inlines trivial functions.
- If the entry list grows beyond ~10 000 items, consider lazy loading or
  virtual scrolling, but do not over-engineer for the common case.

---

## 17. Checklist Before Opening a PR

- [ ] `just check-all` passes locally (fmt + clippy + test)
- [ ] New public items have `///` doc comments
- [ ] New logic has tests in the appropriate `mod tests` block
- [ ] New macros have: block-comment docs, named section divider, tests via real call sites
- [ ] A focused commit has been made with a Conventional Commit message
- [ ] `just release <next-version>` has been run and the tag + branch pushed
- [ ] No new dependencies added without discussion
- [ ] Commit messages follow Conventional Commits
- [ ] `render.rs` smoke tests cover any new active-mode footer arm
- [ ] `explorer.rs` macro arm tests cover any new input-mode that uses `handle_input_mode!`
