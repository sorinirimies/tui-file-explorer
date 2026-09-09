# VHS tape for multi-pane file operations demo
# Demonstrates: marks sourced from one pane and copied into two other panes.

Output examples/vhs/generated/multi_pane_file_ops.gif

Set Shell "bash"
Set FontSize 14
Set Width 1600
Set Height 900
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Hide
Type "cargo build --bin tfe 2>/dev/null"
Enter
Sleep 10s
Type "rm -rf /tmp/tfe-multi-pane-ops /tmp/tfe-multi-pane-ops-config && mkdir -p /tmp/tfe-multi-pane-ops/{01-inbox,02-review,03-archive} && printf 'alpha demo\n' > /tmp/tfe-multi-pane-ops/01-inbox/alpha.txt && printf 'beta demo\n' > /tmp/tfe-multi-pane-ops/01-inbox/beta.txt"
Enter
Type "XDG_CONFIG_HOME=/tmp/tfe-multi-pane-ops-config target/debug/tfe --no-cd /tmp/tfe-multi-pane-ops"
Enter
Sleep 4s
Show

# First pane: enter inbox and mark both files.
Sleep 600ms
Enter
Sleep 800ms
Space
Sleep 500ms
Space
Sleep 900ms

# Create third pane from inbox, ascend, then enter review directory.
Ctrl+T
Sleep 1000ms
Backspace
Sleep 700ms
Down
Sleep 300ms
Enter
Sleep 900ms

# Next pane enters archive directory.
Tab
Sleep 800ms
Down
Sleep 250ms
Down
Sleep 250ms
Enter
Sleep 900ms

# Paste discovers marks in inactive inbox pane and copies both files here.
Type "p"
Sleep 1400ms

# Move backward to review pane and paste same clipboard there too.
Shift+Tab
Sleep 900ms
Type "p"
Sleep 1400ms

# Cycle across all panes to show independent source and destinations.
Shift+Tab
Sleep 800ms
Tab
Sleep 700ms
Tab
Sleep 700ms

Escape
Sleep 1s
