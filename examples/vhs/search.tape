# VHS tape for search demo (incremental search lifecycle)

Output examples/vhs/generated/search.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example basic"
Enter
Sleep 3s

# Show the default listing briefly
Sleep 800ms
Down
Sleep 200ms
Down
Sleep 200ms
Type "g"
Sleep 500ms

# Activate search with /
Sleep 500ms
Type "/"
Sleep 700ms

# Type a query that progressively narrows the list
Type "s"
Sleep 400ms
Type "r"
Sleep 400ms
Type "c"
Sleep 800ms

# Descend into the sole result
Sleep 300ms
Enter
Sleep 800ms

# Search inside src/ for explorer
Sleep 300ms
Type "/"
Sleep 300ms
Type "e"
Sleep 300ms
Type "x"
Sleep 300ms
Type "p"
Sleep 700ms

# Backspace: remove one character at a time
Sleep 400ms
Backspace
Sleep 400ms
Backspace
Sleep 400ms
Backspace
Sleep 600ms

# Empty query — one more Backspace exits search
Sleep 400ms
Backspace
Sleep 600ms

# Ascend back to root
Backspace
Sleep 600ms

# Activate search again, type a query, then clear with Esc
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

# Esc clears the query and deactivates search
Sleep 400ms
Escape
Sleep 700ms

# Second Esc dismisses the explorer
Sleep 400ms
Escape
Sleep 1s
