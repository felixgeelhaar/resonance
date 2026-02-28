//! Taste persistence — YAML load/save/reset for the taste profile.

use std::io;
use std::path::{Path, PathBuf};

use super::profile::TasteProfile;

/// Default path for the taste profile.
pub fn default_profile_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".resonance");
    path.push("taste.yaml");
    path
}

/// Load a taste profile from a YAML file. Returns a new profile if file doesn't exist.
pub fn load_profile(path: &Path) -> Result<TasteProfile, io::Error> {
    if !path.exists() {
        return Ok(TasteProfile::new());
    }
    let content = std::fs::read_to_string(path)?;
    serde_yaml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Save a taste profile to a YAML file, creating parent directories as needed.
pub fn save_profile(path: &Path, profile: &TasteProfile) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml::to_string(profile).map_err(io::Error::other)?;
    std::fs::write(path, yaml)
}

/// Reset the taste profile by removing the file and returning a fresh profile.
pub fn reset_profile(path: &Path) -> Result<TasteProfile, io::Error> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(TasteProfile::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taste::profile::MacroPreference;
    use tempfile::NamedTempFile;

    #[test]
    fn load_nonexistent_returns_default() {
        let path = Path::new("/tmp/resonance_test_nonexistent_taste.yaml");
        // Ensure it doesn't exist
        let _ = std::fs::remove_file(path);
        let profile = load_profile(path).unwrap();
        assert_eq!(profile.version, 1);
        assert!(profile.macro_preferences.is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        let mut profile = TasteProfile::new();
        profile.macro_preferences.insert(
            "filter".to_string(),
            MacroPreference {
                preferred_value: 0.6,
                min_observed: 0.1,
                max_observed: 0.9,
                adjustment_count: 10,
            },
        );
        profile.section_usage.insert("verse".to_string(), 3);

        save_profile(path, &profile).unwrap();
        let loaded = load_profile(path).unwrap();

        assert_eq!(profile, loaded);
    }

    #[test]
    fn reset_removes_file() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        let profile = TasteProfile::new();
        save_profile(&path, &profile).unwrap();
        assert!(path.exists());

        let fresh = reset_profile(&path).unwrap();
        assert!(!path.exists());
        assert_eq!(fresh.version, 1);
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("taste.yaml");

        let profile = TasteProfile::new();
        save_profile(&path, &profile).unwrap();
        assert!(path.exists());
    }
}
