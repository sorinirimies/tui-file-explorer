# VHS tape for theme_switcher demo (live theme cycling across all 27 palettes)

Output examples/vhs/generated/theme_switcher.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 750
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example theme_switcher"
Enter
Sleep 3s

# Navigate the file list a little in the default theme
Sleep 500ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Cycle forward through several themes
Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms

Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Up
Sleep 200ms

Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms

Sleep 600ms
Tab
Sleep 800ms

Up
Sleep 200ms
Up
Sleep 200ms
Down
Sleep 200ms

Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms

Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Up
Sleep 200ms

Sleep 600ms
Tab
Sleep 800ms

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Cycle backwards with Shift+Tab
Sleep 600ms
Shift+Tab
Sleep 700ms
Shift+Tab
Sleep 700ms
Shift+Tab
Sleep 700ms

# Descend into src/ in this theme
Sleep 400ms
Type "g"
Sleep 300ms
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

# Select a .rs file
Sleep 400ms
Enter
Sleep 800ms

# Ascend and do a quick sweep through a few more themes
Backspace
Sleep 500ms
Tab
Sleep 400ms
Tab
Sleep 400ms
Tab
Sleep 800ms

# Toggle hidden files
Type "."
Sleep 700ms
Type "."
Sleep 500ms

# Rapid cycle through all remaining themes
Sleep 400ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 350ms
Tab
Sleep 700ms

Sleep 400ms
Escape
Sleep 1s
