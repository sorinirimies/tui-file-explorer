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
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CopyProgress, Modal, Snackbar};
use crate::inline_editor::render_inline_editor;
use crate::preview::render_preview;
use tui_slider::{style::SliderStyle, Slider, SliderOrientation, SliderState};

// ── Styled-span helpers ───────────────────────────────────────────────────────

/// Create a bold-accent styled span for key-binding labels.
fn key_span<'a>(s: &'a str, theme: &Theme) -> Span<'a> {
    Span::styled(
        s,
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )
}

/// Create a dim styled span for descriptions.
fn dim_span<'a>(s: &'a str, theme: &Theme) -> Span<'a> {
    Span::styled(s, Style::default().fg(theme.dim))
}

// ── Top-level draw ────────────────────────────────────────────────────────────

/// Draw the entire application UI into `frame`.
///
/// Divides the terminal area into:
/// - A main area (one or two explorer panes + optional theme panel).
/// - A fixed-height action bar at the bottom.
/// - An optional modal overlay on top of everything.
pub fn draw(app: &mut App, frame: &mut Frame) {
    let theme = *app.theme();
    let full = frame.area();

    // Paint the entire terminal area with the theme's background colour.
    // Without this, light themes appear broken because ratatui defaults
    // unstyled cells to Color::Reset (the terminal's own background).
    if theme.bg != Color::Reset {
        frame.render_widget(Block::default().style(Style::default().bg(theme.bg)), full);
    }

    // ── Inline editor takes over the entire screen ────────────────────────────
    if let Some(ref editor) = app.inline_editor {
        render_inline_editor(frame, full, editor, &theme);
        return;
    }

    // Vertical split: main area | [debug log panel] | action bar.
    // The debug panel only appears when --verbose is active.
    let debug_height = if full.height >= 30 {
        10
    } else if full.height >= 20 {
        6
    } else {
        3
    };

    let v_chunks = if app.verbose {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(debug_height),
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

    // Horizontal split: one column per visible pane | [preview] | [theme panel].
    let visible_pane_count = if app.single_pane { 1 } else { app.panes.len() };

    let mut h_constraints = vec![];
    if app.show_preview {
        // With preview: pane columns share 40% (single pane) or 50% total,
        // preview gets the rest.
        if app.single_pane {
            h_constraints.push(Constraint::Percentage(40));
        } else {
            let pct = (50 / visible_pane_count.max(1)) as u16;
            for _ in 0..visible_pane_count {
                h_constraints.push(Constraint::Percentage(pct));
            }
        }
        h_constraints.push(Constraint::Min(0)); // Preview takes remaining space
    } else {
        let pct = (100 / visible_pane_count.max(1)) as u16;
        for _ in 0..visible_pane_count {
            h_constraints.push(Constraint::Percentage(pct));
        }
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

    // ── Panes ─────────────────────────────────────────────────────────
    let active_theme = theme;
    let inactive_theme = theme.accent(theme.dim).brand(theme.dim);

    // Sync the current theme name and editor label into every pane so
    // render_header can display them. Only allocate when the value actually
    // changed to avoid extra string allocations per frame.
    let theme_changed = app.panes[0].theme_name != app.theme_name();
    if theme_changed {
        let theme_name = app.theme_name().to_string();
        for p in app.panes.iter_mut() {
            p.theme_name = theme_name.clone();
        }
    }

    let editor_label = if app.editor == crate::app::Editor::None {
        ""
    } else {
        app.editor.label()
    };
    if app.panes[0].editor_name != editor_label {
        let editor_name = editor_label.to_string();
        for p in app.panes.iter_mut() {
            p.editor_name = editor_name.clone();
        }
    }

    let active_idx = app.active_idx;
    if app.single_pane {
        render_themed(
            &mut app.panes[active_idx],
            frame,
            h_chunks[0],
            &active_theme,
        );
    } else {
        for (i, pane) in app.panes.iter_mut().enumerate() {
            let pane_theme = if i == active_idx {
                &active_theme
            } else {
                &inactive_theme
            };
            render_themed(pane, frame, h_chunks[i], pane_theme);
        }
    }

    // ── Preview panel ────────────────────────────────
    if app.show_preview {
        let preview_idx = visible_pane_count;
        let preview_area = h_chunks[preview_idx];

        // Update preview state with the currently highlighted entry.
        let current_path = app.active_pane().current_entry().map(|e| e.path.clone());
        app.preview_state.update(
            current_path.as_deref(),
            preview_area.width,
            preview_area.height,
        );

        render_preview(frame, preview_area, &app.preview_state, &theme);
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

mod action_bar;
mod modal;
mod overlays;
mod panels;

pub use self::action_bar::{render_action_bar, render_nav_hints};
#[cfg(test)]
pub use self::action_bar::{render_action_bar_spans, render_nav_hints_spans};
pub use self::modal::render_modal;
pub use self::overlays::{render_copy_progress, render_debug_panel, render_snackbar};
pub use self::panels::{render_editor_panel, render_options_panel, render_theme_panel};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
