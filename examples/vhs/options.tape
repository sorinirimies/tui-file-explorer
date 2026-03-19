# VHS tape for options demo
# Demonstrates: options panel open/close, Shift+E editor picker, hidden-file
# toggle, sort cycling, single-pane toggle, theme cycling, "no editor" status
# message (both from e key and from Enter on a file), and Tab pane-switching.
#
# Example: cargo run --example options

Output examples/vhs/generated/options.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 900
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example options"
Enter
Sleep 4s

# ── Options panel is open by default — show it briefly ───────────────────────

Sleep 600ms

# ── Close the options panel so we start with a clean dual-pane view ──────────

Type "O"
Sleep 900ms

# ── Navigate down to a file in the left pane ─────────────────────────────────

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Press e with no editor set — status bar should appear with a message ─────

Sleep 500ms
Type "e"
Sleep 2000ms

# ── Press Enter on the highlighted file — also shows the "no editor" message ─

Sleep 400ms
Enter
Sleep 2000ms

# ── Open the options panel with Shift+O ──────────────────────────────────────

Sleep 400ms
Type "O"
Sleep 900ms

# ── Open the editor picker with Shift+E ──────────────────────────────────────

Sleep 400ms
Type "E"
Sleep 900ms

# Scroll down to highlight a few editors
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms

# Select Vim
Enter
Sleep 1000ms

# ── Close the options panel ───────────────────────────────────────────────────

Sleep 500ms
Type "O"
Sleep 700ms

# ── Now e opens the file in the configured editor (vim) ──────────────────────
# Navigate to a plain file and press e — with an editor set the status message
# shows "returned from vim — <filename>" after the editor exits.
# In a recording environment vim may not be installed so we just show the key.

Sleep 400ms
Type "g"
Sleep 300ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

Sleep 500ms
Type "e"
Sleep 1500ms

# ── Re-open options panel to show Shift+E picker again ───────────────────────

Sleep 400ms
Type "O"
Sleep 700ms

# ── Open editor picker and reset to None ─────────────────────────────────────

Sleep 400ms
Type "E"
Sleep 800ms

# Scroll back up to None (top of the list)
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 400ms

# Confirm None
Enter
Sleep 900ms

# ── Toggle hidden files with h (panel open) ───────────────────────────────────

Sleep 400ms
Type "h"
Sleep 800ms

# Toggle hidden back off
Type "h"
Sleep 700ms

# ── Cycle sort modes with s (panel open) ─────────────────────────────────────

Sleep 400ms
Type "s"
Sleep 800ms

# Size ↓ → Extension
Type "s"
Sleep 800ms

# Extension → Name
Type "s"
Sleep 700ms

# ── Toggle single-pane with w (panel open) ───────────────────────────────────

Sleep 400ms
Type "w"
Sleep 900ms

# Restore two-pane
Type "w"
Sleep 800ms

# ── Cycle theme with t (panel open) ──────────────────────────────────────────

Sleep 400ms
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms
Type "t"
Sleep 600ms

# ── Switch active pane with Tab while panel is open ──────────────────────────

Sleep 400ms
Tab
Sleep 800ms

# Navigate the right pane a little
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

Tab
Sleep 800ms

# ── Close the options panel ───────────────────────────────────────────────────

Sleep 500ms
Type "O"
Sleep 900ms

# ── Confirm e still shows the "no editor" message after reset ────────────────

Sleep 400ms
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

Sleep 500ms
Type "e"
Sleep 2000ms

# ── Switch panes and show independent navigation ─────────────────────────────

Sleep 400ms
Tab
Sleep 700ms

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

Sleep 400ms
Tab
Sleep 700ms

Up
Sleep 200ms
Up
Sleep 200ms

# ── Descend into src/ and navigate ───────────────────────────────────────────

Sleep 400ms
Type "g"
Sleep 300ms
Down
Sleep 200ms
Down
Sleep 200ms
Enter
Sleep 700ms

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# Ascend back to root
Backspace
Sleep 600ms

# ── Quit ─────────────────────────────────────────────────────────────────────

Sleep 500ms
Type "q"
Sleep 1s
