//! Pluck synth — Karplus-Strong algorithm (noise burst + feedback delay).

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::event::{Event, NoteOrSample, RenderContext};

use super::oscillator::midi_to_freq;
use super::Instrument;

/// Pluck synth using the Karplus-Strong algorithm.
///
/// Generates a noise burst that is fed through a feedback delay line
/// with averaging, producing a naturally decaying plucked-string sound.
pub struct PluckSynth {
    seed: u64,
}

impl PluckSynth {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }
}

impl Instrument for PluckSynth {
    fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        let midi_note = match &event.trigger {
            NoteOrSample::Note(n) => *n,
            NoteOrSample::Sample(_) => return Vec::new(),
        };

        if event.velocity <= 0.0 {
            return Vec::new();
        }

        let freq = midi_to_freq(midi_note);
        let delay_len = (ctx.sample_rate as f64 / freq).round() as usize;
        if delay_len == 0 {
            return Vec::new();
        }

        let duration_secs = event.duration.as_beats_f64() * 60.0 / ctx.bpm;
        // Pluck has natural decay, but we cap at note duration + extra tail
        let total_secs = duration_secs + 0.2;
        let num_samples = (total_secs * ctx.sample_rate as f64) as usize;

        // Seed derived from base seed + note for uniqueness
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed.wrapping_add(midi_note as u64));

        // Initialize delay buffer with noise burst
        let mut delay_buf: Vec<f64> = (0..delay_len).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let mut delay_idx = 0;

        let velocity = event.velocity as f64;
        let mut output = Vec::with_capacity(num_samples * ctx.channels as usize);

        for i in 0..num_samples {
            let sample = delay_buf[delay_idx];

            // Karplus-Strong: average current and next sample, feed back
            let next_idx = (delay_idx + 1) % delay_len;
            let avg = (delay_buf[delay_idx] + delay_buf[next_idx]) * 0.5;

            // Damping factor (slight loss per cycle)
            let damping = 0.996;
            delay_buf[delay_idx] = avg * damping;
            delay_idx = next_idx;

            // Gentle fade out at the end to avoid clicks
            let fade = if i > num_samples - 200 {
                (num_samples - i) as f64 / 200.0
            } else {
                1.0
            };

            let s = (sample * velocity * fade) as f32;
            for _ in 0..ctx.channels {
                output.push(s);
            }
        }

        output
    }

    fn name(&self) -> &str {
        "pluck"
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
        let synth = PluckSynth::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let out = synth.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn ignores_sample_events() {
        let synth = PluckSynth::new(42);
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "pluck", 0.8);
        let out = synth.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn deterministic() {
        let synth = PluckSynth::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let a = synth.render(&event, &ctx());
        let b = synth.render(&event, &ctx());
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_differ() {
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let a = PluckSynth::new(1).render(&event, &ctx());
        let b = PluckSynth::new(2).render(&event, &ctx());
        assert_ne!(a, b);
    }

    #[test]
    fn output_bounded() {
        let synth = PluckSynth::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 1.0);
        let out = synth.render(&event, &ctx());
        for &s in &out {
            assert!(s.abs() <= 1.5, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn natural_decay() {
        let synth = PluckSynth::new(42);
        let event = Event::note(Beat::ZERO, Beat::from_beats(2), TrackId(0), 60, 1.0);
        let out = synth.render(&event, &ctx());
        // Compare RMS of first and last quarter
        let q = out.len() / 4;
        let first: f32 = (out[..q].iter().map(|s| s * s).sum::<f32>() / q as f32).sqrt();
        let last: f32 =
            (out[3 * q..].iter().map(|s| s * s).sum::<f32>() / (out.len() - 3 * q) as f32).sqrt();
        assert!(
            first > last * 1.5,
            "pluck should decay: first_rms={first}, last_rms={last}"
        );
    }

    #[test]
    fn instrument_trait_name() {
        let synth = PluckSynth::new(42);
        assert_eq!(Instrument::name(&synth), "pluck");
    }
}
