//! Intent console — displays recent intents and their status.

/// A log entry for the intent console.
#[derive(Debug, Clone)]
pub struct IntentLogEntry {
    pub message: String,
    pub timestamp_beats: f64,
}

/// Intent console state — a scrollable log of recent intents.
#[derive(Debug, Clone, Default)]
pub struct IntentConsole {
    entries: Vec<IntentLogEntry>,
    max_entries: usize,
}

impl IntentConsole {
    /// Create a new console with a maximum entry count.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Log an intent message.
    pub fn log(&mut self, message: impl Into<String>, timestamp_beats: f64) {
        self.entries.push(IntentLogEntry {
            message: message.into(),
            timestamp_beats,
        });
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    /// Get all entries (newest last).
    pub fn entries(&self) -> &[IntentLogEntry] {
        &self.entries
    }

    /// Clear the console.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the console is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_entries() {
        let mut console = IntentConsole::new(10);
        console.log("set filter = 0.5", 4.0);
        console.log("jump to verse", 8.0);
        assert_eq!(console.len(), 2);
        assert_eq!(console.entries()[0].message, "set filter = 0.5");
    }

    #[test]
    fn max_entries_evicts_oldest() {
        let mut console = IntentConsole::new(2);
        console.log("a", 0.0);
        console.log("b", 1.0);
        console.log("c", 2.0);
        assert_eq!(console.len(), 2);
        assert_eq!(console.entries()[0].message, "b");
        assert_eq!(console.entries()[1].message, "c");
    }

    #[test]
    fn clear_entries() {
        let mut console = IntentConsole::new(10);
        console.log("test", 0.0);
        console.clear();
        assert!(console.is_empty());
    }
}
