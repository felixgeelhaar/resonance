//! Built-in themes — four color schemes shipped with Resonance.

use ratatui::style::Color;

use super::Theme;

/// Default theme — matches the original hardcoded colors.
pub fn default() -> Theme {
    Theme {
        name: "Default".to_string(),

        editor_fg: Color::White,
        editor_bg: Color::Reset,
        editor_cursor: Color::Yellow,
        editor_line_number: Color::DarkGray,

        status_fg: Color::White,
        status_bg: Color::DarkGray,
        status_accent: Color::Cyan,

        track_header_fg: Color::White,
        track_muted: Color::DarkGray,

        grid_palette: [
            Color::Cyan,
            Color::Magenta,
            Color::Yellow,
            Color::Green,
            Color::Blue,
            Color::Red,
            Color::LightCyan,
            Color::LightMagenta,
        ],
        grid_hit_bright: Color::White,
        grid_hit_dim: Color::DarkGray,
        grid_empty: Color::DarkGray,
        grid_playhead: Color::Green,

        macro_name: Color::Cyan,
        macro_bar: Color::Green,
        macro_value: Color::Yellow,

        diff_add: Color::Green,
        diff_remove: Color::Red,

        help_key: Color::Yellow,
        help_desc: Color::White,

        border: Color::White,
        border_focused: Color::Cyan,
        title: Color::Cyan,

        editor_keyword: Color::Yellow,
        editor_pattern: Color::Cyan,
        editor_number: Color::Green,
        editor_active_line: Color::DarkGray,
        beat_pulse: Color::Yellow,
        vu_low: Color::Green,
        vu_mid: Color::Yellow,
        vu_high: Color::Red,
    }
}

/// Catppuccin Mocha — pastel colors on a dark background.
pub fn catppuccin_mocha() -> Theme {
    Theme {
        name: "Catppuccin Mocha".to_string(),

        editor_fg: Color::Rgb(205, 214, 244),        // text
        editor_bg: Color::Rgb(30, 30, 46),           // base
        editor_cursor: Color::Rgb(249, 226, 175),    // yellow
        editor_line_number: Color::Rgb(88, 91, 112), // surface2

        status_fg: Color::Rgb(205, 214, 244),     // text
        status_bg: Color::Rgb(49, 50, 68),        // surface0
        status_accent: Color::Rgb(137, 180, 250), // blue

        track_header_fg: Color::Rgb(205, 214, 244),
        track_muted: Color::Rgb(88, 91, 112),

        grid_palette: [
            Color::Rgb(137, 180, 250), // blue
            Color::Rgb(203, 166, 247), // mauve
            Color::Rgb(249, 226, 175), // yellow
            Color::Rgb(166, 227, 161), // green
            Color::Rgb(116, 199, 236), // sapphire
            Color::Rgb(243, 139, 168), // red
            Color::Rgb(148, 226, 213), // teal
            Color::Rgb(245, 194, 231), // pink
        ],
        grid_hit_bright: Color::Rgb(205, 214, 244),
        grid_hit_dim: Color::Rgb(88, 91, 112),
        grid_empty: Color::Rgb(69, 71, 90), // surface1
        grid_playhead: Color::Rgb(166, 227, 161),

        macro_name: Color::Rgb(137, 180, 250),
        macro_bar: Color::Rgb(166, 227, 161),
        macro_value: Color::Rgb(249, 226, 175),

        diff_add: Color::Rgb(166, 227, 161),
        diff_remove: Color::Rgb(243, 139, 168),

        help_key: Color::Rgb(249, 226, 175),
        help_desc: Color::Rgb(205, 214, 244),

        border: Color::Rgb(108, 112, 134),         // overlay0
        border_focused: Color::Rgb(137, 180, 250), // blue
        title: Color::Rgb(137, 180, 250),

        editor_keyword: Color::Rgb(203, 166, 247),  // mauve
        editor_pattern: Color::Rgb(148, 226, 213),  // teal
        editor_number: Color::Rgb(166, 227, 161),   // green
        editor_active_line: Color::Rgb(49, 50, 68), // surface0
        beat_pulse: Color::Rgb(249, 226, 175),      // yellow
        vu_low: Color::Rgb(166, 227, 161),          // green
        vu_mid: Color::Rgb(249, 226, 175),          // yellow
        vu_high: Color::Rgb(243, 139, 168),         // red
    }
}

/// Gruvbox Dark — warm retro palette.
pub fn gruvbox_dark() -> Theme {
    Theme {
        name: "Gruvbox Dark".to_string(),

        editor_fg: Color::Rgb(235, 219, 178),          // fg
        editor_bg: Color::Rgb(40, 40, 40),             // bg
        editor_cursor: Color::Rgb(250, 189, 47),       // yellow
        editor_line_number: Color::Rgb(124, 111, 100), // gray

        status_fg: Color::Rgb(235, 219, 178),
        status_bg: Color::Rgb(60, 56, 54),        // bg1
        status_accent: Color::Rgb(131, 165, 152), // aqua

        track_header_fg: Color::Rgb(235, 219, 178),
        track_muted: Color::Rgb(124, 111, 100),

        grid_palette: [
            Color::Rgb(131, 165, 152), // aqua
            Color::Rgb(211, 134, 155), // purple
            Color::Rgb(250, 189, 47),  // yellow
            Color::Rgb(184, 187, 38),  // green
            Color::Rgb(69, 133, 136),  // blue
            Color::Rgb(251, 73, 52),   // red
            Color::Rgb(142, 192, 124), // bright green
            Color::Rgb(254, 128, 25),  // orange
        ],
        grid_hit_bright: Color::Rgb(235, 219, 178),
        grid_hit_dim: Color::Rgb(124, 111, 100),
        grid_empty: Color::Rgb(80, 73, 69), // bg2
        grid_playhead: Color::Rgb(184, 187, 38),

        macro_name: Color::Rgb(131, 165, 152),
        macro_bar: Color::Rgb(184, 187, 38),
        macro_value: Color::Rgb(250, 189, 47),

        diff_add: Color::Rgb(184, 187, 38),
        diff_remove: Color::Rgb(251, 73, 52),

        help_key: Color::Rgb(250, 189, 47),
        help_desc: Color::Rgb(235, 219, 178),

        border: Color::Rgb(168, 153, 132),         // fg4
        border_focused: Color::Rgb(131, 165, 152), // aqua
        title: Color::Rgb(131, 165, 152),

        editor_keyword: Color::Rgb(254, 128, 25),   // orange
        editor_pattern: Color::Rgb(131, 165, 152),  // aqua
        editor_number: Color::Rgb(184, 187, 38),    // green
        editor_active_line: Color::Rgb(60, 56, 54), // bg1
        beat_pulse: Color::Rgb(250, 189, 47),       // yellow
        vu_low: Color::Rgb(184, 187, 38),           // green
        vu_mid: Color::Rgb(250, 189, 47),           // yellow
        vu_high: Color::Rgb(251, 73, 52),           // red
    }
}

/// Minimal — monochrome white/gray for maximum readability.
pub fn minimal() -> Theme {
    Theme {
        name: "Minimal".to_string(),

        editor_fg: Color::White,
        editor_bg: Color::Reset,
        editor_cursor: Color::White,
        editor_line_number: Color::DarkGray,

        status_fg: Color::White,
        status_bg: Color::DarkGray,
        status_accent: Color::White,

        track_header_fg: Color::White,
        track_muted: Color::DarkGray,

        grid_palette: [
            Color::White,
            Color::LightCyan,
            Color::LightYellow,
            Color::LightGreen,
            Color::LightBlue,
            Color::LightRed,
            Color::Gray,
            Color::LightMagenta,
        ],
        grid_hit_bright: Color::White,
        grid_hit_dim: Color::DarkGray,
        grid_empty: Color::DarkGray,
        grid_playhead: Color::White,

        macro_name: Color::White,
        macro_bar: Color::Gray,
        macro_value: Color::White,

        diff_add: Color::LightGreen,
        diff_remove: Color::LightRed,

        help_key: Color::White,
        help_desc: Color::Gray,

        border: Color::DarkGray,
        border_focused: Color::White,
        title: Color::White,

        editor_keyword: Color::White,
        editor_pattern: Color::Gray,
        editor_number: Color::LightGreen,
        editor_active_line: Color::DarkGray,
        beat_pulse: Color::White,
        vu_low: Color::LightGreen,
        vu_mid: Color::LightYellow,
        vu_high: Color::LightRed,
    }
}

/// Strudel — dark background with golden/teal accents inspired by strudel.cc.
pub fn strudel() -> Theme {
    Theme {
        name: "Strudel".to_string(),

        editor_fg: Color::Rgb(220, 220, 220),       // light gray
        editor_bg: Color::Rgb(34, 34, 34),          // #222222
        editor_cursor: Color::Rgb(255, 204, 0),     // golden
        editor_line_number: Color::Rgb(85, 85, 85), // dim

        status_fg: Color::Rgb(220, 220, 220),
        status_bg: Color::Rgb(42, 42, 42),
        status_accent: Color::Rgb(0, 200, 200), // teal

        track_header_fg: Color::Rgb(220, 220, 220),
        track_muted: Color::Rgb(85, 85, 85),

        grid_palette: [
            Color::Rgb(255, 204, 0),   // golden
            Color::Rgb(0, 200, 200),   // teal
            Color::Rgb(255, 153, 0),   // orange
            Color::Rgb(102, 204, 102), // green
            Color::Rgb(180, 120, 255), // purple
            Color::Rgb(255, 80, 80),   // red
            Color::Rgb(100, 160, 255), // blue
            Color::Rgb(255, 150, 200), // pink
        ],
        grid_hit_bright: Color::Rgb(255, 255, 255),
        grid_hit_dim: Color::Rgb(100, 100, 100),
        grid_empty: Color::Rgb(50, 50, 50),
        grid_playhead: Color::Rgb(255, 204, 0),

        macro_name: Color::Rgb(0, 200, 200),
        macro_bar: Color::Rgb(255, 204, 0),
        macro_value: Color::Rgb(220, 220, 220),

        diff_add: Color::Rgb(102, 204, 102),
        diff_remove: Color::Rgb(255, 80, 80),

        help_key: Color::Rgb(255, 204, 0),
        help_desc: Color::Rgb(200, 200, 200),

        border: Color::Rgb(60, 60, 60),
        border_focused: Color::Rgb(255, 204, 0),
        title: Color::Rgb(0, 200, 200),

        editor_keyword: Color::Rgb(255, 204, 0), // golden keywords
        editor_pattern: Color::Rgb(0, 200, 200), // teal patterns
        editor_number: Color::Rgb(102, 204, 102), // green numbers
        editor_active_line: Color::Rgb(50, 50, 50), // subtle highlight
        beat_pulse: Color::Rgb(255, 204, 0),     // golden pulse
        vu_low: Color::Rgb(102, 204, 102),
        vu_mid: Color::Rgb(255, 204, 0),
        vu_high: Color::Rgb(255, 80, 80),
    }
}

/// Returns all built-in themes in display order.
pub fn all_builtins() -> Vec<Theme> {
    vec![
        default(),
        strudel(),
        catppuccin_mocha(),
        gruvbox_dark(),
        minimal(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_count() {
        assert_eq!(all_builtins().len(), 5);
    }

    #[test]
    fn all_builtins_distinct_names() {
        let themes = all_builtins();
        let mut names: Vec<&str> = themes.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), themes.len());
    }

    #[test]
    fn default_backward_compat_palette() {
        let theme = default();
        assert_eq!(theme.grid_palette[0], Color::Cyan);
        assert_eq!(theme.grid_palette[1], Color::Magenta);
        assert_eq!(theme.grid_palette[2], Color::Yellow);
        assert_eq!(theme.grid_palette[3], Color::Green);
    }

    #[test]
    fn each_builtin_valid() {
        for theme in all_builtins() {
            assert!(!theme.name.is_empty());
            assert_eq!(theme.grid_palette.len(), 8);
        }
    }
}
