//! Bass synthesizer — mono, detuned saw with one-pole low-pass filter.

use crate::event::{Event, NoteOrSample, RenderContext};

use super::envelope::AdsrEnvelope;
use super::oscillator::{midi_to_freq, oscillator, Waveform};
use super::Instrument;

/// Mono bass synth with detuned saw oscillators and a one-pole low-pass filter.
pub struct BassSynth {
    envelope: AdsrEnvelope,
    detune_cents: f64,
    filter_cutoff: f64,
}

impl BassSynth {
    pub fn new() -> Self {
        Self {
            envelope: AdsrEnvelope {
                attack: 0.005,
                decay: 0.1,
                sustain: 0.8,
                release: 0.05,
            },
            detune_cents: 7.0,
            filter_cutoff: 800.0,
        }
    }
}

impl Default for BassSynth {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument for BassSynth {
    fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        let midi_note = match &event.trigger {
            NoteOrSample::Note(n) => *n,
            NoteOrSample::Sample(_) => return Vec::new(),
        };

        if event.velocity <= 0.0 {
            return Vec::new();
        }

        let freq = midi_to_freq(midi_note);
        let detune_ratio = 2.0f64.powf(self.detune_cents / 1200.0);
        let freq2 = freq * detune_ratio;

        let duration_secs = event.duration.as_beats_f64() * 60.0 / ctx.bpm;
        let total_secs = self.envelope.total_duration(duration_secs);
        let num_samples = (total_secs * ctx.sample_rate as f64) as usize;

        let mut phase1 = 0.0_f64;
        let mut phase2 = 0.0_f64;
        let mut filter_state = 0.0_f64;

        // One-pole LP coefficient
        let rc = 1.0 / (2.0 * std::f64::consts::PI * self.filter_cutoff);
        let dt = 1.0 / ctx.sample_rate as f64;
        let alpha = dt / (rc + dt);

        let mut output = Vec::with_capacity(num_samples * ctx.channels as usize);

        for i in 0..num_samples {
            let t = i as f64 / ctx.sample_rate as f64;
            let env = self.envelope.amplitude(t, duration_secs);

            let osc1 = oscillator(Waveform::Saw, phase1);
            let osc2 = oscillator(Waveform::Saw, phase2);
            let mixed = (osc1 + osc2) * 0.5;

            // One-pole low-pass
            filter_state += alpha * (mixed - filter_state);

            let sample = (filter_state * env * event.velocity as f64) as f32;

            for _ in 0..ctx.channels {
                output.push(sample);
            }

            phase1 = (phase1 + freq / ctx.sample_rate as f64).fract();
            phase2 = (phase2 + freq2 / ctx.sample_rate as f64).fract();
        }

        output
    }

    fn name(&self) -> &str {
        "bass"
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
        let synth = BassSynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 36, 0.8);
        let out = synth.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn ignores_sample_events() {
        let synth = BassSynth::new();
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = synth.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn zero_velocity_silent() {
        let synth = BassSynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 36, 0.0);
        let out = synth.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn output_bounded() {
        let synth = BassSynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 36, 1.0);
        let out = synth.render(&event, &ctx());
        for &s in &out {
            assert!(s.abs() <= 1.0, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn stereo_output() {
        let synth = BassSynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 36, 0.8);
        let out = synth.render(&event, &ctx());
        assert_eq!(out.len() % 2, 0, "should be stereo (even sample count)");
        // L and R channels should be identical (mono synth)
        for chunk in out.chunks(2) {
            assert!((chunk[0] - chunk[1]).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn instrument_trait_name() {
        let synth = BassSynth::new();
        assert_eq!(Instrument::name(&synth), "bass");
    }
}
