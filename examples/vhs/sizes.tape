# VHS tape: size info on / off
# z hides or shows all sizes; Shift+Z swaps recursive folder sizes for plain item counts.
#
# Uses the throw-away tree from fixture.sh and an empty $HOME, so it never
# touches your real files or settings.

Output examples/vhs/generated/sizes.gif

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

Sleep 2s

# z: hide every size, then bring them back
Type "z"
Sleep 2s
Type "z"
Sleep 2s

# Shift+Z: files keep their size, folders fall back to cheap item counts
Type "Z"
Sleep 2.5s
Type "Z"
Sleep 2.5s

# Open the options panel to see both switches
Type "O"
Sleep 3s
Type "O"
Sleep 1s
Escape
Sleep 1s
