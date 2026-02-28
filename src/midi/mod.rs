//! MIDI controller support — external hardware/software MIDI input.

pub mod config;
pub mod input;
pub mod mapping;

pub use config::MidiConfig;
pub use input::MidiInput;
pub use mapping::{apply_midi_message, MidiMapping};
