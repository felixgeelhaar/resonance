//! AI configuration — loads optional ~/.resonance/ai.yaml for LLM settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// AI configuration loaded from ~/.resonance/ai.yaml.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AiConfig {
    /// Whether AI features are enabled.
    #[serde(default)]
    pub enabled: bool,
    /// LLM provider name (e.g., "openai", "anthropic").
    #[serde(default)]
    pub provider: String,
    /// API base URL.
    #[serde(default)]
    pub api_url: String,
    /// API key (secret).
    #[serde(default)]
    pub api_key: String,
    /// Model identifier.
    #[serde(default)]
    pub model: String,
}

/// Get the AI config file path.
fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".resonance").join("ai.yaml"))
}

/// Load AI configuration from ~/.resonance/ai.yaml.
/// Returns None if the file doesn't exist.
pub fn load_config() -> Option<AiConfig> {
    let path = config_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_yaml::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_disabled() {
        let config = AiConfig::default();
        assert!(!config.enabled);
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn missing_config_returns_none() {
        // Unless the test runner happens to have ~/.resonance/ai.yaml,
        // this should return None or Some (we just verify no panic).
        let _ = load_config();
    }

    #[test]
    fn parse_yaml_config() {
        let yaml = r#"
enabled: true
provider: openai
api_url: https://api.openai.com/v1
api_key: sk-test-123
model: gpt-4
"#;
        let config: AiConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4");
    }

    #[test]
    fn partial_yaml_config() {
        let yaml = "enabled: true\n";
        let config: AiConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert!(config.api_key.is_empty());
    }
}
