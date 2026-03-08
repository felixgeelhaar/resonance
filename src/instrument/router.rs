//! Instrument router — dispatches events to the correct instrument by TrackId.

use std::collections::HashMap;

use crate::dsl::ast::{InstrumentRef, TrackDef};
use crate::event::types::TrackId;
use crate::event::{Event, RenderContext, RenderFn};
use crate::plugin::registry::PluginRegistry;

use super::{
    resolve_kit, BassSynth, DrumKit, Instrument, NoiseGen, PluckSynth, PolySynth, SampleBank,
};

/// Routes events to the correct instrument based on track ID.
pub struct InstrumentRouter {
    routes: HashMap<TrackId, usize>,
    instruments: Vec<Box<dyn Instrument>>,
}

impl InstrumentRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            instruments: Vec::new(),
        }
    }

    /// Add a route from a track ID to an instrument.
    pub fn add_route(&mut self, track_id: TrackId, instrument: Box<dyn Instrument>) {
        let idx = self.instruments.len();
        self.instruments.push(instrument);
        self.routes.insert(track_id, idx);
    }

    /// Render an event using the routed instrument.
    pub fn render(&self, event: &Event, ctx: &RenderContext) -> Vec<f32> {
        if let Some(&idx) = self.routes.get(&event.track_id) {
            self.instruments[idx].render(event, ctx)
        } else {
            Vec::new() // Unknown track → silence
        }
    }

    /// Convert this router into a boxed RenderFn.
    pub fn into_render_fn(self) -> RenderFn {
        Box::new(move |event: &Event, ctx: &RenderContext| self.render(event, ctx))
    }

    /// Build a router from compiled track definitions.
    pub fn from_track_defs(
        defs: &[(TrackId, TrackDef)],
        default_bank: SampleBank,
        seed: u64,
    ) -> Self {
        let mut router = Self::new();

        for (track_id, track_def) in defs {
            let instrument: Box<dyn Instrument> = match &track_def.instrument {
                InstrumentRef::Kit(_) => Box::new(DrumKit::new(default_bank.clone())),
                InstrumentRef::Bass => Box::new(BassSynth::new()),
                InstrumentRef::Poly => Box::new(PolySynth::new()),
                InstrumentRef::Pluck => Box::new(PluckSynth::new(seed)),
                InstrumentRef::Noise => Box::new(NoiseGen::new(seed)),
                InstrumentRef::Plugin(_) => Box::new(BassSynth::new()), // fallback
                InstrumentRef::Fm => Box::new(super::FmSynth::new()),
                InstrumentRef::Wavetable(name) => Box::new(super::WavetableSynth::new(name)),
            };
            router.add_route(*track_id, instrument);
        }

        router
    }

    /// Build a router from compiled track definitions, resolving kit names per track.
    ///
    /// Unlike [`from_track_defs`], this resolves each track's kit name independently,
    /// supporting custom WAV sample directories and plugin instruments. On kit resolution
    /// error, falls back to the synthetic default kit.
    pub fn from_track_defs_with_kits(
        defs: &[(TrackId, TrackDef)],
        sample_rate: u32,
        seed: u64,
        plugin_registry: &PluginRegistry,
    ) -> Self {
        let mut router = Self::new();

        for (track_id, track_def) in defs {
            let instrument: Box<dyn Instrument> = match &track_def.instrument {
                InstrumentRef::Kit(name) => {
                    let bank = resolve_kit(name, sample_rate, seed)
                        .unwrap_or_else(|_| super::build_default_kit(sample_rate, seed));
                    Box::new(DrumKit::new(bank))
                }
                InstrumentRef::Bass => Box::new(BassSynth::new()),
                InstrumentRef::Poly => Box::new(PolySynth::new()),
                InstrumentRef::Pluck => Box::new(PluckSynth::new(seed)),
                InstrumentRef::Noise => Box::new(NoiseGen::new(seed)),
                InstrumentRef::Plugin(name) => plugin_registry
                    .create_instrument(name, sample_rate)
                    .unwrap_or_else(|| Box::new(BassSynth::new())),
                InstrumentRef::Fm => Box::new(super::FmSynth::new()),
                InstrumentRef::Wavetable(name) => Box::new(super::WavetableSynth::new(name)),
            };
            router.add_route(*track_id, instrument);
        }

        router
    }
}

impl Default for InstrumentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Beat, RenderContext};
    use crate::instrument::{SampleBank, SampleData};

    fn ctx() -> RenderContext {
        RenderContext {
            sample_rate: 44100,
            channels: 2,
            bpm: 120.0,
        }
    }

    fn test_bank() -> SampleBank {
        let mut bank = SampleBank::new();
        bank.insert("kick", SampleData::from_mono(vec![0.5, 0.3], 44100));
        bank
    }

    #[test]
    fn routes_to_correct_instrument() {
        let mut router = InstrumentRouter::new();
        router.add_route(TrackId(0), Box::new(DrumKit::new(test_bank())));
        router.add_route(TrackId(1), Box::new(BassSynth::new()));

        let kick = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = router.render(&kick, &ctx());
        assert!(!out.is_empty());

        let note = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(1), 36, 0.8);
        let out = router.render(&note, &ctx());
        assert!(!out.is_empty());
    }

    #[test]
    fn unknown_track_returns_silence() {
        let router = InstrumentRouter::new();
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(99), "kick", 0.8);
        let out = router.render(&event, &ctx());
        assert!(out.is_empty());
    }

    #[test]
    fn into_render_fn_works() {
        let mut router = InstrumentRouter::new();
        router.add_route(TrackId(0), Box::new(DrumKit::new(test_bank())));
        let mut render_fn = router.into_render_fn();

        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = render_fn(&event, &ctx());
        assert!(!out.is_empty());
    }

    #[test]
    fn from_track_defs() {
        let defs = vec![
            (
                TrackId(0),
                TrackDef {
                    name: "drums".to_string(),
                    instrument: InstrumentRef::Kit("default".to_string()),
                    sections: vec![],
                    midi_out: None,
                },
            ),
            (
                TrackId(1),
                TrackDef {
                    name: "bass".to_string(),
                    instrument: InstrumentRef::Bass,
                    sections: vec![],
                    midi_out: None,
                },
            ),
        ];

        let router = InstrumentRouter::from_track_defs(&defs, test_bank(), 42);

        let kick = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = router.render(&kick, &ctx());
        assert!(!out.is_empty());

        let note = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(1), 36, 0.8);
        let out = router.render(&note, &ctx());
        assert!(!out.is_empty());
    }

    #[test]
    fn multi_track_rendering() {
        let mut router = InstrumentRouter::new();
        router.add_route(TrackId(0), Box::new(DrumKit::new(test_bank())));
        router.add_route(TrackId(1), Box::new(BassSynth::new()));
        router.add_route(TrackId(2), Box::new(PolySynth::new()));

        // All should produce output
        let events = [
            Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8),
            Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(1), 36, 0.8),
            Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(2), 60, 0.8),
        ];

        for event in &events {
            let out = router.render(event, &ctx());
            assert!(
                !out.is_empty(),
                "track {:?} should produce output",
                event.track_id
            );
        }
    }

    #[test]
    fn from_track_defs_with_kits_routes_correctly() {
        let defs = vec![
            (
                TrackId(0),
                TrackDef {
                    name: "drums".to_string(),
                    instrument: InstrumentRef::Kit("default".to_string()),
                    sections: vec![],
                    midi_out: None,
                },
            ),
            (
                TrackId(1),
                TrackDef {
                    name: "bass".to_string(),
                    instrument: InstrumentRef::Bass,
                    sections: vec![],
                    midi_out: None,
                },
            ),
        ];

        let registry = PluginRegistry::default();
        let router = InstrumentRouter::from_track_defs_with_kits(&defs, 44100, 42, &registry);

        let kick = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = router.render(&kick, &ctx());
        assert!(!out.is_empty());

        let note = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(1), 36, 0.8);
        let out = router.render(&note, &ctx());
        assert!(!out.is_empty());
    }

    #[test]
    fn from_track_defs_with_kits_error_fallback() {
        // Non-existent path should fallback to default kit
        let defs = vec![(
            TrackId(0),
            TrackDef {
                name: "drums".to_string(),
                instrument: InstrumentRef::Kit("./nonexistent_path_xyz".to_string()),
                sections: vec![],
                midi_out: None,
            },
        )];

        let registry = PluginRegistry::default();
        let router = InstrumentRouter::from_track_defs_with_kits(&defs, 44100, 42, &registry);

        let kick = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out = router.render(&kick, &ctx());
        assert!(!out.is_empty(), "should fallback to default kit");
    }

    #[test]
    fn from_track_defs_with_kits_backward_compat() {
        // Using "default" kit should work the same as from_track_defs
        let defs = vec![(
            TrackId(0),
            TrackDef {
                name: "drums".to_string(),
                instrument: InstrumentRef::Kit("default".to_string()),
                sections: vec![],
                midi_out: None,
            },
        )];

        let registry = PluginRegistry::default();
        let router_old = InstrumentRouter::from_track_defs(&defs, test_bank(), 42);
        let router_new = InstrumentRouter::from_track_defs_with_kits(&defs, 44100, 42, &registry);

        let kick = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8);
        let out_old = router_old.render(&kick, &ctx());
        let out_new = router_new.render(&kick, &ctx());

        assert!(!out_old.is_empty());
        assert!(!out_new.is_empty());
    }

    #[test]
    fn plugin_fallback_to_bass_synth() {
        // Plugin that doesn't exist should fallback to BassSynth
        let defs = vec![(
            TrackId(0),
            TrackDef {
                name: "lead".to_string(),
                instrument: InstrumentRef::Plugin("nonexistent".to_string()),
                sections: vec![],
                midi_out: None,
            },
        )];

        let registry = PluginRegistry::default();
        let router = InstrumentRouter::from_track_defs_with_kits(&defs, 44100, 42, &registry);

        let note = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let out = router.render(&note, &ctx());
        assert!(!out.is_empty(), "plugin fallback should produce output");
    }
}
