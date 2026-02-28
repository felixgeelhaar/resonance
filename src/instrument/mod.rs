//! Instruments — sample-based drum kit, synthetic generators, and sample management.

pub mod bass_synth;
pub mod drum_kit;
pub mod envelope;
pub mod noise_gen;
pub mod oscillator;
pub mod param_defs;
pub mod pluck_synth;
pub mod poly_synth;
pub mod router;
pub mod sample;
pub mod synth;

pub use bass_synth::BassSynth;
pub use drum_kit::DrumKit;
pub use noise_gen::NoiseGen;
pub use pluck_synth::PluckSynth;
pub use poly_synth::PolySynth;
pub use router::InstrumentRouter;
pub use sample::{SampleData, SampleError};
pub use synth::build_default_kit;

use crate::event::{Event, RenderContext};
use std::collections::HashMap;

/// Common interface for all instruments in Resonance.
///
/// Each instrument takes an event and render context, and produces
/// interleaved stereo sample data.
pub trait Instrument: Send {
    /// Render a single event into interleaved samples.
    fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32>;

    /// Human-readable name for this instrument.
    fn name(&self) -> &str;
}

/// A named collection of audio samples.
#[derive(Clone)]
pub struct SampleBank {
    samples: HashMap<String, SampleData>,
}

impl SampleBank {
    /// Create an empty sample bank.
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
        }
    }

    /// Insert a named sample.
    pub fn insert(&mut self, name: impl Into<String>, data: SampleData) {
        self.samples.insert(name.into(), data);
    }

    /// Look up a sample by name.
    pub fn get(&self, name: &str) -> Option<&SampleData> {
        self.samples.get(name)
    }

    /// Number of samples in the bank.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the bank is empty.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

impl Default for SampleBank {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bank() {
        let bank = SampleBank::new();
        assert_eq!(bank.len(), 0);
        assert!(bank.is_empty());
        assert!(bank.get("kick").is_none());
    }

    #[test]
    fn insert_and_get() {
        let mut bank = SampleBank::new();
        bank.insert("kick", SampleData::from_mono(vec![0.5, 0.3], 44100));
        assert_eq!(bank.len(), 1);
        assert!(!bank.is_empty());

        let kick = bank.get("kick").unwrap();
        assert_eq!(kick.len(), 2);
        assert!((kick.samples()[0] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn multiple_samples() {
        let mut bank = SampleBank::new();
        bank.insert("kick", SampleData::from_mono(vec![1.0], 44100));
        bank.insert("snare", SampleData::from_mono(vec![0.5], 44100));
        bank.insert("hat", SampleData::from_mono(vec![0.3], 44100));
        assert_eq!(bank.len(), 3);
    }

    #[test]
    fn overwrite_existing() {
        let mut bank = SampleBank::new();
        bank.insert("kick", SampleData::from_mono(vec![1.0], 44100));
        bank.insert("kick", SampleData::from_mono(vec![0.5], 44100));
        assert_eq!(bank.len(), 1);
        assert!((bank.get("kick").unwrap().samples()[0] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn get_nonexistent() {
        let bank = SampleBank::new();
        assert!(bank.get("nonexistent").is_none());
    }

    #[test]
    fn default_is_empty() {
        let bank = SampleBank::default();
        assert!(bank.is_empty());
    }
}
