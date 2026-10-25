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

// ── Named presets ─────────────────────────────────────────────────────────

#[test]
fn all_presets_is_non_empty() {
    assert!(!Theme::all_presets().is_empty());
}

#[test]
fn all_presets_names_are_non_empty() {
    for (name, _, _) in Theme::all_presets() {
        assert!(!name.is_empty(), "preset has an empty name");
    }
}

#[test]
fn all_presets_descriptions_are_non_empty() {
    for (name, desc, _) in Theme::all_presets() {
        assert!(!desc.is_empty(), "preset '{name}' has an empty description");
    }
}

#[test]
fn all_presets_names_are_unique() {
    let presets = Theme::all_presets();
    let mut seen = std::collections::HashSet::new();
    for (name, _, _) in &presets {
        assert!(seen.insert(*name), "duplicate preset name: '{name}'");
    }
}

#[test]
fn all_presets_first_entry_is_default() {
    let presets = Theme::all_presets();
    let (name, _, theme) = &presets[0];
    assert_eq!(*name, "Default");
    assert_eq!(*theme, Theme::default());
}

#[test]
fn all_presets_contains_dracula() {
    let names: Vec<&str> = Theme::all_presets().iter().map(|(n, _, _)| *n).collect();
    assert!(names.contains(&"Dracula"), "Dracula preset missing");
}

#[test]
fn all_presets_contains_nord() {
    let names: Vec<&str> = Theme::all_presets().iter().map(|(n, _, _)| *n).collect();
    assert!(names.contains(&"Nord"), "Nord preset missing");
}

#[test]
fn all_presets_contains_catppuccin_mocha() {
    let names: Vec<&str> = Theme::all_presets().iter().map(|(n, _, _)| *n).collect();
    assert!(
        names.contains(&"Catppuccin Mocha"),
        "Catppuccin Mocha preset missing"
    );
}

#[test]
fn all_presets_count_is_at_least_43() {
    assert!(
        Theme::all_presets().len() >= 43,
        "expected at least 43 presets, got {}",
        Theme::all_presets().len()
    );
}

#[test]
fn named_preset_dracula_differs_from_default() {
    assert_ne!(Theme::dracula(), Theme::default());
}

#[test]
fn named_preset_nord_differs_from_dracula() {
    assert_ne!(Theme::nord(), Theme::dracula());
}

#[test]
fn theme_clone_equals_original() {
    let t = Theme::dracula();
    assert_eq!(t.clone(), t);
}

#[test]
fn theme_partial_eq_reflexive() {
    let t = Theme::catppuccin_mocha();
    assert_eq!(t, t.clone());
}

#[test]
fn every_preset_is_sourced_from_shared_nuances_and_readable() {
    // Presets (other than the explorer-only Default/Mono) come from tui-nuances,
    // so each must round-trip through the shared mapping.
    for (name, _, t) in Theme::all_presets() {
        if name == "Default" || name == "Mono" {
            continue;
        }
        assert_ne!(t.fg, t.sel_bg, "{name}: fg equals selection bg");
        assert_ne!(t.brand, t.bg, "{name}: brand invisible on bg");
    }
}

#[test]
fn dark_presets_keep_terminal_background_and_light_ones_paint() {
    assert_eq!(Theme::dracula().bg, Color::Reset);
    assert_eq!(Theme::catppuccin_mocha().bg, Color::Reset);
    assert!(matches!(Theme::solarized_light().bg, Color::Rgb(..)));
    assert!(matches!(Theme::gruvbox_light().bg, Color::Rgb(..)));
}

#[test]
fn preset_roles_match_previous_look() {
    // Pinned values from before the move to tui-nuances.
    let d = Theme::dracula();
    assert_eq!(d.brand, Color::Rgb(255, 121, 198));
    assert_eq!(d.dir, Color::Rgb(241, 250, 140));
    assert_eq!(d.accent, Color::Rgb(139, 233, 253));
    assert_eq!(d.sel_bg, Color::Rgb(68, 71, 90));
}
