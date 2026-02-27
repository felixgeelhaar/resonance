//! Resonance — basic drum playback (Phase 0, Task 4).
//!
//! Plays a 2-bar 4/4 drum pattern through the audio engine:
//! Kick, snare, hi-hat, and clap generated synthetically.

use resonance::audio::AudioEngine;
use resonance::event::{Beat, Event, EventScheduler, TrackId};
use resonance::instrument::{build_default_kit, DrumKit};

use std::thread;
use std::time::Duration;

const BPM: f64 = 120.0;
const BLOCK_SIZE: u32 = 1024;
const SEED: u64 = 42;

/// Insert a 2-bar 4/4 drum pattern onto the timeline.
///
/// Pattern per bar:
/// - Kick: beats 0, 1.5, 2.5
/// - Snare: beats 1, 3
/// - Hi-hat: every 8th note (0, 0.5, 1, 1.5, 2, 2.5, 3, 3.5)
/// - Clap: beat 3 (layered with snare)
fn insert_pattern(scheduler: &mut EventScheduler) {
    let drum_track = TrackId(0);
    let eighth = Beat::from_beats_f64(0.5);
    let duration = Beat::from_beats(1);

    for bar in 0..2u32 {
        let bar_offset = Beat::from_bars(bar);

        // Kick: 0, 1.5, 2.5
        for &beat in &[0.0, 1.5, 2.5] {
            scheduler.timeline_mut().insert(Event::sample(
                bar_offset + Beat::from_beats_f64(beat),
                duration,
                drum_track,
                "kick",
                0.9,
            ));
        }

        // Snare: 1, 3
        for &beat in &[1.0, 3.0] {
            scheduler.timeline_mut().insert(Event::sample(
                bar_offset + Beat::from_beats_f64(beat),
                duration,
                drum_track,
                "snare",
                0.8,
            ));
        }

        // Hi-hat: every 8th note
        for i in 0..8 {
            scheduler.timeline_mut().insert(Event::sample(
                bar_offset + Beat::from_ticks(i * eighth.ticks()),
                duration,
                drum_track,
                "hat",
                0.5,
            ));
        }

        // Clap: beat 3
        scheduler.timeline_mut().insert(Event::sample(
            bar_offset + Beat::from_beats(3),
            duration,
            drum_track,
            "clap",
            0.6,
        ));
    }
}

fn main() {
    println!(
        "resonance v{} — basic drum playback",
        env!("CARGO_PKG_VERSION")
    );

    // 1. Start audio engine
    let mut engine = match AudioEngine::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("failed to start audio engine: {e}");
            std::process::exit(1);
        }
    };

    let sample_rate = engine.sample_rate();
    let channels = engine.channels();

    println!("audio: {sample_rate} Hz, {channels} ch");
    println!("tempo: {BPM} BPM, block size: {BLOCK_SIZE} frames");

    // 2. Build synthetic drum kit
    let bank = build_default_kit(sample_rate, SEED);
    let kit = DrumKit::new(bank);
    let mut render_fn = kit.into_render_fn();

    // 3. Create scheduler and insert pattern
    let mut scheduler = EventScheduler::new(BPM, sample_rate, channels, BLOCK_SIZE, SEED);
    insert_pattern(&mut scheduler);

    // 4. Calculate total blocks for 2 bars
    let two_bars_beats = 8.0; // 2 bars * 4 beats
    let two_bars_seconds = two_bars_beats * 60.0 / BPM;
    let two_bars_frames = (two_bars_seconds * sample_rate as f64) as u64;
    let total_blocks = (two_bars_frames / BLOCK_SIZE as u64) + 2; // +2 for overlap/tail

    println!(
        "playing 2-bar pattern ({:.1}s, ~{} blocks)...",
        two_bars_seconds, total_blocks
    );

    // 5. Playback loop
    scheduler.play();
    let sleep_duration = Duration::from_secs_f64(BLOCK_SIZE as f64 / sample_rate as f64 * 0.8);

    for _ in 0..total_blocks {
        if let Some(block) = scheduler.render_block(&mut render_fn) {
            if let Err(e) = engine.send_samples(block) {
                eprintln!("audio error: {e}");
            }
        }
        thread::sleep(sleep_duration);
    }

    // Let audio drain
    thread::sleep(Duration::from_millis(500));
    println!("done.");
}
