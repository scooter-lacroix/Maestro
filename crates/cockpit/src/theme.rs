use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub panel_bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_alt: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub transparent: bool,
}

pub const THEMES: &[(&str, &str)] = &[
    // System theme (respects terminal transparency)
    ("system", "System Terminal (Transparent)"),
    // Popular dark themes
    ("dracula", "Dracula"),
    ("tokyo-night", "Tokyo Night"),
    ("catppuccin-mocha", "Catppuccin Mocha"),
    ("nord", "Nord"),
    ("rose-pine", "Rose Pine"),
    ("github-dark", "GitHub Dark"),
    ("solarized-dark", "Solarized Dark"),
    ("gruvbox-dark", "Gruvbox Dark"),
    ("kanagawa", "Kanagawa"),
    ("oxocarbon", "OxoCarbon"),
    ("monokai", "Monokai"),
    // New themes from rat-theme4
    ("embark", "Embark"),
    ("everforest", "Everforest"),
    ("falcon-dark", "Falcon Dark"),
    ("gatekeeper", "Gatekeeper"),
    ("imperial", "Imperial"),
    ("material", "Material"),
    ("monochrome", "Monochrome"),
    ("ocean", "Ocean"),
    ("radium", "Radium"),
    ("reds", "Reds"),
    ("rust", "Rust"),
    ("tailwind", "Tailwind"),
    ("tundra", "Tundra"),
    ("vscode", "VSCode"),
    ("base16", "Base16"),
    ("black-white", "Black & White"),
    // Legacy
    ("default", "Default"),
];

pub fn theme_from_name(name: &str) -> Theme {
    match name.trim().to_lowercase().as_str() {
        // ===== SYSTEM THEME (Terminal Transparency) =====
        "system" => Theme {
            bg: Color::Reset,
            panel_bg: Color::Reset,
            fg: Color::Reset,
            muted: Color::Reset,
            accent: Color::Cyan,
            accent_alt: Color::Magenta,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            highlight_bg: Color::Rgb(40, 40, 60),
            highlight_fg: Color::Reset,
            transparent: true,
        },
        // ===== POPULAR DARK THEMES =====

        // Dracula - https://draculatheme.com/
        "dracula" => Theme {
            bg: Color::Rgb(40, 42, 54),
            panel_bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(98, 114, 164),
            accent: Color::Rgb(139, 233, 253),
            accent_alt: Color::Rgb(80, 250, 123),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            error: Color::Rgb(255, 85, 85),
            highlight_bg: Color::Rgb(68, 71, 90),
            highlight_fg: Color::Rgb(248, 248, 242),
            transparent: false,
        },
        // Tokyo Night - https://github.com/folke/tokyonight.nvim
        "tokyo-night" | "tokyonight" => Theme {
            bg: Color::Rgb(26, 27, 38),
            panel_bg: Color::Rgb(36, 40, 59),
            fg: Color::Rgb(192, 202, 245),
            muted: Color::Rgb(86, 95, 137),
            accent: Color::Rgb(122, 162, 247),
            accent_alt: Color::Rgb(187, 154, 247),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            error: Color::Rgb(247, 118, 142),
            highlight_bg: Color::Rgb(65, 72, 104),
            highlight_fg: Color::Rgb(192, 202, 245),
            transparent: false,
        },

        // Catppuccin Mocha - https://catppuccin.com/
        "catppuccin-mocha" | "catppuccin" => Theme {
            bg: Color::Rgb(24, 24, 37),
            panel_bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(127, 132, 156),
            accent: Color::Rgb(137, 180, 250),
            accent_alt: Color::Rgb(203, 166, 247),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            highlight_bg: Color::Rgb(69, 71, 90),
            highlight_fg: Color::Rgb(205, 214, 244),
            transparent: false,
        },

        // Nord - https://www.nordtheme.com/
        "nord" => Theme {
            bg: Color::Rgb(46, 52, 64),
            panel_bg: Color::Rgb(59, 66, 82),
            fg: Color::Rgb(236, 239, 244),
            muted: Color::Rgb(129, 161, 193),
            accent: Color::Rgb(136, 192, 208),
            accent_alt: Color::Rgb(180, 142, 173),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            error: Color::Rgb(191, 97, 106),
            highlight_bg: Color::Rgb(67, 76, 94),
            highlight_fg: Color::Rgb(236, 239, 244),
            transparent: false,
        },

        // Rose Pine - https://github.com/rose-pine/rose-pine-theme
        "rose-pine" => Theme {
            bg: Color::Rgb(27, 27, 33),
            panel_bg: Color::Rgb(20, 20, 26),
            fg: Color::Rgb(224, 222, 236),
            muted: Color::Rgb(131, 133, 153),
            accent: Color::Rgb(235, 111, 146),
            accent_alt: Color::Rgb(48, 100, 140),
            success: Color::Rgb(49, 116, 143),
            warning: Color::Rgb(235, 188, 90),
            error: Color::Rgb(235, 111, 146),
            highlight_bg: Color::Rgb(69, 71, 90),
            highlight_fg: Color::Rgb(224, 222, 236),
            transparent: false,
        },

        // GitHub Dark - https://github.com/primer/github-vscode-theme
        "github-dark" => Theme {
            bg: Color::Rgb(22, 27, 34),
            panel_bg: Color::Rgb(27, 34, 43),
            fg: Color::Rgb(197, 208, 222),
            muted: Color::Rgb(110, 118, 129),
            accent: Color::Rgb(56, 139, 253),
            accent_alt: Color::Rgb(46, 160, 67),
            success: Color::Rgb(46, 160, 67),
            warning: Color::Rgb(187, 128, 9),
            error: Color::Rgb(248, 81, 73),
            highlight_bg: Color::Rgb(44, 54, 70),
            highlight_fg: Color::Rgb(197, 208, 222),
            transparent: false,
        },

        // Solarized Dark - https://ethanschoonover.com/solarized/
        "solarized-dark" | "solarized" => Theme {
            bg: Color::Rgb(0, 43, 54),
            panel_bg: Color::Rgb(7, 54, 66),
            fg: Color::Rgb(238, 232, 213),
            muted: Color::Rgb(147, 161, 161),
            accent: Color::Rgb(38, 139, 210),
            accent_alt: Color::Rgb(211, 54, 130),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            error: Color::Rgb(220, 50, 47),
            highlight_bg: Color::Rgb(88, 110, 117),
            highlight_fg: Color::Rgb(238, 232, 213),
            transparent: false,
        },

        // Gruvbox Dark - https://github.com/morhetz/gruvbox
        "gruvbox-dark" | "gruvbox" => Theme {
            bg: Color::Rgb(40, 40, 40),
            panel_bg: Color::Rgb(50, 48, 47),
            fg: Color::Rgb(235, 219, 178),
            muted: Color::Rgb(146, 131, 116),
            accent: Color::Rgb(131, 165, 152),
            accent_alt: Color::Rgb(211, 134, 155),
            success: Color::Rgb(184, 187, 38),
            warning: Color::Rgb(250, 189, 47),
            error: Color::Rgb(251, 73, 52),
            highlight_bg: Color::Rgb(69, 73, 77),
            highlight_fg: Color::Rgb(235, 219, 178),
            transparent: false,
        },

        // Kanagawa - https://github.com/rebelot/kanagawa.nvim
        "kanagawa" => Theme {
            bg: Color::Rgb(26, 27, 38),
            panel_bg: Color::Rgb(22, 23, 32),
            fg: Color::Rgb(192, 202, 245),
            muted: Color::Rgb(113, 124, 165),
            accent: Color::Rgb(117, 175, 238),
            accent_alt: Color::Rgb(187, 154, 247),
            success: Color::Rgb(140, 209, 137),
            warning: Color::Rgb(250, 200, 100),
            error: Color::Rgb(242, 132, 138),
            highlight_bg: Color::Rgb(54, 60, 90),
            highlight_fg: Color::Rgb(192, 202, 245),
            transparent: false,
        },

        // OxoCarbon - https://github.com/nyoom-engineering/oxocarbon.nvim
        "oxocarbon" => Theme {
            bg: Color::Rgb(29, 29, 38),
            panel_bg: Color::Rgb(36, 36, 50),
            fg: Color::Rgb(224, 224, 232),
            muted: Color::Rgb(127, 132, 156),
            accent: Color::Rgb(87, 207, 235),
            accent_alt: Color::Rgb(203, 166, 247),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            highlight_bg: Color::Rgb(69, 71, 90),
            highlight_fg: Color::Rgb(224, 224, 232),
            transparent: false,
        },

        // Monokai - https://monokai.pro/
        "monokai" => Theme {
            bg: Color::Rgb(39, 40, 34),
            panel_bg: Color::Rgb(43, 44, 38),
            fg: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(117, 113, 94),
            accent: Color::Rgb(102, 217, 239),
            accent_alt: Color::Rgb(166, 226, 46),
            success: Color::Rgb(166, 226, 46),
            warning: Color::Rgb(254, 209, 55),
            error: Color::Rgb(249, 38, 114),
            highlight_bg: Color::Rgb(73, 77, 67),
            highlight_fg: Color::Rgb(248, 248, 242),
            transparent: false,
        },

        // ===== NEW THEMES FROM RAT-THEME4 =====

        // Embark - Warm, earthy tones with good contrast
        "embark" => Theme {
            bg: Color::Rgb(30, 30, 35),
            panel_bg: Color::Rgb(40, 40, 45),
            fg: Color::Rgb(220, 220, 220),
            muted: Color::Rgb(140, 140, 150),
            accent: Color::Rgb(235, 175, 100),
            accent_alt: Color::Rgb(150, 200, 200),
            success: Color::Rgb(150, 200, 120),
            warning: Color::Rgb(235, 200, 100),
            error: Color::Rgb(230, 120, 120),
            highlight_bg: Color::Rgb(60, 60, 70),
            highlight_fg: Color::Rgb(220, 220, 220),
            transparent: false,
        },

        // Everforest - Green-based, easy on the eyes
        "everforest" => Theme {
            bg: Color::Rgb(39, 46, 44),
            panel_bg: Color::Rgb(46, 53, 51),
            fg: Color::Rgb(216, 222, 216),
            muted: Color::Rgb(131, 145, 139),
            accent: Color::Rgb(131, 192, 146),
            accent_alt: Color::Rgb(224, 175, 104),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            error: Color::Rgb(230, 126, 128),
            highlight_bg: Color::Rgb(73, 85, 80),
            highlight_fg: Color::Rgb(216, 222, 216),
            transparent: false,
        },

        // Falcon Dark - High contrast with soft blues
        "falcon-dark" => Theme {
            bg: Color::Rgb(25, 28, 35),
            panel_bg: Color::Rgb(32, 36, 45),
            fg: Color::Rgb(224, 224, 224),
            muted: Color::Rgb(130, 140, 160),
            accent: Color::Rgb(97, 175, 239),
            accent_alt: Color::Rgb(198, 120, 221),
            success: Color::Rgb(86, 182, 194),
            warning: Color::Rgb(209, 154, 102),
            error: Color::Rgb(224, 108, 117),
            highlight_bg: Color::Rgb(50, 60, 80),
            highlight_fg: Color::Rgb(224, 224, 224),
            transparent: false,
        },

        // Gatekeeper - Mysterious purple tones
        "gatekeeper" => Theme {
            bg: Color::Rgb(28, 27, 38),
            panel_bg: Color::Rgb(36, 35, 48),
            fg: Color::Rgb(216, 214, 228),
            muted: Color::Rgb(126, 124, 148),
            accent: Color::Rgb(187, 154, 247),
            accent_alt: Color::Rgb(139, 233, 253),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(245, 169, 127),
            error: Color::Rgb(247, 118, 142),
            highlight_bg: Color::Rgb(60, 58, 80),
            highlight_fg: Color::Rgb(216, 214, 228),
            transparent: false,
        },

        // Imperial - Purple and gold royal theme
        "imperial" => Theme {
            bg: Color::Rgb(32, 28, 44),
            panel_bg: Color::Rgb(42, 36, 58),
            fg: Color::Rgb(228, 222, 236),
            muted: Color::Rgb(138, 132, 156),
            accent: Color::Rgb(189, 147, 249),
            accent_alt: Color::Rgb(255, 215, 120),
            success: Color::Rgb(146, 200, 126),
            warning: Color::Rgb(245, 189, 108),
            error: Color::Rgb(242, 143, 173),
            highlight_bg: Color::Rgb(70, 60, 95),
            highlight_fg: Color::Rgb(228, 222, 236),
            transparent: false,
        },

        // Material - Material Design dark theme
        "material" => Theme {
            bg: Color::Rgb(30, 30, 30),
            panel_bg: Color::Rgb(45, 45, 45),
            fg: Color::Rgb(232, 232, 232),
            muted: Color::Rgb(153, 153, 153),
            accent: Color::Rgb(103, 58, 183),
            accent_alt: Color::Rgb(255, 193, 7),
            success: Color::Rgb(76, 175, 80),
            warning: Color::Rgb(255, 152, 0),
            error: Color::Rgb(244, 67, 54),
            highlight_bg: Color::Rgb(66, 66, 66),
            highlight_fg: Color::Rgb(232, 232, 232),
            transparent: false,
        },

        // Monochrome - Pure grayscale
        "monochrome" => Theme {
            bg: Color::Rgb(32, 32, 32),
            panel_bg: Color::Rgb(45, 45, 45),
            fg: Color::Rgb(220, 220, 220),
            muted: Color::Rgb(140, 140, 140),
            accent: Color::Rgb(200, 200, 200),
            accent_alt: Color::Rgb(180, 180, 180),
            success: Color::Rgb(170, 170, 170),
            warning: Color::Rgb(190, 190, 190),
            error: Color::Rgb(210, 210, 210),
            highlight_bg: Color::Rgb(60, 60, 60),
            highlight_fg: Color::Rgb(220, 220, 220),
            transparent: false,
        },

        // Ocean - Deep blue sea colors
        "ocean" => Theme {
            bg: Color::Rgb(24, 38, 54),
            panel_bg: Color::Rgb(34, 48, 64),
            fg: Color::Rgb(224, 232, 244),
            muted: Color::Rgb(124, 142, 174),
            accent: Color::Rgb(97, 175, 239),
            accent_alt: Color::Rgb(198, 120, 221),
            success: Color::Rgb(86, 182, 194),
            warning: Color::Rgb(209, 154, 102),
            error: Color::Rgb(224, 108, 117),
            highlight_bg: Color::Rgb(54, 78, 104),
            highlight_fg: Color::Rgb(224, 232, 244),
            transparent: false,
        },

        // Radium - Radioactive green theme
        "radium" => Theme {
            bg: Color::Rgb(20, 30, 20),
            panel_bg: Color::Rgb(30, 40, 30),
            fg: Color::Rgb(200, 240, 200),
            muted: Color::Rgb(100, 140, 100),
            accent: Color::Rgb(120, 220, 120),
            accent_alt: Color::Rgb(180, 240, 100),
            success: Color::Rgb(140, 230, 140),
            warning: Color::Rgb(240, 220, 100),
            error: Color::Rgb(240, 120, 120),
            highlight_bg: Color::Rgb(50, 80, 50),
            highlight_fg: Color::Rgb(200, 240, 200),
            transparent: false,
        },

        // Reds - Monochromatic red theme
        "reds" => Theme {
            bg: Color::Rgb(40, 20, 20),
            panel_bg: Color::Rgb(50, 30, 30),
            fg: Color::Rgb(240, 220, 220),
            muted: Color::Rgb(180, 120, 120),
            accent: Color::Rgb(255, 120, 120),
            accent_alt: Color::Rgb(255, 180, 150),
            success: Color::Rgb(200, 200, 200),
            warning: Color::Rgb(255, 200, 100),
            error: Color::Rgb(255, 150, 150),
            highlight_bg: Color::Rgb(80, 40, 40),
            highlight_fg: Color::Rgb(240, 220, 220),
            transparent: false,
        },

        // Rust - Rust programming language colors
        "rust" => Theme {
            bg: Color::Rgb(35, 32, 38),
            panel_bg: Color::Rgb(45, 42, 48),
            fg: Color::Rgb(224, 222, 236),
            muted: Color::Rgb(134, 132, 156),
            accent: Color::Rgb(220, 90, 90),
            accent_alt: Color::Rgb(171, 178, 191),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            error: Color::Rgb(224, 108, 117),
            highlight_bg: Color::Rgb(70, 66, 80),
            highlight_fg: Color::Rgb(224, 222, 236),
            transparent: false,
        },

        // Tailwind - Tailwind CSS palette
        "tailwind" => Theme {
            bg: Color::Rgb(22, 27, 34),
            panel_bg: Color::Rgb(30, 41, 59),
            fg: Color::Rgb(241, 245, 249),
            muted: Color::Rgb(148, 163, 184),
            accent: Color::Rgb(56, 189, 248),
            accent_alt: Color::Rgb(168, 85, 247),
            success: Color::Rgb(74, 222, 128),
            warning: Color::Rgb(250, 204, 21),
            error: Color::Rgb(248, 113, 113),
            highlight_bg: Color::Rgb(51, 65, 85),
            highlight_fg: Color::Rgb(241, 245, 249),
            transparent: false,
        },

        // Tundra - Frozen arctic colors
        "tundra" => Theme {
            bg: Color::Rgb(36, 42, 50),
            panel_bg: Color::Rgb(46, 54, 64),
            fg: Color::Rgb(216, 222, 233),
            muted: Color::Rgb(136, 152, 173),
            accent: Color::Rgb(129, 161, 193),
            accent_alt: Color::Rgb(143, 188, 187),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(227, 180, 99),
            error: Color::Rgb(191, 97, 106),
            highlight_bg: Color::Rgb(67, 87, 107),
            highlight_fg: Color::Rgb(216, 222, 233),
            transparent: false,
        },

        // VSCode - VSCode Dark default
        "vscode" => Theme {
            bg: Color::Rgb(30, 30, 30),
            panel_bg: Color::Rgb(37, 37, 38),
            fg: Color::Rgb(220, 220, 220),
            muted: Color::Rgb(153, 153, 153),
            accent: Color::Rgb(56, 139, 253),
            accent_alt: Color::Rgb(206, 145, 120),
            success: Color::Rgb(106, 153, 85),
            warning: Color::Rgb(197, 134, 69),
            error: Color::Rgb(248, 81, 73),
            highlight_bg: Color::Rgb(50, 50, 60),
            highlight_fg: Color::Rgb(220, 220, 220),
            transparent: false,
        },

        // Base16 - Base16 default dark
        "base16" => Theme {
            bg: Color::Rgb(28, 29, 35),
            panel_bg: Color::Rgb(38, 40, 48),
            fg: Color::Rgb(216, 222, 233),
            muted: Color::Rgb(136, 152, 173),
            accent: Color::Rgb(129, 161, 193),
            accent_alt: Color::Rgb(143, 188, 187),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(227, 180, 99),
            error: Color::Rgb(191, 97, 106),
            highlight_bg: Color::Rgb(67, 76, 94),
            highlight_fg: Color::Rgb(216, 222, 233),
            transparent: false,
        },

        // Black & White - High contrast minimal
        "black-white" => Theme {
            bg: Color::Rgb(0, 0, 0),
            panel_bg: Color::Rgb(20, 20, 20),
            fg: Color::Rgb(255, 255, 255),
            muted: Color::Rgb(170, 170, 170),
            accent: Color::Rgb(255, 255, 255),
            accent_alt: Color::Rgb(200, 200, 200),
            success: Color::Rgb(200, 200, 200),
            warning: Color::Rgb(255, 255, 200),
            error: Color::Rgb(255, 200, 200),
            highlight_bg: Color::Rgb(60, 60, 60),
            highlight_fg: Color::Rgb(255, 255, 255),
            transparent: false,
        },

        // ===== DEFAULT =====
        _ => Theme {
            bg: Color::Black,
            panel_bg: Color::Rgb(15, 15, 25),
            fg: Color::White,
            muted: Color::Gray,
            accent: Color::Cyan,
            accent_alt: Color::Magenta,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            highlight_bg: Color::Rgb(40, 40, 60),
            highlight_fg: Color::White,
            transparent: false,
        },
    }
}
