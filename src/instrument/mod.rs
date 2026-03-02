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
pub use sample::{load_kit_from_directory, SampleData, SampleError};
pub use synth::build_default_kit;

use crate::event::{Event, RenderContext};
use std::collections::HashMap;
use std::path::Path;

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
#[derive(Clone, Debug)]
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

/// Resolve a kit name to a [`SampleBank`].
///
/// - `"default"` → synthetic default kit via [`build_default_kit`]
/// - Name containing `/` or `.` → treat as directory path, load WAV files
/// - Otherwise → look in `~/.resonance/kits/<name>/`, fallback to default
pub fn resolve_kit(name: &str, sample_rate: u32, seed: u64) -> Result<SampleBank, SampleError> {
    if name == "default" {
        return Ok(build_default_kit(sample_rate, seed));
    }

    // Path-like names: load from directory
    if name.contains('/') || name.contains('.') {
        return load_kit_from_directory(Path::new(name), sample_rate);
    }

    // Named kit: check ~/.resonance/kits/<name>/
    if let Some(home) = dirs::home_dir() {
        let kit_dir = home.join(".resonance").join("kits").join(name);
        if kit_dir.is_dir() {
            return load_kit_from_directory(&kit_dir, sample_rate);
        }
    }

    // Check installed packs for the kit name
    if let Some(kit_path) = crate::content::packs::resolve_kit_from_packs(name) {
        return load_kit_from_directory(&kit_path, sample_rate);
    }

    // Fallback to default
    Ok(build_default_kit(sample_rate, seed))
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

    // --- resolve_kit tests ---

    #[test]
    fn resolve_kit_default() {
        let bank = resolve_kit("default", 44100, 42).unwrap();
        assert!(!bank.is_empty());
    }

    #[test]
    fn resolve_kit_path_to_directory() {
        let dir = tempfile::tempdir().unwrap();
        // Write a WAV file
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let wav_path = dir.path().join("kick.wav");
        let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
        writer.write_sample(0.5f32).unwrap();
        writer.finalize().unwrap();

        let bank = resolve_kit(dir.path().to_str().unwrap(), 44100, 42).unwrap();
        assert!(bank.get("kick").is_some());
    }

    #[test]
    fn resolve_kit_nonexistent_fallback() {
        // Non-path name that doesn't exist in ~/.resonance/kits/ → fallback to default
        let bank = resolve_kit("nonexistent_kit_name", 44100, 42).unwrap();
        assert!(!bank.is_empty()); // Should get default kit
    }

    #[test]
    fn resolve_kit_path_detection() {
        // Names with / or . are treated as paths
        assert!(resolve_kit("./nonexistent_dir", 44100, 42).is_err());
        assert!(resolve_kit("../nonexistent_dir", 44100, 42).is_err());
    }
}
