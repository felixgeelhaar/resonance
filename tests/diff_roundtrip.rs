//! AST diff round-trip tests — verify diff then apply produces the expected program.

use resonance::dsl::ast::*;
use resonance::dsl::diff::AstDiff;
use resonance::dsl::Compiler;

fn base_program() -> Program {
    Compiler::parse(
        r#"
tempo 120
macro filter = 0.5
map filter -> cutoff (0.0..1.0) linear
track drums {
    kit: default
    section main [2 bars] {
        kick: [X . . . X . . .]
    }
}
"#,
    )
    .unwrap()
}

/// Diff then apply produces the target program.
#[test]
fn diff_apply_produces_target() {
    let old = base_program();
    let new_src = r#"
tempo 140
macro filter = 0.8
map filter -> cutoff (0.0..1.0) linear
track drums {
    kit: default
    section main [2 bars] {
        kick: [X . X . X . X .]
    }
}
track bass {
    bass
    section main [2 bars] {
        note: [C2 . . . . . . .]
    }
}
"#;
    let new = Compiler::parse(new_src).unwrap();
    let diff = AstDiff::diff(&old, &new);
    assert!(!diff.is_empty());

    let applied = diff.apply(&old).unwrap();
    assert!((applied.tempo - 140.0).abs() < f64::EPSILON);
    assert_eq!(applied.tracks.len(), 2);
    assert_eq!(applied.macros[0].default_value, 0.8);
}

/// Round-trip: old → diff(old, new) → apply → diff(result, new) → empty.
#[test]
fn round_trip_diff_apply_diff_empty() {
    let old = base_program();
    let new = Compiler::parse(
        r#"
tempo 130
macro filter = 0.5
map filter -> cutoff (0.0..1.0) linear
track drums {
    kit: default
    section main [2 bars] {
        kick: [X . . . X . . .]
    }
}
"#,
    )
    .unwrap();

    let diff = AstDiff::diff(&old, &new);
    let applied = diff.apply(&old).unwrap();
    let second_diff = AstDiff::diff(&applied, &new);
    assert!(second_diff.is_empty());
}

/// Identical programs produce empty diff.
#[test]
fn identical_programs_empty_diff() {
    let a = base_program();
    let b = base_program();
    let diff = AstDiff::diff(&a, &b);
    assert!(diff.is_empty());
}

/// Performance-safe changes: only tempo + macro changes.
#[test]
fn performance_safe_diff() {
    let old = base_program();
    let mut new = old.clone();
    new.tempo = 135.0;
    new.macros[0].default_value = 0.7;

    let diff = AstDiff::diff(&old, &new);
    assert!(diff.is_performance_safe());
}

/// Structural changes: adding/removing tracks.
#[test]
fn structural_diff() {
    let old = base_program();
    let mut new = old.clone();
    new.tracks.push(TrackDef {
        name: "bass".to_string(),
        instrument: InstrumentRef::Bass,
        sections: vec![],
    });

    let diff = AstDiff::diff(&old, &new);
    assert!(!diff.is_performance_safe());
}

/// Summaries are generated for each change.
#[test]
fn diff_summaries() {
    let old = base_program();
    let mut new = old.clone();
    new.tempo = 140.0;
    new.tracks.push(TrackDef {
        name: "bass".to_string(),
        instrument: InstrumentRef::Bass,
        sections: vec![],
    });

    let diff = AstDiff::diff(&old, &new);
    let summaries = diff.summaries();
    assert!(summaries.len() >= 2);
    // Should contain descriptions of tempo change and track addition
    assert!(summaries
        .iter()
        .any(|s| s.contains("tempo") || s.contains("Tempo")));
    assert!(summaries
        .iter()
        .any(|s| s.contains("bass") || s.contains("track")));
}

/// Multiple changes applied together.
#[test]
fn complex_diff_round_trip() {
    let old = base_program();
    let new = Compiler::parse(
        r#"
tempo 145
macro filter = 0.3
macro depth = 0.7
map filter -> cutoff (100.0..5000.0) exp
map depth -> reverb_mix (0.0..1.0) smoothstep
track drums {
    kit: default
    section main [4 bars] {
        kick: [X . X . X . X . X . X . X . X .]
    }
    section fill [1 bars] {
        kick: [X X X X]
    }
}
track bass {
    bass
    section main [4 bars] {
        note: [C2 . Eb2 . G2 . Bb2 .]
    }
}
"#,
    )
    .unwrap();

    let diff = AstDiff::diff(&old, &new);
    let applied = diff.apply(&old).unwrap();

    // Verify key properties
    assert!((applied.tempo - 145.0).abs() < f64::EPSILON);
    assert_eq!(applied.tracks.len(), 2);
    assert_eq!(applied.macros.len(), 2);
    assert_eq!(applied.mappings.len(), 2);
}
