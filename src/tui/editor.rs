//! DSL code editor — simple text buffer with cursor.

/// A minimal text editor for DSL source code.
#[derive(Debug, Clone)]
pub struct Editor {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_offset: usize,
    viewport_height: usize,
}

impl Editor {
    /// Create an editor with initial content.
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|l| l.to_string()).collect()
        };
        Self {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            viewport_height: 20,
        }
    }

    /// Get the full text content.
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Get all lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get cursor position (row, col).
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Get the current scroll offset (first visible line).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set the viewport height (number of visible lines).
    pub fn set_viewport_height(&mut self, h: usize) {
        self.viewport_height = h.max(1);
        self.ensure_cursor_visible();
    }

    /// Ensure the cursor is within the visible viewport, adjusting scroll_offset.
    fn ensure_cursor_visible(&mut self) {
        if self.viewport_height == 0 {
            return;
        }
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.cursor_row - self.viewport_height + 1;
        }
    }

    /// Insert a character at the cursor.
    pub fn insert_char(&mut self, c: char) {
        if self.cursor_row < self.lines.len() {
            let line = &mut self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            line.insert(col, c);
            self.cursor_col = col + 1;
        }
    }

    /// Insert a new line at the cursor.
    pub fn newline(&mut self) {
        if self.cursor_row < self.lines.len() {
            let col = self.cursor_col.min(self.lines[self.cursor_row].len());
            let rest = self.lines[self.cursor_row][col..].to_string();
            self.lines[self.cursor_row].truncate(col);
            self.cursor_row += 1;
            self.lines.insert(self.cursor_row, rest);
            self.cursor_col = 0;
            self.ensure_cursor_visible();
        }
    }

    /// Delete character before cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let col = self.cursor_col.min(self.lines[self.cursor_row].len());
            self.lines[self.cursor_row].remove(col - 1);
            self.cursor_col = col - 1;
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.ensure_cursor_visible();
        }
    }

    /// Delete character at cursor.
    pub fn delete(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_row].remove(self.cursor_col);
        } else if self.cursor_row + 1 < self.lines.len() {
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
        }
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    /// Move cursor up.
    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
            self.ensure_cursor_visible();
        }
    }

    /// Move cursor down.
    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
            self.ensure_cursor_visible();
        }
    }

    /// Move cursor to start of line.
    pub fn home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of line.
    pub fn end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    /// Replace all content.
    pub fn set_content(&mut self, content: &str) {
        self.lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|l| l.to_string()).collect()
        };
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    /// Number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_content() {
        let ed = Editor::new("hello\nworld");
        assert_eq!(ed.line_count(), 2);
        assert_eq!(ed.lines()[0], "hello");
        assert_eq!(ed.lines()[1], "world");
    }

    #[test]
    fn new_empty() {
        let ed = Editor::new("");
        assert_eq!(ed.line_count(), 1);
        assert_eq!(ed.lines()[0], "");
    }

    #[test]
    fn insert_char() {
        let mut ed = Editor::new("");
        ed.insert_char('a');
        ed.insert_char('b');
        assert_eq!(ed.content(), "ab");
        assert_eq!(ed.cursor(), (0, 2));
    }

    #[test]
    fn newline_splits_line() {
        let mut ed = Editor::new("hello");
        ed.cursor_col = 3;
        ed.newline();
        assert_eq!(ed.lines()[0], "hel");
        assert_eq!(ed.lines()[1], "lo");
        assert_eq!(ed.cursor(), (1, 0));
    }

    #[test]
    fn backspace_removes_char() {
        let mut ed = Editor::new("abc");
        ed.cursor_col = 2;
        ed.backspace();
        assert_eq!(ed.content(), "ac");
        assert_eq!(ed.cursor(), (0, 1));
    }

    #[test]
    fn backspace_joins_lines() {
        let mut ed = Editor::new("hello\nworld");
        ed.cursor_row = 1;
        ed.cursor_col = 0;
        ed.backspace();
        assert_eq!(ed.content(), "helloworld");
        assert_eq!(ed.cursor(), (0, 5));
    }

    #[test]
    fn delete_removes_at_cursor() {
        let mut ed = Editor::new("abc");
        ed.cursor_col = 1;
        ed.delete();
        assert_eq!(ed.content(), "ac");
    }

    #[test]
    fn delete_joins_next_line() {
        let mut ed = Editor::new("hello\nworld");
        ed.cursor_col = 5; // end of first line
        ed.delete();
        assert_eq!(ed.content(), "helloworld");
    }

    #[test]
    fn move_left_right() {
        let mut ed = Editor::new("abc");
        ed.move_right();
        assert_eq!(ed.cursor(), (0, 1));
        ed.move_left();
        assert_eq!(ed.cursor(), (0, 0));
    }

    #[test]
    fn move_up_down() {
        let mut ed = Editor::new("line1\nline2\nline3");
        ed.move_down();
        assert_eq!(ed.cursor(), (1, 0));
        ed.move_down();
        assert_eq!(ed.cursor(), (2, 0));
        ed.move_up();
        assert_eq!(ed.cursor(), (1, 0));
    }

    #[test]
    fn home_and_end() {
        let mut ed = Editor::new("hello");
        ed.cursor_col = 3;
        ed.home();
        assert_eq!(ed.cursor(), (0, 0));
        ed.end();
        assert_eq!(ed.cursor(), (0, 5));
    }

    #[test]
    fn cursor_clamps_on_move() {
        let mut ed = Editor::new("short\nlonger line");
        ed.cursor_row = 1;
        ed.cursor_col = 10;
        ed.move_up(); // moves to row 0, col clamped to "short".len() = 5
        assert_eq!(ed.cursor(), (0, 5));
    }

    #[test]
    fn set_content_resets() {
        let mut ed = Editor::new("old");
        ed.cursor_col = 3;
        ed.set_content("new\ncontent");
        assert_eq!(ed.cursor(), (0, 0));
        assert_eq!(ed.line_count(), 2);
    }

    #[test]
    fn content_round_trip() {
        let src = "tempo 128\ntrack drums {\n  kit: default\n}";
        let ed = Editor::new(src);
        assert_eq!(ed.content(), src);
    }

    #[test]
    fn scroll_offset_starts_at_zero() {
        let ed = Editor::new("hello\nworld");
        assert_eq!(ed.scroll_offset(), 0);
    }

    #[test]
    fn cursor_below_viewport_scrolls_down() {
        let content: String = (0..30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut ed = Editor::new(&content);
        ed.set_viewport_height(10);
        // Move cursor to row 25
        for _ in 0..25 {
            ed.move_down();
        }
        assert_eq!(ed.cursor().0, 25);
        assert_eq!(ed.scroll_offset(), 16); // 25 - 10 + 1
    }

    #[test]
    fn cursor_above_viewport_scrolls_up() {
        let content: String = (0..30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut ed = Editor::new(&content);
        ed.set_viewport_height(10);
        // Move down to row 25, then back up past scroll_offset
        for _ in 0..25 {
            ed.move_down();
        }
        assert_eq!(ed.scroll_offset(), 16);
        // Move up to row 10 — should adjust scroll_offset
        for _ in 0..15 {
            ed.move_up();
        }
        assert_eq!(ed.cursor().0, 10);
        assert_eq!(ed.scroll_offset(), 10);
    }

    #[test]
    fn set_content_resets_scroll_offset() {
        let content: String = (0..30)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut ed = Editor::new(&content);
        ed.set_viewport_height(10);
        for _ in 0..25 {
            ed.move_down();
        }
        assert!(ed.scroll_offset() > 0);
        ed.set_content("new content");
        assert_eq!(ed.scroll_offset(), 0);
        assert_eq!(ed.cursor(), (0, 0));
    }

    #[test]
    fn newline_scrolls_when_at_bottom() {
        let content: String = (0..15)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut ed = Editor::new(&content);
        ed.set_viewport_height(10);
        // Move to row 9 (last visible row when scroll_offset=0)
        for _ in 0..9 {
            ed.move_down();
        }
        assert_eq!(ed.scroll_offset(), 0);
        // Newline should push cursor to row 10, triggering scroll
        ed.newline();
        assert_eq!(ed.cursor().0, 10);
        assert_eq!(ed.scroll_offset(), 1);
    }
}
