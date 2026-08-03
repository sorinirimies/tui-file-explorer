use super::*;

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

    // Scroll thumb.
    crate::render::paint_scrollbar(frame, inner, total, start, theme.accent);
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
