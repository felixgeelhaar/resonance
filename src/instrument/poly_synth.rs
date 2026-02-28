//! Poly synth (pad) — two detuned saw oscillators with slow attack and long release.

use crate::event::{Event, NoteOrSample, RenderContext};

use super::envelope::AdsrEnvelope;
use super::oscillator::{midi_to_freq, oscillator, Waveform};
use super::Instrument;

/// Polyphonic pad synth with two detuned saw oscillators.
pub struct PolySynth {
    envelope: AdsrEnvelope,
    detune_cents: f64,
}

impl PolySynth {
    pub fn new() -> Self {
        Self {
            envelope: AdsrEnvelope {
                attack: 0.15,
                decay: 0.2,
                sustain: 0.6,
                release: 0.4,
            },
            detune_cents: 12.0,
        }
    }
}

impl Default for PolySynth {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument for PolySynth {
    fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        let midi_note = match &event.trigger {
            NoteOrSample::Note(n) => *n,
            NoteOrSample::Sample(_) => return Vec::new(),
        };

        if event.velocity <= 0.0 {
            return Vec::new();
        }

        // Read params from event, falling back to defaults
        let detune_cents = event
            .params
            .get(&super::param_defs::detune())
            .map(|v| v as f64)
            .unwrap_or(self.detune_cents);
        let attack_time = event
            .params
            .get(&super::param_defs::attack())
            .map(|v| v as f64)
            .unwrap_or(self.envelope.attack);
        let release_time = event
            .params
            .get(&super::param_defs::release())
            .map(|v| v as f64)
            .unwrap_or(self.envelope.release);

        let envelope = AdsrEnvelope {
            attack: attack_time,
            decay: self.envelope.decay,
            sustain: self.envelope.sustain,
            release: release_time,
        };

        let freq = midi_to_freq(midi_note);
        let detune_ratio = 2.0f64.powf(detune_cents / 1200.0);
        let freq2 = freq * detune_ratio;

        let duration_secs = event.duration.as_beats_f64() * 60.0 / ctx.bpm;
        let total_secs = envelope.total_duration(duration_secs);
        let num_samples = (total_secs * ctx.sample_rate as f64) as usize;

        let mut phase1 = 0.0_f64;
        let mut phase2 = 0.0_f64;

        let mut output = Vec::with_capacity(num_samples * ctx.channels as usize);

        for i in 0..num_samples {
            let t = i as f64 / ctx.sample_rate as f64;
            let env = envelope.amplitude(t, duration_secs);

            let osc1 = oscillator(Waveform::Saw, phase1);
            let osc2 = oscillator(Waveform::Saw, phase2);
            let mixed = (osc1 + osc2) * 0.4; // softer pad sound

            let sample = (mixed * env * event.velocity as f64) as f32;

            for _ in 0..ctx.channels {
                output.push(sample);
            }

            phase1 = (phase1 + freq / ctx.sample_rate as f64).fract();
            phase2 = (phase2 + freq2 / ctx.sample_rate as f64).fract();
        }

        output
    }

    fn name(&self) -> &str {
        "poly"
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
        let synth = PolySynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(2), TrackId(0), 60, 0.7);
        let out = synth.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn ignores_sample_events() {
        let synth = PolySynth::new();
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "pad", 0.8);
        let out = synth.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn slow_attack_quiet_start() {
        let synth = PolySynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(4), TrackId(0), 60, 1.0);
        let out = synth.render(&event, &ctx());
        // First 50 samples should be quiet (attack = 0.15s = 6615 samples)
        let early = &out[..100]; // first 50 stereo frames
        let rms: f32 = (early.iter().map(|s| s * s).sum::<f32>() / early.len() as f32).sqrt();
        assert!(
            rms < 0.1,
            "start should be quiet due to slow attack, rms={rms}"
        );
    }

    #[test]
    fn output_bounded() {
        let synth = PolySynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 1.0);
        let out = synth.render(&event, &ctx());
        for &s in &out {
            assert!(s.abs() <= 1.0, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn instrument_trait_name() {
        let synth = PolySynth::new();
        assert_eq!(Instrument::name(&synth), "poly");
    }

    #[test]
    fn reads_detune_param() {
        let synth = PolySynth::new();
        let mut event = Event::note(Beat::ZERO, Beat::from_beats(2), TrackId(0), 60, 0.7);
        event.params.set(super::super::param_defs::detune(), 50.0);
        let detuned = synth.render(&event, &ctx());

        let default_event = Event::note(Beat::ZERO, Beat::from_beats(2), TrackId(0), 60, 0.7);
        let normal = synth.render(&default_event, &ctx());

        assert!(!detuned.is_empty());
        assert_ne!(detuned, normal);
    }

    #[test]
    fn reads_attack_param() {
        let synth = PolySynth::new();
        let mut event = Event::note(Beat::ZERO, Beat::from_beats(2), TrackId(0), 60, 0.7);
        // Very short attack
        event.params.set(super::super::param_defs::attack(), 0.001);
        let fast_attack = synth.render(&event, &ctx());

        let default_event = Event::note(Beat::ZERO, Beat::from_beats(2), TrackId(0), 60, 0.7);
        let slow_attack = synth.render(&default_event, &ctx());

        assert!(!fast_attack.is_empty());
        // Fast attack should reach higher amplitude sooner
        let early_fast: f32 = fast_attack[..100].iter().map(|s| s.abs()).sum::<f32>() / 100.0;
        let early_slow: f32 = slow_attack[..100].iter().map(|s| s.abs()).sum::<f32>() / 100.0;
        assert!(
            early_fast > early_slow,
            "fast attack should be louder early: {early_fast} vs {early_slow}"
        );
    }

    #[test]
    fn reads_release_param() {
        let synth = PolySynth::new();
        let mut event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.7);
        // Very long release
        event.params.set(super::super::param_defs::release(), 2.0);
        let long_release = synth.render(&event, &ctx());

        let default_event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.7);
        let normal = synth.render(&default_event, &ctx());

        // Longer release means more samples
        assert!(long_release.len() > normal.len());
    }

    #[test]
    fn default_fallback_when_no_params() {
        let synth = PolySynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(2), TrackId(0), 60, 0.7);
        let out = synth.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.01));
    }
}
