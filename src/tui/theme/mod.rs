//! Theme system — configurable color schemes for the TUI.

pub mod builtin;
pub mod config;

use ratatui::style::Color;

/// A complete color theme for the TUI.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,

    // Editor
    pub editor_fg: Color,
    pub editor_bg: Color,
    pub editor_cursor: Color,
    pub editor_line_number: Color,

    // Status bar
    pub status_fg: Color,
    pub status_bg: Color,
    pub status_accent: Color,

    // Tracks panel
    pub track_header_fg: Color,
    pub track_muted: Color,

    // Grid
    pub grid_palette: [Color; 8],
    pub grid_hit_bright: Color,
    pub grid_hit_dim: Color,
    pub grid_empty: Color,
    pub grid_playhead: Color,

    // Macros panel
    pub macro_name: Color,
    pub macro_bar: Color,
    pub macro_value: Color,

    // Diff preview
    pub diff_add: Color,
    pub diff_remove: Color,

    // Help
    pub help_key: Color,
    pub help_desc: Color,

    // Borders & chrome
    pub border: Color,
    pub border_focused: Color,
    pub title: Color,

    // Syntax highlighting
    pub editor_keyword: Color,
    pub editor_pattern: Color,
    pub editor_number: Color,
    pub editor_active_line: Color,

    // Beat & VU
    pub beat_pulse: Color,
    pub vu_low: Color,
    pub vu_mid: Color,
    pub vu_high: Color,
}

/// Load a theme: tries YAML config first, falls back to the default builtin.
pub fn load_theme() -> Theme {
    config::load_theme_from_yaml().unwrap_or_else(builtin::default)
}

/// Cycle to the next theme in the list, wrapping around.
pub fn cycle_theme(current: &Theme, themes: &[Theme]) -> Theme {
    if themes.is_empty() {
        return current.clone();
    }
    let idx = themes
        .iter()
        .position(|t| t.name == current.name)
        .map(|i| (i + 1) % themes.len())
        .unwrap_or(0);
    themes[idx].clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_has_all_fields() {
        let theme = builtin::default();
        assert_eq!(theme.name, "Default");
        assert_eq!(theme.grid_palette.len(), 8);
    }

    #[test]
    fn theme_clone_works() {
        let theme = builtin::default();
        let cloned = theme.clone();
        assert_eq!(cloned.name, theme.name);
        assert_eq!(cloned.border_focused, theme.border_focused);
    }

    #[test]
    fn load_theme_returns_default_without_yaml() {
        let theme = load_theme();
        // Without a custom YAML, should return a valid theme
        assert!(!theme.name.is_empty());
    }

    #[test]
    fn cycle_single_theme_stays() {
        let theme = builtin::default();
        let themes = vec![theme.clone()];
        let next = cycle_theme(&theme, &themes);
        assert_eq!(next.name, theme.name);
    }

    #[test]
    fn cycle_wraps_around() {
        let themes = builtin::all_builtins();
        assert!(themes.len() >= 2);
        let last = &themes[themes.len() - 1];
        let next = cycle_theme(last, &themes);
        assert_eq!(next.name, themes[0].name);
    }

    #[test]
    fn cycle_advances_to_next() {
        let themes = builtin::all_builtins();
        let first = &themes[0];
        let next = cycle_theme(first, &themes);
        assert_eq!(next.name, themes[1].name);
    }

    #[test]
    fn cycle_empty_themes_returns_current() {
        let theme = builtin::default();
        let next = cycle_theme(&theme, &[]);
        assert_eq!(next.name, theme.name);
    }
}
