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

    /// Override the background colour and return `self` (builder-style).
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = color;
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
            bg: Color::Reset,
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
            bg: Color::Reset,
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
            bg: Color::Reset,
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
            bg: Color::Rgb(253, 246, 227),       // Base3
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
            bg: Color::Reset,
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
            bg: Color::Rgb(251, 241, 199),         // bg0
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
            bg: Color::Rgb(239, 241, 245),        // Base
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
            bg: Color::Reset,
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
            bg: Color::Reset,
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
            bg: Color::Reset,
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
            bg: Color::Reset,
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
            bg: Color::Reset,
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
            bg: Color::Rgb(213, 214, 219),       // TNL bg
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
            bg: Color::Reset,
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
            bg: Color::Reset,
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
            bg: Color::Rgb(245, 240, 215),        // Lotus bg
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
            bg: Color::Reset,
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
            bg: Color::Reset,
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
            bg: Color::Reset,
        }
    }

    /// Default Light — clean light background with vibrant accents.
    pub fn default_light() -> Self {
        Self {
            brand: Color::Rgb(0, 140, 220),
            accent: Color::Rgb(255, 100, 30),
            dir: Color::Rgb(200, 140, 0),
            sel_bg: Color::Rgb(200, 220, 255),
            success: Color::Rgb(30, 160, 80),
            match_file: Color::Rgb(30, 160, 80),
            dim: Color::Rgb(130, 140, 160),
            fg: Color::Rgb(30, 35, 50),
            bg: Color::Rgb(250, 250, 255),
        }
    }

    /// [Cyberpunk](https://github.com/max-uran/cyberpunk) — neon yellow on near-black.
    pub fn cyberpunk() -> Self {
        Self {
            brand: Color::Rgb(252, 238, 10),
            accent: Color::Rgb(0, 210, 235),
            dir: Color::Rgb(255, 150, 0),
            sel_bg: Color::Rgb(50, 48, 20),
            success: Color::Rgb(0, 220, 180),
            match_file: Color::Rgb(0, 220, 180),
            dim: Color::Rgb(90, 90, 100),
            fg: Color::Rgb(230, 230, 220),
            bg: Color::Reset,
        }
    }

    /// [Rosé Pine](https://rosepinetheme.com/) — soho vibes for dark environments.
    pub fn rose_pine() -> Self {
        Self {
            brand: Color::Rgb(196, 167, 231),
            accent: Color::Rgb(156, 207, 216),
            dir: Color::Rgb(246, 193, 119),
            sel_bg: Color::Rgb(64, 61, 82),
            success: Color::Rgb(49, 116, 143),
            match_file: Color::Rgb(49, 116, 143),
            dim: Color::Rgb(110, 106, 134),
            fg: Color::Rgb(224, 222, 244),
            bg: Color::Reset,
        }
    }

    /// [Rosé Pine Moon](https://rosepinetheme.com/) — slightly brighter Rosé Pine.
    pub fn rose_pine_moon() -> Self {
        Self {
            brand: Color::Rgb(196, 167, 231),
            accent: Color::Rgb(156, 207, 216),
            dir: Color::Rgb(246, 193, 119),
            sel_bg: Color::Rgb(68, 65, 90),
            success: Color::Rgb(62, 143, 176),
            match_file: Color::Rgb(62, 143, 176),
            dim: Color::Rgb(110, 106, 134),
            fg: Color::Rgb(224, 222, 244),
            bg: Color::Reset,
        }
    }

    /// [Rosé Pine Dawn](https://rosepinetheme.com/) — light variant.
    pub fn rose_pine_dawn() -> Self {
        Self {
            brand: Color::Rgb(144, 122, 169),
            accent: Color::Rgb(86, 148, 159),
            dir: Color::Rgb(234, 157, 52),
            sel_bg: Color::Rgb(223, 218, 217),
            success: Color::Rgb(40, 105, 131),
            match_file: Color::Rgb(40, 105, 131),
            dim: Color::Rgb(152, 147, 165),
            fg: Color::Rgb(87, 82, 121),
            bg: Color::Rgb(250, 244, 237),
        }
    }

    /// [Ayu Mirage](https://github.com/ayu-theme/ayu-colors) — muted dark with warm accents.
    pub fn ayu_mirage() -> Self {
        Self {
            brand: Color::Rgb(115, 208, 255),
            accent: Color::Rgb(115, 208, 255),
            dir: Color::Rgb(250, 204, 110),
            sel_bg: Color::Rgb(64, 159, 255),
            success: Color::Rgb(135, 217, 108),
            match_file: Color::Rgb(135, 217, 108),
            dim: Color::Rgb(104, 104, 104),
            fg: Color::Rgb(204, 202, 194),
            bg: Color::Reset,
        }
    }

    /// [Everforest Dark](https://github.com/sainnhe/everforest) — soft green tones.
    pub fn everforest_dark() -> Self {
        Self {
            brand: Color::Rgb(167, 192, 128),
            accent: Color::Rgb(127, 187, 179),
            dir: Color::Rgb(219, 188, 127),
            sel_bg: Color::Rgb(76, 55, 67),
            success: Color::Rgb(167, 192, 128),
            match_file: Color::Rgb(167, 192, 128),
            dim: Color::Rgb(122, 132, 120),
            fg: Color::Rgb(211, 198, 170),
            bg: Color::Reset,
        }
    }

    /// [Atom One Dark](https://github.com/atom/one-dark-syntax) — classic dark editor theme.
    pub fn atom_one_dark() -> Self {
        Self {
            brand: Color::Rgb(97, 175, 239),
            accent: Color::Rgb(97, 175, 239),
            dir: Color::Rgb(229, 192, 123),
            sel_bg: Color::Rgb(50, 56, 68),
            success: Color::Rgb(152, 195, 121),
            match_file: Color::Rgb(152, 195, 121),
            dim: Color::Rgb(118, 118, 118),
            fg: Color::Rgb(171, 178, 191),
            bg: Color::Reset,
        }
    }

    /// [Atom One Light](https://github.com/atom/one-light-syntax) — classic light editor theme.
    pub fn atom_one_light() -> Self {
        Self {
            brand: Color::Rgb(47, 90, 243),
            accent: Color::Rgb(47, 90, 243),
            dir: Color::Rgb(210, 182, 124),
            sel_bg: Color::Rgb(237, 237, 237),
            success: Color::Rgb(63, 149, 58),
            match_file: Color::Rgb(63, 149, 58),
            dim: Color::Rgb(118, 118, 118),
            fg: Color::Rgb(42, 44, 51),
            bg: Color::Rgb(249, 249, 249),
        }
    }

    /// [Night Owl](https://github.com/sdras/night-owl-vscode-theme) — deep blue with warm accents.
    pub fn night_owl() -> Self {
        Self {
            brand: Color::Rgb(130, 170, 255),
            accent: Color::Rgb(130, 170, 255),
            dir: Color::Rgb(173, 219, 103),
            sel_bg: Color::Rgb(95, 126, 151),
            success: Color::Rgb(34, 218, 110),
            match_file: Color::Rgb(34, 218, 110),
            dim: Color::Rgb(87, 86, 86),
            fg: Color::Rgb(214, 222, 235),
            bg: Color::Reset,
        }
    }

    /// [Poimandres](https://github.com/drcmda/poimandres-theme) — dark blue with teal highlights.
    pub fn poimandres() -> Self {
        Self {
            brand: Color::Rgb(93, 228, 199),
            accent: Color::Rgb(137, 221, 255),
            dir: Color::Rgb(255, 250, 194),
            sel_bg: Color::Rgb(50, 55, 75),
            success: Color::Rgb(93, 228, 199),
            match_file: Color::Rgb(93, 228, 199),
            dim: Color::Rgb(100, 106, 130),
            fg: Color::Rgb(166, 172, 205),
            bg: Color::Reset,
        }
    }

    /// [Flexoki Dark](https://stephango.com/flexoki) — inky warm dark.
    pub fn flexoki_dark() -> Self {
        Self {
            brand: Color::Rgb(67, 133, 190),
            accent: Color::Rgb(67, 133, 190),
            dir: Color::Rgb(208, 162, 21),
            sel_bg: Color::Rgb(64, 62, 60),
            success: Color::Rgb(135, 154, 57),
            match_file: Color::Rgb(135, 154, 57),
            dim: Color::Rgb(87, 86, 83),
            fg: Color::Rgb(206, 205, 195),
            bg: Color::Reset,
        }
    }

    /// [Flexoki Light](https://stephango.com/flexoki) — warm light paper.
    pub fn flexoki_light() -> Self {
        Self {
            brand: Color::Rgb(32, 94, 166),
            accent: Color::Rgb(32, 94, 166),
            dir: Color::Rgb(173, 131, 1),
            sel_bg: Color::Rgb(206, 205, 195),
            success: Color::Rgb(102, 128, 11),
            match_file: Color::Rgb(102, 128, 11),
            dim: Color::Rgb(111, 110, 105),
            fg: Color::Rgb(16, 15, 15),
            bg: Color::Rgb(255, 252, 240),
        }
    }

    /// [Carbonfox](https://github.com/EdenEast/nightfox.nvim) — dark carbon with bright accents.
    pub fn carbonfox() -> Self {
        Self {
            brand: Color::Rgb(120, 169, 255),
            accent: Color::Rgb(120, 169, 255),
            dir: Color::Rgb(8, 189, 186),
            sel_bg: Color::Rgb(42, 42, 42),
            success: Color::Rgb(37, 190, 106),
            match_file: Color::Rgb(37, 190, 106),
            dim: Color::Rgb(100, 100, 110),
            fg: Color::Rgb(242, 244, 248),
            bg: Color::Reset,
        }
    }

    /// [Andromeda](https://github.com/EliverLara/Andromeda) — purple-tinted dark with teal accents.
    pub fn andromeda() -> Self {
        Self {
            brand: Color::Rgb(5, 188, 121),
            accent: Color::Rgb(15, 168, 205),
            dir: Color::Rgb(229, 229, 18),
            sel_bg: Color::Rgb(90, 92, 98),
            success: Color::Rgb(5, 188, 121),
            match_file: Color::Rgb(5, 188, 121),
            dim: Color::Rgb(102, 102, 102),
            fg: Color::Rgb(229, 229, 229),
            bg: Color::Reset,
        }
    }

    /// [Synthwave '84](https://github.com/robb0wen/synthwave-vscode) — retro neon synthwave.
    pub fn synthwave() -> Self {
        Self {
            brand: Color::Rgb(246, 24, 143),
            accent: Color::Rgb(18, 195, 226),
            dir: Color::Rgb(253, 248, 52),
            sel_bg: Color::Rgb(25, 50, 60),
            success: Color::Rgb(30, 187, 43),
            match_file: Color::Rgb(30, 187, 43),
            dim: Color::Rgb(127, 112, 148),
            fg: Color::Rgb(218, 217, 199),
            bg: Color::Reset,
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
            (
                "Default Light",
                "Clean light background with vibrant accents",
                Theme::default_light(),
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
            // ── New community themes ──────────────────────────────────────────
            (
                "Cyberpunk",
                "Neon yellow & cyan on near-black — high contrast",
                Theme::cyberpunk(),
            ),
            (
                "Rosé Pine",
                "Soho vibes for dark environments",
                Theme::rose_pine(),
            ),
            (
                "Rosé Pine Moon",
                "Slightly brighter Rosé Pine variant",
                Theme::rose_pine_moon(),
            ),
            (
                "Rosé Pine Dawn",
                "Rosé Pine on a light background",
                Theme::rose_pine_dawn(),
            ),
            (
                "Ayu Mirage",
                "Muted dark with warm blue & yellow accents",
                Theme::ayu_mirage(),
            ),
            (
                "Everforest Dark",
                "Soft green tones inspired by nature",
                Theme::everforest_dark(),
            ),
            (
                "Atom One Dark",
                "Classic dark editor theme by GitHub",
                Theme::atom_one_dark(),
            ),
            (
                "Atom One Light",
                "Classic light editor theme by GitHub",
                Theme::atom_one_light(),
            ),
            (
                "Night Owl",
                "Deep blue with warm highlights for night coding",
                Theme::night_owl(),
            ),
            (
                "Poimandres",
                "Dark blue with teal & pastel highlights",
                Theme::poimandres(),
            ),
            (
                "Flexoki Dark",
                "Inky warm dark — minimal and intentional",
                Theme::flexoki_dark(),
            ),
            (
                "Flexoki Light",
                "Warm light paper — minimal and intentional",
                Theme::flexoki_light(),
            ),
            (
                "Carbonfox",
                "Dark carbon with bright blue & teal accents",
                Theme::carbonfox(),
            ),
            (
                "Andromeda",
                "Purple-tinted dark with teal & green accents",
                Theme::andromeda(),
            ),
            (
                "Synthwave '84",
                "Retro neon synthwave — pink, cyan & yellow",
                Theme::synthwave(),
            ),
        ]
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
}
