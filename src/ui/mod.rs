//! Terminal UI drawing functions for the `tfe` binary.
//!
//! All [`ratatui`] rendering that is specific to the two-pane application
//! lives here. The per-pane widget rendering (header, list, footer) remains in
//! the library's own [`mod@crate::render`] module.
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

/// Arrangement of paired action-bar hint cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HintLayout {
    /// Stack hint cells when the available area is narrower than 90 columns.
    #[default]
    Auto,
    /// `Navigate | File Ops` and `Global | Status` remain side by side.
    Horizontal,
    /// Stack each pair vertically for narrow embedded layouts.
    Vertical,
}

impl HintLayout {
    pub fn resolve(self, width: u16) -> Self {
        match self {
            Self::Auto if width < 90 => Self::Vertical,
            Self::Auto => Self::Horizontal,
            layout => layout,
        }
    }

    pub fn action_bar_rows(self, width: u16) -> u16 {
        match self.resolve(width) {
            Self::Horizontal => 6,
            Self::Vertical => 12,
            Self::Auto => unreachable!(),
        }
    }

    pub fn row_height(self, width: u16) -> u16 {
        self.action_bar_rows(width) / 2
    }
}

/// Layout controls for embedding the full application UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppViewOptions {
    /// Render the six-row navigation and status area.
    pub show_action_bar: bool,
    /// Render the debug panel when `App::verbose` is enabled.
    pub show_debug_panel: bool,
    /// Width of the theme side panel.
    pub theme_panel_width: u16,
    /// Width of the options and editor side panels.
    pub settings_panel_width: u16,
    /// Percentage reserved for all explorer panes when preview is visible.
    pub panes_with_preview_percent: u16,
    /// Minimum target width per visible pane. Extra panes remain open and the
    /// visible window follows the active pane.
    pub minimum_pane_width: u16,
    /// Arrangement of action-bar hint cells.
    pub hint_layout: HintLayout,
}

impl Default for AppViewOptions {
    fn default() -> Self {
        Self {
            show_action_bar: true,
            show_debug_panel: true,
            theme_panel_width: 32,
            settings_panel_width: 42,
            panes_with_preview_percent: 50,
            minimum_pane_width: 24,
            hint_layout: HintLayout::Auto,
        }
    }
}

/// Draw the complete application into the full terminal frame.
pub fn draw(app: &mut App, frame: &mut Frame) {
    draw_in(app, frame, frame.area());
}

/// Draw the complete application inside `area`.
///
/// Use this when embedding the full file explorer beside other Ratatui widgets.
pub fn draw_in(app: &mut App, frame: &mut Frame, area: Rect) {
    draw_in_with_options(app, frame, area, AppViewOptions::default());
}

/// Draw the complete application inside `area` with custom layout options.
pub fn draw_in_with_options(app: &mut App, frame: &mut Frame, area: Rect, options: AppViewOptions) {
    if area.is_empty() {
        return;
    }
    if app.panes.is_empty() {
        return;
    }
    let theme = *app.theme();
    let full = area;

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

    let show_debug = app.verbose && options.show_debug_panel;
    let hint_layout = options.hint_layout.resolve(full.width);
    let action_bar_height = hint_layout.action_bar_rows(full.width);
    let action_row_height = hint_layout.row_height(full.width);
    let mut vertical_constraints = vec![Constraint::Min(0)];
    if show_debug {
        vertical_constraints.push(Constraint::Length(debug_height));
    }
    if options.show_action_bar {
        vertical_constraints.push(Constraint::Length(action_bar_height));
    }
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vertical_constraints)
        .split(full);

    let main_area = v_chunks[0];
    let action_area = options
        .show_action_bar
        .then(|| v_chunks[v_chunks.len() - 1]);

    // Split the action bar vertically into three rows of 3:
    //   row 0 — Navigate | File Ops
    //   row 1 — Global   | Status
    let action_rows = action_area.map(|area| {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(action_row_height),
                Constraint::Length(action_row_height),
            ])
            .split(area)
    });

    // ── Debug log panel (verbose only) ────────────────────────────────────────
    if show_debug {
        render_debug_panel(frame, v_chunks[1], app, &theme);
    }

    // Horizontal split: one column per visible pane | [preview] | [theme panel].
    let pane_capacity = (main_area.width / options.minimum_pane_width.max(1)).max(1) as usize;
    let visible_pane_count = if app.single_pane {
        1
    } else {
        app.panes.len().min(pane_capacity)
    };
    let visible_start = if app.single_pane {
        app.active_idx
    } else {
        app.active_idx
            .saturating_sub(visible_pane_count.saturating_sub(1))
            .min(app.panes.len().saturating_sub(visible_pane_count))
    };

    let mut h_constraints = vec![];
    if app.show_preview {
        // With preview: pane columns share 40% (single pane) or 50% total,
        // preview gets the rest.
        if app.single_pane {
            h_constraints.push(Constraint::Percentage(40));
        } else {
            let pct = (options.panes_with_preview_percent.min(100)
                / visible_pane_count.max(1) as u16)
                .max(1);
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
        h_constraints.push(Constraint::Length(options.theme_panel_width));
    }
    if app.show_options_panel {
        h_constraints.push(Constraint::Length(options.settings_panel_width));
    }
    if app.show_editor_panel {
        h_constraints.push(Constraint::Length(options.settings_panel_width));
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
        for (slot, pane_index) in (visible_start..visible_start + visible_pane_count).enumerate() {
            let pane_theme = if pane_index == active_idx {
                &active_theme
            } else {
                &inactive_theme
            };
            render_themed(
                &mut app.panes[pane_index],
                frame,
                h_chunks[slot],
                pane_theme,
            );
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
    if let Some(rows) = action_rows {
        render_nav_hints_with_layout(frame, rows[0], rows[1], app, &theme, hint_layout);
    }

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

pub use self::action_bar::{render_action_bar, render_nav_hints, render_nav_hints_with_layout};
#[cfg(test)]
pub use self::action_bar::{render_action_bar_spans, render_nav_hints_spans};
pub use self::modal::render_modal;
pub use self::overlays::{render_copy_progress, render_debug_panel, render_snackbar};
pub use self::panels::{render_editor_panel, render_options_panel, render_theme_panel};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
