# =============================================================================
# basic.tape — VHS recording for the tui-file-explorer "basic" example
#
# Prerequisites:
#   cargo build --example basic
#   vhs examples/vhs/basic.tape
#
# Output: examples/vhs/generated/basic.gif
# =============================================================================

Output examples/vhs/generated/basic.gif

Set Shell       "bash"
Set FontSize    14
Set FontFamily  "JetBrains Mono"
Set Width       1200
Set Height      700
Set Theme       "Dracula"
Set Padding     16
Set PlaybackSpeed 1.0

# ── Launch ─────────────────────────────────────────────────────────────────────
Type "cargo run --example basic"
Enter
Sleep 3s

# ── Navigate down through a few entries ───────────────────────────────────────
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Jump to top and bottom ────────────────────────────────────────────────────
Sleep 300ms
Ctrl+Home
Sleep 400ms
End
Sleep 400ms
Home
Sleep 400ms

# ── Descend into the src directory ───────────────────────────────────────────
# Navigate to src/ (likely near the top after dirs-first sort)
Down
Sleep 200ms
Down
Sleep 200ms
Enter
Sleep 600ms

# ── Browse inside src/ ────────────────────────────────────────────────────────
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Ascend back to root ───────────────────────────────────────────────────────
Sleep 400ms
Backspace
Sleep 600ms

# ── Toggle hidden files on ────────────────────────────────────────────────────
Sleep 300ms
Type "."
Sleep 600ms

# ── Toggle hidden files off ───────────────────────────────────────────────────
Type "."
Sleep 500ms

# ── Select a file ─────────────────────────────────────────────────────────────
# Navigate down to the first non-directory file
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
Enter
Sleep 800ms

# ── Dismiss ───────────────────────────────────────────────────────────────────
Sleep 500ms
Escape
Sleep 1s
