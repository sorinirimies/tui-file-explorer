//! The 40+ named colour presets, as an `impl` block on [`Theme`].

use super::*;

// ── Named presets ─────────────────────────────────────────────────────────────

impl Theme {
    /// Build a preset from the shared [`tui_nuances`] theme `key`.
    ///
    /// Dark themes keep the terminal's own background (`Color::Reset`); light
    /// themes paint their explicit light background, so they stay readable.
    fn from_shared(key: &str) -> Self {
        let shared = crate::nuances::NuancesTheme::by_name(key)
            .unwrap_or_else(|| panic!("tui-nuances is missing built-in theme '{key}'"));
        let mut t = Self::from_nuances(&shared);
        if !shared.background.is_some_and(tui_nuances::color::is_light) {
            t.bg = Color::Reset;
        }
        t
    }

    // ── Dark themes ───────────────────────────────────────────────────────────

    /// [Dracula](https://draculatheme.com/) — pink, cyan, purple on dark grey.
    #[must_use]
    pub fn dracula() -> Self {
        Self::from_shared("dracula")
    }

    /// [Nord](https://www.nordtheme.com/) — arctic, bluish tones.
    #[must_use]
    pub fn nord() -> Self {
        Self::from_shared("nord")
    }

    /// [Solarized Dark](https://ethanschoonover.com/solarized/).
    #[must_use]
    pub fn solarized_dark() -> Self {
        Self::from_shared("solarized-dark")
    }

    /// [Solarized Light](https://ethanschoonover.com/solarized/).
    #[must_use]
    pub fn solarized_light() -> Self {
        Self::from_shared("solarized-light")
    }

    /// [Gruvbox Dark](https://github.com/morhetz/gruvbox).
    #[must_use]
    pub fn gruvbox_dark() -> Self {
        Self::from_shared("gruvbox")
    }

    /// [Gruvbox Light](https://github.com/morhetz/gruvbox).
    #[must_use]
    pub fn gruvbox_light() -> Self {
        Self::from_shared("gruvbox-light")
    }

    /// [Catppuccin Latte](https://github.com/catppuccin/catppuccin) — light variant.
    #[must_use]
    pub fn catppuccin_latte() -> Self {
        Self::from_shared("catppuccin-latte")
    }

    /// [Catppuccin Frappé](https://github.com/catppuccin/catppuccin) — medium-dark variant.
    #[must_use]
    pub fn catppuccin_frappe() -> Self {
        Self::from_shared("catppuccin-frappe")
    }

    /// [Catppuccin Macchiato](https://github.com/catppuccin/catppuccin) — dark variant.
    #[must_use]
    pub fn catppuccin_macchiato() -> Self {
        Self::from_shared("catppuccin-macchiato")
    }

    /// [Catppuccin Mocha](https://github.com/catppuccin/catppuccin) — darkest variant.
    #[must_use]
    pub fn catppuccin_mocha() -> Self {
        Self::from_shared("catppuccin-mocha")
    }

    /// [Tokyo Night](https://github.com/folke/tokyonight.nvim) — dark blue/purple night.
    #[must_use]
    pub fn tokyo_night() -> Self {
        Self::from_shared("tokyo-night")
    }

    /// [Tokyo Night Storm](https://github.com/folke/tokyonight.nvim) — slightly lighter dark.
    #[must_use]
    pub fn tokyo_night_storm() -> Self {
        Self::from_shared("tokyo-night-storm")
    }

    /// [Tokyo Night Light](https://github.com/folke/tokyonight.nvim) — light variant.
    #[must_use]
    pub fn tokyo_night_light() -> Self {
        Self::from_shared("tokyo-night-light")
    }

    /// [Kanagawa Wave](https://github.com/rebelot/kanagawa.nvim) — deep blue ink.
    #[must_use]
    pub fn kanagawa_wave() -> Self {
        Self::from_shared("kanagawa-wave")
    }

    /// [Kanagawa Dragon](https://github.com/rebelot/kanagawa.nvim) — darker earth tones.
    #[must_use]
    pub fn kanagawa_dragon() -> Self {
        Self::from_shared("kanagawa-dragon")
    }

    /// [Kanagawa Lotus](https://github.com/rebelot/kanagawa.nvim) — light parchment variant.
    #[must_use]
    pub fn kanagawa_lotus() -> Self {
        Self::from_shared("kanagawa-lotus")
    }

    /// [Moonfly](https://github.com/bluz71/vim-moonfly-colors) — deep dark with vibrant accents.
    #[must_use]
    pub fn moonfly() -> Self {
        Self::from_shared("moonfly")
    }

    /// [Nightfly](https://github.com/bluz71/vim-nightfly-colors) — deep ocean blues.
    #[must_use]
    pub fn nightfly() -> Self {
        Self::from_shared("nightfly")
    }

    /// [Oxocarbon](https://github.com/nyoom-engineering/oxocarbon.nvim) — IBM Carbon-inspired.
    #[must_use]
    pub fn oxocarbon() -> Self {
        Self::from_shared("oxocarbon")
    }

    /// Default Light — clean light background with vibrant accents.
    #[must_use]
    pub fn default_light() -> Self {
        Self::from_shared("default-light")
    }

    /// [Cyberpunk](https://github.com/max-uran/cyberpunk) — neon yellow on near-black.
    #[must_use]
    pub fn cyberpunk() -> Self {
        Self::from_shared("cyberpunk")
    }

    /// [Rosé Pine](https://rosepinetheme.com/) — soho vibes for dark environments.
    #[must_use]
    pub fn rose_pine() -> Self {
        Self::from_shared("rose-pine")
    }

    /// [Rosé Pine Moon](https://rosepinetheme.com/) — slightly brighter Rosé Pine.
    #[must_use]
    pub fn rose_pine_moon() -> Self {
        Self::from_shared("rose-pine-moon")
    }

    /// [Rosé Pine Dawn](https://rosepinetheme.com/) — light variant.
    #[must_use]
    pub fn rose_pine_dawn() -> Self {
        Self::from_shared("rose-pine-dawn")
    }

    /// [Ayu Mirage](https://github.com/ayu-theme/ayu-colors) — muted dark with warm accents.
    #[must_use]
    pub fn ayu_mirage() -> Self {
        Self::from_shared("ayu-mirage")
    }

    /// [Everforest Dark](https://github.com/sainnhe/everforest) — soft green tones.
    #[must_use]
    pub fn everforest_dark() -> Self {
        Self::from_shared("everforest-dark")
    }

    /// [Atom One Dark](https://github.com/atom/one-dark-syntax) — classic dark editor theme.
    #[must_use]
    pub fn atom_one_dark() -> Self {
        Self::from_shared("one-dark")
    }

    /// [Atom One Light](https://github.com/atom/one-light-syntax) — classic light editor theme.
    #[must_use]
    pub fn atom_one_light() -> Self {
        Self::from_shared("atom-one-light")
    }

    /// [Night Owl](https://github.com/sdras/night-owl-vscode-theme) — deep blue with warm accents.
    #[must_use]
    pub fn night_owl() -> Self {
        Self::from_shared("night-owl")
    }

    /// [Poimandres](https://github.com/drcmda/poimandres-theme) — dark blue with teal highlights.
    #[must_use]
    pub fn poimandres() -> Self {
        Self::from_shared("poimandres")
    }

    /// [Flexoki Dark](https://stephango.com/flexoki) — inky warm dark.
    #[must_use]
    pub fn flexoki_dark() -> Self {
        Self::from_shared("flexoki-dark")
    }

    /// [Flexoki Light](https://stephango.com/flexoki) — warm light paper.
    #[must_use]
    pub fn flexoki_light() -> Self {
        Self::from_shared("flexoki-light")
    }

    /// [Carbonfox](https://github.com/EdenEast/nightfox.nvim) — dark carbon with bright accents.
    #[must_use]
    pub fn carbonfox() -> Self {
        Self::from_shared("carbonfox")
    }

    /// [Andromeda](https://github.com/EliverLara/Andromeda) — purple-tinted dark with teal accents.
    #[must_use]
    pub fn andromeda() -> Self {
        Self::from_shared("andromeda")
    }

    /// [Synthwave '84](https://github.com/robb0wen/synthwave-vscode) — retro neon synthwave.
    #[must_use]
    pub fn synthwave() -> Self {
        Self::from_shared("synthwave")
    }

    // ── Decorative / custom themes ────────────────────────────────────────────

    /// Grape — deep violet & soft blue, easy on the eyes in dark environments.
    #[must_use]
    pub fn grape() -> Self {
        Self::from_shared("grape")
    }

    /// Ocean — teal & aquamarine, calm nautical feel.
    #[must_use]
    pub fn ocean() -> Self {
        Self::from_shared("ocean")
    }

    /// Sunset — warm amber & rose, vibrant high-energy palette.
    #[must_use]
    pub fn sunset() -> Self {
        Self::from_shared("sunset")
    }

    /// Forest — earthy greens & bark browns, natural low-contrast.
    #[must_use]
    pub fn forest() -> Self {
        Self::from_shared("forest")
    }

    /// Rose — pinks & corals, playful pastel-inspired palette.
    #[must_use]
    pub fn rose() -> Self {
        Self::from_shared("rose")
    }

    /// Mono — greyscale only, maximally distraction-free.
    #[must_use]
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
    #[must_use]
    pub fn neon() -> Self {
        Self::from_shared("neon")
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
    #[must_use]
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
