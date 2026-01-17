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
}

pub const THEMES: &[(&str, &str)] = &[
    ("catppuccin-mocha", "Catppuccin Mocha"),
    ("tokyo-night", "Tokyo Night"),
    ("nord", "Nord"),
    ("solarized-dark", "Solarized Dark"),
    ("gruvbox-dark", "Gruvbox Dark"),
    ("default", "Default"),
];

pub fn theme_from_name(name: &str) -> Theme {
    match name.trim().to_lowercase().as_str() {
        "catppuccin-mocha" | "catppuccin" => Theme {
            bg: Color::Rgb(17, 17, 27),        // base
            panel_bg: Color::Rgb(24, 24, 37),  // mantle
            fg: Color::Rgb(205, 214, 244),     // text
            muted: Color::Rgb(127, 132, 156),  // overlay1
            accent: Color::Rgb(137, 180, 250), // blue
            accent_alt: Color::Rgb(203, 166, 247), // mauve
            success: Color::Rgb(166, 227, 161), // green
            warning: Color::Rgb(249, 226, 175), // yellow
            error: Color::Rgb(243, 139, 168),   // red
            highlight_bg: Color::Rgb(49, 50, 68), // surface0
            highlight_fg: Color::Rgb(205, 214, 244),
        },
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
        },
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
        },
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
        },
        "gruvbox-dark" | "gruvbox" => Theme {
            bg: Color::Rgb(40, 40, 40),
            panel_bg: Color::Rgb(60, 56, 54),
            fg: Color::Rgb(235, 219, 178),
            muted: Color::Rgb(146, 131, 116),
            accent: Color::Rgb(131, 165, 152),
            accent_alt: Color::Rgb(211, 134, 155),
            success: Color::Rgb(184, 187, 38),
            warning: Color::Rgb(250, 189, 47),
            error: Color::Rgb(204, 36, 29),
            highlight_bg: Color::Rgb(80, 73, 69),
            highlight_fg: Color::Rgb(235, 219, 178),
        },
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
        },
    }
}

