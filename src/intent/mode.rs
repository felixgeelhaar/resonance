//! Intent mode detection — determines whether a diff is performance-safe or structural.

use crate::dsl::diff::AstDiff;

/// The detected mode for an intent based on its diff content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentMode {
    /// Safe for live performance: only macro/mapping/tempo changes.
    Performance,
    /// Structural changes requiring user confirmation: track/section/pattern edits.
    Structural,
}

/// Detect the appropriate intent mode for a given diff.
pub fn detect_mode(diff: &AstDiff) -> IntentMode {
    if diff.is_performance_safe() {
        IntentMode::Performance
    } else {
        IntentMode::Structural
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::*;
    use crate::dsl::diff::AstChange;

    #[test]
    fn empty_diff_is_performance() {
        let diff = AstDiff { changes: vec![] };
        assert_eq!(detect_mode(&diff), IntentMode::Performance);
    }

    #[test]
    fn macro_changes_are_performance() {
        let diff = AstDiff {
            changes: vec![
                AstChange::MacroDefaultChanged {
                    name: "filter".to_string(),
                    old: 0.5,
                    new: 0.8,
                },
                AstChange::MappingAdded {
                    mapping: MappingDef {
                        macro_name: "filter".to_string(),
                        target_param: "cutoff".to_string(),
                        range: (0.0, 1.0),
                        curve: CurveKind::Linear,
                    },
                },
            ],
        };
        assert_eq!(detect_mode(&diff), IntentMode::Performance);
    }

    #[test]
    fn tempo_change_is_performance() {
        let diff = AstDiff {
            changes: vec![AstChange::TempoChanged {
                old: 120.0,
                new: 140.0,
            }],
        };
        assert_eq!(detect_mode(&diff), IntentMode::Performance);
    }

    #[test]
    fn track_change_is_structural() {
        let diff = AstDiff {
            changes: vec![AstChange::TrackAdded {
                track: TrackDef {
                    name: "bass".to_string(),
                    instrument: InstrumentRef::Bass,
                    sections: vec![],
                },
            }],
        };
        assert_eq!(detect_mode(&diff), IntentMode::Structural);
    }

    #[test]
    fn pattern_change_is_structural() {
        let diff = AstDiff {
            changes: vec![AstChange::PatternChanged {
                track_name: "drums".to_string(),
                section_name: "main".to_string(),
                target: "kick".to_string(),
                old_steps: vec![Step::Hit, Step::Rest],
                new_steps: vec![Step::Hit, Step::Hit],
            }],
        };
        assert_eq!(detect_mode(&diff), IntentMode::Structural);
    }

    #[test]
    fn mixed_changes_are_structural() {
        let diff = AstDiff {
            changes: vec![
                AstChange::MacroDefaultChanged {
                    name: "filter".to_string(),
                    old: 0.5,
                    new: 0.8,
                },
                AstChange::TrackRemoved {
                    name: "drums".to_string(),
                },
            ],
        };
        assert_eq!(detect_mode(&diff), IntentMode::Structural);
    }

    #[test]
    fn section_change_is_structural() {
        let diff = AstDiff {
            changes: vec![AstChange::SectionAdded {
                track_name: "drums".to_string(),
                section: SectionDef {
                    name: "chorus".to_string(),
                    length_bars: 4,
                    patterns: vec![],
                    overrides: vec![],
                },
            }],
        };
        assert_eq!(detect_mode(&diff), IntentMode::Structural);
    }
}
