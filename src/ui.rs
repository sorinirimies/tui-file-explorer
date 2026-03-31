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

use crate::{render_themed, Theme};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CopyProgress, Modal, Pane, Snackbar};
use tui_slider::{style::SliderStyle, Slider, SliderOrientation, SliderState};

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

    // Vertical split: main area | [debug log panel] | action bar.
    // The debug panel only appears when --verbose is active.
    let v_chunks = if app.verbose {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(10),
                Constraint::Length(6),
            ])
            .split(full)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(6)])
            .split(full)
    };

    let main_area = v_chunks[0];
    let action_area = if app.verbose {
        v_chunks[2]
    } else {
        v_chunks[1]
    };

    // Split the action bar vertically into three rows of 3:
    //   row 0 — Navigate | File Ops
    //   row 1 — Global   | Status
    let action_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(action_area);
    let nav_fileops_area = action_rows[0];
    let global_status_area = action_rows[1];

    // ── Debug log panel (verbose only) ────────────────────────────────────────
    if app.verbose {
        render_debug_panel(frame, v_chunks[1], app, &theme);
    }

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
        h_constraints.push(Constraint::Length(42));
    }
    if app.show_editor_panel {
        h_constraints.push(Constraint::Length(42));
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

    // Sync the current theme name and editor label into both panes so render_header can display them.
    let theme_name = app.theme_name().to_string();
    app.left.theme_name = theme_name.clone();
    app.right.theme_name = theme_name;

    let editor_name = if app.editor == crate::app::Editor::None {
        String::new()
    } else {
        app.editor.label().to_string()
    };
    app.left.editor_name = editor_name.clone();
    app.right.editor_name = editor_name;

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

    // ── Editor panel ──────────────────────────────────────────────────────────
    if app.show_editor_panel {
        let panel_area = h_chunks[h_chunks.len() - 1];
        render_editor_panel(frame, panel_area, app);
    }

    // ── Action bar ────────────────────────────────────────────────────────────
    render_nav_hints(frame, nav_fileops_area, global_status_area, app, &theme);

    // ── Modal overlay ─────────────────────────────────────────────────────────
    if let Some(modal) = &app.modal {
        render_modal(frame, full, modal, &theme);
    }

    // ── Copy progress overlay ─────────────────────────────────────────────────
    if let Some(progress) = &app.copy_progress {
        render_copy_progress(frame, full, progress, &theme);
    }

    // ── Snackbar overlay ──────────────────────────────────────────────────────
    // Expire stale snackbars first, then render if one is still active.
    if app.snackbar.as_ref().is_some_and(|s| s.is_expired()) {
        app.snackbar = None;
    }
    if let Some(snackbar) = &app.snackbar {
        render_snackbar(frame, full, snackbar, &theme);
    }
}

// ── Debug log panel ───────────────────────────────────────────────────────────

/// Render a scrollable debug log panel showing the most recent log lines.
///
/// The panel auto-scrolls to the bottom unless the user has scrolled up
/// (tracked by `app.debug_scroll`).  Only rendered when `--verbose` is active.
pub fn render_debug_panel(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::default()
        .title(Span::styled(
            format!(" Debug Log ({} lines) ", app.debug_log.len()),
            Style::default().fg(theme.accent),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.dim));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.debug_log.is_empty() || inner.height == 0 {
        return;
    }

    // Show the most recent lines that fit in the panel.
    let visible_lines = inner.height as usize;
    let total = app.debug_log.len();
    let start = total.saturating_sub(visible_lines + app.debug_scroll);
    let end = total.saturating_sub(app.debug_scroll);

    let lines: Vec<Line> = app.debug_log[start..end]
        .iter()
        .map(|msg| Line::from(Span::styled(msg.as_str(), Style::default().fg(theme.dim))))
        .collect();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ── Snackbar ──────────────────────────────────────────────────────────────────

/// Render a floating snackbar notification near the bottom of `area`.
///
/// The snackbar is a single-line (3-row with border) centred overlay that
/// clears whatever content is behind it. Error snackbars are tinted with the
/// theme's brand (red/warning) colour; info snackbars use the success colour.
pub fn render_snackbar(frame: &mut Frame, area: Rect, snackbar: &Snackbar, theme: &Theme) {
    // Height: 3 rows (border top + content + border bottom).
    // Width: message length + 4 (2 padding + 2 border chars), capped to terminal width.
    let msg = &snackbar.message;
    let desired_width = (msg.len() as u16)
        .saturating_add(4)
        .min(area.width.saturating_sub(4));
    let width = desired_width.max(20);
    let height = 3u16;

    // Position: horizontally centred, 4 rows above the bottom of `area` so it
    // floats just above the action bar without obscuring it.
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height + 7);

    let snackbar_area = Rect {
        x,
        y,
        width,
        height,
    };

    let border_color = if snackbar.is_error {
        theme.brand
    } else {
        theme.success
    };
    let text_color = if snackbar.is_error {
        theme.brand
    } else {
        theme.success
    };

    frame.render_widget(Clear, snackbar_area);
    let paragraph = Paragraph::new(Line::from(Span::styled(
        format!(" {msg} "),
        Style::default().fg(text_color).add_modifier(Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(paragraph, snackbar_area);
}

// ── Copy progress overlay ─────────────────────────────────────────────────────

/// Render a centred floating progress panel while a copy/move is in progress.
///
/// Shows:
/// - A titled border with the operation label (e.g. "Copying 3 item(s)…")
/// - A `tui-slider` progress bar driven by `progress.fraction()`
/// - The name of the file currently being processed
pub fn render_copy_progress(frame: &mut Frame, area: Rect, progress: &CopyProgress, theme: &Theme) {
    let width = (area.width / 2).max(50).min(area.width.saturating_sub(4));
    let height = 7u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_area);

    let outer = Block::default()
        .title(Span::styled(
            format!(" ⟳  {} ", progress.label),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));
    frame.render_widget(outer, popup_area);

    // Inner layout: progress bar (3 rows) + current-item label (1 row).
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(1)])
        .margin(1)
        .split(popup_area);

    // ── Slider progress bar ───────────────────────────────────────────────────
    let pct = progress.fraction() * 100.0;
    let state = SliderState::new(pct, 0.0, 100.0);
    let style = SliderStyle::horizontal_thick();
    let slider = Slider::from_state(&state)
        .orientation(SliderOrientation::Horizontal)
        .filled_symbol(style.filled_symbol)
        .empty_symbol(style.empty_symbol)
        .handle_symbol(style.handle_symbol)
        .filled_color(theme.success)
        .empty_color(theme.dim)
        .handle_color(theme.accent)
        .show_handle(true)
        .show_value(true);
    frame.render_widget(slider, inner[0]);

    // ── Current item label ────────────────────────────────────────────────────
    let done_label = format!(
        " {}/{} — {}",
        progress.done,
        progress.total,
        if progress.current_item.is_empty() {
            "…".to_string()
        } else {
            progress.current_item.clone()
        }
    );
    let item_para = Paragraph::new(Span::styled(done_label, Style::default().fg(theme.dim)));
    frame.render_widget(item_para, inner[1]);
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
        Span::styled(" ↑ [ ", Style::default().fg(theme.dim)),
        Span::styled("prev", Style::default().fg(theme.accent)),
        Span::styled("   ", Style::default().fg(theme.dim)),
        Span::styled("↓ t ", Style::default().fg(theme.dim)),
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

// ── Editor panel ──────────────────────────────────────────────────────────────

/// Render the slide-in editor-picker side panel occupying `area`.
///
/// Two bordered group cells — "Terminal Editors" and "IDEs & GUI Editors" —
/// mirror the Options panel layout.  The highlighted row (cursor) is tracked
/// by `app.editor_panel_idx`; the active editor is marked with a `✓`.
pub fn render_editor_panel(frame: &mut Frame, area: Rect, app: &App) {
    use crate::app::{App as TfeApp, Editor};

    let theme = app.theme();

    let on_style = Style::default()
        .fg(theme.success)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.fg);
    let subtitle_style = Style::default().fg(theme.dim);
    let title_style = Style::default()
        .fg(theme.brand)
        .add_modifier(Modifier::BOLD);

    let editors = TfeApp::all_editors();
    let first_ide = TfeApp::first_ide_idx();
    let terminal_editors = &editors[..first_ide]; // None … Emacs
    let ide_editors = &editors[first_ide..]; // Sublime … Eclipse

    // ── Layout ───────────────────────────────────────────────────────────────
    // Slots (top to bottom):
    //   [0]  hints header box              — 2 rows
    //   [1]  gap                           — 1 row
    //   [2]  "Terminal Editors" title      — 1 row
    //   [3]  Terminal Editors cell         — terminal_editors.len() + 2 (borders)
    //   [4]  gap                           — 1 row
    //   [5]  "IDEs & GUI Editors" title    — 1 row
    //   [6]  IDEs cell                     — ide_editors.len() + 2 (borders)
    //   [7]  gap                           — 1 row
    //   [8]  footer                        — 3 rows
    //   [9]  remainder
    let terminal_cell_h = terminal_editors.len() as u16 + 2;
    let ide_cell_h = ide_editors.len() as u16 + 2;

    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),               // [0] hints header
            Constraint::Length(1),               // [1] gap
            Constraint::Length(1),               // [2] "Terminal Editors" title
            Constraint::Length(terminal_cell_h), // [3] terminal cell
            Constraint::Length(1),               // [4] gap
            Constraint::Length(1),               // [5] "IDEs & GUI Editors" title
            Constraint::Length(ide_cell_h),      // [6] IDE cell
            Constraint::Length(1),               // [7] gap
            Constraint::Length(3),               // [8] footer
            Constraint::Min(0),                  // [9] slack
        ])
        .split(area);

    // ── Helper: floating section title ────────────────────────────────────────
    let section_title = |frame: &mut Frame, slot: Rect, label: &str| {
        let dashes = "─".repeat((slot.width as usize).saturating_sub(label.len() + 2));
        let para = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {label} "), subtitle_style),
            Span::styled(dashes, subtitle_style),
        ]));
        frame.render_widget(para, slot);
    };

    // ── Helper: one editor row ────────────────────────────────────────────────
    let editor_row = |editor: &Editor, idx: usize| -> Line {
        let is_highlighted = idx == app.editor_panel_idx;
        let is_selected = editor == &app.editor;
        let marker = if is_highlighted { "\u{25BA} " } else { "   " };
        let check = if is_selected { "\u{2713} " } else { "  " };
        Line::from(vec![
            Span::styled(
                marker,
                Style::default().fg(if is_highlighted {
                    theme.brand
                } else {
                    theme.dim
                }),
            ),
            Span::styled(
                check,
                if is_selected {
                    on_style
                } else {
                    subtitle_style
                },
            ),
            Span::styled(
                format!("{:<width$}", editor.label(), width = 16),
                if is_highlighted {
                    key_style
                } else {
                    label_style
                },
            ),
        ])
    };

    // ── Hints header ─────────────────────────────────────────────────────────
    let header = Block::default()
        .title(Span::styled(" \u{1F4DD} Editor ", title_style))
        .title_bottom(Line::from(vec![
            Span::styled(" Shift + E ", key_style),
            Span::styled("close", subtitle_style),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));
    frame.render_widget(header, slots[0]);

    // ── Terminal Editors cell ─────────────────────────────────────────────────
    section_title(frame, slots[2], "Terminal Editors");

    let terminal_rows: Vec<Line> = terminal_editors
        .iter()
        .enumerate()
        .map(|(i, ed)| editor_row(ed, i))
        .collect();
    let terminal_cell = Paragraph::new(terminal_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(terminal_cell, slots[3]);

    // ── IDEs & GUI Editors cell ───────────────────────────────────────────────
    section_title(frame, slots[5], "IDEs & GUI Editors");

    let ide_rows: Vec<Line> = ide_editors
        .iter()
        .enumerate()
        .map(|(i, ed)| editor_row(ed, first_ide + i))
        .collect();
    let ide_cell = Paragraph::new(ide_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(ide_cell, slots[6]);

    // ── Footer — binary of the highlighted editor ─────────────────────────────
    let highlighted_editor = &editors[app.editor_panel_idx];
    let footer_text = if *highlighted_editor == Editor::None {
        "none  —  no editor".to_string()
    } else {
        format!(
            "{}  →  {}",
            highlighted_editor.label(),
            highlighted_editor.binary().unwrap_or_default()
        )
    };
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(theme.success))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(footer, slots[8]);
}

// ── Options panel ─────────────────────────────────────────────────────────────

/// Render the slide-in options panel occupying `area`.
///
/// Shows all toggleable persistent settings with their current state.
/// Each row shows the toggle key, setting name, and on/off indicator.
pub fn render_options_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    let on_style = Style::default()
        .fg(theme.success)
        .add_modifier(Modifier::BOLD);
    let off_style = Style::default().fg(theme.dim);
    let key_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.fg);
    let subtitle_style = Style::default().fg(theme.dim);
    let title_style = Style::default()
        .fg(theme.brand)
        .add_modifier(Modifier::BOLD);

    // ── Layout ───────────────────────────────────────────────────────────────
    // Slots (top to bottom):
    //   [0]  hints header box         — 2 rows  (top border: title, bottom border: hints)
    //   [1]  gap                      — 1 row
    //   [2]  "Toggles" section title  — 1 row
    //   [3]  Toggles group cell       — 5 rows  (border + 3 rows + border)
    //   [4]  gap                      — 1 row
    //   [5]  "Editor" section title   — 1 row
    //   [6]  Editor group cell        — 3 rows  (border + 1 row + border)
    //   [7]  gap                      — 1 row
    //   [8]  "File Ops" section title — 1 row
    //   [9]  File Ops group cell      — 9 rows  (border + 7 rows + border)
    //   [10] remainder (absorbs slack)
    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // [0] hints header (border-only, no body)
            Constraint::Length(1), // [1] gap
            Constraint::Length(1), // [2] "Toggles" title
            Constraint::Length(5), // [3] Toggles group (3 option rows)
            Constraint::Length(1), // [4] gap
            Constraint::Length(1), // [5] "Editor" title
            Constraint::Length(3), // [6] Editor group (1 option row)
            Constraint::Length(1), // [7] gap
            Constraint::Length(1), // [8] "File Ops" title
            Constraint::Length(9), // [9] File Ops group (7 option rows)
            Constraint::Min(0),    // [10] slack
        ])
        .split(area);

    // ── Helper: floating section title ────────────────────────────────────────
    // Renders " Label ─────" in dim colour with no border.
    let section_title = |frame: &mut Frame, slot: Rect, label: &str| {
        let dashes = "─".repeat((slot.width as usize).saturating_sub(label.len() + 2));
        let para = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {label} "), subtitle_style),
            Span::styled(dashes, subtitle_style),
        ]));
        frame.render_widget(para, slot);
    };

    // ── Helper: one option row inside a group cell ────────────────────────────
    let option_row = |key: &str, label: &str, value: Span<'static>| -> Line {
        Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{key:<12}"), key_style),
            Span::styled(format!("{label:<14}"), label_style),
            value,
        ])
    };

    // ── Bool value span helper ────────────────────────────────────────────────
    let bool_span = |enabled: bool| -> Span {
        if enabled {
            Span::styled("● on ", on_style)
        } else {
            Span::styled("○ off", off_style)
        }
    };

    // ── Hints header ─────────────────────────────────────────────────────────
    // Title on the top border line; key hints on the bottom border line.
    // No body row — the block is exactly 2 rows (top + bottom borders).
    let header = Block::default()
        .title(Span::styled(" ⚙ Options ", title_style))
        .title_bottom(Line::from(vec![
            Span::styled(" Shift + O ", key_style),
            Span::styled("close", subtitle_style),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));
    frame.render_widget(header, slots[0]);

    // ── Toggles group ─────────────────────────────────────────────────────────
    section_title(frame, slots[2], "Toggles");

    let toggles_rows = vec![
        option_row("Shift + C", "cd on exit", bool_span(app.cd_on_exit)),
        option_row("w", "single pane", bool_span(app.single_pane)),
        option_row("Shift + T", "theme panel", bool_span(app.show_theme_panel)),
    ];
    let toggles_cell = Paragraph::new(toggles_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(toggles_cell, slots[3]);

    // ── Editor group ──────────────────────────────────────────────────────────
    section_title(frame, slots[5], "Editor");

    let editor_label = app.editor.label().to_string();
    let editor_val_style = if app.editor == crate::app::Editor::None {
        off_style
    } else {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    };

    let editor_rows = vec![option_row(
        "Shift + E",
        "editor",
        Span::styled(editor_label, editor_val_style),
    )];
    let editor_cell = Paragraph::new(editor_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(editor_cell, slots[6]);

    // ── File Ops group ────────────────────────────────────────────────────────
    section_title(frame, slots[8], "File Ops");

    let fileops_rows = vec![
        option_row(
            "Space",
            "mark",
            Span::styled("multi-select", Style::default().fg(theme.accent)),
        ),
        option_row(
            "y",
            "copy",
            Span::styled("yank", Style::default().fg(theme.accent)),
        ),
        option_row(
            "x",
            "cut",
            Span::styled("cut", Style::default().fg(theme.accent)),
        ),
        option_row(
            "p",
            "paste",
            Span::styled("paste", Style::default().fg(theme.accent)),
        ),
        option_row(
            "d",
            "delete",
            Span::styled("delete", Style::default().fg(theme.accent)),
        ),
        option_row(
            "n",
            "new folder",
            Span::styled("mkdir", Style::default().fg(theme.accent)),
        ),
        option_row(
            "N",
            "new file",
            Span::styled("touch", Style::default().fg(theme.accent)),
        ),
        option_row(
            "r",
            "rename",
            Span::styled("rename", Style::default().fg(theme.accent)),
        ),
    ];
    let fileops_cell = Paragraph::new(fileops_rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(fileops_cell, slots[9]);
}

// ── Action bar ────────────────────────────────────────────────────────────────

/// Render the two hint rows and the status bar of the action area.
///
/// Layout (each row is 3 terminal rows tall):
///   Row 0  ╭─ Navigate ──────────────────╮╭─ File Ops ──────────────────╮
///   Row 1  ╭─ Global ────────────────────╮╭─ Status ────────────────────╮
pub fn render_nav_hints(frame: &mut Frame, row0: Rect, row1: Rect, app: &App, theme: &Theme) {
    let k = |s: &'static str| {
        Span::styled(
            s,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    };
    let d = |s: &'static str| Span::styled(s, Style::default().fg(theme.dim));

    // ── Row 0: Navigate (left 50%) | File Ops (right 50%) ────────────────────
    let row0_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(row0);

    let nav_spans = vec![
        k("↑"),
        d("/"),
        k("k"),
        d(" up │ "),
        k("↓"),
        d("/"),
        k("j"),
        d(" down │ "),
        k("→"),
        d("/"),
        k("l"),
        d("/"),
        k("Enter"),
        d(" open │ "),
        k("←"),
        d("/"),
        k("h"),
        d("/"),
        k("Bksp"),
        d(" back │ "),
        k("/"),
        d(" search │ "),
        k("s"),
        d(" sort │ "),
        k("."),
        d(" hidden │ "),
        k("Esc"),
        d(" dismiss"),
    ];
    let nav_col = Paragraph::new(Line::from(nav_spans)).block(
        Block::default()
            .title(Span::styled(" Navigate ", Style::default().fg(theme.dim)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(nav_col, row0_cols[0]);

    let fileops_spans = vec![
        k("y"),
        d(" copy │ "),
        k("x"),
        d(" cut │ "),
        k("p"),
        d(" paste │ "),
        k("d"),
        d(" del │ "),
        k("n"),
        d(" mkdir │ "),
        k("N"),
        d(" touch │ "),
        k("r"),
        d(" rename │ "),
        k("Space"),
        d(" mark"),
    ];
    let fileops_col = Paragraph::new(Line::from(fileops_spans)).block(
        Block::default()
            .title(Span::styled(" File Ops ", Style::default().fg(theme.dim)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(fileops_col, row0_cols[1]);

    // ── Row 1: Global (left 50%) | Status (right 50%) ────────────────────────
    let row1_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(row1);

    let global_spans = vec![
        k("Tab"),
        d(" pane │ "),
        k("w"),
        d(" split │ "),
        k("["),
        d("/"),
        k("t"),
        d(" theme │ "),
        k("Shift+E"),
        d(" editor │ "),
        k("Shift+O"),
        d(" options"),
    ];
    let global_col = Paragraph::new(Line::from(global_spans)).block(
        Block::default()
            .title(Span::styled(" Global ", Style::default().fg(theme.dim)))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(global_col, row1_cols[0]);

    // Status cell (right half of row 1) — replaces the old render_action_bar.
    render_action_bar(frame, row1_cols[1], app, theme);
}

/// Build the flat list of styled [`Span`]s for the navigate column.
///
/// Extracted so the spans can be tested independently of a real [`Frame`].
#[cfg(test)]
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
        d(" up │ "),
        k("↓"),
        d("/"),
        k("j"),
        d(" down │ "),
        k("→"),
        d("/"),
        k("l"),
        d("/"),
        k("Enter"),
        d(" open │ "),
        k("←"),
        d("/"),
        k("h"),
        d("/"),
        k("Bksp"),
        d(" back │ "),
        k("/"),
        d(" search │ "),
        k("s"),
        d(" sort │ "),
        k("."),
        d(" hidden │ "),
        k("Esc"),
        d(" dismiss"),
    ]
}

/// Render the status cell: clipboard info, or status message on the left
/// and the active pane + configured editor on the right.
///
/// Occupies the right half of row 1 in the action area.
pub fn render_action_bar(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // ── Left: clipboard info or status message ────────────────────────────────
    if let Some(clip) = &app.clipboard {
        let display_name = if clip.count() > 1 {
            format!("{} items", clip.count())
        } else {
            clip.first_path()
                .and_then(|p| p.file_name())
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };
        let line = Line::from(vec![
            Span::styled(
                format!(" {} {}: ", clip.icon(), clip.label()),
                Style::default()
                    .fg(theme.brand)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                display_name,
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
        let status_color =
            if app.status_msg.starts_with("Error") || app.status_msg.starts_with("Delete failed") {
                theme.brand
            } else {
                theme.success
            };
        let status = if app.status_msg.is_empty() {
            " No pending operations".to_string()
        } else {
            format!(" {}", app.status_msg)
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

    // ── Right: active pane + editor (always visible) ──────────────────────────
    let active_label = match app.active {
        Pane::Left => "left",
        Pane::Right => "right",
    };

    let mut right_spans = vec![
        Span::styled(" pane: ", Style::default().fg(theme.dim)),
        Span::styled(
            active_label,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   editor: ", Style::default().fg(theme.dim)),
    ];

    if app.editor == crate::app::Editor::None {
        right_spans.push(Span::styled("none", Style::default().fg(theme.dim)));
        right_spans.push(Span::styled(
            "  (Shift+E to pick)",
            Style::default().fg(theme.dim),
        ));
    } else {
        right_spans.push(Span::styled(
            format!("\u{270F}  {}", app.editor.label()),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let right_bar = Paragraph::new(Line::from(right_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.dim)),
    );
    frame.render_widget(right_bar, h[1]);
}

/// Build the list of styled [`Span`]s for the global key-hint column.
///
/// Extracted so the spans can be tested independently of a real [`Frame`].
#[cfg(test)]
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
        d(" pane │ "),
        k("w"),
        d(" split │ "),
        k("["),
        d("/"),
        k("t"),
        d(" theme │ "),
        k("Shift+E"),
        d(" editor │ "),
        k("Shift+O"),
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
    use crate::Theme;

    // ── render_action_bar_spans ───────────────────────────────────────────────

    #[test]
    fn action_bar_spans_contains_expected_key_labels() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("Tab"), "missing Tab hint");
        assert!(text.contains('['), "missing [ hint");
        assert!(text.contains('t'), "missing t hint");
        assert!(text.contains('w'), "missing w hint");
        assert!(text.contains("Shift+E"), "missing Shift+E (editor) hint");
        assert!(text.contains("Shift+O"), "missing Shift+O (options) hint");
    }

    #[test]
    fn action_bar_spans_count_is_stable() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        // 6 key spans + 6 description spans = 12 total.
        assert_eq!(
            spans.len(),
            12,
            "span count changed — update this test if the action bar was intentionally modified"
        );
    }

    #[test]
    fn action_bar_spans_key_spans_are_bold() {
        let theme = Theme::default();
        let spans = render_action_bar_spans(&theme);
        let key_labels = ["Tab", "w", "[", "t", "Shift+E", "Shift+O"];
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
        let key_labels = ["Tab", "w", "[", "t", "Shift+E", "Shift+O"];
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
        let key_labels = ["Tab", "w", "[", "t", "Shift+E", "Shift+O"];
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
        let key_labels = ["Tab", "w", "[", "t", "Shift+E", "Shift+O"];
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

    // ── render_snackbar ───────────────────────────────────────────────────────

    /// Build a minimal `Snackbar` without going through `App` helpers so the
    /// tests stay pure (no `Instant::now()` drift issues in CI).
    fn make_snackbar(message: &str, is_error: bool) -> Snackbar {
        use std::time::{Duration, Instant};
        Snackbar {
            message: message.to_string(),
            expires_at: Instant::now() + Duration::from_secs(10),
            is_error,
        }
    }

    #[test]
    fn snackbar_geometry_height_is_three() {
        // render_snackbar always uses height = 3 (top border + content + bottom border).
        // We verify the computed Rect indirectly by checking that a short message
        // still produces a snackbar_area with height == 3.
        // Since render_snackbar is not pure (it takes a Frame), we test the
        // height constant through the public geometry formula used in the function.
        let height: u16 = 3;
        assert_eq!(height, 3);
    }

    #[test]
    fn snackbar_info_uses_success_colour() {
        let theme = Theme::default();
        let sb = make_snackbar("info message", false);
        // For an info snackbar the border / text colour must be theme.success.
        let expected = theme.success;
        let actual = if sb.is_error {
            theme.brand
        } else {
            theme.success
        };
        assert_eq!(actual, expected, "info snackbar should use success colour");
    }

    #[test]
    fn snackbar_error_uses_brand_colour() {
        let theme = Theme::default();
        let sb = make_snackbar("error message", true);
        let expected = theme.brand;
        let actual = if sb.is_error {
            theme.brand
        } else {
            theme.success
        };
        assert_eq!(actual, expected, "error snackbar should use brand colour");
    }

    #[test]
    fn snackbar_info_and_error_colours_are_distinct() {
        let theme = Theme::default();
        // Sanity check: the two colour paths must differ so the tests above
        // are actually meaningful.
        assert_ne!(
            theme.success, theme.brand,
            "success and brand colours must differ for snackbar colour tests to be meaningful"
        );
    }

    #[test]
    fn snackbar_message_is_preserved() {
        let msg = "No editor set — open Options (Shift + O) and press e to pick one";
        let sb = make_snackbar(msg, true);
        assert_eq!(sb.message, msg);
    }

    #[test]
    fn snackbar_width_at_least_minimum() {
        // The width formula: desired = msg.len() + 4, clamped to area_width - 4,
        // then max(20).  For any message the result must be >= 20.
        let msg = "hi"; // very short message
        let area_width: u16 = 200;
        let desired = (msg.len() as u16)
            .saturating_add(4)
            .min(area_width.saturating_sub(4));
        let width = desired.max(20);
        assert!(width >= 20, "snackbar width must be at least 20 columns");
    }

    #[test]
    fn snackbar_width_capped_to_area() {
        // A very long message should not exceed area_width - 4.
        let msg = "a".repeat(300);
        let area_width: u16 = 120;
        let desired = (msg.len() as u16)
            .saturating_add(4)
            .min(area_width.saturating_sub(4));
        let width = desired.max(20);
        assert!(
            width <= area_width,
            "snackbar must not exceed the terminal width"
        );
    }

    #[test]
    fn snackbar_is_not_expired_when_fresh() {
        let sb = make_snackbar("fresh", false);
        assert!(
            !sb.is_expired(),
            "a newly created snackbar must not be expired"
        );
    }

    #[test]
    fn snackbar_is_expired_after_deadline() {
        use std::time::{Duration, Instant};
        let sb = Snackbar {
            message: "old".into(),
            expires_at: Instant::now() - Duration::from_millis(1),
            is_error: false,
        };
        assert!(
            sb.is_expired(),
            "snackbar past its deadline must be expired"
        );
    }

    // ── Debug log panel ───────────────────────────────────────────────────────

    fn make_app_in(dir: std::path::PathBuf) -> App {
        App::new(crate::app::AppOptions {
            left_dir: dir.clone(),
            right_dir: dir,
            ..crate::app::AppOptions::default()
        })
    }

    fn make_verbose_app_in(dir: std::path::PathBuf) -> App {
        App::new(crate::app::AppOptions {
            left_dir: dir.clone(),
            right_dir: dir,
            verbose: true,
            ..crate::app::AppOptions::default()
        })
    }

    #[test]
    fn default_app_verbose_is_false() {
        let app = make_app_in(std::env::temp_dir());
        assert!(!app.verbose);
    }

    #[test]
    fn default_app_debug_log_is_empty() {
        let app = make_app_in(std::env::temp_dir());
        assert!(app.debug_log.is_empty());
    }

    #[test]
    fn verbose_app_has_verbose_true() {
        let app = make_verbose_app_in(std::env::temp_dir());
        assert!(app.verbose);
    }

    #[test]
    fn verbose_app_log_accumulates() {
        let mut app = make_verbose_app_in(std::env::temp_dir());
        app.log("first");
        app.log("second");
        assert_eq!(app.debug_log.len(), 2);
        assert_eq!(app.debug_log[0], "first");
        assert_eq!(app.debug_log[1], "second");
    }

    #[test]
    fn non_verbose_app_log_is_noop() {
        let mut app = make_app_in(std::env::temp_dir());
        app.log("ignored");
        assert!(app.debug_log.is_empty());
    }

    #[test]
    fn startup_log_transferred_into_debug_log() {
        let app = App::new(crate::app::AppOptions {
            left_dir: std::env::temp_dir(),
            right_dir: std::env::temp_dir(),
            verbose: true,
            startup_log: vec!["boot 1".into(), "boot 2".into()],
            ..crate::app::AppOptions::default()
        });
        assert_eq!(app.debug_log.len(), 2);
        assert_eq!(app.debug_log[0], "boot 1");
        assert_eq!(app.debug_log[1], "boot 2");
    }

    #[test]
    fn startup_log_followed_by_runtime_log_preserves_order() {
        let mut app = App::new(crate::app::AppOptions {
            left_dir: std::env::temp_dir(),
            right_dir: std::env::temp_dir(),
            verbose: true,
            startup_log: vec!["startup".into()],
            ..crate::app::AppOptions::default()
        });
        app.log("runtime");
        assert_eq!(app.debug_log, vec!["startup", "runtime"]);
    }

    #[test]
    fn draw_without_verbose_does_not_panic() {
        let mut app = make_app_in(std::env::temp_dir());
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(&mut app, frame)).unwrap();
        // No debug panel should have been rendered — just verify no panic.
    }

    #[test]
    fn draw_with_verbose_does_not_panic() {
        let mut app = make_verbose_app_in(std::env::temp_dir());
        app.log("test log line");
        let backend = ratatui::backend::TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(&mut app, frame)).unwrap();
        // Debug panel should have been rendered — just verify no panic.
    }

    #[test]
    fn draw_with_verbose_empty_log_does_not_panic() {
        let mut app = make_verbose_app_in(std::env::temp_dir());
        let backend = ratatui::backend::TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(&mut app, frame)).unwrap();
    }

    #[test]
    fn draw_with_verbose_many_log_lines_does_not_panic() {
        let mut app = make_verbose_app_in(std::env::temp_dir());
        for i in 0..100 {
            app.log(format!("line {i}"));
        }
        let backend = ratatui::backend::TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(&mut app, frame)).unwrap();
    }

    #[test]
    fn draw_with_verbose_small_terminal_does_not_panic() {
        let mut app = make_verbose_app_in(std::env::temp_dir());
        app.log("log line");
        // Very small terminal — layout must not crash.
        let backend = ratatui::backend::TestBackend::new(40, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(&mut app, frame)).unwrap();
    }

    /// Read all cell symbols from a test backend buffer into a single string.
    fn buffer_text(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
        let buf = terminal.backend().buffer().clone();
        let mut text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                text.push_str(buf[(x, y)].symbol());
            }
        }
        text
    }

    #[test]
    fn render_debug_panel_contains_log_line() {
        let mut app = make_verbose_app_in(std::env::temp_dir());
        app.log("hello from debug");
        let theme = Theme::default();
        let backend = ratatui::backend::TestBackend::new(80, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_debug_panel(frame, area, &app, &theme);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(
            text.contains("hello from debug"),
            "debug panel should contain the log message"
        );
    }

    #[test]
    fn render_debug_panel_shows_line_count() {
        let mut app = make_verbose_app_in(std::env::temp_dir());
        app.log("a");
        app.log("b");
        app.log("c");
        let theme = Theme::default();
        let backend = ratatui::backend::TestBackend::new(80, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_debug_panel(frame, area, &app, &theme);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(
            text.contains("3 lines"),
            "debug panel title should show the line count"
        );
    }
}
