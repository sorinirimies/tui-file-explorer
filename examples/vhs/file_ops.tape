# VHS tape for file_ops demo (copy, cut, paste, delete across two panes)

Output examples/vhs/generated/file_ops.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 900
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --bin tfe"
Enter
Sleep 3s

# Left pane is active — navigate down to a file
Sleep 500ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Yank (copy) the highlighted file
Sleep 600ms
Type "y"
Sleep 800ms

# Switch to the right pane
Tab
Sleep 500ms

# Navigate into a subdirectory as the paste destination
Down
Sleep 200ms
Enter
Sleep 700ms

# Paste the copied file into this directory
Sleep 500ms
Type "p"
Sleep 900ms

# Switch back to the left pane
Tab
Sleep 500ms

# Navigate to a different file and cut it
Type "g"
Sleep 300ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Type "x"
Sleep 800ms

# Switch to right pane and paste (move)
Tab
Sleep 400ms
Type "p"
Sleep 900ms

# Paste again on same destination — triggers overwrite confirmation
Sleep 500ms
Tab
Sleep 300ms
Type "y"
Sleep 400ms
Tab
Sleep 400ms
Type "p"
Sleep 900ms

# Confirm overwrite with y
Type "y"
Sleep 900ms

# Switch back to left pane and delete a file
Tab
Sleep 500ms
Type "g"
Sleep 300ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Type "d"
Sleep 900ms

# Cancel the delete with n
Type "n"
Sleep 700ms

# Delete again and confirm
Type "d"
Sleep 800ms
Type "y"
Sleep 900ms

# Toggle single-pane to review the result
Type "w"
Sleep 700ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Type "g"
Sleep 500ms

# Restore two-pane layout
Type "w"
Sleep 700ms

Sleep 400ms
Escape
Sleep 1s
