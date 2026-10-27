//! The built-in theme catalogue — every `tui-nuances` built-in, as an `impl` block on [`Theme`].

use super::*;

impl Theme {
    /// Every built-in `tui-nuances` theme as `(name, Theme)`, in catalogue order.
    ///
    /// `tui-nuances` is the single source of truth: no colours are defined here.
    ///
    /// ```
    /// use tui_file_explorer::Theme;
    ///
    /// for (name, _theme) in Theme::presets() {
    ///     println!("{name}");
    /// }
    /// assert!(Theme::presets().len() >= 90);
    /// ```
    #[must_use]
    pub fn presets() -> Vec<(&'static str, Theme)> {
        crate::nuances::NuancesTheme::names()
            .into_iter()
            .filter_map(|n| {
                crate::nuances::NuancesTheme::by_name(n).map(|t| (n, Self::from_nuances(&t)))
            })
            .collect()
    }
}
