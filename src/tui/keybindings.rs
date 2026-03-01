//! Key bindings — maps key events to application actions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::layout::FocusPanel;

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
    /// Adjust a macro by index and delta (default step).
    AdjustMacro(usize, f64),
    /// Adjust a macro with fine step (0.01).
    AdjustMacroFine(usize, f64),
    /// Adjust a macro with coarse step (0.20).
    AdjustMacroCoarse(usize, f64),
    /// Undo macro change (perform mode).
    MacroUndo,
    /// Redo macro change (perform mode).
    MacroRedo,
    /// Toggle a layer by index (0-indexed).
    ToggleLayer(usize),
    /// Accept the current diff preview.
    AcceptDiff,
    /// Reject the current diff preview.
    RejectDiff,
    /// Scroll diff preview up.
    DiffScrollUp,
    /// Scroll diff preview down.
    DiffScrollDown,
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
    /// Toggle help overlay.
    ToggleHelp,
    /// Toggle crash log overlay.
    ToggleCrashLog,
    /// Zoom in the grid visualization.
    GridZoomIn,
    /// Zoom out the grid visualization.
    GridZoomOut,
    /// Escape key (close overlays, return to editor focus).
    Escape,
    /// Navigate within a non-editor panel (arrow keys).
    PanelNavigate(KeyCode),
    /// Cycle to the next theme.
    CycleTheme,
    /// Evaluate code immediately (Ctrl+Enter — REPL).
    EvalImmediate,
    /// Activate the command bar (Ctrl+;).
    ActivateCommandBar,
    /// Insert a character in the command bar.
    CommandBarInsert(char),
    /// Submit the command bar input.
    CommandBarSubmit,
    /// Cancel the command bar.
    CommandBarCancel,
    /// Backspace in the command bar.
    CommandBarBackspace,
    /// Move cursor left in command bar.
    CommandBarLeft,
    /// Move cursor right in command bar.
    CommandBarRight,
    /// Navigate command bar history up.
    CommandBarHistoryUp,
    /// Navigate command bar history down.
    CommandBarHistoryDown,
    /// Next tutorial lesson.
    TutorialNext,
    /// Previous tutorial lesson.
    TutorialPrev,
    /// Toggle the DSL reference overlay.
    ToggleDslReference,
    /// Reconnect to the default audio output device.
    ReconnectAudio,
    /// Toggle the settings panel.
    ToggleSettings,
    /// Settings: switch to next tab.
    SettingsNextTab,
    /// Settings: switch to previous tab.
    SettingsPrevTab,
    /// Settings: move to next field.
    SettingsNextField,
    /// Settings: move to previous field.
    SettingsPrevField,
    /// Settings: toggle/activate current field.
    SettingsToggleField,
    /// Settings: insert character while editing a text field.
    SettingsInsert(char),
    /// Settings: backspace while editing a text field.
    SettingsBackspace,
    /// Settings: stop editing current text field.
    SettingsStopEdit,
    /// Settings: save all settings to disk.
    SettingsSave,
}

/// Map a key event to an application action based on the current mode.
/// Convenience wrapper that defaults to Editor focus and no diff preview.
pub fn map_key(key: KeyEvent, is_edit_mode: bool) -> Option<Action> {
    map_key_with_diff(key, is_edit_mode, false, FocusPanel::Editor)
}

/// Map a key event with diff preview and focus awareness.
/// Editor actions only fire when `focus == FocusPanel::Editor`.
pub fn map_key_with_diff(
    key: KeyEvent,
    is_edit_mode: bool,
    diff_preview_visible: bool,
    focus: FocusPanel,
) -> Option<Action> {
    map_key_full(key, is_edit_mode, diff_preview_visible, focus, false, false)
}

/// Full key mapping with command bar, tutorial, and settings awareness.
pub fn map_key_full(
    key: KeyEvent,
    is_edit_mode: bool,
    diff_preview_visible: bool,
    focus: FocusPanel,
    command_bar_active: bool,
    tutorial_active: bool,
) -> Option<Action> {
    map_key_all(
        key,
        is_edit_mode,
        diff_preview_visible,
        focus,
        command_bar_active,
        tutorial_active,
        false,
        false,
    )
}

/// Full key mapping with all modal states including settings panel.
#[allow(clippy::too_many_arguments)]
pub fn map_key_all(
    key: KeyEvent,
    is_edit_mode: bool,
    diff_preview_visible: bool,
    focus: FocusPanel,
    command_bar_active: bool,
    tutorial_active: bool,
    settings_active: bool,
    settings_editing: bool,
) -> Option<Action> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    // Settings panel intercepts keys when visible
    if settings_active {
        // Ctrl+Q still quits
        if ctrl && key.code == KeyCode::Char('q') {
            return Some(Action::Quit);
        }
        // Ctrl+S saves
        if ctrl && key.code == KeyCode::Char('s') {
            return Some(Action::SettingsSave);
        }

        if settings_editing {
            // Text editing mode
            return match key.code {
                KeyCode::Esc => Some(Action::SettingsStopEdit),
                KeyCode::Enter => Some(Action::SettingsStopEdit),
                KeyCode::Backspace => Some(Action::SettingsBackspace),
                KeyCode::Char(c) => Some(Action::SettingsInsert(c)),
                _ => None,
            };
        }

        return match key.code {
            KeyCode::Esc => Some(Action::ToggleSettings),
            KeyCode::Tab if shift => Some(Action::SettingsPrevTab),
            KeyCode::Tab => Some(Action::SettingsNextTab),
            KeyCode::BackTab => Some(Action::SettingsPrevTab),
            KeyCode::Up => Some(Action::SettingsPrevField),
            KeyCode::Down => Some(Action::SettingsNextField),
            KeyCode::Enter => Some(Action::SettingsToggleField),
            KeyCode::Left => Some(Action::SettingsPrevTab),
            KeyCode::Right => Some(Action::SettingsNextTab),
            _ => None,
        };
    }

    // Command bar mode intercepts almost all keys
    if command_bar_active {
        // Ctrl+Q still quits
        if ctrl && key.code == KeyCode::Char('q') {
            return Some(Action::Quit);
        }
        return match key.code {
            KeyCode::Enter => Some(Action::CommandBarSubmit),
            KeyCode::Esc => Some(Action::CommandBarCancel),
            KeyCode::Backspace => Some(Action::CommandBarBackspace),
            KeyCode::Left => Some(Action::CommandBarLeft),
            KeyCode::Right => Some(Action::CommandBarRight),
            KeyCode::Up => Some(Action::CommandBarHistoryUp),
            KeyCode::Down => Some(Action::CommandBarHistoryDown),
            KeyCode::Char(c) => Some(Action::CommandBarInsert(c)),
            _ => None,
        };
    }

    // Diff preview mode intercepts most keys
    if diff_preview_visible {
        return match key.code {
            KeyCode::Enter => Some(Action::AcceptDiff),
            KeyCode::Esc => Some(Action::RejectDiff),
            KeyCode::Up => Some(Action::DiffScrollUp),
            KeyCode::Down => Some(Action::DiffScrollDown),
            _ => None,
        };
    }

    // Global bindings (both modes, all panels)
    if ctrl {
        return match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('r') => Some(Action::CompileReload),
            KeyCode::Char('p') => Some(Action::ToggleMode),
            KeyCode::Char('l') => Some(Action::ToggleCrashLog),
            KeyCode::Char('t') => Some(Action::CycleTheme),
            KeyCode::Char('d') => Some(Action::ReconnectAudio),
            KeyCode::Char(',') => Some(Action::ToggleSettings),
            KeyCode::Char('z') if !is_edit_mode => Some(Action::MacroUndo),
            KeyCode::Char('y') if !is_edit_mode => Some(Action::MacroRedo),
            KeyCode::Enter => Some(Action::EvalImmediate),
            KeyCode::Char(';') => Some(Action::ActivateCommandBar),
            KeyCode::Right if tutorial_active => Some(Action::TutorialNext),
            KeyCode::Left if tutorial_active => Some(Action::TutorialPrev),
            _ => None,
        };
    }

    // Help toggle (? key) — available in all panels except editor in edit mode
    if key.code == KeyCode::Char('?') && !(is_edit_mode && focus == FocusPanel::Editor) {
        // Shift+? toggles DSL reference, plain ? toggles help
        if shift {
            return Some(Action::ToggleDslReference);
        }
        return Some(Action::ToggleHelp);
    }

    match key.code {
        KeyCode::Tab => return Some(Action::CycleFocus),
        KeyCode::Char(' ') if !is_edit_mode => return Some(Action::TogglePlayback),
        KeyCode::Esc if !diff_preview_visible => return Some(Action::Escape),
        _ => {}
    }

    if is_edit_mode && focus == FocusPanel::Editor {
        // Editor mode bindings — ONLY when editor panel has focus
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
    } else if is_edit_mode {
        // Edit mode but non-editor panel: only navigation
        match key.code {
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                Some(Action::PanelNavigate(key.code))
            }
            _ => None,
        }
    } else {
        // Perform mode bindings (available regardless of focus)
        // Shift+1..9 toggles layers, Shift+F1..F8 fine macro adjust
        if shift {
            return match key.code {
                KeyCode::Char('!') => Some(Action::ToggleLayer(0)),
                KeyCode::Char('@') => Some(Action::ToggleLayer(1)),
                KeyCode::Char('#') => Some(Action::ToggleLayer(2)),
                KeyCode::Char('$') => Some(Action::ToggleLayer(3)),
                KeyCode::Char('%') => Some(Action::ToggleLayer(4)),
                KeyCode::Char('^') => Some(Action::ToggleLayer(5)),
                KeyCode::Char('&') => Some(Action::ToggleLayer(6)),
                KeyCode::Char('*') => Some(Action::ToggleLayer(7)),
                KeyCode::Char('(') => Some(Action::ToggleLayer(8)),
                KeyCode::F(n @ 1..=8) => Some(Action::AdjustMacroFine((n - 1) as usize, 0.01)),
                _ => None,
            };
        }

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
            KeyCode::Char('+') | KeyCode::Char('=') => Some(Action::GridZoomIn),
            KeyCode::Char('-') => Some(Action::GridZoomOut),
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

    fn shift_key(c: char) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::SHIFT,
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
    fn space_inserts_in_edit_with_editor_focus() {
        // map_key defaults to Editor focus
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
    fn number_keys_insert_in_edit_with_editor_focus() {
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
    fn editor_keys_in_edit_mode_with_editor_focus() {
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

    #[test]
    fn diff_preview_enter_accepts() {
        assert_eq!(
            map_key_with_diff(key(KeyCode::Enter), false, true, FocusPanel::Editor),
            Some(Action::AcceptDiff)
        );
    }

    #[test]
    fn diff_preview_esc_rejects() {
        assert_eq!(
            map_key_with_diff(key(KeyCode::Esc), false, true, FocusPanel::Editor),
            Some(Action::RejectDiff)
        );
    }

    #[test]
    fn diff_preview_arrows_scroll() {
        assert_eq!(
            map_key_with_diff(key(KeyCode::Up), false, true, FocusPanel::Editor),
            Some(Action::DiffScrollUp)
        );
        assert_eq!(
            map_key_with_diff(key(KeyCode::Down), false, true, FocusPanel::Editor),
            Some(Action::DiffScrollDown)
        );
    }

    #[test]
    fn diff_preview_blocks_other_keys() {
        assert_eq!(
            map_key_with_diff(key(KeyCode::Char('1')), false, true, FocusPanel::Editor),
            None
        );
        assert_eq!(
            map_key_with_diff(key(KeyCode::Tab), false, true, FocusPanel::Editor),
            None
        );
    }

    #[test]
    fn shift_number_toggles_layer_in_perform() {
        assert_eq!(
            map_key_with_diff(shift_key('!'), false, false, FocusPanel::Editor),
            Some(Action::ToggleLayer(0))
        );
        assert_eq!(
            map_key_with_diff(shift_key('@'), false, false, FocusPanel::Editor),
            Some(Action::ToggleLayer(1))
        );
        assert_eq!(
            map_key_with_diff(shift_key('#'), false, false, FocusPanel::Editor),
            Some(Action::ToggleLayer(2))
        );
    }

    #[test]
    fn shift_keys_insert_in_edit_with_editor_focus() {
        assert_eq!(
            map_key_with_diff(shift_key('!'), true, false, FocusPanel::Editor),
            Some(Action::EditorInsert('!'))
        );
    }

    // --- Focus isolation tests ---

    #[test]
    fn editor_keys_ignored_when_tracks_focused() {
        // In edit mode, but Tracks panel has focus — typing should NOT go to editor
        assert_eq!(
            map_key_with_diff(key(KeyCode::Char('a')), true, false, FocusPanel::Tracks),
            None
        );
    }

    #[test]
    fn editor_keys_ignored_when_grid_focused() {
        assert_eq!(
            map_key_with_diff(key(KeyCode::Char('x')), true, false, FocusPanel::Grid),
            None
        );
    }

    #[test]
    fn arrow_keys_navigate_panel_when_not_editor_focused() {
        assert_eq!(
            map_key_with_diff(key(KeyCode::Up), true, false, FocusPanel::Tracks),
            Some(Action::PanelNavigate(KeyCode::Up))
        );
        assert_eq!(
            map_key_with_diff(key(KeyCode::Down), true, false, FocusPanel::Macros),
            Some(Action::PanelNavigate(KeyCode::Down))
        );
    }

    #[test]
    fn global_bindings_work_from_any_panel() {
        for panel in [
            FocusPanel::Editor,
            FocusPanel::Tracks,
            FocusPanel::Grid,
            FocusPanel::Macros,
            FocusPanel::IntentConsole,
        ] {
            assert_eq!(
                map_key_with_diff(ctrl_key('q'), false, false, panel),
                Some(Action::Quit)
            );
            assert_eq!(
                map_key_with_diff(key(KeyCode::Tab), true, false, panel),
                Some(Action::CycleFocus)
            );
        }
    }

    #[test]
    fn help_toggle_works_in_perform_mode() {
        assert_eq!(
            map_key_with_diff(key(KeyCode::Char('?')), false, false, FocusPanel::Editor),
            Some(Action::ToggleHelp)
        );
    }

    #[test]
    fn ctrl_l_toggles_crash_log() {
        assert_eq!(map_key(ctrl_key('l'), false), Some(Action::ToggleCrashLog));
        assert_eq!(map_key(ctrl_key('l'), true), Some(Action::ToggleCrashLog));
    }

    #[test]
    fn ctrl_t_cycles_theme() {
        assert_eq!(map_key(ctrl_key('t'), false), Some(Action::CycleTheme));
        assert_eq!(map_key(ctrl_key('t'), true), Some(Action::CycleTheme));
    }

    #[test]
    fn ctrl_t_works_from_any_panel() {
        for panel in [
            FocusPanel::Editor,
            FocusPanel::Tracks,
            FocusPanel::Grid,
            FocusPanel::Macros,
            FocusPanel::IntentConsole,
        ] {
            assert_eq!(
                map_key_with_diff(ctrl_key('t'), false, false, panel),
                Some(Action::CycleTheme)
            );
        }
    }

    #[test]
    fn help_toggle_not_in_editor_edit_mode() {
        // When editing in editor, ? should insert the character
        assert_eq!(
            map_key_with_diff(key(KeyCode::Char('?')), true, false, FocusPanel::Editor),
            Some(Action::EditorInsert('?'))
        );
    }

    #[test]
    fn help_toggle_works_from_non_editor_in_edit_mode() {
        assert_eq!(
            map_key_with_diff(key(KeyCode::Char('?')), true, false, FocusPanel::Tracks),
            Some(Action::ToggleHelp)
        );
    }

    // --- New REPL/command bar keybinding tests ---

    #[test]
    fn ctrl_enter_evals() {
        assert_eq!(
            map_key_full(
                ctrl_key_event(KeyCode::Enter),
                false,
                false,
                FocusPanel::Editor,
                false,
                false
            ),
            Some(Action::EvalImmediate)
        );
    }

    #[test]
    fn ctrl_semicolon_activates_command_bar() {
        assert_eq!(
            map_key_full(
                ctrl_key_event(KeyCode::Char(';')),
                false,
                false,
                FocusPanel::Editor,
                false,
                false
            ),
            Some(Action::ActivateCommandBar)
        );
    }

    #[test]
    fn command_bar_routes_chars() {
        // When command bar is active, regular chars go to CommandBarInsert
        assert_eq!(
            map_key_full(
                key(KeyCode::Char('a')),
                true,
                false,
                FocusPanel::Editor,
                true,
                false
            ),
            Some(Action::CommandBarInsert('a'))
        );
    }

    #[test]
    fn command_bar_enter_submits() {
        assert_eq!(
            map_key_full(
                key(KeyCode::Enter),
                true,
                false,
                FocusPanel::Editor,
                true,
                false
            ),
            Some(Action::CommandBarSubmit)
        );
    }

    #[test]
    fn command_bar_esc_cancels() {
        assert_eq!(
            map_key_full(
                key(KeyCode::Esc),
                true,
                false,
                FocusPanel::Editor,
                true,
                false
            ),
            Some(Action::CommandBarCancel)
        );
    }

    #[test]
    fn command_bar_ctrl_q_still_quits() {
        assert_eq!(
            map_key_full(
                ctrl_key_event(KeyCode::Char('q')),
                true,
                false,
                FocusPanel::Editor,
                true,
                false
            ),
            Some(Action::Quit)
        );
    }

    #[test]
    fn tutorial_ctrl_right_next() {
        assert_eq!(
            map_key_full(
                ctrl_key_event(KeyCode::Right),
                false,
                false,
                FocusPanel::Editor,
                false,
                true
            ),
            Some(Action::TutorialNext)
        );
    }

    #[test]
    fn tutorial_ctrl_left_prev() {
        assert_eq!(
            map_key_full(
                ctrl_key_event(KeyCode::Left),
                false,
                false,
                FocusPanel::Editor,
                false,
                true
            ),
            Some(Action::TutorialPrev)
        );
    }

    #[test]
    fn ctrl_d_reconnects_audio() {
        assert_eq!(map_key(ctrl_key('d'), false), Some(Action::ReconnectAudio));
        assert_eq!(map_key(ctrl_key('d'), true), Some(Action::ReconnectAudio));
    }

    #[test]
    fn ctrl_d_works_from_any_panel() {
        for panel in [
            FocusPanel::Editor,
            FocusPanel::Tracks,
            FocusPanel::Grid,
            FocusPanel::Macros,
            FocusPanel::IntentConsole,
        ] {
            assert_eq!(
                map_key_with_diff(ctrl_key('d'), false, false, panel),
                Some(Action::ReconnectAudio)
            );
        }
    }

    #[test]
    fn existing_map_key_still_works() {
        // Verify backward compatibility: map_key still routes correctly
        assert_eq!(map_key(ctrl_key('q'), false), Some(Action::Quit));
        assert_eq!(map_key(key(KeyCode::Tab), false), Some(Action::CycleFocus));
        assert_eq!(
            map_key(key(KeyCode::Char(' ')), false),
            Some(Action::TogglePlayback)
        );
    }

    /// Helper for creating a Ctrl+key event from a KeyCode (not just char).
    fn ctrl_key_event(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }
}
