//! Key bindings — maps key events to application actions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Application-level actions triggered by key events.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Quit the application.
    Quit,
    /// Toggle play/stop.
    TogglePlayback,
    /// Compile and reload DSL source.
    CompileReload,
    /// Toggle between Edit and Perform modes.
    ToggleMode,
    /// Cycle focus to the next panel.
    CycleFocus,
    /// Jump to a section by number (0-indexed).
    JumpSection(usize),
    /// Adjust a macro by index and delta.
    AdjustMacro(usize, f64),
    /// Insert a character in the editor.
    EditorInsert(char),
    /// Delete character before cursor.
    EditorBackspace,
    /// Delete character at cursor.
    EditorDelete,
    /// Move cursor in editor.
    EditorLeft,
    EditorRight,
    EditorUp,
    EditorDown,
    /// New line in editor.
    EditorNewline,
    /// Navigate to start/end of line.
    EditorHome,
    EditorEnd,
}

/// Map a key event to an application action based on the current mode.
pub fn map_key(key: KeyEvent, is_edit_mode: bool) -> Option<Action> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Global bindings (both modes)
    if ctrl {
        return match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('r') => Some(Action::CompileReload),
            KeyCode::Char('p') => Some(Action::ToggleMode),
            _ => None,
        };
    }

    match key.code {
        KeyCode::Tab => return Some(Action::CycleFocus),
        KeyCode::Char(' ') if !is_edit_mode => return Some(Action::TogglePlayback),
        _ => {}
    }

    if is_edit_mode {
        // Editor mode bindings
        match key.code {
            KeyCode::Char(c) => Some(Action::EditorInsert(c)),
            KeyCode::Backspace => Some(Action::EditorBackspace),
            KeyCode::Delete => Some(Action::EditorDelete),
            KeyCode::Enter => Some(Action::EditorNewline),
            KeyCode::Left => Some(Action::EditorLeft),
            KeyCode::Right => Some(Action::EditorRight),
            KeyCode::Up => Some(Action::EditorUp),
            KeyCode::Down => Some(Action::EditorDown),
            KeyCode::Home => Some(Action::EditorHome),
            KeyCode::End => Some(Action::EditorEnd),
            _ => None,
        }
    } else {
        // Perform mode bindings
        match key.code {
            KeyCode::Char('1') => Some(Action::JumpSection(0)),
            KeyCode::Char('2') => Some(Action::JumpSection(1)),
            KeyCode::Char('3') => Some(Action::JumpSection(2)),
            KeyCode::Char('4') => Some(Action::JumpSection(3)),
            KeyCode::Char('5') => Some(Action::JumpSection(4)),
            KeyCode::Char('6') => Some(Action::JumpSection(5)),
            KeyCode::Char('7') => Some(Action::JumpSection(6)),
            KeyCode::Char('8') => Some(Action::JumpSection(7)),
            KeyCode::Char('9') => Some(Action::JumpSection(8)),
            KeyCode::F(n @ 1..=8) => Some(Action::AdjustMacro((n - 1) as usize, 0.05)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn ctrl_q_quits() {
        assert_eq!(map_key(ctrl_key('q'), false), Some(Action::Quit));
        assert_eq!(map_key(ctrl_key('q'), true), Some(Action::Quit));
    }

    #[test]
    fn ctrl_r_compiles() {
        assert_eq!(map_key(ctrl_key('r'), false), Some(Action::CompileReload));
    }

    #[test]
    fn ctrl_p_toggles_mode() {
        assert_eq!(map_key(ctrl_key('p'), false), Some(Action::ToggleMode));
    }

    #[test]
    fn tab_cycles_focus() {
        assert_eq!(map_key(key(KeyCode::Tab), false), Some(Action::CycleFocus));
        assert_eq!(map_key(key(KeyCode::Tab), true), Some(Action::CycleFocus));
    }

    #[test]
    fn space_toggles_playback_in_perform() {
        assert_eq!(
            map_key(key(KeyCode::Char(' ')), false),
            Some(Action::TogglePlayback)
        );
    }

    #[test]
    fn space_inserts_in_edit() {
        assert_eq!(
            map_key(key(KeyCode::Char(' ')), true),
            Some(Action::EditorInsert(' '))
        );
    }

    #[test]
    fn number_keys_jump_section_in_perform() {
        assert_eq!(
            map_key(key(KeyCode::Char('1')), false),
            Some(Action::JumpSection(0))
        );
        assert_eq!(
            map_key(key(KeyCode::Char('9')), false),
            Some(Action::JumpSection(8))
        );
    }

    #[test]
    fn number_keys_insert_in_edit() {
        assert_eq!(
            map_key(key(KeyCode::Char('1')), true),
            Some(Action::EditorInsert('1'))
        );
    }

    #[test]
    fn f_keys_adjust_macros_in_perform() {
        assert_eq!(
            map_key(key(KeyCode::F(1)), false),
            Some(Action::AdjustMacro(0, 0.05))
        );
        assert_eq!(
            map_key(key(KeyCode::F(8)), false),
            Some(Action::AdjustMacro(7, 0.05))
        );
    }

    #[test]
    fn editor_keys_in_edit_mode() {
        assert_eq!(
            map_key(key(KeyCode::Backspace), true),
            Some(Action::EditorBackspace)
        );
        assert_eq!(
            map_key(key(KeyCode::Enter), true),
            Some(Action::EditorNewline)
        );
        assert_eq!(map_key(key(KeyCode::Left), true), Some(Action::EditorLeft));
    }

    #[test]
    fn arrow_keys_no_action_in_perform() {
        assert_eq!(map_key(key(KeyCode::Left), false), None);
    }
}
