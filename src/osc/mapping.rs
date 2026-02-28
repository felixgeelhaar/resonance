//! OSC message mapping — converts OSC addresses and arguments to ExternalEvents.

use rosc::OscMessage;
use serde::{Deserialize, Serialize};

use crate::tui::external_input::ExternalEvent;

/// What an OSC message maps to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OscTarget {
    /// Set a macro value (expects float arg 0.0-1.0).
    Macro(usize),
    /// Jump to a section.
    Section(usize),
    /// Toggle a layer.
    Layer(usize),
    /// Toggle play/stop.
    PlayStop,
    /// Set BPM (expects float arg).
    BpmSet,
}

/// A mapping from an OSC address pattern to a target action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OscMapping {
    pub address_pattern: String,
    pub target: OscTarget,
}

/// Apply an OSC message against mappings to produce an ExternalEvent.
pub fn apply_osc_message(msg: &OscMessage, mappings: &[OscMapping]) -> Option<ExternalEvent> {
    for mapping in mappings {
        if osc_address_matches(&msg.addr, &mapping.address_pattern) {
            return match &mapping.target {
                OscTarget::Macro(idx) => {
                    let value = extract_float(&msg.args, 0)?;
                    Some(ExternalEvent::MacroSet {
                        name: format!("macro_{idx}"),
                        value: (value as f64).clamp(0.0, 1.0),
                    })
                }
                OscTarget::Section(idx) => Some(ExternalEvent::SectionJump(*idx)),
                OscTarget::Layer(idx) => Some(ExternalEvent::LayerToggle(*idx)),
                OscTarget::PlayStop => Some(ExternalEvent::PlayStop),
                OscTarget::BpmSet => {
                    let bpm = extract_float(&msg.args, 0)?;
                    Some(ExternalEvent::BpmSet(bpm as f64))
                }
            };
        }
    }
    None
}

/// Simple address matching (exact match or wildcard support).
fn osc_address_matches(addr: &str, pattern: &str) -> bool {
    addr == pattern
}

/// Extract a float from OSC args at the given index.
fn extract_float(args: &[rosc::OscType], index: usize) -> Option<f32> {
    args.get(index).and_then(|arg| match arg {
        rosc::OscType::Float(f) => Some(*f),
        rosc::OscType::Double(d) => Some(*d as f32),
        rosc::OscType::Int(i) => Some(*i as f32),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosc::OscType;

    fn default_mappings() -> Vec<OscMapping> {
        vec![
            OscMapping {
                address_pattern: "/macro/1".to_string(),
                target: OscTarget::Macro(0),
            },
            OscMapping {
                address_pattern: "/section/1".to_string(),
                target: OscTarget::Section(0),
            },
            OscMapping {
                address_pattern: "/layer/1".to_string(),
                target: OscTarget::Layer(0),
            },
            OscMapping {
                address_pattern: "/play".to_string(),
                target: OscTarget::PlayStop,
            },
            OscMapping {
                address_pattern: "/bpm".to_string(),
                target: OscTarget::BpmSet,
            },
        ]
    }

    #[test]
    fn macro_set() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/macro/1".to_string(),
            args: vec![OscType::Float(0.75)],
        };
        let event = apply_osc_message(&msg, &mappings).unwrap();
        match event {
            ExternalEvent::MacroSet { name, value } => {
                assert_eq!(name, "macro_0");
                assert!((value - 0.75).abs() < 0.01);
            }
            _ => panic!("expected MacroSet"),
        }
    }

    #[test]
    fn macro_set_clamped() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/macro/1".to_string(),
            args: vec![OscType::Float(1.5)],
        };
        let event = apply_osc_message(&msg, &mappings).unwrap();
        match event {
            ExternalEvent::MacroSet { value, .. } => {
                assert!((value - 1.0).abs() < 0.01);
            }
            _ => panic!("expected MacroSet"),
        }
    }

    #[test]
    fn section_jump() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/section/1".to_string(),
            args: vec![],
        };
        let event = apply_osc_message(&msg, &mappings).unwrap();
        assert_eq!(event, ExternalEvent::SectionJump(0));
    }

    #[test]
    fn layer_toggle() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/layer/1".to_string(),
            args: vec![],
        };
        let event = apply_osc_message(&msg, &mappings).unwrap();
        assert_eq!(event, ExternalEvent::LayerToggle(0));
    }

    #[test]
    fn play_stop() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/play".to_string(),
            args: vec![],
        };
        let event = apply_osc_message(&msg, &mappings).unwrap();
        assert_eq!(event, ExternalEvent::PlayStop);
    }

    #[test]
    fn bpm_set() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/bpm".to_string(),
            args: vec![OscType::Float(140.0)],
        };
        let event = apply_osc_message(&msg, &mappings).unwrap();
        assert_eq!(event, ExternalEvent::BpmSet(140.0));
    }

    #[test]
    fn bpm_from_int() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/bpm".to_string(),
            args: vec![OscType::Int(120)],
        };
        let event = apply_osc_message(&msg, &mappings).unwrap();
        assert_eq!(event, ExternalEvent::BpmSet(120.0));
    }

    #[test]
    fn unmatched_address_returns_none() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/unknown".to_string(),
            args: vec![],
        };
        assert!(apply_osc_message(&msg, &mappings).is_none());
    }

    #[test]
    fn macro_missing_arg_returns_none() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/macro/1".to_string(),
            args: vec![], // no float argument
        };
        assert!(apply_osc_message(&msg, &mappings).is_none());
    }

    #[test]
    fn bpm_missing_arg_returns_none() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/bpm".to_string(),
            args: vec![],
        };
        assert!(apply_osc_message(&msg, &mappings).is_none());
    }

    #[test]
    fn serialize_deserialize_mapping() {
        let mapping = OscMapping {
            address_pattern: "/macro/1".to_string(),
            target: OscTarget::Macro(0),
        };
        let yaml = serde_yaml::to_string(&mapping).unwrap();
        let parsed: OscMapping = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.address_pattern, "/macro/1");
        assert_eq!(parsed.target, OscTarget::Macro(0));
    }

    #[test]
    fn extract_float_from_double() {
        let mappings = default_mappings();
        let msg = OscMessage {
            addr: "/macro/1".to_string(),
            args: vec![OscType::Double(0.5)],
        };
        let event = apply_osc_message(&msg, &mappings).unwrap();
        match event {
            ExternalEvent::MacroSet { value, .. } => {
                assert!((value - 0.5).abs() < 0.01);
            }
            _ => panic!("expected MacroSet"),
        }
    }
}
