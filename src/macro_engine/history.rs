//! Macro history — per-macro undo/redo stacks for value snapshots.

use std::collections::HashMap;

const MAX_HISTORY_DEPTH: usize = 100;

/// Per-macro undo/redo stacks.
#[derive(Debug, Clone, Default)]
pub struct MacroHistory {
    undo_stacks: HashMap<usize, Vec<f64>>,
    redo_stacks: HashMap<usize, Vec<f64>>,
}

impl MacroHistory {
    /// Create a new empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a value snapshot for a macro. Clears the redo stack for that macro.
    pub fn record(&mut self, macro_idx: usize, value: f64) {
        let undo = self.undo_stacks.entry(macro_idx).or_default();
        undo.push(value);
        if undo.len() > MAX_HISTORY_DEPTH {
            undo.remove(0);
        }
        // Clear redo on new recording
        self.redo_stacks.entry(macro_idx).or_default().clear();
    }

    /// Undo the last change for a macro. Returns the previous value if available.
    pub fn undo(&mut self, macro_idx: usize) -> Option<f64> {
        let undo = self.undo_stacks.get_mut(&macro_idx)?;
        let current = undo.pop()?;
        self.redo_stacks.entry(macro_idx).or_default().push(current);
        undo.last().copied()
    }

    /// Redo the last undone change for a macro. Returns the restored value if available.
    pub fn redo(&mut self, macro_idx: usize) -> Option<f64> {
        let redo = self.redo_stacks.get_mut(&macro_idx)?;
        let value = redo.pop()?;
        self.undo_stacks.entry(macro_idx).or_default().push(value);
        Some(value)
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stacks.clear();
        self.redo_stacks.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_history_is_empty() {
        let history = MacroHistory::new();
        assert!(history.undo_stacks.is_empty());
        assert!(history.redo_stacks.is_empty());
    }

    #[test]
    fn record_and_undo() {
        let mut history = MacroHistory::new();
        history.record(0, 0.5);
        history.record(0, 0.7);
        let prev = history.undo(0);
        assert_eq!(prev, Some(0.5));
    }

    #[test]
    fn undo_empty_returns_none() {
        let mut history = MacroHistory::new();
        assert_eq!(history.undo(0), None);
    }

    #[test]
    fn undo_single_entry_returns_none() {
        let mut history = MacroHistory::new();
        history.record(0, 0.5);
        // Undo pops 0.5, stack is now empty, returns None (no previous)
        assert_eq!(history.undo(0), None);
    }

    #[test]
    fn redo_after_undo() {
        let mut history = MacroHistory::new();
        history.record(0, 0.5);
        history.record(0, 0.7);
        history.undo(0); // back to 0.5
        let redone = history.redo(0);
        assert_eq!(redone, Some(0.7));
    }

    #[test]
    fn redo_empty_returns_none() {
        let mut history = MacroHistory::new();
        history.record(0, 0.5);
        assert_eq!(history.redo(0), None);
    }

    #[test]
    fn record_clears_redo() {
        let mut history = MacroHistory::new();
        history.record(0, 0.5);
        history.record(0, 0.7);
        history.undo(0); // back to 0.5, redo has 0.7
        history.record(0, 0.9); // new record clears redo
        assert_eq!(history.redo(0), None);
    }

    #[test]
    fn independent_macro_histories() {
        let mut history = MacroHistory::new();
        history.record(0, 0.5);
        history.record(1, 0.3);
        history.record(0, 0.7);
        history.record(1, 0.6);

        assert_eq!(history.undo(0), Some(0.5));
        assert_eq!(history.undo(1), Some(0.3));
    }

    #[test]
    fn capacity_limit() {
        let mut history = MacroHistory::new();
        for i in 0..110 {
            history.record(0, i as f64 / 110.0);
        }
        // Should have at most MAX_HISTORY_DEPTH entries
        assert!(history.undo_stacks[&0].len() <= MAX_HISTORY_DEPTH);
    }

    #[test]
    fn multiple_undo_redo_cycle() {
        let mut history = MacroHistory::new();
        history.record(0, 0.1);
        history.record(0, 0.3);
        history.record(0, 0.5);

        assert_eq!(history.undo(0), Some(0.3));
        assert_eq!(history.undo(0), Some(0.1));
        assert_eq!(history.redo(0), Some(0.3));
        assert_eq!(history.redo(0), Some(0.5));
    }

    #[test]
    fn clear_removes_all() {
        let mut history = MacroHistory::new();
        history.record(0, 0.5);
        history.record(1, 0.7);
        history.clear();
        assert_eq!(history.undo(0), None);
        assert_eq!(history.undo(1), None);
    }
}
