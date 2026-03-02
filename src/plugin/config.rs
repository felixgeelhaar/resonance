//! Plugin manifest — YAML configuration for config-based instruments.

use serde::Deserialize;
use std::collections::HashMap;

/// A plugin manifest loaded from `plugin.yaml`.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub instrument: Option<InstrumentDef>,
}

/// Definition of an instrument provided by a plugin.
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentDef {
    pub kind: InstrumentKind,
    /// For samplers: trigger name → relative WAV path.
    pub samples: Option<HashMap<String, String>>,
    /// For synths: "sine", "saw", "square", "triangle".
    pub waveform: Option<String>,
    /// ADSR envelope parameters.
    pub envelope: Option<EnvelopeDef>,
    /// Filter cutoff frequency (0.0–1.0 normalized).
    pub filter_cutoff: Option<f64>,
}

/// ADSR envelope definition in plugin config.
#[derive(Debug, Clone, Deserialize)]
pub struct EnvelopeDef {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

/// The kind of instrument a plugin provides.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InstrumentKind {
    Sampler,
    Synth,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sampler_manifest() {
        let yaml = r#"
name: "808 Kit"
version: "1.0.0"
author: "Test"
description: "An 808 drum kit"
instrument:
  kind: sampler
  samples:
    kick: "kick.wav"
    snare: "snare.wav"
    hat: "hat.wav"
"#;
        let manifest: PluginManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "808 Kit");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.author.as_deref(), Some("Test"));
        let inst = manifest.instrument.unwrap();
        assert_eq!(inst.kind, InstrumentKind::Sampler);
        let samples = inst.samples.unwrap();
        assert_eq!(samples.get("kick").unwrap(), "kick.wav");
        assert_eq!(samples.len(), 3);
    }

    #[test]
    fn parse_synth_manifest() {
        let yaml = r#"
name: "Warm Pad"
version: "1.0.0"
instrument:
  kind: synth
  waveform: "saw"
  envelope:
    attack: 0.5
    decay: 0.3
    sustain: 0.6
    release: 1.0
  filter_cutoff: 0.4
"#;
        let manifest: PluginManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "Warm Pad");
        let inst = manifest.instrument.unwrap();
        assert_eq!(inst.kind, InstrumentKind::Synth);
        assert_eq!(inst.waveform.as_deref(), Some("saw"));
        let env = inst.envelope.unwrap();
        assert!((env.attack - 0.5).abs() < f64::EPSILON);
        assert!((env.sustain - 0.6).abs() < f64::EPSILON);
        assert!((inst.filter_cutoff.unwrap() - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_minimal_manifest() {
        let yaml = r#"
name: "Minimal"
version: "0.1.0"
"#;
        let manifest: PluginManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "Minimal");
        assert!(manifest.instrument.is_none());
        assert!(manifest.author.is_none());
        assert!(manifest.description.is_none());
    }
}
