# VHS tape for theme_switcher demo (live theme cycling across all 43 palettes)

Output examples/vhs/generated/theme_switcher.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 750
Set PlaybackSpeed 1.5
Set Theme "Catppuccin Mocha"

Hide
Type "cargo build --example theme_switcher 2>/dev/null"
Enter
Sleep 10s
Type "cargo run --example theme_switcher"
Enter
Sleep 3s
Show

# Navigate the file list a little in the default theme
Sleep 500ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Cycle forward through all 43 themes
# 1 Default -> 2 Default Light
Tab
Sleep 500ms
Down
Sleep 150ms

# 2 -> 3 Grape
Tab
Sleep 500ms
Up
Sleep 150ms

# 3 -> 4 Ocean
Tab
Sleep 500ms
Down
Sleep 150ms

# 4 -> 5 Sunset
Tab
Sleep 500ms

# 5 -> 6 Forest
Tab
Sleep 500ms
Down
Sleep 150ms

# 6 -> 7 Rose
Tab
Sleep 500ms

# 7 -> 8 Mono
Tab
Sleep 500ms

# 8 -> 9 Neon
Tab
Sleep 500ms
Up
Sleep 150ms

# 9 -> 10 Dracula
Tab
Sleep 500ms
Down
Sleep 150ms

# 10 -> 11 Nord
Tab
Sleep 500ms

# 11 -> 12 Solarized Dark
Tab
Sleep 500ms

# 12 -> 13 Solarized Light
Tab
Sleep 500ms
Down
Sleep 150ms

# 13 -> 14 Gruvbox Dark
Tab
Sleep 500ms

# 14 -> 15 Gruvbox Light
Tab
Sleep 500ms

# 15 -> 16 Catppuccin Latte
Tab
Sleep 500ms
Up
Sleep 150ms

# 16 -> 17 Catppuccin Frappé
Tab
Sleep 500ms

# 17 -> 18 Catppuccin Macchiato
Tab
Sleep 500ms

# 18 -> 19 Catppuccin Mocha
Tab
Sleep 500ms
Down
Sleep 150ms

# 19 -> 20 Tokyo Night
Tab
Sleep 500ms

# 20 -> 21 Tokyo Night Storm
Tab
Sleep 500ms

# 21 -> 22 Tokyo Night Light
Tab
Sleep 500ms

# 22 -> 23 Kanagawa Wave
Tab
Sleep 500ms
Up
Sleep 150ms

# 23 -> 24 Kanagawa Dragon
Tab
Sleep 500ms

# 24 -> 25 Kanagawa Lotus
Tab
Sleep 500ms

# 25 -> 26 Moonfly
Tab
Sleep 500ms
Down
Sleep 150ms

# 26 -> 27 Nightfly
Tab
Sleep 500ms

# 27 -> 28 Oxocarbon
Tab
Sleep 500ms

# 28 -> 29 Cyberpunk
Tab
Sleep 500ms

# 29 -> 30 Rosé Pine
Tab
Sleep 500ms
Up
Sleep 150ms

# 30 -> 31 Rosé Pine Moon
Tab
Sleep 500ms

# 31 -> 32 Rosé Pine Dawn
Tab
Sleep 500ms

# 32 -> 33 Ayu Mirage
Tab
Sleep 500ms
Down
Sleep 150ms

# 33 -> 34 Everforest Dark
Tab
Sleep 500ms

# 34 -> 35 Atom One Dark
Tab
Sleep 500ms

# 35 -> 36 Atom One Light
Tab
Sleep 500ms

# 36 -> 37 Night Owl
Tab
Sleep 500ms
Up
Sleep 150ms

# 37 -> 38 Poimandres
Tab
Sleep 500ms

# 38 -> 39 Flexoki Dark
Tab
Sleep 500ms

# 39 -> 40 Flexoki Light
Tab
Sleep 500ms
Down
Sleep 150ms

# 40 -> 41 Carbonfox
Tab
Sleep 500ms

# 41 -> 42 Andromeda
Tab
Sleep 500ms

# 42 -> 43 Synthwave '84
Tab
Sleep 700ms

# Cycle backwards through a few to show Shift+Tab
Shift+Tab
Sleep 500ms
Shift+Tab
Sleep 500ms
Shift+Tab
Sleep 500ms

# Navigate in the file list
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

# Back and dismiss
Backspace
Sleep 500ms

Sleep 400ms
Escape
Sleep 1s
