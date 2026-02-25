//! Ratatui rendering functions for the file-explorer widget.
//!
//! Two public entry-points are provided:
//!
//! * [`render`] — uses the built-in [`Theme::default()`] palette.
//! * [`render_themed`] — accepts a [`Theme`] so every colour can be overridden.
//!
//! Both delegate to the same three private helpers (`render_header`,
//! `render_list`, `render_footer`) that handle the three vertical segments of
//! the widget area.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph},
    Frame,
};

use crate::{
    explorer::{entry_icon, fmt_size},
    palette::Theme,
    FileExplorer,
};

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

    let header = Paragraph::new(Span::styled(
        display_path,
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    ))
    .block(
        Block::default()
            .title(Span::styled(
                " \u{1F4C1}  File Explorer ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent))
            .padding(Padding::horizontal(1)),
    )
    .alignment(Alignment::Left);

    frame.render_widget(header, area);
}

// ── Entry list ────────────────────────────────────────────────────────────────

fn render_list(explorer: &mut FileExplorer, frame: &mut Frame, area: Rect, theme: &Theme) {
    let visible_height = area.height.saturating_sub(2) as usize;

    // Keep scroll_offset in sync so the cursor is always visible.
    if explorer.cursor < explorer.scroll_offset {
        explorer.scroll_offset = explorer.cursor;
    } else if explorer.cursor >= explorer.scroll_offset + visible_height {
        explorer.scroll_offset = explorer.cursor - visible_height + 1;
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
            let is_selectable = explorer.is_selectable(entry);

            let icon = entry_icon(entry);

            let name_style = if entry.is_dir {
                Style::default().fg(theme.dir).add_modifier(Modifier::BOLD)
            } else if is_selectable {
                Style::default()
                    .fg(theme.match_file)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.dim)
            };

            let size_str = match entry.size {
                Some(b) => fmt_size(b),
                None => String::new(),
            };

            let mut spans = vec![
                Span::styled(" ", Style::default()),
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
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let count = explorer.entries.len();
    let pos = if count == 0 {
        "empty".to_string()
    } else {
        format!("{}/{count}", explorer.cursor + 1)
    };

    let block = Block::default()
        .title(Span::styled(
            format!(" Files {pos} "),
            Style::default().fg(theme.dim),
        ))
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
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(46)])
        .split(area);

    // ── Left panel: hints or active search input ──────────────────────────────
    if explorer.search_active {
        // Show the live search query with a blinking-cursor marker.
        let left_line = Line::from(vec![
            Span::styled(
                " / ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &explorer.search_query,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(theme.accent)),
            Span::styled(
                "  Backspace delete  Esc cancel",
                Style::default().fg(theme.dim),
            ),
        ]);
        let search_para = Paragraph::new(left_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.brand)),
        );
        frame.render_widget(search_para, chunks[0]);
    } else {
        let hints = " \u{2191}/k Up  \u{2193}/j Down  Enter Confirm  \u{2190} Ascend  \
                     / Search  s Sort  . Hidden  Esc Dismiss";
        let hints_para = Paragraph::new(Span::styled(hints, Style::default().fg(theme.dim))).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.dim)),
        );
        frame.render_widget(hints_para, chunks[0]);
    }

    // ── Right panel: sort mode + filter info ──────────────────────────────────
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
    frame.render_widget(status_para, chunks[1]);
}
