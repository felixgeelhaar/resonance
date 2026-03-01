//! Help screen — modal overlay showing keybinding reference.

/// A line in the help screen.
#[derive(Debug, Clone)]
pub struct HelpLine {
    pub text: String,
    pub is_header: bool,
}

/// Help screen state.
#[derive(Debug, Clone)]
pub struct HelpScreen {
    pub visible: bool,
    pub scroll_offset: usize,
    content: Vec<HelpLine>,
}

impl HelpScreen {
    /// Create a new help screen with the full keybinding reference.
    pub fn new() -> Self {
        Self {
            visible: false,
            scroll_offset: 0,
            content: Self::build_content(),
        }
    }

    /// Toggle visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.scroll_offset = 0;
        }
    }

    /// Show the help screen.
    pub fn show(&mut self) {
        self.visible = true;
        self.scroll_offset = 0;
    }

    /// Hide the help screen.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Scroll up.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down.
    pub fn scroll_down(&mut self, max_visible: usize) {
        let max_scroll = self.content.len().saturating_sub(max_visible);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Get all help lines.
    pub fn lines(&self) -> &[HelpLine] {
        &self.content
    }

    fn build_content() -> Vec<HelpLine> {
        let mut lines = Vec::new();

        let h = |text: &str| HelpLine {
            text: text.to_string(),
            is_header: true,
        };
        let l = |text: &str| HelpLine {
            text: text.to_string(),
            is_header: false,
        };

        lines.push(h("GLOBAL (all modes, all panels)"));
        lines.push(l("  Ctrl-Q       Quit"));
        lines.push(l("  Ctrl-Enter   Evaluate code (REPL)"));
        lines.push(l("  Ctrl-R       Compile & reload DSL"));
        lines.push(l("  Ctrl-P       Toggle Edit/Perform mode"));
        lines.push(l("  Ctrl-;       Open command bar"));
        lines.push(l("  Tab          Cycle panel focus"));
        lines.push(l("  Esc          Close overlay / return to editor"));
        lines.push(l("  ?            Toggle this help screen"));
        lines.push(l("  Shift-?      DSL quick reference"));
        lines.push(l("  Ctrl-L       Toggle crash log"));
        lines.push(l("  Ctrl-T       Cycle theme"));
        lines.push(l("  Ctrl-,       Open settings"));
        lines.push(l("  Ctrl-D       Reconnect audio device"));
        lines.push(l(""));

        lines.push(h("EDIT MODE (editor panel focused)"));
        lines.push(l("  Any key      Insert character"));
        lines.push(l("  Backspace    Delete before cursor"));
        lines.push(l("  Delete       Delete at cursor"));
        lines.push(l("  Enter        New line"));
        lines.push(l("  Arrows       Move cursor"));
        lines.push(l("  Home/End     Start/end of line"));
        lines.push(l(""));

        lines.push(h("PERFORM MODE"));
        lines.push(l("  Space        Toggle play/stop"));
        lines.push(l("  1-9          Jump to section"));
        lines.push(l("  Shift+1-9    Toggle layer"));
        lines.push(l("  F1-F8        Adjust macro (+5%)"));
        lines.push(l("  Shift+F1-F8  Fine adjust macro (+1%)"));
        lines.push(l("  Ctrl-Z       Undo macro change"));
        lines.push(l("  Ctrl-Y       Redo macro change"));
        lines.push(l("  +/-          Grid zoom in/out"));
        lines.push(l(""));

        lines.push(h("COMMAND BAR"));
        lines.push(l("  :tutorial    Start interactive tutorial"));
        lines.push(l("  :next/:prev  Navigate tutorial lessons"));
        lines.push(l("  :preset NAME Load a preset (house/techno/ambient/dnb)"));
        lines.push(l("  :presets     List available presets"));
        lines.push(l("  :ref         DSL quick reference"));
        lines.push(l("  :help        Toggle help screen"));
        lines.push(l("  :eval        Evaluate code (same as Ctrl-Enter)"));
        lines.push(l("  :audio       Reconnect audio device"));
        lines.push(l("  :settings    Open settings panel"));
        lines.push(l("  :clear       Clear editor"));
        lines.push(l("  (text)       Natural language command"));
        lines.push(l(""));

        lines.push(h("DIFF PREVIEW"));
        lines.push(l("  Enter        Accept proposed changes"));
        lines.push(l("  Esc          Reject proposed changes"));
        lines.push(l("  Up/Down      Scroll preview"));
        lines.push(l(""));

        lines.push(h("TUTORIAL (when active)"));
        lines.push(l("  Ctrl-Right   Next lesson"));
        lines.push(l("  Ctrl-Left    Previous lesson"));
        lines.push(l(""));

        lines.push(h("TIPS"));
        lines.push(l("  - Ctrl-Enter evaluates and auto-starts playback"));
        lines.push(l("  - Use Ctrl-; then type 'add reverb' or 'faster'"));
        lines.push(l("  - Tab switches focus between panels"));
        lines.push(l("  - Keys only edit when Editor panel is focused"));
        lines.push(l("  - Compile with Ctrl-R to see grid visualization"));

        lines
    }
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hidden() {
        let help = HelpScreen::new();
        assert!(!help.visible);
        assert_eq!(help.scroll_offset, 0);
    }

    #[test]
    fn toggle_shows_and_hides() {
        let mut help = HelpScreen::new();
        help.toggle();
        assert!(help.visible);
        help.toggle();
        assert!(!help.visible);
    }

    #[test]
    fn toggle_resets_scroll() {
        let mut help = HelpScreen::new();
        help.show();
        help.scroll_down(5);
        assert!(help.scroll_offset > 0);
        help.hide();
        help.toggle(); // show again
        assert_eq!(help.scroll_offset, 0);
    }

    #[test]
    fn content_not_empty() {
        let help = HelpScreen::new();
        assert!(!help.lines().is_empty());
    }

    #[test]
    fn has_section_headers() {
        let help = HelpScreen::new();
        let headers: Vec<_> = help.lines().iter().filter(|l| l.is_header).collect();
        assert!(headers.len() >= 6); // Global, Edit, Perform, Command Bar, Diff Preview, Tutorial, Tips
    }

    #[test]
    fn scroll_bounds() {
        let mut help = HelpScreen::new();
        help.scroll_up(); // should not underflow
        assert_eq!(help.scroll_offset, 0);

        // Scroll down many times
        for _ in 0..200 {
            help.scroll_down(10);
        }
        // Should be clamped to content length - visible
        assert!(help.scroll_offset <= help.lines().len());
    }
}
