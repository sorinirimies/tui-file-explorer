# VHS tape for dual_pane demo
# Demonstrates: DualPane library widget — Tab focus-switch, w single/dual toggle,
# independent pane navigation, status bar updates, search, sort, dismiss.

Output examples/vhs/generated/dual_pane.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 900
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example dual_pane"
Enter
Sleep 4s

# ── Left pane is active by default ───────────────────────────────────────────

Sleep 500ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Switch focus to the right pane with Tab ───────────────────────────────────

Sleep 700ms
Tab
Sleep 900ms

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
Sleep 700ms
Tab
Sleep 900ms

# Continue navigating the left pane
Up
Sleep 200ms
Up
Sleep 200ms
Down
Sleep 200ms

# Descend into src/ on the left pane while the right pane stays at root
Sleep 500ms
Down
Sleep 200ms
Down
Sleep 200ms
Enter
Sleep 800ms

# Navigate inside src/
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Switch to the right pane — it has not moved from root
Sleep 700ms
Tab
Sleep 900ms

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Switch back to the left pane (still inside src/)
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Up
Sleep 200ms

# ── Incremental search in the left pane (src/) ───────────────────────────────

Sleep 500ms
Type "/"
Sleep 500ms
Type "a"
Sleep 350ms
Type "p"
Sleep 350ms
Type "p"
Sleep 800ms

# Clear the search with Esc
Sleep 500ms
Escape
Sleep 700ms

# Ascend back to root
Backspace
Sleep 700ms

# ── Sort mode cycle on the left pane ─────────────────────────────────────────

Sleep 400ms
Type "s"
Sleep 800ms
Type "s"
Sleep 800ms
Type "s"
Sleep 700ms

# ── Toggle hidden files on then off ──────────────────────────────────────────

Sleep 400ms
Type "."
Sleep 700ms
Type "."
Sleep 600ms

# ── Collapse to single-pane with w ───────────────────────────────────────────

Sleep 700ms
Type "w"
Sleep 1000ms

# Active pane (left) fills the full width; navigate freely
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Up
Sleep 200ms

# Search inside single-pane mode
Sleep 400ms
Type "/"
Sleep 400ms
Type "e"
Sleep 350ms
Type "x"
Sleep 350ms
Type "a"
Sleep 800ms
Escape
Sleep 600ms

# ── Restore two-pane layout with w ───────────────────────────────────────────

Sleep 700ms
Type "w"
Sleep 1000ms

# Right pane reappears with its cursor position preserved
Sleep 400ms
Tab
Sleep 700ms

Up
Sleep 200ms
Up
Sleep 200ms
Down
Sleep 200ms

# Switch back to the left pane
Sleep 500ms
Tab
Sleep 700ms

Down
Sleep 200ms
Down
Sleep 200ms

# ── Jump to top, then descend and select a file ──────────────────────────────

Sleep 400ms
Type "g"
Sleep 400ms
Down
Sleep 200ms
Enter
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Select a file — prints path to stdout and exits
Sleep 600ms
Enter
Sleep 1s
