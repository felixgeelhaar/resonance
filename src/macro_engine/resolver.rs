//! Mapping conflict resolution — precedence rules for multiple mapping sources.
//!
//! When multiple sources (base, section override, layer) target the same parameter,
//! the resolver determines which mapping wins based on precedence:
//! `Base < SectionOverride < Layer(0) < Layer(1) < ...`

use crate::event::types::ParamId;

use super::Mapping;

/// The source of a mapping — determines precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MappingSource {
    /// Base mapping from the global `map` declarations.
    Base,
    /// Section-specific mapping override.
    SectionOverride,
    /// Layer mapping, ordered by layer index (higher index = higher precedence).
    Layer(usize),
}

/// A mapping tagged with its source for conflict resolution.
#[derive(Debug, Clone)]
pub struct SourcedMapping {
    pub mapping: Mapping,
    pub source: MappingSource,
}

/// A detected conflict between multiple sources targeting the same parameter.
#[derive(Debug, Clone)]
pub struct MappingConflict {
    pub param: ParamId,
    pub sources: Vec<MappingSource>,
    pub winner: MappingSource,
}

/// Resolve a set of sourced mappings into winners and detected conflicts.
///
/// For each target parameter, the mapping with the highest-precedence source wins.
/// Returns (winners, conflicts) where conflicts only include params targeted by 2+ sources.
pub fn resolve_mappings(
    sourced: &[SourcedMapping],
) -> (Vec<&SourcedMapping>, Vec<MappingConflict>) {
    use std::collections::HashMap;

    // Group by target param
    let mut by_param: HashMap<&ParamId, Vec<&SourcedMapping>> = HashMap::new();
    for sm in sourced {
        by_param
            .entry(&sm.mapping.target_param)
            .or_default()
            .push(sm);
    }

    let mut winners = Vec::new();
    let mut conflicts = Vec::new();

    for (param, mut sources) in by_param {
        // Sort by precedence (highest last)
        sources.sort_by_key(|s| s.source);

        if sources.len() > 1 {
            let winner_source = sources.last().unwrap().source;
            let conflict_sources: Vec<MappingSource> = sources.iter().map(|s| s.source).collect();
            conflicts.push(MappingConflict {
                param: param.clone(),
                sources: conflict_sources,
                winner: winner_source,
            });
        }

        // Winner is the last (highest precedence)
        winners.push(*sources.last().unwrap());
    }

    // Sort winners by param for deterministic output
    winners.sort_by(|a, b| a.mapping.target_param.0.cmp(&b.mapping.target_param.0));

    (winners, conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::CurveKind;

    fn mapping(param: &str, macro_name: &str) -> Mapping {
        Mapping {
            macro_name: macro_name.to_string(),
            target_param: ParamId(param.to_string()),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        }
    }

    #[test]
    fn base_wins_alone() {
        let sourced = vec![SourcedMapping {
            mapping: mapping("cutoff", "filter"),
            source: MappingSource::Base,
        }];
        let (winners, conflicts) = resolve_mappings(&sourced);
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].source, MappingSource::Base);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn section_override_beats_base() {
        let sourced = vec![
            SourcedMapping {
                mapping: mapping("cutoff", "filter"),
                source: MappingSource::Base,
            },
            SourcedMapping {
                mapping: mapping("cutoff", "filter_section"),
                source: MappingSource::SectionOverride,
            },
        ];
        let (winners, conflicts) = resolve_mappings(&sourced);
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].source, MappingSource::SectionOverride);
        assert_eq!(winners[0].mapping.macro_name, "filter_section");
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].winner, MappingSource::SectionOverride);
    }

    #[test]
    fn layer_beats_section_override() {
        let sourced = vec![
            SourcedMapping {
                mapping: mapping("cutoff", "filter"),
                source: MappingSource::SectionOverride,
            },
            SourcedMapping {
                mapping: mapping("cutoff", "filter_layer"),
                source: MappingSource::Layer(0),
            },
        ];
        let (winners, conflicts) = resolve_mappings(&sourced);
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].source, MappingSource::Layer(0));
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn higher_layer_beats_lower_layer() {
        let sourced = vec![
            SourcedMapping {
                mapping: mapping("reverb_mix", "depth_low"),
                source: MappingSource::Layer(0),
            },
            SourcedMapping {
                mapping: mapping("reverb_mix", "depth_high"),
                source: MappingSource::Layer(2),
            },
        ];
        let (winners, _conflicts) = resolve_mappings(&sourced);
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].source, MappingSource::Layer(2));
        assert_eq!(winners[0].mapping.macro_name, "depth_high");
    }

    #[test]
    fn no_conflict_for_different_params() {
        let sourced = vec![
            SourcedMapping {
                mapping: mapping("cutoff", "filter"),
                source: MappingSource::Base,
            },
            SourcedMapping {
                mapping: mapping("drive", "intensity"),
                source: MappingSource::SectionOverride,
            },
        ];
        let (winners, conflicts) = resolve_mappings(&sourced);
        assert_eq!(winners.len(), 2);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn conflict_detection_reports_all_sources() {
        let sourced = vec![
            SourcedMapping {
                mapping: mapping("cutoff", "a"),
                source: MappingSource::Base,
            },
            SourcedMapping {
                mapping: mapping("cutoff", "b"),
                source: MappingSource::SectionOverride,
            },
            SourcedMapping {
                mapping: mapping("cutoff", "c"),
                source: MappingSource::Layer(0),
            },
        ];
        let (winners, conflicts) = resolve_mappings(&sourced);
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].source, MappingSource::Layer(0));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].sources.len(), 3);
        assert_eq!(conflicts[0].winner, MappingSource::Layer(0));
    }

    #[test]
    fn full_precedence_chain() {
        let sourced = vec![
            SourcedMapping {
                mapping: mapping("cutoff", "base"),
                source: MappingSource::Base,
            },
            SourcedMapping {
                mapping: mapping("cutoff", "section"),
                source: MappingSource::SectionOverride,
            },
            SourcedMapping {
                mapping: mapping("cutoff", "layer0"),
                source: MappingSource::Layer(0),
            },
            SourcedMapping {
                mapping: mapping("cutoff", "layer1"),
                source: MappingSource::Layer(1),
            },
        ];
        let (winners, conflicts) = resolve_mappings(&sourced);
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].source, MappingSource::Layer(1));
        assert_eq!(winners[0].mapping.macro_name, "layer1");
        assert_eq!(conflicts[0].sources.len(), 4);
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let (winners, conflicts) = resolve_mappings(&[]);
        assert!(winners.is_empty());
        assert!(conflicts.is_empty());
    }

    #[test]
    fn multiple_params_with_mixed_conflicts() {
        let sourced = vec![
            // cutoff: base + layer(0) → conflict
            SourcedMapping {
                mapping: mapping("cutoff", "filter"),
                source: MappingSource::Base,
            },
            SourcedMapping {
                mapping: mapping("cutoff", "filter_layer"),
                source: MappingSource::Layer(0),
            },
            // drive: base only → no conflict
            SourcedMapping {
                mapping: mapping("drive", "intensity"),
                source: MappingSource::Base,
            },
            // reverb: section + layer(1) → conflict
            SourcedMapping {
                mapping: mapping("reverb_mix", "depth_section"),
                source: MappingSource::SectionOverride,
            },
            SourcedMapping {
                mapping: mapping("reverb_mix", "depth_layer"),
                source: MappingSource::Layer(1),
            },
        ];
        let (winners, conflicts) = resolve_mappings(&sourced);
        assert_eq!(winners.len(), 3); // cutoff, drive, reverb_mix
        assert_eq!(conflicts.len(), 2); // cutoff, reverb_mix
    }

    #[test]
    fn mapping_source_ordering() {
        assert!(MappingSource::Base < MappingSource::SectionOverride);
        assert!(MappingSource::SectionOverride < MappingSource::Layer(0));
        assert!(MappingSource::Layer(0) < MappingSource::Layer(1));
        assert!(MappingSource::Layer(1) < MappingSource::Layer(99));
    }

    #[test]
    fn winners_sorted_deterministically() {
        let sourced = vec![
            SourcedMapping {
                mapping: mapping("z_param", "macro_z"),
                source: MappingSource::Base,
            },
            SourcedMapping {
                mapping: mapping("a_param", "macro_a"),
                source: MappingSource::Base,
            },
        ];
        let (winners, _) = resolve_mappings(&sourced);
        assert_eq!(winners[0].mapping.target_param.0, "a_param");
        assert_eq!(winners[1].mapping.target_param.0, "z_param");
    }
}
