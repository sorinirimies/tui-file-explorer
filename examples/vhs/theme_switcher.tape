# =============================================================================
# theme_switcher.tape — VHS recording for the tui-file-explorer theme_switcher
#
# Demonstrates live theme switching: press Tab to cycle through all 8 built-in
# named themes while navigating the file explorer.
#
# Prerequisites:
#   cargo build --example theme_switcher
#   vhs examples/vhs/theme_switcher.tape
#
# Output: examples/vhs/generated/theme_switcher.gif
# =============================================================================

Output examples/vhs/generated/theme_switcher.gif

Set Shell         "bash"
Set FontSize      14
Set FontFamily    "JetBrains Mono"
Set Width         1400
Set Height        750
Set Theme         "Dracula"
Set Padding       16
Set PlaybackSpeed 1.0

# ── Launch ────────────────────────────────────────────────────────────────────
Type "cargo run --example theme_switcher"
Enter
Sleep 3s

# ── Default theme — do a little navigation so the list looks alive ────────────
Sleep 500ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Switch to Grape ───────────────────────────────────────────────────────────
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms

# ── Switch to Ocean ───────────────────────────────────────────────────────────
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms
Up
Sleep 200ms

# ── Switch to Sunset ─────────────────────────────────────────────────────────
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms

# ── Switch to Forest ─────────────────────────────────────────────────────────
Sleep 600ms
Tab
Sleep 800ms

Up
Sleep 200ms
Up
Sleep 200ms
Down
Sleep 200ms

# ── Switch to Rose ───────────────────────────────────────────────────────────
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms

# ── Switch to Mono ───────────────────────────────────────────────────────────
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Up
Sleep 200ms

# ── Switch to Neon ───────────────────────────────────────────────────────────
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Cycle backwards with Shift+Tab back to Forest ────────────────────────────
Sleep 600ms
Shift+Tab
Sleep 700ms
Shift+Tab
Sleep 700ms
Shift+Tab
Sleep 700ms

# ── Navigate into src/ directory in the Forest theme ─────────────────────────
Sleep 400ms
Type "g"
Sleep 300ms
Down
Sleep 200ms
Enter
Sleep 700ms

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Select a .rs file ────────────────────────────────────────────────────────
Sleep 400ms
Enter
Sleep 800ms

# ── Ascend and switch to Neon for a dramatic finish ───────────────────────────
Backspace
Sleep 500ms
Tab
Sleep 400ms
Tab
Sleep 400ms
Tab
Sleep 800ms

# ── Toggle hidden files ───────────────────────────────────────────────────────
Type "."
Sleep 700ms
Type "."
Sleep 500ms

# ── Cycle all the way through once more quickly ───────────────────────────────
Sleep 400ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 700ms

# ── Dismiss ───────────────────────────────────────────────────────────────────
Sleep 400ms
Escape
Sleep 1s
