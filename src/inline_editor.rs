//! Inline text editor for the TUI file explorer.
//!
//! Provides a minimal, modal-free text editor that can open, display, edit, and
//! save plain-text files directly inside the terminal UI.  Only files up to
//! [`MAX_EDIT_FILE_SIZE`] bytes are accepted — anything larger is rejected to
//! avoid freezing the terminal.
//!
//! ## Key bindings
//!
//! | Key           | Action                                       |
//! |---------------|----------------------------------------------|
//! | Arrow keys    | Move cursor                                  |
//! | Home / End    | Jump to beginning / end of line               |
//! | PgUp / PgDn   | Scroll one page (20 lines)                   |
//! | Printable     | Insert character at cursor                   |
//! | Enter         | Split line at cursor                         |
//! | Backspace     | Delete char before cursor / join lines        |
//! | Delete        | Delete char at cursor / join lines            |
//! | Tab           | Insert 4 spaces                              |
//! | Ctrl+S        | Save to disk                                 |
//! | Esc           | Exit the editor                              |

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::palette::Theme;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum file size the editor will open (10 MiB).
const MAX_EDIT_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Number of spaces inserted for a Tab key press.
const TAB_WIDTH: usize = 4;

/// Number of lines scrolled per PgUp / PgDn key press.
const PAGE_SIZE: usize = 20;

// ── EditorAction ──────────────────────────────────────────────────────────────

/// Result of handling a key event in the inline editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    /// Key was consumed, continue editing.
    Continue,
    /// File was saved successfully.
    Saved,
    /// User wants to exit the editor.
    Exit,
}

// ── InlineEditor ──────────────────────────────────────────────────────────────

/// A simple, single-file text editor that lives inside the TUI.
pub struct InlineEditor {
    /// Lines of the file being edited.
    lines: Vec<String>,
    /// Current cursor row (0-based line index).
    cursor_row: usize,
    /// Current cursor column (0-based character index).
    cursor_col: usize,
    /// Vertical scroll offset (first visible line).
    scroll_row: usize,
    /// Horizontal scroll offset.
    scroll_col: usize,
    /// Path to the file being edited.
    path: PathBuf,
    /// Whether the buffer has been modified since last save.
    modified: bool,
    /// Status message shown at the bottom.
    status: String,
}

impl InlineEditor {
    /// Open a file for editing.
    ///
    /// Returns `Err` if the file does not exist, cannot be read, or exceeds
    /// [`MAX_EDIT_FILE_SIZE`].
    pub fn open(path: &Path) -> io::Result<Self> {
        let meta = fs::metadata(path)?;
        if meta.len() > MAX_EDIT_FILE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "file is too large ({} bytes, max {})",
                    meta.len(),
                    MAX_EDIT_FILE_SIZE
                ),
            ));
        }

        let content = fs::read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }

        Ok(Self {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            path: path.to_path_buf(),
            modified: false,
            status: String::new(),
        })
    }

    // ── Key handling ──────────────────────────────────────────────────────

    /// Handle a key event. Returns the [`EditorAction`] to take.
    pub fn handle_key(&mut self, key: KeyEvent) -> EditorAction {
        // Only react to key-press events.
        if key.kind != KeyEventKind::Press {
            return EditorAction::Continue;
        }

        match (key.modifiers, key.code) {
            // ── Save ──────────────────────────────────────────────────
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => match self.save() {
                Ok(()) => {
                    self.status = "saved".into();
                    EditorAction::Saved
                }
                Err(e) => {
                    self.status = format!("save failed: {e}");
                    EditorAction::Continue
                }
            },

            // ── Exit ──────────────────────────────────────────────────
            (_, KeyCode::Esc) => EditorAction::Exit,

            // ── Navigation ────────────────────────────────────────────
            (_, KeyCode::Up) => {
                self.move_up();
                EditorAction::Continue
            }
            (_, KeyCode::Down) => {
                self.move_down();
                EditorAction::Continue
            }
            (_, KeyCode::Left) => {
                self.move_left();
                EditorAction::Continue
            }
            (_, KeyCode::Right) => {
                self.move_right();
                EditorAction::Continue
            }
            (_, KeyCode::Home) => {
                self.cursor_col = 0;
                self.adjust_scroll_col();
                EditorAction::Continue
            }
            (_, KeyCode::End) => {
                self.cursor_col = self.current_line_len();
                self.adjust_scroll_col();
                EditorAction::Continue
            }
            (_, KeyCode::PageUp) => {
                self.page_up();
                EditorAction::Continue
            }
            (_, KeyCode::PageDown) => {
                self.page_down();
                EditorAction::Continue
            }

            // ── Editing ───────────────────────────────────────────────
            (_, KeyCode::Char(c))
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.insert_char(c);
                EditorAction::Continue
            }
            (_, KeyCode::Enter) => {
                self.insert_newline();
                EditorAction::Continue
            }
            (_, KeyCode::Backspace) => {
                self.backspace();
                EditorAction::Continue
            }
            (_, KeyCode::Delete) => {
                self.delete();
                EditorAction::Continue
            }
            (_, KeyCode::Tab) => {
                self.insert_tab();
                EditorAction::Continue
            }

            // ── Everything else ───────────────────────────────────────
            _ => EditorAction::Continue,
        }
    }

    // ── Persistence ───────────────────────────────────────────────────────

    /// Save the current buffer to disk.
    pub fn save(&mut self) -> io::Result<()> {
        let content = self.lines.join("\n");
        fs::write(&self.path, &content)?;
        self.modified = false;
        Ok(())
    }

    // ── Getters ───────────────────────────────────────────────────────────

    /// Number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Whether the buffer has been modified since the last save.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Path to the file being edited.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Current status message.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Current cursor row (0-based).
    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    /// Current cursor column (0-based).
    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    /// Slice of all lines in the buffer.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Vertical scroll offset (first visible line).
    pub fn scroll_row(&self) -> usize {
        self.scroll_row
    }

    /// Horizontal scroll offset.
    pub fn scroll_col(&self) -> usize {
        self.scroll_col
    }

    // ── Movement helpers (private) ────────────────────────────────────────

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_cursor_col();
            self.adjust_scroll_row();
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.clamp_cursor_col();
            self.adjust_scroll_row();
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.current_line_len();
        }
        self.adjust_scroll_row();
        self.adjust_scroll_col();
    }

    fn move_right(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
        self.adjust_scroll_row();
        self.adjust_scroll_col();
    }

    fn page_up(&mut self) {
        self.cursor_row = self.cursor_row.saturating_sub(PAGE_SIZE);
        self.scroll_row = self.scroll_row.saturating_sub(PAGE_SIZE);
        self.clamp_cursor_col();
        self.adjust_scroll_row();
    }

    fn page_down(&mut self) {
        let max_row = self.lines.len().saturating_sub(1);
        self.cursor_row = (self.cursor_row + PAGE_SIZE).min(max_row);
        self.scroll_row = (self.scroll_row + PAGE_SIZE).min(max_row);
        self.clamp_cursor_col();
        self.adjust_scroll_row();
    }

    // ── Editing helpers (private) ─────────────────────────────────────────

    fn insert_char(&mut self, c: char) {
        let byte_idx = self.cursor_byte_offset();
        self.lines[self.cursor_row].insert(byte_idx, c);
        self.cursor_col += 1;
        self.modified = true;
        self.adjust_scroll_col();
    }

    fn insert_newline(&mut self) {
        let byte_idx = self.cursor_byte_offset();
        let tail = self.lines[self.cursor_row][byte_idx..].to_string();
        self.lines[self.cursor_row].truncate(byte_idx);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.lines.insert(self.cursor_row, tail);
        self.modified = true;
        self.adjust_scroll_row();
        self.adjust_scroll_col();
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let byte_start = self.byte_offset_of_char(self.cursor_row, self.cursor_col - 1);
            let byte_end = self.byte_offset_of_char(self.cursor_row, self.cursor_col);
            self.lines[self.cursor_row].replace_range(byte_start..byte_end, "");
            self.cursor_col -= 1;
            self.modified = true;
        } else if self.cursor_row > 0 {
            let removed = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.current_line_len();
            self.lines[self.cursor_row].push_str(&removed);
            self.modified = true;
        }
        self.adjust_scroll_row();
        self.adjust_scroll_col();
    }

    fn delete(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col < len {
            let byte_start = self.cursor_byte_offset();
            let byte_end = self.byte_offset_of_char(self.cursor_row, self.cursor_col + 1);
            self.lines[self.cursor_row].replace_range(byte_start..byte_end, "");
            self.modified = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
            self.modified = true;
        }
    }

    fn insert_tab(&mut self) {
        for _ in 0..TAB_WIDTH {
            self.insert_char(' ');
        }
    }

    // ── Internal utilities ────────────────────────────────────────────────

    /// Character-length of the current line.
    fn current_line_len(&self) -> usize {
        self.lines[self.cursor_row].chars().count()
    }

    /// Byte offset corresponding to `self.cursor_col` in the current line.
    fn cursor_byte_offset(&self) -> usize {
        self.byte_offset_of_char(self.cursor_row, self.cursor_col)
    }

    /// Byte offset of the `col`-th character in `line_idx`.
    fn byte_offset_of_char(&self, line_idx: usize, col: usize) -> usize {
        self.lines[line_idx]
            .char_indices()
            .nth(col)
            .map(|(i, _)| i)
            .unwrap_or(self.lines[line_idx].len())
    }

    /// Clamp `cursor_col` to the length of the current line.
    fn clamp_cursor_col(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col > len {
            self.cursor_col = len;
        }
    }

    /// Ensure `scroll_row` keeps the cursor visible within `PAGE_SIZE` lines.
    fn adjust_scroll_row(&mut self) {
        if self.cursor_row < self.scroll_row {
            self.scroll_row = self.cursor_row;
        } else if self.cursor_row >= self.scroll_row + PAGE_SIZE {
            self.scroll_row = self.cursor_row.saturating_sub(PAGE_SIZE - 1);
        }
    }

    /// Ensure `scroll_col` keeps the cursor visible.
    fn adjust_scroll_col(&mut self) {
        if self.cursor_col < self.scroll_col {
            self.scroll_col = self.cursor_col;
        }
        // We use a generous window — the render function uses the actual
        // available width, but here we clamp to a sane default.
        let visible_cols = 80usize;
        if self.cursor_col >= self.scroll_col + visible_cols {
            self.scroll_col = self.cursor_col.saturating_sub(visible_cols - 1);
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Render the inline editor into the given area.
///
/// Layout (top to bottom):
///
/// 1. **Header** — 1 line: `✏️  Editing: <filename> [modified]`
/// 2. **Content** — remaining lines minus footer: line numbers + text
/// 3. **Footer** — 1 line: status message + cursor position + key hints
pub fn render_inline_editor(frame: &mut Frame, area: Rect, editor: &InlineEditor, theme: &Theme) {
    if area.height < 3 {
        return; // not enough room for header + 1 content line + footer
    }

    let header_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    let footer_area = Rect {
        x: area.x,
        y: area.y + area.height - 1,
        width: area.width,
        height: 1,
    };
    let content_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(2),
    };

    // ── Header ────────────────────────────────────────────────────────────
    let file_name = editor
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| editor.path.display().to_string());

    let mod_indicator = if editor.modified { " [modified]" } else { "" };
    let header_text = format!("✏️  Editing: {file_name}{mod_indicator}");
    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_text,
        Style::default().fg(theme.brand).bold(),
    )]));
    frame.render_widget(header, header_area);

    // ── Content ───────────────────────────────────────────────────────────
    let visible_rows = content_area.height as usize;
    let gutter_width: u16 = 5; // "NNNNN"
    let sep_width: u16 = 3; // " │ "
    let text_start_col = content_area.x + gutter_width + sep_width;
    let text_width = content_area.width.saturating_sub(gutter_width + sep_width) as usize;

    for row_offset in 0..visible_rows {
        let line_idx = editor.scroll_row + row_offset;
        let y = content_area.y + row_offset as u16;

        if line_idx >= editor.lines.len() {
            // Past end of file — render a tilde like vi
            let tilde = Paragraph::new(Line::from(Span::styled(
                "    ~",
                Style::default().fg(theme.dim),
            )));
            frame.render_widget(
                tilde,
                Rect {
                    x: content_area.x,
                    y,
                    width: content_area.width,
                    height: 1,
                },
            );
            continue;
        }

        let is_cursor_line = line_idx == editor.cursor_row;
        let line_bg = if is_cursor_line {
            theme.sel_bg
        } else {
            Color::Reset
        };

        // ── Line number ───────────────────────────────────────────────
        let line_num = format!("{:>5}", line_idx + 1);
        let gutter = Paragraph::new(Line::from(Span::styled(
            line_num,
            Style::default().fg(theme.accent).bg(line_bg),
        )));
        frame.render_widget(
            gutter,
            Rect {
                x: content_area.x,
                y,
                width: gutter_width,
                height: 1,
            },
        );

        // ── Separator ─────────────────────────────────────────────────
        let sep = Paragraph::new(Line::from(Span::styled(
            " │ ",
            Style::default().fg(theme.dim).bg(line_bg),
        )));
        frame.render_widget(
            sep,
            Rect {
                x: content_area.x + gutter_width,
                y,
                width: sep_width,
                height: 1,
            },
        );

        // ── Line text with cursor ─────────────────────────────────────
        let line = &editor.lines[line_idx];
        let chars: Vec<char> = line.chars().collect();
        let visible_start = editor.scroll_col;

        let mut spans: Vec<Span> = Vec::new();

        if is_cursor_line {
            // Build char-by-char so we can highlight the cursor position.
            for vi in 0..text_width {
                let ci = visible_start + vi; // character index in line
                if ci == editor.cursor_col {
                    // Cursor: render a block character in accent colour.
                    if ci < chars.len() {
                        spans.push(Span::styled(
                            chars[ci].to_string(),
                            Style::default().fg(theme.brand).bg(theme.accent),
                        ));
                    } else {
                        spans.push(Span::styled(
                            "█",
                            Style::default().fg(theme.accent).bg(line_bg),
                        ));
                        // pad the rest if needed
                        if vi + 1 < text_width {
                            let pad = " ".repeat(text_width - vi - 1);
                            spans
                                .push(Span::styled(pad, Style::default().fg(theme.fg).bg(line_bg)));
                        }
                        break;
                    }
                } else if ci < chars.len() {
                    spans.push(Span::styled(
                        chars[ci].to_string(),
                        Style::default().fg(theme.fg).bg(line_bg),
                    ));
                } else {
                    // Past end of line — fill remaining with bg.
                    let remaining = text_width - vi;
                    spans.push(Span::styled(
                        " ".repeat(remaining),
                        Style::default().fg(theme.fg).bg(line_bg),
                    ));
                    break;
                }
            }
            // Edge case: cursor is exactly at visible_start + 0 and text_width
            // is 0 — nothing to render.
        } else {
            // Non-cursor line: render the visible portion in one go.
            let visible: String = chars.iter().skip(visible_start).take(text_width).collect();
            spans.push(Span::styled(
                visible,
                Style::default().fg(theme.fg).bg(line_bg),
            ));
        }

        let text_line = Paragraph::new(Line::from(spans));
        frame.render_widget(
            text_line,
            Rect {
                x: text_start_col,
                y,
                width: text_width as u16,
                height: 1,
            },
        );
    }

    // ── Scroll thumb ─────────────────────────────────────────────────────────────
    crate::render::paint_scrollbar(
        frame,
        content_area,
        editor.lines.len(),
        editor.scroll_row,
        theme.accent,
    );

    // ── Footer ───────────────────────────────────────────────────────────────────
    let status_style = if editor.status.starts_with("save failed") {
        Style::default().fg(theme.brand)
    } else {
        Style::default().fg(theme.success)
    };

    let right_info = format!(
        "Ln {}, Col {} │ Ctrl+S save │ Esc exit",
        editor.cursor_row + 1,
        editor.cursor_col + 1,
    );

    let right_width = right_info.chars().count();
    let left_width = area.width as usize - right_width.min(area.width as usize);

    let status_display: String = if editor.status.len() > left_width {
        editor.status.chars().take(left_width).collect()
    } else {
        let pad = left_width.saturating_sub(editor.status.chars().count());
        format!("{}{}", editor.status, " ".repeat(pad))
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(status_display, status_style),
        Span::styled(right_info, Style::default().fg(theme.dim)),
    ]));
    frame.render_widget(footer, footer_area);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use std::io::Write;
    use tempfile::tempdir;

    /// Helper: create a press `KeyEvent`.
    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    /// Helper: create a press `KeyEvent` with modifiers.
    fn press_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    /// Helper: write `content` into a temp file and return (`dir`, `path`).
    fn temp_file(content: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, content).unwrap();
        (dir, path)
    }

    // ── Construction ──────────────────────────────────────────────────────

    #[test]
    fn open_reads_file_content() {
        let (_dir, path) = temp_file("hello\nworld");
        let ed = InlineEditor::open(&path).unwrap();
        assert_eq!(ed.lines(), &["hello", "world"]);
    }

    #[test]
    fn open_empty_file_has_one_line() {
        let (_dir, path) = temp_file("");
        let ed = InlineEditor::open(&path).unwrap();
        assert_eq!(ed.lines(), &[""]);
        assert_eq!(ed.line_count(), 1);
    }

    #[test]
    fn open_nonexistent_file_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nope.txt");
        assert!(InlineEditor::open(&path).is_err());
    }

    #[test]
    fn open_too_large_file_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("big.txt");
        // Create a file just over the limit.
        let mut f = fs::File::create(&path).unwrap();
        let chunk = vec![b'A'; 1024];
        for _ in 0..(MAX_EDIT_FILE_SIZE / 1024 + 1) {
            f.write_all(&chunk).unwrap();
        }
        drop(f);
        assert!(InlineEditor::open(&path).is_err());
    }

    // ── Getters ───────────────────────────────────────────────────────────

    #[test]
    fn line_count_matches_file_lines() {
        let (_dir, path) = temp_file("a\nb\nc");
        let ed = InlineEditor::open(&path).unwrap();
        assert_eq!(ed.line_count(), 3);
    }

    #[test]
    fn is_modified_false_initially() {
        let (_dir, path) = temp_file("hello");
        let ed = InlineEditor::open(&path).unwrap();
        assert!(!ed.is_modified());
    }

    #[test]
    fn path_returns_opened_path() {
        let (_dir, path) = temp_file("hello");
        let ed = InlineEditor::open(&path).unwrap();
        assert_eq!(ed.path(), path);
    }

    #[test]
    fn status_empty_initially() {
        let (_dir, path) = temp_file("hello");
        let ed = InlineEditor::open(&path).unwrap();
        assert!(ed.status().is_empty());
    }

    // ── Cursor movement ───────────────────────────────────────────────────

    #[test]
    fn move_down_increments_row() {
        let (_dir, path) = temp_file("a\nb\nc");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Down));
        assert_eq!(ed.cursor_row(), 1);
    }

    #[test]
    fn move_up_decrements_row() {
        let (_dir, path) = temp_file("a\nb\nc");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Down));
        ed.handle_key(press(KeyCode::Down));
        ed.handle_key(press(KeyCode::Up));
        assert_eq!(ed.cursor_row(), 1);
    }

    #[test]
    fn move_up_at_top_stays() {
        let (_dir, path) = temp_file("a\nb");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Up));
        assert_eq!(ed.cursor_row(), 0);
    }

    #[test]
    fn move_down_at_bottom_stays() {
        let (_dir, path) = temp_file("a\nb");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Down));
        ed.handle_key(press(KeyCode::Down)); // already at last line
        assert_eq!(ed.cursor_row(), 1);
    }

    #[test]
    fn move_right_increments_col() {
        let (_dir, path) = temp_file("abc");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Right));
        assert_eq!(ed.cursor_col(), 1);
    }

    #[test]
    fn move_left_decrements_col() {
        let (_dir, path) = temp_file("abc");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Right));
        ed.handle_key(press(KeyCode::Right));
        ed.handle_key(press(KeyCode::Left));
        assert_eq!(ed.cursor_col(), 1);
    }

    #[test]
    fn move_left_at_col_zero_goes_to_prev_line_end() {
        let (_dir, path) = temp_file("abc\nde");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Down)); // row 1, col 0
        ed.handle_key(press(KeyCode::Left)); // should go to row 0, col 3
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 3);
    }

    #[test]
    fn move_right_at_line_end_goes_to_next_line_start() {
        let (_dir, path) = temp_file("ab\ncd");
        let mut ed = InlineEditor::open(&path).unwrap();
        // Move to end of first line
        ed.handle_key(press(KeyCode::End));
        assert_eq!(ed.cursor_col(), 2);
        // Move right should go to row 1, col 0
        ed.handle_key(press(KeyCode::Right));
        assert_eq!(ed.cursor_row(), 1);
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn cursor_col_clamped_on_vertical_move() {
        let (_dir, path) = temp_file("abcdef\nab");
        let mut ed = InlineEditor::open(&path).unwrap();
        // Move to col 5 on the first line
        for _ in 0..5 {
            ed.handle_key(press(KeyCode::Right));
        }
        assert_eq!(ed.cursor_col(), 5);
        // Move down to shorter line — col should clamp to 2
        ed.handle_key(press(KeyCode::Down));
        assert_eq!(ed.cursor_col(), 2);
    }

    #[test]
    fn home_moves_to_col_zero() {
        let (_dir, path) = temp_file("hello world");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::End));
        assert!(ed.cursor_col() > 0);
        ed.handle_key(press(KeyCode::Home));
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn end_moves_to_line_end() {
        let (_dir, path) = temp_file("hello");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::End));
        assert_eq!(ed.cursor_col(), 5);
    }

    // ── Text editing ──────────────────────────────────────────────────────

    #[test]
    fn insert_char_at_cursor() {
        let (_dir, path) = temp_file("ac");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Right)); // after 'a'
        ed.handle_key(press(KeyCode::Char('b')));
        assert_eq!(ed.lines()[0], "abc");
        assert_eq!(ed.cursor_col(), 2);
    }

    #[test]
    fn insert_char_sets_modified() {
        let (_dir, path) = temp_file("x");
        let mut ed = InlineEditor::open(&path).unwrap();
        assert!(!ed.is_modified());
        ed.handle_key(press(KeyCode::Char('y')));
        assert!(ed.is_modified());
    }

    #[test]
    fn backspace_deletes_char() {
        let (_dir, path) = temp_file("abc");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::End)); // col 3
        ed.handle_key(press(KeyCode::Backspace));
        assert_eq!(ed.lines()[0], "ab");
        assert_eq!(ed.cursor_col(), 2);
    }

    #[test]
    fn backspace_at_line_start_joins_lines() {
        let (_dir, path) = temp_file("ab\ncd");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Down)); // row 1, col 0
        ed.handle_key(press(KeyCode::Backspace));
        assert_eq!(ed.line_count(), 1);
        assert_eq!(ed.lines()[0], "abcd");
        assert_eq!(ed.cursor_row(), 0);
        assert_eq!(ed.cursor_col(), 2); // end of "ab"
    }

    #[test]
    fn delete_removes_char_at_cursor() {
        let (_dir, path) = temp_file("abc");
        let mut ed = InlineEditor::open(&path).unwrap();
        // cursor at col 0 — delete 'a'
        ed.handle_key(press(KeyCode::Delete));
        assert_eq!(ed.lines()[0], "bc");
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn delete_at_line_end_joins_with_next() {
        let (_dir, path) = temp_file("ab\ncd");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::End)); // col 2
        ed.handle_key(press(KeyCode::Delete));
        assert_eq!(ed.line_count(), 1);
        assert_eq!(ed.lines()[0], "abcd");
    }

    #[test]
    fn enter_splits_line() {
        let (_dir, path) = temp_file("abcd");
        let mut ed = InlineEditor::open(&path).unwrap();
        // Move to col 2 then press Enter
        ed.handle_key(press(KeyCode::Right));
        ed.handle_key(press(KeyCode::Right));
        ed.handle_key(press(KeyCode::Enter));
        assert_eq!(ed.line_count(), 2);
        assert_eq!(ed.lines()[0], "ab");
        assert_eq!(ed.lines()[1], "cd");
        assert_eq!(ed.cursor_row(), 1);
        assert_eq!(ed.cursor_col(), 0);
    }

    #[test]
    fn tab_inserts_spaces() {
        let (_dir, path) = temp_file("x");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Tab));
        assert_eq!(ed.lines()[0], "    x");
        assert_eq!(ed.cursor_col(), TAB_WIDTH);
    }

    // ── Save ──────────────────────────────────────────────────────────────

    #[test]
    fn save_writes_to_disk() {
        let (_dir, path) = temp_file("original");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::End));
        ed.handle_key(press(KeyCode::Char('!')));
        ed.save().unwrap();
        let on_disk = fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, "original!");
    }

    #[test]
    fn save_clears_modified_flag() {
        let (_dir, path) = temp_file("hi");
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::Char('x')));
        assert!(ed.is_modified());
        ed.save().unwrap();
        assert!(!ed.is_modified());
    }

    // ── Key handling — action types ───────────────────────────────────────

    #[test]
    fn esc_returns_exit() {
        let (_dir, path) = temp_file("x");
        let mut ed = InlineEditor::open(&path).unwrap();
        assert_eq!(ed.handle_key(press(KeyCode::Esc)), EditorAction::Exit);
    }

    #[test]
    fn ctrl_s_saves_and_returns_saved() {
        let (_dir, path) = temp_file("x");
        let mut ed = InlineEditor::open(&path).unwrap();
        let action = ed.handle_key(press_mod(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(action, EditorAction::Saved);
    }

    #[test]
    fn regular_char_returns_continue() {
        let (_dir, path) = temp_file("x");
        let mut ed = InlineEditor::open(&path).unwrap();
        let action = ed.handle_key(press(KeyCode::Char('a')));
        assert_eq!(action, EditorAction::Continue);
    }

    // ── Scroll ────────────────────────────────────────────────────────────

    #[test]
    fn scroll_keeps_cursor_visible() {
        // Create a file with 40 lines — more than one page (PAGE_SIZE = 20).
        let content: String = (0..40)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (_dir, path) = temp_file(&content);
        let mut ed = InlineEditor::open(&path).unwrap();
        // Move cursor down past the page boundary.
        for _ in 0..25 {
            ed.handle_key(press(KeyCode::Down));
        }
        assert_eq!(ed.cursor_row(), 25);
        // scroll_row should have adjusted so cursor is visible.
        assert!(ed.scroll_row() <= ed.cursor_row());
        assert!(ed.cursor_row() < ed.scroll_row() + PAGE_SIZE);
    }

    #[test]
    fn page_down_advances_scroll() {
        let content: String = (0..60)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (_dir, path) = temp_file(&content);
        let mut ed = InlineEditor::open(&path).unwrap();
        ed.handle_key(press(KeyCode::PageDown));
        assert_eq!(ed.cursor_row(), PAGE_SIZE);
        assert!(ed.scroll_row() <= ed.cursor_row());
    }

    #[test]
    fn page_up_retreats_scroll() {
        let content: String = (0..60)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (_dir, path) = temp_file(&content);
        let mut ed = InlineEditor::open(&path).unwrap();
        // Go down two pages then up one.
        ed.handle_key(press(KeyCode::PageDown));
        ed.handle_key(press(KeyCode::PageDown));
        let row_before = ed.cursor_row();
        ed.handle_key(press(KeyCode::PageUp));
        assert_eq!(ed.cursor_row(), row_before - PAGE_SIZE);
    }

    // ── Release / Repeat events are ignored ───────────────────────────────

    #[test]
    fn release_event_is_ignored() {
        let (_dir, path) = temp_file("x");
        let mut ed = InlineEditor::open(&path).unwrap();
        let release = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        let action = ed.handle_key(release);
        assert_eq!(action, EditorAction::Continue);
        // Nothing should have been inserted.
        assert_eq!(ed.lines()[0], "x");
    }

    // ── UTF-8 safety ──────────────────────────────────────────────────────

    #[test]
    fn insert_and_delete_multibyte_chars() {
        let (_dir, path) = temp_file("aé");
        let mut ed = InlineEditor::open(&path).unwrap();
        assert_eq!(ed.lines()[0].chars().count(), 2);

        // Move to col 1 (between 'a' and 'é'), insert '→'
        ed.handle_key(press(KeyCode::Right));
        ed.handle_key(press(KeyCode::Char('→')));
        assert_eq!(ed.lines()[0], "a→é");
        assert_eq!(ed.cursor_col(), 2);

        // Backspace should remove '→'
        ed.handle_key(press(KeyCode::Backspace));
        assert_eq!(ed.lines()[0], "aé");
        assert_eq!(ed.cursor_col(), 1);
    }

    // ── CRLF handling ─────────────────────────────────────────────────────

    #[test]
    fn crlf_line_endings_are_handled() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("crlf.txt");
        fs::write(&path, "line1\r\nline2\r\n").unwrap();
        let ed = InlineEditor::open(&path).unwrap();
        // `str::lines()` strips the trailing empty line produced by a
        // trailing line-ending, so we only get two lines here.
        assert_eq!(ed.lines(), &["line1", "line2"]);
    }
}
