//! Pattern engine — compiles AST into events.
//!
//! Transforms a [`Program`] AST into a [`CompiledSong`] containing
//! tempo, events, track definitions, macros, and mappings.

use crate::event::types::{Event, TrackId};
use crate::event::Beat;

use super::ast::*;
use super::error::CompileError;
use super::note::parse_note_name;

/// The result of compiling a DSL program.
#[derive(Debug, Clone)]
pub struct CompiledSong {
    pub tempo: f64,
    pub events: Vec<Event>,
    pub track_defs: Vec<(TrackId, TrackDef)>,
    pub macros: Vec<MacroDef>,
    pub mappings: Vec<MappingDef>,
}

/// Compile a Program AST into a CompiledSong.
pub fn compile_program(program: &Program) -> Result<CompiledSong, CompileError> {
    let mut events = Vec::new();
    let mut track_defs = Vec::new();

    for (idx, track) in program.tracks.iter().enumerate() {
        let track_id = TrackId(idx as u32);
        track_defs.push((track_id, track.clone()));

        let is_drum = matches!(track.instrument, InstrumentRef::Kit(_));

        let mut section_offset = Beat::ZERO;

        for section in &track.sections {
            let section_events = compile_section(section, track_id, is_drum, section_offset)?;
            events.extend(section_events);
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
    let mut events = Vec::new();
    let num_steps = pattern.steps.len();
    if num_steps == 0 {
        return Ok(events);
    }

    // Total beats in this section
    let total_beats = length_bars as f64 * 4.0;
    let step_duration_beats = total_beats / num_steps as f64;

    for (i, step) in pattern.steps.iter().enumerate() {
        let time_beats = i as f64 * step_duration_beats;
        let time = offset + Beat::from_beats_f64(time_beats);
        let duration = Beat::from_beats_f64(step_duration_beats);

        let velocity = if let Some(ref vels) = pattern.velocities {
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
                let vel = if pattern.velocities.is_some() {
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

    Ok(events)
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
}
