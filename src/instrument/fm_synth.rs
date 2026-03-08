//! FM Synthesizer — 2-operator FM with modulator → carrier, SVF filter.

use std::f64::consts::PI;

use crate::event::{Event, NoteOrSample, RenderContext};

use super::envelope::AdsrEnvelope;
use super::filter::{FilterMode, SvfFilter};
use super::oscillator::midi_to_freq;
use super::Instrument;

/// 2-operator FM synth: `sin(2π·fc·t + index·sin(2π·fm·t))` where `fm = fc·ratio`.
pub struct FmSynth {
    envelope: AdsrEnvelope,
    ratio: f64,
    index: f64,
}

impl FmSynth {
    pub fn new() -> Self {
        Self {
            envelope: AdsrEnvelope {
                attack: 0.01,
                decay: 0.15,
                sustain: 0.6,
                release: 0.1,
            },
            ratio: 2.0,
            index: 1.0,
        }
    }
}

impl Default for FmSynth {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument for FmSynth {
    fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        let midi_note = match &event.trigger {
            NoteOrSample::Note(n) => *n,
            NoteOrSample::Sample(_) => return Vec::new(),
        };

        if event.velocity <= 0.0 {
            return Vec::new();
        }

        let ratio = event
            .params
            .get(&super::param_defs::fm_ratio())
            .map(|v| v as f64)
            .unwrap_or(self.ratio);
        let index = event
            .params
            .get(&super::param_defs::fm_index())
            .map(|v| v as f64)
            .unwrap_or(self.index);
        let cutoff = event
            .params
            .get(&super::param_defs::cutoff())
            .map(|v| v as f64)
            .unwrap_or(4000.0);
        let resonance = event
            .params
            .get(&super::param_defs::resonance())
            .map(|v| v as f64)
            .unwrap_or(0.707);

        let fc = midi_to_freq(midi_note);
        let fm = fc * ratio;

        let duration_secs = event.duration.as_beats_f64() * 60.0 / ctx.bpm;
        let total_secs = self.envelope.total_duration(duration_secs);
        let num_samples = (total_secs * ctx.sample_rate as f64) as usize;

        let mut carrier_phase = 0.0_f64;
        let mut mod_phase = 0.0_f64;
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

            let modulator = (2.0 * PI * mod_phase).sin();
            let carrier = (2.0 * PI * carrier_phase + index * modulator).sin();
            let filtered = filter.process(carrier);

            let sample = (filtered * env * event.velocity as f64) as f32;

            for _ in 0..ctx.channels {
                output.push(sample);
            }

            carrier_phase += fc / ctx.sample_rate as f64;
            mod_phase += fm / ctx.sample_rate as f64;
        }

        output
    }

    fn name(&self) -> &str {
        "fm"
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
    fn renders_note() {
        let synth = FmSynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let out = synth.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn ignores_sample_events() {
        let synth = FmSynth::new();
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = synth.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn zero_velocity_silent() {
        let synth = FmSynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.0);
        let out = synth.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn ratio_changes_timbre() {
        let synth = FmSynth::new();
        let mut event1 = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        event1.params.set(super::super::param_defs::fm_ratio(), 1.0);
        let out1 = synth.render(&event1, &ctx());

        let mut event2 = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        event2.params.set(super::super::param_defs::fm_ratio(), 4.0);
        let out2 = synth.render(&event2, &ctx());

        assert_ne!(out1, out2);
    }

    #[test]
    fn index_changes_timbre() {
        let synth = FmSynth::new();
        let mut event1 = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        event1.params.set(super::super::param_defs::fm_index(), 0.1);
        let out1 = synth.render(&event1, &ctx());

        let mut event2 = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        event2.params.set(super::super::param_defs::fm_index(), 5.0);
        let out2 = synth.render(&event2, &ctx());

        assert_ne!(out1, out2);
    }

    #[test]
    fn output_bounded() {
        let synth = FmSynth::new();
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 1.0);
        let out = synth.render(&event, &ctx());
        for &s in &out {
            assert!(s.abs() <= 1.5, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn name() {
        let synth = FmSynth::new();
        assert_eq!(Instrument::name(&synth), "fm");
    }
}
