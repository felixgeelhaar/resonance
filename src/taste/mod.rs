//! Taste engine — opt-in learning, ~/.resonance/taste.yaml, proposal weighting.
//!
//! Tracks macro movements, accepted/rejected diffs, and section usage.
//! Learning is disabled by default. Never mutates active code.

pub mod bias;
pub mod persistence;
pub mod profile;
pub mod tracker;

use std::path::PathBuf;

pub use bias::{BiasScore, TasteBias};
pub use persistence::{default_profile_path, load_profile, reset_profile, save_profile};
pub use profile::TasteProfile;
pub use tracker::SessionTracker;

/// The main taste engine combining session tracking, persistent profile, and bias scoring.
#[derive(Debug, Clone)]
pub struct TasteEngine {
    profile: TasteProfile,
    session: SessionTracker,
    bias: TasteBias,
    learning_enabled: bool,
    profile_path: PathBuf,
}

impl TasteEngine {
    /// Create a new taste engine. Learning is disabled by default.
    pub fn new() -> Self {
        Self {
            profile: TasteProfile::new(),
            session: SessionTracker::new(),
            bias: TasteBias::new(),
            learning_enabled: false,
            profile_path: default_profile_path(),
        }
    }

    /// Create a taste engine with a custom profile path.
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            profile: TasteProfile::new(),
            session: SessionTracker::new(),
            bias: TasteBias::new(),
            learning_enabled: false,
            profile_path: path,
        }
    }

    /// Enable or disable learning.
    pub fn set_learning_enabled(&mut self, enabled: bool) {
        self.learning_enabled = enabled;
    }

    /// Whether learning is enabled.
    pub fn is_learning_enabled(&self) -> bool {
        self.learning_enabled
    }

    /// Record a macro movement (only if learning is enabled).
    pub fn record_macro_movement(&mut self, name: &str, value: f64) {
        if self.learning_enabled {
            self.session.record_macro_movement(name, value);
        }
    }

    /// Record a section jump (only if learning is enabled).
    pub fn record_section_jump(&mut self, section_name: &str) {
        if self.learning_enabled {
            self.session.record_section_jump(section_name);
        }
    }

    /// Record a diff acceptance (only if learning is enabled).
    pub fn record_diff_accepted(&mut self, description: &str) {
        if self.learning_enabled {
            self.session.record_diff_accepted(description);
        }
    }

    /// Record a diff rejection (only if learning is enabled).
    pub fn record_diff_rejected(&mut self, description: &str) {
        if self.learning_enabled {
            self.session.record_diff_rejected(description);
        }
    }

    /// Get a bias score for a change description.
    pub fn bias(&self, description: &str) -> BiasScore {
        self.bias.score(description, &self.profile)
    }

    /// Get a bias score for a macro value.
    pub fn bias_macro(&self, name: &str, value: f64) -> BiasScore {
        self.bias.score_macro_value(name, value, &self.profile)
    }

    /// Flush session events into the profile.
    pub fn flush_session(&mut self) {
        self.session.flush_to_profile(&mut self.profile);
        self.session.clear();
    }

    /// Save the profile to disk.
    pub fn save(&self) -> Result<(), std::io::Error> {
        save_profile(&self.profile_path, &self.profile)
    }

    /// Load the profile from disk.
    pub fn load(&mut self) -> Result<(), std::io::Error> {
        self.profile = load_profile(&self.profile_path)?;
        Ok(())
    }

    /// Reset the profile (removes file and creates fresh profile).
    pub fn reset(&mut self) -> Result<(), std::io::Error> {
        self.profile = reset_profile(&self.profile_path)?;
        self.session.clear();
        Ok(())
    }

    /// Get a reference to the current profile for inspection.
    pub fn profile(&self) -> &TasteProfile {
        &self.profile
    }
}

impl Default for TasteEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_engine_learning_disabled() {
        let engine = TasteEngine::new();
        assert!(!engine.is_learning_enabled());
        assert!(engine.profile().macro_preferences.is_empty());
    }

    #[test]
    fn learning_toggle() {
        let mut engine = TasteEngine::new();
        engine.set_learning_enabled(true);
        assert!(engine.is_learning_enabled());
        engine.set_learning_enabled(false);
        assert!(!engine.is_learning_enabled());
    }

    #[test]
    fn no_recording_when_disabled() {
        let mut engine = TasteEngine::new();
        engine.record_macro_movement("filter", 0.5);
        engine.record_section_jump("verse");
        engine.flush_session();
        assert!(engine.profile().macro_preferences.is_empty());
        assert!(engine.profile().section_usage.is_empty());
    }

    #[test]
    fn recording_when_enabled() {
        let mut engine = TasteEngine::new();
        engine.set_learning_enabled(true);
        engine.record_macro_movement("filter", 0.7);
        engine.record_section_jump("chorus");
        engine.flush_session();
        assert!(engine.profile().macro_preferences.contains_key("filter"));
        assert_eq!(*engine.profile().section_usage.get("chorus").unwrap(), 1);
    }

    #[test]
    fn bias_with_empty_profile() {
        let engine = TasteEngine::new();
        let score = engine.bias("Added track bass");
        assert_eq!(score, BiasScore::neutral());
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("taste.yaml");

        let mut engine = TasteEngine::with_path(path.clone());
        engine.set_learning_enabled(true);
        engine.record_macro_movement("filter", 0.6);
        engine.flush_session();
        engine.save().unwrap();

        let mut engine2 = TasteEngine::with_path(path);
        engine2.load().unwrap();
        assert!(engine2.profile().macro_preferences.contains_key("filter"));
    }

    #[test]
    fn reset_clears_everything() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("taste.yaml");

        let mut engine = TasteEngine::with_path(path);
        engine.set_learning_enabled(true);
        engine.record_macro_movement("filter", 0.5);
        engine.flush_session();
        engine.save().unwrap();

        engine.reset().unwrap();
        assert!(engine.profile().macro_preferences.is_empty());
    }
}
