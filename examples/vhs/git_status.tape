# VHS tape: git change markers
# v shows M / A / ? next to changed entries; folders show what is inside.
#
# Uses the throw-away tree from fixture.sh and an empty $HOME, so it never
# touches your real files or settings.

Output examples/vhs/generated/git_status.gif

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
Type "HOME=/tmp/tfe-demo-home target/debug/tfe --single-pane --no-trash /tmp/tfe-demo/repo"
Enter
Sleep 3s
Show

Sleep 2s
Type "v"
Sleep 3s
Enter
Sleep 2.5s
Type "h"
Sleep 1.5s
Type "v"
Sleep 2s
Escape
Sleep 1s
