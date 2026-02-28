//! Layout — panel arrangement and focus management.

/// Which panel currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    Editor,
    Tracks,
    Grid,
    Macros,
    IntentConsole,
}

impl FocusPanel {
    /// Cycle to the next panel.
    pub fn next(self) -> Self {
        match self {
            Self::Editor => Self::Tracks,
            Self::Tracks => Self::Grid,
            Self::Grid => Self::Macros,
            Self::Macros => Self::IntentConsole,
            Self::IntentConsole => Self::Editor,
        }
    }
}

/// Application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Edit,
    Perform,
}

impl AppMode {
    /// Toggle between Edit and Perform.
    pub fn toggle(self) -> Self {
        match self {
            Self::Edit => Self::Perform,
            Self::Perform => Self::Edit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_cycles() {
        let start = FocusPanel::Editor;
        let next = start.next().next().next().next().next();
        assert_eq!(next, FocusPanel::Editor); // Full cycle back
    }

    #[test]
    fn mode_toggle() {
        assert_eq!(AppMode::Edit.toggle(), AppMode::Perform);
        assert_eq!(AppMode::Perform.toggle(), AppMode::Edit);
    }

    #[test]
    fn focus_panel_order() {
        assert_eq!(FocusPanel::Editor.next(), FocusPanel::Tracks);
        assert_eq!(FocusPanel::Tracks.next(), FocusPanel::Grid);
        assert_eq!(FocusPanel::Grid.next(), FocusPanel::Macros);
        assert_eq!(FocusPanel::Macros.next(), FocusPanel::IntentConsole);
        assert_eq!(FocusPanel::IntentConsole.next(), FocusPanel::Editor);
    }
}
