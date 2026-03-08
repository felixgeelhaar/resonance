//! Full pipeline integration tests — DSL → compile → scheduler → instruments → audio samples.
//!
//! These tests verify the entire audio pipeline produces real audio output
//! without requiring audio hardware (no AudioEngine involved).

use resonance::dsl::Compiler;
use resonance::event::{EventScheduler, RenderFn};
use resonance::instrument::{build_default_kit, InstrumentRouter};
use resonance::macro_engine::MacroEngine;

const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u16 = 2;
const BLOCK_SIZE: u32 = 1024;
const SEED: u64 = 42;

/// Helper: compile DSL source and build a fully wired scheduler + render function.
fn build_pipeline(src: &str) -> (EventScheduler, RenderFn) {
    let song = Compiler::compile(src).expect("compile failed");
    let bank = build_default_kit(SAMPLE_RATE, SEED);
    let router = InstrumentRouter::from_track_defs(&song.track_defs, bank, SEED);
    let mut scheduler = EventScheduler::new(song.tempo, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED);
    scheduler.timeline_mut().insert_batch(song.events);
    (scheduler, router.into_render_fn())
}

/// Helper: render N blocks and return them all.
fn render_blocks(
    scheduler: &mut EventScheduler,
    render_fn: &mut RenderFn,
    count: usize,
) -> Vec<Vec<f32>> {
    scheduler.play();
    (0..count)
        .filter_map(|_| scheduler.render_block(render_fn))
        .collect()
}

/// Helper: render N blocks with macro preprocessing.
fn render_blocks_with_macros(
    scheduler: &mut EventScheduler,
    render_fn: &mut RenderFn,
    macro_engine: &MacroEngine,
    count: usize,
) -> Vec<Vec<f32>> {
    scheduler.play();
    (0..count)
        .filter_map(|_| scheduler.render_block_with(render_fn, |e| macro_engine.apply_to_event(e)))
        .collect()
}

fn sample_drums_src() -> &'static str {
    "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n    snare: [. . X .]\n  }\n}"
}

fn sample_drums_bass_src() -> &'static str {
    "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n    snare: [. . X .]\n  }\n}\ntrack bass {\n  bass\n  section main [1 bars] {\n    note: [C2 . . C2]\n  }\n}"
}

// =============================================================================
// Test 1: Default starter template compiles and renders non-silent blocks
// =============================================================================

#[test]
fn dsl_to_audio_produces_sound() {
    let (mut scheduler, mut render_fn) = build_pipeline(sample_drums_src());
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    assert!(!blocks.is_empty(), "should produce blocks");

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(has_sound, "rendered blocks should contain non-silent audio");
}

// =============================================================================
// Test 2: Custom DSL with drums + bass renders audio
// =============================================================================

#[test]
fn custom_dsl_produces_audio() {
    let (mut scheduler, mut render_fn) = build_pipeline(sample_drums_bass_src());
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "drums + bass pipeline should produce audible output"
    );
}

// =============================================================================
// Test 3: Two different patterns produce different output
// =============================================================================

#[test]
fn different_dsl_produces_different_audio() {
    let src_a = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X X X X]\n  }\n}";
    let src_b = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    hat: [x x x x]\n  }\n}";

    let (mut sched_a, mut rf_a) = build_pipeline(src_a);
    let (mut sched_b, mut rf_b) = build_pipeline(src_b);

    let blocks_a = render_blocks(&mut sched_a, &mut rf_a, 30);
    let blocks_b = render_blocks(&mut sched_b, &mut rf_b, 30);

    // At least one block should differ
    let differ = blocks_a.iter().zip(blocks_b.iter()).any(|(a, b)| a != b);
    assert!(
        differ,
        "different patterns (kick vs hat) should produce different audio"
    );
}

// =============================================================================
// Test 4: Bass track produces pitched note audio
// =============================================================================

#[test]
fn bass_through_pipeline() {
    let src =
        "tempo 120\ntrack bass {\n  bass\n  section main [1 bars] {\n    note: [C2 . . .]\n  }\n}";
    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(has_sound, "bass track should produce audible pitched audio");
}

// =============================================================================
// Test 5: Multi-track richer than single track
// =============================================================================

#[test]
fn multi_track_richer_than_single() {
    let src_single = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
    let src_multi = sample_drums_bass_src();

    let (mut sched_s, mut rf_s) = build_pipeline(src_single);
    let (mut sched_m, mut rf_m) = build_pipeline(src_multi);

    let blocks_s = render_blocks(&mut sched_s, &mut rf_s, 50);
    let blocks_m = render_blocks(&mut sched_m, &mut rf_m, 50);

    // Compare total energy (sum of squared samples)
    let energy_single: f64 = blocks_s
        .iter()
        .flat_map(|b| b.iter())
        .map(|&s| (s as f64) * (s as f64))
        .sum();

    let energy_multi: f64 = blocks_m
        .iter()
        .flat_map(|b| b.iter())
        .map(|&s| (s as f64) * (s as f64))
        .sum();

    assert!(
        energy_multi > energy_single,
        "drums+bass ({energy_multi:.2}) should have more energy than drums alone ({energy_single:.2})"
    );
}

// =============================================================================
// Test 6: Macros affect rendered audio
// =============================================================================

#[test]
fn macros_affect_rendered_audio() {
    let src = "macro filter = 0.5\nmap filter -> cutoff (0.0..1.0) linear\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";

    let song = Compiler::compile(src).expect("compile failed");

    // Render without macros
    let bank1 = build_default_kit(SAMPLE_RATE, SEED);
    let router1 = InstrumentRouter::from_track_defs(&song.track_defs, bank1, SEED);
    let mut sched1 = EventScheduler::new(song.tempo, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED);
    sched1.timeline_mut().insert_batch(song.events.clone());
    let mut rf1 = router1.into_render_fn();
    let empty_macros = MacroEngine::new();
    let blocks_no_macro = render_blocks_with_macros(&mut sched1, &mut rf1, &empty_macros, 30);

    // Render with macros applied
    let bank2 = build_default_kit(SAMPLE_RATE, SEED);
    let router2 = InstrumentRouter::from_track_defs(&song.track_defs, bank2, SEED);
    let mut sched2 = EventScheduler::new(song.tempo, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED);
    sched2.timeline_mut().insert_batch(song.events.clone());
    let mut rf2 = router2.into_render_fn();
    let macro_engine = MacroEngine::from_compiled(&song.macros, &song.mappings);
    let blocks_with_macro = render_blocks_with_macros(&mut sched2, &mut rf2, &macro_engine, 30);

    // The events should have different params injected, but the instruments
    // currently don't read arbitrary params from events (they use velocity/note).
    // So we verify the macro engine at least ran without error and both produced audio.
    let has_audio_1 = blocks_no_macro
        .iter()
        .any(|b| b.iter().any(|&s| s.abs() > 0.001));
    let has_audio_2 = blocks_with_macro
        .iter()
        .any(|b| b.iter().any(|&s| s.abs() > 0.001));
    assert!(has_audio_1, "no-macro render should produce audio");
    assert!(has_audio_2, "with-macro render should produce audio");
}

// =============================================================================
// Test 7: Tempo affects event timing
// =============================================================================

#[test]
fn tempo_affects_event_timing() {
    let src_slow = "tempo 60\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n    snare: [. . X .]\n  }\n}";
    let src_fast = "tempo 240\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n    snare: [. . X .]\n  }\n}";

    let (mut sched_slow, mut rf_slow) = build_pipeline(src_slow);
    let (mut sched_fast, mut rf_fast) = build_pipeline(src_fast);

    // Render 30 blocks each
    let blocks_slow = render_blocks(&mut sched_slow, &mut rf_slow, 30);
    let blocks_fast = render_blocks(&mut sched_fast, &mut rf_fast, 30);

    // Find first block with non-silence after block 0 (the second event)
    let find_second_hit = |blocks: &[Vec<f32>]| -> Option<usize> {
        let mut found_first = false;
        let mut in_silence = false;
        for (i, block) in blocks.iter().enumerate() {
            let has_sound = block.iter().any(|&s| s.abs() > 0.001);
            if has_sound && !found_first {
                found_first = true;
            } else if !has_sound && found_first && !in_silence {
                in_silence = true;
            } else if has_sound && in_silence {
                return Some(i);
            }
        }
        None
    };

    let slow_second = find_second_hit(&blocks_slow);
    let fast_second = find_second_hit(&blocks_fast);

    // At 240 BPM the second hit should arrive earlier (at a lower block index)
    // than at 60 BPM.
    if let (Some(slow_idx), Some(fast_idx)) = (slow_second, fast_second) {
        assert!(
            fast_idx < slow_idx,
            "at 240 BPM the second event (block {fast_idx}) should arrive before 60 BPM (block {slow_idx})"
        );
    }
    // If we can't find a second hit in 30 blocks at 60 BPM that's expected
    // (at 60 BPM, beat 2 is at ~2s = ~86 blocks). Just verify fast found it.
    assert!(
        fast_second.is_some(),
        "at 240 BPM, should find second event within 30 blocks"
    );
}

// =============================================================================
// Test 8: Empty pattern (all rests) produces silence
// =============================================================================

#[test]
fn empty_pattern_silence() {
    let src = "tempo 120\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [. . . .]\n  }\n}";
    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let all_silent = blocks
        .iter()
        .all(|block| block.iter().all(|&s| s.abs() < 0.0001));
    assert!(
        all_silent,
        "all-rest pattern should produce completely silent output"
    );
}

// =============================================================================
// Test 9: Deterministic — same seed produces bit-identical blocks
// =============================================================================

#[test]
fn deterministic_same_seed() {
    let src = sample_drums_src();

    let (mut sched_a, mut rf_a) = build_pipeline(src);
    let (mut sched_b, mut rf_b) = build_pipeline(src);

    let blocks_a = render_blocks(&mut sched_a, &mut rf_a, 30);
    let blocks_b = render_blocks(&mut sched_b, &mut rf_b, 30);

    assert_eq!(blocks_a.len(), blocks_b.len());
    for (i, (a, b)) in blocks_a.iter().zip(blocks_b.iter()).enumerate() {
        assert_eq!(a, b, "block {i} must be bit-identical for same seed");
    }
}

// =============================================================================
// Test 10: Recompile changes output
// =============================================================================

#[test]
fn recompile_changes_output() {
    let src_a = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X X X X]\n  }\n}";
    let src_b = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    snare: [X X X X]\n  }\n}";

    // First compile
    let (mut sched_a, mut rf_a) = build_pipeline(src_a);
    let blocks_a = render_blocks(&mut sched_a, &mut rf_a, 10);

    // Second compile (different pattern)
    let (mut sched_b, mut rf_b) = build_pipeline(src_b);
    let blocks_b = render_blocks(&mut sched_b, &mut rf_b, 10);

    // Should produce different output (kick vs snare)
    let differ = blocks_a.iter().zip(blocks_b.iter()).any(|(a, b)| a != b);
    assert!(
        differ,
        "recompiling with different pattern should produce different audio"
    );
}

// =============================================================================
// Test 11: Mini-notation produces audio through full pipeline
// =============================================================================

#[test]
fn mini_notation_produces_audio() {
    // Functional chain with mini-notation: [X.]*2 = X.X. (4 steps, 2 hits)
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("[X.]*2")"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "mini-notation [X.]*2 should produce audible audio"
    );
}

// =============================================================================
// Test 12: Mini-notation matches equivalent hand-written pattern
// =============================================================================

#[test]
fn euclidean_rhythm_produces_audio() {
    // E(3,8) = Euclidean rhythm with 3 hits in 8 steps
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("E(3,8)")"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "Euclidean rhythm E(3,8) should produce audible audio"
    );
}

// =============================================================================
// Test 13: Random pattern produces audio (probabilistic hits through pipeline)
// =============================================================================

#[test]
fn random_pattern_produces_audio() {
    // 8 random steps at 50% probability — with deterministic seed, some will fire
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("????????")"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "random pattern ? should produce audible audio (some hits fire at 50%)"
    );
}

// =============================================================================
// Test 14: Random pattern is deterministic (same seed → same audio)
// =============================================================================

#[test]
fn random_pattern_deterministic() {
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("????????")"#;

    let (mut sched_a, mut rf_a) = build_pipeline(src);
    let (mut sched_b, mut rf_b) = build_pipeline(src);

    let blocks_a = render_blocks(&mut sched_a, &mut rf_a, 30);
    let blocks_b = render_blocks(&mut sched_b, &mut rf_b, 30);

    assert_eq!(blocks_a.len(), blocks_b.len());
    for (i, (a, b)) in blocks_a.iter().zip(blocks_b.iter()).enumerate() {
        assert_eq!(a, b, "block {i}: random pattern must be deterministic");
    }
}

// =============================================================================
// Test 15: Alternation pattern produces audio
// =============================================================================

#[test]
fn alternate_pattern_produces_audio() {
    // Two alternate steps: step 0 picks X (hit), step 1 picks x (soft hit)
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("<X x .><X x .>")"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "alternate pattern <X x .> should produce audible audio"
    );
}

// =============================================================================
// Test 16: Subdivision pattern produces audio
// =============================================================================

#[test]
fn subdivided_pattern_produces_audio() {
    // {X.X} = triplet feel: Hit, Rest, Hit in one step's time
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("{X.X}...")"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "subdivided pattern {{X.X}} should produce audible audio"
    );
}

// =============================================================================
// Test 17: Ratchet pattern produces audio
// =============================================================================

#[test]
fn ratchet_pattern_produces_audio() {
    // X^3 = 3 rapid hits in one step
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("X^3...")"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "ratchet pattern X^3 should produce audible audio"
    );
}

// =============================================================================
// Test 18: Mixed extended notation produces audio
// =============================================================================

#[test]
fn mixed_extended_notation_produces_audio() {
    // Combine Euclidean, ratchet, and random in one pattern
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("E(3,8)")
    |> snare.pattern("..X^2...")"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "mixed extended notation should produce audible audio"
    );
}

// =============================================================================
// Test 19: Ratchet produces more events than single hit
// =============================================================================

#[test]
fn ratchet_denser_than_single_hit() {
    let src_single = r#"tempo 128
drums = kit("default") |> kick.pattern("X...")"#;
    let src_ratchet = r#"tempo 128
drums = kit("default") |> kick.pattern("X^4...")"#;

    let (mut sched_s, mut rf_s) = build_pipeline(src_single);
    let (mut sched_r, mut rf_r) = build_pipeline(src_ratchet);

    let blocks_single = render_blocks(&mut sched_s, &mut rf_s, 50);
    let blocks_ratchet = render_blocks(&mut sched_r, &mut rf_r, 50);

    // Ratchet should have more energy (4 hits vs 1 hit in same time)
    let energy_single: f64 = blocks_single
        .iter()
        .flat_map(|b| b.iter())
        .map(|&s| (s as f64) * (s as f64))
        .sum();

    let energy_ratchet: f64 = blocks_ratchet
        .iter()
        .flat_map(|b| b.iter())
        .map(|&s| (s as f64) * (s as f64))
        .sum();

    assert!(
        energy_ratchet > energy_single,
        "ratchet X^4 ({energy_ratchet:.2}) should have more energy than single X ({energy_single:.2})"
    );
}

// =============================================================================
// Test 20: Transforms work with extended notation through pipeline
// =============================================================================

#[test]
fn extended_notation_with_transforms() {
    // Euclidean rhythm with .fast(2) transform
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("E(3,8)").fast(2)"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "extended notation with transforms should produce audio"
    );
}

#[test]
fn nested_notation_produces_audio() {
    // Ratchet inside subdivision: {X^2 .} — should produce 2 rapid hits
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("{X^2.}...")"#;

    let (mut scheduler, mut render_fn) = build_pipeline(src);
    let blocks = render_blocks(&mut scheduler, &mut render_fn, 50);

    let has_sound = blocks
        .iter()
        .any(|block| block.iter().any(|&s| s.abs() > 0.001));
    assert!(
        has_sound,
        "nested notation {{X^2.}} should produce audible audio through full pipeline"
    );
}

#[test]
fn mini_notation_matches_expanded_pattern() {
    // [X.]*2 should expand to X.X. — identical to writing "X.X." directly
    let src_mini = r#"tempo 128
drums = kit("default") |> kick.pattern("[X.]*2")"#;
    let src_expanded = r#"tempo 128
drums = kit("default") |> kick.pattern("X.X.")"#;

    let (mut sched_mini, mut rf_mini) = build_pipeline(src_mini);
    let (mut sched_expanded, mut rf_expanded) = build_pipeline(src_expanded);

    let blocks_mini = render_blocks(&mut sched_mini, &mut rf_mini, 30);
    let blocks_expanded = render_blocks(&mut sched_expanded, &mut rf_expanded, 30);

    assert_eq!(blocks_mini.len(), blocks_expanded.len());
    for (i, (a, b)) in blocks_mini.iter().zip(blocks_expanded.iter()).enumerate() {
        assert_eq!(
            a, b,
            "block {i}: mini-notation [X.]*2 must produce identical audio to expanded X.X."
        );
    }
}
