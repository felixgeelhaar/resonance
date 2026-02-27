//! Event stream engine — deterministic scheduler, seedable randomness, transform pipeline.
//!
//! The [`EventScheduler`] sits between the DSL/pattern layer and the audio engine.
//! It maintains musical time via [`Transport`], stores events on a [`Timeline`],
//! and renders them into sample buffers using a caller-provided [`RenderFn`].
//!
//! The scheduler does **not** own an `AudioEngine` — the caller is responsible
//! for sending the rendered buffers to audio output. This keeps all scheduling
//! logic testable without audio hardware.

pub mod beat;
pub mod timeline;
pub mod transport;
pub mod types;

pub use beat::{Beat, DEFAULT_BEATS_PER_BAR, TICKS_PER_BEAT};
pub use timeline::Timeline;
pub use transport::{PlayState, Transport};
pub use types::{Event, NoteOrSample, Params, TrackId};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Context passed to the render callback for each event.
pub struct RenderContext {
    pub sample_rate: u32,
    pub channels: u16,
    pub bpm: f64,
}

/// A callback that renders a single event into interleaved samples.
///
/// The returned `Vec<f32>` contains interleaved channel data. Its length
/// can exceed the current block — overflow is handled by the overlap buffer.
pub type RenderFn = Box<dyn FnMut(&Event, &RenderContext) -> Vec<f32>>;

/// The event scheduler: renders musical events into audio sample blocks.
pub struct EventScheduler {
    timeline: Timeline,
    transport: Transport,
    rng: ChaCha8Rng,
    block_size_frames: u32,
    /// Samples that spilled past the previous block boundary.
    overlap_buffer: Vec<f32>,
}

impl EventScheduler {
    /// Create a new scheduler.
    ///
    /// - `bpm`: tempo in beats per minute
    /// - `sample_rate`: audio sample rate (e.g. 44100)
    /// - `channels`: number of audio channels (e.g. 2 for stereo)
    /// - `block_size_frames`: number of frames per render block (e.g. 1024)
    /// - `seed`: RNG seed for deterministic randomness
    pub fn new(
        bpm: f64,
        sample_rate: u32,
        channels: u16,
        block_size_frames: u32,
        seed: u64,
    ) -> Self {
        Self {
            timeline: Timeline::new(),
            transport: Transport::new(bpm, sample_rate, channels),
            rng: ChaCha8Rng::seed_from_u64(seed),
            block_size_frames,
            overlap_buffer: Vec::new(),
        }
    }

    /// Start playback.
    pub fn play(&mut self) {
        self.transport.play();
    }

    /// Stop playback.
    pub fn stop(&mut self) {
        self.transport.stop();
    }

    /// Reset the scheduler to the beginning.
    pub fn reset(&mut self) {
        self.transport.reset();
        self.timeline.reset_cursor();
        self.overlap_buffer.clear();
    }

    /// Get a mutable reference to the RNG for seeded randomness.
    pub fn rng(&mut self) -> &mut ChaCha8Rng {
        &mut self.rng
    }

    /// Get a reference to the transport.
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    /// Get a mutable reference to the transport.
    pub fn transport_mut(&mut self) -> &mut Transport {
        &mut self.transport
    }

    /// Get a mutable reference to the timeline for event insertion.
    pub fn timeline_mut(&mut self) -> &mut Timeline {
        &mut self.timeline
    }

    /// Set BPM (takes effect on the next render_block call).
    pub fn set_bpm(&mut self, bpm: f64) {
        self.transport.set_bpm(bpm);
    }

    /// Render the next block of audio samples.
    ///
    /// Returns `None` if the transport is stopped.
    /// Returns `Some(Vec<f32>)` with interleaved samples of length
    /// `block_size_frames * channels`.
    ///
    /// The `render_fn` callback is invoked for each event that falls within
    /// this block's time window. Rendered samples are mixed additively.
    pub fn render_block(&mut self, render_fn: &mut RenderFn) -> Option<Vec<f32>> {
        let (from, to) = self.transport.advance_by_frames(self.block_size_frames)?;

        let channels = self.transport.channels() as usize;
        let block_samples = self.block_size_frames as usize * channels;
        let mut output = vec![0.0f32; block_samples];

        // Mix in overlap from previous block
        let overlap_len = self.overlap_buffer.len().min(block_samples);
        for (out, &ovl) in output[..overlap_len]
            .iter_mut()
            .zip(&self.overlap_buffer[..overlap_len])
        {
            *out += ovl;
        }
        // Keep any remaining overlap that extends beyond this block too
        if self.overlap_buffer.len() > block_samples {
            self.overlap_buffer = self.overlap_buffer[block_samples..].to_vec();
        } else {
            self.overlap_buffer.clear();
        }

        let bpm = self.transport.bpm();
        let sample_rate = self.transport.sample_rate();
        let ctx = RenderContext {
            sample_rate,
            channels: channels as u16,
            bpm,
        };

        let block_start_sample = from.to_sample_offset(bpm, sample_rate);

        let events = self.timeline.drain_range(from, to);
        for event in &events {
            let rendered = render_fn(event, &ctx);
            if rendered.is_empty() {
                continue;
            }

            let event_global_sample = event.time.to_sample_offset(bpm, sample_rate);
            let offset_frames = event_global_sample.saturating_sub(block_start_sample);
            let offset_samples = offset_frames as usize * channels;

            // Mix rendered samples into output, spilling into overlap if needed
            for (i, &sample) in rendered.iter().enumerate() {
                let pos = offset_samples + i;
                if pos < block_samples {
                    output[pos] += sample;
                } else {
                    // Spill into overlap buffer
                    let overlap_pos = pos - block_samples;
                    if overlap_pos >= self.overlap_buffer.len() {
                        self.overlap_buffer.resize(overlap_pos + 1, 0.0);
                    }
                    self.overlap_buffer[overlap_pos] += sample;
                }
            }
        }

        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 44100;
    const CHANNELS: u16 = 2;
    const BLOCK_SIZE: u32 = 1024;
    const BPM: f64 = 120.0;
    const SEED: u64 = 42;

    fn make_scheduler() -> EventScheduler {
        EventScheduler::new(BPM, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED)
    }

    /// A render function that returns a short impulse (1.0 for one frame).
    fn impulse_render() -> RenderFn {
        Box::new(|_event: &Event, ctx: &RenderContext| vec![1.0; ctx.channels as usize])
    }

    /// A render function that returns a fixed-length sample (e.g. simulating a drum hit).
    fn fixed_length_render(frames: usize) -> RenderFn {
        Box::new(move |event: &Event, ctx: &RenderContext| {
            let len = frames * ctx.channels as usize;
            vec![event.velocity; len]
        })
    }

    #[test]
    fn creation() {
        let s = make_scheduler();
        assert_eq!(s.transport().state(), PlayState::Stopped);
        assert_eq!(s.transport().position(), Beat::ZERO);
    }

    #[test]
    fn stopped_returns_none() {
        let mut s = make_scheduler();
        let mut render = impulse_render();
        assert!(s.render_block(&mut render).is_none());
    }

    #[test]
    fn empty_timeline_renders_silence() {
        let mut s = make_scheduler();
        s.play();
        let mut render = impulse_render();
        let block = s.render_block(&mut render).unwrap();
        assert_eq!(block.len(), BLOCK_SIZE as usize * CHANNELS as usize);
        assert!(block.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn single_event_at_beat_zero() {
        let mut s = make_scheduler();
        s.timeline_mut().insert(Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            0.9,
        ));
        s.play();
        let mut render = impulse_render();
        let block = s.render_block(&mut render).unwrap();

        // First frame should have the impulse
        assert!((block[0] - 1.0).abs() < f32::EPSILON);
        assert!((block[1] - 1.0).abs() < f32::EPSILON);
        // Rest should be silence
        assert!(block[2..].iter().all(|&s| s == 0.0));
    }

    #[test]
    fn event_timing_accuracy() {
        // Place an event at beat 1 (= 22050 frames at 120 BPM / 44100 Hz).
        // With block size 1024, that's block index 22050/1024 ≈ 21.5 → block 21.
        let mut s = make_scheduler();
        s.timeline_mut().insert(Event::sample(
            Beat::from_beats(1),
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            1.0,
        ));
        s.play();
        let mut render = impulse_render();

        // Render blocks until we find the impulse
        let mut found = false;
        for _ in 0..30 {
            let block = s.render_block(&mut render).unwrap();
            if block.iter().any(|&s| s != 0.0) {
                found = true;
                break;
            }
        }
        assert!(found, "event at beat 1 was never rendered");
    }

    #[test]
    fn multiple_events_mix_additively() {
        let mut s = make_scheduler();
        // Two events at beat 0 with different velocities
        s.timeline_mut().insert(Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            0.5,
        ));
        s.timeline_mut().insert(Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(1),
            "snare",
            0.3,
        ));
        s.play();

        // Render function returns velocity as sample value
        let mut render: RenderFn = Box::new(|event: &Event, ctx: &RenderContext| {
            vec![event.velocity; ctx.channels as usize]
        });

        let block = s.render_block(&mut render).unwrap();
        // Both should mix: 0.5 + 0.3 = 0.8
        assert!((block[0] - 0.8).abs() < f32::EPSILON);
        assert!((block[1] - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn events_across_blocks() {
        let mut s = make_scheduler();
        // Event at beat 0 (block 0) and beat 1 (block ~21)
        s.timeline_mut().insert_batch(vec![
            Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 1.0),
            Event::sample(
                Beat::from_beats(1),
                Beat::from_beats(1),
                TrackId(0),
                "snare",
                1.0,
            ),
        ]);
        s.play();
        let mut render = impulse_render();

        let mut events_found = 0;
        for _ in 0..30 {
            let block = s.render_block(&mut render).unwrap();
            if block.iter().any(|&s| s != 0.0) {
                events_found += 1;
            }
        }
        assert_eq!(
            events_found, 2,
            "should find exactly 2 events across blocks"
        );
    }

    #[test]
    fn overlap_buffer_continuity() {
        let mut s = EventScheduler::new(BPM, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED);
        // Event at beat 0, render function returns more samples than one block
        s.timeline_mut().insert(Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            0.7,
        ));
        s.play();

        let long_frames = BLOCK_SIZE as usize + 512; // exceeds one block
        let mut render = fixed_length_render(long_frames);

        let block1 = s.render_block(&mut render).unwrap();
        let block2 = s.render_block(&mut render).unwrap();

        // Block 1 should be fully filled
        assert!(block1.iter().all(|&s| (s - 0.7).abs() < f32::EPSILON));

        // Block 2 should have overlap in first 512 frames (1024 samples for stereo)
        let overlap_samples = 512 * CHANNELS as usize;
        for &s in &block2[..overlap_samples] {
            assert!(
                (s - 0.7).abs() < f32::EPSILON,
                "overlap region should contain spilled samples"
            );
        }
        // Rest of block 2 should be silence
        assert!(block2[overlap_samples..].iter().all(|&s| s == 0.0));
    }

    #[test]
    fn determinism_two_schedulers() {
        let run = |seed: u64| -> Vec<Vec<f32>> {
            let mut s = EventScheduler::new(BPM, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, seed);
            s.timeline_mut().insert_batch(vec![
                Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8),
                Event::sample(
                    Beat::from_beats_f64(0.5),
                    Beat::from_beats(1),
                    TrackId(1),
                    "hat",
                    0.5,
                ),
                Event::sample(
                    Beat::from_beats(1),
                    Beat::from_beats(1),
                    TrackId(0),
                    "snare",
                    0.9,
                ),
            ]);
            s.play();
            let mut render = impulse_render();
            (0..30)
                .map(|_| s.render_block(&mut render).unwrap())
                .collect()
        };

        let a = run(SEED);
        let b = run(SEED);
        assert_eq!(a.len(), b.len());
        for (block_a, block_b) in a.iter().zip(b.iter()) {
            assert_eq!(block_a, block_b, "blocks must be bit-identical");
        }
    }

    #[test]
    fn reset_returns_to_start() {
        let mut s = make_scheduler();
        s.timeline_mut().insert(Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            1.0,
        ));
        s.play();
        let mut render = impulse_render();

        // Render a few blocks
        for _ in 0..5 {
            s.render_block(&mut render);
        }
        assert!(s.transport().position().ticks() > 0);

        s.reset();
        assert_eq!(s.transport().position(), Beat::ZERO);
    }

    #[test]
    fn bpm_change() {
        let mut s = make_scheduler();
        s.timeline_mut().insert(Event::sample(
            Beat::from_beats(1),
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            1.0,
        ));
        s.play();
        let mut render = impulse_render();

        // At 120 BPM, beat 1 = frame 22050 → block ~21
        // Change to 240 BPM: beat 1 = frame 11025 → block ~10
        s.set_bpm(240.0);

        let mut found_block = None;
        for i in 0..20 {
            let block = s.render_block(&mut render).unwrap();
            if block.iter().any(|&s| s != 0.0) {
                found_block = Some(i);
                break;
            }
        }
        assert!(
            found_block.is_some(),
            "event should be found after BPM change"
        );
        // At 240 BPM, should appear earlier than block 21
        assert!(found_block.unwrap() < 15);
    }
}
