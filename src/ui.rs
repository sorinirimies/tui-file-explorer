//! Terminal UI drawing functions for the `tfe` binary.
//!
//! All [`ratatui`] rendering that is specific to the two-pane application
//! lives here. The per-pane widget rendering (header, list, footer) remains in
//! the library's own [`tui_file_explorer::render`] module.
//!
//! Public entry-points:
//!
//! * [`draw`]               — top-level draw callback passed to `Terminal::draw`.
//! * [`render_theme_panel`] — the slide-in theme-picker side panel.
//! * [`render_action_bar`]  — the bottom status / key-hint bar.
//! * [`render_modal`]       — the blocking confirmation dialog overlay.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use tui_file_explorer::{render_themed, Theme};

use crate::app::{App, Modal, Pane};

// ── Top-level draw ────────────────────────────────────────────────────────────

/// Draw the entire application UI into `frame`.
///
/// Divides the terminal area into:
/// - A main area (one or two explorer panes + optional theme panel).
/// - A fixed-height action bar at the bottom.
/// - An optional modal overlay on top of everything.
pub fn draw(app: &mut App, frame: &mut Frame) {
    let theme = app.theme().clone();
    let full = frame.area();

    // Vertical split: main area | action bar (6 rows = nav-hints row + status row).
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)])
        .split(full);

    let main_area = v_chunks[0];
    let action_area = v_chunks[1];

    // Split the action bar vertically into: nav hints (top) | status+shortcuts (bottom).
    let action_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(action_area);
    let nav_area = action_rows[0];
    let status_area = action_rows[1];

    // Horizontal split: left pane | [right pane] | [theme panel].
    let mut h_constraints = vec![];
    if app.single_pane {
        h_constraints.push(Constraint::Min(0));
    } else {
        h_constraints.push(Constraint::Percentage(50));
        h_constraints.push(Constraint::Percentage(50));
    }
    if app.show_theme_panel {
        h_constraints.push(Constraint::Length(32));
    }
    if app.show_options_panel {
        h_constraints.push(Constraint::Length(36));
    }
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(h_constraints)
        .split(main_area);

    // ── Panes ─────────────────────────────────────────────────────────────────
    let active_theme = theme.clone();
    let inactive_theme = theme.clone().accent(theme.dim).brand(theme.dim);

    let (left_theme, right_theme) = match app.active {
        Pane::Left => (&active_theme, &inactive_theme),
        Pane::Right => (&inactive_theme, &active_theme),
    };

    render_themed(&mut app.left, frame, h_chunks[0], left_theme);

    if !app.single_pane {
        render_themed(&mut app.right, frame, h_chunks[1], right_theme);
    }

    // ── Theme panel ───────────────────────────────────────────────────────────
    if app.show_theme_panel {
        let panel_area = h_chunks[h_chunks.len() - 1];
        render_theme_panel(frame, panel_area, app);
    }

    // ── Options panel ─────────────────────────────────────────────────────────
    if app.show_options_panel {
        let panel_area = h_chunks[h_chunks.len() - 1];
        render_options_panel(frame, panel_area, app);
    }

    // ── Action bar ────────────────────────────────────────────────────────────
    render_nav_hints(frame, nav_area, &theme);
    render_action_bar(frame, status_area, app, &theme);

    // ── Modal overlay ─────────────────────────────────────────────────────────
    if let Some(modal) = &app.modal {
        render_modal(frame, full, modal, &theme);
    }
}

// ── Theme panel ───────────────────────────────────────────────────────────────

/// Render the slide-in theme-picker panel occupying `area`.
///
/// The panel is divided into three vertical zones:
/// - A controls header showing the `[` / `t` key hints.
/// - A scrollable list of all available themes.
/// - A description footer for the currently selected theme.
pub fn render_theme_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    // Three-row vertical layout: controls | list | description.
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(area);

    // Controls header.
    let controls = Paragraph::new(Line::from(vec![
        Span::styled(" [ ", Style::default().fg(theme.dim)),
        Span::styled("prev", Style::default().fg(theme.accent)),
        Span::styled("    ", Style::default().fg(theme.dim)),
        Span::styled("t ", Style::default().fg(theme.accent)),
        Span::styled("next", Style::default().fg(theme.accent)),
    ]))
    .block(
        Block::default()
            .title(Span::styled(
                " \u{1F3A8} Themes ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_widget(controls, v[0]);

    // Scrollable theme list — keep the selected item in view.
    let visible = v[1].height.saturating_sub(2) as usize;
    let scroll = if app.theme_idx >= visible {
        app.theme_idx - visible + 1
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .themes
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(i, (name, _, _))| {
            let is_active = i == app.theme_idx;
            let marker = if is_active { "\u{25BA} " } else { "   " };
            let line = Line::from(vec![
                Span::styled(
                    format!("{marker}{:>2}. ", i + 1),
                    Style::default().fg(if is_active { theme.brand } else { theme.dim }),
                ),
                Span::styled(
                    name.to_string(),
                    if is_active {
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    },
                ),
            ]);
            if is_active {
                ListItem::new(line).style(Style::default().bg(theme.sel_bg))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.theme_idx.saturating_sub(scroll)));

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_stateful_widget(list, v[1], &mut list_state);

    // Description footer.
    let desc_text = format!("{}\n{}", app.theme_name(), app.theme_desc());
    let desc = Paragraph::new(desc_text)
        .style(Style::default().fg(theme.success))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(desc, v[2]);
}

// ── Options panel ─────────────────────────────────────────────────────────────

/// Render the slide-in options panel occupying `area`.
///
/// Shows all toggleable persistent settings with their current state.
/// Each row shows the toggle key, setting name, and on/off indicator.
pub fn render_options_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    // Three-row vertical layout: header | boolean toggles | editor section.
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5), // 3 boolean rows + top/bottom border
            Constraint::Length(4), // separator row + 1 editor row + top/bottom border
        ])
        .split(area);

    // Header.
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " O ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("close  ", Style::default().fg(theme.dim)),
        Span::styled(
            "C ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("toggle cd", Style::default().fg(theme.dim)),
    ]))
    .block(
        Block::default()
            .title(Span::styled(
                " ⚙ Options ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_widget(header, v[0]);

    // Boolean toggles list — each row: key | name | value.
    let on_style = Style::default()
        .fg(theme.success)
        .add_modifier(Modifier::BOLD);
    let off_style = Style::default().fg(theme.dim);
    let key_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.fg);

    let opts: &[(&str, &str, bool)] = &[
        ("C", "cd on exit", app.cd_on_exit),
        ("w", "single pane", app.single_pane),
        ("T", "theme panel", app.show_theme_panel),
    ];

    let bool_items: Vec<ListItem> = opts
        .iter()
        .map(|(key, label, enabled)| {
            let (indicator, val_style) = if *enabled {
                ("● on ", on_style)
            } else {
                ("○ off", off_style)
            };
            ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("{key:<2}"), key_style),
                Span::raw("  "),
                Span::styled(format!("{label:<14}"), label_style),
                Span::styled(indicator, val_style),
            ]))
        })
        .collect();

    let bool_list = List::new(bool_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_widget(bool_list, v[1]);

    // Editor section — shows the active editor and lets the user cycle it with e.
    // When no editor is configured the label is rendered in dim so it reads as
    // "inactive", matching how boolean toggles look when they are off.
    let editor_label = app.editor.label().to_string();
    let editor_val_style = if app.editor == crate::app::Editor::None {
        off_style
    } else {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    };
    let editor_item = ListItem::new(Line::from(vec![
        Span::raw(" "),
        Span::styled("e ", key_style),
        Span::raw("  "),
        Span::styled(format!("{:<14}", "open with"), label_style),
        Span::styled(editor_label, editor_val_style),
    ]));

    let editor_list = List::new(vec![editor_item]).block(
        Block::default()
            .title(Span::styled(" Editor ", Style::default().fg(theme.dim)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent)),
    );
    frame.render_widget(editor_list, v[2]);
}

// ── Action bar ────────────────────────────────────────────────────────────────

/// Render the full-width navigation hint row (top half of the action bar).
pub fn render_nav_hints(frame: &mut Frame, area: Rect, theme: &Theme) {
    let hints = Line::from(render_nav_hints_spans(theme));
    let nav_bar = Paragraph::new(hints).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(nav_bar, area);
}

/// Build the list of styled [`Span`]s for the navigation hint row.
///
/// Extracted so the spans can be tested independently of a real [`Frame`].
pub fn render_nav_hints_spans(theme: &Theme) -> Vec<Span<'_>> {
    let k = |s: &'static str| {
        Span::styled(
            s,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    };
    let d = |s: &'static str| Span::styled(s, Style::default().fg(theme.dim));
    vec![
        k("↑"),
        d("/"),
        k("k"),
        d(" up  "),
        k("↓"),
        d("/"),
        k("j"),
        d(" down  "),
        k("→"),
        d("/"),
        k("l"),
        d("/"),
        k("Enter"),
        d(" confirm  "),
        k("←"),
        d("/"),
        k("h"),
        d("/"),
        k("Bksp"),
        d(" ascend  "),
        k("/"),
        d(" search  "),
        k("s"),
        d(" sort  "),
        k("."),
        d(" hidden  "),
        k("Esc"),
        d(" dismiss"),
    ]
}

/// Render the bottom status/shortcut bar occupying `area`.
///
/// The bar is split into two halves:
/// - **Left** — clipboard info when something is yanked, otherwise the current
///   status message (or the active-pane indicator when the status is empty).
/// - **Right** — global key-binding hints.
pub fn render_action_bar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left half: clipboard info, status message, or active-pane indicator.
    if let Some(clip) = &app.clipboard {
        let name = clip.path.file_name().unwrap_or_default().to_string_lossy();
        let line = Line::from(vec![
            Span::styled(
                format!(" {} {}: ", clip.icon(), clip.label()),
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                name.to_string(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        let left_bar = Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.brand)),
        );
        frame.render_widget(left_bar, h[0]);
    } else {
        let status = if app.status_msg.is_empty() {
            let active = match app.active {
                Pane::Left => "left",
                Pane::Right => "right",
            };
            format!(" Active pane: {active}")
        } else {
            format!(" {}", app.status_msg)
        };
        let status_color =
            if app.status_msg.starts_with("Error") || app.status_msg.starts_with("Delete failed") {
                theme.brand
            } else {
                theme.success
            };
        let left_bar = Paragraph::new(Span::styled(status, Style::default().fg(status_color)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.dim)),
            );
        frame.render_widget(left_bar, h[0]);
    }

    // Right half: global key hints.
    let hints = Line::from(render_action_bar_spans(theme));
    let right_bar = Paragraph::new(hints).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(right_bar, h[1]);
}

/// Build the list of styled [`Span`]s for the global key-hint row.
///
/// Extracted so the spans can be tested independently of a real [`Frame`].
pub fn render_action_bar_spans(theme: &Theme) -> Vec<Span<'_>> {
    let k = |s: &'static str| {
        Span::styled(
            s,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    };
    let d = |s: &'static str| Span::styled(s, Style::default().fg(theme.dim));
    vec![
        k("Tab"),
        d(" pane  "),
        k("Spc"),
        d(" mark  "),
        k("y"),
        d(" copy  "),
        k("x"),
        d(" cut  "),
        k("p"),
        d(" paste  "),
        k("d"),
        d(" del  "),
        k("e"),
        d(" edit  "),
        k("["),
        d("/"),
        k("t"),
        d(" theme  "),
        k("w"),
        d(" split  "),
        k("O"),
        d(" options"),
    ]
}

// ── Modal ─────────────────────────────────────────────────────────────────────

/// Render a blocking confirmation modal centred over `area`.
///
/// The modal clears whatever is behind it, draws a double-border box with a
/// title, a body message, and a key-hint footer.
pub fn render_modal(frame: &mut Frame, area: Rect, modal: &Modal, theme: &Theme) {
    // ── MultiDeleteConfirm — taller modal with a scrollable name list ─────────
    if let Modal::MultiDelete { paths } = modal {
        let count = paths.len();
        // Show up to 6 file names inside the box, then a "+ N more" note.
        const MAX_SHOWN: usize = 6;
        let shown: Vec<&std::path::PathBuf> = paths.iter().take(MAX_SHOWN).collect();
        let remainder = count.saturating_sub(MAX_SHOWN);

        // Width: wide enough for the longest shown name + padding.
        let max_name_len = shown
            .iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().len())
            .max()
            .unwrap_or(0);
        let w = (max_name_len as u16 + 8)
            .max(44)
            .min(area.width.saturating_sub(4));
        // Height: header line + one row per shown entry + optional overflow line
        //         + blank gap + hint line + 2 border rows.
        let list_rows = shown.len() + if remainder > 0 { 1 } else { 0 };
        let h = (list_rows as u16 + 5).min(area.height.saturating_sub(2));
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let modal_area = Rect::new(x, y, w, h);

        frame.render_widget(Clear, modal_area);

        let outer = Block::default()
            .title(Span::styled(
                " Confirm Multi-Delete ",
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.brand));
        frame.render_widget(outer, modal_area);

        // Inner layout: summary | file list | hint.
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .margin(1)
            .split(modal_area);

        // Summary line.
        let summary = Paragraph::new(Span::styled(
            format!("Delete {count} item(s)?"),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(summary, v[0]);

        // File name list.
        let mut name_lines: Vec<Line> = shown
            .iter()
            .map(|p| {
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                Line::from(vec![
                    Span::styled("  ◆ ", Style::default().fg(theme.brand)),
                    Span::styled(name.to_string(), Style::default().fg(theme.accent)),
                ])
            })
            .collect();
        if remainder > 0 {
            name_lines.push(Line::from(Span::styled(
                format!("  … and {remainder} more"),
                Style::default().fg(theme.dim),
            )));
        }
        let list_para = Paragraph::new(name_lines);
        frame.render_widget(list_para, v[1]);

        // Hint line.
        let hint_para = Paragraph::new(Line::from(vec![
            Span::styled(
                "  y",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  confirm    ", Style::default().fg(theme.dim)),
            Span::styled(
                "any key",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  cancel  ", Style::default().fg(theme.dim)),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(hint_para, v[2]);

        return;
    }

    // ── Single-item modals (Delete / Overwrite) ───────────────────────────────
    let (title, body) = match modal {
        Modal::Delete { path } => (
            " Confirm Delete ",
            format!(
                "Delete '{}' ?",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
        ),
        Modal::Overwrite { dst, .. } => (
            " Confirm Overwrite ",
            format!(
                "'{}' already exists. Overwrite?",
                dst.file_name().unwrap_or_default().to_string_lossy()
            ),
        ),
        // Already handled above.
        Modal::MultiDelete { .. } => unreachable!(),
    };

    let w = (body.len() as u16 + 6).max(40).min(area.width - 4);
    let h = 7u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let modal_area = Rect::new(x, y, w, h);

    frame.render_widget(Clear, modal_area);

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(modal_area);

    let outer = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.brand)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.brand));
    frame.render_widget(outer, modal_area);

    let body_para = Paragraph::new(Span::styled(
        body,
        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center);
    frame.render_widget(body_para, v[0]);

    let hint_para = Paragraph::new(Line::from(vec![
        Span::styled(
            "  y",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  confirm    ", Style::default().fg(theme.dim)),
        Span::styled(
            "any key",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  cancel  ", Style::default().fg(theme.dim)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(hint_para, v[2]);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tui_file_explorer::Theme;

    // ── render_action_bar_spans ───────────────────────────────────────────────

    #[test]
    fn action_bar_spans_contains_expected_key_labels() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("Tab"), "missing Tab hint");
        assert!(text.contains('y'), "missing y hint");
        assert!(text.contains('x'), "missing x hint");
        assert!(text.contains('p'), "missing p hint");
        assert!(text.contains('d'), "missing d hint");
        assert!(text.contains('['), "missing [ hint");
        assert!(text.contains('t'), "missing t hint");
        assert!(text.contains("Spc"), "missing Spc hint");
        assert!(text.contains('w'), "missing w hint");
    }

    #[test]
    fn action_bar_spans_count_is_stable() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        // 11 key spans + 11 description spans = 22 total.
        assert_eq!(
            spans.len(),
            22,
            "span count changed — update this test if the action bar was intentionally modified"
        );
    }

    #[test]
    fn action_bar_spans_key_spans_are_bold() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        // Key spans are the ones whose content matches a known key label.
        let key_labels = ["Tab", "Spc", "y", "x", "p", "d", "e", "[", "t", "w", "O"];
        for label in key_labels {
            let span = spans
                .iter()
                .find(|s| s.content.as_ref() == label)
                .unwrap_or_else(|| panic!("span for key '{label}' not found"));
            assert!(
                span.style.add_modifier.contains(Modifier::BOLD),
                "key span '{label}' should be bold"
            );
        }
    }

    #[test]
    fn action_bar_spans_description_spans_are_not_bold() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        let key_labels = ["Tab", "Spc", "y", "x", "p", "d", "e", "[", "t", "w", "O"];
        // Every span
        for span in &spans {
            if !key_labels.contains(&span.content.as_ref()) {
                assert!(
                    !span.style.add_modifier.contains(Modifier::BOLD),
                    "description span '{}' should not be bold",
                    span.content
                );
            }
        }
    }

    #[test]
    fn action_bar_spans_key_spans_use_accent_colour() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        let key_labels = ["Tab", "Spc", "y", "x", "p", "d", "e", "[", "t", "w", "O"];
        for label in key_labels {
            let span = spans
                .iter()
                .find(|s| s.content.as_ref() == label)
                .unwrap_or_else(|| panic!("span for key '{label}' not found"));
            assert_eq!(
                span.style.fg,
                Some(theme.accent),
                "key span '{label}' should use the accent colour"
            );
        }
    }

    #[test]
    fn action_bar_spans_description_spans_use_dim_colour() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        let key_labels = ["Tab", "Spc", "y", "x", "p", "d", "e", "[", "t", "w", "O"];
        for span in &spans {
            if !key_labels.contains(&span.content.as_ref()) {
                assert_eq!(
                    span.style.fg,
                    Some(theme.dim),
                    "description span '{}' should use the dim colour",
                    span.content
                );
            }
        }
    }

    // ── render_nav_hints_spans ────────────────────────────────────────────────

    #[test]
    fn nav_hints_spans_contain_arrow_keys() {
        let theme = Theme::default();
        let spans = render_nav_hints_spans(&theme);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains('k'), "missing k (up)");
        assert!(text.contains('j'), "missing j (down)");
        assert!(text.contains('h'), "missing h (ascend)");
        assert!(text.contains('l'), "missing l (confirm)");
        assert!(text.contains("Enter"), "missing Enter");
        assert!(text.contains("Bksp"), "missing Bksp");
    }

    #[test]
    fn nav_hints_spans_contain_search_and_sort() {
        let theme = Theme::default();
        let spans = render_nav_hints_spans(&theme);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains('/'), "missing / (search)");
        assert!(text.contains('s'), "missing s (sort)");
        assert!(text.contains('.'), "missing . (hidden)");
        assert!(text.contains("Esc"), "missing Esc (dismiss)");
    }

    #[test]
    fn nav_hints_key_spans_are_bold() {
        let theme = Theme::default();
        let spans = render_nav_hints_spans(&theme);
        // '/' appears both as a dim separator (between e.g. "↑" and "k") and as
        // the bold search-activation key.  Exclude it from the simple
        // "first match" check and verify it separately below.
        let key_labels = [
            "↑", "k", "↓", "j", "→", "l", "Enter", "←", "h", "Bksp", "s", ".", "Esc",
        ];
        for label in key_labels {
            let span = spans
                .iter()
                .find(|s| s.content.as_ref() == label)
                .unwrap_or_else(|| panic!("nav hint span for '{label}' not found"));
            assert!(
                span.style.add_modifier.contains(Modifier::BOLD),
                "nav key span '{label}' should be bold"
            );
        }
        // '/' is used both as a separator (dim) and as the search key (bold).
        // Assert that at least one '/' span is bold.
        let slash_bold = spans
            .iter()
            .any(|s| s.content.as_ref() == "/" && s.style.add_modifier.contains(Modifier::BOLD));
        assert!(slash_bold, "the search '/' key span should be bold");
    }

    #[test]
    fn nav_hints_key_spans_use_accent_colour() {
        let theme = Theme::default();
        let spans = render_nav_hints_spans(&theme);
        // Exclude '/' — it appears as both a dim separator and a bold accent key.
        let key_labels = ["↑", "k", "↓", "j", "Enter", "Bksp", "s", ".", "Esc"];
        for label in key_labels {
            let span = spans
                .iter()
                .find(|s| s.content.as_ref() == label)
                .unwrap_or_else(|| panic!("nav hint span for '{label}' not found"));
            assert_eq!(
                span.style.fg,
                Some(theme.accent),
                "nav key span '{label}' should use the accent colour"
            );
        }
        // Verify the search '/' key span (bold one) uses the accent colour.
        let slash_accent = spans.iter().any(|s| {
            s.content.as_ref() == "/"
                && s.style.add_modifier.contains(Modifier::BOLD)
                && s.style.fg == Some(theme.accent)
        });
        assert!(
            slash_accent,
            "the search '/' key span should use the accent colour"
        );
    }

    #[test]
    fn nav_hints_description_spans_use_dim_colour() {
        let theme = Theme::default();
        let spans = render_nav_hints_spans(&theme);
        // Bold key labels — spans carrying these as content must be accent-coloured.
        // '/' is excluded because it also appears as a dim separator between combos.
        let key_labels = [
            "↑", "k", "↓", "j", "→", "l", "Enter", "←", "h", "Bksp", "s", ".", "Esc",
        ];
        for span in &spans {
            let content = span.content.as_ref();
            // Skip bold key spans and '/' (mixed role).
            if key_labels.contains(&content) || content == "/" {
                continue;
            }
            assert_eq!(
                span.style.fg,
                Some(theme.dim),
                "nav description span '{}' should use the dim colour",
                span.content
            );
        }
    }

    #[test]
    fn nav_hints_span_count_is_stable() {
        let theme = Theme::default();
        let spans = render_nav_hints_spans(&theme);
        // 14 key spans + 14 separator/description spans = 28 total.
        assert_eq!(
            spans.len(),
            28,
            "nav hint span count changed — update this test if the nav bar was intentionally modified"
        );
    }
}
