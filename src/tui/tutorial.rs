//! Tutorial mode — progressive lesson navigation with explanation overlay.

use crate::content::tutorials::{TutorialLesson, TutorialPack};

/// Tutorial mode state.
#[derive(Debug, Clone)]
pub struct TutorialMode {
    pub active: bool,
    pack: Option<TutorialPack>,
    current_lesson_idx: usize,
    pub explanation_visible: bool,
    pub scroll_offset: usize,
}

impl TutorialMode {
    pub fn new() -> Self {
        Self {
            active: false,
            pack: None,
            current_lesson_idx: 0,
            explanation_visible: false,
            scroll_offset: 0,
        }
    }

    /// Start tutorial mode with the built-in tutorial pack.
    pub fn start(&mut self) {
        self.load_pack(crate::content::tutorials::builtin_tutorial());
    }

    /// Load a tutorial pack and activate tutorial mode.
    pub fn load_pack(&mut self, pack: TutorialPack) {
        self.pack = Some(pack);
        self.active = true;
        self.current_lesson_idx = 0;
        self.explanation_visible = true;
        self.scroll_offset = 0;
    }

    /// Stop tutorial mode.
    pub fn stop(&mut self) {
        self.active = false;
        self.explanation_visible = false;
    }

    /// Get the current lesson (if any).
    pub fn current_lesson(&self) -> Option<&TutorialLesson> {
        self.pack
            .as_ref()
            .and_then(|p| p.lessons.get(self.current_lesson_idx))
    }

    /// Get the current lesson index.
    pub fn current_index(&self) -> usize {
        self.current_lesson_idx
    }

    /// Get the total number of lessons.
    pub fn total_lessons(&self) -> usize {
        self.pack.as_ref().map_or(0, |p| p.lessons.len())
    }

    /// Advance to the next lesson. Returns true if advanced, false if at end.
    pub fn next_lesson(&mut self) -> bool {
        if let Some(ref pack) = self.pack {
            if self.current_lesson_idx + 1 < pack.lessons.len() {
                self.current_lesson_idx += 1;
                self.explanation_visible = true;
                self.scroll_offset = 0;
                return true;
            }
        }
        false
    }

    /// Go back to the previous lesson. Returns true if moved back.
    pub fn prev_lesson(&mut self) -> bool {
        if self.current_lesson_idx > 0 {
            self.current_lesson_idx -= 1;
            self.explanation_visible = true;
            self.scroll_offset = 0;
            return true;
        }
        false
    }

    /// Toggle the explanation overlay visibility.
    pub fn toggle_explanation(&mut self) {
        self.explanation_visible = !self.explanation_visible;
        if self.explanation_visible {
            self.scroll_offset = 0;
        }
    }

    /// Scroll explanation up.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll explanation down.
    pub fn scroll_down(&mut self, max_visible: usize) {
        if let Some(lesson) = self.current_lesson() {
            let total = lesson.explanation.len() + lesson.hints.len() + 2; // +2 for header/spacing
            let max_scroll = total.saturating_sub(max_visible);
            if self.scroll_offset < max_scroll {
                self.scroll_offset += 1;
            }
        }
    }
}

impl Default for TutorialMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_inactive() {
        let tut = TutorialMode::new();
        assert!(!tut.active);
        assert!(tut.current_lesson().is_none());
    }

    #[test]
    fn start_loads_builtin() {
        let mut tut = TutorialMode::new();
        tut.start();
        assert!(tut.active);
        assert!(tut.current_lesson().is_some());
        assert_eq!(tut.current_index(), 0);
        assert!(tut.explanation_visible);
    }

    #[test]
    fn lesson_navigation() {
        let mut tut = TutorialMode::new();
        tut.start();
        let total = tut.total_lessons();
        assert!(total >= 5);

        // Forward
        assert!(tut.next_lesson());
        assert_eq!(tut.current_index(), 1);

        // Back
        assert!(tut.prev_lesson());
        assert_eq!(tut.current_index(), 0);

        // Can't go before first
        assert!(!tut.prev_lesson());
        assert_eq!(tut.current_index(), 0);
    }

    #[test]
    fn navigation_bounds() {
        let mut tut = TutorialMode::new();
        tut.start();
        let total = tut.total_lessons();

        // Go to last lesson
        for _ in 0..total {
            tut.next_lesson();
        }
        assert_eq!(tut.current_index(), total - 1);

        // Can't go past last
        assert!(!tut.next_lesson());
        assert_eq!(tut.current_index(), total - 1);
    }

    #[test]
    fn toggle_explanation() {
        let mut tut = TutorialMode::new();
        tut.start();
        assert!(tut.explanation_visible);
        tut.toggle_explanation();
        assert!(!tut.explanation_visible);
        tut.toggle_explanation();
        assert!(tut.explanation_visible);
    }

    #[test]
    fn scroll_bounds() {
        let mut tut = TutorialMode::new();
        tut.start();
        tut.scroll_up(); // Should not underflow
        assert_eq!(tut.scroll_offset, 0);
    }

    #[test]
    fn stop_deactivates() {
        let mut tut = TutorialMode::new();
        tut.start();
        assert!(tut.active);
        tut.stop();
        assert!(!tut.active);
        assert!(!tut.explanation_visible);
    }
}
