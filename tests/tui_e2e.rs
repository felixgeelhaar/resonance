//! TUI end-to-end tests — verify focus routing, playback, grid rendering,
//! help system, keybinding completeness, and mode transitions.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

use resonance::tui::keybindings::{self, Action};
use resonance::tui::layout::{AppMode, FocusPanel};
use resonance::tui::{App, CompileStatus, GridCell};

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

fn sample_src() -> &'static str {
    "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n    snare: [. . X .]\n  }\n}"
}

// =============================================================================
// Focus Routing Tests
// =============================================================================

#[test]
fn focus_routing_editor_captures_keys_only_when_focused() {
    // In edit mode with editor focus, typing inserts characters
    let action =
        keybindings::map_key_with_diff(key(KeyCode::Char('a')), true, false, FocusPanel::Editor);
    assert_eq!(action, Some(Action::EditorInsert('a')));

    // In edit mode with Tracks focus, typing does NOT insert
    let action =
        keybindings::map_key_with_diff(key(KeyCode::Char('a')), true, false, FocusPanel::Tracks);
    assert_eq!(action, None);
}

#[test]
fn focus_routing_keys_dont_leak_to_editor_from_grid() {
    let action =
        keybindings::map_key_with_diff(key(KeyCode::Char('x')), true, false, FocusPanel::Grid);
    assert_eq!(action, None);
}

#[test]
fn focus_routing_keys_dont_leak_to_editor_from_macros() {
    let action =
        keybindings::map_key_with_diff(key(KeyCode::Char('m')), true, false, FocusPanel::Macros);
    assert_eq!(action, None);
}

#[test]
fn focus_routing_keys_dont_leak_to_editor_from_intent_console() {
    let action = keybindings::map_key_with_diff(
        key(KeyCode::Char('z')),
        true,
        false,
        FocusPanel::IntentConsole,
    );
    assert_eq!(action, None);
}

#[test]
fn focus_routing_global_keys_work_from_all_panels() {
    let panels = [
        FocusPanel::Editor,
        FocusPanel::Tracks,
        FocusPanel::Grid,
        FocusPanel::Macros,
        FocusPanel::IntentConsole,
    ];

    for panel in panels {
        // Ctrl-Q always quits
        assert_eq!(
            keybindings::map_key_with_diff(ctrl_key('q'), true, false, panel),
            Some(Action::Quit),
            "Ctrl-Q should quit from {:?}",
            panel
        );

        // Tab always cycles
        assert_eq!(
            keybindings::map_key_with_diff(key(KeyCode::Tab), true, false, panel),
            Some(Action::CycleFocus),
            "Tab should cycle from {:?}",
            panel
        );

        // Ctrl-R always compiles
        assert_eq!(
            keybindings::map_key_with_diff(ctrl_key('r'), false, false, panel),
            Some(Action::CompileReload),
            "Ctrl-R should compile from {:?}",
            panel
        );

        // Ctrl-P always toggles mode
        assert_eq!(
            keybindings::map_key_with_diff(ctrl_key('p'), false, false, panel),
            Some(Action::ToggleMode),
            "Ctrl-P should toggle mode from {:?}",
            panel
        );
    }
}

#[test]
fn focus_routing_arrow_keys_navigate_panels() {
    assert_eq!(
        keybindings::map_key_with_diff(key(KeyCode::Up), true, false, FocusPanel::Tracks),
        Some(Action::PanelNavigate(KeyCode::Up))
    );
    assert_eq!(
        keybindings::map_key_with_diff(key(KeyCode::Down), true, false, FocusPanel::Grid),
        Some(Action::PanelNavigate(KeyCode::Down))
    );
}

#[test]
fn focus_routing_arrow_keys_move_cursor_in_editor() {
    assert_eq!(
        keybindings::map_key_with_diff(key(KeyCode::Up), true, false, FocusPanel::Editor),
        Some(Action::EditorUp)
    );
    assert_eq!(
        keybindings::map_key_with_diff(key(KeyCode::Left), true, false, FocusPanel::Editor),
        Some(Action::EditorLeft)
    );
}

// =============================================================================
// Panel Cycling Tests
// =============================================================================

#[test]
fn panel_cycling_full_loop() {
    let mut app = App::new("");
    assert_eq!(app.focus, FocusPanel::Editor);

    let expected = [
        FocusPanel::Tracks,
        FocusPanel::Grid,
        FocusPanel::Macros,
        FocusPanel::IntentConsole,
        FocusPanel::Editor, // back to start
    ];

    for expected_panel in expected {
        app.handle_action(Action::CycleFocus);
        assert_eq!(app.focus, expected_panel);
    }
}

#[test]
fn escape_returns_to_editor_from_any_panel() {
    let mut app = App::new("");

    for panel in [
        FocusPanel::Tracks,
        FocusPanel::Grid,
        FocusPanel::Macros,
        FocusPanel::IntentConsole,
    ] {
        app.focus = panel;
        app.handle_action(Action::Escape);
        assert_eq!(
            app.focus,
            FocusPanel::Editor,
            "Escape should return to Editor from {:?}",
            panel
        );
    }
}

// =============================================================================
// Mode Transition Tests
// =============================================================================

#[test]
fn mode_transition_edit_to_perform() {
    let mut app = App::new("");
    assert_eq!(app.mode, AppMode::Edit);
    assert!(app.status.is_edit_mode);

    app.handle_action(Action::ToggleMode);
    assert_eq!(app.mode, AppMode::Perform);
    assert!(!app.status.is_edit_mode);
}

#[test]
fn mode_transition_perform_to_edit() {
    let mut app = App::new("");
    app.handle_action(Action::ToggleMode); // to Perform
    app.handle_action(Action::ToggleMode); // back to Edit
    assert_eq!(app.mode, AppMode::Edit);
    assert!(app.status.is_edit_mode);
}

// =============================================================================
// Playback State Tests
// =============================================================================

#[test]
fn playback_toggle_updates_status() {
    let mut app = App::new("");
    assert!(!app.is_playing);
    assert!(!app.status.is_playing);

    app.handle_action(Action::TogglePlayback);
    assert!(app.is_playing);
    assert!(app.status.is_playing);

    app.handle_action(Action::TogglePlayback);
    assert!(!app.is_playing);
    assert!(!app.status.is_playing);
}

#[test]
fn playback_advances_beat() {
    let mut app = App::new("");
    app.status.bpm = 120.0;
    app.is_playing = true;

    // First call sets last_tick
    app.advance_beat();

    // Simulate 500ms elapsed
    app.set_last_tick(Instant::now() - Duration::from_millis(500));
    app.advance_beat();

    // At 120BPM, 500ms ≈ 1 beat = 960 ticks
    assert!(
        app.current_beat.ticks() > 0,
        "Beat should advance after time passes"
    );
}

#[test]
fn playback_updates_position_display() {
    let mut app = App::new("");
    app.status.bpm = 120.0;
    app.is_playing = true;

    // Simulate 5 seconds at 120BPM = 10 beats = 2.5 bars
    app.advance_beat();
    app.set_last_tick(Instant::now() - Duration::from_millis(5000));
    app.advance_beat();

    // Position should show bar > 1
    assert!(
        app.status.position_bars >= 2,
        "Should be at bar 3+ after 5 seconds at 120BPM, got bar {}",
        app.status.position_bars
    );
}

#[test]
fn playback_stopped_does_not_advance() {
    let mut app = App::new("");
    app.is_playing = false;
    app.advance_beat();

    // Even after calling advance, beat stays at zero
    assert_eq!(app.current_beat.ticks(), 0);
}

// =============================================================================
// Compile & Grid Visualization Tests
// =============================================================================

#[test]
fn compile_populates_events_for_grid() {
    let mut app = App::new(sample_src());
    app.handle_action(Action::CompileReload);

    assert_eq!(app.status.compile_status, CompileStatus::Ok);
    assert!(
        !app.compiled_events.is_empty(),
        "Compiled events should be populated after compile"
    );
}

#[test]
fn grid_projects_compiled_events() {
    let mut app = App::new(sample_src());
    app.handle_action(Action::CompileReload);

    let grids = resonance::tui::project_events(&app.compiled_events, 1, 8, None);
    assert!(
        !grids.is_empty(),
        "Grid should project events from compiled song"
    );

    // Check that at least one grid has a hit
    let has_hit = grids.iter().any(|g| {
        g.cells
            .iter()
            .any(|c| matches!(c, GridCell::Hit(_) | GridCell::Note(_)))
    });
    assert!(has_hit, "Grid should contain at least one hit/note");
}

#[test]
fn grid_shows_cursor_during_playback() {
    let mut app = App::new(sample_src());
    app.handle_action(Action::CompileReload);

    // Use beat 1.5 which maps to step 3 — an empty cell (kick is at 0, snare at 2)
    let cursor_beat = resonance::event::Beat::from_ticks(960 + 480); // 1.5 beats
    let grids = resonance::tui::project_events(&app.compiled_events, 1, 8, Some(cursor_beat));

    // The cursor should appear on at least one track's empty cell
    let has_cursor = grids
        .iter()
        .any(|g| g.cells.iter().any(|c| matches!(c, GridCell::Cursor)));
    assert!(has_cursor, "Grid should show cursor at playback position");
}

#[test]
fn compile_error_does_not_populate_events() {
    let mut app = App::new("invalid {{{ source");
    app.handle_action(Action::CompileReload);

    assert!(matches!(app.status.compile_status, CompileStatus::Error(_)));
    assert!(
        app.compiled_events.is_empty(),
        "Events should remain empty on compile error"
    );
}

// =============================================================================
// Help System Tests
// =============================================================================

#[test]
fn help_toggle_shows_and_hides() {
    let mut app = App::new("");
    assert!(!app.help_screen.visible);

    app.handle_action(Action::ToggleHelp);
    assert!(app.help_screen.visible);

    app.handle_action(Action::ToggleHelp);
    assert!(!app.help_screen.visible);
}

#[test]
fn help_escape_closes() {
    let mut app = App::new("");
    app.help_screen.show();
    assert!(app.help_screen.visible);

    app.handle_action(Action::Escape);
    assert!(!app.help_screen.visible);
}

#[test]
fn help_question_mark_in_perform_mode() {
    let action =
        keybindings::map_key_with_diff(key(KeyCode::Char('?')), false, false, FocusPanel::Editor);
    assert_eq!(action, Some(Action::ToggleHelp));
}

#[test]
fn help_question_mark_not_in_editor_edit_mode() {
    // When actively editing, ? should insert the character
    let action =
        keybindings::map_key_with_diff(key(KeyCode::Char('?')), true, false, FocusPanel::Editor);
    assert_eq!(action, Some(Action::EditorInsert('?')));
}

#[test]
fn help_content_has_all_sections() {
    let app = App::new("");
    let lines = app.help_screen.lines();
    let headers: Vec<_> = lines.iter().filter(|l| l.is_header).collect();

    // Should have at least: Global, Edit, Perform, Diff Preview, Tips
    assert!(
        headers.len() >= 4,
        "Help should have at least 4 section headers, found {}",
        headers.len()
    );

    let header_text: String = headers
        .iter()
        .map(|h| h.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(header_text.contains("GLOBAL"), "Help should mention GLOBAL");
    assert!(
        header_text.contains("EDIT"),
        "Help should mention EDIT MODE"
    );
    assert!(
        header_text.contains("PERFORM"),
        "Help should mention PERFORM"
    );
    assert!(
        header_text.contains("DIFF"),
        "Help should mention DIFF PREVIEW"
    );
}

#[test]
fn help_mentions_key_bindings() {
    let app = App::new("");
    let all_text: String = app
        .help_screen
        .lines()
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    assert!(all_text.contains("Ctrl-Q"), "Help should mention Ctrl-Q");
    assert!(all_text.contains("Ctrl-R"), "Help should mention Ctrl-R");
    assert!(all_text.contains("Tab"), "Help should mention Tab");
    assert!(all_text.contains("Space"), "Help should mention Space");
    assert!(all_text.contains("1-9"), "Help should mention section keys");
    assert!(
        all_text.contains("Shift+1-9"),
        "Help should mention layer toggles"
    );
    assert!(all_text.contains("F1-F8"), "Help should mention macro keys");
}

// =============================================================================
// Context-Sensitive Hints Tests
// =============================================================================

#[test]
fn context_hint_edit_mode_editor() {
    let mut app = App::new("");
    app.mode = AppMode::Edit;
    app.focus = FocusPanel::Editor;
    assert!(
        app.context_hint().contains("Type to edit"),
        "Edit+Editor hint should mention typing"
    );
}

#[test]
fn context_hint_edit_mode_other_panel() {
    let mut app = App::new("");
    app.mode = AppMode::Edit;
    app.focus = FocusPanel::Tracks;
    assert!(
        app.context_hint().contains("Esc:back to editor"),
        "Edit+NonEditor hint should mention Esc"
    );
}

#[test]
fn context_hint_perform_mode() {
    let mut app = App::new("");
    app.mode = AppMode::Perform;
    let hint = app.context_hint();
    assert!(
        hint.contains("Space:play"),
        "Perform hint should mention Space"
    );
    assert!(
        hint.contains("1-9:section"),
        "Perform hint should mention sections"
    );
}

#[test]
fn context_hint_diff_preview() {
    let mut app = App::new("");
    app.diff_preview
        .show(vec![resonance::tui::diff_preview::DiffLine {
            text: "test".to_string(),
            kind: resonance::tui::diff_preview::DiffLineKind::Context,
        }]);
    assert!(
        app.context_hint().contains("Enter:accept"),
        "Diff preview hint should mention Enter"
    );
}

#[test]
fn context_hint_help_screen() {
    let mut app = App::new("");
    app.help_screen.show();
    assert!(
        app.context_hint().contains("close help"),
        "Help hint should mention closing"
    );
}

// =============================================================================
// Keybinding Completeness Tests
// =============================================================================

#[test]
fn all_perform_mode_section_keys_work() {
    for (i, c) in "123456789".chars().enumerate() {
        let action =
            keybindings::map_key_with_diff(key(KeyCode::Char(c)), false, false, FocusPanel::Editor);
        assert_eq!(
            action,
            Some(Action::JumpSection(i)),
            "Key '{c}' should jump to section {i}"
        );
    }
}

#[test]
fn all_layer_toggle_keys_work() {
    let shift_chars = ['!', '@', '#', '$', '%', '^', '&', '*', '('];
    for (i, c) in shift_chars.iter().enumerate() {
        let action =
            keybindings::map_key_with_diff(shift_key(*c), false, false, FocusPanel::Editor);
        assert_eq!(
            action,
            Some(Action::ToggleLayer(i)),
            "Shift+{c} should toggle layer {i}"
        );
    }
}

#[test]
fn all_macro_adjust_keys_work() {
    for n in 1..=8u8 {
        let action =
            keybindings::map_key_with_diff(key(KeyCode::F(n)), false, false, FocusPanel::Editor);
        assert_eq!(
            action,
            Some(Action::AdjustMacro((n - 1) as usize, 0.05)),
            "F{n} should adjust macro {}",
            n - 1
        );
    }
}

// =============================================================================
// Full Lifecycle: Compile → Play → Verify
// =============================================================================

#[test]
fn full_lifecycle_compile_play_verify() {
    let mut app = App::new(sample_src());

    // Step 1: Compile
    app.handle_action(Action::CompileReload);
    assert_eq!(app.status.compile_status, CompileStatus::Ok);
    assert!((app.status.bpm - 128.0).abs() < f64::EPSILON);
    assert!(!app.compiled_events.is_empty());
    assert_eq!(app.track_list.len(), 1);

    // Step 2: Switch to Perform mode
    app.handle_action(Action::ToggleMode);
    assert_eq!(app.mode, AppMode::Perform);

    // Step 3: Start playback
    app.handle_action(Action::TogglePlayback);
    assert!(app.is_playing);

    // Step 4: Advance beat — with scheduler connected, each advance_beat()
    // renders one 1024-frame block. Need ~43 blocks per beat at 44100 Hz / 128 BPM.
    // Render enough blocks to advance past beat 1.
    for _ in 0..100 {
        app.advance_beat();
    }

    assert!(app.current_beat.ticks() > 0);
    assert!(app.status.position_bars > 0 || app.status.position_beats > 0);

    // Step 5: Grid should project with cursor
    let grids = resonance::tui::project_events(&app.compiled_events, 1, 8, Some(app.current_beat));
    assert!(!grids.is_empty());

    // Step 6: Stop playback
    app.handle_action(Action::TogglePlayback);
    assert!(!app.is_playing);
}

#[test]
fn full_lifecycle_focus_isolation_during_edit() {
    let mut app = App::new("");
    app.mode = AppMode::Edit;

    // Type in editor
    app.focus = FocusPanel::Editor;
    app.handle_action(Action::EditorInsert('t'));
    app.handle_action(Action::EditorInsert('e'));
    assert_eq!(app.editor.content(), "te");

    // Switch to Tracks — typing should NOT affect editor
    app.focus = FocusPanel::Tracks;
    // Simulate key event through mapper
    let action =
        keybindings::map_key_with_diff(key(KeyCode::Char('x')), true, false, FocusPanel::Tracks);
    assert_eq!(action, None);
    // Editor content unchanged
    assert_eq!(app.editor.content(), "te");

    // Switch back — typing works again
    app.focus = FocusPanel::Editor;
    app.handle_action(Action::EditorInsert('s'));
    assert_eq!(app.editor.content(), "tes");
}

// =============================================================================
// Audio Pipeline Integration Tests
// =============================================================================

#[test]
fn compile_creates_scheduler() {
    let mut app = App::new(sample_src());
    assert!(
        app.scheduler.is_none(),
        "scheduler should be None before compile"
    );

    app.handle_action(Action::CompileReload);
    assert_eq!(app.status.compile_status, CompileStatus::Ok);
    assert!(
        app.scheduler.is_some(),
        "scheduler should be Some after compile"
    );
}

#[test]
fn play_with_scheduler() {
    let mut app = App::new(sample_src());
    app.handle_action(Action::CompileReload);
    assert!(app.scheduler.is_some());

    // Start playback
    app.handle_action(Action::TogglePlayback);
    assert!(app.is_playing);

    // Scheduler should be in playing state
    let transport = app.scheduler.as_ref().unwrap().transport();
    assert_eq!(transport.state(), resonance::event::PlayState::Playing);

    // Stop playback
    app.handle_action(Action::TogglePlayback);
    assert!(!app.is_playing);
    let transport = app.scheduler.as_ref().unwrap().transport();
    assert_eq!(transport.state(), resonance::event::PlayState::Stopped);
}

#[test]
fn recompile_while_playing() {
    let mut app = App::new(sample_src());
    app.handle_action(Action::CompileReload);
    app.handle_action(Action::TogglePlayback);
    assert!(app.is_playing);

    // Advance a few blocks
    for _ in 0..10 {
        app.advance_beat();
    }
    let _beat_before = app.current_beat;

    // Recompile while playing — should create fresh scheduler (which starts playing)
    app.handle_action(Action::CompileReload);
    assert!(app.scheduler.is_some());

    // The fresh scheduler should be in playing state (since is_playing was true)
    let transport = app.scheduler.as_ref().unwrap().transport();
    assert_eq!(transport.state(), resonance::event::PlayState::Playing);

    // Position resets because it's a new scheduler
    assert_eq!(
        transport.position(),
        resonance::event::Beat::ZERO,
        "fresh scheduler should start from beat zero"
    );

    // But we can continue advancing
    for _ in 0..10 {
        app.advance_beat();
    }
    assert!(
        app.current_beat.ticks() > 0,
        "should advance after recompile"
    );
}
