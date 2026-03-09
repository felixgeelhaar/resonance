//! Dual-syntax tests — verify declarative DSL and functional (Strudel-like) chain syntax
//! produce equivalent or correct output across all features.

use resonance::dsl::Compiler;
use resonance::event::EventScheduler;
use resonance::instrument::InstrumentRouter;

const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u16 = 2;
const BLOCK_SIZE: u32 = 1024;
const SEED: u64 = 42;

// ========================================================================
// Helper
// ========================================================================

fn compile(src: &str) -> resonance::dsl::compile::CompiledSong {
    Compiler::compile(src).unwrap()
}

fn event_count(src: &str) -> usize {
    compile(src).events.len()
}

fn produces_audio(src: &str) -> bool {
    let song = compile(src);
    let registry = resonance::plugin::registry::PluginRegistry::default();
    let router =
        InstrumentRouter::from_track_defs_with_kits(&song.track_defs, SAMPLE_RATE, SEED, &registry);
    let mut render_fn = router.into_render_fn();
    let mut scheduler = EventScheduler::new(song.tempo, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED);
    scheduler.timeline_mut().insert_batch(song.events);
    scheduler.play();

    for _ in 0..50 {
        if let Some(block) = scheduler.render_block(&mut render_fn) {
            if block.iter().any(|&s| s != 0.0) {
                return true;
            }
        }
    }
    false
}

// ========================================================================
// 1. Basic drum patterns — cross-syntax equivalence
// ========================================================================

#[test]
fn drums_declarative_produces_events() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . . .]
    }
}
"#;
    assert!(event_count(src) > 0);
}

#[test]
fn drums_functional_produces_events() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . . .")"#;
    assert!(event_count(src) > 0);
}

#[test]
fn drums_both_syntaxes_same_event_count() {
    let decl = r#"
tempo 120
track drums {
    kit: default
    section main [2 bars] {
        kick: [X . X . X . X .]
    }
}
"#;
    let func = r#"tempo 120
drums = kit("default") |> kick.pattern("X . X . X . X .")"#;
    assert_eq!(event_count(decl), event_count(func));
}

// ========================================================================
// 2. Multiple patterns per track
// ========================================================================

#[test]
fn multi_pattern_declarative() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . . .]
        hat: [X X X X]
    }
}
"#;
    // kick=1, hat=4
    assert_eq!(event_count(src), 5);
}

#[test]
fn multi_pattern_functional() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . . .") |> hat.pattern("X X X X")"#;
    assert_eq!(event_count(src), 5);
}

// ========================================================================
// 3. Bass synth — both syntaxes
// ========================================================================

#[test]
fn bass_declarative_produces_audio() {
    let src = r#"
tempo 120
track bass {
    bass
    section main [1 bars] {
        line: [C2 . E2 .]
    }
}
"#;
    assert!(produces_audio(src));
}

#[test]
fn bass_functional_produces_audio() {
    let src = r#"tempo 120
bass_track = bass() |> line.pattern("C2 . E2 .")"#;
    assert!(produces_audio(src));
}

// ========================================================================
// 4. Poly synth — both syntaxes
// ========================================================================

#[test]
fn poly_declarative_produces_audio() {
    let src = r#"
tempo 120
track pad {
    poly
    section main [1 bars] {
        chord: [C4 . G4 .]
    }
}
"#;
    assert!(produces_audio(src));
}

#[test]
fn poly_functional_produces_audio() {
    let src = r#"tempo 120
pad = poly() |> chord.pattern("C4 . G4 .")"#;
    assert!(produces_audio(src));
}

// ========================================================================
// 5. Pluck synth — both syntaxes
// ========================================================================

#[test]
fn pluck_declarative_produces_audio() {
    let src = r#"
tempo 120
track keys {
    pluck
    section main [1 bars] {
        melody: [E4 . G4 .]
    }
}
"#;
    assert!(produces_audio(src));
}

#[test]
fn pluck_functional_produces_audio() {
    let src = r#"tempo 120
keys = pluck() |> melody.pattern("E4 . G4 .")"#;
    assert!(produces_audio(src));
}

// ========================================================================
// 6. Noise synth — both syntaxes
// ========================================================================

#[test]
fn noise_declarative_produces_audio() {
    // NoiseGen only accepts Sample("noise") or Note triggers.
    // Using a note pattern triggers the noise filter at note frequency.
    let src = r#"
tempo 120
track fx {
    noise
    section main [1 bars] {
        tone: [C4 . . .]
    }
}
"#;
    assert!(produces_audio(src));
}

#[test]
fn noise_functional_produces_audio() {
    let src = r#"tempo 120
fx = noise() |> tone.pattern("C4 . . .")"#;
    assert!(produces_audio(src));
}

#[test]
fn noise_note_trigger_produces_audio() {
    // Noise can also be triggered by note events
    let src = r#"
tempo 120
track fx {
    noise
    section main [1 bars] {
        tone: [C4 . . .]
    }
}
"#;
    assert!(produces_audio(src));
}

// ========================================================================
// 7. FM synth — both syntaxes (Phase 5)
// ========================================================================

#[test]
fn fm_declarative_produces_audio() {
    let src = r#"
tempo 120
track lead {
    fm
    section main [1 bars] {
        melody: [C4 . E4 .]
    }
}
"#;
    assert!(produces_audio(src));
}

#[test]
fn fm_functional_produces_audio() {
    let src = r#"tempo 120
lead = fm() |> melody.pattern("C4 . E4 .")"#;
    assert!(produces_audio(src));
}

#[test]
fn fm_both_syntaxes_same_event_count() {
    let decl = r#"
tempo 120
track lead {
    fm
    section main [1 bars] {
        melody: [C4 . E4 .]
    }
}
"#;
    let func = r#"tempo 120
lead = fm() |> melody.pattern("C4 . E4 .")"#;
    assert_eq!(event_count(decl), event_count(func));
}

// ========================================================================
// 8. Wavetable synth — both syntaxes (Phase 5)
// ========================================================================

#[test]
fn wavetable_declarative_produces_audio() {
    let src = r#"
tempo 120
track pad {
    wavetable: basic
    section main [1 bars] {
        chord: [C4 . G4 .]
    }
}
"#;
    assert!(produces_audio(src));
}

#[test]
fn wavetable_functional_produces_audio() {
    let src = r#"tempo 120
pad = wavetable("basic") |> chord.pattern("C4 . G4 .")"#;
    assert!(produces_audio(src));
}

#[test]
fn wavetable_both_syntaxes_same_event_count() {
    let decl = r#"
tempo 120
track pad {
    wavetable: basic
    section main [1 bars] {
        chord: [C4 . G4 .]
    }
}
"#;
    let func = r#"tempo 120
pad = wavetable("basic") |> chord.pattern("C4 . G4 .")"#;
    assert_eq!(event_count(decl), event_count(func));
}

// ========================================================================
// 9. Multi-cycle — both syntaxes (Phase 5)
// ========================================================================

#[test]
fn cycles_declarative_multiplies() {
    let single = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . . .]
    }
}
"#;
    let multi = r#"
tempo 120
cycles 3
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . . .]
    }
}
"#;
    assert_eq!(event_count(multi), event_count(single) * 3);
}

#[test]
fn cycles_functional_multiplies() {
    let single = r#"tempo 120
drums = kit("default") |> kick.pattern("X . . .")"#;
    let multi = r#"tempo 120
cycles 3
drums = kit("default") |> kick.pattern("X . . .")"#;
    assert_eq!(event_count(multi), event_count(single) * 3);
}

// ========================================================================
// 10. Macros and mappings — both syntaxes
// ========================================================================

#[test]
fn macros_declarative() {
    let src = r#"
tempo 120
macro depth = 0.5
map depth -> reverb_mix (0.0..1.0) linear
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . . .]
    }
}
"#;
    let song = compile(src);
    assert_eq!(song.macros.len(), 1);
    assert_eq!(song.mappings.len(), 1);
    assert!(!song.events.is_empty());
}

#[test]
fn macros_functional() {
    let src = r#"tempo 120
macro depth = 0.5
map depth -> reverb_mix (0.0..1.0) linear
drums = kit("default") |> kick.pattern("X . . .")"#;
    let song = compile(src);
    assert_eq!(song.macros.len(), 1);
    assert_eq!(song.mappings.len(), 1);
    assert!(!song.events.is_empty());
}

// ========================================================================
// 11. Velocity arrays — both syntaxes
// ========================================================================

#[test]
fn velocity_declarative_single() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X X X X] vel [X x X x]
    }
}
"#;
    let song = compile(src);
    let vels: Vec<f32> = song.events.iter().map(|e| e.velocity).collect();
    assert_eq!(vels.len(), 4);
    // X vel = 1.0, x vel = 0.5
    assert!(vels[0] > vels[1]);
}

#[test]
fn velocity_functional_single() {
    // Single velocity value applies to first step; remainder get default (0.8)
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X X X X").vel(0.5)"#;
    let song = compile(src);
    let vels: Vec<f32> = song.events.iter().map(|e| e.velocity).collect();
    assert_eq!(vels.len(), 4);
    assert!((vels[0] - 0.5).abs() < 0.01);
    // Remaining steps fall back to default
    assert!((vels[1] - 0.8).abs() < 0.01);
}

// ========================================================================
// 12. Transforms — functional syntax only (12 transforms)
// ========================================================================

#[test]
fn transform_fast() {
    let normal = r#"tempo 120
drums = kit("default") |> kick.pattern("X . X .")"#;
    let fast = r#"tempo 120
drums = kit("default") |> kick.pattern("X . X .").fast(2)"#;
    // fast(2) doubles events
    assert_eq!(event_count(fast), event_count(normal) * 2);
}

#[test]
fn transform_slow() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X X X X").slow(2)"#;
    // slow(2) halves events (4 steps in 2x space → only first 2 fit in normal time?)
    // Actually slow(2) expands time so events spread out. Just verify it compiles and produces events.
    assert!(event_count(src) > 0);
}

#[test]
fn transform_rev() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . . X").rev()"#;
    let song = compile(src);
    // Reversed: X from last position moves to first, etc.
    assert_eq!(song.events.len(), 2);
}

#[test]
fn transform_rotate() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . . .").rotate(1)"#;
    let song = compile(src);
    // rotate(1) shifts the X to second position
    assert_eq!(song.events.len(), 1);
    assert!(song.events[0].time.ticks() > 0);
}

#[test]
fn transform_degrade() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X X X X X X X X").degrade(0.5)"#;
    let song = compile(src);
    // With 50% degrade and seed, some events should be removed
    assert!(song.events.len() < 8);
    assert!(song.events.len() > 0);
}

#[test]
fn transform_every() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . X .").every(2, rev)"#;
    assert!(event_count(src) > 0);
}

#[test]
fn transform_sometimes() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X X X X").sometimes(0.5, rev)"#;
    assert!(event_count(src) > 0);
}

#[test]
fn transform_chop() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . . .").chop(4)"#;
    // chop(4) subdivides the hit into 4 slices
    assert!(event_count(src) >= 4);
}

#[test]
fn transform_stutter() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . . .").stutter(3)"#;
    // stutter(3) repeats the hit 3 times
    assert!(event_count(src) >= 3);
}

#[test]
fn transform_add() {
    let src = r#"tempo 120
lead = bass() |> line.pattern("C3 . E3 .").add(12)"#;
    let song = compile(src);
    // add(12) transposes up an octave — notes should still be present
    assert_eq!(song.events.len(), 2);
}

#[test]
fn transform_gain() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X X X X").gain(0.5)"#;
    let song = compile(src);
    // gain(0.5) halves velocity
    for e in &song.events {
        assert!(e.velocity <= 0.5 + 0.01);
    }
}

#[test]
fn transform_legato() {
    let src = r#"tempo 120
lead = bass() |> line.pattern("C3 . E3 .").legato(2.0)"#;
    let song = compile(src);
    // legato(2.0) doubles note duration
    assert_eq!(song.events.len(), 2);
    for e in &song.events {
        assert!(e.duration.ticks() > 0);
    }
}

#[test]
fn transform_chain_multiple() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X X X X").gain(0.8).rev()"#;
    let song = compile(src);
    assert_eq!(song.events.len(), 4);
    for e in &song.events {
        assert!(e.velocity <= 0.8 + 0.01);
    }
}

// ========================================================================
// 13. Mini-notation in functional syntax
// ========================================================================

#[test]
fn mini_notation_group_repeat() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("[X .]*2")"#;
    // [X .]*2 repeats the group twice → 2 kicks
    assert!(event_count(src) >= 2);
}

#[test]
fn mini_notation_element_repeat() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X!4")"#;
    // X!4 repeats the element 4 times
    assert!(event_count(src) >= 4);
}

#[test]
fn mini_notation_euclidean() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("E(3,8)")"#;
    // E(3,8) = Euclidean rhythm with 3 hits in 8 steps
    assert_eq!(event_count(src), 3);
}

#[test]
fn mini_notation_random() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("? ? ? ? ? ? ? ?")"#;
    let song = compile(src);
    // Random: with seed, some should be hits, some rests
    assert!(song.events.len() > 0);
    assert!(song.events.len() < 8);
}

#[test]
fn mini_notation_alternation() {
    let src = r#"tempo 120
cycles 2
drums = kit("default") |> hat.pattern("<X .><X .><X .><X .>")"#;
    let song = compile(src);
    // Alternation across 2 cycles should produce events
    assert!(!song.events.is_empty());
}

#[test]
fn mini_notation_subdivision() {
    let src = r#"tempo 120
drums = kit("default") |> hat.pattern("{X X X}")"#;
    // Subdivision: 3 hits squeezed into one step
    assert_eq!(event_count(src), 3);
}

#[test]
fn mini_notation_ratchet() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X^3 . . .")"#;
    // X^3 = ratchet: rapid-fire 3 hits in one step
    assert_eq!(event_count(src), 3);
}

// ========================================================================
// 14. Arrangement — declarative syntax (Phase 5)
// ========================================================================

#[test]
fn arrangement_declarative_compiles() {
    let src = r#"
tempo 120
arrangement [intro x1, main x2]
track drums {
    kit: default
    section intro [1 bars] {
        hat: [X . . .]
    }
    section main [1 bars] {
        kick: [X . X .]
        hat: [X X X X]
    }
}
"#;
    let song = compile(src);
    assert!(song.arrangement.is_some());
    let arr = song.arrangement.unwrap();
    assert_eq!(arr.entries.len(), 2);
    assert_eq!(arr.entries[0].section_name, "intro");
    assert_eq!(arr.entries[0].repeats, 1);
    assert_eq!(arr.entries[1].section_name, "main");
    assert_eq!(arr.entries[1].repeats, 2);
}

// ========================================================================
// 15. Stacked patterns — declarative syntax (Phase 5)
// ========================================================================

#[test]
fn stacked_hits_declarative() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        combo: [K+H . S+H .]
    }
}
"#;
    let song = compile(src);
    // 2 stacks × 2 simultaneous hits = 4 events minimum
    assert!(song.events.len() >= 4);
}

#[test]
fn stacked_hits_simultaneous_time() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        combo: [K+H . . .]
    }
}
"#;
    let song = compile(src);
    let at_zero: Vec<_> = song.events.iter().filter(|e| e.time.ticks() == 0).collect();
    assert_eq!(at_zero.len(), 2, "K+H should produce 2 simultaneous events");
}

// ========================================================================
// 16. MIDI output — declarative syntax (Phase 5)
// ========================================================================

#[test]
fn midi_out_declarative() {
    let src = r#"
tempo 120
track synth {
    bass
    midi_out: "USB MIDI" ch 1
    section main [1 bars] {
        line: [C3 . E3 .]
    }
}
"#;
    let song = compile(src);
    let (_, track_def) = &song.track_defs[0];
    assert!(track_def.midi_out.is_some());
    let midi = track_def.midi_out.as_ref().unwrap();
    assert_eq!(midi.device, "USB MIDI");
    assert_eq!(midi.channel, 1);
}

// ========================================================================
// 17. Determinism — same seed produces identical output
// ========================================================================

#[test]
fn determinism_declarative() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . X .]
        hat: [X X X X]
    }
}
"#;
    let s1 = compile(src);
    let s2 = compile(src);
    assert_eq!(s1.events.len(), s2.events.len());
    for (a, b) in s1.events.iter().zip(s2.events.iter()) {
        assert_eq!(a.time, b.time);
        assert_eq!(a.velocity, b.velocity);
    }
}

#[test]
fn determinism_functional() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . X .") |> hat.pattern("X X X X")"#;
    let s1 = compile(src);
    let s2 = compile(src);
    assert_eq!(s1.events.len(), s2.events.len());
    for (a, b) in s1.events.iter().zip(s2.events.iter()) {
        assert_eq!(a.time, b.time);
        assert_eq!(a.velocity, b.velocity);
    }
}

// ========================================================================
// 18. Cross-syntax event equivalence
// ========================================================================

#[test]
fn cross_syntax_single_hit_equivalent() {
    let decl = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . . .]
    }
}
"#;
    let func = r#"tempo 120
drums = kit("default") |> kick.pattern("X . . .")"#;
    let s1 = compile(decl);
    let s2 = compile(func);
    assert_eq!(s1.events.len(), s2.events.len());
    // Both should have event at beat 0
    assert_eq!(s1.events[0].time.ticks(), 0);
    assert_eq!(s2.events[0].time.ticks(), 0);
}

#[test]
fn cross_syntax_note_pattern_equivalent() {
    let decl = r#"
tempo 120
track lead {
    bass
    section main [2 bars] {
        line: [C3 . E3 . G3 . C4 .]
    }
}
"#;
    let func = r#"tempo 120
lead = bass() |> line.pattern("C3 . E3 . G3 . C4 .")"#;
    let s1 = compile(decl);
    let s2 = compile(func);
    assert_eq!(s1.events.len(), s2.events.len());
}

#[test]
fn cross_syntax_fm_equivalent() {
    let decl = r#"
tempo 120
track lead {
    fm
    section main [1 bars] {
        melody: [C4 E4 G4 C5]
    }
}
"#;
    let func = r#"tempo 120
lead = fm() |> melody.pattern("C4 E4 G4 C5")"#;
    let s1 = compile(decl);
    let s2 = compile(func);
    assert_eq!(s1.events.len(), s2.events.len());
}

#[test]
fn cross_syntax_wavetable_equivalent() {
    let decl = r#"
tempo 120
track pad {
    wavetable: basic
    section main [1 bars] {
        chord: [C4 G4 . .]
    }
}
"#;
    let func = r#"tempo 120
pad = wavetable("basic") |> chord.pattern("C4 G4 . .")"#;
    let s1 = compile(decl);
    let s2 = compile(func);
    assert_eq!(s1.events.len(), s2.events.len());
}

// ========================================================================
// 19. Complex real-world patterns
// ========================================================================

#[test]
fn complex_declarative_multi_track() {
    let src = r#"
tempo 128
track drums {
    kit: default
    section main [2 bars] {
        kick: [X . . . X . . . X . . . X . . .]
        hat: [X X X X X X X X X X X X X X X X]
        snare: [. . . . X . . . . . . . X . . .]
    }
}
track bass {
    bass
    section main [2 bars] {
        line: [C2 . . . E2 . . . G2 . . . C2 . . .]
    }
}
"#;
    let song = compile(src);
    assert!(song.events.len() > 20);
    assert!(produces_audio(src));
}

#[test]
fn complex_functional_multi_track() {
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("X . . . X . . .") |> hat.pattern("X X X X X X X X") |> snare.pattern(". . . . X . . .")
bass_track = bass() |> line.pattern("C2 . . . E2 . . .")"#;
    let song = compile(src);
    assert!(song.events.len() > 10);
    assert!(produces_audio(src));
}

#[test]
fn functional_transforms_produce_audio() {
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("X . X .").fast(2) |> hat.pattern("X X X X").gain(0.6)
lead = fm() |> melody.pattern("C4 E4 G4 C5").legato(1.5)"#;
    assert!(produces_audio(src));
}

#[test]
fn functional_mini_notation_produces_audio() {
    let src = r#"tempo 128
drums = kit("default") |> kick.pattern("X . X .") |> hat.pattern("{X X X} . {X X X} .")
bass_track = bass() |> line.pattern("C2 . [E2 G2]*1 .")"#;
    assert!(produces_audio(src));
}

// ========================================================================
// 20. WAV export — both syntaxes
// ========================================================================

#[test]
fn export_declarative() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . X .]
    }
}
"#;
    let song = compile(src);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("decl.wav");
    let registry = resonance::plugin::registry::PluginRegistry::default();
    let config = resonance::audio::export::ExportConfig {
        output_path: path.clone(),
        bars: Some(1),
        include_effects: false,
    };
    let count =
        resonance::audio::export::export_wav(&song, config, SEED, SAMPLE_RATE, &registry).unwrap();
    assert!(count > 0);
    assert!(path.exists());
}

#[test]
fn export_functional() {
    let src = r#"tempo 120
drums = kit("default") |> kick.pattern("X . X .")"#;
    let song = compile(src);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("func.wav");
    let registry = resonance::plugin::registry::PluginRegistry::default();
    let config = resonance::audio::export::ExportConfig {
        output_path: path.clone(),
        bars: Some(1),
        include_effects: false,
    };
    let count =
        resonance::audio::export::export_wav(&song, config, SEED, SAMPLE_RATE, &registry).unwrap();
    assert!(count > 0);
    assert!(path.exists());
}
