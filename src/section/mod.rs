//! Section/Layer controller — quantized transitions, scene jumping, layer enable/disable.
//!
//! Manages the section lifecycle: which section is active, pending transitions
//! that fire on bar boundaries, and layers that can be toggled on/off.

pub mod transition;

pub use transition::QuantizedTransitionManager;

use crate::event::beat::{Beat, DEFAULT_BEATS_PER_BAR, TICKS_PER_BEAT};
use crate::macro_engine::Mapping;

/// A section: a named region with a length and optional mapping overrides.
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub length_in_bars: u32,
    pub mapping_overrides: Vec<Mapping>,
}

/// A layer: a named set of mapping additions that can be toggled.
#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub mapping_additions: Vec<Mapping>,
    pub enabled: bool,
}

/// Controls section transitions, layer toggling, and active mapping resolution.
#[derive(Debug, Clone)]
pub struct SectionController {
    sections: Vec<Section>,
    layers: Vec<Layer>,
    active_idx: usize,
    pending_transition: Option<PendingTransition>,
    transition_mgr: QuantizedTransitionManager,
    loop_length_bars: Option<u32>,
}

/// A transition waiting to fire at a bar boundary.
#[derive(Debug, Clone)]
struct PendingTransition {
    target_idx: usize,
    fire_at: Beat,
}

impl SectionController {
    /// Create a new section controller with the given sections.
    pub fn new(sections: Vec<Section>) -> Self {
        Self {
            sections,
            layers: Vec::new(),
            active_idx: 0,
            pending_transition: None,
            transition_mgr: QuantizedTransitionManager::default(),
            loop_length_bars: None,
        }
    }

    /// Get the currently active section.
    pub fn active_section(&self) -> Option<&Section> {
        self.sections.get(self.active_idx)
    }

    /// Get the active section index.
    pub fn active_index(&self) -> usize {
        self.active_idx
    }

    /// Get the number of sections.
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// Schedule a transition to the named section at the next bar boundary.
    /// Returns `false` if the section name doesn't exist.
    pub fn schedule_transition(&mut self, name: &str, current_pos: Beat) -> bool {
        if let Some(idx) = self.sections.iter().position(|s| s.name == name) {
            let fire_at = self.transition_mgr.next_bar_boundary(current_pos);
            self.pending_transition = Some(PendingTransition {
                target_idx: idx,
                fire_at,
            });
            true
        } else {
            false
        }
    }

    /// Schedule a transition by section index at the next bar boundary.
    /// Returns `false` if the index is out of range.
    pub fn schedule_transition_by_index(&mut self, idx: usize, current_pos: Beat) -> bool {
        if idx < self.sections.len() {
            let fire_at = self.transition_mgr.next_bar_boundary(current_pos);
            self.pending_transition = Some(PendingTransition {
                target_idx: idx,
                fire_at,
            });
            true
        } else {
            false
        }
    }

    /// Check if a pending transition should fire at the given position.
    /// If so, applies the transition and returns `true`.
    pub fn update(&mut self, current_pos: Beat) -> bool {
        if let Some(ref pending) = self.pending_transition {
            if current_pos >= pending.fire_at {
                self.active_idx = pending.target_idx;
                self.pending_transition = None;
                return true;
            }
        }
        false
    }

    /// Whether there is a pending transition.
    pub fn has_pending_transition(&self) -> bool {
        self.pending_transition.is_some()
    }

    /// Add a layer.
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Toggle a layer by name. Returns `false` if not found.
    pub fn toggle_layer(&mut self, name: &str) -> bool {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.name == name) {
            layer.enabled = !layer.enabled;
            true
        } else {
            false
        }
    }

    /// Get all active mappings: base section overrides + enabled layer additions.
    pub fn active_mappings(&self) -> Vec<&Mapping> {
        let mut mappings = Vec::new();

        if let Some(section) = self.sections.get(self.active_idx) {
            for m in &section.mapping_overrides {
                mappings.push(m);
            }
        }

        for layer in &self.layers {
            if layer.enabled {
                for m in &layer.mapping_additions {
                    mappings.push(m);
                }
            }
        }

        mappings
    }

    /// Set the loop length in bars. Pass `None` to disable looping.
    pub fn set_loop_length(&mut self, bars: Option<u32>) {
        self.loop_length_bars = bars;
    }

    /// Get the loop length in bars, if set.
    pub fn loop_length_bars(&self) -> Option<u32> {
        self.loop_length_bars
    }

    /// Check if a position has crossed the loop boundary and needs wrapping.
    /// Returns the wrapped position if looping is active and position exceeds
    /// the loop length, otherwise returns `None`.
    pub fn loop_wrap(&self, pos: Beat) -> Option<Beat> {
        if let Some(bars) = self.loop_length_bars {
            let loop_ticks = bars as u64 * DEFAULT_BEATS_PER_BAR as u64 * TICKS_PER_BEAT;
            if loop_ticks > 0 && pos.ticks() >= loop_ticks {
                Some(Beat::from_ticks(pos.ticks() % loop_ticks))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get a reference to the transition manager.
    pub fn transition_manager(&self) -> &QuantizedTransitionManager {
        &self.transition_mgr
    }
}

impl Default for SectionController {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::ast::CurveKind;
    use crate::event::types::ParamId;

    fn test_sections() -> Vec<Section> {
        vec![
            Section {
                name: "intro".to_string(),
                length_in_bars: 4,
                mapping_overrides: vec![],
            },
            Section {
                name: "verse".to_string(),
                length_in_bars: 8,
                mapping_overrides: vec![Mapping {
                    macro_name: "filter".to_string(),
                    target_param: ParamId("cutoff".to_string()),
                    range: (0.2, 0.8),
                    curve: CurveKind::Linear,
                }],
            },
            Section {
                name: "chorus".to_string(),
                length_in_bars: 8,
                mapping_overrides: vec![],
            },
        ]
    }

    #[test]
    fn initial_section_is_first() {
        let ctrl = SectionController::new(test_sections());
        assert_eq!(ctrl.active_index(), 0);
        assert_eq!(ctrl.active_section().unwrap().name, "intro");
    }

    #[test]
    fn schedule_transition_by_name() {
        let mut ctrl = SectionController::new(test_sections());
        assert!(ctrl.schedule_transition("verse", Beat::from_beats(2)));
        assert!(ctrl.has_pending_transition());
    }

    #[test]
    fn schedule_nonexistent_returns_false() {
        let mut ctrl = SectionController::new(test_sections());
        assert!(!ctrl.schedule_transition("bridge", Beat::ZERO));
    }

    #[test]
    fn transition_fires_at_bar_boundary() {
        let mut ctrl = SectionController::new(test_sections());
        ctrl.schedule_transition("verse", Beat::from_beats(2));

        // Before bar boundary (beat 3) — no transition
        assert!(!ctrl.update(Beat::from_beats(3)));
        assert_eq!(ctrl.active_index(), 0);

        // At bar boundary (beat 4 = bar 1 start)
        assert!(ctrl.update(Beat::from_bars(1)));
        assert_eq!(ctrl.active_index(), 1);
        assert_eq!(ctrl.active_section().unwrap().name, "verse");
        assert!(!ctrl.has_pending_transition());
    }

    #[test]
    fn transition_by_index() {
        let mut ctrl = SectionController::new(test_sections());
        assert!(ctrl.schedule_transition_by_index(2, Beat::ZERO));
        ctrl.update(Beat::from_bars(1));
        assert_eq!(ctrl.active_section().unwrap().name, "chorus");
    }

    #[test]
    fn transition_by_invalid_index_returns_false() {
        let mut ctrl = SectionController::new(test_sections());
        assert!(!ctrl.schedule_transition_by_index(10, Beat::ZERO));
    }

    #[test]
    fn layer_toggle() {
        let mut ctrl = SectionController::new(test_sections());
        ctrl.add_layer(Layer {
            name: "reverb".to_string(),
            mapping_additions: vec![Mapping {
                macro_name: "depth".to_string(),
                target_param: ParamId("reverb_mix".to_string()),
                range: (0.0, 1.0),
                curve: CurveKind::Linear,
            }],
            enabled: false,
        });

        // Initially disabled → no layer mappings
        assert_eq!(ctrl.active_mappings().len(), 0);

        // Toggle on
        assert!(ctrl.toggle_layer("reverb"));
        assert_eq!(ctrl.active_mappings().len(), 1);

        // Toggle off
        assert!(ctrl.toggle_layer("reverb"));
        assert_eq!(ctrl.active_mappings().len(), 0);
    }

    #[test]
    fn toggle_nonexistent_layer_returns_false() {
        let mut ctrl = SectionController::new(test_sections());
        assert!(!ctrl.toggle_layer("missing"));
    }

    #[test]
    fn active_mappings_merges_section_and_layers() {
        let mut ctrl = SectionController::new(test_sections());
        // Switch to verse (which has a mapping override)
        ctrl.schedule_transition("verse", Beat::ZERO);
        ctrl.update(Beat::from_bars(1));

        ctrl.add_layer(Layer {
            name: "fx".to_string(),
            mapping_additions: vec![Mapping {
                macro_name: "intensity".to_string(),
                target_param: ParamId("drive".to_string()),
                range: (0.0, 1.0),
                curve: CurveKind::Exp,
            }],
            enabled: true,
        });

        // Should have: 1 section override + 1 layer addition
        assert_eq!(ctrl.active_mappings().len(), 2);
    }

    #[test]
    fn loop_wrap_basic() {
        let mut ctrl = SectionController::new(test_sections());
        ctrl.set_loop_length(Some(4)); // 4 bars

        // Position at bar 5 should wrap to bar 1
        let wrapped = ctrl.loop_wrap(Beat::from_bars(5)).unwrap();
        assert_eq!(wrapped, Beat::from_bars(1));
    }

    #[test]
    fn loop_wrap_exact_boundary() {
        let mut ctrl = SectionController::new(test_sections());
        ctrl.set_loop_length(Some(4));

        // Position exactly at loop length should wrap to 0
        let wrapped = ctrl.loop_wrap(Beat::from_bars(4)).unwrap();
        assert_eq!(wrapped, Beat::ZERO);
    }

    #[test]
    fn loop_wrap_within_range_returns_none() {
        let mut ctrl = SectionController::new(test_sections());
        ctrl.set_loop_length(Some(4));

        // Position within loop — no wrapping needed
        assert!(ctrl.loop_wrap(Beat::from_bars(2)).is_none());
    }

    #[test]
    fn loop_wrap_disabled_returns_none() {
        let ctrl = SectionController::new(test_sections());
        assert!(ctrl.loop_wrap(Beat::from_bars(100)).is_none());
    }

    #[test]
    fn section_count() {
        let ctrl = SectionController::new(test_sections());
        assert_eq!(ctrl.section_count(), 3);
    }

    #[test]
    fn empty_controller() {
        let ctrl = SectionController::default();
        assert_eq!(ctrl.section_count(), 0);
        assert!(ctrl.active_section().is_none());
        assert!(ctrl.active_mappings().is_empty());
    }
}
