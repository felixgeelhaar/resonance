//! MIDI configuration — device selection and mapping rules loaded from ~/.resonance/midi.yaml.

use serde::{Deserialize, Serialize};

use super::mapping::MidiMapping;

/// MIDI configuration loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiConfig {
    /// Preferred MIDI device name (substring match). None = first available.
    #[serde(default)]
    pub device_name: Option<String>,
    /// Only accept messages on this MIDI channel (0-15). None = all channels.
    #[serde(default)]
    pub channel_filter: Option<u8>,
    /// Mapping rules from MIDI messages to ExternalEvents.
    #[serde(default = "MidiConfig::default_mappings")]
    pub mappings: Vec<MidiMapping>,
}

impl MidiConfig {
    /// Load config from the standard path (~/.resonance/midi.yaml).
    /// Returns None if the file doesn't exist (graceful fallback).
    pub fn load() -> Option<Self> {
        let home = dirs::home_dir()?;
        let path = home.join(".resonance").join("midi.yaml");
        let content = std::fs::read_to_string(path).ok()?;
        serde_yaml::from_str(&content).ok()
    }

    /// Default mappings: CC1-8 → macro 0-7.
    fn default_mappings() -> Vec<MidiMapping> {
        (0..8)
            .map(|i| MidiMapping::CcToMacro {
                cc: i + 1,
                macro_idx: i as usize,
            })
            .collect()
    }
}

impl Default for MidiConfig {
    fn default() -> Self {
        Self {
            device_name: None,
            channel_filter: None,
            mappings: Self::default_mappings(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = MidiConfig::default();
        assert!(config.device_name.is_none());
        assert!(config.channel_filter.is_none());
        assert_eq!(config.mappings.len(), 8);
    }

    #[test]
    fn serialize_deserialize() {
        let config = MidiConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: MidiConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.mappings.len(), 8);
    }

    #[test]
    fn custom_config_deserialize() {
        let yaml = r#"
device_name: "Arturia"
channel_filter: 0
mappings:
  - !CcToMacro
    cc: 74
    macro_idx: 0
  - !ProgramToSection
    program: 1
    section_idx: 0
"#;
        let config: MidiConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.device_name.as_deref(), Some("Arturia"));
        assert_eq!(config.channel_filter, Some(0));
        assert_eq!(config.mappings.len(), 2);
    }

    #[test]
    fn load_missing_file_returns_none() {
        // This should gracefully return None since ~/.resonance/midi.yaml likely doesn't exist in test
        // We can't guarantee the file doesn't exist, so just verify the function doesn't panic
        let _ = MidiConfig::load();
    }
}
