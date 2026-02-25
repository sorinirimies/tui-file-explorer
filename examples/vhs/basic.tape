# VHS tape for basic demo (navigation, search, sort, hidden-file toggle, selection)

Output examples/vhs/generated/basic.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example basic"
Enter
Sleep 3s

# Basic navigation
Sleep 400ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Up
Sleep 200ms
Up
Sleep 200ms

# Jump to bottom then back to top
Sleep 300ms
Type "G"
Sleep 500ms
Type "g"
Sleep 400ms

# Descend into src/
Down
Sleep 200ms
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

# Ascend back to root
Sleep 400ms
Backspace
Sleep 600ms

# Incremental search — activate with /, filter live
Sleep 500ms
Type "/"
Sleep 400ms
Type "s"
Sleep 300ms
Type "r"
Sleep 300ms
Type "c"
Sleep 700ms

# Descend into the filtered result
Enter
Sleep 700ms

# Search inside src/ for .rs files
Sleep 300ms
Type "/"
Sleep 300ms
Type "r"
Sleep 300ms
Type "s"
Sleep 700ms

# Clear search with Esc
Sleep 400ms
Escape
Sleep 500ms

# Ascend back to root
Backspace
Sleep 600ms

# Cycle sort modes: Name → Size ↓ → Extension → Name
Sleep 400ms
Type "s"
Sleep 700ms
Type "s"
Sleep 700ms
Type "s"
Sleep 600ms

# Toggle hidden files on then off
Sleep 300ms
Type "."
Sleep 600ms
Type "."
Sleep 500ms

# Select a file
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

Sleep 500ms
Escape
Sleep 1s
