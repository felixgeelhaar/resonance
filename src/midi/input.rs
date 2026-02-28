//! MIDI input — connects to a MIDI device and routes messages to the external input channel.

use std::io;

use midir::{MidiInput as MidirInput, MidiInputConnection};

use super::config::MidiConfig;
use super::mapping::apply_midi_message;
use crate::tui::external_input::ExternalInputSender;

/// Active MIDI input connection.
pub struct MidiInput {
    _connection: MidiInputConnection<()>,
    port_name: String,
}

impl MidiInput {
    /// Start listening on a MIDI port.
    /// Finds a port matching the config's device_name (or the first available port).
    /// Messages are parsed and sent as ExternalEvents via the sender.
    pub fn start(config: &MidiConfig, sender: ExternalInputSender) -> io::Result<Self> {
        let midi_in = MidirInput::new("resonance")
            .map_err(|e| io::Error::other(format!("MIDI init: {e}")))?;

        let ports = midi_in.ports();
        if ports.is_empty() {
            return Err(io::Error::other("no MIDI input ports available"));
        }

        // Find matching port
        let (port, port_name) = if let Some(ref name_filter) = config.device_name {
            ports
                .iter()
                .find_map(|p| {
                    let name = midi_in.port_name(p).unwrap_or_default();
                    if name.contains(name_filter.as_str()) {
                        Some((p.clone(), name))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    io::Error::other(format!("MIDI device matching '{name_filter}' not found"))
                })?
        } else {
            let p = ports[0].clone();
            let name = midi_in
                .port_name(&p)
                .unwrap_or_else(|_| "unknown".to_string());
            (p, name)
        };

        let mappings = config.mappings.clone();
        let channel_filter = config.channel_filter;

        let connection = midi_in
            .connect(
                &port,
                "resonance-input",
                move |_timestamp, msg, _| {
                    if let Some(event) = apply_midi_message(msg, &mappings, channel_filter) {
                        let _ = sender.send(event);
                    }
                },
                (),
            )
            .map_err(|e| io::Error::other(format!("MIDI connect: {e}")))?;

        Ok(Self {
            _connection: connection,
            port_name,
        })
    }

    /// Get the connected port name.
    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    /// List all available MIDI input device names.
    pub fn list_devices() -> Vec<String> {
        let Ok(midi_in) = MidirInput::new("resonance-list") else {
            return Vec::new();
        };
        midi_in
            .ports()
            .iter()
            .filter_map(|p| midi_in.port_name(p).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_devices_does_not_panic() {
        // Just verify the function works without panicking
        let devices = MidiInput::list_devices();
        // May be empty in CI/test environments
        let _ = devices;
    }
}
