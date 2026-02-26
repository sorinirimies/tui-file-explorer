# VHS tape for pane_toggle demo
# Demonstrates: Tab pane-switch, w single/two-pane toggle, T theme-panel open/close

Output examples/vhs/generated/pane_toggle.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 900
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --bin tfe"
Enter
Sleep 3s

# ── Left pane is active by default ───────────────────────────────────────────

Sleep 500ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Switch focus to the right pane with Tab ───────────────────────────────────

Sleep 600ms
Tab
Sleep 800ms

# Navigate the right pane independently — both panes track their own cursors
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Switch back to the left pane
Sleep 600ms
Tab
Sleep 800ms

# Navigate further in the left pane
Up
Sleep 200ms
Up
Sleep 200ms

# Descend into src/ in the left pane while the right pane stays at root
Sleep 400ms
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

# Switch to right pane — it has not moved
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms

# Switch back to left pane (inside src/)
Sleep 500ms
Tab
Sleep 600ms

# ── Collapse to single-pane with w ───────────────────────────────────────────

Sleep 600ms
Type "w"
Sleep 900ms

# The active pane (left, inside src/) fills the full width
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Up
Sleep 200ms

# Ascend back to root while in single-pane mode
Sleep 400ms
Backspace
Sleep 600ms

# Navigate in single-pane mode
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Restore two-pane layout with w ───────────────────────────────────────────

Sleep 600ms
Type "w"
Sleep 900ms

# Right pane reappears with its own cursor position preserved
Sleep 500ms
Tab
Sleep 600ms

Up
Sleep 200ms
Up
Sleep 200ms

Sleep 500ms
Tab
Sleep 600ms

# ── Open the theme panel sidebar with T ──────────────────────────────────────

Sleep 600ms
Type "T"
Sleep 900ms

# Cycle forward through a few themes with t
Sleep 400ms
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms
Type "t"
Sleep 700ms

# Cycle backward with [
Sleep 400ms
Type "["
Sleep 600ms
Type "["
Sleep 700ms

# The explorer panes remain navigable with the theme panel open
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Tab
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Tab
Sleep 500ms

# ── Close the theme panel with T ─────────────────────────────────────────────

Sleep 600ms
Type "T"
Sleep 900ms

# Panes expand back to fill the full width
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Collapse to single-pane once more, then open the theme panel ─────────────

Sleep 500ms
Type "w"
Sleep 700ms

Type "T"
Sleep 700ms

Type "t"
Sleep 500ms
Type "t"
Sleep 500ms
Type "t"
Sleep 700ms

# Close panel and restore two-pane in one sequence
Sleep 400ms
Type "T"
Sleep 500ms
Type "w"
Sleep 700ms

# ── Finish ────────────────────────────────────────────────────────────────────

Sleep 400ms
Escape
Sleep 1s
