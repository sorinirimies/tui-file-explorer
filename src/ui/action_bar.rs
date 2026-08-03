use super::*;

// ── Action bar ────────────────────────────────────────────────────────────────

/// Render the two hint rows and the status bar of the action area.
///
/// Layout (each row is 3 terminal rows tall):
///   Row 0  ╭─ Navigate ──────────────────╮╭─ File Ops ──────────────────╮
///   Row 1  ╭─ Global ────────────────────╮╭─ Status ────────────────────╮
pub fn render_nav_hints(frame: &mut Frame, row0: Rect, row1: Rect, app: &App, theme: &Theme) {
    let k = |s: &'static str| key_span(s, theme);
    let d = |s: &'static str| dim_span(s, theme);

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
        k("i"),
        d(" edit │ "),
        k("P"),
        d(" preview │ "),
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
        k("C-t"),
        d("/"),
        k("C-w"),
        d(" open/close pane │ "),
        k("w"),
        d(" split │ "),
        k("["),
        d("/"),
        k("t"),
        d(" theme │ "),
        k("Shift+E"),
        d(" editor │ "),
        k("Shift+O"),
        d(" options │ "),
        k("C-j"),
        d("/"),
        k("C-k"),
        d(" scroll preview"),
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
    let k = |s: &'static str| key_span(s, theme);
    let d = |s: &'static str| dim_span(s, theme);
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
    let active_label = format!("{}/{}", app.active_idx + 1, app.panes.len());

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
    let k = |s: &'static str| key_span(s, theme);
    let d = |s: &'static str| dim_span(s, theme);
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
