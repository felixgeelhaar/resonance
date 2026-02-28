//! Taste profile — serializable user preference data.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dsl::ast::CurveKind;

/// Persistent taste profile stored at `~/.resonance/taste.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TasteProfile {
    /// Preferred macro ranges and last-known values.
    pub macro_preferences: HashMap<String, MacroPreference>,
    /// Section usage counts.
    pub section_usage: HashMap<String, u32>,
    /// Patterns from accepted structural diffs.
    pub accepted_patterns: Vec<String>,
    /// Patterns from rejected structural diffs.
    pub rejected_patterns: Vec<String>,
    /// Preferred curve types per mapping target.
    pub curve_preferences: HashMap<String, CurvePreference>,
    /// Profile schema version for forward compatibility.
    pub version: u32,
}

/// Preference data for a single macro.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroPreference {
    pub preferred_value: f64,
    pub min_observed: f64,
    pub max_observed: f64,
    pub adjustment_count: u32,
}

/// A serializable curve preference (mirrors CurveKind but serializable).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CurvePreference {
    Linear,
    Log,
    Exp,
    Smoothstep,
}

impl From<CurveKind> for CurvePreference {
    fn from(k: CurveKind) -> Self {
        match k {
            CurveKind::Linear => Self::Linear,
            CurveKind::Log => Self::Log,
            CurveKind::Exp => Self::Exp,
            CurveKind::Smoothstep => Self::Smoothstep,
        }
    }
}

impl From<CurvePreference> for CurveKind {
    fn from(p: CurvePreference) -> Self {
        match p {
            CurvePreference::Linear => Self::Linear,
            CurvePreference::Log => Self::Log,
            CurvePreference::Exp => Self::Exp,
            CurvePreference::Smoothstep => Self::Smoothstep,
        }
    }
}

impl TasteProfile {
    /// Create a new empty profile.
    pub fn new() -> Self {
        Self {
            macro_preferences: HashMap::new(),
            section_usage: HashMap::new(),
            accepted_patterns: Vec::new(),
            rejected_patterns: Vec::new(),
            curve_preferences: HashMap::new(),
            version: 1,
        }
    }
}

impl Default for TasteProfile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_is_empty() {
        let profile = TasteProfile::new();
        assert!(profile.macro_preferences.is_empty());
        assert!(profile.section_usage.is_empty());
        assert!(profile.accepted_patterns.is_empty());
        assert!(profile.rejected_patterns.is_empty());
        assert!(profile.curve_preferences.is_empty());
        assert_eq!(profile.version, 1);
    }

    #[test]
    fn yaml_round_trip() {
        let mut profile = TasteProfile::new();
        profile.macro_preferences.insert(
            "filter".to_string(),
            MacroPreference {
                preferred_value: 0.7,
                min_observed: 0.2,
                max_observed: 0.9,
                adjustment_count: 15,
            },
        );
        profile.section_usage.insert("verse".to_string(), 5);
        profile
            .accepted_patterns
            .push("Added track bass".to_string());
        profile.rejected_patterns.push("Removed drums".to_string());
        profile
            .curve_preferences
            .insert("cutoff".to_string(), CurvePreference::Exp);

        let yaml = serde_yaml::to_string(&profile).unwrap();
        let restored: TasteProfile = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(profile, restored);
    }

    #[test]
    fn curve_preference_round_trip() {
        let kinds = [
            CurveKind::Linear,
            CurveKind::Log,
            CurveKind::Exp,
            CurveKind::Smoothstep,
        ];
        for kind in kinds {
            let pref = CurvePreference::from(kind);
            let back = CurveKind::from(pref);
            assert_eq!(back, kind);
        }
    }
}
