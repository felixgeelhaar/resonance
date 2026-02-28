//! Macro meters panel — displays macro values as horizontal bars.

/// Display info for a single macro meter.
#[derive(Debug, Clone)]
pub struct MacroMeter {
    pub name: String,
    pub value: f64,
}

/// Macro meters panel state.
#[derive(Debug, Clone, Default)]
pub struct MacroPanel {
    pub meters: Vec<MacroMeter>,
}

impl MacroPanel {
    /// Update meters from current macro values.
    pub fn update(&mut self, macros: &std::collections::HashMap<String, f64>) {
        self.meters = macros
            .iter()
            .map(|(name, &value)| MacroMeter {
                name: name.clone(),
                value,
            })
            .collect();
        self.meters.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Number of macros.
    pub fn len(&self) -> usize {
        self.meters.len()
    }

    /// Whether the panel is empty.
    pub fn is_empty(&self) -> bool {
        self.meters.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn update_from_macros() {
        let mut panel = MacroPanel::default();
        let mut macros = HashMap::new();
        macros.insert("filter".to_string(), 0.5);
        macros.insert("drive".to_string(), 0.8);
        panel.update(&macros);
        assert_eq!(panel.len(), 2);
        // Sorted alphabetically
        assert_eq!(panel.meters[0].name, "drive");
        assert_eq!(panel.meters[1].name, "filter");
    }

    #[test]
    fn empty_macros() {
        let panel = MacroPanel::default();
        assert!(panel.is_empty());
    }
}
