# VHS tape for options demo
# Demonstrates: options panel open/close, editor cycling, hidden-file toggle,
# sort cycling, single-pane toggle, theme cycling, "no editor" status message
# (both from e key and from Enter on a file), and Tab pane-switching.
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

# ── Cycle through all editors with e (panel must be open) ────────────────────

# None → Helix
Sleep 400ms
Type "e"
Sleep 700ms

# Helix → Neovim
Type "e"
Sleep 700ms

# Neovim → Vim
Type "e"
Sleep 700ms

# Vim → Nano
Type "e"
Sleep 700ms

# Nano → Micro
Type "e"
Sleep 700ms

# Micro → None
Type "e"
Sleep 700ms

# Back to Helix — leave an editor set for the next section
Type "e"
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

# ── Now e opens the file in the configured editor (Helix) ────────────────────
# Navigate to a plain file and press e — with an editor set the status message
# shows "returned from helix — <filename>" after the editor exits.
# In a recording environment hx may not be installed so we just show the key.

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

# ── Re-open options and cycle editor back to None ────────────────────────────

Sleep 400ms
Type "O"
Sleep 700ms

# Helix → Neovim → Vim → Nano → Micro → None
Type "e"
Sleep 350ms
Type "e"
Sleep 350ms
Type "e"
Sleep 350ms
Type "e"
Sleep 350ms
Type "e"
Sleep 800ms

# ── Close the panel and confirm the "no editor" message reappears ────────────

Sleep 400ms
Type "O"
Sleep 700ms

# Navigate to a file
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

# e with editor back to None → status message
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
