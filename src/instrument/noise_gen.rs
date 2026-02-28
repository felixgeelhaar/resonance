//! Noise generator — white noise with filter sweep, seeded RNG.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::event::{Event, NoteOrSample, RenderContext};

use super::envelope::AdsrEnvelope;
use super::Instrument;

/// Noise generator with amplitude envelope and one-pole filter.
///
/// Can be triggered by both `Note` and `Sample("noise")` events.
/// When triggered by a Note, the filter cutoff tracks the note frequency.
pub struct NoiseGen {
    seed: u64,
    envelope: AdsrEnvelope,
}

impl NoiseGen {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            envelope: AdsrEnvelope {
                attack: 0.01,
                decay: 0.3,
                sustain: 0.3,
                release: 0.2,
            },
        }
    }
}

impl Instrument for NoiseGen {
    fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        if event.velocity <= 0.0 {
            return Vec::new();
        }

        // Accept both Note and Sample("noise") triggers
        let base_cutoff = match &event.trigger {
            NoteOrSample::Note(n) => {
                // Track note frequency for filter
                super::oscillator::midi_to_freq(*n)
            }
            NoteOrSample::Sample(name) if name == "noise" => 2000.0,
            NoteOrSample::Sample(_) => return Vec::new(),
        };

        // Read cutoff param, falling back to trigger-derived cutoff
        let cutoff = event
            .params
            .get(&super::param_defs::cutoff())
            .map(|v| v as f64)
            .unwrap_or(base_cutoff);

        let duration_secs = event.duration.as_beats_f64() * 60.0 / ctx.bpm;
        let total_secs = self.envelope.total_duration(duration_secs);
        let num_samples = (total_secs * ctx.sample_rate as f64) as usize;

        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        let mut filter_state = 0.0_f64;

        let rc = 1.0 / (2.0 * std::f64::consts::PI * cutoff);
        let dt = 1.0 / ctx.sample_rate as f64;
        let alpha = dt / (rc + dt);

        let velocity = event.velocity as f64;
        let mut output = Vec::with_capacity(num_samples * ctx.channels as usize);

        for i in 0..num_samples {
            let t = i as f64 / ctx.sample_rate as f64;
            let env = self.envelope.amplitude(t, duration_secs);

            let noise: f64 = rng.gen_range(-1.0..1.0);
            filter_state += alpha * (noise - filter_state);

            let sample = (filter_state * env * velocity) as f32;
            for _ in 0..ctx.channels {
                output.push(sample);
            }
        }

        output
    }

    fn name(&self) -> &str {
        "noise"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Beat, TrackId};

    fn ctx() -> RenderContext {
        RenderContext {
            sample_rate: 44100,
            channels: 2,
            bpm: 120.0,
        }
    }

    #[test]
    fn renders_note_event() {
        let gen = NoiseGen::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let out = gen.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.001));
    }

    #[test]
    fn renders_noise_sample_event() {
        let gen = NoiseGen::new(42);
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "noise", 0.8);
        let out = gen.render(&event, &ctx());
        assert!(!out.is_empty());
    }

    #[test]
    fn ignores_non_noise_sample() {
        let gen = NoiseGen::new(42);
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = gen.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn deterministic() {
        let gen = NoiseGen::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let a = gen.render(&event, &ctx());
        let b = gen.render(&event, &ctx());
        assert_eq!(a, b);
    }

    #[test]
    fn output_bounded() {
        let gen = NoiseGen::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 1.0);
        let out = gen.render(&event, &ctx());
        for &s in &out {
            assert!(s.abs() <= 1.0, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn zero_velocity_silent() {
        let gen = NoiseGen::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.0);
        let out = gen.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn instrument_trait_name() {
        let gen = NoiseGen::new(42);
        assert_eq!(Instrument::name(&gen), "noise");
    }

    #[test]
    fn reads_cutoff_param() {
        let gen = NoiseGen::new(42);
        let mut event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        event.params.set(super::super::param_defs::cutoff(), 200.0);
        let filtered = gen.render(&event, &ctx());

        let default_event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let unfiltered = gen.render(&default_event, &ctx());

        assert!(!filtered.is_empty());
        assert_ne!(filtered, unfiltered);
    }

    #[test]
    fn cutoff_param_overrides_note_tracking() {
        let gen = NoiseGen::new(42);
        // Note C2 (MIDI 36) would give ~65 Hz cutoff, but param overrides to 5000 Hz
        let mut event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 36, 0.8);
        event.params.set(super::super::param_defs::cutoff(), 5000.0);
        let high_cut = gen.render(&event, &ctx());

        let default_event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 36, 0.8);
        let low_cut = gen.render(&default_event, &ctx());

        // Higher cutoff should have more high-frequency content (higher RMS)
        let rms_high: f32 =
            (high_cut.iter().map(|s| s * s).sum::<f32>() / high_cut.len() as f32).sqrt();
        let rms_low: f32 =
            (low_cut.iter().map(|s| s * s).sum::<f32>() / low_cut.len() as f32).sqrt();
        assert!(
            rms_high > rms_low,
            "higher cutoff should pass more energy: {rms_high} vs {rms_low}"
        );
    }

    #[test]
    fn default_fallback_when_no_params() {
        let gen = NoiseGen::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let out = gen.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.001));
    }
}
