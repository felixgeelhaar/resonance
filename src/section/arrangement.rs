//! Arrangement controller — auto-advance sections with repeat counts.

use crate::dsl::ast::ArrangementEntry;

/// Controls auto-advancing through an arrangement plan.
#[derive(Debug, Clone)]
pub struct ArrangementController {
    entries: Vec<ArrangementEntry>,
    current_entry: usize,
    current_repeat: u32,
    active: bool,
}

impl ArrangementController {
    /// Create a new arrangement controller from entries.
    pub fn new(entries: Vec<ArrangementEntry>) -> Self {
        Self {
            entries,
            current_entry: 0,
            current_repeat: 0,
            active: true,
        }
    }

    /// Get the current section name, if any.
    pub fn current_section(&self) -> Option<&str> {
        if !self.active || self.is_complete() {
            return None;
        }
        self.entries
            .get(self.current_entry)
            .map(|e| e.section_name.as_str())
    }

    /// Check if the arrangement should advance, given that the current section has ended.
    /// Returns the next section name to transition to, or None if no change needed.
    pub fn check_advance(&mut self) -> Option<&str> {
        if !self.active || self.is_complete() {
            return None;
        }

        let entry = &self.entries[self.current_entry];
        self.current_repeat += 1;

        if self.current_repeat >= entry.repeats {
            // Move to next entry
            self.current_entry += 1;
            self.current_repeat = 0;

            if self.current_entry < self.entries.len() {
                Some(&self.entries[self.current_entry].section_name)
            } else {
                None // Arrangement complete
            }
        } else {
            // Repeat same section — return its name to re-trigger
            Some(&self.entries[self.current_entry].section_name)
        }
    }

    /// Whether all entries have been played through.
    pub fn is_complete(&self) -> bool {
        self.current_entry >= self.entries.len()
    }

    /// Total number of bars in the arrangement.
    pub fn total_bars(&self, section_bars: &dyn Fn(&str) -> u32) -> u32 {
        self.entries
            .iter()
            .map(|e| section_bars(&e.section_name) * e.repeats)
            .sum()
    }

    /// Reset to the beginning.
    pub fn reset(&mut self) {
        self.current_entry = 0;
        self.current_repeat = 0;
    }

    /// Enable or disable the arrangement.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Whether the arrangement is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get the current progress as (entry_index, total_entries, repeat_index, total_repeats).
    pub fn progress(&self) -> (usize, usize, u32, u32) {
        let total = self.entries.len();
        if self.is_complete() {
            return (total, total, 0, 0);
        }
        let repeats = self
            .entries
            .get(self.current_entry)
            .map(|e| e.repeats)
            .unwrap_or(1);
        (self.current_entry, total, self.current_repeat, repeats)
    }

    /// Jump to an entry matching the given section name.
    /// Returns true if found.
    pub fn jump_to(&mut self, section_name: &str) -> bool {
        if let Some(idx) = self
            .entries
            .iter()
            .position(|e| e.section_name == section_name)
        {
            self.current_entry = idx;
            self.current_repeat = 0;
            true
        } else {
            false
        }
    }

    /// Format a status string for display.
    pub fn status_string(&self) -> String {
        if !self.active {
            return "arr: off".to_string();
        }
        if self.is_complete() {
            return "arr: done".to_string();
        }
        let (entry_idx, total, rep, total_reps) = self.progress();
        let name = self.entries[entry_idx].section_name.as_str();
        format!(
            "[{}/{}] {} (rep {}/{})",
            entry_idx + 1,
            total,
            name,
            rep + 1,
            total_reps
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entries() -> Vec<ArrangementEntry> {
        vec![
            ArrangementEntry {
                section_name: "intro".to_string(),
                repeats: 1,
            },
            ArrangementEntry {
                section_name: "verse".to_string(),
                repeats: 2,
            },
            ArrangementEntry {
                section_name: "chorus".to_string(),
                repeats: 1,
            },
            ArrangementEntry {
                section_name: "outro".to_string(),
                repeats: 1,
            },
        ]
    }

    #[test]
    fn initial_section() {
        let ctrl = ArrangementController::new(test_entries());
        assert_eq!(ctrl.current_section(), Some("intro"));
        assert!(!ctrl.is_complete());
    }

    #[test]
    fn advance_through_entries() {
        let mut ctrl = ArrangementController::new(test_entries());

        // intro x1 — after 1 play, advance to verse
        let next = ctrl.check_advance();
        assert_eq!(next, Some("verse"));

        // verse x2 — first play stays on verse
        let next = ctrl.check_advance();
        assert_eq!(next, Some("verse"));

        // verse x2 — second play advances to chorus
        let next = ctrl.check_advance();
        assert_eq!(next, Some("chorus"));

        // chorus x1 — advance to outro
        let next = ctrl.check_advance();
        assert_eq!(next, Some("outro"));

        // outro x1 — arrangement complete
        let next = ctrl.check_advance();
        assert!(next.is_none());
        assert!(ctrl.is_complete());
    }

    #[test]
    fn total_bars() {
        let ctrl = ArrangementController::new(test_entries());
        let bars = ctrl.total_bars(&|name| match name {
            "intro" => 4,
            "verse" => 8,
            "chorus" => 8,
            "outro" => 4,
            _ => 0,
        });
        // 4*1 + 8*2 + 8*1 + 4*1 = 4 + 16 + 8 + 4 = 32
        assert_eq!(bars, 32);
    }

    #[test]
    fn reset() {
        let mut ctrl = ArrangementController::new(test_entries());
        ctrl.check_advance();
        ctrl.check_advance();
        ctrl.reset();
        assert_eq!(ctrl.current_section(), Some("intro"));
    }

    #[test]
    fn jump_to() {
        let mut ctrl = ArrangementController::new(test_entries());
        assert!(ctrl.jump_to("chorus"));
        assert_eq!(ctrl.current_section(), Some("chorus"));
    }

    #[test]
    fn jump_to_nonexistent() {
        let mut ctrl = ArrangementController::new(test_entries());
        assert!(!ctrl.jump_to("bridge"));
    }

    #[test]
    fn status_string() {
        let ctrl = ArrangementController::new(test_entries());
        assert_eq!(ctrl.status_string(), "[1/4] intro (rep 1/1)");
    }

    #[test]
    fn inactive_returns_none() {
        let mut ctrl = ArrangementController::new(test_entries());
        ctrl.set_active(false);
        assert!(ctrl.current_section().is_none());
        assert!(ctrl.check_advance().is_none());
    }
}
