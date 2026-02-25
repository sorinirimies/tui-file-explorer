# VHS tape for sort demo (Name → Size ↓ → Extension cycling)

Output examples/vhs/generated/sort.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example basic"
Enter
Sleep 3s

# Default Name sort — scroll through the listing
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

# Switch to Size ↓ sort (largest first)
Sleep 500ms
Type "s"
Sleep 900ms

# Scroll through the size-sorted listing
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

# Switch to Extension sort (grouped by ext, then name)
Sleep 500ms
Type "s"
Sleep 900ms

# Scroll through the extension-sorted listing
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

# Cycle back to Name sort
Sleep 500ms
Type "s"
Sleep 800ms

# Descend into src/ — sort applies inside subdirectories too
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Enter
Sleep 700ms

# Jump straight to Extension sort inside src/
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

# Cycle back to Name inside src/
Sleep 400ms
Type "s"
Sleep 700ms

# Ascend back to root
Backspace
Sleep 600ms

# Combine sort + search: Extension sort then search for "rs"
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

# Esc clears search (sort mode stays active)
Sleep 400ms
Escape
Sleep 600ms

# Reset to Name sort
Sleep 400ms
Type "s"
Sleep 700ms

Sleep 400ms
Escape
Sleep 1s
