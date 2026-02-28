//! MIDI message mapping — converts raw MIDI bytes to ExternalEvents.

use serde::{Deserialize, Serialize};

use crate::tui::external_input::ExternalEvent;

/// Mapping rule from MIDI messages to application events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MidiMapping {
    /// Map a CC number to a macro index (CC value 0-127 → 0.0-1.0).
    CcToMacro { cc: u8, macro_idx: usize },
    /// Map a note range to a track name.
    NoteToTrack { note_range: (u8, u8), track: String },
    /// Map a program change to a section index.
    ProgramToSection { program: u8, section_idx: usize },
}

/// Parse a raw MIDI message and apply mappings to produce an ExternalEvent.
///
/// MIDI message format:
/// - Note On:  [0x90 | channel, note, velocity]
/// - Note Off: [0x80 | channel, note, velocity]
/// - CC:       [0xB0 | channel, cc_number, value]
/// - Program:  [0xC0 | channel, program]
pub fn apply_midi_message(
    msg: &[u8],
    mappings: &[MidiMapping],
    channel_filter: Option<u8>,
) -> Option<ExternalEvent> {
    if msg.is_empty() {
        return None;
    }

    let status = msg[0] & 0xF0;
    let channel = msg[0] & 0x0F;

    // Apply channel filter
    if let Some(filter) = channel_filter {
        if channel != filter {
            return None;
        }
    }

    match status {
        // Note On
        0x90 if msg.len() >= 3 => {
            let note = msg[1];
            let velocity = msg[2];
            if velocity == 0 {
                // Note On with velocity 0 = Note Off
                return apply_note_off(note, mappings);
            }
            for mapping in mappings {
                if let MidiMapping::NoteToTrack { note_range, track } = mapping {
                    if note >= note_range.0 && note <= note_range.1 {
                        return Some(ExternalEvent::NoteOn {
                            track: track.clone(),
                            note,
                            velocity: velocity as f32 / 127.0,
                        });
                    }
                }
            }
            None
        }
        // Note Off
        0x80 if msg.len() >= 3 => {
            let note = msg[1];
            apply_note_off(note, mappings)
        }
        // CC
        0xB0 if msg.len() >= 3 => {
            let cc_number = msg[1];
            let value = msg[2];
            for mapping in mappings {
                if let MidiMapping::CcToMacro { cc, macro_idx } = mapping {
                    if cc_number == *cc {
                        return Some(ExternalEvent::MacroSet {
                            name: format!("macro_{macro_idx}"),
                            value: value as f64 / 127.0,
                        });
                    }
                }
            }
            Some(ExternalEvent::CC {
                channel,
                controller: cc_number,
                value,
            })
        }
        // Program Change
        0xC0 if msg.len() >= 2 => {
            let program = msg[1];
            for mapping in mappings {
                if let MidiMapping::ProgramToSection {
                    program: p,
                    section_idx,
                } = mapping
                {
                    if program == *p {
                        return Some(ExternalEvent::SectionJump(*section_idx));
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn apply_note_off(note: u8, mappings: &[MidiMapping]) -> Option<ExternalEvent> {
    for mapping in mappings {
        if let MidiMapping::NoteToTrack { note_range, track } = mapping {
            if note >= note_range.0 && note <= note_range.1 {
                return Some(ExternalEvent::NoteOff {
                    track: track.clone(),
                    note,
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_mappings() -> Vec<MidiMapping> {
        vec![
            MidiMapping::CcToMacro {
                cc: 1,
                macro_idx: 0,
            },
            MidiMapping::CcToMacro {
                cc: 2,
                macro_idx: 1,
            },
            MidiMapping::NoteToTrack {
                note_range: (36, 47),
                track: "drums".to_string(),
            },
            MidiMapping::ProgramToSection {
                program: 0,
                section_idx: 0,
            },
            MidiMapping::ProgramToSection {
                program: 1,
                section_idx: 1,
            },
        ]
    }

    #[test]
    fn cc_to_macro() {
        let mappings = default_mappings();
        // CC1, value 64 on channel 0
        let msg = [0xB0, 1, 64];
        let event = apply_midi_message(&msg, &mappings, None).unwrap();
        match event {
            ExternalEvent::MacroSet { name, value } => {
                assert_eq!(name, "macro_0");
                assert!((value - 64.0 / 127.0).abs() < 0.01);
            }
            _ => panic!("expected MacroSet"),
        }
    }

    #[test]
    fn cc_unmapped_returns_raw_cc() {
        let mappings = default_mappings();
        // CC74 (not mapped)
        let msg = [0xB0, 74, 100];
        let event = apply_midi_message(&msg, &mappings, None).unwrap();
        assert_eq!(
            event,
            ExternalEvent::CC {
                channel: 0,
                controller: 74,
                value: 100
            }
        );
    }

    #[test]
    fn note_on_to_track() {
        let mappings = default_mappings();
        // Note On, note 36, velocity 100
        let msg = [0x90, 36, 100];
        let event = apply_midi_message(&msg, &mappings, None).unwrap();
        match event {
            ExternalEvent::NoteOn {
                track,
                note,
                velocity,
            } => {
                assert_eq!(track, "drums");
                assert_eq!(note, 36);
                assert!((velocity - 100.0 / 127.0).abs() < 0.01);
            }
            _ => panic!("expected NoteOn"),
        }
    }

    #[test]
    fn note_on_velocity_zero_is_note_off() {
        let mappings = default_mappings();
        let msg = [0x90, 36, 0];
        let event = apply_midi_message(&msg, &mappings, None).unwrap();
        assert!(matches!(event, ExternalEvent::NoteOff { .. }));
    }

    #[test]
    fn note_off() {
        let mappings = default_mappings();
        let msg = [0x80, 40, 0];
        let event = apply_midi_message(&msg, &mappings, None).unwrap();
        match event {
            ExternalEvent::NoteOff { track, note } => {
                assert_eq!(track, "drums");
                assert_eq!(note, 40);
            }
            _ => panic!("expected NoteOff"),
        }
    }

    #[test]
    fn note_out_of_range_returns_none() {
        let mappings = default_mappings();
        let msg = [0x90, 60, 100]; // note 60 is outside 36-47
        assert!(apply_midi_message(&msg, &mappings, None).is_none());
    }

    #[test]
    fn program_change_to_section() {
        let mappings = default_mappings();
        let msg = [0xC0, 1]; // Program 1
        let event = apply_midi_message(&msg, &mappings, None).unwrap();
        assert_eq!(event, ExternalEvent::SectionJump(1));
    }

    #[test]
    fn program_change_unmapped_returns_none() {
        let mappings = default_mappings();
        let msg = [0xC0, 99]; // Not mapped
        assert!(apply_midi_message(&msg, &mappings, None).is_none());
    }

    #[test]
    fn channel_filter_passes() {
        let mappings = default_mappings();
        let msg = [0xB0, 1, 64]; // Channel 0, CC1
        let event = apply_midi_message(&msg, &mappings, Some(0));
        assert!(event.is_some());
    }

    #[test]
    fn channel_filter_blocks() {
        let mappings = default_mappings();
        let msg = [0xB1, 1, 64]; // Channel 1, CC1
        let event = apply_midi_message(&msg, &mappings, Some(0));
        assert!(event.is_none());
    }

    #[test]
    fn empty_message_returns_none() {
        assert!(apply_midi_message(&[], &[], None).is_none());
    }

    #[test]
    fn unknown_status_returns_none() {
        let msg = [0xF0, 0x7E]; // System exclusive
        assert!(apply_midi_message(&msg, &[], None).is_none());
    }

    #[test]
    fn serialize_deserialize_mappings() {
        let mapping = MidiMapping::CcToMacro {
            cc: 1,
            macro_idx: 0,
        };
        let yaml = serde_yaml::to_string(&mapping).unwrap();
        let parsed: MidiMapping = serde_yaml::from_str(&yaml).unwrap();
        match parsed {
            MidiMapping::CcToMacro { cc, macro_idx } => {
                assert_eq!(cc, 1);
                assert_eq!(macro_idx, 0);
            }
            _ => panic!("wrong variant"),
        }
    }
}
