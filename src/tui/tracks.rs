//! Track list panel — shows track names and mute/solo state.

/// Track display info for the TUI track list panel.
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub name: String,
    pub instrument_type: String,
    pub muted: bool,
}

/// Track list state.
#[derive(Debug, Clone, Default)]
pub struct TrackList {
    pub tracks: Vec<TrackInfo>,
    pub selected: usize,
}

impl TrackList {
    /// Create from track definitions.
    pub fn from_defs(defs: &[(String, String)]) -> Self {
        let tracks = defs
            .iter()
            .map(|(name, inst)| TrackInfo {
                name: name.clone(),
                instrument_type: inst.clone(),
                muted: false,
            })
            .collect();
        Self {
            tracks,
            selected: 0,
        }
    }

    /// Toggle mute for the selected track.
    pub fn toggle_mute(&mut self) {
        if let Some(track) = self.tracks.get_mut(self.selected) {
            track.muted = !track.muted;
        }
    }

    /// Number of tracks.
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_defs_creates_tracks() {
        let list = TrackList::from_defs(&[
            ("drums".to_string(), "kit".to_string()),
            ("bass".to_string(), "bass".to_string()),
        ]);
        assert_eq!(list.len(), 2);
        assert_eq!(list.tracks[0].name, "drums");
        assert!(!list.tracks[0].muted);
    }

    #[test]
    fn toggle_mute() {
        let mut list = TrackList::from_defs(&[("drums".to_string(), "kit".to_string())]);
        assert!(!list.tracks[0].muted);
        list.toggle_mute();
        assert!(list.tracks[0].muted);
        list.toggle_mute();
        assert!(!list.tracks[0].muted);
    }
}
