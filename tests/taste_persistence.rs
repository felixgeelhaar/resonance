//! Taste persistence integration tests — file I/O and session lifecycle.

use resonance::taste::profile::{MacroPreference, TasteProfile};
use resonance::taste::{load_profile, save_profile, TasteEngine};

#[test]
fn taste_profile_yaml_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taste.yaml");

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
    profile.section_usage.insert("chorus".to_string(), 3);
    profile
        .accepted_patterns
        .push("Added track bass".to_string());
    profile
        .rejected_patterns
        .push("Removed track drums".to_string());

    save_profile(&path, &profile).unwrap();
    let loaded = load_profile(&path).unwrap();
    assert_eq!(profile, loaded);
}

#[test]
fn taste_engine_session_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taste.yaml");

    let mut engine = TasteEngine::with_path(path.clone());
    engine.set_learning_enabled(true);

    // Record events
    engine.record_macro_movement("filter", 0.3);
    engine.record_macro_movement("filter", 0.7);
    engine.record_section_jump("verse");
    engine.record_section_jump("chorus");
    engine.record_section_jump("verse");
    engine.record_diff_accepted("Added track bass");
    engine.record_diff_rejected("Removed drums");

    // Flush to profile
    engine.flush_session();

    // Verify profile state
    let pref = engine.profile().macro_preferences.get("filter").unwrap();
    assert!((pref.preferred_value - 0.7).abs() < f64::EPSILON);
    assert!((pref.min_observed - 0.3).abs() < f64::EPSILON);
    assert!((pref.max_observed - 0.7).abs() < f64::EPSILON);
    assert_eq!(pref.adjustment_count, 2);

    assert_eq!(*engine.profile().section_usage.get("verse").unwrap(), 2);
    assert_eq!(*engine.profile().section_usage.get("chorus").unwrap(), 1);
    assert_eq!(engine.profile().accepted_patterns.len(), 1);
    assert_eq!(engine.profile().rejected_patterns.len(), 1);

    // Save to disk
    engine.save().unwrap();

    // Load in new engine
    let mut engine2 = TasteEngine::with_path(path);
    engine2.load().unwrap();
    assert_eq!(engine.profile(), engine2.profile());
}

#[test]
fn taste_engine_disabled_no_recording() {
    let mut engine = TasteEngine::new();
    // Learning disabled by default
    engine.record_macro_movement("filter", 0.5);
    engine.record_section_jump("verse");
    engine.flush_session();

    assert!(engine.profile().macro_preferences.is_empty());
    assert!(engine.profile().section_usage.is_empty());
}

#[test]
fn taste_engine_reset() {
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

#[test]
fn taste_bias_scoring() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("taste.yaml");

    let mut engine = TasteEngine::with_path(path);
    engine.set_learning_enabled(true);
    engine.record_diff_accepted("Added track bass");
    engine.record_diff_rejected("Removed track drums");
    engine.flush_session();

    // Similar to accepted → positive
    let score = engine.bias("Added track synth");
    assert!(score.is_positive());

    // Similar to rejected → negative
    let score = engine.bias("Removed track bass");
    assert!(score.is_negative());
}
