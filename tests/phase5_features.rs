//! Phase 5 integration tests — stacked patterns, cycles, FM/wavetable, export, arrangement.

use resonance::dsl::Compiler;
use resonance::event::EventScheduler;
use resonance::instrument::InstrumentRouter;

const SAMPLE_RATE: u32 = 44100;
const CHANNELS: u16 = 2;
const BLOCK_SIZE: u32 = 1024;
const SEED: u64 = 42;

// ---- Polyphonic Drum Patterns ----

#[test]
fn stacked_pattern_compiles() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        combo: [K+H . S+H .]
    }
}
"#;
    let song = Compiler::compile(src).unwrap();
    // Stacked hits produce multiple events at same time
    assert!(song.events.len() >= 4); // at least 4 events: K, H, S, H
}

#[test]
fn stacked_pattern_events_are_simultaneous() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        combo: [K+H . . .]
    }
}
"#;
    let song = Compiler::compile(src).unwrap();
    // First step should produce 2 events at beat 0
    let beat_zero_events: Vec<_> = song.events.iter().filter(|e| e.time.ticks() == 0).collect();
    assert_eq!(
        beat_zero_events.len(),
        2,
        "K+H should produce 2 events at beat 0"
    );
}

// ---- Multi-Cycle Pattern Playback ----

#[test]
fn cycles_directive_multiplies_events() {
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
    let song1 = Compiler::compile(single).unwrap();
    let song3 = Compiler::compile(multi).unwrap();
    assert_eq!(song3.events.len(), song1.events.len() * 3);
}

#[test]
fn cycles_alternation_varies_per_cycle() {
    // Use functional chain syntax for mini-notation alternation
    let src = r#"tempo 120
cycles 2
drums = kit("default") |> hat.pattern("<X .><X .><X .><X .>")"#;
    let song = Compiler::compile(src).unwrap();
    // Alternation should produce events across 2 cycles
    assert!(!song.events.is_empty());
}

// ---- Custom Synthesis Engines ----

#[test]
fn fm_synth_compiles_and_renders() {
    let src = r#"
tempo 120
track lead {
    fm
    section main [1 bars] {
        melody: [C4 . E4 .]
    }
}
"#;
    let song = Compiler::compile(src).unwrap();
    assert!(!song.events.is_empty());

    let plugin_registry = resonance::plugin::registry::PluginRegistry::default();
    let router = InstrumentRouter::from_track_defs_with_kits(
        &song.track_defs,
        SAMPLE_RATE,
        SEED,
        &plugin_registry,
    );
    let mut render_fn = router.into_render_fn();
    let mut scheduler = EventScheduler::new(song.tempo, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED);
    scheduler.timeline_mut().insert_batch(song.events);
    scheduler.play();

    let mut has_audio = false;
    for _ in 0..30 {
        if let Some(block) = scheduler.render_block(&mut render_fn) {
            if block.iter().any(|&s| s != 0.0) {
                has_audio = true;
                break;
            }
        }
    }
    assert!(has_audio, "FM synth should produce audio");
}

#[test]
fn wavetable_synth_compiles_and_renders() {
    let src = r#"
tempo 120
track pad {
    wavetable: basic
    section main [1 bars] {
        chord: [C4 . G4 .]
    }
}
"#;
    let song = Compiler::compile(src).unwrap();
    assert!(!song.events.is_empty());

    let plugin_registry = resonance::plugin::registry::PluginRegistry::default();
    let router = InstrumentRouter::from_track_defs_with_kits(
        &song.track_defs,
        SAMPLE_RATE,
        SEED,
        &plugin_registry,
    );
    let mut render_fn = router.into_render_fn();
    let mut scheduler = EventScheduler::new(song.tempo, SAMPLE_RATE, CHANNELS, BLOCK_SIZE, SEED);
    scheduler.timeline_mut().insert_batch(song.events);
    scheduler.play();

    let mut has_audio = false;
    for _ in 0..30 {
        if let Some(block) = scheduler.render_block(&mut render_fn) {
            if block.iter().any(|&s| s != 0.0) {
                has_audio = true;
                break;
            }
        }
    }
    assert!(has_audio, "Wavetable synth should produce audio");
}

// ---- WAV Export ----

#[test]
fn export_via_api() {
    let src = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . X .]
    }
}
"#;
    let song = Compiler::compile(src).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.wav");
    let registry = resonance::plugin::registry::PluginRegistry::default();
    let config = resonance::audio::export::ExportConfig {
        output_path: path.clone(),
        bars: Some(1),
        include_effects: false,
    };
    let count = resonance::audio::export::export_wav(&song, config, 42, 44100, &registry).unwrap();
    assert!(count > 0);
    assert!(path.exists());

    // Verify WAV is valid
    let reader = hound::WavReader::open(&path).unwrap();
    assert_eq!(reader.spec().channels, 2);
}

// ---- Arrangement ----

#[test]
fn arrangement_parses_and_compiles() {
    let src = r#"
tempo 120
arrangement [intro x1, verse x2, chorus x1]
track drums {
    kit: default
    section intro [2 bars] {
        kick: [X . . .]
    }
    section verse [4 bars] {
        kick: [X . X . X . X . X . X . X . X .]
    }
    section chorus [4 bars] {
        kick: [X X X X X X X X X X X X X X X X]
    }
}
"#;
    let song = Compiler::compile(src).unwrap();
    assert!(song.arrangement.is_some());
    let arr = song.arrangement.unwrap();
    assert_eq!(arr.entries.len(), 3);
    assert_eq!(arr.entries[0].section_name, "intro");
    assert_eq!(arr.entries[0].repeats, 1);
    assert_eq!(arr.entries[1].section_name, "verse");
    assert_eq!(arr.entries[1].repeats, 2);
    assert_eq!(arr.entries[2].section_name, "chorus");
    assert_eq!(arr.entries[2].repeats, 1);
}

#[test]
fn arrangement_controller_status() {
    use resonance::dsl::ast::ArrangementEntry;
    use resonance::section::ArrangementController;

    let entries = vec![
        ArrangementEntry {
            section_name: "intro".to_string(),
            repeats: 1,
        },
        ArrangementEntry {
            section_name: "verse".to_string(),
            repeats: 2,
        },
    ];
    let ctrl = ArrangementController::new(entries);
    assert_eq!(ctrl.current_section(), Some("intro"));
    assert_eq!(ctrl.status_string(), "[1/2] intro (rep 1/1)");
}

// ---- MIDI Output Definition ----

#[test]
fn midi_out_definition_parses() {
    let src = r#"
tempo 120
track lead {
    bass
    midi_out: "USB MIDI" ch 1
    section main [1 bars] {
        line: [C3 . E3 .]
    }
}
"#;
    let song = Compiler::compile(src).unwrap();
    let (_, track_def) = &song.track_defs[0];
    assert!(track_def.midi_out.is_some());
    let midi_out = track_def.midi_out.as_ref().unwrap();
    assert_eq!(midi_out.device, "USB MIDI");
    assert_eq!(midi_out.channel, 1);
}

// ---- Timeline Loop Point ----

#[test]
fn timeline_loop_point_api() {
    use resonance::event::{Beat, Timeline};

    let mut tl = Timeline::new();
    assert!(tl.loop_point().is_none());
    tl.set_loop_point(Beat::from_bars(4));
    assert_eq!(tl.loop_point(), Some(Beat::from_bars(4)));
    tl.clear_loop_point();
    assert!(tl.loop_point().is_none());
}
