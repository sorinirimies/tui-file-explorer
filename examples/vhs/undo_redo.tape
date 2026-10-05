# VHS tape: undo and redo
# u undoes the last create / rename; Ctrl+R redoes it.
#
# Uses the throw-away tree from fixture.sh and an empty $HOME, so it never
# touches your real files or settings.

Output examples/vhs/generated/undo_redo.gif

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
# Create a folder and a file
Type "n"
Sleep 600ms
Type "scratch"
Sleep 800ms
Enter
Sleep 1.5s
Type "N"
Sleep 600ms
Type "todo.txt"
Sleep 800ms
Enter
Sleep 1.5s

# Undo both…
Type "u"
Sleep 2s
Type "u"
Sleep 2s

# …and redo them
Ctrl+R
Sleep 2s
Ctrl+R
Sleep 2.5s
Escape
Sleep 1s
