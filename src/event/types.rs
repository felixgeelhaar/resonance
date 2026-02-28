//! Event data model — the fundamental unit of musical information in Resonance.
//!
//! An [`Event`] represents a single note or sample trigger at a specific point
//! in musical time, with velocity, duration, and extensible parameters.

use std::collections::HashMap;

use super::beat::Beat;

/// Identifies a track in the event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackId(pub u32);

/// Identifies a parameter target for macro mappings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParamId(pub String);

/// What the event triggers: a pitched note or a named sample.
#[derive(Debug, Clone, PartialEq)]
pub enum NoteOrSample {
    /// MIDI note number (0–127).
    Note(u8),
    /// Named sample reference (e.g. "kick", "snare").
    Sample(String),
}

/// Extensible parameter bag for events.
///
/// Stores macro-controlled parameters as `ParamId → f32` mappings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Params {
    pub values: HashMap<ParamId, f32>,
}

impl Params {
    /// Get a parameter value by id.
    pub fn get(&self, id: &ParamId) -> Option<f32> {
        self.values.get(id).copied()
    }

    /// Set a parameter value.
    pub fn set(&mut self, id: ParamId, value: f32) {
        self.values.insert(id, value);
    }
}

/// A single event on the timeline.
#[derive(Debug, Clone)]
pub struct Event {
    /// When this event fires, in musical time.
    pub time: Beat,
    /// Duration of the event (used for note-off, sustain).
    pub duration: Beat,
    /// Which track owns this event.
    pub track_id: TrackId,
    /// What to play: a note or a sample.
    pub trigger: NoteOrSample,
    /// Velocity in the range 0.0–1.0.
    pub velocity: f32,
    /// Additional parameters (empty for Phase 0).
    pub params: Params,
}

impl Event {
    /// Create a sample-trigger event.
    pub fn sample(
        time: Beat,
        duration: Beat,
        track_id: TrackId,
        name: &str,
        velocity: f32,
    ) -> Self {
        Self {
            time,
            duration,
            track_id,
            trigger: NoteOrSample::Sample(name.to_string()),
            velocity,
            params: Params::default(),
        }
    }

    /// Create a note event.
    pub fn note(time: Beat, duration: Beat, track_id: TrackId, note: u8, velocity: f32) -> Self {
        Self {
            time,
            duration,
            track_id,
            trigger: NoteOrSample::Note(note),
            velocity,
            params: Params::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_constructor() {
        let e = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        assert_eq!(e.time, Beat::ZERO);
        assert_eq!(e.track_id, TrackId(0));
        assert_eq!(e.trigger, NoteOrSample::Sample("kick".to_string()));
        assert!((e.velocity - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn note_constructor() {
        let e = Event::note(
            Beat::from_beats(2),
            Beat::from_beats(1),
            TrackId(1),
            60,
            0.5,
        );
        assert_eq!(e.trigger, NoteOrSample::Note(60));
        assert_eq!(e.track_id, TrackId(1));
    }

    #[test]
    fn track_id_equality() {
        assert_eq!(TrackId(0), TrackId(0));
        assert_ne!(TrackId(0), TrackId(1));
    }

    #[test]
    fn note_or_sample_variants() {
        let note = NoteOrSample::Note(127);
        let sample = NoteOrSample::Sample("snare".into());
        assert_ne!(note, sample);

        if let NoteOrSample::Note(n) = note {
            assert_eq!(n, 127);
        }
        if let NoteOrSample::Sample(s) = sample {
            assert_eq!(s, "snare");
        }
    }

    #[test]
    fn params_default_is_empty() {
        let p = Params::default();
        assert!(p.values.is_empty());
    }

    #[test]
    fn params_set_and_get() {
        let mut p = Params::default();
        let id = ParamId("filter_cutoff".to_string());
        p.set(id.clone(), 0.75);
        assert!((p.get(&id).unwrap() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn params_get_missing_returns_none() {
        let p = Params::default();
        assert!(p.get(&ParamId("missing".to_string())).is_none());
    }

    #[test]
    fn params_overwrite() {
        let mut p = Params::default();
        let id = ParamId("volume".to_string());
        p.set(id.clone(), 0.5);
        p.set(id.clone(), 0.9);
        assert!((p.get(&id).unwrap() - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn param_id_equality() {
        let a = ParamId("cutoff".to_string());
        let b = ParamId("cutoff".to_string());
        let c = ParamId("resonance".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn param_id_hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ParamId("a".to_string()));
        set.insert(ParamId("b".to_string()));
        set.insert(ParamId("a".to_string()));
        assert_eq!(set.len(), 2);
    }
}
