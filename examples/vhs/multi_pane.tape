# VHS tape for dynamic multi-pane demo
# Demonstrates: Ctrl+T pane creation, independent navigation across 3/4 panes,
# Tab/Shift+Tab focus cycling, single-pane focus, and Ctrl+W pane closing.

Output examples/vhs/generated/multi_pane.gif

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
Type "rm -rf /tmp/tfe-multi-pane-demo /tmp/tfe-multi-pane-config && mkdir -p /tmp/tfe-multi-pane-demo/{01-alpha,02-beta,03-gamma} && touch /tmp/tfe-multi-pane-demo/01-alpha/{alpha-one.txt,alpha-two.rs} /tmp/tfe-multi-pane-demo/02-beta/{beta-one.md,beta-two.toml} /tmp/tfe-multi-pane-demo/03-gamma/{gamma-one.json,gamma-two.txt}"
Enter
Type "XDG_CONFIG_HOME=/tmp/tfe-multi-pane-config target/debug/tfe --no-cd /tmp/tfe-multi-pane-demo"
Enter
Sleep 4s
Show

# Start with default two panes, then open a third pane after active pane.
Sleep 700ms
Ctrl+T
Sleep 1200ms

# Third pane owns independent location: enter 01-alpha.
Enter
Sleep 900ms
Down
Sleep 400ms

# Cycle forward to next pane and enter 02-beta.
Tab
Sleep 800ms
Down
Sleep 300ms
Enter
Sleep 900ms
Down
Sleep 400ms

# Wrap to first pane and enter 03-gamma.
Tab
Sleep 800ms
Down
Sleep 250ms
Down
Sleep 250ms
Enter
Sleep 900ms

# Cycle backward to prove Shift+Tab support.
Shift+Tab
Sleep 900ms

# Open fourth pane, cloned from active pane's directory.
Ctrl+T
Sleep 1200ms
Down
Sleep 400ms

# Close fourth pane; focus returns to previous surviving pane.
Ctrl+W
Sleep 1000ms

# Focus active pane full-width, then restore all three panes.
Type "w"
Sleep 1000ms
Up
Sleep 350ms
Type "w"
Sleep 1100ms

# Close panes down to one.
Ctrl+W
Sleep 900ms
Ctrl+W
Sleep 900ms

# Last pane cannot be closed; status explains invariant.
Ctrl+W
Sleep 1000ms

# Add pane again from current location.
Ctrl+T
Sleep 1100ms
Tab
Sleep 700ms
Shift+Tab
Sleep 700ms

Escape
Sleep 1s
