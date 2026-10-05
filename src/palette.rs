//! Default colour palette used by the file-explorer widget.
//!
//! Every constant is `pub` so that downstream crates can reference the same
//! values when building complementary widgets.
//!
//! For full visual customisation pass a [`Theme`] to [`crate::render_themed`]
//! instead of the zero-argument [`crate::render()`].
//!
//! ## Named presets
//!
//! A collection of well-known editor / terminal themes is available as
//! associated constructors on [`Theme`]:
//!
//! ```no_run
//! use tui_file_explorer::Theme;
//!
//! let t = Theme::grape();
//! let t = Theme::catppuccin_mocha();
//! let t = Theme::tokyo_night();
//! let t = Theme::gruvbox_dark();
//!
//! // Or iterate the full catalogue (name, description, theme):
//! for (name, desc, _theme) in Theme::all_presets() {
//!     println!("{name} — {desc}");
//! }
//! ```

use ratatui::style::Color;

// ── Palette constants (defaults) ──────────────────────────────────────────────

/// Brand / accent orange — used for the widget title.
pub const C_BRAND: Color = Color::Rgb(255, 100, 30);
/// Cyan accent — used for borders and the path display.
pub const C_ACCENT: Color = Color::Rgb(80, 200, 255);
/// Green success — used for selectable files and the status bar.
pub const C_SUCCESS: Color = Color::Rgb(80, 220, 120);
/// Muted grey — used for dimmed text and the footer hints.
pub const C_DIM: Color = Color::Rgb(120, 120, 130);
/// Default foreground white.
pub const C_FG: Color = Color::White;
/// Background colour for the selected / highlighted row.
pub const C_SEL_BG: Color = Color::Rgb(40, 60, 80);
/// Yellow — used for directory names.
pub const C_DIR: Color = Color::Rgb(255, 210, 80);
/// Green — used for files that match the extension filter.
pub const C_MATCH: Color = Color::Rgb(80, 220, 120);
/// Default background (terminal default / transparent).
pub const C_BG: Color = Color::Reset;

// ── Theme ─────────────────────────────────────────────────────────────────────

/// A complete colour theme for the file-explorer widget.
///
/// Construct one with [`Theme::default()`] to get the built-in palette, then
/// override individual fields as needed, or build one entirely from scratch.
///
/// Pass a reference to [`crate::render_themed`] to apply your theme.
///
/// # Example
///
/// ```no_run
/// use tui_file_explorer::{render_themed, Theme};
/// use ratatui::style::Color;
///
/// let mut theme = Theme::default();
/// theme.brand  = Color::Magenta;
/// theme.accent = Color::Cyan;
///
/// // terminal.draw(|frame| {
/// //     render_themed(&mut explorer, frame, frame.area(), &theme);
/// // });
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Widget title colour (e.g. "📁 File Explorer").
    pub brand: Color,
    /// Border and current-path colour.
    pub accent: Color,
    /// Selectable-file and status-bar colour.
    pub success: Color,
    /// Dimmed text (footer hints, non-matching files, file sizes).
    pub dim: Color,
    /// Default foreground (icons for plain files).
    pub fg: Color,
    /// Background of the highlighted / selected row.
    pub sel_bg: Color,
    /// Directory name colour.
    pub dir: Color,
    /// Colour for files that match the active extension filter.
    pub match_file: Color,
    /// Overall background colour.  `Color::Reset` inherits the terminal's
    /// own background; light themes should set an explicit light colour.
    pub bg: Color,
}

impl Default for Theme {
    /// Returns the built-in palette (same colours as the palette constants).
    fn default() -> Self {
        Self {
            brand: C_BRAND,
            accent: C_ACCENT,
            success: C_SUCCESS,
            dim: C_DIM,
            fg: C_FG,
            sel_bg: C_SEL_BG,
            dir: C_DIR,
            match_file: C_MATCH,
            bg: C_BG,
        }
    }
}

impl Theme {
    /// Override the brand colour and return `self` (builder-style).
    #[must_use]
    pub fn brand(mut self, color: Color) -> Self {
        self.brand = color;
        self
    }

    /// Override the accent colour and return `self` (builder-style).
    #[must_use]
    pub fn accent(mut self, color: Color) -> Self {
        self.accent = color;
        self
    }

    /// Override the success colour and return `self` (builder-style).
    #[must_use]
    pub fn success(mut self, color: Color) -> Self {
        self.success = color;
        self
    }

    /// Override the dim colour and return `self` (builder-style).
    #[must_use]
    pub fn dim(mut self, color: Color) -> Self {
        self.dim = color;
        self
    }

    /// Override the foreground colour and return `self` (builder-style).
    #[must_use]
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Override the selection-background colour and return `self` (builder-style).
    #[must_use]
    pub fn sel_bg(mut self, color: Color) -> Self {
        self.sel_bg = color;
        self
    }

    /// Override the directory colour and return `self` (builder-style).
    #[must_use]
    pub fn dir(mut self, color: Color) -> Self {
        self.dir = color;
        self
    }

    /// Override the matched-file colour and return `self` (builder-style).
    #[must_use]
    pub fn match_file(mut self, color: Color) -> Self {
        self.match_file = color;
        self
    }

    /// Override the background colour and return `self` (builder-style).
    #[must_use]
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }
}

mod presets;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
