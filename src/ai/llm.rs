//! Optional LLM client — feature-gated behind `llm` feature flag.
//!
//! Sends natural language input plus current DSL source to an LLM API,
//! which returns modified DSL source. The system prompt teaches the LLM
//! the Resonance DSL grammar so it can produce valid code.

use super::config::AiConfig;

/// System prompt that teaches the LLM the Resonance DSL grammar.
const SYSTEM_PROMPT: &str = r#"You are an AI assistant for Resonance, a terminal-native live coding music instrument.
Your job is to modify DSL source code based on natural language instructions.

IMPORTANT: Return ONLY the modified DSL source code, no explanation, no markdown, no code fences.

Resonance DSL grammar:
- `tempo NNN` — set BPM (20-999)
- `macro NAME = VALUE` — define a macro (0.0-1.0)
- `map MACRO -> PARAM (MIN..MAX) CURVE` — map macro to parameter
  Curves: linear, exp, log, smoothstep
- `track NAME { TYPE sections... }` — define a track
  Types: `kit: default` (drums), `bass`, `poly` (chords), `pluck`, `noise`
- `section NAME [N bars] { patterns... }` — section within a track
- Drum patterns: `kick: [X . . . X . . .]` (X=loud, x=soft, .=silent, 16 steps)
  Voices: kick, snare, hat, clap, tom, rim
- Note patterns: `note: [C2 . . C2 . . Eb2 .]` (notes with octave)
- `layer NAME { MACRO -> PARAM (MIN..MAX) CURVE }` — optional layer
- Parameters: cutoff, reverb_mix, reverb_decay, delay_mix, delay_feedback, drive, attack

Example:
```
tempo 124
macro feel = 0.4
map feel -> cutoff (200.0..6000.0) exp
track drums {
  kit: default
  section main [4 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    snare: [. . . . X . . . . . . . X . . .]
  }
}
```
"#;

/// LLM client for transforming DSL source via API calls.
pub struct LlmClient {
    api_url: String,
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

impl LlmClient {
    /// Create an LLM client from config. Returns None if config is missing or disabled.
    pub fn from_config(config: &AiConfig) -> Option<Self> {
        if !config.enabled || config.api_key.is_empty() {
            return None;
        }
        Some(Self {
            api_url: if config.api_url.is_empty() {
                "https://api.openai.com/v1/chat/completions".to_string()
            } else {
                config.api_url.clone()
            },
            api_key: config.api_key.clone(),
            model: if config.model.is_empty() {
                "gpt-4o-mini".to_string()
            } else {
                config.model.clone()
            },
            client: reqwest::blocking::Client::new(),
        })
    }

    /// Transform DSL source based on natural language input.
    /// Returns the proposed new DSL source.
    pub fn transform(&self, nl_input: &str, current_source: &str) -> Result<String, String> {
        let user_msg = format!(
            "Current DSL source:\n```\n{}\n```\n\nInstruction: {}\n\nReturn only the modified DSL source.",
            current_source, nl_input
        );

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_msg}
            ],
            "temperature": 0.3,
            "max_tokens": 2000
        });

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(format!("API error {status}: {text}"));
        }

        let json: serde_json::Value = response
            .json()
            .map_err(|e| format!("failed to parse response: {e}"))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "no content in response".to_string())?;

        // Strip markdown code fences if present
        let cleaned = content
            .trim()
            .strip_prefix("```")
            .and_then(|s| s.strip_suffix("```"))
            .unwrap_or(content)
            .trim()
            .strip_prefix("dsl\n")
            .or_else(|| content.strip_prefix("```dsl\n"))
            .unwrap_or(content)
            .trim()
            .to_string();

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_disabled_config_returns_none() {
        let config = AiConfig::default();
        assert!(LlmClient::from_config(&config).is_none());
    }

    #[test]
    fn from_config_without_key_returns_none() {
        let config = AiConfig {
            enabled: true,
            api_key: String::new(),
            ..AiConfig::default()
        };
        assert!(LlmClient::from_config(&config).is_none());
    }

    #[test]
    fn from_valid_config_returns_some() {
        let config = AiConfig {
            enabled: true,
            api_key: "sk-test".to_string(),
            ..AiConfig::default()
        };
        assert!(LlmClient::from_config(&config).is_some());
    }
}
