# VHS tape: bookmarks
# b bookmarks the current folder, Shift+B opens the list and jumps.
#
# Uses the throw-away tree from fixture.sh and an empty $HOME, so it never
# touches your real files or settings.

Output examples/vhs/generated/bookmarks.gif

Set Shell "bash"
Set FontSize 14
Set Width 1600
Set Height 900
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Hide
Type "cargo build --bin tfe 2>/dev/null && bash examples/vhs/fixture.sh"
Enter
Sleep 12s
Type "clear"
Enter
Type "HOME=/tmp/tfe-demo-home target/debug/tfe --single-pane --no-trash /tmp/tfe-demo"
Enter
Sleep 3s
Show

Sleep 1.5s
# Enter src/ (5th row) and bookmark it
Type "g"
Down 4
Sleep 800ms
Enter
Sleep 1s
Type "b"
Sleep 1.8s

# Back up, bookmark docs/ (3rd row) too
Type "h"
Sleep 1s
Type "g"
Down 2
Sleep 800ms
Enter
Sleep 1s
Type "b"
Sleep 1.8s
Type "h"
Sleep 1s

# Open the bookmark list and jump
Type "B"
Sleep 2s
Down
Sleep 1s
Enter
Sleep 2.5s
Escape
Sleep 1s
