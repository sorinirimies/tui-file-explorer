# =============================================================================
# sort.tape — VHS recording showcasing the sort mode cycling feature
#
# Demonstrates:
#   - Default sort: Name (A → Z), dirs first
#   - Press s once  → Size ↓ (largest files first)
#   - Press s again → Extension (grouped by file extension, then name)
#   - Press s again → back to Name
#   - set_sort_mode works together with extension filter
#   - Footer right panel shows the active sort label at all times
#
# Prerequisites:
#   cargo build --example basic
#   vhs examples/vhs/sort.tape
#
# Output: examples/vhs/generated/sort.gif
# =============================================================================

Output examples/vhs/generated/sort.gif

Set Shell         "bash"
Set FontSize      14
Set FontFamily    "JetBrains Mono"
Set Width         1200
Set Height        700
Set Theme         "Dracula"
Set Padding       16
Set PlaybackSpeed 1.0

# ── Launch (no extension filter — show all files so sort differences are clear)
Type "cargo run --example basic"
Enter
Sleep 3s

# ── Default state: Name sort — scan the full listing ─────────────────────────
Sleep 600ms
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
Type "g"
Sleep 600ms

# ── Switch to Size ↓ sort (largest first) ────────────────────────────────────
Sleep 500ms
Type "s"
Sleep 900ms

# ── Scroll through the size-sorted listing ────────────────────────────────────
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
Down
Sleep 200ms
Type "g"
Sleep 600ms

# ── Switch to Extension sort (grouped by ext, then name) ─────────────────────
Sleep 500ms
Type "s"
Sleep 900ms

# ── Scroll through the extension-sorted listing ───────────────────────────────
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
Down
Sleep 200ms
Type "g"
Sleep 600ms

# ── Cycle back to Name sort ───────────────────────────────────────────────────
Sleep 500ms
Type "s"
Sleep 800ms

# ── Descend into src/ and show sort applies inside subdirectories too ─────────
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Enter
Sleep 700ms

# ── Sort by extension inside src/ ─────────────────────────────────────────────
Sleep 400ms
Type "s"
Sleep 400ms
Type "s"
Sleep 800ms

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

# ── Cycle back to Name inside src/ ────────────────────────────────────────────
Sleep 400ms
Type "s"
Sleep 700ms

# ── Ascend back to root ───────────────────────────────────────────────────────
Backspace
Sleep 600ms

# ── Combine sort + search: sort by extension, then search for "rs" ───────────
Sleep 400ms
Type "s"
Sleep 400ms
Type "s"
Sleep 600ms
Type "/"
Sleep 400ms
Type "r"
Sleep 300ms
Type "s"
Sleep 700ms

# ── Clear search with Esc (sort mode stays) ───────────────────────────────────
Sleep 400ms
Escape
Sleep 600ms

# ── Reset to Name sort ────────────────────────────────────────────────────────
Sleep 400ms
Type "s"
Sleep 700ms

# ── Dismiss ───────────────────────────────────────────────────────────────────
Sleep 400ms
Escape
Sleep 1s
