# VHS tape: archive preview and extract
# The preview lists archive contents; Shift+X extracts into a sibling folder.
#
# Uses the throw-away tree from fixture.sh and an empty $HOME, so it never
# touches your real files or settings.

Output examples/vhs/generated/archives.gif

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
Type "P"
Sleep 1s
# Into archives/: the preview lists the zip's contents
Type "g"
Enter
Sleep 2.5s
# …and the tarball's
Down
Sleep 2.5s
Up
Sleep 1s

# Extract the zip next to itself, look inside, then undo
Type "X"
Sleep 3s
Down
Sleep 800ms
Up
Sleep 1.5s
Type "u"
Sleep 2.5s
Escape
Sleep 1s
