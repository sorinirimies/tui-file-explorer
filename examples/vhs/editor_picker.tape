# VHS tape for editor_picker demo
# Demonstrates: editor picker panel open/close, Terminal Editors section,
# IDEs & GUI Editors section, scrolling, selection, status message, and
# opening a file with the configured editor.
#
# Example: cargo run --example editor_picker

Output examples/vhs/generated/editor_picker.gif

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 900
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

Type "cargo run --example editor_picker"
Enter
Sleep 4s

# ── Panel opens automatically on launch ──────────────────────────────────────

Sleep 800ms

# ── Scroll down through Terminal Editors section ──────────────────────────────

Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms

# ── Cross into IDEs & GUI Editors section ─────────────────────────────────────

Sleep 600ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms

# ── Scroll back up with k ─────────────────────────────────────────────────────

Sleep 600ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 300ms
Type "k"
Sleep 500ms

# ── Select VS Code ────────────────────────────────────────────────────────────

Sleep 400ms
Down
Sleep 300ms
Down
Sleep 300ms

# Highlight is now on vscode — confirm with Enter
Sleep 500ms
Enter
Sleep 1200ms

# ── Panel closed — status bar shows "Editor set to vscode" ───────────────────

Sleep 800ms

# ── Navigate the file list ────────────────────────────────────────────────────

Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms
Down
Sleep 200ms

# ── Re-open the panel with Shift+E ───────────────────────────────────────────

Sleep 500ms
Type "E"
Sleep 900ms

# Cursor should land on vscode (synced to active editor)
Sleep 600ms

# ── Scroll down to IntelliJ ───────────────────────────────────────────────────

Down
Sleep 300ms
Down
Sleep 300ms
Down
Sleep 300ms
Down
Sleep 300ms
Down
Sleep 300ms
Sleep 500ms

# ── Select IntelliJ ──────────────────────────────────────────────────────────

Enter
Sleep 1000ms

# ── Navigate back up and re-open to show RubyMine / Rider / Eclipse ──────────

Sleep 500ms
Type "E"
Sleep 800ms

Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Down
Sleep 250ms
Sleep 600ms

# ── Cancel with Esc — editor stays as intellij ───────────────────────────────

Escape
Sleep 1000ms

# ── Press e on a plain file to show "No editor reachable" path ───────────────
# Navigate into src/ to find .rs files

Sleep 400ms
Type "/"
Sleep 400ms
Type "app"
Sleep 500ms
Escape
Sleep 400ms
Enter
Sleep 700ms

Down
Sleep 200ms
Down
Sleep 200ms

Sleep 500ms
Type "e"
Sleep 2000ms

# ── Re-open panel, go back to none, confirm ───────────────────────────────────

Sleep 400ms
Type "E"
Sleep 700ms

# cursor is at intellij — go all the way back to top (None)
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Sleep 400ms
Enter
Sleep 1000ms

# ── Press e again — should show "no editor set" status ───────────────────────

Sleep 400ms
Type "e"
Sleep 2000ms

# ── Quit ─────────────────────────────────────────────────────────────────────

Sleep 500ms
Escape
Sleep 1s
