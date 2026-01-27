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
