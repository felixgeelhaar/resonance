//! Macro mapping engine — explicit macros → target params with curves.
//!
//! Macros are named f64 values in [0.0, 1.0] that drive instrument parameters
//! through explicit mappings with configurable curves.

pub mod curve;

use std::collections::HashMap;

use crate::dsl::ast::CurveKind;
use crate::event::types::{Event, ParamId};

pub use curve::{apply_curve, map_value};

/// A mapping from a macro to a parameter target.
#[derive(Debug, Clone)]
pub struct Mapping {
    pub macro_name: String,
    pub target_param: ParamId,
    pub range: (f64, f64),
    pub curve: CurveKind,
}

/// The macro engine: holds named macros and mappings, resolves parameters.
#[derive(Debug, Clone)]
pub struct MacroEngine {
    macros: HashMap<String, f64>,
    mappings: Vec<Mapping>,
}

impl MacroEngine {
    /// Create an empty macro engine.
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            mappings: Vec::new(),
        }
    }

    /// Add a macro with a default value. Value is clamped to [0.0, 1.0].
    pub fn add_macro(&mut self, name: impl Into<String>, default: f64) {
        self.macros.insert(name.into(), default.clamp(0.0, 1.0));
    }

    /// Set a macro value. Value is clamped to [0.0, 1.0].
    /// Returns `false` if the macro doesn't exist.
    pub fn set_macro(&mut self, name: &str, value: f64) -> bool {
        if let Some(v) = self.macros.get_mut(name) {
            *v = value.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    /// Get the current value of a macro.
    pub fn get_macro(&self, name: &str) -> Option<f64> {
        self.macros.get(name).copied()
    }

    /// Adjust a macro value by a delta. Value is clamped to [0.0, 1.0].
    /// Returns `false` if the macro doesn't exist.
    pub fn adjust_macro(&mut self, name: &str, delta: f64) -> bool {
        if let Some(v) = self.macros.get_mut(name) {
            *v = (*v + delta).clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    /// Add a mapping from a macro to a parameter.
    pub fn add_mapping(&mut self, mapping: Mapping) {
        self.mappings.push(mapping);
    }

    /// Resolve all mappings into a map of ParamId → f32 values.
    pub fn resolve_params(&self) -> HashMap<ParamId, f32> {
        let mut params = HashMap::new();
        for mapping in &self.mappings {
            if let Some(&macro_val) = self.macros.get(&mapping.macro_name) {
                let value = map_value(mapping.curve, macro_val, mapping.range) as f32;
                params.insert(mapping.target_param.clone(), value);
            }
        }
        params
    }

    /// Apply all macro mappings to an event's params.
    pub fn apply_to_event(&self, event: &mut Event) {
        for (param_id, value) in self.resolve_params() {
            event.params.set(param_id, value);
        }
    }

    /// Get the number of registered macros.
    pub fn macro_count(&self) -> usize {
        self.macros.len()
    }

    /// Get the number of registered mappings.
    pub fn mapping_count(&self) -> usize {
        self.mappings.len()
    }

    /// Get all macro names and values.
    pub fn macros(&self) -> &HashMap<String, f64> {
        &self.macros
    }

    /// Build a MacroEngine from compiled song data.
    pub fn from_compiled(
        macros: &[crate::dsl::ast::MacroDef],
        mappings: &[crate::dsl::ast::MappingDef],
    ) -> Self {
        let mut engine = Self::new();
        for m in macros {
            engine.add_macro(&m.name, m.default_value);
        }
        for m in mappings {
            engine.add_mapping(Mapping {
                macro_name: m.macro_name.clone(),
                target_param: ParamId(m.target_param.clone()),
                range: m.range,
                curve: m.curve,
            });
        }
        engine
    }
}

impl Default for MacroEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::CurveKind;
    use crate::event::types::{Event, NoteOrSample, ParamId, TrackId};
    use crate::event::Beat;

    #[test]
    fn add_and_get_macro() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.5);
        assert!((engine.get_macro("filter").unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn get_nonexistent_macro_returns_none() {
        let engine = MacroEngine::new();
        assert!(engine.get_macro("missing").is_none());
    }

    #[test]
    fn set_macro_updates_value() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.5);
        assert!(engine.set_macro("filter", 0.8));
        assert!((engine.get_macro("filter").unwrap() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn set_nonexistent_macro_returns_false() {
        let mut engine = MacroEngine::new();
        assert!(!engine.set_macro("missing", 0.5));
    }

    #[test]
    fn macro_value_clamped_on_add() {
        let mut engine = MacroEngine::new();
        engine.add_macro("a", 1.5);
        engine.add_macro("b", -0.3);
        assert!((engine.get_macro("a").unwrap() - 1.0).abs() < f64::EPSILON);
        assert!((engine.get_macro("b").unwrap()).abs() < f64::EPSILON);
    }

    #[test]
    fn macro_value_clamped_on_set() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.5);
        engine.set_macro("filter", 2.0);
        assert!((engine.get_macro("filter").unwrap() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adjust_macro_adds_delta() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.5);
        engine.adjust_macro("filter", 0.2);
        assert!((engine.get_macro("filter").unwrap() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn adjust_macro_clamps() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.9);
        engine.adjust_macro("filter", 0.5);
        assert!((engine.get_macro("filter").unwrap() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adjust_nonexistent_returns_false() {
        let mut engine = MacroEngine::new();
        assert!(!engine.adjust_macro("missing", 0.1));
    }

    #[test]
    fn resolve_params_linear() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.5);
        engine.add_mapping(Mapping {
            macro_name: "filter".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        });
        let params = engine.resolve_params();
        let cutoff = params.get(&ParamId("cutoff".to_string())).unwrap();
        assert!((*cutoff - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn resolve_params_exp_curve() {
        let mut engine = MacroEngine::new();
        engine.add_macro("intensity", 0.5);
        engine.add_mapping(Mapping {
            macro_name: "intensity".to_string(),
            target_param: ParamId("drive".to_string()),
            range: (0.0, 100.0),
            curve: CurveKind::Exp,
        });
        let params = engine.resolve_params();
        let drive = *params.get(&ParamId("drive".to_string())).unwrap();
        // Exp(0.5) = 0.25, so 0.25 * 100 = 25
        assert!((drive - 25.0).abs() < 0.01);
    }

    #[test]
    fn resolve_params_with_range() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 1.0);
        engine.add_mapping(Mapping {
            macro_name: "filter".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (200.0, 8000.0),
            curve: CurveKind::Linear,
        });
        let params = engine.resolve_params();
        let cutoff = *params.get(&ParamId("cutoff".to_string())).unwrap();
        assert!((cutoff - 8000.0).abs() < 0.01);
    }

    #[test]
    fn unmapped_macro_not_in_params() {
        let mut engine = MacroEngine::new();
        engine.add_macro("unused", 0.5);
        let params = engine.resolve_params();
        assert!(params.is_empty());
    }

    #[test]
    fn mapping_without_macro_not_resolved() {
        let mut engine = MacroEngine::new();
        engine.add_mapping(Mapping {
            macro_name: "missing".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        });
        let params = engine.resolve_params();
        assert!(params.is_empty());
    }

    #[test]
    fn apply_to_event_sets_params() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.75);
        engine.add_mapping(Mapping {
            macro_name: "filter".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        });

        let mut event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        engine.apply_to_event(&mut event);

        let cutoff = event.params.get(&ParamId("cutoff".to_string())).unwrap();
        assert!((cutoff - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn multiple_mappings_resolve() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.5);
        engine.add_macro("intensity", 1.0);
        engine.add_mapping(Mapping {
            macro_name: "filter".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        });
        engine.add_mapping(Mapping {
            macro_name: "intensity".to_string(),
            target_param: ParamId("drive".to_string()),
            range: (0.0, 10.0),
            curve: CurveKind::Linear,
        });

        let params = engine.resolve_params();
        assert_eq!(params.len(), 2);
        assert!((*params.get(&ParamId("cutoff".to_string())).unwrap() - 0.5).abs() < f32::EPSILON);
        assert!((*params.get(&ParamId("drive".to_string())).unwrap() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn from_compiled_builds_engine() {
        use crate::dsl::ast::{MacroDef, MappingDef};

        let macros = vec![MacroDef {
            name: "filter".to_string(),
            default_value: 0.5,
        }];
        let mappings = vec![MappingDef {
            macro_name: "filter".to_string(),
            target_param: "cutoff".to_string(),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        }];

        let engine = MacroEngine::from_compiled(&macros, &mappings);
        assert_eq!(engine.macro_count(), 1);
        assert_eq!(engine.mapping_count(), 1);
        assert!((engine.get_macro("filter").unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn macro_count_and_mapping_count() {
        let mut engine = MacroEngine::new();
        assert_eq!(engine.macro_count(), 0);
        assert_eq!(engine.mapping_count(), 0);

        engine.add_macro("a", 0.5);
        engine.add_macro("b", 0.3);
        assert_eq!(engine.macro_count(), 2);

        engine.add_mapping(Mapping {
            macro_name: "a".to_string(),
            target_param: ParamId("x".to_string()),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        });
        assert_eq!(engine.mapping_count(), 1);
    }

    #[test]
    fn apply_preserves_existing_event_data() {
        let mut engine = MacroEngine::new();
        engine.add_macro("filter", 0.5);
        engine.add_mapping(Mapping {
            macro_name: "filter".to_string(),
            target_param: ParamId("cutoff".to_string()),
            range: (0.0, 1.0),
            curve: CurveKind::Linear,
        });

        let mut event = Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(0), 60, 0.8);
        engine.apply_to_event(&mut event);

        // Original event data should be preserved
        assert_eq!(event.trigger, NoteOrSample::Note(60));
        assert!((event.velocity - 0.8).abs() < f32::EPSILON);
        assert_eq!(event.track_id, TrackId(0));
    }
}
