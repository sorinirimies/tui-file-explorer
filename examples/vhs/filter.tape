# =============================================================================
# filter.tape — VHS recording for the tui-file-explorer "filter" example
#
# Demonstrates the extension-filter feature: only files with the given
# extensions are selectable; all other files are shown but dimmed.
#
# Prerequisites:
#   cargo build --example basic
#   vhs examples/vhs/filter.tape
#
# Output: examples/vhs/generated/filter.gif
# =============================================================================

Output examples/vhs/generated/filter.gif

Set Shell       "bash"
Set FontSize    14
Set FontFamily  "JetBrains Mono"
Set Width       1200
Set Height      700
Set Theme       "Dracula"
Set Padding     16
Set PlaybackSpeed 1.0

# ── Launch with a .rs + .toml extension filter ────────────────────────────────
Type "cargo run --example basic -- rs toml"
Enter
Sleep 3s

# ── Show that the footer reflects the active filter ───────────────────────────
Sleep 800ms

# ── Navigate — notice .rs / .toml files are highlighted, others are dimmed ────
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms

# ── Attempt to select a non-matching file (should show status message) ────────
# Navigate to a directory first and descend into src/
Home
Sleep 300ms
Down
Sleep 200ms
Enter
Sleep 600ms

# ── Inside src/ — .rs files should be selectable (highlighted green) ──────────
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms

# ── Select a matching .rs file ────────────────────────────────────────────────
Sleep 300ms
Enter
Sleep 800ms

# ── Ascend and show that Cargo.toml is also selectable ────────────────────────
Backspace
Sleep 500ms

# Navigate to Cargo.toml
Home
Sleep 300ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Select Cargo.toml ─────────────────────────────────────────────────────────
Sleep 300ms
Enter
Sleep 800ms

# ── Toggle hidden files to show dot-files are also subject to the filter ──────
Sleep 400ms
Type "."
Sleep 700ms
Type "."
Sleep 500ms

# ── Dismiss ───────────────────────────────────────────────────────────────────
Sleep 400ms
Escape
Sleep 1s
