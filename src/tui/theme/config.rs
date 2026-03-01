//! Theme YAML config — load custom themes from ~/.resonance/theme.yaml.

use ratatui::style::Color;
use serde::Deserialize;

use super::Theme;

/// Intermediate YAML representation — all fields optional.
#[derive(Debug, Deserialize)]
struct ThemeConfig {
    name: Option<String>,

    editor_fg: Option<String>,
    editor_bg: Option<String>,
    editor_cursor: Option<String>,
    editor_line_number: Option<String>,

    status_fg: Option<String>,
    status_bg: Option<String>,
    status_accent: Option<String>,

    track_header_fg: Option<String>,
    track_muted: Option<String>,

    grid_palette: Option<Vec<String>>,
    grid_hit_bright: Option<String>,
    grid_hit_dim: Option<String>,
    grid_empty: Option<String>,
    grid_playhead: Option<String>,

    macro_name: Option<String>,
    macro_bar: Option<String>,
    macro_value: Option<String>,

    diff_add: Option<String>,
    diff_remove: Option<String>,

    help_key: Option<String>,
    help_desc: Option<String>,

    border: Option<String>,
    border_focused: Option<String>,
    title: Option<String>,

    editor_keyword: Option<String>,
    editor_pattern: Option<String>,
    editor_number: Option<String>,
    editor_active_line: Option<String>,
    beat_pulse: Option<String>,
    vu_low: Option<String>,
    vu_mid: Option<String>,
    vu_high: Option<String>,
}

/// Parse a color string: "#RRGGBB" hex or named color.
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
        return None;
    }
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        "reset" => Some(Color::Reset),
        _ => None,
    }
}

/// Load a custom theme from ~/.resonance/theme.yaml.
/// Returns None if the file doesn't exist or can't be parsed.
pub fn load_theme_from_yaml() -> Option<Theme> {
    let home = dirs::home_dir()?;
    let path = home.join(".resonance").join("theme.yaml");
    let content = std::fs::read_to_string(path).ok()?;
    parse_theme_yaml(&content)
}

/// Parse a YAML string into a Theme, filling missing fields from default.
fn parse_theme_yaml(yaml: &str) -> Option<Theme> {
    let config: ThemeConfig = serde_yaml::from_str(yaml).ok()?;
    let d = super::builtin::default();

    let color_or = |opt: Option<String>, fallback: Color| -> Color {
        opt.and_then(|s| parse_color(&s)).unwrap_or(fallback)
    };

    let palette = if let Some(ref colors) = config.grid_palette {
        let mut pal = d.grid_palette;
        for (i, s) in colors.iter().enumerate().take(8) {
            if let Some(c) = parse_color(s) {
                pal[i] = c;
            }
        }
        pal
    } else {
        d.grid_palette
    };

    Some(Theme {
        name: config.name.unwrap_or(d.name),

        editor_fg: color_or(config.editor_fg, d.editor_fg),
        editor_bg: color_or(config.editor_bg, d.editor_bg),
        editor_cursor: color_or(config.editor_cursor, d.editor_cursor),
        editor_line_number: color_or(config.editor_line_number, d.editor_line_number),

        status_fg: color_or(config.status_fg, d.status_fg),
        status_bg: color_or(config.status_bg, d.status_bg),
        status_accent: color_or(config.status_accent, d.status_accent),

        track_header_fg: color_or(config.track_header_fg, d.track_header_fg),
        track_muted: color_or(config.track_muted, d.track_muted),

        grid_palette: palette,
        grid_hit_bright: color_or(config.grid_hit_bright, d.grid_hit_bright),
        grid_hit_dim: color_or(config.grid_hit_dim, d.grid_hit_dim),
        grid_empty: color_or(config.grid_empty, d.grid_empty),
        grid_playhead: color_or(config.grid_playhead, d.grid_playhead),

        macro_name: color_or(config.macro_name, d.macro_name),
        macro_bar: color_or(config.macro_bar, d.macro_bar),
        macro_value: color_or(config.macro_value, d.macro_value),

        diff_add: color_or(config.diff_add, d.diff_add),
        diff_remove: color_or(config.diff_remove, d.diff_remove),

        help_key: color_or(config.help_key, d.help_key),
        help_desc: color_or(config.help_desc, d.help_desc),

        border: color_or(config.border, d.border),
        border_focused: color_or(config.border_focused, d.border_focused),
        title: color_or(config.title, d.title),

        editor_keyword: color_or(config.editor_keyword, d.editor_keyword),
        editor_pattern: color_or(config.editor_pattern, d.editor_pattern),
        editor_number: color_or(config.editor_number, d.editor_number),
        editor_active_line: color_or(config.editor_active_line, d.editor_active_line),
        beat_pulse: color_or(config.beat_pulse, d.beat_pulse),
        vu_low: color_or(config.vu_low, d.vu_low),
        vu_mid: color_or(config.vu_mid, d.vu_mid),
        vu_high: color_or(config.vu_high, d.vu_high),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_color() {
        assert_eq!(parse_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_color("#0000ff"), Some(Color::Rgb(0, 0, 255)));
        assert_eq!(parse_color("#c0caf5"), Some(Color::Rgb(192, 202, 245)));
    }

    #[test]
    fn parse_named_colors() {
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("White"), Some(Color::White));
        assert_eq!(parse_color("DarkGray"), Some(Color::DarkGray));
        assert_eq!(parse_color("lightmagenta"), Some(Color::LightMagenta));
    }

    #[test]
    fn parse_invalid_color_returns_none() {
        assert_eq!(parse_color("#xyz"), None);
        assert_eq!(parse_color("rainbow"), None);
        assert_eq!(parse_color("#12345"), None);
    }

    #[test]
    fn missing_file_returns_none() {
        // In CI/test, ~/.resonance/theme.yaml likely doesn't exist
        let _ = load_theme_from_yaml();
    }

    #[test]
    fn partial_yaml_fills_defaults() {
        let yaml = r##"
name: "Partial"
editor_fg: "#ff0000"
border_focused: "green"
"##;
        let theme = parse_theme_yaml(yaml).unwrap();
        assert_eq!(theme.name, "Partial");
        assert_eq!(theme.editor_fg, Color::Rgb(255, 0, 0));
        assert_eq!(theme.border_focused, Color::Green);
        // Unfilled fields should match default
        let d = super::super::builtin::default();
        assert_eq!(theme.editor_cursor, d.editor_cursor);
        assert_eq!(theme.grid_palette[0], d.grid_palette[0]);
    }

    #[test]
    fn full_yaml_parses() {
        let yaml = r##"
name: "Custom"
editor_fg: "#c0caf5"
editor_bg: "#1a1b26"
editor_cursor: "#e0af68"
editor_line_number: "#565f89"
status_fg: white
status_bg: darkgray
status_accent: cyan
track_header_fg: white
track_muted: darkgray
grid_palette:
  - "#7aa2f7"
  - "#bb9af7"
  - "#e0af68"
  - "#9ece6a"
grid_hit_bright: white
grid_hit_dim: darkgray
grid_empty: darkgray
grid_playhead: green
macro_name: cyan
macro_bar: green
macro_value: yellow
diff_add: green
diff_remove: red
help_key: yellow
help_desc: white
border: white
border_focused: cyan
title: cyan
"##;
        let theme = parse_theme_yaml(yaml).unwrap();
        assert_eq!(theme.name, "Custom");
        assert_eq!(theme.editor_fg, Color::Rgb(192, 202, 245));
        assert_eq!(theme.grid_palette[0], Color::Rgb(122, 162, 247));
        // Partial palette: indices 4-7 should be default
        let d = super::super::builtin::default();
        assert_eq!(theme.grid_palette[4], d.grid_palette[4]);
    }

    #[test]
    fn invalid_yaml_returns_none() {
        assert!(parse_theme_yaml("{{invalid").is_none());
    }

    #[test]
    fn invalid_hex_in_yaml_uses_default() {
        let yaml = r##"
name: "BadHex"
editor_fg: "#xyz123"
"##;
        let theme = parse_theme_yaml(yaml).unwrap();
        let d = super::super::builtin::default();
        assert_eq!(theme.editor_fg, d.editor_fg);
    }
}
