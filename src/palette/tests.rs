//! Unit tests for [`super`].

use super::*;

// ── Palette constants ─────────────────────────────────────────────────────

#[test]
fn default_theme_brand_matches_constant() {
    assert_eq!(Theme::default().brand, C_BRAND);
}

#[test]
fn default_theme_accent_matches_constant() {
    assert_eq!(Theme::default().accent, C_ACCENT);
}

#[test]
fn default_theme_success_matches_constant() {
    assert_eq!(Theme::default().success, C_SUCCESS);
}

#[test]
fn default_theme_dim_matches_constant() {
    assert_eq!(Theme::default().dim, C_DIM);
}

#[test]
fn default_theme_fg_matches_constant() {
    assert_eq!(Theme::default().fg, C_FG);
}

#[test]
fn default_theme_sel_bg_matches_constant() {
    assert_eq!(Theme::default().sel_bg, C_SEL_BG);
}

#[test]
fn default_theme_dir_matches_constant() {
    assert_eq!(Theme::default().dir, C_DIR);
}

#[test]
fn default_theme_match_file_matches_constant() {
    assert_eq!(Theme::default().match_file, C_MATCH);
}

#[test]
fn default_theme_bg_matches_constant() {
    assert_eq!(Theme::default().bg, C_BG);
}

// ── Builder setters ───────────────────────────────────────────────────────

#[test]
fn builder_brand_overrides_field() {
    let color = Color::Rgb(1, 2, 3);
    let theme = Theme::default().brand(color);
    assert_eq!(theme.brand, color);
}

#[test]
fn builder_accent_overrides_field() {
    let color = Color::Rgb(4, 5, 6);
    let theme = Theme::default().accent(color);
    assert_eq!(theme.accent, color);
}

#[test]
fn builder_success_overrides_field() {
    let color = Color::Rgb(7, 8, 9);
    let theme = Theme::default().success(color);
    assert_eq!(theme.success, color);
}

#[test]
fn builder_dim_overrides_field() {
    let color = Color::Rgb(10, 11, 12);
    let theme = Theme::default().dim(color);
    assert_eq!(theme.dim, color);
}

#[test]
fn builder_fg_overrides_field() {
    let color = Color::Rgb(13, 14, 15);
    let theme = Theme::default().fg(color);
    assert_eq!(theme.fg, color);
}

#[test]
fn builder_sel_bg_overrides_field() {
    let color = Color::Rgb(16, 17, 18);
    let theme = Theme::default().sel_bg(color);
    assert_eq!(theme.sel_bg, color);
}

#[test]
fn builder_dir_overrides_field() {
    let color = Color::Rgb(19, 20, 21);
    let theme = Theme::default().dir(color);
    assert_eq!(theme.dir, color);
}

#[test]
fn builder_match_file_overrides_field() {
    let color = Color::Rgb(22, 23, 24);
    let theme = Theme::default().match_file(color);
    assert_eq!(theme.match_file, color);
}

#[test]
fn builder_bg_overrides_field() {
    let color = Color::Rgb(25, 26, 27);
    let theme = Theme::default().bg(color);
    assert_eq!(theme.bg, color);
}

#[test]
fn builder_chained_overrides_multiple_fields() {
    let brand = Color::Rgb(1, 0, 0);
    let accent = Color::Rgb(0, 1, 0);
    let theme = Theme::default().brand(brand).accent(accent);
    assert_eq!(theme.brand, brand);
    assert_eq!(theme.accent, accent);
    // Unmodified fields stay at their defaults.
    assert_eq!(theme.dim, C_DIM);
}

#[test]
fn builder_does_not_mutate_other_fields() {
    let original = Theme::default();
    let modified = original.brand(Color::Red);
    // All other fields survive unchanged.
    assert_eq!(modified.accent, original.accent);
    assert_eq!(modified.success, original.success);
    assert_eq!(modified.dim, original.dim);
    assert_eq!(modified.fg, original.fg);
    assert_eq!(modified.sel_bg, original.sel_bg);
    assert_eq!(modified.dir, original.dir);
    assert_eq!(modified.match_file, original.match_file);
    assert_eq!(modified.bg, original.bg);
}

// ── Presets ───────────────────────────────────────────────────────────────

#[test]
fn presets_mirror_the_nuances_catalogue() {
    let presets = Theme::presets();
    let names = tui_nuances::Theme::names();
    assert_eq!(presets.len(), names.len());
    assert!(presets.len() >= 90);
    for ((name, theme), want) in presets.iter().zip(names) {
        assert_eq!(*name, want);
        let shared = tui_nuances::Theme::by_name(want).unwrap();
        assert_eq!(*theme, Theme::from_nuances(&shared));
    }
}

#[test]
fn presets_names_are_unique() {
    let presets = Theme::presets();
    let mut seen = std::collections::HashSet::new();
    for (name, _) in &presets {
        assert!(seen.insert(*name), "duplicate preset name: '{name}'");
    }
}

#[test]
fn theme_clone_equals_original() {
    let t = Theme::from(&tui_nuances::Theme::dracula());
    assert_eq!(t.clone(), t);
}

#[test]
fn theme_partial_eq_reflexive() {
    let t = Theme::from(&tui_nuances::Theme::catppuccin_mocha());
    assert_eq!(t, t.clone());
}
