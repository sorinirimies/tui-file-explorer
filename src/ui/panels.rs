use super::*;

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
    //   [3]  Toggles group cell       — 6 rows  (border + 4 rows + border)
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
            Constraint::Length(6), // [3] Toggles group (4 option rows)
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
        option_row("z", "show sizes", bool_span(app.active_pane().show_sizes)),
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
