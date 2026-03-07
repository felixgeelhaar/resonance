//! Pattern engine — compiles AST into events.
//!
//! Transforms a [`Program`] AST into a [`CompiledSong`] containing
//! tempo, events, track definitions, macros, and mappings.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::event::types::{Event, TrackId};
use crate::event::Beat;

use super::ast::*;
use super::error::CompileError;
use super::note::parse_note_name;

/// A compiled section with its mapping overrides.
#[derive(Debug, Clone)]
pub struct CompiledSection {
    pub name: String,
    pub length_in_bars: u32,
    pub mapping_overrides: Vec<MappingOverrideDef>,
}

/// The result of compiling a DSL program.
#[derive(Debug, Clone)]
pub struct CompiledSong {
    pub tempo: f64,
    pub events: Vec<Event>,
    pub track_defs: Vec<(TrackId, TrackDef)>,
    pub macros: Vec<MacroDef>,
    pub mappings: Vec<MappingDef>,
    pub sections: Vec<CompiledSection>,
    pub layers: Vec<LayerDef>,
}

/// Compile a Program AST into a CompiledSong.
pub fn compile_program(program: &Program) -> Result<CompiledSong, CompileError> {
    let mut events = Vec::new();
    let mut track_defs = Vec::new();
    let mut sections = Vec::new();

    for (idx, track) in program.tracks.iter().enumerate() {
        let track_id = TrackId(idx as u32);
        track_defs.push((track_id, track.clone()));

        let is_drum = matches!(track.instrument, InstrumentRef::Kit(_));

        let mut section_offset = Beat::ZERO;

        for section in &track.sections {
            let section_events = compile_section(section, track_id, is_drum, section_offset)?;
            events.extend(section_events);

            // Collect compiled sections (deduplicate by name)
            if !sections
                .iter()
                .any(|s: &CompiledSection| s.name == section.name)
            {
                sections.push(CompiledSection {
                    name: section.name.clone(),
                    length_in_bars: section.length_bars,
                    mapping_overrides: section.overrides.clone(),
                });
            }

            section_offset = section_offset + Beat::from_bars(section.length_bars);
        }
    }

    // Sort events by time
    events.sort_by(|a, b| a.time.cmp(&b.time));

    Ok(CompiledSong {
        tempo: program.tempo,
        events,
        track_defs,
        macros: program.macros.clone(),
        mappings: program.mappings.clone(),
        sections,
        layers: program.layers.clone(),
    })
}

fn compile_section(
    section: &SectionDef,
    track_id: TrackId,
    is_drum: bool,
    offset: Beat,
) -> Result<Vec<Event>, CompileError> {
    let mut events = Vec::new();

    for pattern in &section.patterns {
        let pattern_events =
            compile_pattern(pattern, track_id, is_drum, offset, section.length_bars)?;
        events.extend(pattern_events);
    }

    Ok(events)
}

fn compile_pattern(
    pattern: &PatternDef,
    track_id: TrackId,
    is_drum: bool,
    offset: Beat,
    length_bars: u32,
) -> Result<Vec<Event>, CompileError> {
    // Apply pre-event transforms (modify steps/velocities before event generation)
    let mut steps = pattern.steps.clone();
    let mut velocities = pattern.velocities.clone();
    let seed = compute_pattern_seed(&pattern.target, &steps);

    for transform in &pattern.transforms {
        apply_step_transform(&mut steps, &mut velocities, transform, seed);
    }

    let mut events = Vec::new();
    let num_steps = steps.len();
    if num_steps == 0 {
        return Ok(events);
    }

    // Total beats in this section
    let total_beats = length_bars as f64 * 4.0;
    let step_duration_beats = total_beats / num_steps as f64;

    for (i, step) in steps.iter().enumerate() {
        let time_beats = i as f64 * step_duration_beats;
        let time = offset + Beat::from_beats_f64(time_beats);
        let duration = Beat::from_beats_f64(step_duration_beats);

        let velocity = if let Some(ref vels) = velocities {
            if i < vels.len() {
                vels[i] as f32
            } else {
                0.8
            }
        } else {
            match step {
                Step::Hit => 0.85,
                Step::Accent(v) => *v as f32,
                Step::Note(_) => 0.8,
                Step::Rest => continue,
            }
        };

        if velocity <= 0.0 {
            continue;
        }

        match step {
            Step::Hit => {
                if is_drum {
                    events.push(Event::sample(
                        time,
                        duration,
                        track_id,
                        &pattern.target,
                        velocity,
                    ));
                } else {
                    // For non-drum instruments, Hit defaults to the pattern target as note
                    if let Some(midi) = parse_note_name(&pattern.target) {
                        events.push(Event::note(time, duration, track_id, midi, velocity));
                    }
                }
            }
            Step::Accent(v) => {
                let vel = if velocities.is_some() {
                    velocity
                } else {
                    *v as f32
                };
                if is_drum {
                    events.push(Event::sample(
                        time,
                        duration,
                        track_id,
                        &pattern.target,
                        vel,
                    ));
                } else if let Some(midi) = parse_note_name(&pattern.target) {
                    events.push(Event::note(time, duration, track_id, midi, vel));
                }
            }
            Step::Note(name) => {
                let midi = parse_note_name(name).ok_or_else(|| {
                    CompileError::compile(format!("invalid note name: '{name}'"), 0, 0)
                })?;
                events.push(Event::note(time, duration, track_id, midi, velocity));
            }
            Step::Rest => {}
        }
    }

    // Apply post-event transforms (modify events after generation)
    for transform in &pattern.transforms {
        apply_event_transform(&mut events, transform);
    }

    Ok(events)
}

/// Deterministic seed from pattern target and steps for reproducible randomness.
fn compute_pattern_seed(target: &str, steps: &[Step]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    for b in target.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV prime
    }
    for (i, step) in steps.iter().enumerate() {
        hash ^= i as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
        let step_val = match step {
            Step::Hit => 1u64,
            Step::Rest => 0,
            Step::Accent(_) => 2,
            Step::Note(_) => 3,
        };
        hash ^= step_val;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Apply a transform that modifies steps/velocities (pre-event).
fn apply_step_transform(
    steps: &mut Vec<Step>,
    velocities: &mut Option<Vec<f64>>,
    transform: &Transform,
    seed: u64,
) {
    match transform {
        Transform::Fast(n) => {
            if *n <= 0.0 || steps.is_empty() {
                return;
            }
            let repeats = n.round().max(1.0) as usize;
            let original = steps.clone();
            steps.clear();
            for _ in 0..repeats {
                steps.extend(original.iter().cloned());
            }
            if let Some(vels) = velocities {
                let original_vels = vels.clone();
                vels.clear();
                for _ in 0..repeats {
                    vels.extend(original_vels.iter());
                }
            }
        }
        Transform::Slow(n) => {
            if *n <= 0.0 || steps.is_empty() {
                return;
            }
            let keep = (steps.len() as f64 / n).round().max(1.0) as usize;
            steps.truncate(keep);
            if let Some(vels) = velocities {
                vels.truncate(keep);
            }
        }
        Transform::Rev => {
            steps.reverse();
            if let Some(vels) = velocities {
                vels.reverse();
            }
        }
        Transform::Rotate(n) => {
            if steps.is_empty() {
                return;
            }
            let len = steps.len();
            let shift = ((*n % len as i32) + len as i32) as usize % len;
            steps.rotate_right(shift);
            if let Some(vels) = velocities {
                if vels.len() == len {
                    vels.rotate_right(shift);
                }
            }
        }
        Transform::Degrade(prob) => {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            for step in steps.iter_mut() {
                if rng.gen::<f64>() < *prob {
                    *step = Step::Rest;
                }
            }
        }
        Transform::Every(n, inner) => {
            // In compile-time expansion, we have one cycle of steps.
            // `every(N, t)` applies the transform to 1 out of N repetitions.
            // Since we compile a single cycle, apply if cycle 0 mod N == 0
            // (first cycle gets the transform).
            if *n > 0 {
                apply_step_transform(steps, velocities, inner, seed);
            }
        }
        Transform::Sometimes(prob, inner) => {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            if rng.gen::<f64>() < *prob {
                apply_step_transform(steps, velocities, inner, seed);
            }
        }
        Transform::Chop(n) => {
            if *n == 0 || steps.is_empty() {
                return;
            }
            let mut new_steps = Vec::new();
            for step in steps.iter() {
                match step {
                    Step::Rest => {
                        for _ in 0..*n {
                            new_steps.push(Step::Rest);
                        }
                    }
                    _ => {
                        for _ in 0..*n {
                            new_steps.push(step.clone());
                        }
                    }
                }
            }
            *steps = new_steps;
            // Velocities: expand each velocity N times
            if let Some(vels) = velocities {
                let mut new_vels = Vec::new();
                for v in vels.iter() {
                    for _ in 0..*n {
                        new_vels.push(*v);
                    }
                }
                *vels = new_vels;
            }
        }
        Transform::Stutter(n) => {
            if *n == 0 || steps.is_empty() {
                return;
            }
            let original = steps.clone();
            let mut new_steps = Vec::with_capacity(original.len());
            let mut i = 0;
            while i < original.len() {
                match &original[i] {
                    Step::Rest => {
                        new_steps.push(Step::Rest);
                        i += 1;
                    }
                    step => {
                        // Repeat this step N times, consuming subsequent positions
                        for _ in 0..*n {
                            new_steps.push(step.clone());
                        }
                        // Skip the next N-1 steps (they get replaced)
                        i += (*n as usize).min(original.len() - i);
                    }
                }
            }
            *steps = new_steps;
            *velocities = None; // Velocities invalidated by stutter
        }
        // Post-event transforms are handled separately
        Transform::Add(_) | Transform::Gain(_) | Transform::Legato(_) => {}
    }
}

/// Apply a transform that modifies events (post-event).
fn apply_event_transform(events: &mut [Event], transform: &Transform) {
    match transform {
        Transform::Add(semitones) => {
            for event in events.iter_mut() {
                if let crate::event::NoteOrSample::Note(ref mut midi) = event.trigger {
                    *midi = (*midi as i32 + semitones).clamp(0, 127) as u8;
                }
            }
        }
        Transform::Gain(factor) => {
            for event in events.iter_mut() {
                event.velocity = (event.velocity * *factor as f32).clamp(0.0, 1.0);
            }
        }
        Transform::Legato(factor) => {
            for event in events.iter_mut() {
                let ticks = event.duration.ticks();
                let new_ticks = (ticks as f64 * factor).round() as u64;
                event.duration = Beat::from_ticks(new_ticks);
            }
        }
        _ => {} // Step transforms already handled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::Compiler;
    use crate::event::NoteOrSample;

    #[test]
    fn compile_minimal_drums() {
        let src = r#"
tempo 128
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . . X . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert!((song.tempo - 128.0).abs() < f64::EPSILON);
        assert_eq!(song.events.len(), 2); // Two kicks
        assert_eq!(song.track_defs.len(), 1);
    }

    #[test]
    fn compile_note_events() {
        let src = r#"
track bass {
  bass
  section main [1 bars] {
    note: [C2 . . C2 . . Eb2 .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 3); // C2, C2, Eb2

        // Check MIDI note values
        assert_eq!(song.events[0].trigger, NoteOrSample::Note(36)); // C2
        assert_eq!(song.events[1].trigger, NoteOrSample::Note(36)); // C2
        assert_eq!(song.events[2].trigger, NoteOrSample::Note(39)); // Eb2
    }

    #[test]
    fn compile_events_sorted_by_time() {
        let src = r#"
tempo 120
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
    snare: [. X . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();

        for i in 1..song.events.len() {
            assert!(
                song.events[i].time >= song.events[i - 1].time,
                "events not sorted at index {i}"
            );
        }
    }

    #[test]
    fn compile_event_timing() {
        let src = r#"
tempo 120
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . X .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 2);

        // 4 steps over 1 bar (4 beats) = 1 beat per step
        assert_eq!(song.events[0].time, Beat::ZERO);
        assert_eq!(song.events[1].time, Beat::from_beats(2));
    }

    #[test]
    fn compile_multi_section_offset() {
        let src = r#"
track drums {
  kit: default
  section a [1 bars] {
    kick: [X . . .]
  }
  section b [1 bars] {
    snare: [. X . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 2);

        // Section a: kick at beat 0
        assert_eq!(song.events[0].time, Beat::ZERO);

        // Section b starts at bar 1 (beat 4); snare at step 1 of 4 = beat 5
        assert_eq!(song.events[1].time, Beat::from_beats(5));
    }

    #[test]
    fn compile_rest_steps_produce_no_events() {
        let src = r#"
track drums {
  kit: default
  section main [1 bars] {
    kick: [. . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert!(song.events.is_empty());
    }

    #[test]
    fn compile_multiple_tracks() {
        let src = r#"
tempo 130
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
  }
}
track bass {
  bass
  section main [1 bars] {
    note: [C2 . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.track_defs.len(), 2);
        assert_eq!(song.events.len(), 2);
        assert_eq!(song.track_defs[0].0, TrackId(0));
        assert_eq!(song.track_defs[1].0, TrackId(1));
    }

    #[test]
    fn compile_macros_and_mappings() {
        let src = r#"
macro filter = 0.5
map filter -> cutoff (0.0..1.0) exp
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.macros.len(), 1);
        assert_eq!(song.mappings.len(), 1);
    }

    #[test]
    fn compile_invalid_note_name_error() {
        let src = r#"
track bass {
  bass
  section main [1 bars] {
    note: [C10 . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let result = compile_program(&program);
        assert!(result.is_err());
    }

    #[test]
    fn compile_eight_step_pattern_timing() {
        let src = r#"
track drums {
  kit: default
  section main [2 bars] {
    hat: [x x x x x x x x]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 8);

        // 8 steps over 2 bars (8 beats) = 1 beat per step
        for (i, event) in song.events.iter().enumerate() {
            let expected = Beat::from_beats(i as u32);
            assert_eq!(event.time, expected, "event {i} timing");
        }
    }

    #[test]
    fn compile_preserves_velocity() {
        let src = r#"
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . x .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 2);
        assert!((song.events[0].velocity - 0.85).abs() < 0.01); // Hit
        assert!((song.events[1].velocity - 0.5).abs() < 0.01); // Ghost/Accent
    }

    #[test]
    fn compile_sections_populated() {
        let src = r#"
track drums {
  kit: default
  section intro [2 bars] {
    kick: [X . . . X . . .]
  }
  section main [4 bars] {
    kick: [X . X . X . X .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.sections.len(), 2);
        assert_eq!(song.sections[0].name, "intro");
        assert_eq!(song.sections[0].length_in_bars, 2);
        assert_eq!(song.sections[1].name, "main");
        assert_eq!(song.sections[1].length_in_bars, 4);
    }

    #[test]
    fn compile_sections_deduplicate_by_name() {
        let src = r#"
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
  }
}
track bass {
  bass
  section main [1 bars] {
    note: [C2 . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Both tracks have "main" — should only appear once
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].name, "main");
    }

    #[test]
    fn compile_section_overrides() {
        let src = r#"
track drums {
  kit: default
  section verse [2 bars] {
    kick: [X . . .]
    override filter -> cutoff (0.2..0.6) linear
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].name, "verse");
        assert_eq!(song.sections[0].mapping_overrides.len(), 1);
        let ovr = &song.sections[0].mapping_overrides[0];
        assert_eq!(ovr.macro_name, "filter");
        assert_eq!(ovr.target_param, "cutoff");
        assert!((ovr.range.0 - 0.2).abs() < f64::EPSILON);
        assert!((ovr.range.1 - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn compile_section_no_overrides_backward_compat() {
        let src = r#"
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.sections.len(), 1);
        assert!(song.sections[0].mapping_overrides.is_empty());
    }

    #[test]
    fn compile_layers_populated() {
        let src = r#"
layer reverb_wash {
    depth -> reverb_mix (0.0..0.8) smoothstep
    depth -> delay_mix (0.0..0.4) linear
}
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.layers.len(), 1);
        assert_eq!(song.layers[0].name, "reverb_wash");
        assert_eq!(song.layers[0].mappings.len(), 2);
    }

    #[test]
    fn compile_no_layers_backward_compat() {
        let src = r#"
track drums {
  kit: default
  section main [1 bars] {
    kick: [X . . .]
  }
}
"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert!(song.layers.is_empty());
    }

    // --- Transform compilation tests ---

    #[test]
    fn compile_transform_fast_doubles_events() {
        let src = r#"drums = kit("default") |> kick.pattern("X.X.").fast(2)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Original: X.X. = 2 hits. fast(2) doubles to X.X.X.X. = 4 hits
        assert_eq!(song.events.len(), 4);
    }

    #[test]
    fn compile_transform_slow_halves_events() {
        let src = r#"drums = kit("default") |> kick.pattern("X.X.X.X.").slow(2)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Original: X.X.X.X. = 4 hits. slow(2) keeps first 4 steps: X.X. = 2 hits
        assert_eq!(song.events.len(), 2);
    }

    #[test]
    fn compile_transform_rev_reverses_order() {
        let src = r#"drums = kit("default") |> kick.pattern("X...").rev()"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Original: X... (hit at pos 0). rev: ...X (hit at pos 3)
        assert_eq!(song.events.len(), 1);
        // The hit should now be at the last position
        assert!(song.events[0].time > Beat::ZERO);
    }

    #[test]
    fn compile_transform_rotate() {
        let src = r#"drums = kit("default") |> kick.pattern("X...").rotate(2)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Original: X... rotate(2) = ..X.
        assert_eq!(song.events.len(), 1);
        assert!(song.events[0].time > Beat::ZERO);
    }

    #[test]
    fn compile_transform_degrade_deterministic() {
        let src = r#"drums = kit("default") |> kick.pattern("XXXXXXXX").degrade(0.5)"#;
        let program = Compiler::parse(src).unwrap();
        let song1 = compile_program(&program).unwrap();
        let song2 = compile_program(&program).unwrap();
        // Same seed → same result
        assert_eq!(song1.events.len(), song2.events.len());
        // Should remove roughly half the hits
        assert!(song1.events.len() < 8);
        assert!(!song1.events.is_empty());
    }

    #[test]
    fn compile_transform_chop_subdivides() {
        let src = r#"drums = kit("default") |> kick.pattern("X.").chop(2)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Original: X. → chop(2) → XX.. = 2 hits
        assert_eq!(song.events.len(), 2);
    }

    #[test]
    fn compile_transform_stutter_repeats() {
        let src = r#"drums = kit("default") |> kick.pattern("X...").stutter(2)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // X... → stutter(2) → XX.. = 2 hits
        assert_eq!(song.events.len(), 2);
    }

    #[test]
    fn compile_transform_add_transposes() {
        let src = r#"lead = bass() |> note.pattern("X.X.").add(7)"#;
        // "note" as target won't parse as a note name, so hits produce no events
        // Use a proper note pattern instead
        let src2 = r#"
track bass {
  bass
  section main [1 bars] {
    note: [C3 . . .]
  }
}
"#;
        let program = Compiler::parse(src2).unwrap();
        let mut song = compile_program(&program).unwrap();
        // C3 = MIDI 48, manually apply add transform
        assert_eq!(song.events[0].trigger, NoteOrSample::Note(48));

        // Now test via functional syntax with a note target
        // Use the transform on compiled events directly
        apply_event_transform(&mut song.events, &Transform::Add(7));
        assert_eq!(song.events[0].trigger, NoteOrSample::Note(55)); // C3 + 7 = G3
    }

    #[test]
    fn compile_transform_gain_scales_velocity() {
        let src = r#"drums = kit("default") |> kick.pattern("X...").gain(0.5)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 1);
        // Default Hit velocity is 0.85, gain(0.5) → 0.425
        assert!((song.events[0].velocity - 0.425).abs() < 0.01);
    }

    #[test]
    fn compile_transform_legato_scales_duration() {
        let src = r#"drums = kit("default") |> kick.pattern("X...").legato(2.0)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 1);
        // 4 steps over 2 bars (8 beats) = 2 beats per step. legato(2.0) → 4 beats.
        let expected_duration = Beat::from_beats_f64(4.0);
        assert_eq!(song.events[0].duration, expected_duration);
    }

    #[test]
    fn compile_transform_chain_fast_rev() {
        let src = r#"drums = kit("default") |> kick.pattern("X...").fast(2).rev()"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // X... fast(2) → X...X... (2 hits). rev → ...X...X (still 2 hits)
        assert_eq!(song.events.len(), 2);
    }

    #[test]
    fn compile_transform_every_applies_inner() {
        let src = r#"drums = kit("default") |> kick.pattern("X...").every(4, rev)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // every(4, rev) applies rev to first cycle — X... → ...X
        assert_eq!(song.events.len(), 1);
        assert!(song.events[0].time > Beat::ZERO);
    }

    #[test]
    fn compile_no_transforms_unchanged() {
        let src = r#"drums = kit("default") |> kick.pattern("X.X.")"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 2);
    }
}
