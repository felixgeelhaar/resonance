//! AST diffing — compares two Program ASTs and produces structured diffs.
//!
//! Foundation for structural intents and diff preview UI.

use super::ast::*;

/// A single change between two ASTs.
#[derive(Debug, Clone, PartialEq)]
pub enum AstChange {
    TempoChanged {
        old: f64,
        new: f64,
    },
    TrackAdded {
        track: TrackDef,
    },
    TrackRemoved {
        name: String,
    },
    TrackInstrumentChanged {
        track_name: String,
        old: InstrumentRef,
        new: InstrumentRef,
    },
    SectionAdded {
        track_name: String,
        section: SectionDef,
    },
    SectionRemoved {
        track_name: String,
        section_name: String,
    },
    SectionLengthChanged {
        track_name: String,
        section_name: String,
        old_bars: u32,
        new_bars: u32,
    },
    PatternAdded {
        track_name: String,
        section_name: String,
        pattern: PatternDef,
    },
    PatternRemoved {
        track_name: String,
        section_name: String,
        target: String,
    },
    PatternChanged {
        track_name: String,
        section_name: String,
        target: String,
        old_steps: Vec<Step>,
        new_steps: Vec<Step>,
    },
    MacroAdded {
        macro_def: MacroDef,
    },
    MacroRemoved {
        name: String,
    },
    MacroDefaultChanged {
        name: String,
        old: f64,
        new: f64,
    },
    MappingAdded {
        mapping: MappingDef,
    },
    MappingRemoved {
        macro_name: String,
        target_param: String,
    },
    MappingChanged {
        macro_name: String,
        target_param: String,
        old: MappingDef,
        new: MappingDef,
    },
}

/// Error when applying a diff.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffError(pub String);

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DiffError: {}", self.0)
    }
}

/// A structured diff between two Program ASTs.
#[derive(Debug, Clone, PartialEq)]
pub struct AstDiff {
    pub changes: Vec<AstChange>,
}

impl AstDiff {
    /// Compute the diff between two Program ASTs.
    pub fn diff(old: &Program, new: &Program) -> Self {
        let mut changes = Vec::new();

        // Tempo
        if (old.tempo - new.tempo).abs() > f64::EPSILON {
            changes.push(AstChange::TempoChanged {
                old: old.tempo,
                new: new.tempo,
            });
        }

        // Tracks
        diff_tracks(&old.tracks, &new.tracks, &mut changes);

        // Macros
        diff_macros(&old.macros, &new.macros, &mut changes);

        // Mappings
        diff_mappings(&old.mappings, &new.mappings, &mut changes);

        AstDiff { changes }
    }

    /// Apply this diff to a program, producing a new program.
    pub fn apply(&self, program: &Program) -> Result<Program, DiffError> {
        let mut result = program.clone();

        for change in &self.changes {
            apply_change(&mut result, change)?;
        }

        Ok(result)
    }

    /// Whether this diff contains no changes.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Whether this diff is safe during live performance (only macro/mapping changes).
    pub fn is_performance_safe(&self) -> bool {
        self.changes.iter().all(|c| {
            matches!(
                c,
                AstChange::MacroAdded { .. }
                    | AstChange::MacroRemoved { .. }
                    | AstChange::MacroDefaultChanged { .. }
                    | AstChange::MappingAdded { .. }
                    | AstChange::MappingRemoved { .. }
                    | AstChange::MappingChanged { .. }
                    | AstChange::TempoChanged { .. }
            )
        })
    }

    /// Generate human-readable summaries for each change.
    pub fn summaries(&self) -> Vec<String> {
        self.changes.iter().map(summary_for_change).collect()
    }
}

fn diff_tracks(old: &[TrackDef], new: &[TrackDef], changes: &mut Vec<AstChange>) {
    // Index tracks by name
    let old_names: Vec<&str> = old.iter().map(|t| t.name.as_str()).collect();
    let new_names: Vec<&str> = new.iter().map(|t| t.name.as_str()).collect();

    // Removed tracks
    for t in old {
        if !new_names.contains(&t.name.as_str()) {
            changes.push(AstChange::TrackRemoved {
                name: t.name.clone(),
            });
        }
    }

    // Added tracks
    for t in new {
        if !old_names.contains(&t.name.as_str()) {
            changes.push(AstChange::TrackAdded { track: t.clone() });
        }
    }

    // Modified tracks (same name exists in both)
    for new_track in new {
        if let Some(old_track) = old.iter().find(|t| t.name == new_track.name) {
            // Instrument changed?
            if old_track.instrument != new_track.instrument {
                changes.push(AstChange::TrackInstrumentChanged {
                    track_name: new_track.name.clone(),
                    old: old_track.instrument.clone(),
                    new: new_track.instrument.clone(),
                });
            }

            // Sections
            diff_sections(
                &new_track.name,
                &old_track.sections,
                &new_track.sections,
                changes,
            );
        }
    }
}

fn diff_sections(
    track_name: &str,
    old: &[SectionDef],
    new: &[SectionDef],
    changes: &mut Vec<AstChange>,
) {
    let old_names: Vec<&str> = old.iter().map(|s| s.name.as_str()).collect();
    let new_names: Vec<&str> = new.iter().map(|s| s.name.as_str()).collect();

    for s in old {
        if !new_names.contains(&s.name.as_str()) {
            changes.push(AstChange::SectionRemoved {
                track_name: track_name.to_string(),
                section_name: s.name.clone(),
            });
        }
    }

    for s in new {
        if !old_names.contains(&s.name.as_str()) {
            changes.push(AstChange::SectionAdded {
                track_name: track_name.to_string(),
                section: s.clone(),
            });
        }
    }

    for new_sec in new {
        if let Some(old_sec) = old.iter().find(|s| s.name == new_sec.name) {
            if old_sec.length_bars != new_sec.length_bars {
                changes.push(AstChange::SectionLengthChanged {
                    track_name: track_name.to_string(),
                    section_name: new_sec.name.clone(),
                    old_bars: old_sec.length_bars,
                    new_bars: new_sec.length_bars,
                });
            }

            diff_patterns(
                track_name,
                &new_sec.name,
                &old_sec.patterns,
                &new_sec.patterns,
                changes,
            );
        }
    }
}

fn diff_patterns(
    track_name: &str,
    section_name: &str,
    old: &[PatternDef],
    new: &[PatternDef],
    changes: &mut Vec<AstChange>,
) {
    let old_targets: Vec<&str> = old.iter().map(|p| p.target.as_str()).collect();
    let new_targets: Vec<&str> = new.iter().map(|p| p.target.as_str()).collect();

    for p in old {
        if !new_targets.contains(&p.target.as_str()) {
            changes.push(AstChange::PatternRemoved {
                track_name: track_name.to_string(),
                section_name: section_name.to_string(),
                target: p.target.clone(),
            });
        }
    }

    for p in new {
        if !old_targets.contains(&p.target.as_str()) {
            changes.push(AstChange::PatternAdded {
                track_name: track_name.to_string(),
                section_name: section_name.to_string(),
                pattern: p.clone(),
            });
        }
    }

    for new_pat in new {
        if let Some(old_pat) = old.iter().find(|p| p.target == new_pat.target) {
            if old_pat.steps != new_pat.steps {
                changes.push(AstChange::PatternChanged {
                    track_name: track_name.to_string(),
                    section_name: section_name.to_string(),
                    target: new_pat.target.clone(),
                    old_steps: old_pat.steps.clone(),
                    new_steps: new_pat.steps.clone(),
                });
            }
        }
    }
}

fn diff_macros(old: &[MacroDef], new: &[MacroDef], changes: &mut Vec<AstChange>) {
    let old_names: Vec<&str> = old.iter().map(|m| m.name.as_str()).collect();
    let new_names: Vec<&str> = new.iter().map(|m| m.name.as_str()).collect();

    for m in old {
        if !new_names.contains(&m.name.as_str()) {
            changes.push(AstChange::MacroRemoved {
                name: m.name.clone(),
            });
        }
    }

    for m in new {
        if !old_names.contains(&m.name.as_str()) {
            changes.push(AstChange::MacroAdded {
                macro_def: m.clone(),
            });
        }
    }

    for new_macro in new {
        if let Some(old_macro) = old.iter().find(|m| m.name == new_macro.name) {
            if (old_macro.default_value - new_macro.default_value).abs() > f64::EPSILON {
                changes.push(AstChange::MacroDefaultChanged {
                    name: new_macro.name.clone(),
                    old: old_macro.default_value,
                    new: new_macro.default_value,
                });
            }
        }
    }
}

fn diff_mappings(old: &[MappingDef], new: &[MappingDef], changes: &mut Vec<AstChange>) {
    // Key mappings by (macro_name, target_param)
    let old_keys: Vec<(&str, &str)> = old
        .iter()
        .map(|m| (m.macro_name.as_str(), m.target_param.as_str()))
        .collect();
    let new_keys: Vec<(&str, &str)> = new
        .iter()
        .map(|m| (m.macro_name.as_str(), m.target_param.as_str()))
        .collect();

    for m in old {
        let key = (m.macro_name.as_str(), m.target_param.as_str());
        if !new_keys.contains(&key) {
            changes.push(AstChange::MappingRemoved {
                macro_name: m.macro_name.clone(),
                target_param: m.target_param.clone(),
            });
        }
    }

    for m in new {
        let key = (m.macro_name.as_str(), m.target_param.as_str());
        if !old_keys.contains(&key) {
            changes.push(AstChange::MappingAdded { mapping: m.clone() });
        }
    }

    for new_map in new {
        let key = (new_map.macro_name.as_str(), new_map.target_param.as_str());
        if let Some(old_map) = old
            .iter()
            .find(|m| (m.macro_name.as_str(), m.target_param.as_str()) == key)
        {
            if old_map != new_map {
                changes.push(AstChange::MappingChanged {
                    macro_name: new_map.macro_name.clone(),
                    target_param: new_map.target_param.clone(),
                    old: old_map.clone(),
                    new: new_map.clone(),
                });
            }
        }
    }
}

fn apply_change(program: &mut Program, change: &AstChange) -> Result<(), DiffError> {
    match change {
        AstChange::TempoChanged { new, .. } => {
            program.tempo = *new;
        }
        AstChange::TrackAdded { track } => {
            program.tracks.push(track.clone());
        }
        AstChange::TrackRemoved { name } => {
            program.tracks.retain(|t| t.name != *name);
        }
        AstChange::TrackInstrumentChanged {
            track_name, new, ..
        } => {
            let track = program
                .tracks
                .iter_mut()
                .find(|t| t.name == *track_name)
                .ok_or_else(|| DiffError(format!("track not found: {track_name}")))?;
            track.instrument = new.clone();
        }
        AstChange::SectionAdded {
            track_name,
            section,
        } => {
            let track = program
                .tracks
                .iter_mut()
                .find(|t| t.name == *track_name)
                .ok_or_else(|| DiffError(format!("track not found: {track_name}")))?;
            track.sections.push(section.clone());
        }
        AstChange::SectionRemoved {
            track_name,
            section_name,
        } => {
            let track = program
                .tracks
                .iter_mut()
                .find(|t| t.name == *track_name)
                .ok_or_else(|| DiffError(format!("track not found: {track_name}")))?;
            track.sections.retain(|s| s.name != *section_name);
        }
        AstChange::SectionLengthChanged {
            track_name,
            section_name,
            new_bars,
            ..
        } => {
            let track = program
                .tracks
                .iter_mut()
                .find(|t| t.name == *track_name)
                .ok_or_else(|| DiffError(format!("track not found: {track_name}")))?;
            let section = track
                .sections
                .iter_mut()
                .find(|s| s.name == *section_name)
                .ok_or_else(|| DiffError(format!("section not found: {section_name}")))?;
            section.length_bars = *new_bars;
        }
        AstChange::PatternAdded {
            track_name,
            section_name,
            pattern,
        } => {
            let track = program
                .tracks
                .iter_mut()
                .find(|t| t.name == *track_name)
                .ok_or_else(|| DiffError(format!("track not found: {track_name}")))?;
            let section = track
                .sections
                .iter_mut()
                .find(|s| s.name == *section_name)
                .ok_or_else(|| DiffError(format!("section not found: {section_name}")))?;
            section.patterns.push(pattern.clone());
        }
        AstChange::PatternRemoved {
            track_name,
            section_name,
            target,
        } => {
            let track = program
                .tracks
                .iter_mut()
                .find(|t| t.name == *track_name)
                .ok_or_else(|| DiffError(format!("track not found: {track_name}")))?;
            let section = track
                .sections
                .iter_mut()
                .find(|s| s.name == *section_name)
                .ok_or_else(|| DiffError(format!("section not found: {section_name}")))?;
            section.patterns.retain(|p| p.target != *target);
        }
        AstChange::PatternChanged {
            track_name,
            section_name,
            target,
            new_steps,
            ..
        } => {
            let track = program
                .tracks
                .iter_mut()
                .find(|t| t.name == *track_name)
                .ok_or_else(|| DiffError(format!("track not found: {track_name}")))?;
            let section = track
                .sections
                .iter_mut()
                .find(|s| s.name == *section_name)
                .ok_or_else(|| DiffError(format!("section not found: {section_name}")))?;
            let pattern = section
                .patterns
                .iter_mut()
                .find(|p| p.target == *target)
                .ok_or_else(|| DiffError(format!("pattern not found: {target}")))?;
            pattern.steps = new_steps.clone();
        }
        AstChange::MacroAdded { macro_def } => {
            program.macros.push(macro_def.clone());
        }
        AstChange::MacroRemoved { name } => {
            program.macros.retain(|m| m.name != *name);
        }
        AstChange::MacroDefaultChanged { name, new, .. } => {
            let m = program
                .macros
                .iter_mut()
                .find(|m| m.name == *name)
                .ok_or_else(|| DiffError(format!("macro not found: {name}")))?;
            m.default_value = *new;
        }
        AstChange::MappingAdded { mapping } => {
            program.mappings.push(mapping.clone());
        }
        AstChange::MappingRemoved {
            macro_name,
            target_param,
        } => {
            program
                .mappings
                .retain(|m| !(m.macro_name == *macro_name && m.target_param == *target_param));
        }
        AstChange::MappingChanged {
            macro_name,
            target_param,
            new,
            ..
        } => {
            let m = program
                .mappings
                .iter_mut()
                .find(|m| m.macro_name == *macro_name && m.target_param == *target_param)
                .ok_or_else(|| {
                    DiffError(format!("mapping not found: {macro_name} -> {target_param}"))
                })?;
            *m = new.clone();
        }
    }
    Ok(())
}

fn summary_for_change(change: &AstChange) -> String {
    match change {
        AstChange::TempoChanged { old, new } => format!("Tempo: {old} → {new}"),
        AstChange::TrackAdded { track } => format!("+ Track '{}'", track.name),
        AstChange::TrackRemoved { name } => format!("- Track '{name}'"),
        AstChange::TrackInstrumentChanged {
            track_name,
            old,
            new,
        } => format!("~ Track '{track_name}' instrument: {old:?} → {new:?}"),
        AstChange::SectionAdded {
            track_name,
            section,
        } => format!("+ Section '{}' in '{track_name}'", section.name),
        AstChange::SectionRemoved {
            track_name,
            section_name,
        } => format!("- Section '{section_name}' from '{track_name}'"),
        AstChange::SectionLengthChanged {
            track_name,
            section_name,
            old_bars,
            new_bars,
        } => format!("~ Section '{section_name}' in '{track_name}': {old_bars} → {new_bars} bars"),
        AstChange::PatternAdded {
            track_name,
            section_name,
            pattern,
        } => format!(
            "+ Pattern '{}' in '{track_name}/{section_name}'",
            pattern.target
        ),
        AstChange::PatternRemoved {
            track_name,
            section_name,
            target,
        } => format!("- Pattern '{target}' from '{track_name}/{section_name}'"),
        AstChange::PatternChanged {
            track_name,
            section_name,
            target,
            ..
        } => format!("~ Pattern '{target}' in '{track_name}/{section_name}'"),
        AstChange::MacroAdded { macro_def } => {
            format!("+ Macro '{}' = {}", macro_def.name, macro_def.default_value)
        }
        AstChange::MacroRemoved { name } => format!("- Macro '{name}'"),
        AstChange::MacroDefaultChanged { name, old, new } => {
            format!("~ Macro '{name}': {old} → {new}")
        }
        AstChange::MappingAdded { mapping } => format!(
            "+ Mapping {} → {} ({:?})",
            mapping.macro_name, mapping.target_param, mapping.curve
        ),
        AstChange::MappingRemoved {
            macro_name,
            target_param,
        } => format!("- Mapping {macro_name} → {target_param}"),
        AstChange::MappingChanged {
            macro_name,
            target_param,
            ..
        } => format!("~ Mapping {macro_name} → {target_param}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_program() -> Program {
        Program {
            tempo: 120.0,
            tracks: vec![TrackDef {
                name: "drums".to_string(),
                instrument: InstrumentRef::Kit("default".to_string()),
                sections: vec![SectionDef {
                    name: "main".to_string(),
                    length_bars: 2,
                    patterns: vec![PatternDef {
                        target: "kick".to_string(),
                        steps: vec![Step::Hit, Step::Rest, Step::Rest, Step::Rest],
                        velocities: None,
                    }],
                    overrides: vec![],
                }],
            }],
            macros: vec![MacroDef {
                name: "filter".to_string(),
                default_value: 0.5,
            }],
            mappings: vec![MappingDef {
                macro_name: "filter".to_string(),
                target_param: "cutoff".to_string(),
                range: (0.0, 1.0),
                curve: CurveKind::Linear,
            }],
            layers: vec![],
        }
    }

    #[test]
    fn identical_programs_produce_empty_diff() {
        let a = base_program();
        let b = base_program();
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.is_empty());
        assert!(diff.is_performance_safe());
    }

    #[test]
    fn tempo_change() {
        let a = base_program();
        let mut b = base_program();
        b.tempo = 140.0;
        let diff = AstDiff::diff(&a, &b);
        assert_eq!(diff.changes.len(), 1);
        assert!(matches!(
            &diff.changes[0],
            AstChange::TempoChanged { old, new } if (*old - 120.0).abs() < f64::EPSILON && (*new - 140.0).abs() < f64::EPSILON
        ));
        assert!(diff.is_performance_safe());
    }

    #[test]
    fn track_added() {
        let a = base_program();
        let mut b = base_program();
        b.tracks.push(TrackDef {
            name: "bass".to_string(),
            instrument: InstrumentRef::Bass,
            sections: vec![],
        });
        let diff = AstDiff::diff(&a, &b);
        assert!(diff
            .changes
            .iter()
            .any(|c| matches!(c, AstChange::TrackAdded { track } if track.name == "bass")));
        assert!(!diff.is_performance_safe());
    }

    #[test]
    fn track_removed() {
        let a = base_program();
        let mut b = base_program();
        b.tracks.clear();
        let diff = AstDiff::diff(&a, &b);
        assert!(diff
            .changes
            .iter()
            .any(|c| matches!(c, AstChange::TrackRemoved { name } if name == "drums")));
    }

    #[test]
    fn track_instrument_changed() {
        let a = base_program();
        let mut b = base_program();
        b.tracks[0].instrument = InstrumentRef::Bass;
        let diff = AstDiff::diff(&a, &b);
        assert!(diff
            .changes
            .iter()
            .any(|c| matches!(c, AstChange::TrackInstrumentChanged { track_name, .. } if track_name == "drums")));
    }

    #[test]
    fn section_added() {
        let a = base_program();
        let mut b = base_program();
        b.tracks[0].sections.push(SectionDef {
            name: "chorus".to_string(),
            length_bars: 4,
            patterns: vec![],
            overrides: vec![],
        });
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::SectionAdded { track_name, section }
            if track_name == "drums" && section.name == "chorus"
        )));
    }

    #[test]
    fn section_removed() {
        let a = base_program();
        let mut b = base_program();
        b.tracks[0].sections.clear();
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::SectionRemoved { track_name, section_name }
            if track_name == "drums" && section_name == "main"
        )));
    }

    #[test]
    fn section_length_changed() {
        let a = base_program();
        let mut b = base_program();
        b.tracks[0].sections[0].length_bars = 4;
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(
            c,
            AstChange::SectionLengthChanged {
                old_bars: 2,
                new_bars: 4,
                ..
            }
        )));
    }

    #[test]
    fn pattern_added() {
        let a = base_program();
        let mut b = base_program();
        b.tracks[0].sections[0].patterns.push(PatternDef {
            target: "snare".to_string(),
            steps: vec![Step::Rest, Step::Hit, Step::Rest, Step::Rest],
            velocities: None,
        });
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::PatternAdded { pattern, .. } if pattern.target == "snare"
        )));
    }

    #[test]
    fn pattern_removed() {
        let a = base_program();
        let mut b = base_program();
        b.tracks[0].sections[0].patterns.clear();
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::PatternRemoved { target, .. } if target == "kick"
        )));
    }

    #[test]
    fn pattern_changed() {
        let a = base_program();
        let mut b = base_program();
        b.tracks[0].sections[0].patterns[0].steps =
            vec![Step::Hit, Step::Hit, Step::Rest, Step::Rest];
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::PatternChanged { target, .. } if target == "kick"
        )));
    }

    #[test]
    fn macro_added() {
        let a = base_program();
        let mut b = base_program();
        b.macros.push(MacroDef {
            name: "depth".to_string(),
            default_value: 0.3,
        });
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::MacroAdded { macro_def } if macro_def.name == "depth"
        )));
        assert!(diff.is_performance_safe());
    }

    #[test]
    fn macro_removed() {
        let a = base_program();
        let mut b = base_program();
        b.macros.clear();
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::MacroRemoved { name } if name == "filter"
        )));
    }

    #[test]
    fn macro_default_changed() {
        let a = base_program();
        let mut b = base_program();
        b.macros[0].default_value = 0.8;
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::MacroDefaultChanged { name, .. } if name == "filter"
        )));
    }

    #[test]
    fn mapping_added() {
        let a = base_program();
        let mut b = base_program();
        b.mappings.push(MappingDef {
            macro_name: "filter".to_string(),
            target_param: "resonance".to_string(),
            range: (0.0, 1.0),
            curve: CurveKind::Exp,
        });
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::MappingAdded { mapping } if mapping.target_param == "resonance"
        )));
    }

    #[test]
    fn mapping_removed() {
        let a = base_program();
        let mut b = base_program();
        b.mappings.clear();
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::MappingRemoved { macro_name, target_param }
            if macro_name == "filter" && target_param == "cutoff"
        )));
    }

    #[test]
    fn mapping_changed() {
        let a = base_program();
        let mut b = base_program();
        b.mappings[0].curve = CurveKind::Exp;
        let diff = AstDiff::diff(&a, &b);
        assert!(diff.changes.iter().any(|c| matches!(c,
            AstChange::MappingChanged { macro_name, target_param, .. }
            if macro_name == "filter" && target_param == "cutoff"
        )));
    }

    #[test]
    fn diff_then_apply_round_trip() {
        let a = base_program();
        let mut b = base_program();
        b.tempo = 140.0;
        b.tracks[0].sections[0].patterns[0].steps =
            vec![Step::Hit, Step::Hit, Step::Hit, Step::Rest];
        b.macros[0].default_value = 0.8;

        let diff = AstDiff::diff(&a, &b);
        let result = diff.apply(&a).unwrap();

        assert!((result.tempo - 140.0).abs() < f64::EPSILON);
        assert_eq!(
            result.tracks[0].sections[0].patterns[0].steps,
            vec![Step::Hit, Step::Hit, Step::Hit, Step::Rest]
        );
        assert!((result.macros[0].default_value - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn apply_track_add_and_remove() {
        let a = base_program();

        // Add a track
        let diff = AstDiff {
            changes: vec![AstChange::TrackAdded {
                track: TrackDef {
                    name: "bass".to_string(),
                    instrument: InstrumentRef::Bass,
                    sections: vec![],
                },
            }],
        };
        let result = diff.apply(&a).unwrap();
        assert_eq!(result.tracks.len(), 2);

        // Remove it
        let diff2 = AstDiff {
            changes: vec![AstChange::TrackRemoved {
                name: "bass".to_string(),
            }],
        };
        let result2 = diff2.apply(&result).unwrap();
        assert_eq!(result2.tracks.len(), 1);
    }

    #[test]
    fn apply_errors_on_missing_track() {
        let a = base_program();
        let diff = AstDiff {
            changes: vec![AstChange::TrackInstrumentChanged {
                track_name: "nonexistent".to_string(),
                old: InstrumentRef::Bass,
                new: InstrumentRef::Poly,
            }],
        };
        assert!(diff.apply(&a).is_err());
    }

    #[test]
    fn performance_safe_classification() {
        // Only macro/mapping changes are safe
        let safe = AstDiff {
            changes: vec![
                AstChange::MacroDefaultChanged {
                    name: "x".to_string(),
                    old: 0.0,
                    new: 1.0,
                },
                AstChange::MappingAdded {
                    mapping: MappingDef {
                        macro_name: "x".to_string(),
                        target_param: "y".to_string(),
                        range: (0.0, 1.0),
                        curve: CurveKind::Linear,
                    },
                },
            ],
        };
        assert!(safe.is_performance_safe());

        // Track changes are not safe
        let unsafe_diff = AstDiff {
            changes: vec![AstChange::TrackAdded {
                track: TrackDef {
                    name: "x".to_string(),
                    instrument: InstrumentRef::Bass,
                    sections: vec![],
                },
            }],
        };
        assert!(!unsafe_diff.is_performance_safe());
    }

    #[test]
    fn summaries_generated() {
        let a = base_program();
        let mut b = base_program();
        b.tempo = 140.0;
        b.macros[0].default_value = 0.8;
        let diff = AstDiff::diff(&a, &b);
        let summaries = diff.summaries();
        assert_eq!(summaries.len(), 2);
        assert!(summaries[0].contains("Tempo"));
        assert!(summaries[1].contains("Macro"));
    }

    #[test]
    fn empty_diff_produces_no_summaries() {
        let a = base_program();
        let diff = AstDiff::diff(&a, &a);
        assert!(diff.summaries().is_empty());
    }

    #[test]
    fn complex_round_trip() {
        let a = base_program();
        let mut b = base_program();
        // Multiple changes
        b.tempo = 140.0;
        b.tracks.push(TrackDef {
            name: "bass".to_string(),
            instrument: InstrumentRef::Bass,
            sections: vec![],
        });
        b.tracks[0].sections[0].length_bars = 4;
        b.tracks[0].sections[0].patterns[0].steps =
            vec![Step::Hit, Step::Hit, Step::Rest, Step::Hit];
        b.macros.push(MacroDef {
            name: "depth".to_string(),
            default_value: 0.3,
        });
        b.mappings[0].range = (100.0, 8000.0);

        let diff = AstDiff::diff(&a, &b);
        assert!(!diff.is_empty());
        assert!(!diff.is_performance_safe()); // has track changes

        let result = diff.apply(&a).unwrap();
        assert!((result.tempo - 140.0).abs() < f64::EPSILON);
        assert_eq!(result.tracks.len(), 2);
        assert_eq!(result.tracks[0].sections[0].length_bars, 4);
        assert_eq!(result.macros.len(), 2);
    }

    #[test]
    fn diff_apply_preserves_unrelated_data() {
        let a = base_program();
        let mut b = base_program();
        b.tempo = 150.0; // Only change tempo

        let diff = AstDiff::diff(&a, &b);
        let result = diff.apply(&a).unwrap();

        // Everything else should be preserved
        assert_eq!(result.tracks.len(), 1);
        assert_eq!(result.tracks[0].name, "drums");
        assert_eq!(result.macros.len(), 1);
        assert_eq!(result.mappings.len(), 1);
    }

    #[test]
    fn multiple_sections_diff() {
        let mut a = base_program();
        a.tracks[0].sections.push(SectionDef {
            name: "chorus".to_string(),
            length_bars: 4,
            patterns: vec![],
            overrides: vec![],
        });

        let mut b = a.clone();
        b.tracks[0].sections[1].length_bars = 8;

        let diff = AstDiff::diff(&a, &b);
        assert_eq!(diff.changes.len(), 1);
        assert!(matches!(
            &diff.changes[0],
            AstChange::SectionLengthChanged {
                section_name,
                old_bars: 4,
                new_bars: 8,
                ..
            } if section_name == "chorus"
        ));
    }
}
