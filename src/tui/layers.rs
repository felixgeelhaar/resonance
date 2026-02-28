//! Layer panel — displays and manages layer toggle state.

/// A single layer entry for the panel display.
#[derive(Debug, Clone)]
pub struct LayerEntry {
    pub name: String,
    pub enabled: bool,
}

/// The layer panel widget state.
#[derive(Debug, Clone)]
pub struct LayerPanel {
    pub entries: Vec<LayerEntry>,
}

impl LayerPanel {
    /// Create a new empty layer panel.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Update the panel from layer names and enabled states.
    pub fn update(&mut self, layers: &[(String, bool)]) {
        self.entries = layers
            .iter()
            .map(|(name, enabled)| LayerEntry {
                name: name.clone(),
                enabled: *enabled,
            })
            .collect();
    }

    /// Get the number of layers.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no layers.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get a layer name by index.
    pub fn name_at(&self, idx: usize) -> Option<&str> {
        self.entries.get(idx).map(|e| e.name.as_str())
    }
}

impl Default for LayerPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_panel_is_empty() {
        let panel = LayerPanel::new();
        assert!(panel.is_empty());
        assert_eq!(panel.len(), 0);
    }

    #[test]
    fn update_populates_entries() {
        let mut panel = LayerPanel::new();
        panel.update(&[("reverb".to_string(), true), ("delay".to_string(), false)]);
        assert_eq!(panel.len(), 2);
        assert_eq!(panel.entries[0].name, "reverb");
        assert!(panel.entries[0].enabled);
        assert_eq!(panel.entries[1].name, "delay");
        assert!(!panel.entries[1].enabled);
    }

    #[test]
    fn name_at_returns_correct_name() {
        let mut panel = LayerPanel::new();
        panel.update(&[("fx".to_string(), false)]);
        assert_eq!(panel.name_at(0), Some("fx"));
        assert_eq!(panel.name_at(1), None);
    }

    #[test]
    fn update_replaces_previous() {
        let mut panel = LayerPanel::new();
        panel.update(&[("a".to_string(), true)]);
        assert_eq!(panel.len(), 1);
        panel.update(&[("b".to_string(), false), ("c".to_string(), true)]);
        assert_eq!(panel.len(), 2);
        assert_eq!(panel.entries[0].name, "b");
    }
}
