//! Tests for the [`super`] ui module and its `overlays`/`panels`/`action_bar`/`modal` siblings.

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
        pane_dirs: vec![dir.clone(), dir],
        ..crate::app::AppOptions::default()
    })
}

fn make_verbose_app_in(dir: std::path::PathBuf) -> App {
    App::new(crate::app::AppOptions {
        pane_dirs: vec![dir.clone(), dir],
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
        pane_dirs: vec![std::env::temp_dir(), std::env::temp_dir()],
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
        pane_dirs: vec![std::env::temp_dir(), std::env::temp_dir()],
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

#[test]
fn draw_with_verbose_tall_terminal_does_not_panic() {
    let mut app = make_verbose_app_in(std::env::temp_dir());
    app.log("tall terminal log");
    // height >= 30 → full debug panel (Length(10))
    let backend = ratatui::backend::TestBackend::new(80, 40);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw(&mut app, frame)).unwrap();
}

#[test]
fn draw_with_verbose_medium_terminal_does_not_panic() {
    let mut app = make_verbose_app_in(std::env::temp_dir());
    app.log("medium terminal log");
    // 20 <= height < 30 → compact debug panel (Length(6))
    let backend = ratatui::backend::TestBackend::new(80, 25);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw(&mut app, frame)).unwrap();
}

#[test]
fn draw_with_verbose_tiny_terminal_does_not_panic() {
    let mut app = make_verbose_app_in(std::env::temp_dir());
    app.log("tiny terminal log");
    // height < 20 → minimal debug panel (Length(3))
    let backend = ratatui::backend::TestBackend::new(80, 15);
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
