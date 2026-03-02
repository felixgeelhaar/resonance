//! Command bar — single-line input widget for `:` commands and natural language input.

/// Command bar state.
#[derive(Debug, Clone)]
pub struct CommandBar {
    pub active: bool,
    input: String,
    cursor_pos: usize,
    history: Vec<String>,
    history_idx: Option<usize>,
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_idx: None,
        }
    }

    /// Activate the command bar (show cursor, accept input).
    pub fn activate(&mut self) {
        self.active = true;
        self.input.clear();
        self.cursor_pos = 0;
        self.history_idx = None;
    }

    /// Deactivate the command bar (hide, clear input).
    pub fn deactivate(&mut self) {
        self.active = false;
        self.input.clear();
        self.cursor_pos = 0;
        self.history_idx = None;
    }

    /// Get the current input text.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Get the cursor position.
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.history_idx = None;
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            // Find the previous character boundary
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
        }
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
        }
    }

    /// Navigate history up (older).
    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_idx {
            Some(i) if i + 1 < self.history.len() => i + 1,
            Some(i) => i,
            None => 0,
        };
        self.history_idx = Some(idx);
        let entry = self.history[self.history.len() - 1 - idx].clone();
        self.input = entry;
        self.cursor_pos = self.input.len();
    }

    /// Navigate history down (newer).
    pub fn history_down(&mut self) {
        match self.history_idx {
            Some(0) => {
                self.history_idx = None;
                self.input.clear();
                self.cursor_pos = 0;
            }
            Some(i) => {
                let idx = i - 1;
                self.history_idx = Some(idx);
                let entry = self.history[self.history.len() - 1 - idx].clone();
                self.input = entry;
                self.cursor_pos = self.input.len();
            }
            None => {}
        }
    }

    /// Submit the current input. Returns the text and clears the bar.
    /// Adds to history if non-empty.
    pub fn submit(&mut self) -> String {
        let text = self.input.clone();
        if !text.is_empty() {
            self.history.push(text.clone());
        }
        self.input.clear();
        self.cursor_pos = 0;
        self.history_idx = None;
        self.active = false;
        text
    }
}

impl Default for CommandBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_inactive() {
        let bar = CommandBar::new();
        assert!(!bar.active);
        assert!(bar.input().is_empty());
    }

    #[test]
    fn activate_deactivate() {
        let mut bar = CommandBar::new();
        bar.activate();
        assert!(bar.active);
        bar.deactivate();
        assert!(!bar.active);
    }

    #[test]
    fn insert_and_read() {
        let mut bar = CommandBar::new();
        bar.activate();
        bar.insert_char('h');
        bar.insert_char('i');
        assert_eq!(bar.input(), "hi");
        assert_eq!(bar.cursor_pos(), 2);
    }

    #[test]
    fn backspace() {
        let mut bar = CommandBar::new();
        bar.activate();
        bar.insert_char('a');
        bar.insert_char('b');
        bar.backspace();
        assert_eq!(bar.input(), "a");
        assert_eq!(bar.cursor_pos(), 1);
    }

    #[test]
    fn backspace_at_start() {
        let mut bar = CommandBar::new();
        bar.activate();
        bar.backspace(); // Should not panic
        assert!(bar.input().is_empty());
    }

    #[test]
    fn cursor_movement() {
        let mut bar = CommandBar::new();
        bar.activate();
        bar.insert_char('a');
        bar.insert_char('b');
        bar.insert_char('c');
        assert_eq!(bar.cursor_pos(), 3);
        bar.move_left();
        assert_eq!(bar.cursor_pos(), 2);
        bar.move_left();
        assert_eq!(bar.cursor_pos(), 1);
        bar.move_right();
        assert_eq!(bar.cursor_pos(), 2);
    }

    #[test]
    fn cursor_bounds() {
        let mut bar = CommandBar::new();
        bar.activate();
        bar.move_left(); // Already at 0
        assert_eq!(bar.cursor_pos(), 0);
        bar.insert_char('x');
        bar.move_right(); // Already at end
        assert_eq!(bar.cursor_pos(), 1);
    }

    #[test]
    fn submit_clears_and_returns() {
        let mut bar = CommandBar::new();
        bar.activate();
        bar.insert_char(':');
        bar.insert_char('h');
        let text = bar.submit();
        assert_eq!(text, ":h");
        assert!(bar.input().is_empty());
        assert!(!bar.active);
    }

    #[test]
    fn submit_adds_to_history() {
        let mut bar = CommandBar::new();
        bar.activate();
        bar.insert_char('a');
        bar.submit();
        bar.activate();
        bar.insert_char('b');
        bar.submit();
        assert_eq!(bar.history.len(), 2);
    }

    #[test]
    fn history_navigation() {
        let mut bar = CommandBar::new();
        bar.activate();
        bar.insert_char('a');
        bar.submit();
        bar.activate();
        bar.insert_char('b');
        bar.submit();

        bar.activate();
        bar.history_up();
        assert_eq!(bar.input(), "b");
        bar.history_up();
        assert_eq!(bar.input(), "a");
        bar.history_down();
        assert_eq!(bar.input(), "b");
        bar.history_down();
        assert!(bar.input().is_empty());
    }

    #[test]
    fn empty_submit_not_in_history() {
        let mut bar = CommandBar::new();
        bar.activate();
        let text = bar.submit();
        assert!(text.is_empty());
        assert!(bar.history.is_empty());
    }
}
