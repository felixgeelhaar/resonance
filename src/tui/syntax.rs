//! Lightweight per-line DSL syntax highlighter for the editor.

use ratatui::style::Style;
use ratatui::text::Span;

use super::theme::Theme;

/// DSL keywords that get keyword highlighting.
const KEYWORDS: &[&str] = &[
    "track",
    "tempo",
    "macro",
    "map",
    "section",
    "layer",
    "kit",
    "pattern",
    "note",
    "velocity",
    "swing",
    "humanize",
    "reverb",
    "delay",
    "drive",
    "sidechain",
    "limiter",
    "enabled",
];

/// Highlight a single line of DSL source into styled spans.
pub fn highlight_line<'a>(line: &'a str, theme: &Theme) -> Vec<Span<'a>> {
    if line.is_empty() {
        return vec![Span::raw("")];
    }

    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some(&(start, ch)) = chars.peek() {
        // Comment: // to end of line
        if ch == '/' {
            let rest = &line[start..];
            if rest.starts_with("//") {
                spans.push(Span::styled(
                    rest,
                    Style::default().fg(theme.editor_line_number),
                ));
                return spans;
            }
        }

        // Pattern brackets: [...]
        if ch == '[' {
            // Find matching ]
            let end = line[start..].find(']').map(|i| start + i + 1);
            if let Some(end) = end {
                spans.push(Span::styled(
                    &line[start..end],
                    Style::default().fg(theme.editor_pattern),
                ));
                // Advance past the bracket content
                while chars.peek().is_some_and(|&(i, _)| i < end) {
                    chars.next();
                }
                continue;
            }
        }

        // String literals: "..."
        if ch == '"' {
            let rest = &line[start + 1..];
            let end = rest
                .find('"')
                .map(|i| start + 1 + i + 1)
                .unwrap_or(line.len());
            spans.push(Span::styled(
                &line[start..end],
                Style::default().fg(theme.editor_pattern),
            ));
            while chars.peek().is_some_and(|&(i, _)| i < end) {
                chars.next();
            }
            continue;
        }

        // Numbers
        if ch.is_ascii_digit()
            || (ch == '-'
                && chars
                    .clone()
                    .nth(1)
                    .is_some_and(|(_, c)| c.is_ascii_digit()))
        {
            let mut end = start;
            let mut saw_dot = false;
            // skip leading minus
            if ch == '-' {
                end += 1;
            }
            for (i, c) in chars.clone().skip(if ch == '-' { 1 } else { 0 }) {
                if c.is_ascii_digit() {
                    end = i + c.len_utf8();
                } else if c == '.' && !saw_dot {
                    saw_dot = true;
                    end = i + 1;
                } else {
                    break;
                }
            }
            // Only highlight if it's not part of an identifier
            let before = if start > 0 {
                line.as_bytes().get(start - 1).map(|&b| b as char)
            } else {
                None
            };
            if before.is_none() || !before.unwrap().is_alphanumeric() {
                spans.push(Span::styled(
                    &line[start..end],
                    Style::default().fg(theme.editor_number),
                ));
                while chars.peek().is_some_and(|&(i, _)| i < end) {
                    chars.next();
                }
                continue;
            }
        }

        // Words (identifiers/keywords)
        if ch.is_alphabetic() || ch == '_' {
            let mut end = start;
            for (i, c) in chars.clone() {
                if c.is_alphanumeric() || c == '_' {
                    end = i + c.len_utf8();
                } else {
                    break;
                }
            }
            let word = &line[start..end];
            if KEYWORDS.contains(&word) {
                spans.push(Span::styled(
                    word,
                    Style::default().fg(theme.editor_keyword),
                ));
            } else {
                spans.push(Span::styled(word, Style::default().fg(theme.editor_fg)));
            }
            while chars.peek().is_some_and(|&(i, _)| i < end) {
                chars.next();
            }
            continue;
        }

        // Everything else: punctuation, whitespace, operators
        let byte_len = ch.len_utf8();
        spans.push(Span::styled(
            &line[start..start + byte_len],
            Style::default().fg(theme.editor_fg),
        ));
        chars.next();
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::theme::builtin;
    use ratatui::style::Color;

    fn get_colors(spans: &[Span]) -> Vec<Color> {
        spans
            .iter()
            .filter(|s| !s.content.trim().is_empty())
            .map(|s| s.style.fg.unwrap_or(Color::Reset))
            .collect()
    }

    #[test]
    fn keywords_highlighted() {
        let theme = builtin::default();
        let spans = highlight_line("track drums", &theme);
        // "track" should be keyword color
        assert!(!spans.is_empty());
        assert_eq!(spans[0].style.fg.unwrap(), theme.editor_keyword);
    }

    #[test]
    fn pattern_brackets_highlighted() {
        let theme = builtin::default();
        let spans = highlight_line("[X . x .]", &theme);
        assert!(!spans.is_empty());
        assert_eq!(spans[0].style.fg.unwrap(), theme.editor_pattern);
    }

    #[test]
    fn numbers_highlighted() {
        let theme = builtin::default();
        let spans = highlight_line("tempo 120", &theme);
        let colors = get_colors(&spans);
        assert!(colors.contains(&theme.editor_keyword)); // "tempo"
        assert!(colors.contains(&theme.editor_number)); // "120"
    }

    #[test]
    fn plain_text_uses_editor_fg() {
        let theme = builtin::default();
        let spans = highlight_line("myvar", &theme);
        assert_eq!(spans[0].style.fg.unwrap(), theme.editor_fg);
    }

    #[test]
    fn empty_line() {
        let theme = builtin::default();
        let spans = highlight_line("", &theme);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "");
    }

    #[test]
    fn comment_highlighted() {
        let theme = builtin::default();
        let spans = highlight_line("// this is a comment", &theme);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg.unwrap(), theme.editor_line_number);
    }

    #[test]
    fn mixed_line() {
        let theme = builtin::default();
        let spans = highlight_line("track drums { kit: default }", &theme);
        assert!(spans.len() >= 3);
        // First span should be keyword "track"
        assert_eq!(spans[0].style.fg.unwrap(), theme.editor_keyword);
    }

    #[test]
    fn strudel_theme_syntax_colors() {
        let theme = builtin::strudel();
        let spans = highlight_line("tempo 128", &theme);
        let colors = get_colors(&spans);
        assert!(colors.contains(&theme.editor_keyword));
        assert!(colors.contains(&theme.editor_number));
    }
}
