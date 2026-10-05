# VHS tape: bulk rename
# Mark entries, Shift+R, edit all the names at once, save and quit.
#
# Uses the throw-away tree from fixture.sh and an empty $HOME, so it never
# touches your real files or settings.

Output examples/vhs/generated/bulk_rename.gif

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
# Go into docs/ (3rd row) and mark the three chapters
Type "g"
Down 2
Sleep 600ms
Enter
Sleep 1.5s
# Space marks an entry and moves down
Type " "
Sleep 400ms
Type " "
Sleep 400ms
Type " "
Sleep 1s

# Edit all names at once: prefix each with a number
Type "R"
Sleep 2s
Type "01-"
Sleep 700ms
Down
Sleep 400ms
Left 3
Sleep 300ms
Type "02-"
Sleep 700ms
Down
Sleep 400ms
Left 3
Sleep 300ms
Type "03-"
Sleep 1.5s
Ctrl+S
Sleep 800ms
Escape
Sleep 2.5s

# One undo reverts the whole batch
Type "u"
Sleep 2.5s
Escape
Sleep 1s
