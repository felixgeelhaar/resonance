//! MIDI output — send Note On/Off and CC to external devices.

use crate::event::Beat;

/// An active MIDI note that needs a Note Off message.
#[derive(Debug, Clone)]
pub struct ActiveNote {
    pub channel: u8,
    pub note: u8,
    pub end_beat: Beat,
}

/// MIDI output connection wrapper.
///
/// In test/CI environments without MIDI hardware, the connection is optional.
/// All methods are no-ops when no connection is available.
pub struct MidiOutput {
    #[cfg(not(test))]
    _connection: Option<midir::MidiOutputConnection>,
    active_notes: Vec<ActiveNote>,
    device_name: String,
}

impl MidiOutput {
    /// Create a new MIDI output (no connection).
    pub fn new(device_name: &str) -> Self {
        Self {
            #[cfg(not(test))]
            _connection: None,
            active_notes: Vec::new(),
            device_name: device_name.to_string(),
        }
    }

    /// Try to connect to the named MIDI output device.
    #[cfg(not(test))]
    pub fn connect(device_name: &str) -> Option<Self> {
        let midi_out = midir::MidiOutput::new("resonance-output").ok()?;
        let ports = midi_out.ports();
        let port = ports.iter().find(|p| {
            midi_out
                .port_name(p)
                .map(|n| n.contains(device_name))
                .unwrap_or(false)
        })?;
        let connection = midi_out.connect(port, "resonance").ok()?;
        Some(Self {
            _connection: Some(connection),
            active_notes: Vec::new(),
            device_name: device_name.to_string(),
        })
    }

    /// List available MIDI output devices.
    pub fn list_devices() -> Vec<String> {
        let Ok(midi_out) = midir::MidiOutput::new("resonance-list") else {
            return Vec::new();
        };
        midi_out
            .ports()
            .iter()
            .filter_map(|p| midi_out.port_name(p).ok())
            .collect()
    }

    /// Send a Note On message.
    pub fn send_note_on(&mut self, channel: u8, note: u8, velocity: u8, end_beat: Beat) {
        self.active_notes.push(ActiveNote {
            channel,
            note,
            end_beat,
        });
        #[cfg(not(test))]
        if let Some(ref mut conn) = self._connection {
            let status = 0x90 | (channel & 0x0F);
            let _ = conn.send(&[status, note & 0x7F, velocity & 0x7F]);
        }
        #[cfg(test)]
        let _ = velocity;
    }

    /// Send a Note Off message.
    pub fn send_note_off(&mut self, channel: u8, note: u8) {
        #[cfg(not(test))]
        if let Some(ref mut conn) = self._connection {
            let status = 0x80 | (channel & 0x0F);
            let _ = conn.send(&[status, note & 0x7F, 0]);
        }
        let _ = (channel, note); // suppress unused warning in test
    }

    /// Send a CC message.
    pub fn send_cc(&mut self, channel: u8, cc: u8, value: u8) {
        #[cfg(not(test))]
        if let Some(ref mut conn) = self._connection {
            let status = 0xB0 | (channel & 0x0F);
            let _ = conn.send(&[status, cc & 0x7F, value & 0x7F]);
        }
        let _ = (channel, cc, value);
    }

    /// Flush expired notes: send Note Off for any notes past their end time.
    pub fn flush_expired_notes(&mut self, current_beat: Beat) {
        let expired: Vec<_> = self
            .active_notes
            .iter()
            .filter(|n| current_beat >= n.end_beat)
            .cloned()
            .collect();

        for note in &expired {
            self.send_note_off(note.channel, note.note);
        }

        self.active_notes.retain(|n| current_beat < n.end_beat);
    }

    /// Get the device name.
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Number of currently active (sounding) notes.
    pub fn active_note_count(&self) -> usize {
        self.active_notes.len()
    }
}

/// MIDI output routing for a track.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MidiOutputRoute {
    pub track_name: String,
    pub device: String,
    pub channel: u8,
}

/// MIDI output configuration loaded from YAML.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct MidiOutputConfig {
    pub routes: Vec<MidiOutputRoute>,
}

impl MidiOutputConfig {
    /// Load from a YAML file.
    pub fn load(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&content).map_err(|e| std::io::Error::other(e.to_string()))
    }

    /// Save to a YAML file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let content =
            serde_yaml::to_string(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_tracking() {
        let mut out = MidiOutput::new("test");
        out.send_note_on(0, 60, 100, Beat::from_beats(2));
        out.send_note_on(0, 64, 80, Beat::from_beats(3));
        assert_eq!(out.active_note_count(), 2);

        out.flush_expired_notes(Beat::from_beats(2));
        assert_eq!(out.active_note_count(), 1);

        out.flush_expired_notes(Beat::from_beats(3));
        assert_eq!(out.active_note_count(), 0);
    }

    #[test]
    fn config_roundtrip() {
        let config = MidiOutputConfig {
            routes: vec![MidiOutputRoute {
                track_name: "lead".to_string(),
                device: "USB MIDI".to_string(),
                channel: 1,
            }],
        };
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: MidiOutputConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.routes.len(), 1);
        assert_eq!(parsed.routes[0].device, "USB MIDI");
    }

    #[test]
    fn list_devices_no_panic() {
        // Should not panic even without MIDI hardware
        let devices = MidiOutput::list_devices();
        let _ = devices; // may be empty in CI
    }

    #[test]
    fn device_name() {
        let out = MidiOutput::new("test device");
        assert_eq!(out.device_name(), "test device");
    }

    #[test]
    fn send_cc_no_panic() {
        let mut out = MidiOutput::new("test");
        out.send_cc(0, 1, 127); // should not panic
    }
}
