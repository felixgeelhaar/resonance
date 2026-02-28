//! OSC configuration — listen port and mapping rules loaded from ~/.resonance/osc.yaml.

use serde::{Deserialize, Serialize};

use super::mapping::OscMapping;

/// OSC configuration loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OscConfig {
    /// UDP port to listen on.
    #[serde(default = "default_port")]
    pub listen_port: u16,
    /// Mapping rules from OSC addresses to ExternalEvents.
    #[serde(default = "OscConfig::default_mappings")]
    pub mappings: Vec<OscMapping>,
}

fn default_port() -> u16 {
    9000
}

impl OscConfig {
    /// Load config from the standard path (~/.resonance/osc.yaml).
    /// Returns None if the file doesn't exist (graceful fallback).
    pub fn load() -> Option<Self> {
        let home = dirs::home_dir()?;
        let path = home.join(".resonance").join("osc.yaml");
        let content = std::fs::read_to_string(path).ok()?;
        serde_yaml::from_str(&content).ok()
    }

    /// Default mappings: /macro/1-8, /section/1-8, /play, /bpm.
    fn default_mappings() -> Vec<OscMapping> {
        use super::mapping::OscTarget;

        let mut mappings = Vec::new();
        for i in 0..8 {
            mappings.push(OscMapping {
                address_pattern: format!("/macro/{}", i + 1),
                target: OscTarget::Macro(i),
            });
        }
        for i in 0..8 {
            mappings.push(OscMapping {
                address_pattern: format!("/section/{}", i + 1),
                target: OscTarget::Section(i),
            });
        }
        for i in 0..8 {
            mappings.push(OscMapping {
                address_pattern: format!("/layer/{}", i + 1),
                target: OscTarget::Layer(i),
            });
        }
        mappings.push(OscMapping {
            address_pattern: "/play".to_string(),
            target: OscTarget::PlayStop,
        });
        mappings.push(OscMapping {
            address_pattern: "/bpm".to_string(),
            target: OscTarget::BpmSet,
        });
        mappings
    }
}

impl Default for OscConfig {
    fn default() -> Self {
        Self {
            listen_port: default_port(),
            mappings: Self::default_mappings(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = OscConfig::default();
        assert_eq!(config.listen_port, 9000);
        assert!(!config.mappings.is_empty());
    }

    #[test]
    fn serialize_deserialize() {
        let config = OscConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: OscConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.listen_port, 9000);
        assert_eq!(parsed.mappings.len(), config.mappings.len());
    }

    #[test]
    fn custom_config_deserialize() {
        let yaml = r#"
listen_port: 8000
mappings:
  - address_pattern: "/filter"
    target: !Macro 0
"#;
        let config: OscConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.listen_port, 8000);
        assert_eq!(config.mappings.len(), 1);
    }

    #[test]
    fn load_missing_file_returns_none() {
        let _ = OscConfig::load();
    }
}
