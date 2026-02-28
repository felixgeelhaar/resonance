//! Session tracker — records macro movements, section jumps, and diff decisions
//! during a single session. Flushed to the profile on demand.

use super::profile::{MacroPreference, TasteProfile};

/// Events tracked during a session.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    MacroMovement { name: String, value: f64 },
    SectionJump { section_name: String },
    DiffAccepted { description: String },
    DiffRejected { description: String },
}

/// Accumulates session events for later flushing to a profile.
#[derive(Debug, Clone)]
pub struct SessionTracker {
    events: Vec<SessionEvent>,
}

impl SessionTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Record a macro movement.
    pub fn record_macro_movement(&mut self, name: &str, value: f64) {
        self.events.push(SessionEvent::MacroMovement {
            name: name.to_string(),
            value,
        });
    }

    /// Record a section jump.
    pub fn record_section_jump(&mut self, section_name: &str) {
        self.events.push(SessionEvent::SectionJump {
            section_name: section_name.to_string(),
        });
    }

    /// Record an accepted diff.
    pub fn record_diff_accepted(&mut self, description: &str) {
        self.events.push(SessionEvent::DiffAccepted {
            description: description.to_string(),
        });
    }

    /// Record a rejected diff.
    pub fn record_diff_rejected(&mut self, description: &str) {
        self.events.push(SessionEvent::DiffRejected {
            description: description.to_string(),
        });
    }

    /// Get the number of recorded events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Flush session events into the profile, updating preferences.
    pub fn flush_to_profile(&self, profile: &mut TasteProfile) {
        for event in &self.events {
            match event {
                SessionEvent::MacroMovement { name, value } => {
                    let pref = profile
                        .macro_preferences
                        .entry(name.clone())
                        .or_insert_with(|| MacroPreference {
                            preferred_value: *value,
                            min_observed: *value,
                            max_observed: *value,
                            adjustment_count: 0,
                        });
                    pref.preferred_value = *value;
                    if *value < pref.min_observed {
                        pref.min_observed = *value;
                    }
                    if *value > pref.max_observed {
                        pref.max_observed = *value;
                    }
                    pref.adjustment_count += 1;
                }
                SessionEvent::SectionJump { section_name } => {
                    *profile
                        .section_usage
                        .entry(section_name.clone())
                        .or_insert(0) += 1;
                }
                SessionEvent::DiffAccepted { description } => {
                    if !profile.accepted_patterns.contains(description) {
                        profile.accepted_patterns.push(description.clone());
                    }
                }
                SessionEvent::DiffRejected { description } => {
                    if !profile.rejected_patterns.contains(description) {
                        profile.rejected_patterns.push(description.clone());
                    }
                }
            }
        }
    }

    /// Clear all recorded events.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for SessionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_is_empty() {
        let tracker = SessionTracker::new();
        assert_eq!(tracker.event_count(), 0);
    }

    #[test]
    fn records_macro_movement() {
        let mut tracker = SessionTracker::new();
        tracker.record_macro_movement("filter", 0.7);
        assert_eq!(tracker.event_count(), 1);
    }

    #[test]
    fn records_section_jump() {
        let mut tracker = SessionTracker::new();
        tracker.record_section_jump("chorus");
        assert_eq!(tracker.event_count(), 1);
    }

    #[test]
    fn records_diff_decisions() {
        let mut tracker = SessionTracker::new();
        tracker.record_diff_accepted("Added bass track");
        tracker.record_diff_rejected("Removed drums");
        assert_eq!(tracker.event_count(), 2);
    }

    #[test]
    fn flush_updates_macro_preferences() {
        let mut tracker = SessionTracker::new();
        tracker.record_macro_movement("filter", 0.3);
        tracker.record_macro_movement("filter", 0.8);
        tracker.record_macro_movement("filter", 0.5);

        let mut profile = TasteProfile::new();
        tracker.flush_to_profile(&mut profile);

        let pref = profile.macro_preferences.get("filter").unwrap();
        assert!((pref.preferred_value - 0.5).abs() < f64::EPSILON); // Last value
        assert!((pref.min_observed - 0.3).abs() < f64::EPSILON);
        assert!((pref.max_observed - 0.8).abs() < f64::EPSILON);
        assert_eq!(pref.adjustment_count, 3);
    }

    #[test]
    fn flush_updates_section_usage() {
        let mut tracker = SessionTracker::new();
        tracker.record_section_jump("verse");
        tracker.record_section_jump("chorus");
        tracker.record_section_jump("verse");

        let mut profile = TasteProfile::new();
        tracker.flush_to_profile(&mut profile);

        assert_eq!(*profile.section_usage.get("verse").unwrap(), 2);
        assert_eq!(*profile.section_usage.get("chorus").unwrap(), 1);
    }

    #[test]
    fn flush_updates_diff_patterns() {
        let mut tracker = SessionTracker::new();
        tracker.record_diff_accepted("Added bass");
        tracker.record_diff_rejected("Removed drums");

        let mut profile = TasteProfile::new();
        tracker.flush_to_profile(&mut profile);

        assert_eq!(profile.accepted_patterns, vec!["Added bass"]);
        assert_eq!(profile.rejected_patterns, vec!["Removed drums"]);
    }

    #[test]
    fn flush_deduplicates_patterns() {
        let mut tracker = SessionTracker::new();
        tracker.record_diff_accepted("Added bass");
        tracker.record_diff_accepted("Added bass");

        let mut profile = TasteProfile::new();
        tracker.flush_to_profile(&mut profile);

        assert_eq!(profile.accepted_patterns.len(), 1);
    }

    #[test]
    fn clear_removes_events() {
        let mut tracker = SessionTracker::new();
        tracker.record_macro_movement("filter", 0.5);
        tracker.clear();
        assert_eq!(tracker.event_count(), 0);
    }
}
