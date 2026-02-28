//! Crash log — circular buffer of recent error messages for debugging.

use std::collections::VecDeque;
use std::time::SystemTime;

/// A timestamped error entry.
#[derive(Debug, Clone)]
pub struct CrashEntry {
    pub timestamp: SystemTime,
    pub message: String,
}

/// Circular buffer of recent errors.
#[derive(Debug, Clone)]
pub struct CrashLog {
    entries: VecDeque<CrashEntry>,
    capacity: usize,
}

impl CrashLog {
    /// Create a new crash log with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a new error message, evicting the oldest if at capacity.
    pub fn push(&mut self, message: String) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(CrashEntry {
            timestamp: SystemTime::now(),
            message,
        });
    }

    /// Get the N most recent entries (newest last).
    pub fn recent(&self, n: usize) -> Vec<&CrashEntry> {
        let len = self.entries.len();
        let skip = len.saturating_sub(n);
        self.entries.iter().skip(skip).collect()
    }

    /// Get all entries as a slice (works when VecDeque is contiguous).
    pub fn entries(&self) -> impl Iterator<Item = &CrashEntry> {
        self.entries.iter()
    }

    /// Whether the log has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Default for CrashLog {
    fn default() -> Self {
        Self::new(50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_crash_log_is_empty() {
        let log = CrashLog::new(10);
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn push_adds_entries() {
        let mut log = CrashLog::new(10);
        log.push("error 1".to_string());
        log.push("error 2".to_string());
        assert_eq!(log.len(), 2);
        assert!(!log.is_empty());
    }

    #[test]
    fn capacity_overflow_evicts_oldest() {
        let mut log = CrashLog::new(3);
        log.push("a".to_string());
        log.push("b".to_string());
        log.push("c".to_string());
        log.push("d".to_string()); // should evict "a"
        assert_eq!(log.len(), 3);

        let messages: Vec<&str> = log.entries().map(|e| e.message.as_str()).collect();
        assert_eq!(messages, vec!["b", "c", "d"]);
    }

    #[test]
    fn recent_returns_newest() {
        let mut log = CrashLog::new(10);
        log.push("a".to_string());
        log.push("b".to_string());
        log.push("c".to_string());

        let recent = log.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].message, "b");
        assert_eq!(recent[1].message, "c");
    }

    #[test]
    fn recent_more_than_available() {
        let mut log = CrashLog::new(10);
        log.push("a".to_string());
        let recent = log.recent(5);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].message, "a");
    }

    #[test]
    fn entries_are_timestamped() {
        let mut log = CrashLog::new(10);
        log.push("test".to_string());
        let entry = log.entries().next().unwrap();
        // Timestamp should be recent (within last second)
        let elapsed = entry.timestamp.elapsed().unwrap();
        assert!(elapsed.as_secs() < 1);
    }

    #[test]
    fn default_capacity_is_50() {
        let log = CrashLog::default();
        // Fill beyond 50 to verify capacity
        let mut log = log;
        for i in 0..60 {
            log.push(format!("err {i}"));
        }
        assert_eq!(log.len(), 50);
    }

    #[test]
    fn entries_iterator_preserves_order() {
        let mut log = CrashLog::new(10);
        log.push("first".to_string());
        log.push("second".to_string());
        log.push("third".to_string());

        let messages: Vec<&str> = log.entries().map(|e| e.message.as_str()).collect();
        assert_eq!(messages, vec!["first", "second", "third"]);
    }
}
