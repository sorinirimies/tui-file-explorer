# =============================================================================
# file_ops.tape — VHS recording showcasing two-pane file operations
#
# Demonstrates:
#   - Two-pane layout (left / right, Tab to switch focus)
#   - Yank (y) — mark a file for copy
#   - Cut  (x) — mark a file for move
#   - Paste (p) — copy/move clipboard into the other pane's directory
#   - Overwrite confirmation modal (paste onto an existing file)
#   - Delete (d) — remove a file with confirmation
#   - Status bar reflecting clipboard state throughout
#
# Prerequisites:
#   cargo build --example basic
#   vhs examples/vhs/file_ops.tape
#
# Output: examples/vhs/generated/file_ops.gif
# =============================================================================

Output examples/vhs/generated/file_ops.gif

Set Shell         "bash"
Set FontSize      14
Set FontFamily    "JetBrains Mono"
Set Width         1400
Set Height        900
Set Theme         "Dracula"
Set Padding       16
Set PlaybackSpeed 1.0

# ── Launch in two-pane mode (default) ────────────────────────────────────────
Type "cargo run --example basic"
Enter
Sleep 3s

# ── Orient: left pane is active, browse to src/ ──────────────────────────────
Sleep 500ms
Down
Sleep 200ms
Down
Sleep 200ms
Enter
Sleep 700ms

# ── Navigate to a .rs file to yank ───────────────────────────────────────────
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Yank (copy) the highlighted file ─────────────────────────────────────────
Sleep 600ms
Type "y"
Sleep 800ms

# ── Switch to right pane and navigate to examples/ ───────────────────────────
Sleep 400ms
Tab
Sleep 500ms
Down
Sleep 200ms
Enter
Sleep 700ms

# ── Paste into examples/ ─────────────────────────────────────────────────────
Sleep 500ms
Type "p"
Sleep 900ms

# ── Switch back to left pane ──────────────────────────────────────────────────
Sleep 400ms
Tab
Sleep 400ms

# ── Ascend to root ────────────────────────────────────────────────────────────
Backspace
Sleep 500ms

# ── Cut a file this time ──────────────────────────────────────────────────────
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Type "x"
Sleep 800ms

# ── Switch to right pane (still in examples/) ────────────────────────────────
Sleep 400ms
Tab
Sleep 400ms

# ── Paste (move) — triggers overwrite confirmation since file already exists ──
Sleep 400ms
Type "p"
Sleep 900ms

# ── Confirm overwrite with y ──────────────────────────────────────────────────
Sleep 600ms
Type "y"
Sleep 900ms

# ── Switch back to left pane ──────────────────────────────────────────────────
Sleep 400ms
Tab
Sleep 400ms

# ── Navigate to a file to delete ─────────────────────────────────────────────
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Initiate delete — confirmation modal appears ──────────────────────────────
Sleep 600ms
Type "d"
Sleep 900ms

# ── Cancel delete with n ──────────────────────────────────────────────────────
Sleep 500ms
Type "n"
Sleep 700ms

# ── Delete again and confirm with y ──────────────────────────────────────────
Sleep 400ms
Type "d"
Sleep 800ms
Type "y"
Sleep 900ms

# ── Toggle single-pane mode to review the result ─────────────────────────────
Sleep 500ms
Type "w"
Sleep 700ms

# ── Scroll through the listing ───────────────────────────────────────────────
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Type "g"
Sleep 500ms

# ── Restore two-pane mode ─────────────────────────────────────────────────────
Sleep 400ms
Type "w"
Sleep 700ms

# ── Dismiss ───────────────────────────────────────────────────────────────────
Sleep 400ms
Escape
Sleep 1s
