//! Drum kit — maps sample names to audio data and renders events.

use crate::event::{Event, NoteOrSample, RenderContext, RenderFn};

use super::{Instrument, SampleBank};

/// A drum kit that renders sample-trigger events using a [`SampleBank`].
pub struct DrumKit {
    bank: SampleBank,
}

impl DrumKit {
    /// Create a drum kit backed by the given sample bank.
    pub fn new(bank: SampleBank) -> Self {
        Self { bank }
    }

    /// Convert this drum kit into a boxed [`RenderFn`] compatible with `EventScheduler`.
    pub fn into_render_fn(self) -> RenderFn {
        Box::new(move |event: &Event, ctx: &RenderContext| Instrument::render(&self, event, ctx))
    }
}

impl Instrument for DrumKit {
    fn render(&self, event: &Event, _ctx: &RenderContext) -> Vec<f32> {
        let name = match &event.trigger {
            NoteOrSample::Sample(name) => name,
            NoteOrSample::Note(_) => return Vec::new(),
        };

        let sample_data = match self.bank.get(name) {
            Some(data) => data,
            None => return Vec::new(),
        };

        if event.velocity <= 0.0 {
            return Vec::new();
        }

        let mono = sample_data.samples();
        let velocity = event.velocity;

        // Mono → stereo interleave with velocity scaling.
        let mut output = Vec::with_capacity(mono.len() * 2);
        for &s in mono {
            let scaled = s * velocity;
            output.push(scaled); // L
            output.push(scaled); // R
        }

        output
    }

    fn name(&self) -> &str {
        "drum_kit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Beat, TrackId};
    use crate::instrument::{Instrument, SampleData};

    fn test_ctx() -> RenderContext {
        RenderContext {
            sample_rate: 44100,
            channels: 2,
            bpm: 120.0,
        }
    }

    fn test_bank() -> SampleBank {
        let mut bank = SampleBank::new();
        bank.insert("kick", SampleData::from_mono(vec![0.5, 0.3, 0.1], 44100));
        bank.insert("snare", SampleData::from_mono(vec![0.4, 0.2], 44100));
        bank
    }

    #[test]
    fn renders_sample_event_stereo() {
        let kit = DrumKit::new(test_bank());
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 1.0);
        let ctx = test_ctx();
        let out = kit.render(&event, &ctx);

        // 3 mono samples → 6 stereo samples
        assert_eq!(out.len(), 6);
        assert!((out[0] - 0.5).abs() < f32::EPSILON); // L
        assert!((out[1] - 0.5).abs() < f32::EPSILON); // R
        assert!((out[2] - 0.3).abs() < f32::EPSILON);
        assert!((out[3] - 0.3).abs() < f32::EPSILON);
        assert!((out[4] - 0.1).abs() < f32::EPSILON);
        assert!((out[5] - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn note_event_returns_empty() {
        let kit = DrumKit::new(test_bank());
        let event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        let ctx = test_ctx();
        let out = kit.render(&event, &ctx);
        assert!(out.is_empty());
    }

    #[test]
    fn unknown_sample_returns_empty() {
        let kit = DrumKit::new(test_bank());
        let event = Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "nonexistent",
            0.8,
        );
        let ctx = test_ctx();
        let out = kit.render(&event, &ctx);
        assert!(out.is_empty());
    }

    #[test]
    fn velocity_scaling() {
        let kit = DrumKit::new(test_bank());
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.5);
        let ctx = test_ctx();
        let out = kit.render(&event, &ctx);

        // 0.5 * 0.5 = 0.25
        assert!((out[0] - 0.25).abs() < f32::EPSILON);
        assert!((out[1] - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn velocity_zero_returns_empty() {
        let kit = DrumKit::new(test_bank());
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.0);
        let ctx = test_ctx();
        let out = kit.render(&event, &ctx);
        assert!(out.is_empty());
    }

    #[test]
    fn instrument_trait_name() {
        let kit = DrumKit::new(test_bank());
        assert_eq!(Instrument::name(&kit), "drum_kit");
    }

    #[test]
    fn instrument_trait_render() {
        let kit = DrumKit::new(test_bank());
        let event = Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 1.0);
        let ctx = test_ctx();
        let out = Instrument::render(&kit, &event, &ctx);
        assert_eq!(out.len(), 6);
    }

    #[test]
    fn into_render_fn_works_with_scheduler() {
        use crate::event::EventScheduler;

        let bank = test_bank();
        let kit = DrumKit::new(bank);
        let mut render_fn = kit.into_render_fn();

        let mut scheduler = EventScheduler::new(120.0, 44100, 2, 1024, 42);
        scheduler.timeline_mut().insert(Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            0.8,
        ));
        scheduler.play();

        let block = scheduler.render_block(&mut render_fn).unwrap();
        // Block should contain non-zero samples from the kick
        assert!(block.iter().any(|&s| s != 0.0));
    }
}
