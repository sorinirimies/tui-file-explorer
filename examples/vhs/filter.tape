# VHS tape for filter demo (extension filter — selectable vs dimmed files)

Output examples/vhs/generated/filter.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example basic -- rs toml"
Enter
Sleep 3s

# Footer shows the active filter (rs, toml)
Sleep 800ms

# Navigate — .rs/.toml files are highlighted, others are dimmed
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms

# Descend into src/ where .rs files are selectable
Type "g"
Sleep 300ms
Down
Sleep 200ms
Enter
Sleep 600ms

# .rs files are highlighted green
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms

# Select a matching .rs file
Sleep 300ms
Enter
Sleep 800ms

# Ascend back to root
Backspace
Sleep 500ms

# Navigate to Cargo.toml — also selectable
Type "g"
Sleep 300ms
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

# Select Cargo.toml
Sleep 300ms
Enter
Sleep 800ms

# Toggle hidden files on to show dot-files are also subject to the filter
Sleep 400ms
Type "."
Sleep 700ms
Type "."
Sleep 500ms

Sleep 400ms
Escape
Sleep 1s
