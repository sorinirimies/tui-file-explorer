//! Default colour palette used by the file-explorer widget.
//!
//! Every constant is `pub` so that downstream crates can reference the same
//! values when building complementary widgets.
//!
//! For full visual customisation pass a [`Theme`] to [`crate::render_themed`]
//! instead of the zero-argument [`crate::render`].
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
#[derive(Debug, Clone, PartialEq)]
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
        }
    }
}

impl Theme {
    /// Convenience constructor — identical to `Theme::default()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the brand colour and return `self` (builder-style).
    pub fn brand(mut self, color: Color) -> Self {
        self.brand = color;
        self
    }

    /// Override the accent colour and return `self` (builder-style).
    pub fn accent(mut self, color: Color) -> Self {
        self.accent = color;
        self
    }

    /// Override the success colour and return `self` (builder-style).
    pub fn success(mut self, color: Color) -> Self {
        self.success = color;
        self
    }

    /// Override the dim colour and return `self` (builder-style).
    pub fn dim(mut self, color: Color) -> Self {
        self.dim = color;
        self
    }

    /// Override the foreground colour and return `self` (builder-style).
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Override the selection-background colour and return `self` (builder-style).
    pub fn sel_bg(mut self, color: Color) -> Self {
        self.sel_bg = color;
        self
    }

    /// Override the directory colour and return `self` (builder-style).
    pub fn dir(mut self, color: Color) -> Self {
        self.dir = color;
        self
    }

    /// Override the matched-file colour and return `self` (builder-style).
    pub fn match_file(mut self, color: Color) -> Self {
        self.match_file = color;
        self
    }
}

// ── Named presets ─────────────────────────────────────────────────────────────

impl Theme {
    // ── Dark themes ───────────────────────────────────────────────────────────

    /// [Dracula](https://draculatheme.com/) — pink, cyan, purple on dark grey.
    pub fn dracula() -> Self {
        Self {
            brand: Color::Rgb(255, 121, 198),     // Pink
            accent: Color::Rgb(139, 233, 253),    // Cyan
            dir: Color::Rgb(241, 250, 140),       // Yellow
            sel_bg: Color::Rgb(68, 71, 90),       // Current Line
            success: Color::Rgb(80, 250, 123),    // Green
            match_file: Color::Rgb(80, 250, 123), // Green
            dim: Color::Rgb(98, 114, 164),        // Comment
            fg: Color::Rgb(248, 248, 242),        // Foreground
        }
    }

    /// [Nord](https://www.nordtheme.com/) — arctic, bluish tones.
    pub fn nord() -> Self {
        Self {
            brand: Color::Rgb(136, 192, 208),      // Nord8  – light blue
            accent: Color::Rgb(129, 161, 193),     // Nord9  – blue
            dir: Color::Rgb(235, 203, 139),        // Nord13 – yellow
            sel_bg: Color::Rgb(59, 66, 82),        // Nord1
            success: Color::Rgb(163, 190, 140),    // Nord14 – green
            match_file: Color::Rgb(163, 190, 140), // Nord14 – green
            dim: Color::Rgb(76, 86, 106),          // Nord3
            fg: Color::Rgb(216, 222, 233),         // Nord4
        }
    }

    /// [Solarized Dark](https://ethanschoonover.com/solarized/).
    pub fn solarized_dark() -> Self {
        Self {
            brand: Color::Rgb(38, 139, 210),     // Blue
            accent: Color::Rgb(42, 161, 152),    // Cyan
            dir: Color::Rgb(181, 137, 0),        // Yellow
            sel_bg: Color::Rgb(7, 54, 66),       // Base02
            success: Color::Rgb(133, 153, 0),    // Green
            match_file: Color::Rgb(133, 153, 0), // Green
            dim: Color::Rgb(88, 110, 117),       // Base01
            fg: Color::Rgb(131, 148, 150),       // Base0
        }
    }

    /// [Solarized Light](https://ethanschoonover.com/solarized/).
    pub fn solarized_light() -> Self {
        Self {
            brand: Color::Rgb(38, 139, 210),     // Blue
            accent: Color::Rgb(42, 161, 152),    // Cyan
            dir: Color::Rgb(181, 137, 0),        // Yellow
            sel_bg: Color::Rgb(238, 232, 213),   // Base2
            success: Color::Rgb(133, 153, 0),    // Green
            match_file: Color::Rgb(0, 110, 100), // Darker cyan for light bg
            dim: Color::Rgb(147, 161, 161),      // Base1
            fg: Color::Rgb(101, 123, 131),       // Base00
        }
    }

    /// [Gruvbox Dark](https://github.com/morhetz/gruvbox).
    pub fn gruvbox_dark() -> Self {
        Self {
            brand: Color::Rgb(254, 128, 25),       // Bright Orange
            accent: Color::Rgb(250, 189, 47),      // Bright Yellow
            dir: Color::Rgb(250, 189, 47),         // Bright Yellow
            sel_bg: Color::Rgb(60, 56, 54),        // bg1
            success: Color::Rgb(184, 187, 38),     // Bright Green
            match_file: Color::Rgb(142, 192, 124), // Bright Aqua
            dim: Color::Rgb(146, 131, 116),        // Gray
            fg: Color::Rgb(235, 219, 178),         // fg
        }
    }

    /// [Gruvbox Light](https://github.com/morhetz/gruvbox).
    pub fn gruvbox_light() -> Self {
        Self {
            brand: Color::Rgb(214, 93, 14),        // Orange (dark variant)
            accent: Color::Rgb(215, 153, 33),      // Yellow (dark variant)
            dir: Color::Rgb(181, 118, 20),         // Dark Yellow
            sel_bg: Color::Rgb(213, 196, 161),     // bg2
            success: Color::Rgb(121, 116, 14),     // Dark Green
            match_file: Color::Rgb(104, 157, 106), // Dark Aqua
            dim: Color::Rgb(146, 131, 116),        // Gray
            fg: Color::Rgb(60, 56, 54),            // fg1
        }
    }

    /// [Catppuccin Latte](https://github.com/catppuccin/catppuccin) — light variant.
    pub fn catppuccin_latte() -> Self {
        Self {
            brand: Color::Rgb(136, 57, 239),      // Mauve
            accent: Color::Rgb(30, 102, 245),     // Blue
            dir: Color::Rgb(254, 100, 11),        // Peach
            sel_bg: Color::Rgb(204, 208, 218),    // Surface0
            success: Color::Rgb(64, 160, 43),     // Green
            match_file: Color::Rgb(23, 146, 153), // Teal
            dim: Color::Rgb(156, 160, 176),       // Overlay0
            fg: Color::Rgb(76, 79, 105),          // Text
        }
    }

    /// [Catppuccin Frappé](https://github.com/catppuccin/catppuccin) — medium-dark variant.
    pub fn catppuccin_frappe() -> Self {
        Self {
            brand: Color::Rgb(202, 158, 230),      // Mauve
            accent: Color::Rgb(140, 170, 238),     // Blue
            dir: Color::Rgb(229, 200, 144),        // Yellow
            sel_bg: Color::Rgb(65, 69, 89),        // Surface0
            success: Color::Rgb(166, 209, 137),    // Green
            match_file: Color::Rgb(129, 200, 190), // Teal
            dim: Color::Rgb(115, 121, 148),        // Overlay0
            fg: Color::Rgb(198, 208, 245),         // Text
        }
    }

    /// [Catppuccin Macchiato](https://github.com/catppuccin/catppuccin) — dark variant.
    pub fn catppuccin_macchiato() -> Self {
        Self {
            brand: Color::Rgb(198, 160, 246),      // Mauve
            accent: Color::Rgb(138, 173, 244),     // Blue
            dir: Color::Rgb(238, 212, 159),        // Yellow
            sel_bg: Color::Rgb(54, 58, 79),        // Surface0
            success: Color::Rgb(166, 218, 149),    // Green
            match_file: Color::Rgb(139, 213, 202), // Teal
            dim: Color::Rgb(110, 115, 141),        // Overlay0
            fg: Color::Rgb(202, 211, 245),         // Text
        }
    }

    /// [Catppuccin Mocha](https://github.com/catppuccin/catppuccin) — darkest variant.
    pub fn catppuccin_mocha() -> Self {
        Self {
            brand: Color::Rgb(203, 166, 247),      // Mauve
            accent: Color::Rgb(137, 180, 250),     // Blue
            dir: Color::Rgb(249, 226, 175),        // Yellow
            sel_bg: Color::Rgb(49, 50, 68),        // Surface0
            success: Color::Rgb(166, 227, 161),    // Green
            match_file: Color::Rgb(148, 226, 213), // Teal
            dim: Color::Rgb(108, 112, 134),        // Overlay0
            fg: Color::Rgb(205, 214, 244),         // Text
        }
    }

    /// [Tokyo Night](https://github.com/folke/tokyonight.nvim) — dark blue/purple night.
    pub fn tokyo_night() -> Self {
        Self {
            brand: Color::Rgb(187, 154, 247),      // Purple
            accent: Color::Rgb(122, 162, 247),     // Blue
            dir: Color::Rgb(224, 175, 104),        // Yellow/Gold
            sel_bg: Color::Rgb(41, 46, 66),        // Slightly lighter than bg
            success: Color::Rgb(158, 206, 106),    // Green
            match_file: Color::Rgb(115, 218, 202), // Teal
            dim: Color::Rgb(86, 95, 137),          // Comment
            fg: Color::Rgb(192, 202, 245),         // Foreground
        }
    }

    /// [Tokyo Night Storm](https://github.com/folke/tokyonight.nvim) — slightly lighter dark.
    pub fn tokyo_night_storm() -> Self {
        Self {
            brand: Color::Rgb(187, 154, 247),      // Purple
            accent: Color::Rgb(122, 162, 247),     // Blue
            dir: Color::Rgb(224, 175, 104),        // Yellow/Gold
            sel_bg: Color::Rgb(45, 49, 75),        // Slightly lighter than bg
            success: Color::Rgb(158, 206, 106),    // Green
            match_file: Color::Rgb(115, 218, 202), // Teal
            dim: Color::Rgb(86, 95, 137),          // Comment
            fg: Color::Rgb(192, 202, 245),         // Foreground
        }
    }

    /// [Tokyo Night Light](https://github.com/folke/tokyonight.nvim) — light variant.
    pub fn tokyo_night_light() -> Self {
        Self {
            brand: Color::Rgb(90, 74, 120),      // Dark Purple
            accent: Color::Rgb(46, 126, 233),    // Blue
            dir: Color::Rgb(140, 108, 62),       // Dark Yellow
            sel_bg: Color::Rgb(208, 213, 227),   // Highlight
            success: Color::Rgb(72, 94, 48),     // Dark Green
            match_file: Color::Rgb(15, 75, 110), // Dark Teal
            dim: Color::Rgb(132, 140, 176),      // Muted
            fg: Color::Rgb(52, 59, 88),          // Foreground
        }
    }

    /// [Kanagawa Wave](https://github.com/rebelot/kanagawa.nvim) — deep blue ink.
    pub fn kanagawa_wave() -> Self {
        Self {
            brand: Color::Rgb(210, 126, 153),      // Sakura Pink
            accent: Color::Rgb(126, 156, 216),     // Crystal Blue
            dir: Color::Rgb(220, 165, 97),         // Carp Yellow
            sel_bg: Color::Rgb(42, 42, 55),        // bg_dim
            success: Color::Rgb(118, 148, 106),    // Spring Green
            match_file: Color::Rgb(106, 149, 137), // Wave Teal
            dim: Color::Rgb(114, 113, 105),        // Fuji Gray
            fg: Color::Rgb(220, 215, 186),         // Fuji White
        }
    }

    /// [Kanagawa Dragon](https://github.com/rebelot/kanagawa.nvim) — darker earth tones.
    pub fn kanagawa_dragon() -> Self {
        Self {
            brand: Color::Rgb(210, 126, 153),      // Sakura Pink
            accent: Color::Rgb(139, 164, 176),     // Dragon Blue
            dir: Color::Rgb(200, 170, 109),        // Dragon Yellow
            sel_bg: Color::Rgb(40, 39, 39),        // bg_dim
            success: Color::Rgb(135, 169, 135),    // Dragon Green
            match_file: Color::Rgb(142, 164, 162), // Dragon Aqua
            dim: Color::Rgb(166, 166, 156),        // Dragon Gray
            fg: Color::Rgb(197, 201, 197),         // Dragon White
        }
    }

    /// [Kanagawa Lotus](https://github.com/rebelot/kanagawa.nvim) — light parchment variant.
    pub fn kanagawa_lotus() -> Self {
        Self {
            brand: Color::Rgb(160, 154, 190),     // Lotus Violet
            accent: Color::Rgb(77, 105, 155),     // Lotus Blue
            dir: Color::Rgb(119, 113, 63),        // Lotus Yellow
            sel_bg: Color::Rgb(231, 219, 160),    // bg_dim
            success: Color::Rgb(111, 137, 78),    // Lotus Green
            match_file: Color::Rgb(78, 140, 162), // Lotus Teal
            dim: Color::Rgb(196, 178, 138),       // Lotus Gray
            fg: Color::Rgb(84, 84, 100),          // Lotus Ink
        }
    }

    /// [Moonfly](https://github.com/bluz71/vim-moonfly-colors) — deep dark with vibrant accents.
    pub fn moonfly() -> Self {
        Self {
            brand: Color::Rgb(174, 129, 255),      // Purple
            accent: Color::Rgb(128, 160, 255),     // Blue
            dir: Color::Rgb(227, 199, 138),        // Wheat/Yellow
            sel_bg: Color::Rgb(28, 28, 28),        // bgHighlight
            success: Color::Rgb(140, 200, 95),     // Green
            match_file: Color::Rgb(121, 219, 195), // Cyan/Emerald
            dim: Color::Rgb(78, 78, 78),           // Dark Gray
            fg: Color::Rgb(178, 178, 178),         // Foreground
        }
    }

    /// [Nightfly](https://github.com/bluz71/vim-nightfly-colors) — deep ocean blues.
    pub fn nightfly() -> Self {
        Self {
            brand: Color::Rgb(199, 146, 234),     // Violet
            accent: Color::Rgb(130, 170, 255),    // Blue
            dir: Color::Rgb(255, 202, 40),        // Yellow
            sel_bg: Color::Rgb(11, 41, 66),       // Slightly lighter than bg
            success: Color::Rgb(161, 205, 94),    // Green
            match_file: Color::Rgb(33, 199, 168), // Emerald/Cyan
            dim: Color::Rgb(75, 100, 121),        // Muted blue-grey
            fg: Color::Rgb(172, 187, 203),        // Foreground
        }
    }

    /// [Oxocarbon](https://github.com/nyoom-engineering/oxocarbon.nvim) — IBM Carbon-inspired.
    pub fn oxocarbon() -> Self {
        Self {
            brand: Color::Rgb(255, 126, 182),     // Magenta/Pink
            accent: Color::Rgb(120, 169, 255),    // Blue
            dir: Color::Rgb(255, 213, 0),         // Yellow
            sel_bg: Color::Rgb(38, 38, 38),       // bg highlight
            success: Color::Rgb(66, 190, 101),    // Green
            match_file: Color::Rgb(51, 177, 255), // Cyan
            dim: Color::Rgb(82, 82, 82),          // Muted
            fg: Color::Rgb(242, 244, 248),        // Foreground
        }
    }

    // ── Decorative / custom themes ────────────────────────────────────────────

    /// Grape — deep violet & soft blue, easy on the eyes in dark environments.
    pub fn grape() -> Self {
        Self::default()
            .brand(Color::Rgb(200, 120, 255))
            .accent(Color::Rgb(130, 180, 255))
            .dir(Color::Rgb(200, 160, 255))
            .sel_bg(Color::Rgb(50, 35, 80))
            .success(Color::Rgb(160, 110, 255))
            .match_file(Color::Rgb(180, 130, 255))
            .dim(Color::Rgb(110, 100, 130))
    }

    /// Ocean — teal & aquamarine, calm nautical feel.
    pub fn ocean() -> Self {
        Self::default()
            .brand(Color::Rgb(0, 200, 180))
            .accent(Color::Rgb(0, 175, 210))
            .dir(Color::Rgb(100, 220, 210))
            .sel_bg(Color::Rgb(0, 50, 70))
            .success(Color::Rgb(80, 230, 200))
            .match_file(Color::Rgb(80, 230, 200))
            .dim(Color::Rgb(80, 120, 130))
            .fg(Color::Rgb(200, 240, 245))
    }

    /// Sunset — warm amber & rose, vibrant high-energy palette.
    pub fn sunset() -> Self {
        Self::default()
            .brand(Color::Rgb(255, 80, 80))
            .accent(Color::Rgb(255, 150, 50))
            .dir(Color::Rgb(255, 200, 60))
            .sel_bg(Color::Rgb(80, 30, 20))
            .success(Color::Rgb(255, 180, 80))
            .match_file(Color::Rgb(255, 180, 80))
            .dim(Color::Rgb(140, 100, 80))
            .fg(Color::Rgb(255, 235, 210))
    }

    /// Forest — earthy greens & bark browns, natural low-contrast.
    pub fn forest() -> Self {
        Self::default()
            .brand(Color::Rgb(100, 200, 80))
            .accent(Color::Rgb(80, 160, 80))
            .dir(Color::Rgb(170, 220, 100))
            .sel_bg(Color::Rgb(20, 50, 20))
            .success(Color::Rgb(120, 210, 90))
            .match_file(Color::Rgb(120, 210, 90))
            .dim(Color::Rgb(90, 120, 80))
            .fg(Color::Rgb(210, 235, 200))
    }

    /// Rose — pinks & corals, playful pastel-inspired palette.
    pub fn rose() -> Self {
        Self::default()
            .brand(Color::Rgb(255, 100, 150))
            .accent(Color::Rgb(255, 140, 180))
            .dir(Color::Rgb(255, 180, 200))
            .sel_bg(Color::Rgb(80, 20, 40))
            .success(Color::Rgb(255, 160, 190))
            .match_file(Color::Rgb(255, 160, 190))
            .dim(Color::Rgb(140, 90, 110))
            .fg(Color::Rgb(255, 230, 235))
    }

    /// Mono — greyscale only, maximally distraction-free.
    pub fn mono() -> Self {
        Self::default()
            .brand(Color::Rgb(220, 220, 220))
            .accent(Color::Rgb(180, 180, 180))
            .dir(Color::Rgb(200, 200, 200))
            .sel_bg(Color::Rgb(50, 50, 55))
            .success(Color::Rgb(200, 200, 200))
            .match_file(Color::Rgb(230, 230, 230))
            .dim(Color::Rgb(110, 110, 115))
            .fg(Color::Rgb(210, 210, 210))
    }

    /// Neon — electric brights on near-black, synthwave / retro.
    pub fn neon() -> Self {
        Self::default()
            .brand(Color::Rgb(255, 0, 200))
            .accent(Color::Rgb(0, 255, 200))
            .dir(Color::Rgb(255, 220, 0))
            .sel_bg(Color::Rgb(30, 0, 50))
            .success(Color::Rgb(0, 255, 130))
            .match_file(Color::Rgb(0, 255, 130))
            .dim(Color::Rgb(100, 80, 120))
            .fg(Color::Rgb(230, 230, 255))
    }

    // ── Catalogue ─────────────────────────────────────────────────────────────

    /// Return every named preset as a `(display_name, description, theme)` tuple.
    ///
    /// The list includes both the decorative palettes defined by this crate and
    /// the well-known editor / terminal schemes that mirror the catalogue found
    /// in [Iced](https://docs.rs/iced/latest/iced/theme/enum.Theme.html).
    ///
    /// # Example
    ///
    /// ```
    /// use tui_file_explorer::Theme;
    ///
    /// for (name, desc, _theme) in Theme::all_presets() {
    ///     println!("{name} — {desc}");
    /// }
    /// assert!(Theme::all_presets().len() >= 27);
    /// ```
    pub fn all_presets() -> Vec<(&'static str, &'static str, Theme)> {
        vec![
            // ── Built-in ──────────────────────────────────────────────────────
            (
                "Default",
                "The built-in palette — orange title, cyan borders, yellow dirs",
                Theme::default(),
            ),
            // ── Decorative ────────────────────────────────────────────────────
            (
                "Grape",
                "Deep violet & soft blue — easy on the eyes in dark environments",
                Theme::grape(),
            ),
            (
                "Ocean",
                "Teal & aquamarine — calm, nautical feel",
                Theme::ocean(),
            ),
            (
                "Sunset",
                "Warm amber & rose — vibrant, high-energy palette",
                Theme::sunset(),
            ),
            (
                "Forest",
                "Earthy greens & bark browns — natural, low-contrast",
                Theme::forest(),
            ),
            (
                "Rose",
                "Pinks & corals — playful, pastel-inspired",
                Theme::rose(),
            ),
            (
                "Mono",
                "Greyscale only — maximally distraction-free",
                Theme::mono(),
            ),
            (
                "Neon",
                "Electric brights on near-black — synthwave / retro",
                Theme::neon(),
            ),
            // ── Editor / terminal presets ─────────────────────────────────────
            (
                "Dracula",
                "Pink, cyan & purple on dark grey",
                Theme::dracula(),
            ),
            ("Nord", "Arctic bluish tones", Theme::nord()),
            (
                "Solarized Dark",
                "Precision colours for machines and people — dark",
                Theme::solarized_dark(),
            ),
            (
                "Solarized Light",
                "Precision colours for machines and people — light",
                Theme::solarized_light(),
            ),
            (
                "Gruvbox Dark",
                "Retro groove — dark warm background",
                Theme::gruvbox_dark(),
            ),
            (
                "Gruvbox Light",
                "Retro groove — light warm background",
                Theme::gruvbox_light(),
            ),
            (
                "Catppuccin Latte",
                "Soothing pastel — light",
                Theme::catppuccin_latte(),
            ),
            (
                "Catppuccin Frappé",
                "Soothing pastel — medium-dark",
                Theme::catppuccin_frappe(),
            ),
            (
                "Catppuccin Macchiato",
                "Soothing pastel — dark",
                Theme::catppuccin_macchiato(),
            ),
            (
                "Catppuccin Mocha",
                "Soothing pastel — darkest",
                Theme::catppuccin_mocha(),
            ),
            (
                "Tokyo Night",
                "A clean dark blue / purple night",
                Theme::tokyo_night(),
            ),
            (
                "Tokyo Night Storm",
                "Tokyo Night on a slightly lighter background",
                Theme::tokyo_night_storm(),
            ),
            (
                "Tokyo Night Light",
                "Tokyo Night inverted to a light background",
                Theme::tokyo_night_light(),
            ),
            (
                "Kanagawa Wave",
                "Deep blue ink brushed on parchment",
                Theme::kanagawa_wave(),
            ),
            (
                "Kanagawa Dragon",
                "Darker earth tones — charcoal & moss",
                Theme::kanagawa_dragon(),
            ),
            (
                "Kanagawa Lotus",
                "Light parchment variant of Kanagawa",
                Theme::kanagawa_lotus(),
            ),
            (
                "Moonfly",
                "Deep dark background with vibrant accents",
                Theme::moonfly(),
            ),
            ("Nightfly", "Deep ocean blues", Theme::nightfly()),
            (
                "Oxocarbon",
                "IBM Carbon Design System inspired",
                Theme::oxocarbon(),
            ),
        ]
    }
}
