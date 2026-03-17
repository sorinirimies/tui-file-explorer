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
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};

use crate::{
    dual_pane::DualPane,
    explorer::{entry_icon, fmt_size},
    palette::Theme,
    FileExplorer,
};

// ── render_input_footer! ──────────────────────────────────────────────────────

/// Render an inline text-input footer and early-return from `render_footer`.
///
/// Checks `$explorer.$active`; if true, renders a single-line [`Paragraph`]
/// containing: label span + typed-input span + cursor-block span + hint span,
/// then calls `frame.render_widget(…, area)` and `return`.
///
/// Parameters
/// ----------
/// `$explorer`   — `&FileExplorer` reference (named `explorer` in the function)
/// `$frame`      — `&mut Frame`
/// `$area`       — `Rect`
/// `$theme`      — `&Theme`
/// `$active`     — field name on `$explorer` (e.g. `mkdir_active`)
/// `$input_expr` — expression that yields the input string (e.g.
///                 `explorer.mkdir_input()` or `&explorer.search_query`)
/// `$label`      — string literal for the label span
/// `$colour`     — `theme` field name for both label and border colour
/// `$hint`       — string literal for the trailing hint
macro_rules! render_input_footer {
    ($explorer:expr, $frame:expr, $area:expr, $theme:expr,
     $active:ident, $input_expr:expr, $label:expr, $colour:ident, $hint:expr) => {
        if $explorer.$active {
            let left_line = Line::from(vec![
                Span::styled(
                    $label,
                    Style::default()
                        .fg($theme.$colour)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    $input_expr,
                    Style::default()
                        .fg($theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("\u{2588}", Style::default().fg($theme.accent)),
                Span::styled($hint, Style::default().fg($theme.dim)),
            ]);
            let para = Paragraph::new(left_line).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg($theme.$colour)),
            );
            $frame.render_widget(para, $area);
            return;
        }
    };
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
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
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
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
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
        .padding(Padding::horizontal(1));

    if !explorer.theme_name.is_empty() {
        let theme_label = format!(" {} ", explorer.theme_name);
        block = block.title(
            ratatui::text::Line::from(Span::styled(theme_label, Style::default().fg(theme.dim)))
                .right_aligned(),
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

            let size_str = match entry.size {
                Some(b) => fmt_size(b),
                None => String::new(),
            };

            // Leading marker: ◆ for marked entries, space otherwise.
            let marker = if is_marked {
                Span::styled(
                    "◆",
                    Style::default()
                        .fg(theme.brand)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(" ", Style::default())
            };

            let mut spans = vec![
                marker,
                Span::styled(
                    format!("{icon} "),
                    Style::default().fg(if entry.is_dir { theme.dir } else { theme.fg }),
                ),
                Span::styled(entry.name.clone(), name_style),
            ];

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
    let title = if marked_count > 0 {
        format!(" Files {pos}  ◆ {marked_count} marked ")
    } else {
        format!(" Files {pos} ")
    };

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(theme.dim)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));

    let mut list_state = ListState::default();
    if !explorer.entries.is_empty() {
        list_state.select(Some(explorer.cursor.saturating_sub(explorer.scroll_offset)));
    }

    let list = List::new(items).block(block);
    frame.render_stateful_widget(list, area, &mut list_state);
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn render_footer(explorer: &FileExplorer, frame: &mut Frame, area: Rect, theme: &Theme) {
    // ── Mkdir input (shown instead of status bar while mkdir mode is active) ──
    render_input_footer!(
        explorer,
        frame,
        area,
        theme,
        mkdir_active,
        explorer.mkdir_input(),
        " \u{1F4C2} New folder: ",
        success,
        "  Enter confirm  Esc cancel"
    );

    // ── Touch input (shown instead of status bar while touch mode is active) ──
    render_input_footer!(
        explorer,
        frame,
        area,
        theme,
        touch_active,
        explorer.touch_input(),
        " \u{1F4C4} New file: ",
        accent,
        "  Enter confirm  Esc cancel"
    );

    // ── Rename input (shown instead of status bar while rename mode is active) ─
    render_input_footer!(
        explorer,
        frame,
        area,
        theme,
        rename_active,
        explorer.rename_input(),
        " \u{270F}\u{FE0F}  Rename: ",
        brand,
        "  Enter confirm  Esc cancel"
    );

    // ── Search input (shown instead of status bar while search is active) ─────
    render_input_footer!(
        explorer,
        frame,
        area,
        theme,
        search_active,
        explorer.search_query.as_str(),
        " / ",
        brand,
        "  Backspace delete  Esc cancel"
    );

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
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.dim)),
        );
    frame.render_widget(status_para, area);
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
        explorer.mkdir_active = true;
        terminal
            .draw(|frame| {
                render(&mut explorer, frame, frame.area());
            })
            .unwrap();
    }

    #[test]
    fn render_footer_touch_active_does_not_panic() {
        let mut terminal = make_terminal();
        let mut explorer = make_explorer();
        explorer.touch_active = true;
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
        explorer.rename_active = true;
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
