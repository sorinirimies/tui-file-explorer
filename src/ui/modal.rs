use super::*;

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
