//! Config-based instrument — renders audio from plugin YAML definitions.

use std::collections::HashMap;
use std::path::Path;

use crate::event::types::NoteOrSample;
use crate::event::{Event, RenderContext};
use crate::instrument::envelope::AdsrEnvelope;
use crate::instrument::oscillator::{midi_to_freq, oscillator, Waveform};
use crate::instrument::{Instrument, SampleBank, SampleData};

use super::config::{InstrumentDef, InstrumentKind};

/// An instrument created from a plugin's YAML config.
pub struct ConfigInstrument {
    name: String,
    kind: InstrumentKind,
    samples: SampleBank,
    waveform: Waveform,
    envelope: AdsrEnvelope,
    filter_cutoff: f64,
}

impl ConfigInstrument {
    /// Create a sampler instrument from sample file mappings.
    pub fn sampler(
        name: String,
        sample_map: &HashMap<String, String>,
        base_dir: &Path,
        sample_rate: u32,
    ) -> Self {
        let mut bank = SampleBank::new();
        for (trigger, rel_path) in sample_map {
            let wav_path = base_dir.join(rel_path);
            if let Ok(data) = load_wav_file(&wav_path, sample_rate) {
                bank.insert(trigger.clone(), data);
            }
        }
        Self {
            name,
            kind: InstrumentKind::Sampler,
            samples: bank,
            waveform: Waveform::Sine,
            envelope: AdsrEnvelope {
                attack: 0.0,
                decay: 0.0,
                sustain: 1.0,
                release: 0.01,
            },
            filter_cutoff: 1.0,
        }
    }

    /// Create a synth instrument from config parameters.
    pub fn synth(name: String, def: &InstrumentDef) -> Self {
        let waveform = match def.waveform.as_deref() {
            Some("saw") => Waveform::Saw,
            Some("square") => Waveform::Square,
            Some("triangle") => Waveform::Triangle,
            _ => Waveform::Sine,
        };

        let envelope = if let Some(env) = &def.envelope {
            AdsrEnvelope {
                attack: env.attack,
                decay: env.decay,
                sustain: env.sustain,
                release: env.release,
            }
        } else {
            AdsrEnvelope {
                attack: 0.01,
                decay: 0.1,
                sustain: 0.7,
                release: 0.2,
            }
        };

        Self {
            name,
            kind: InstrumentKind::Synth,
            samples: SampleBank::new(),
            waveform,
            envelope,
            filter_cutoff: def.filter_cutoff.unwrap_or(1.0),
        }
    }

    fn render_sampler(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        let trigger_name = match &event.trigger {
            NoteOrSample::Sample(name) => name.as_str(),
            NoteOrSample::Note(_) => return Vec::new(),
        };
        let sample = match self.samples.get(trigger_name) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let duration_secs = event.duration.as_beats_f64() * 60.0 / ctx.bpm;
        let num_samples = (duration_secs * ctx.sample_rate as f64) as usize;
        let num_frames = num_samples.min(sample.len());

        let mut out = Vec::with_capacity(num_frames * ctx.channels as usize);
        let vel = event.velocity;
        for i in 0..num_frames {
            let s = sample.samples()[i] * vel;
            for _ in 0..ctx.channels {
                out.push(s);
            }
        }
        out
    }

    fn render_synth(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        let note = match &event.trigger {
            NoteOrSample::Note(n) => *n,
            NoteOrSample::Sample(_) => 60,
        };
        let freq = midi_to_freq(note);
        let duration_secs = event.duration.as_beats_f64() * 60.0 / ctx.bpm;
        let total_secs = self.envelope.total_duration(duration_secs);
        let num_samples = (total_secs * ctx.sample_rate as f64) as usize;

        let mut out = Vec::with_capacity(num_samples * ctx.channels as usize);
        let vel = event.velocity;

        for i in 0..num_samples {
            let t = i as f64 / ctx.sample_rate as f64;
            let phase = (t * freq).fract();
            let osc = oscillator(self.waveform, phase);
            let env = self.envelope.amplitude(t, duration_secs);
            let sample = (osc * env * vel as f64 * self.filter_cutoff) as f32;
            for _ in 0..ctx.channels {
                out.push(sample);
            }
        }
        out
    }
}

impl Instrument for ConfigInstrument {
    fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        match self.kind {
            InstrumentKind::Sampler => self.render_sampler(event, ctx),
            InstrumentKind::Synth => self.render_synth(event, ctx),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Load a single WAV file into SampleData.
fn load_wav_file(path: &Path, _target_rate: u32) -> Result<SampleData, std::io::Error> {
    let reader = hound::WavReader::open(path)
        .map_err(|e| std::io::Error::other(format!("WAV read error: {e}")))?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max)
                .collect()
        }
    };

    // If stereo, mix down to mono
    let mono = if spec.channels == 2 {
        samples.chunks(2).map(|c| (c[0] + c[1]) * 0.5).collect()
    } else {
        samples
    };

    Ok(SampleData::from_mono(mono, spec.sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::types::TrackId;
    use crate::event::Beat;

    fn ctx() -> RenderContext {
        RenderContext {
            sample_rate: 44100,
            channels: 2,
            bpm: 120.0,
        }
    }

    #[test]
    fn synth_renders_non_empty() {
        let def = InstrumentDef {
            kind: InstrumentKind::Synth,
            samples: None,
            waveform: Some("sine".to_string()),
            envelope: None,
            filter_cutoff: None,
        };
        let inst = ConfigInstrument::synth("test_synth".to_string(), &def);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let out = inst.render(&event, &ctx());
        assert!(!out.is_empty());
        assert!(out.iter().any(|&s| s.abs() > 0.0));
    }

    #[test]
    fn synth_saw_waveform() {
        let def = InstrumentDef {
            kind: InstrumentKind::Synth,
            samples: None,
            waveform: Some("saw".to_string()),
            envelope: Some(super::super::config::EnvelopeDef {
                attack: 0.0,
                decay: 0.0,
                sustain: 1.0,
                release: 0.01,
            }),
            filter_cutoff: Some(1.0),
        };
        let inst = ConfigInstrument::synth("saw_synth".to_string(), &def);
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let out = inst.render(&event, &ctx());
        assert!(!out.is_empty());
    }

    #[test]
    fn sampler_empty_bank_returns_empty() {
        let inst = ConfigInstrument::sampler(
            "empty".to_string(),
            &HashMap::new(),
            Path::new("/nonexistent"),
            44100,
        );
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = inst.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn instrument_name() {
        let def = InstrumentDef {
            kind: InstrumentKind::Synth,
            samples: None,
            waveform: None,
            envelope: None,
            filter_cutoff: None,
        };
        let inst = ConfigInstrument::synth("My Synth".to_string(), &def);
        assert_eq!(inst.name(), "My Synth");
    }
}
