# =============================================================================
# search.tape — VHS recording showcasing the incremental search feature
#
# Demonstrates:
#   - Press / to activate search mode (footer turns into a live input bar)
#   - Typing filters the list in real-time (case-insensitive substring match)
#   - Backspace pops the last character from the query
#   - Extra Backspace on an empty query deactivates search
#   - Esc clears the query (first press) then dismisses (second press)
#   - Search is cleared automatically when navigating into a subdirectory
#
# Prerequisites:
#   cargo build --example basic
#   vhs examples/vhs/search.tape
#
# Output: examples/vhs/generated/search.gif
# =============================================================================

Output examples/vhs/generated/search.gif

Set Shell         "bash"
Set FontSize      14
Set FontFamily    "JetBrains Mono"
Set Width         1200
Set Height        700
Set Theme         "Dracula"
Set Padding       16
Set PlaybackSpeed 1.0

# ── Launch ────────────────────────────────────────────────────────────────────
Type "cargo run --example basic"
Enter
Sleep 3s

# ── Show the default unfiltered listing for a moment ─────────────────────────
Sleep 800ms
Down
Sleep 200ms
Down
Sleep 200ms
Type "g"
Sleep 500ms

# ── Activate search with / ────────────────────────────────────────────────────
Sleep 500ms
Type "/"
Sleep 700ms

# ── Type a query that progressively narrows the list ─────────────────────────
Type "s"
Sleep 400ms
Type "r"
Sleep 400ms
Type "c"
Sleep 800ms

# ── Navigate to the sole result and descend ───────────────────────────────────
Sleep 300ms
Enter
Sleep 800ms

# ── Search inside src/ for explorer ───────────────────────────────────────────
Sleep 300ms
Type "/"
Sleep 300ms
Type "e"
Sleep 300ms
Type "x"
Sleep 300ms
Type "p"
Sleep 700ms

# ── Backspace: remove one character at a time ─────────────────────────────────
Sleep 400ms
Backspace
Sleep 400ms
Backspace
Sleep 400ms
Backspace
Sleep 600ms

# ── Query is now empty but search is still active — one more Backspace exits ──
Sleep 400ms
Backspace
Sleep 600ms

# ── Search is cleared — listing is back to full ───────────────────────────────
Sleep 500ms

# ── Ascend back to root ───────────────────────────────────────────────────────
Backspace
Sleep 600ms

# ── Activate search again, type a query, then clear with Esc ─────────────────
Sleep 400ms
Type "/"
Sleep 400ms
Type "c"
Sleep 300ms
Type "a"
Sleep 300ms
Type "r"
Sleep 300ms
Type "g"
Sleep 700ms

# ── Esc clears the query and deactivates search (returns Pending, not Dismiss) ─
Sleep 400ms
Escape
Sleep 700ms

# ── List is restored; second Esc dismisses the explorer ───────────────────────
Sleep 400ms
Escape
Sleep 1s
