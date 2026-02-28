//! Phase 2 integration tests — full lifecycle with overrides, layers, intents, and conflict resolution.

use resonance::dsl::ast::CurveKind;
use resonance::dsl::compile::compile_program;
use resonance::dsl::Compiler;
use resonance::event::types::ParamId;
use resonance::event::Beat;
use resonance::intent::{IntentMode, StructuralIntentProcessor};
use resonance::macro_engine::resolver::{resolve_mappings, MappingSource, SourcedMapping};
use resonance::macro_engine::{MacroEngine, Mapping};
use resonance::section::{Layer, Section, SectionController};

/// Full song lifecycle: parse, compile, populate controller, jump sections, toggle layers.
#[test]
fn full_song_lifecycle() {
    let src = r#"
tempo 140
macro filter = 0.5
map filter -> cutoff (200.0..8000.0) exp
layer reverb_wash {
    depth -> reverb_mix (0.0..0.8) smoothstep
}
track drums {
    kit: default
    section intro [2 bars] {
        kick: [X . . . X . . .]
    }
    section verse [4 bars] {
        kick: [X . X . X . X .]
        override filter -> cutoff (0.2..0.6) linear
    }
}
track bass {
    bass
    section intro [2 bars] {
        note: [C2 . . . . . . .]
    }
    section verse [4 bars] {
        note: [C2 . Eb2 . C2 . G1 .]
    }
}
"#;
    let program = Compiler::parse(src).unwrap();
    let song = compile_program(&program).unwrap();

    // Verify compilation
    assert!((song.tempo - 140.0).abs() < f64::EPSILON);
    assert_eq!(song.track_defs.len(), 2);
    assert_eq!(song.sections.len(), 2); // intro, verse (deduplicated)
    assert_eq!(song.layers.len(), 1);
    assert!(!song.events.is_empty());

    // Populate section controller
    let sections: Vec<Section> = song
        .sections
        .iter()
        .map(|cs| Section {
            name: cs.name.clone(),
            length_in_bars: cs.length_in_bars,
            mapping_overrides: cs
                .mapping_overrides
                .iter()
                .map(|o| Mapping {
                    macro_name: o.macro_name.clone(),
                    target_param: ParamId(o.target_param.clone()),
                    range: o.range,
                    curve: o.curve,
                })
                .collect(),
        })
        .collect();

    let mut controller = SectionController::new(sections);

    // Add layer from compiled song
    for layer_def in &song.layers {
        controller.add_layer(Layer {
            name: layer_def.name.clone(),
            mapping_additions: layer_def
                .mappings
                .iter()
                .map(|m| Mapping {
                    macro_name: m.macro_name.clone(),
                    target_param: ParamId(m.target_param.clone()),
                    range: m.range,
                    curve: m.curve,
                })
                .collect(),
            enabled: layer_def.enabled_by_default,
        });
    }

    // Intro section: no overrides, no enabled layers
    assert_eq!(controller.active_section().unwrap().name, "intro");
    assert!(controller.active_mappings().is_empty());

    // Transition to verse
    controller.schedule_transition("verse", Beat::from_beats(2));
    controller.update(Beat::from_bars(1));
    assert_eq!(controller.active_section().unwrap().name, "verse");
    assert_eq!(controller.active_mappings().len(), 1); // section override

    // Toggle layer on
    assert!(controller.toggle_layer("reverb_wash"));
    assert_eq!(controller.active_mappings().len(), 2); // section + layer

    // Toggle layer off
    assert!(controller.toggle_layer("reverb_wash"));
    assert_eq!(controller.active_mappings().len(), 1); // section only
}

/// Structural intent: propose → accept → reject lifecycle.
#[test]
fn structural_intent_lifecycle() {
    let src_old = r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] { kick: [X . . .] }
}
"#;
    let src_new = r#"
tempo 140
track drums {
    kit: default
    section main [1 bars] { kick: [X . X .] }
}
track bass {
    bass
    section main [1 bars] { note: [C2 . . .] }
}
"#;

    let old = Compiler::parse(src_old).unwrap();
    let new = Compiler::parse(src_new).unwrap();

    let diff = resonance::dsl::diff::AstDiff::diff(&old, &new);
    assert!(!diff.is_empty());
    assert!(!diff.is_performance_safe()); // structural changes

    let mode = resonance::intent::detect_mode(&diff);
    assert_eq!(mode, IntentMode::Structural);

    // Propose
    let mut processor = StructuralIntentProcessor::new();
    processor.propose(
        "Add bass and change tempo".to_string(),
        diff.clone(),
        src_new.to_string(),
    );
    assert!(processor.pending().is_some());

    // Accept
    let accepted_diff = processor.accept().unwrap();
    assert!(!accepted_diff.is_empty());

    // Apply to old program
    let result = accepted_diff.apply(&old).unwrap();
    assert!((result.tempo - 140.0).abs() < f64::EPSILON);
    assert_eq!(result.tracks.len(), 2);
}

/// Mapping conflict resolution: base < section < layer precedence.
#[test]
fn mapping_conflict_resolution_e2e() {
    let base = SourcedMapping {
        mapping: Mapping {
            macro_name: "filter".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        },
        source: MappingSource::Base,
    };
    let section = SourcedMapping {
        mapping: Mapping {
            macro_name: "filter_section".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (0.2, 0.6),
            curve: CurveKind::Linear,
        },
        source: MappingSource::SectionOverride,
    };
    let layer = SourcedMapping {
        mapping: Mapping {
            macro_name: "filter_layer".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (0.4, 0.8),
            curve: CurveKind::Exp,
        },
        source: MappingSource::Layer(0),
    };

    let sourced = vec![base, section, layer];
    let (winners, conflicts) = resolve_mappings(&sourced);

    assert_eq!(winners.len(), 1);
    assert_eq!(winners[0].source, MappingSource::Layer(0));
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].sources.len(), 3);
}

/// Live performance simulation: macro engine + section controller + layers.
#[test]
fn live_performance_simulation() {
    let mut engine = MacroEngine::new();
    engine.add_macro("filter", 0.5);
    engine.add_mapping(Mapping {
        macro_name: "filter".to_string(),
        target_param: ParamId("cutoff".to_string()),
        range: (200.0, 8000.0),
        curve: CurveKind::Exp,
    });

    let sections = vec![
        Section {
            name: "verse".to_string(),
            length_in_bars: 4,
            mapping_overrides: vec![Mapping {
                macro_name: "filter".to_string(),
                target_param: ParamId("cutoff".to_string()),
                range: (200.0, 2000.0),
                curve: CurveKind::Linear,
            }],
        },
        Section {
            name: "chorus".to_string(),
            length_in_bars: 4,
            mapping_overrides: vec![],
        },
    ];

    let mut controller = SectionController::new(sections);

    // Verse has section override for cutoff
    assert_eq!(controller.active_mappings().len(), 1);

    // Jump to chorus — no overrides
    controller.schedule_transition("chorus", Beat::ZERO);
    controller.update(Beat::from_bars(1));
    assert!(controller.active_mappings().is_empty());

    // Resolve params from base engine
    let params = engine.resolve_params();
    assert!(params.contains_key(&ParamId("cutoff".to_string())));
}

/// Backward compatibility: all original patterns still parse and compile.
#[test]
fn backward_compat_basic_drum_pattern() {
    let src = r#"
tempo 128
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . . X . . .]
    snare: [. . X . . . X .]
    hat: [x x x x x x x x]
  }
}
"#;
    let program = Compiler::parse(src).unwrap();
    let song = compile_program(&program).unwrap();
    assert!((song.tempo - 128.0).abs() < f64::EPSILON);
    assert!(song.layers.is_empty());
    assert!(!song.events.is_empty());
}

/// Backward compatibility: functional chain syntax still works.
#[test]
fn backward_compat_functional_chain() {
    let src = r#"drums = kit("default") |> kick.pattern("X..x")"#;
    let program = Compiler::parse(src).unwrap();
    let song = compile_program(&program).unwrap();
    assert_eq!(song.track_defs.len(), 1);
}

/// Determinism: same source produces identical compiled output.
#[test]
fn determinism_compile_identical() {
    let src = r#"
tempo 130
macro filter = 0.5
map filter -> cutoff (0.0..1.0) linear
layer fx { filter -> drive (0.0..10.0) exp }
track drums {
    kit: default
    section main [2 bars] {
        kick: [X . X . X . X .]
        override filter -> cutoff (0.2..0.8) smoothstep
    }
}
"#;
    let song1 = compile_program(&Compiler::parse(src).unwrap()).unwrap();
    let song2 = compile_program(&Compiler::parse(src).unwrap()).unwrap();

    assert_eq!(song1.events.len(), song2.events.len());
    for (a, b) in song1.events.iter().zip(song2.events.iter()) {
        assert_eq!(a.time, b.time);
        assert_eq!(a.trigger, b.trigger);
        assert!((a.velocity - b.velocity).abs() < f32::EPSILON);
    }
}
