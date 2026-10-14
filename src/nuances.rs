//! Shared theming via [`tui_nuances`].
//!
//! The explorer's own [`Theme`] (nine colours) stays the style type used by
//! every `render_*` function. This module bridges it to the shared
//! [`NuancesTheme`] so all of an app's TUI components can use one theme:
//!
//! ```
//! use tui_file_explorer::{NuancesTheme, Theme, ThemeBuilder};
//! use ratatui::style::Color;
//!
//! let shared = NuancesTheme::nord();
//! let plain: Theme = (&shared).into();                       // From<&NuancesTheme>
//! let tuned = ThemeBuilder::new().theme(&shared).brand(Color::Magenta).build();
//! assert_eq!(tuned.brand, Color::Magenta);                   // explicit wins
//! assert_eq!(tuned.accent, plain.accent);                    // rest from theme
//! ```
//!
//! # Role mapping
//!
//! | explorer | shared |
//! |---|---|
//! | `brand` | `title` |
//! | `accent` | `accent` |
//! | `success`, `match_file` | `success` |
//! | `dim` | `muted` |
//! | `fg` | `foreground` |
//! | `sel_bg` | `selection_bg` |
//! | `dir` | `primary` |
//! | `bg` | `background` (`Color::Reset` when `None`) |
//!
//! # Precedence
//!
//! With [`ThemeBuilder`], explicit per-field colours always win over the
//! shared theme, regardless of call order. Without any theme the default look
//! is unchanged.

use ratatui::style::Color;
pub use tui_nuances::{
    Theme as NuancesTheme, ThemeProvider, ThemeSwitcher as NuancesSwitcher, Themed,
};

use crate::palette::Theme;

impl Theme {
    /// Map a shared [`NuancesTheme`] onto the explorer's colour roles.
    #[must_use]
    pub fn from_nuances(t: &NuancesTheme) -> Self {
        Self {
            brand: t.title,
            accent: t.accent,
            success: t.success,
            dim: t.muted,
            fg: t.foreground,
            sel_bg: t.selection_bg,
            dir: t.primary,
            match_file: t.success,
            bg: t.background.unwrap_or(Color::Reset),
        }
    }

    /// Build from any [`ThemeProvider`] (e.g. an application theme type).
    #[must_use]
    pub fn from_provider(p: &impl ThemeProvider) -> Self {
        Self::from_nuances(&p.nuance_theme())
    }
}

impl From<&NuancesTheme> for Theme {
    fn from(t: &NuancesTheme) -> Self {
        Self::from_nuances(t)
    }
}

impl From<NuancesTheme> for Theme {
    fn from(t: NuancesTheme) -> Self {
        Self::from_nuances(&t)
    }
}

/// Replaces every colour with the mapped shared theme.
impl Themed for Theme {
    fn apply_theme(self, theme: &NuancesTheme) -> Self {
        Self::from_nuances(theme)
    }
}

macro_rules! builder_fields {
    ($($f:ident),* $(,)?) => {
        /// Layered theme builder: shared theme as base, explicit colours on top.
        ///
        /// Explicit colours win over the shared theme in any call order.
        #[derive(Debug, Clone, Default)]
        pub struct ThemeBuilder {
            base: Option<Theme>,
            $($f: Option<Color>,)*
        }

        impl ThemeBuilder {
            $(
                #[doc = concat!("Explicitly set `", stringify!($f), "` (wins over the shared theme).")]
                #[must_use]
                pub fn $f(mut self, color: Color) -> Self {
                    self.$f = Some(color);
                    self
                }
            )*

            /// Resolve to a concrete [`Theme`]: defaults < shared theme < explicit.
            #[must_use]
            pub fn build(&self) -> Theme {
                let mut t = self.base.unwrap_or_default();
                $(if let Some(c) = self.$f { t.$f = c; })*
                t
            }
        }
    };
}

builder_fields!(brand, accent, success, dim, fg, sel_bg, dir, match_file, bg);

impl ThemeBuilder {
    /// Start from the default look.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use a shared theme as the base layer.
    #[must_use]
    pub fn theme(mut self, theme: &NuancesTheme) -> Self {
        self.base = Some(Theme::from_nuances(theme));
        self
    }
}

impl Themed for ThemeBuilder {
    fn apply_theme(self, theme: &NuancesTheme) -> Self {
        self.theme(theme)
    }
}

impl From<ThemeBuilder> for Theme {
    fn from(b: ThemeBuilder) -> Self {
        b.build()
    }
}

/// [`render_themed`](crate::render_themed) with a shared theme.
pub fn render_nuanced(
    explorer: &mut crate::FileExplorer,
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    theme: &NuancesTheme,
) {
    crate::render_themed(explorer, frame, area, &Theme::from_nuances(theme));
}

/// [`render_dual_pane_themed`](crate::render_dual_pane_themed) with a shared theme.
pub fn render_dual_pane_nuanced(
    dual: &mut crate::DualPane,
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    theme: &NuancesTheme,
) {
    crate::render_dual_pane_themed(dual, frame, area, &Theme::from_nuances(theme));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn mapping() {
        let n = NuancesTheme::dracula();
        let t = Theme::from_nuances(&n);
        assert_eq!(t.brand, n.title);
        assert_eq!(t.dim, n.muted);
        assert_eq!(t.sel_bg, n.selection_bg);
        assert_eq!(t.dir, n.primary);
        assert_eq!(t.bg, n.background.unwrap_or(Color::Reset));
        assert_eq!(Theme::from(&n), Theme::from(n.clone()));
        assert_eq!(Theme::default().apply_theme(&n), t);
    }

    #[test]
    fn no_theme_is_default() {
        assert_eq!(ThemeBuilder::new().build(), Theme::default());
    }

    #[test]
    fn explicit_wins_any_order() {
        let n = NuancesTheme::nord();
        let a = ThemeBuilder::new().brand(Color::Red).theme(&n).build();
        let b = ThemeBuilder::new().theme(&n).brand(Color::Red).build();
        assert_eq!(a, b);
        assert_eq!(a.brand, Color::Red);
        assert_eq!(a.accent, n.accent);
    }

    #[test]
    fn every_builtin_renders() {
        for n in NuancesTheme::all() {
            let mut term = Terminal::new(TestBackend::new(60, 14)).unwrap();
            let mut ex = crate::FileExplorer::new(std::env::temp_dir(), vec![]);
            term.draw(|f| render_nuanced(&mut ex, f, f.area(), &n))
                .unwrap();
            let mut dual = crate::DualPane::builder(std::env::temp_dir()).build();
            term.draw(|f| render_dual_pane_nuanced(&mut dual, f, f.area(), &n))
                .unwrap();
        }
    }
}
