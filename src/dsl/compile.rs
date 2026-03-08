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
    pub arrangement: Option<ArrangementDef>,
}

/// Compile a Program AST into a CompiledSong.
pub fn compile_program(program: &Program) -> Result<CompiledSong, CompileError> {
    let num_cycles = program.cycles.unwrap_or(1).max(1);
    let mut events = Vec::new();
    let mut track_defs = Vec::new();
    let mut sections = Vec::new();

    // Compute single-cycle length for time offsets
    let single_cycle_bars: u32 = program
        .tracks
        .iter()
        .map(|t| t.sections.iter().map(|s| s.length_bars).sum::<u32>())
        .max()
        .unwrap_or(1);

    for (idx, track) in program.tracks.iter().enumerate() {
        let track_id = TrackId(idx as u32);
        track_defs.push((track_id, track.clone()));

        let is_drum = matches!(track.instrument, InstrumentRef::Kit(_));

        for cycle in 0..num_cycles {
            let cycle_offset = Beat::from_bars(single_cycle_bars * cycle);
            let mut section_offset = Beat::ZERO;

            for section in &track.sections {
                let section_events = compile_section(
                    section,
                    track_id,
                    is_drum,
                    cycle_offset + section_offset,
                    cycle,
                )?;
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
        arrangement: program.arrangement.clone(),
    })
}

fn compile_section(
    section: &SectionDef,
    track_id: TrackId,
    is_drum: bool,
    offset: Beat,
    cycle_index: u32,
) -> Result<Vec<Event>, CompileError> {
    let mut events = Vec::new();

    for pattern in &section.patterns {
        let pattern_events = compile_pattern(
            pattern,
            track_id,
            is_drum,
            offset,
            section.length_bars,
            cycle_index,
        )?;
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
    cycle_index: u32,
) -> Result<Vec<Event>, CompileError> {
    // Apply pre-event transforms (modify steps/velocities before event generation)
    let mut steps = pattern.steps.clone();
    let mut velocities = pattern.velocities.clone();
    let seed = compute_pattern_seed(&pattern.target, &steps);

    for transform in &pattern.transforms {
        apply_step_transform(&mut steps, &mut velocities, transform, seed, cycle_index);
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
                Step::Hit | Step::Ratchet(_, _) => 0.85,
                Step::Accent(v) => *v as f32,
                Step::Note(_) => 0.8,
                Step::Rest => continue,
                Step::Random(prob) => {
                    // Use seeded RNG to decide if this step fires
                    let step_seed = seed.wrapping_add(i as u64);
                    let mut rng = ChaCha8Rng::seed_from_u64(step_seed);
                    if rng.gen::<f64>() < *prob {
                        0.85
                    } else {
                        continue;
                    }
                }
                Step::Alternate(options) => {
                    if options.is_empty() {
                        continue;
                    }
                    // Pick based on cycle index for multi-cycle alternation
                    let picked = &options[cycle_index as usize % options.len()];
                    match picked {
                        Step::Hit => 0.85,
                        Step::Accent(v) => *v as f32,
                        Step::Rest => continue,
                        _ => 0.8,
                    }
                }
                Step::Subdivided(_) => 0.85, // handled below
                Step::Stacked(_) => 0.85,    // handled below
            }
        };

        if velocity <= 0.0 {
            continue;
        }

        match step {
            Step::Hit => {
                emit_step_event(
                    &mut events,
                    &Step::Hit,
                    time,
                    duration,
                    track_id,
                    is_drum,
                    &pattern.target,
                    velocity,
                    seed,
                    i,
                )?;
            }
            Step::Accent(v) => {
                let vel = if velocities.is_some() {
                    velocity
                } else {
                    *v as f32
                };
                emit_step_event(
                    &mut events,
                    step,
                    time,
                    duration,
                    track_id,
                    is_drum,
                    &pattern.target,
                    vel,
                    seed,
                    i,
                )?;
            }
            Step::Note(name) => {
                let midi = parse_note_name(name).ok_or_else(|| {
                    CompileError::compile(format!("invalid note name: '{name}'"), 0, 0)
                })?;
                events.push(Event::note(time, duration, track_id, midi, velocity));
            }
            Step::Random(_) => {
                // If we reach here, the RNG decided this step fires (velocity set above)
                emit_step_event(
                    &mut events,
                    &Step::Hit,
                    time,
                    duration,
                    track_id,
                    is_drum,
                    &pattern.target,
                    velocity,
                    seed,
                    i,
                )?;
            }
            Step::Alternate(options) => {
                if !options.is_empty() {
                    let picked = &options[cycle_index as usize % options.len()];
                    emit_step_event(
                        &mut events,
                        picked,
                        time,
                        duration,
                        track_id,
                        is_drum,
                        &pattern.target,
                        velocity,
                        seed,
                        i,
                    )?;
                }
            }
            Step::Subdivided(inner) => {
                if !inner.is_empty() {
                    let sub_count = inner.len();
                    let sub_dur_beats = step_duration_beats / sub_count as f64;
                    let sub_duration = Beat::from_beats_f64(sub_dur_beats);
                    for (si, sub_step) in inner.iter().enumerate() {
                        let sub_time = time + Beat::from_beats_f64(si as f64 * sub_dur_beats);
                        let sub_vel = match sub_step {
                            Step::Hit | Step::Random(_) => 0.85,
                            Step::Accent(v) => *v as f32,
                            Step::Rest => continue,
                            _ => 0.8,
                        };
                        emit_step_event(
                            &mut events,
                            sub_step,
                            sub_time,
                            sub_duration,
                            track_id,
                            is_drum,
                            &pattern.target,
                            sub_vel,
                            seed,
                            i * 1000 + si,
                        )?;
                    }
                }
            }
            Step::Ratchet(inner, count) => {
                let count = *count as usize;
                if count > 0 {
                    let sub_dur_beats = step_duration_beats / count as f64;
                    let sub_duration = Beat::from_beats_f64(sub_dur_beats);
                    for ri in 0..count {
                        let sub_time = time + Beat::from_beats_f64(ri as f64 * sub_dur_beats);
                        emit_step_event(
                            &mut events,
                            inner,
                            sub_time,
                            sub_duration,
                            track_id,
                            is_drum,
                            &pattern.target,
                            velocity,
                            seed,
                            i * 1000 + ri,
                        )?;
                    }
                }
            }
            Step::Stacked(targets) => {
                // Emit one event per target at the same time
                for target_name in targets {
                    if is_drum {
                        events.push(Event::sample(
                            time,
                            duration,
                            track_id,
                            target_name,
                            velocity,
                        ));
                    } else if let Some(midi) = parse_note_name(target_name) {
                        events.push(Event::note(time, duration, track_id, midi, velocity));
                    }
                }
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

/// Emit an event for a single step, recursing into nested structures.
#[allow(clippy::too_many_arguments)]
fn emit_step_event(
    events: &mut Vec<Event>,
    step: &Step,
    time: Beat,
    duration: Beat,
    track_id: TrackId,
    is_drum: bool,
    target: &str,
    velocity: f32,
    seed: u64,
    step_index: usize,
) -> Result<(), CompileError> {
    match step {
        Step::Hit => {
            if is_drum {
                events.push(Event::sample(time, duration, track_id, target, velocity));
            } else if let Some(midi) = parse_note_name(target) {
                events.push(Event::note(time, duration, track_id, midi, velocity));
            }
        }
        Step::Accent(_) => {
            if is_drum {
                events.push(Event::sample(time, duration, track_id, target, velocity));
            } else if let Some(midi) = parse_note_name(target) {
                events.push(Event::note(time, duration, track_id, midi, velocity));
            }
        }
        Step::Note(name) => {
            let midi = parse_note_name(name).ok_or_else(|| {
                CompileError::compile(format!("invalid note name: '{name}'"), 0, 0)
            })?;
            events.push(Event::note(time, duration, track_id, midi, velocity));
        }
        Step::Rest => {}
        Step::Random(prob) => {
            let step_seed = seed.wrapping_add(step_index as u64);
            let mut rng = ChaCha8Rng::seed_from_u64(step_seed);
            if rng.gen::<f64>() < *prob {
                if is_drum {
                    events.push(Event::sample(time, duration, track_id, target, velocity));
                } else if let Some(midi) = parse_note_name(target) {
                    events.push(Event::note(time, duration, track_id, midi, velocity));
                }
            }
        }
        Step::Alternate(options) => {
            if !options.is_empty() {
                let picked = &options[step_index % options.len()];
                let vel = match picked {
                    Step::Hit | Step::Random(_) => velocity,
                    Step::Accent(v) => *v as f32,
                    Step::Rest => return Ok(()),
                    _ => velocity,
                };
                emit_step_event(
                    events, picked, time, duration, track_id, is_drum, target, vel, seed,
                    step_index,
                )?;
            }
        }
        Step::Subdivided(inner) => {
            if !inner.is_empty() {
                let dur_ticks = duration.ticks();
                let sub_dur_ticks = dur_ticks / inner.len() as u64;
                let sub_duration = Beat::from_ticks(sub_dur_ticks);
                for (si, sub_step) in inner.iter().enumerate() {
                    let sub_time = time + Beat::from_ticks(si as u64 * sub_dur_ticks);
                    let sub_vel = match sub_step {
                        Step::Hit | Step::Random(_) => 0.85,
                        Step::Accent(v) => *v as f32,
                        Step::Rest => continue,
                        _ => 0.8,
                    };
                    let sub_idx = step_index.wrapping_mul(inner.len()).wrapping_add(si);
                    emit_step_event(
                        events,
                        sub_step,
                        sub_time,
                        sub_duration,
                        track_id,
                        is_drum,
                        target,
                        sub_vel,
                        seed,
                        sub_idx,
                    )?;
                }
            }
        }
        Step::Ratchet(inner_step, count) => {
            let count = *count as usize;
            if count > 0 {
                let dur_ticks = duration.ticks();
                let sub_dur_ticks = dur_ticks / count as u64;
                let sub_duration = Beat::from_ticks(sub_dur_ticks);
                for ri in 0..count {
                    let sub_time = time + Beat::from_ticks(ri as u64 * sub_dur_ticks);
                    let sub_idx = step_index.wrapping_mul(count).wrapping_add(ri);
                    emit_step_event(
                        events,
                        inner_step,
                        sub_time,
                        sub_duration,
                        track_id,
                        is_drum,
                        target,
                        velocity,
                        seed,
                        sub_idx,
                    )?;
                }
            }
        }
        Step::Stacked(targets) => {
            for target_name in targets {
                if is_drum {
                    events.push(Event::sample(
                        time,
                        duration,
                        track_id,
                        target_name,
                        velocity,
                    ));
                } else if let Some(midi) = parse_note_name(target_name) {
                    events.push(Event::note(time, duration, track_id, midi, velocity));
                }
            }
        }
    }
    Ok(())
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
            Step::Random(_) => 4,
            Step::Alternate(_) => 5,
            Step::Subdivided(_) => 6,
            Step::Ratchet(_, _) => 7,
            Step::Stacked(_) => 8,
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
    cycle_index: u32,
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
            // Apply the transform every N cycles based on cycle_index
            if *n > 0 && cycle_index % *n == 0 {
                apply_step_transform(steps, velocities, inner, seed, cycle_index);
            }
        }
        Transform::Sometimes(prob, inner) => {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            if rng.gen::<f64>() < *prob {
                apply_step_transform(steps, velocities, inner, seed, cycle_index);
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

    // --- Extended mini-notation compiler tests ---

    #[test]
    fn compile_random_deterministic() {
        let src = r#"drums = kit("default") |> kick.pattern("????????")"#;
        let program = Compiler::parse(src).unwrap();
        let song1 = compile_program(&program).unwrap();
        let song2 = compile_program(&program).unwrap();
        // Same seed → same events
        assert_eq!(song1.events.len(), song2.events.len());
        // With 50% probability and 8 steps, expect roughly half to fire
        assert!(!song1.events.is_empty());
        assert!(song1.events.len() < 8);
    }

    #[test]
    fn compile_subdivided_timing() {
        let src = r#"drums = kit("default") |> kick.pattern("{X.X}")"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // {X.X} = 1 subdivided step with 3 inner steps: Hit, Rest, Hit = 2 events
        assert_eq!(song.events.len(), 2);
        // Both events should be within the first step's time range
        assert!(song.events[0].time < song.events[1].time);
    }

    #[test]
    fn compile_ratchet_timing() {
        let src = r#"drums = kit("default") |> kick.pattern("X^3...")"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // X^3 = 3 events in first step, ... = 3 rests = 3 events total
        assert_eq!(song.events.len(), 3);
        // All 3 events should be within the first step's time window
        // 4 steps over 2 bars = 2 beats per step, so all 3 in first 2 beats
        let two_beats = Beat::from_beats(2);
        assert!(song.events.iter().all(|e| e.time < two_beats));
    }

    #[test]
    fn compile_euclidean_produces_events() {
        let src = r#"drums = kit("default") |> kick.pattern("E(3,8)")"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 3);
    }

    // --- Transform + new variant interaction tests ---

    #[test]
    fn compile_degrade_with_random_steps() {
        // degrade replaces steps with Rest; Random steps should be degradable
        let src = r#"drums = kit("default") |> kick.pattern("????????").degrade(0.5)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // After degrade(0.5) on 8 Random(0.5) steps, some get replaced with Rest.
        // The remaining Random steps then go through RNG. Result should be fewer events.
        // Just verify it compiles and produces a deterministic result.
        let song2 = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), song2.events.len());
    }

    #[test]
    fn compile_rev_with_subdivided() {
        // rev should reverse the step order, moving Subdivided to end
        let src = r#"drums = kit("default") |> kick.pattern("{X.X}...").rev()"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Original: {X.X}... → rev: ...{X.X}
        // Subdivided step now at position 3 of 4. Should produce 2 events near the end.
        assert_eq!(song.events.len(), 2);
        // Both events should be in the last quarter of the section
        let three_beats = Beat::from_beats(3);
        assert!(
            song.events.iter().all(|e| e.time >= three_beats),
            "reversed subdivision should place events near end"
        );
    }

    #[test]
    fn compile_fast_with_ratchet() {
        // fast(2) should double the pattern including ratchet steps
        let src = r#"drums = kit("default") |> kick.pattern("X^3...").fast(2)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Original: X^3... = 3 events. fast(2) → X^3...X^3... = 6 events
        assert_eq!(song.events.len(), 6);
    }

    #[test]
    fn compile_chop_with_alternate() {
        // chop(2) should duplicate each step; Alternate steps get duplicated too
        let src = r#"drums = kit("default") |> kick.pattern("<X x>..").chop(2)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // Original: <X x>.. (3 steps). chop(2) → <X x><X x>.... (6 steps)
        // Step 0: Alternate picks options[0%2]=X → hit
        // Step 1: Alternate picks options[1%2]=x → hit
        // Steps 2-5: Rest
        assert_eq!(song.events.len(), 2);
    }

    #[test]
    fn compile_gain_with_ratchet() {
        // gain transform should scale velocity of ratchet sub-events
        let src = r#"drums = kit("default") |> kick.pattern("X^3...").gain(0.5)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        assert_eq!(song.events.len(), 3);
        // Default ratchet velocity is 0.85, gain(0.5) → 0.425
        for event in &song.events {
            assert!(
                (event.velocity - 0.425).abs() < 0.01,
                "ratchet event velocity {} should be 0.425 after gain(0.5)",
                event.velocity
            );
        }
    }

    #[test]
    fn compile_rotate_with_random() {
        // rotate should shift Random steps just like any other step
        let src = r#"drums = kit("default") |> kick.pattern("?...").rotate(2)"#;
        let program = Compiler::parse(src).unwrap();
        let song = compile_program(&program).unwrap();
        // ?... rotate(2) → ..?. — random step is now at position 2
        // Default section is 2 bars (8 beats), 4 steps = 2 beats/step. Position 2 = beat 4.
        if !song.events.is_empty() {
            let expected = Beat::from_beats(4);
            assert_eq!(song.events[0].time, expected);
        }
    }

    #[test]
    fn compile_subdivided_with_velocity_array() {
        // When velocity array is present, it gates the subdivided step
        // Inner sub-steps use their own velocities
        let pattern = PatternDef {
            target: "kick".to_string(),
            steps: vec![
                Step::Subdivided(vec![Step::Hit, Step::Rest, Step::Hit]),
                Step::Hit,
            ],
            velocities: Some(vec![0.9, 0.7]),
            transforms: vec![],
        };
        let events = compile_pattern(&pattern, TrackId(0), true, Beat::ZERO, 1, 0).unwrap();
        // Step 0: Subdivided with vel 0.9 (> 0 → active), produces 2 sub-events
        // Step 1: Hit with vel 0.7
        assert_eq!(events.len(), 3);
        // Sub-events should have inner velocities (0.85 for Hit), not 0.9
        assert!((events[0].velocity - 0.85).abs() < 0.01);
        assert!((events[1].velocity - 0.85).abs() < 0.01);
        // Regular hit should use the array velocity
        assert!((events[2].velocity - 0.7).abs() < 0.01);
    }

    #[test]
    fn compile_subdivided_zero_velocity_skips() {
        // When velocity array has 0.0 for a Subdivided step, skip it entirely
        let pattern = PatternDef {
            target: "kick".to_string(),
            steps: vec![
                Step::Subdivided(vec![Step::Hit, Step::Hit, Step::Hit]),
                Step::Hit,
            ],
            velocities: Some(vec![0.0, 0.8]),
            transforms: vec![],
        };
        let events = compile_pattern(&pattern, TrackId(0), true, Beat::ZERO, 1, 0).unwrap();
        // Step 0: Subdivided with vel 0.0 → skipped
        // Step 1: Hit with vel 0.8
        assert_eq!(events.len(), 1);
    }

    // --- Recursive nesting tests (Gap 4 & 5) ---

    #[test]
    fn compile_random_inside_ratchet_respects_probability() {
        // ?0.0^3 — Random(0.0) ratcheted 3 times. prob=0 → all 3 should be silent.
        let pattern = PatternDef {
            target: "kick".to_string(),
            steps: vec![Step::Ratchet(Box::new(Step::Random(0.0)), 3)],
            velocities: None,
            transforms: vec![],
        };
        let events = compile_pattern(&pattern, TrackId(0), true, Beat::ZERO, 1, 0).unwrap();
        assert_eq!(
            events.len(),
            0,
            "Random(0.0) inside ratchet should produce 0 events"
        );
    }

    #[test]
    fn compile_random_inside_subdivision_respects_probability() {
        // {?0.0 X} — subdivision with Random(0.0) and Hit
        let pattern = PatternDef {
            target: "kick".to_string(),
            steps: vec![Step::Subdivided(vec![Step::Random(0.0), Step::Hit])],
            velocities: None,
            transforms: vec![],
        };
        let events = compile_pattern(&pattern, TrackId(0), true, Beat::ZERO, 1, 0).unwrap();
        // Random(0.0) never fires, Hit fires once
        assert_eq!(
            events.len(),
            1,
            "only the Hit sub-step should produce an event"
        );
    }

    #[test]
    fn compile_alternate_inside_subdivision() {
        // {<X .> X} — subdivision with Alternate([Hit, Rest]) and Hit
        let pattern = PatternDef {
            target: "kick".to_string(),
            steps: vec![Step::Subdivided(vec![
                Step::Alternate(vec![Step::Hit, Step::Rest]),
                Step::Hit,
            ])],
            velocities: None,
            transforms: vec![],
        };
        let events = compile_pattern(&pattern, TrackId(0), true, Beat::ZERO, 1, 0).unwrap();
        // Sub-step 0: Alternate picks index 0 % 2 = 0 → Hit
        // Sub-step 1: Hit
        assert_eq!(
            events.len(),
            2,
            "alternate inside subdivision should produce events"
        );
    }

    #[test]
    fn compile_ratchet_inside_subdivision() {
        // {X^2 .} — subdivision with Ratchet(Hit,2) and Rest
        let pattern = PatternDef {
            target: "kick".to_string(),
            steps: vec![Step::Subdivided(vec![
                Step::Ratchet(Box::new(Step::Hit), 2),
                Step::Rest,
            ])],
            velocities: None,
            transforms: vec![],
        };
        let events = compile_pattern(&pattern, TrackId(0), true, Beat::ZERO, 1, 0).unwrap();
        // Sub-step 0: Ratchet expands to 2 hits
        // Sub-step 1: Rest
        assert_eq!(events.len(), 2, "ratchet inside subdivision should expand");
    }

    #[test]
    fn compile_subdivided_inside_ratchet() {
        // Subdivided({X .})^2 — ratchet with a subdivision as inner step
        let pattern = PatternDef {
            target: "kick".to_string(),
            steps: vec![Step::Ratchet(
                Box::new(Step::Subdivided(vec![Step::Hit, Step::Rest])),
                2,
            )],
            velocities: None,
            transforms: vec![],
        };
        let events = compile_pattern(&pattern, TrackId(0), true, Beat::ZERO, 1, 0).unwrap();
        // Ratchet repeats 2 times, each time the subdivision produces 1 event (Hit, Rest)
        assert_eq!(
            events.len(),
            2,
            "subdivided inside ratchet should work recursively"
        );
    }
}
