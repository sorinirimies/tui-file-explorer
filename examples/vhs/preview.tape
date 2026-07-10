# VHS tape for preview & inline editor demo
# Demonstrates: preview panel toggle (P), automatic preview updates on navigation,
# preview scrolling (Ctrl+J / Ctrl+K), inline editor (i), and editor navigation.

Output examples/vhs/generated/preview.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 750
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

# Launch tfe pointing at ~/Desktop for a mix of text and image files
Hide
Type "cargo build --release 2>/dev/null"
Enter
Sleep 10s
Type "cargo run --release -- ~/Desktop/"
Enter
Sleep 4s
Show

# Browse through files

# Toggle preview panel with P
Sleep 400ms
Type "P"
Sleep 1s

# Browse through files — preview updates automatically
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Up
Sleep 500ms
Up
Sleep 500ms

# Scroll preview with Ctrl+J
Sleep 400ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms
Ctrl+J
Sleep 300ms

# Scroll back up with Ctrl+K
Ctrl+K
Sleep 300ms
Ctrl+K
Sleep 300ms

# Open inline editor with i
Sleep 500ms
Type "i"
Sleep 1s

# Navigate in the editor
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Right
Sleep 200ms
Right
Sleep 200ms

# Exit editor with Esc
Sleep 500ms
Escape
Sleep 700ms

# Turn off preview
Type "P"
Sleep 500ms

# Dismiss
Escape
Sleep 1s
