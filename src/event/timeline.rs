//! Sorted event storage with cursor-based consumption.
//!
//! Events are stored sorted by time. A cursor tracks the current read position
//! so that `drain_range` only scans unconsumed events. Batch insertion defers
//! sorting until the next read operation for efficiency.

use super::beat::Beat;
use super::types::Event;

/// A sorted timeline of events with a read cursor.
pub struct Timeline {
    events: Vec<Event>,
    cursor: usize,
    dirty: bool,
}

impl Timeline {
    /// Create an empty timeline.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            cursor: 0,
            dirty: false,
        }
    }

    /// Insert a single event, maintaining sorted order.
    pub fn insert(&mut self, event: Event) {
        self.ensure_sorted();
        let pos = self.events[self.cursor..]
            .binary_search_by(|e| e.time.cmp(&event.time))
            .unwrap_or_else(|i| i)
            + self.cursor;
        self.events.insert(pos, event);
    }

    /// Insert a batch of events. Defers sorting until the next read.
    pub fn insert_batch(&mut self, events: impl IntoIterator<Item = Event>) {
        self.events.extend(events);
        self.dirty = true;
    }

    /// Drain all events in `[from, to)` and advance the cursor past them.
    ///
    /// Returns the drained events in time order.
    pub fn drain_range(&mut self, from: Beat, to: Beat) -> Vec<Event> {
        self.ensure_sorted();

        let mut result = Vec::new();
        while self.cursor < self.events.len() {
            let event_time = self.events[self.cursor].time;
            if event_time >= to {
                break;
            }
            if event_time >= from {
                result.push(self.events[self.cursor].clone());
            }
            self.cursor += 1;
        }
        result
    }

    /// Peek at the next unconsumed event without advancing the cursor.
    pub fn peek_next(&mut self) -> Option<&Event> {
        self.ensure_sorted();
        self.events.get(self.cursor)
    }

    /// Reset the cursor to the beginning.
    pub fn reset_cursor(&mut self) {
        self.cursor = 0;
    }

    /// Total number of events in the timeline.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the timeline is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Number of unconsumed events remaining after the cursor.
    pub fn remaining(&self) -> usize {
        self.events.len().saturating_sub(self.cursor)
    }

    /// Remove all events and reset the cursor.
    pub fn clear(&mut self) {
        self.events.clear();
        self.cursor = 0;
        self.dirty = false;
    }

    /// Sort events if a batch insert marked the timeline as dirty.
    /// Uses a stable sort to preserve insertion order for simultaneous events.
    fn ensure_sorted(&mut self) {
        if self.dirty {
            self.events[self.cursor..].sort_by(|a, b| a.time.cmp(&b.time));
            self.dirty = false;
        }
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::types::{NoteOrSample, TrackId};

    fn sample_event(beat: u32, name: &str) -> Event {
        Event::sample(
            Beat::from_beats(beat),
            Beat::from_beats(1),
            TrackId(0),
            name,
            0.8,
        )
    }

    #[test]
    fn empty_timeline() {
        let mut tl = Timeline::new();
        assert_eq!(tl.len(), 0);
        assert!(tl.is_empty());
        assert_eq!(tl.remaining(), 0);
        assert!(tl.peek_next().is_none());
    }

    #[test]
    fn single_insert() {
        let mut tl = Timeline::new();
        tl.insert(sample_event(0, "kick"));
        assert_eq!(tl.len(), 1);
        assert_eq!(tl.remaining(), 1);
    }

    #[test]
    fn sorted_order_maintained() {
        let mut tl = Timeline::new();
        tl.insert(sample_event(2, "snare"));
        tl.insert(sample_event(0, "kick"));
        tl.insert(sample_event(1, "hat"));

        let events = tl.drain_range(Beat::ZERO, Beat::from_beats(10));
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].trigger, NoteOrSample::Sample("kick".into()));
        assert_eq!(events[1].trigger, NoteOrSample::Sample("hat".into()));
        assert_eq!(events[2].trigger, NoteOrSample::Sample("snare".into()));
    }

    #[test]
    fn batch_insert_sorts_on_read() {
        let mut tl = Timeline::new();
        tl.insert_batch(vec![
            sample_event(3, "clap"),
            sample_event(0, "kick"),
            sample_event(1, "hat"),
        ]);
        assert_eq!(tl.len(), 3);

        let events = tl.drain_range(Beat::ZERO, Beat::from_beats(10));
        assert_eq!(events[0].trigger, NoteOrSample::Sample("kick".into()));
        assert_eq!(events[1].trigger, NoteOrSample::Sample("hat".into()));
        assert_eq!(events[2].trigger, NoteOrSample::Sample("clap".into()));
    }

    #[test]
    fn drain_range_basic() {
        let mut tl = Timeline::new();
        tl.insert_batch(vec![
            sample_event(0, "kick"),
            sample_event(1, "hat"),
            sample_event(2, "snare"),
            sample_event(3, "clap"),
        ]);

        let events = tl.drain_range(Beat::from_beats(1), Beat::from_beats(3));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].trigger, NoteOrSample::Sample("hat".into()));
        assert_eq!(events[1].trigger, NoteOrSample::Sample("snare".into()));
    }

    #[test]
    fn drain_range_advances_cursor() {
        let mut tl = Timeline::new();
        tl.insert_batch(vec![
            sample_event(0, "kick"),
            sample_event(1, "hat"),
            sample_event(2, "snare"),
        ]);

        tl.drain_range(Beat::ZERO, Beat::from_beats(2));
        assert_eq!(tl.remaining(), 1);

        let events = tl.drain_range(Beat::from_beats(2), Beat::from_beats(4));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].trigger, NoteOrSample::Sample("snare".into()));
    }

    #[test]
    fn drain_range_empty_window() {
        let mut tl = Timeline::new();
        tl.insert(sample_event(5, "kick"));

        let events = tl.drain_range(Beat::ZERO, Beat::from_beats(3));
        assert!(events.is_empty());
        assert_eq!(tl.remaining(), 1);
    }

    #[test]
    fn boundary_from_inclusive_to_exclusive() {
        let mut tl = Timeline::new();
        tl.insert_batch(vec![sample_event(1, "kick"), sample_event(2, "snare")]);

        // from=1 (inclusive), to=2 (exclusive)
        let events = tl.drain_range(Beat::from_beats(1), Beat::from_beats(2));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].trigger, NoteOrSample::Sample("kick".into()));
    }

    #[test]
    fn reset_cursor() {
        let mut tl = Timeline::new();
        tl.insert_batch(vec![sample_event(0, "kick"), sample_event(1, "hat")]);

        tl.drain_range(Beat::ZERO, Beat::from_beats(10));
        assert_eq!(tl.remaining(), 0);

        tl.reset_cursor();
        assert_eq!(tl.remaining(), 2);
    }

    #[test]
    fn peek_next_does_not_advance() {
        let mut tl = Timeline::new();
        tl.insert(sample_event(0, "kick"));

        let first = tl.peek_next().unwrap().time;
        let second = tl.peek_next().unwrap().time;
        assert_eq!(first, second);
        assert_eq!(tl.remaining(), 1);
    }

    #[test]
    fn clear_removes_everything() {
        let mut tl = Timeline::new();
        tl.insert_batch(vec![sample_event(0, "kick"), sample_event(1, "hat")]);
        tl.drain_range(Beat::ZERO, Beat::from_beats(1));

        tl.clear();
        assert_eq!(tl.len(), 0);
        assert_eq!(tl.remaining(), 0);
        assert!(tl.peek_next().is_none());
    }
}
