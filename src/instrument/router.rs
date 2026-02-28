//! Instrument router — dispatches events to the correct instrument by TrackId.

use std::collections::HashMap;

use crate::dsl::ast::{InstrumentRef, TrackDef};
use crate::event::types::TrackId;
use crate::event::{Event, RenderContext, RenderFn};

use super::{BassSynth, DrumKit, Instrument, NoiseGen, PluckSynth, PolySynth, SampleBank};

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
                },
            ),
            (
                TrackId(1),
                TrackDef {
                    name: "bass".to_string(),
                    instrument: InstrumentRef::Bass,
                    sections: vec![],
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
}
