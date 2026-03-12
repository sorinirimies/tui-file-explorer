# VHS tape for create_entries demo
# Demonstrates: new folder (n), new file (N), cancel with Esc,
# nested path creation, and cursor landing on the new entry.
#
# Run with: vhs examples/vhs/create_entries.tape

Output examples/vhs/generated/create_entries.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example create_entries"
Enter
Sleep 4s

# ── Show the default listing briefly ─────────────────────────────────────────

Sleep 600ms
Down
Sleep 200ms
Down
Sleep 200ms
Type "g"
Sleep 500ms

# ── New folder — activate with n ─────────────────────────────────────────────

Sleep 600ms
Type "n"
Sleep 700ms

# Type the folder name character by character so the input bar is visible
Type "m"
Sleep 200ms
Type "y"
Sleep 200ms
Type "_"
Sleep 200ms
Type "p"
Sleep 200ms
Type "r"
Sleep 200ms
Type "o"
Sleep 200ms
Type "j"
Sleep 200ms
Type "e"
Sleep 200ms
Type "c"
Sleep 200ms
Type "t"
Sleep 700ms

# Confirm — folder is created, cursor jumps to it
Enter
Sleep 1200ms

# ── New file — activate with N ────────────────────────────────────────────────

Sleep 600ms
Type "N"
Sleep 700ms

# Type the file name
Type "R"
Sleep 200ms
Type "E"
Sleep 200ms
Type "A"
Sleep 200ms
Type "D"
Sleep 200ms
Type "M"
Sleep 200ms
Type "E"
Sleep 200ms
Type "."
Sleep 200ms
Type "m"
Sleep 200ms
Type "d"
Sleep 800ms

# Confirm — file is created, cursor jumps to it
Enter
Sleep 1200ms

# ── New file with an extension ────────────────────────────────────────────────

Sleep 500ms
Type "N"
Sleep 600ms

Type "m"
Sleep 150ms
Type "a"
Sleep 150ms
Type "i"
Sleep 150ms
Type "n"
Sleep 150ms
Type "."
Sleep 150ms
Type "r"
Sleep 150ms
Type "s"
Sleep 800ms

Enter
Sleep 1200ms

# ── Cancel mid-way — press Esc without confirming ────────────────────────────

Sleep 600ms
Type "n"
Sleep 600ms

Type "t"
Sleep 150ms
Type "e"
Sleep 150ms
Type "m"
Sleep 150ms
Type "p"
Sleep 700ms

# Esc cancels — nothing is created
Escape
Sleep 1000ms

# ── Same cancel demo for touch mode ──────────────────────────────────────────

Sleep 500ms
Type "N"
Sleep 600ms

Type "d"
Sleep 150ms
Type "r"
Sleep 150ms
Type "a"
Sleep 150ms
Type "f"
Sleep 150ms
Type "t"
Sleep 700ms

Escape
Sleep 1000ms

# ── Backspace correction — type a wrong char then fix it ─────────────────────

Sleep 500ms
Type "n"
Sleep 600ms

Type "f"
Sleep 150ms
Type "o"
Sleep 150ms
Type "o"
Sleep 150ms
Type "b"
Sleep 150ms
Type "a"
Sleep 150ms
Type "r"
Sleep 600ms

# Oops — delete the last three chars
Backspace
Sleep 250ms
Backspace
Sleep 250ms
Backspace
Sleep 400ms

# Retype correctly
Type "l"
Sleep 150ms
Type "d"
Sleep 150ms
Type "e"
Sleep 150ms
Type "r"
Sleep 800ms

Enter
Sleep 1200ms

# ── Nested path — folder hierarchy in one go ─────────────────────────────────

Sleep 600ms
Type "n"
Sleep 600ms

Type "s"
Sleep 150ms
Type "r"
Sleep 150ms
Type "c"
Sleep 150ms
Type "/"
Sleep 150ms
Type "l"
Sleep 150ms
Type "i"
Sleep 150ms
Type "b"
Sleep 800ms

Enter
Sleep 1200ms

# ── Nested file path ─────────────────────────────────────────────────────────

Sleep 500ms
Type "N"
Sleep 600ms

Type "d"
Sleep 150ms
Type "o"
Sleep 150ms
Type "c"
Sleep 150ms
Type "s"
Sleep 150ms
Type "/"
Sleep 150ms
Type "g"
Sleep 150ms
Type "u"
Sleep 150ms
Type "i"
Sleep 150ms
Type "d"
Sleep 150ms
Type "e"
Sleep 150ms
Type "."
Sleep 150ms
Type "m"
Sleep 150ms
Type "d"
Sleep 800ms

Enter
Sleep 1200ms

# ── Navigate to show the created entries ─────────────────────────────────────

Sleep 500ms
Type "g"
Sleep 400ms
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

# ── Quit ─────────────────────────────────────────────────────────────────────

Sleep 600ms
Type "q"
Sleep 1s
