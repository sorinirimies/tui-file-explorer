//! Ratatui rendering functions for the file-explorer widget.
//!
//! ## Single-pane entry-points
//!
//! * [`render`] — renders one [`FileExplorer`] using the built-in [`Theme::default()`] palette.
//! * [`render_themed`] — same, but accepts a custom [`Theme`].
//!
//! ## Dual-pane entry-points
//!
//! * [`render_dual_pane`] — renders a [`DualPane`] using the default palette.
//! * [`render_dual_pane_themed`] — same, but accepts a custom [`Theme`].
//!
//! In dual-pane mode the available area is split evenly into two columns, each
//! rendered as an independent [`FileExplorer`].  The active pane's border is
//! drawn in the theme's accent colour; the inactive pane's border is dimmed.
//! In single-pane mode (`dual.single_pane == true`) the full area is given to
//! the active pane.
//!
//! Both families delegate to the same three private helpers (`render_header`,
//! `render_list`, `render_footer`) that handle the three vertical segments of
//! each pane area.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};

use crate::{
    dual_pane::DualPane,
    explorer::{entry_icon, fmt_size},
    palette::Theme,
    FileExplorer, PromptKind,
};

// ── Render constants ──────────────────────────────────────────────────────────

/// Character used for scroll-position indicators.
pub const SCROLLBAR_CHAR: char = '\u{2590}'; // ▐ — right half block

/// Marker displayed next to space-marked (selected) entries.
pub const MARK_INDICATOR: &str = "◆";

/// Default header height in terminal rows.
pub const HEADER_HEIGHT: u16 = 3;

/// Default footer height in terminal rows.
pub const FOOTER_HEIGHT: u16 = 3;

// ── Scroll-thumb helper ───────────────────────────────────────────────────────

/// Paint a thin, semi-translucent scroll thumb on the right edge of `area`.
///
/// The thumb appears only when `total > viewport` (i.e. when not all content
/// is visible).  It uses the right-half-block character (`▐`, U+2590) so it
/// occupies only half the cell width, giving a sleek look.
///
/// * `total` — total number of items / lines in the content.
/// * `offset` — current scroll offset (0-based index of the first visible
///   item).
/// * `color` — foreground colour for the thumb glyph (typically
///   `theme.accent` or `theme.dim`).
pub(crate) fn paint_scrollbar(
    frame: &mut Frame,
    area: Rect,
    total: usize,
    offset: usize,
    color: Color,
) {
    let viewport = area.height as usize;
    if viewport == 0 || total <= viewport {
        return; // everything fits — no scrollbar needed
    }

    let track = area.height as f64;
    let thumb_h = (viewport as f64 / total as f64 * track).max(1.0);
    let thumb_top = (offset as f64 / total as f64 * track).min(track - thumb_h);

    let thumb_start = thumb_top as u16;
    let thumb_end = (thumb_top + thumb_h).ceil().min(track) as u16;

    let x = area.x + area.width.saturating_sub(1);
    let buf = frame.buffer_mut();

    for y in 0..area.height {
        if y >= thumb_start && y < thumb_end {
            if let Some(cell) = buf.cell_mut(Position::new(x, area.y + y)) {
                cell.set_char(SCROLLBAR_CHAR).set_fg(color);
            }
        }
    }
}

// ── Git marker ────────────────────────────────────────────────────────────────

/// One-column git status marker (`M`, `A`, `?`, …), blank when unchanged.
fn git_marker_span(mark: Option<crate::GitMark>, theme: &Theme) -> Span<'static> {
    use crate::GitMark;
    let Some(mark) = mark else {
        return Span::raw(" ");
    };
    let colour = match mark {
        GitMark::Added => theme.success,
        GitMark::Untracked => theme.accent,
        GitMark::Modified => theme.dir,
        GitMark::Renamed => theme.accent,
        GitMark::Deleted | GitMark::Conflict => theme.brand,
    };
    Span::styled(
        mark.symbol().to_string(),
        Style::default().fg(colour).add_modifier(Modifier::BOLD),
    )
}

// ── Input footer ──────────────────────────────────────────────────────────────

/// Render a single-line text-input footer: label + typed text + cursor block
/// + trailing hint, inside a rounded border drawn in `colour`.
fn render_input_footer(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    label: &str,
    input: &str,
    colour: ratatui::style::Color,
    hint: &str,
) {
    let line = Line::from(vec![
        Span::styled(
            label.to_string(),
            Style::default().fg(colour).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            input.to_string(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(theme.accent)),
        Span::styled(hint.to_string(), Style::default().fg(theme.dim)),
    ]);
    let para = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colour))
            .style(Style::default().bg(theme.bg)),
    );
    frame.render_widget(para, area);
}

// ── Public render entry-points ────────────────────────────────────────────────

/// Render the file explorer into `area` using the default colour theme.
///
/// This is the simplest rendering entry-point. Call it from your application's
/// `Terminal::draw` closure, passing a mutable reference to the explorer state
/// and the current Ratatui [`Frame`].
///
/// The widget renders three vertical zones:
/// * **Header** — current directory path inside a rounded border.
/// * **List**   — scrollable, highlighted list of directory entries.
/// * **Footer** — key hints (left) and status / filter info (right).
///
/// # Example
///
/// ```no_run
/// # use tui_file_explorer::{FileExplorer, render};
/// # use ratatui::{Terminal, backend::TestBackend};
/// # let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
/// # let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
/// terminal.draw(|frame| {
///     render(&mut explorer, frame, frame.area());
/// }).unwrap();
/// ```
pub fn render(explorer: &mut FileExplorer, frame: &mut Frame, area: Rect) {
    render_themed(explorer, frame, area, &Theme::default());
}

/// Render a [`DualPane`] into `area` using the default colour theme.
///
/// In dual-pane mode the area is split evenly into two columns.  In
/// single-pane mode (`dual.single_pane == true`) the full area is given to
/// the active pane.
///
/// The active pane's border uses `theme.accent`; the inactive pane's border
/// uses `theme.dim` so the user always knows which side has focus.
///
/// # Example
///
/// ```no_run
/// # use tui_file_explorer::{DualPane, render_dual_pane};
/// # use ratatui::{Terminal, backend::TestBackend};
/// # let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
/// let mut dual = DualPane::builder(std::env::current_dir().unwrap()).build();
/// terminal.draw(|frame| {
///     render_dual_pane(&mut dual, frame, frame.area());
/// }).unwrap();
/// ```
pub fn render_dual_pane(dual: &mut DualPane, frame: &mut Frame, area: Rect) {
    render_dual_pane_themed(dual, frame, area, &Theme::default());
}

/// Render a [`DualPane`] into `area` with a custom [`Theme`].
///
/// This is identical to [`render_dual_pane`] except that you supply the
/// colour palette.
///
/// # Example
///
/// ```no_run
/// # use tui_file_explorer::{DualPane, render_dual_pane_themed, Theme};
/// # use ratatui::{Terminal, backend::TestBackend};
/// # let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
/// let mut dual  = DualPane::builder(std::env::current_dir().unwrap()).build();
/// let theme = Theme::nord();
/// terminal.draw(|frame| {
///     render_dual_pane_themed(&mut dual, frame, frame.area(), &theme);
/// }).unwrap();
/// ```
pub fn render_dual_pane_themed(dual: &mut DualPane, frame: &mut Frame, area: Rect, theme: &Theme) {
    use crate::dual_pane::DualPaneActive;

    if dual.single_pane {
        // Full area goes to whichever pane is active.
        match dual.active_side {
            DualPaneActive::Left => render_pane(
                &mut dual.left,
                frame,
                area,
                theme,
                true, // is_active
            ),
            DualPaneActive::Right => render_pane(&mut dual.right, frame, area, theme, true),
        }
    } else {
        // Split evenly: left | right.
        let halves = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let left_active = dual.active_side == DualPaneActive::Left;
        render_pane(&mut dual.left, frame, halves[0], theme, left_active);
        render_pane(&mut dual.right, frame, halves[1], theme, !left_active);
    }
}

/// Render a single [`FileExplorer`] pane, dimming the border when inactive.
fn render_pane(
    explorer: &mut FileExplorer,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    is_active: bool,
) {
    // Build a locally-adjusted theme copy so the inactive pane has a dimmed
    // border without altering the caller's theme value.
    let pane_theme;
    let effective_theme = if is_active {
        theme
    } else {
        pane_theme = Theme {
            accent: theme.dim,
            ..*theme
        };
        &pane_theme
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Min(1),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(area);

    render_header(explorer, frame, chunks[0], effective_theme);
    render_list(explorer, frame, chunks[1], effective_theme);
    render_footer(explorer, frame, chunks[2], effective_theme);
}

/// Render the file explorer into `area` with a custom [`Theme`].
///
/// This is identical to [`render`] except that you supply the colour palette.
/// Construct a [`Theme`] from [`Theme::default()`] and override the fields you
/// care about, or build one entirely from scratch.
///
/// # Example
///
/// ```no_run
/// # use tui_file_explorer::{FileExplorer, render_themed, Theme};
/// # use ratatui::{Terminal, backend::TestBackend, style::Color};
/// # let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
/// # let mut explorer = FileExplorer::new(std::env::current_dir().unwrap(), vec![]);
/// let theme = Theme::default()
///     .brand(Color::Magenta)
///     .accent(Color::Cyan)
///     .dir(Color::Yellow);
///
/// terminal.draw(|frame| {
///     render_themed(&mut explorer, frame, frame.area(), &theme);
/// }).unwrap();
/// ```
pub fn render_themed(explorer: &mut FileExplorer, frame: &mut Frame, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Min(1),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .split(area);

    render_header(explorer, frame, chunks[0], theme);
    render_list(explorer, frame, chunks[1], theme);
    render_footer(explorer, frame, chunks[2], theme);
}

// ── Header ────────────────────────────────────────────────────────────────────

fn render_header(explorer: &FileExplorer, frame: &mut Frame, area: Rect, theme: &Theme) {
    let path_str = explorer.current_dir.to_string_lossy();

    // Truncate from the left when the path exceeds available width.
    let inner_width = area.width.saturating_sub(4) as usize;
    let display_path = if path_str.len() > inner_width && inner_width > 3 {
        let skip = path_str.len() - inner_width + 1;
        format!("\u{2026}{}", &path_str[skip..])
    } else {
        path_str.to_string()
    };

    let version = concat!(" v", env!("CARGO_PKG_VERSION"), " ");

    let mut block = Block::default()
        .title(Span::styled(
            " \u{1F4C1}  File Explorer ",
            Style::default()
                .fg(theme.brand)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(
            ratatui::text::Line::from(Span::styled(version, Style::default().fg(theme.dim)))
                .right_aligned(),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.bg));

    if !explorer.theme_name.is_empty() {
        let theme_label = format!(" {} ", explorer.theme_name);
        block = block.title(
            ratatui::text::Line::from(Span::styled(theme_label, Style::default().fg(theme.dim)))
                .right_aligned(),
        );
    }

    if !explorer.editor_name.is_empty() {
        let editor_label = format!(" \u{270F}  {} ", explorer.editor_name);
        block = block.title_bottom(
            ratatui::text::Line::from(Span::styled(editor_label, Style::default().fg(theme.dim)))
                .left_aligned(),
        );
    }

    let header = Paragraph::new(Span::styled(
        display_path,
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    ))
    .block(block)
    .alignment(Alignment::Left);

    frame.render_widget(header, area);
}

// ── Entry list ────────────────────────────────────────────────────────────────

fn render_list(explorer: &mut FileExplorer, frame: &mut Frame, area: Rect, theme: &Theme) {
    let visible_height = area.height.saturating_sub(2) as usize;

    // Keep scroll_offset in sync so the cursor is always visible.
    // All arithmetic uses saturating ops so a zero visible_height or a cursor
    // of 0 can never underflow the usize values.
    if explorer.cursor < explorer.scroll_offset {
        explorer.scroll_offset = explorer.cursor;
    } else if explorer.cursor >= explorer.scroll_offset.saturating_add(visible_height) {
        explorer.scroll_offset = explorer
            .cursor
            .saturating_sub(visible_height.saturating_sub(1));
    }
    // Guard: scroll_offset must never exceed the last valid entry index.
    let max_scroll = explorer.entries.len().saturating_sub(1);
    if explorer.scroll_offset > max_scroll {
        explorer.scroll_offset = max_scroll;
    }

    let visible_end = explorer
        .entries
        .len()
        .min(explorer.scroll_offset.saturating_add(visible_height));
    // Pick up finished off-thread work (folder sizes, git status) first so a
    // result computed since the last frame is drawn immediately.
    explorer.poll_background();
    if explorer.show_sizes {
        let visible = explorer.scroll_offset..visible_end;
        // Cheap shallow item counts for the visible directories only…
        explorer.fill_item_counts(visible.clone());
        // …and recursive sizes computed in the background; rows show their
        // item count until the real size arrives on a later frame.
        if explorer.show_dir_sizes {
            explorer.request_dir_sizes(visible);
        }
    }

    // Resolve symlink targets for the visible rows (cached until reload).
    let link_targets: Vec<Option<std::path::PathBuf>> = (explorer.scroll_offset..visible_end)
        .map(|idx| {
            let path = explorer.entries[idx].path.clone();
            explorer.link_target(&path)
        })
        .collect();

    let items: Vec<ListItem> = explorer
        .entries
        .iter()
        .skip(explorer.scroll_offset)
        .take(visible_height)
        .enumerate()
        .map(|(visible_idx, entry)| {
            let abs_idx = visible_idx + explorer.scroll_offset;
            let is_selected = abs_idx == explorer.cursor;
            let is_marked = explorer.marked.contains(&entry.path);

            let icon = entry_icon(entry);

            // All visible entries already passed the extension filter in
            // load_entries, so files are always styled as selectable.
            let name_style = if is_marked {
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(theme.dir).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.match_file)
                    .add_modifier(Modifier::BOLD)
            };

            let size_str = if !explorer.show_sizes {
                String::new()
            } else if entry.is_dir {
                let cached = if explorer.show_dir_sizes {
                    explorer.dir_size_cache.get(&entry.path)
                } else {
                    None
                };
                match cached {
                    Some(&(bytes, partial, _)) if partial => format!("{}+", fmt_size(bytes)),
                    Some(&(bytes, _, _)) => fmt_size(bytes),
                    None => match entry.item_count {
                        Some(1) => "1 item".to_string(),
                        Some(n) => format!("{n} items"),
                        None => String::new(),
                    },
                }
            } else {
                match entry.size {
                    Some(b) => fmt_size(b),
                    None => String::new(),
                }
            };

            // Leading marker: ◆ for marked entries, space otherwise.
            let marker = if is_marked {
                Span::styled(
                    MARK_INDICATOR,
                    Style::default()
                        .fg(theme.brand)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(" ", Style::default())
            };

            let mut spans = vec![marker];
            if explorer.show_git {
                spans.push(git_marker_span(explorer.git_mark(&entry.path), theme));
            }
            spans.extend([
                Span::styled(
                    format!("{icon} "),
                    Style::default().fg(if entry.is_dir { theme.dir } else { theme.fg }),
                ),
                Span::styled(entry.name.as_str(), name_style),
            ]);

            if let Some(Some(target)) = link_targets.get(visible_idx) {
                spans.push(Span::styled(
                    format!(" \u{2192} {}", target.display()),
                    Style::default()
                        .fg(theme.dim)
                        .add_modifier(Modifier::ITALIC),
                ));
            }

            if !size_str.is_empty() {
                spans.push(Span::styled(
                    format!("  {size_str}"),
                    Style::default().fg(theme.dim),
                ));
            }

            if entry.is_dir {
                spans.push(Span::styled("/", Style::default().fg(theme.dir)));
            }

            let line = Line::from(spans);
            if is_selected {
                ListItem::new(line).style(
                    Style::default()
                        .bg(theme.sel_bg)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_marked {
                ListItem::new(line).style(Style::default().add_modifier(Modifier::BOLD))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let count = explorer.entries.len();
    let marked_count = explorer.marked.len();
    let pos = if count == 0 {
        "empty".to_string()
    } else {
        format!("{}/{count}", explorer.cursor + 1)
    };
    let loading = if explorer.is_loading() {
        " · loading…"
    } else {
        ""
    };
    let title = if marked_count > 0 {
        format!(" Files {pos}  ◆ {marked_count} marked{loading} ")
    } else {
        format!(" Files {pos}{loading} ")
    };

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(theme.dim)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.bg));

    let mut list_state = ListState::default();
    if !explorer.entries.is_empty() {
        list_state.select(Some(explorer.cursor.saturating_sub(explorer.scroll_offset)));
    }

    let list = List::new(items).block(block);
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    frame.render_stateful_widget(list, area, &mut list_state);

    // ── Scroll thumb ──────────────────────────────────────────────────────
    paint_scrollbar(
        frame,
        inner,
        explorer.entries.len(),
        explorer.scroll_offset,
        theme.accent,
    );
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn render_footer(explorer: &FileExplorer, frame: &mut Frame, area: Rect, theme: &Theme) {
    // A prompt or an active search replaces the status bar.
    if let Some(prompt) = &explorer.prompt {
        let colour = match prompt.kind {
            PromptKind::Mkdir => theme.success,
            PromptKind::Touch => theme.accent,
            PromptKind::Rename => theme.brand,
        };
        render_input_footer(
            frame,
            area,
            theme,
            prompt.kind.label(),
            &prompt.text,
            colour,
            "  Enter confirm  Esc cancel",
        );
        return;
    }
    if explorer.search_active {
        render_input_footer(
            frame,
            area,
            theme,
            " / ",
            &explorer.search_query,
            theme.brand,
            "  Backspace delete  Esc cancel",
        );
        return;
    }

    // ── Sort / filter status (full width) ─────────────────────────────────────
    let status = if explorer.status.is_empty() {
        let filter = if explorer.extension_filter.is_empty() {
            "all".to_string()
        } else {
            explorer
                .extension_filter
                .iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let hidden_hint = if explorer.show_hidden { " +hidden" } else { "" };
        format!(
            "sort:{} filter:{}{} ",
            explorer.sort_mode.label(),
            filter,
            hidden_hint,
        )
    } else {
        format!(" {} ", explorer.status)
    };

    let status_para = Paragraph::new(Span::styled(status, Style::default().fg(theme.success)))
        .alignment(Alignment::Right)
        .block(
            Block::default()
                .title(Span::styled(
                    disk_usage_label(explorer),
                    Style::default().fg(theme.dim),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.dim))
                .style(Style::default().bg(theme.bg)),
        );
    frame.render_widget(status_para, area);
}

/// Build the footer's left-aligned disk-usage label, e.g.
/// `" \u{1F4BE} 74.7 GB free / 500.0 GB "`.
///
/// Returns an empty string when [`FileExplorer::disk_usage`] is `None`
/// (query failed, or unsupported platform) so the title is simply omitted.
fn disk_usage_label(explorer: &FileExplorer) -> String {
    match explorer.disk_usage {
        Some(usage) => format!(
            " \u{1F4BE} {} free / {} ",
            fmt_size(usage.free_bytes),
            fmt_size(usage.total_bytes),
        ),
        None => String::new(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn make_terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 24)).unwrap()
    }

    fn make_explorer() -> FileExplorer {
        FileExplorer::new(std::env::current_dir().unwrap(), vec![])
    }

    #[test]
    fn render_footer_mkdir_active_does_not_panic() {
        let mut terminal = make_terminal();
        let mut explorer = make_explorer();
        explorer.open_prompt(PromptKind::Mkdir, "");
        terminal
            .draw(|frame| {
                render(&mut explorer, frame, frame.area());
            })
            .unwrap();
    }

    #[test]
    fn render_list_skips_dir_size_walk_when_show_sizes_disabled() {
        let mut terminal = make_terminal();
        let mut explorer = make_explorer();
        explorer.show_sizes = false;
        terminal
            .draw(|frame| {
                render(&mut explorer, frame, frame.area());
            })
            .unwrap();
        assert!(
            explorer.dir_size_cache.is_empty(),
            "disabling show_sizes must skip the recursive dir-size walk entirely"
        );
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    /// The rendered screen row containing `needle` (list rows only — the
    /// footer's disk-usage text is machine dependent).
    fn row_with(terminal: &Terminal<TestBackend>, needle: &str) -> String {
        let buf = terminal.backend().buffer();
        let width = buf.area.width as usize;
        buf.content()
            .chunks(width)
            .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
            .find(|row| row.contains(needle))
            .unwrap_or_else(|| panic!("no row contains {needle:?}"))
    }

    fn explorer_with_dir_and_file() -> (tempfile::TempDir, FileExplorer) {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("folder_x")).unwrap();
        std::fs::write(
            tmp.path().join("folder_x").join("inner.txt"),
            vec![0u8; 2048],
        )
        .unwrap();
        std::fs::write(tmp.path().join("file.bin"), vec![0u8; 4096]).unwrap();
        let explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        (tmp, explorer)
    }

    #[test]
    fn render_list_without_dir_sizes_shows_item_count_and_skips_walk() {
        let (_tmp, mut explorer) = explorer_with_dir_and_file();
        explorer.show_dir_sizes = false;
        let mut terminal = make_terminal();
        terminal
            .draw(|frame| render(&mut explorer, frame, frame.area()))
            .unwrap();
        assert!(
            explorer.dir_size_cache.is_empty(),
            "show_dir_sizes=false must skip the recursive walk"
        );
        assert!(
            row_with(&terminal, "folder_x").contains("1 item"),
            "folder should show its item count"
        );
        assert!(
            row_with(&terminal, "file.bin").contains("KB"),
            "file size must still be shown"
        );
    }

    #[cfg(unix)]
    #[test]
    fn render_list_shows_symlink_target() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("real.txt"), b"x").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("real.txt"), tmp.path().join("alias")).unwrap();
        let mut explorer = FileExplorer::new(tmp.path().to_path_buf(), vec![]);
        let mut terminal = make_terminal();
        terminal
            .draw(|frame| render(&mut explorer, frame, frame.area()))
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("alias"), "{text}");
        assert!(text.contains('\u{2192}'), "symlink arrow missing");
    }

    #[test]
    fn render_list_with_dir_sizes_walks_and_caches() {
        let (_tmp, mut explorer) = explorer_with_dir_and_file();
        let mut terminal = make_terminal();
        terminal
            .draw(|frame| render(&mut explorer, frame, frame.area()))
            .unwrap();
        // Folder sizes are computed off-thread; wait for them.
        explorer.wait_for_background(std::time::Duration::from_secs(5));
        assert!(!explorer.dir_size_cache.is_empty());

        // The next frame shows the real size instead of the item count.
        terminal
            .draw(|frame| render(&mut explorer, frame, frame.area()))
            .unwrap();
        assert!(row_with(&terminal, "folder_x").contains("KB"));
    }

    #[test]
    fn render_list_with_sizes_off_hides_file_sizes_too() {
        let (_tmp, mut explorer) = explorer_with_dir_and_file();
        explorer.show_sizes = false;
        let mut terminal = make_terminal();
        terminal
            .draw(|frame| render(&mut explorer, frame, frame.area()))
            .unwrap();
        assert!(!row_with(&terminal, "folder_x").contains("item"));
        assert!(!row_with(&terminal, "file.bin").contains("KB"));
    }

    #[test]
    fn render_footer_touch_active_does_not_panic() {
        let mut terminal = make_terminal();
        let mut explorer = make_explorer();
        explorer.open_prompt(PromptKind::Touch, "");
        terminal
            .draw(|frame| {
                render(&mut explorer, frame, frame.area());
            })
            .unwrap();
    }

    #[test]
    fn render_footer_rename_active_does_not_panic() {
        let mut terminal = make_terminal();
        let mut explorer = make_explorer();
        explorer.open_prompt(PromptKind::Rename, "");
        terminal
            .draw(|frame| {
                render(&mut explorer, frame, frame.area());
            })
            .unwrap();
    }

    #[test]
    fn render_footer_search_active_does_not_panic() {
        let mut terminal = make_terminal();
        let mut explorer = make_explorer();
        explorer.search_active = true;
        terminal
            .draw(|frame| {
                render(&mut explorer, frame, frame.area());
            })
            .unwrap();
    }

    #[test]
    fn render_footer_all_inactive_does_not_panic() {
        let mut terminal = make_terminal();
        let mut explorer = make_explorer();
        terminal
            .draw(|frame| {
                render(&mut explorer, frame, frame.area());
            })
            .unwrap();
    }
}
