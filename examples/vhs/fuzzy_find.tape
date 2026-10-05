# VHS tape: recursive fuzzy finder
# f searches the whole tree as you type; Enter jumps to the match.
#
# Uses the throw-away tree from fixture.sh and an empty $HOME, so it never
# touches your real files or settings.

Output examples/vhs/generated/fuzzy_find.gif

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
Type "f"
Sleep 1.5s
Type "main"
Sleep 2s
Down
Sleep 800ms
Up
Sleep 800ms
Enter
Sleep 2s

# Back to the top of the tree, then search for a markdown file
Type "h"
Sleep 1s
Type "f"
Sleep 1s
Type "chap"
Sleep 2s
Down
Sleep 1s
Enter
Sleep 2.5s
Escape
Sleep 1s
