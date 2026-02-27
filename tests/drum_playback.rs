//! Integration tests for the drum playback pipeline.
//!
//! Tests the full path: events → EventScheduler → DrumKit render → sample blocks.
//! No audio hardware required — tests only verify rendered sample data.

use resonance::event::{Beat, Event, EventScheduler, TrackId};
use resonance::instrument::{build_default_kit, DrumKit};

const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u16 = 2;
const BLOCK_SIZE: u32 = 1024;
const BPM: f64 = 120.0;
const SEED: u64 = 42;

/// Insert the standard 2-bar drum pattern (same as main.rs).
fn insert_pattern(scheduler: &mut EventScheduler) {
    let drum_track = TrackId(0);
    let eighth = Beat::from_beats_f64(0.5);
    let duration = Beat::from_beats(1);

    for bar in 0..2u32 {
        let bar_offset = Beat::from_bars(bar);

        for &beat in &[0.0, 1.5, 2.5] {
            scheduler.timeline_mut().insert(Event::sample(
                bar_offset + Beat::from_beats_f64(beat),
                duration,
                drum_track,
                "kick",
                0.9,
            ));
        }

        for &beat in &[1.0, 3.0] {
            scheduler.timeline_mut().insert(Event::sample(
                bar_offset + Beat::from_beats_f64(beat),
                duration,
                drum_track,
                "snare",
                0.8,
            ));
        }

        for i in 0..8 {
            scheduler.timeline_mut().insert(Event::sample(
                bar_offset + Beat::from_ticks(i * eighth.ticks()),
                duration,
                drum_track,
                "hat",
                0.5,
            ));
        }

        scheduler.timeline_mut().insert(Event::sample(
            bar_offset + Beat::from_beats(3),
            duration,
            drum_track,
            "clap",
            0.6,
        ));
    }
}

/// Run the full pipeline and collect all rendered blocks.
fn run_pipeline(seed: u64) -> Vec<Vec<f32>> {
    let bank = build_default_kit(SAMPLE_RATE, seed);
    let kit = DrumKit::new(bank);
    let mut render_fn = kit.into_render_fn();

    let mut scheduler = EventScheduler::new(BPM, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, seed);
    insert_pattern(&mut scheduler);
    scheduler.play();

    // 2 bars at 120 BPM = 4 seconds = 176400 frames ≈ 173 blocks of 1024
    let total_blocks = 180;
    (0..total_blocks)
        .filter_map(|_| scheduler.render_block(&mut render_fn))
        .collect()
}

#[test]
fn full_pipeline_produces_audio() {
    let blocks = run_pipeline(SEED);
    assert!(!blocks.is_empty());

    // At least some blocks should contain non-zero samples
    let non_silent = blocks
        .iter()
        .filter(|b| b.iter().any(|&s| s != 0.0))
        .count();
    assert!(
        non_silent > 0,
        "expected some non-silent blocks, got all silence"
    );
}

#[test]
fn full_pipeline_determinism() {
    let run_a = run_pipeline(SEED);
    let run_b = run_pipeline(SEED);

    assert_eq!(run_a.len(), run_b.len());
    for (i, (a, b)) in run_a.iter().zip(run_b.iter()).enumerate() {
        assert_eq!(a, b, "block {i} differs between runs");
    }
}

#[test]
fn full_pipeline_different_seeds_differ() {
    let run_a = run_pipeline(SEED);
    let run_b = run_pipeline(SEED + 100);

    // The kick is deterministic (no seed), but snare/hat/clap use seeds.
    // Overall output should differ.
    let differs = run_a.iter().zip(run_b.iter()).any(|(a, b)| a != b);
    assert!(differs, "different seeds should produce different output");
}

#[test]
fn output_length_matches_duration() {
    let blocks = run_pipeline(SEED);

    // Each block = BLOCK_SIZE frames * CHANNELS samples
    let block_samples = BLOCK_SIZE as usize * CHANNELS as usize;
    for block in &blocks {
        assert_eq!(
            block.len(),
            block_samples,
            "every block should have {block_samples} samples"
        );
    }

    // Total rendered frames
    let total_frames = blocks.len() as u64 * BLOCK_SIZE as u64;
    // 2 bars at 120 BPM = 4 seconds = 176400 frames
    let expected_frames = (4.0 * SAMPLE_RATE as f64) as u64;
    // We render 180 blocks, so total should be >= expected
    assert!(total_frames >= expected_frames);
}

#[test]
fn events_are_distributed_across_blocks() {
    let blocks = run_pipeline(SEED);

    // Count blocks with non-zero content
    let non_silent: Vec<usize> = blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            if b.iter().any(|&s| s != 0.0) {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    // We have events spread across 2 bars (4 seconds).
    // At 44100 Hz / 1024 block size, events should span many blocks.
    assert!(
        non_silent.len() > 10,
        "expected events in many blocks, got {}",
        non_silent.len()
    );

    // Events should span a wide range of block indices
    let first = *non_silent.first().unwrap();
    let last = *non_silent.last().unwrap();
    assert!(
        last - first > 50,
        "expected events spanning many blocks, range was {first}..{last}"
    );
}

#[test]
fn samples_are_finite_and_reasonable() {
    let blocks = run_pipeline(SEED);

    for (i, block) in blocks.iter().enumerate() {
        for (j, &sample) in block.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "block {i} sample {j} is not finite: {sample}"
            );
            // Pre-limiter output may exceed 1.0 due to additive mixing,
            // but should stay within reasonable range for drum samples.
            assert!(
                sample.abs() < 10.0,
                "block {i} sample {j} unreasonably large: {sample}"
            );
        }
    }
}

#[test]
fn single_event_renders_correctly() {
    let bank = build_default_kit(SAMPLE_RATE, SEED);
    let kit = DrumKit::new(bank);
    let mut render_fn = kit.into_render_fn();

    let mut scheduler = EventScheduler::new(BPM, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED);
    scheduler.timeline_mut().insert(Event::sample(
        Beat::ZERO,
        Beat::from_beats(1),
        TrackId(0),
        "kick",
        0.9,
    ));
    scheduler.play();

    let block = scheduler.render_block(&mut render_fn).unwrap();
    // First block should have audio from the kick at beat 0
    assert!(block.iter().any(|&s| s != 0.0));
}
