//! Wavetable Synthesizer — morphable waveform frames with SVF filter.

use std::f64::consts::PI;

use crate::event::{Event, NoteOrSample, RenderContext};

use super::envelope::AdsrEnvelope;
use super::filter::{FilterMode, SvfFilter};
use super::oscillator::midi_to_freq;
use super::Instrument;

/// A wavetable: a set of waveform frames that can be morphed between.
#[derive(Debug, Clone)]
pub struct Wavetable {
    frames: Vec<Vec<f32>>,
    frame_size: usize,
}

impl Wavetable {
    /// Create a wavetable with the given frames. All frames must be the same size.
    pub fn new(frames: Vec<Vec<f32>>) -> Self {
        let frame_size = frames.first().map(|f| f.len()).unwrap_or(256);
        Self { frames, frame_size }
    }

    /// Generate the "basic" built-in wavetable: sine → saw → square morph.
    pub fn basic(frame_size: usize) -> Self {
        let sine: Vec<f32> = (0..frame_size)
            .map(|i| (2.0 * PI * i as f64 / frame_size as f64).sin() as f32)
            .collect();
        let saw: Vec<f32> = (0..frame_size)
            .map(|i| (2.0 * i as f64 / frame_size as f64 - 1.0) as f32)
            .collect();
        let square: Vec<f32> = (0..frame_size)
            .map(|i| if i < frame_size / 2 { 1.0f32 } else { -1.0f32 })
            .collect();
        Self::new(vec![sine, saw, square])
    }

    /// Sample the wavetable at a given phase [0.0, 1.0) and morph position [0.0, 1.0].
    pub fn sample(&self, phase: f64, morph: f64) -> f64 {
        if self.frames.is_empty() {
            return 0.0;
        }

        let morph = morph.clamp(0.0, 1.0);
        let frame_pos = morph * (self.frames.len() - 1) as f64;
        let frame_lo = (frame_pos.floor() as usize).min(self.frames.len() - 1);
        let frame_hi = (frame_lo + 1).min(self.frames.len() - 1);
        let frac = frame_pos - frame_lo as f64;

        let idx = (phase * self.frame_size as f64) as usize % self.frame_size;
        let lo_val = self.frames[frame_lo][idx] as f64;
        let hi_val = self.frames[frame_hi][idx] as f64;

        lo_val + frac * (hi_val - lo_val)
    }
}

/// Wavetable synth with morphable waveforms and SVF filter.
pub struct WavetableSynth {
    wavetable: Wavetable,
    envelope: AdsrEnvelope,
}

impl WavetableSynth {
    pub fn new(name: &str) -> Self {
        let wavetable = match name {
            "basic" => Wavetable::basic(256),
            _ => Wavetable::basic(256),
        };
        Self {
            wavetable,
            envelope: AdsrEnvelope {
                attack: 0.05,
                decay: 0.1,
                sustain: 0.7,
                release: 0.2,
            },
        }
    }
}

impl Instrument for WavetableSynth {
    fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        let midi_note = match &event.trigger {
            NoteOrSample::Note(n) => *n,
            NoteOrSample::Sample(_) => return Vec::new(),
        };

        if event.velocity <= 0.0 {
            return Vec::new();
        }

        let morph = event
            .params
            .get(&super::param_defs::morph())
            .map(|v| v as f64)
            .unwrap_or(0.0);
        let cutoff = event
            .params
            .get(&super::param_defs::cutoff())
            .map(|v| v as f64)
            .unwrap_or(6000.0);
        let resonance = event
            .params
            .get(&super::param_defs::resonance())
            .map(|v| v as f64)
            .unwrap_or(0.707);

        let freq = midi_to_freq(midi_note);
        let duration_secs = event.duration.as_beats_f64() * 60.0 / ctx.bpm;
        let total_secs = self.envelope.total_duration(duration_secs);
        let num_samples = (total_secs * ctx.sample_rate as f64) as usize;

        let mut phase = 0.0_f64;
        let mut filter = SvfFilter::new(
            FilterMode::LowPass,
            cutoff,
            resonance,
            ctx.sample_rate as f64,
        );

        let mut output = Vec::with_capacity(num_samples * ctx.channels as usize);

        for i in 0..num_samples {
            let t = i as f64 / ctx.sample_rate as f64;
            let env = self.envelope.amplitude(t, duration_secs);

            let raw = self.wavetable.sample(phase, morph);
            let filtered = filter.process(raw);

            let sample = (filtered * env * event.velocity as f64) as f32;

            for _ in 0..ctx.channels {
                output.push(sample);
            }

            phase = (phase + freq / ctx.sample_rate as f64).fract();
        }

        output
    }

    fn name(&self) -> &str {
        "wavetable"
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
    fn basic_wavetable_has_three_frames() {
        let wt = Wavetable::basic(256);
        assert_eq!(wt.frames.len(), 3);
        assert_eq!(wt.frame_size, 256);
    }

    #[test]
    fn wavetable_sample_morph_0_is_first_frame() {
        let wt = Wavetable::basic(256);
        let val = wt.sample(0.25, 0.0); // sine at quarter = ~1.0
        assert!((val - 1.0).abs() < 0.05, "expected ~1.0, got {val}");
    }

    #[test]
    fn wavetable_sample_morph_1_is_last_frame() {
        let wt = Wavetable::basic(256);
        let val = wt.sample(0.25, 1.0); // square at quarter = 1.0
        assert!((val - 1.0).abs() < 0.01, "expected 1.0, got {val}");
    }

    #[test]
    fn renders_note() {
        let synth = WavetableSynth::new("basic");
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let out = synth.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn ignores_sample_events() {
        let synth = WavetableSynth::new("basic");
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = synth.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn morph_varies_timbre() {
        let synth = WavetableSynth::new("basic");
        let mut event1 = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        event1.params.set(super::super::param_defs::morph(), 0.0);
        let out1 = synth.render(&event1, &ctx());

        let mut event2 = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        event2.params.set(super::super::param_defs::morph(), 1.0);
        let out2 = synth.render(&event2, &ctx());

        assert_ne!(out1, out2);
    }

    #[test]
    fn name() {
        let synth = WavetableSynth::new("basic");
        assert_eq!(Instrument::name(&synth), "wavetable");
    }
}
