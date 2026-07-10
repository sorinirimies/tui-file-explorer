# VHS tape for image/picture preview demo
# Demonstrates: preview panel with image files, switching between text and image
# previews, preview scrolling (Ctrl+J / Ctrl+K), and navigating between images.
# Uses real files from ~/Desktop for authentic previews.

Output examples/vhs/generated/picture_preview.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 800
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

# Pre-build so the launch is instant
Hide
Type "cargo build --release 2>/dev/null"
Enter
Sleep 10s

# Launch tfe pointing at ~/Desktop
Type "cargo run --release -- ~/Desktop/"
Enter
Sleep 4s
Show

# Pause to show the file listing
Sleep 1s

# Toggle preview panel with P
Type "P"
Sleep 1s

# Browse through files — preview updates automatically
Down
Sleep 600ms
Down
Sleep 600ms
Down
Sleep 600ms

# Show the preview for a moment
Sleep 1s

# Navigate to more files
Down
Sleep 600ms
Down
Sleep 600ms

# Scroll the preview down with Ctrl+J
Sleep 500ms
Ctrl+J
Sleep 400ms
Ctrl+J
Sleep 400ms
Ctrl+J
Sleep 400ms

# Scroll back up with Ctrl+K
Ctrl+K
Sleep 400ms
Ctrl+K
Sleep 400ms
Ctrl+K
Sleep 400ms

# Continue browsing
Down
Sleep 600ms
Down
Sleep 600ms
Sleep 1s

# Go back up through the files
Up
Sleep 600ms
Up
Sleep 600ms
Up
Sleep 600ms
Up
Sleep 600ms
Sleep 1s

# Back down to another file
Down
Sleep 600ms
Down
Sleep 600ms
Down
Sleep 600ms
Sleep 1s

# Close the preview panel
Type "P"
Sleep 700ms

# Exit
Escape
Sleep 1s
