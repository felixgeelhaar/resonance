//! Status bar — displays BPM, playback position, section, and mode.

/// Status information for the TUI status bar.
#[derive(Debug, Clone)]
pub struct StatusInfo {
    pub bpm: f64,
    pub position_bars: u64,
    pub position_beats: u64,
    pub section_name: String,
    pub is_playing: bool,
    pub is_edit_mode: bool,
    pub compile_status: CompileStatus,
}

/// Compilation status indicator.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileStatus {
    Ok,
    Error(String),
    Idle,
}

impl StatusInfo {
    /// Format the position as "bar.beat".
    pub fn position_display(&self) -> String {
        format!("{}.{}", self.position_bars + 1, self.position_beats + 1)
    }

    /// Format the mode indicator.
    pub fn mode_display(&self) -> &str {
        if self.is_edit_mode {
            "EDIT"
        } else {
            "PERFORM"
        }
    }

    /// Format the playback indicator.
    pub fn playback_display(&self) -> &str {
        if self.is_playing {
            "PLAY"
        } else {
            "STOP"
        }
    }
}

impl Default for StatusInfo {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            position_bars: 0,
            position_beats: 0,
            section_name: String::new(),
            is_playing: false,
            is_edit_mode: true,
            compile_status: CompileStatus::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_display_format() {
        let status = StatusInfo {
            position_bars: 3,
            position_beats: 2,
            ..Default::default()
        };
        assert_eq!(status.position_display(), "4.3");
    }

    #[test]
    fn mode_display() {
        let edit = StatusInfo {
            is_edit_mode: true,
            ..Default::default()
        };
        assert_eq!(edit.mode_display(), "EDIT");

        let perform = StatusInfo {
            is_edit_mode: false,
            ..Default::default()
        };
        assert_eq!(perform.mode_display(), "PERFORM");
    }

    #[test]
    fn playback_display() {
        let playing = StatusInfo {
            is_playing: true,
            ..Default::default()
        };
        assert_eq!(playing.playback_display(), "PLAY");

        let stopped = StatusInfo {
            is_playing: false,
            ..Default::default()
        };
        assert_eq!(stopped.playback_display(), "STOP");
    }
}
