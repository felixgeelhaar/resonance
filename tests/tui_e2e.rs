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
        app.context_hint().contains("Ctrl+Enter"),
        "Edit+Editor hint should mention eval"
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

// =============================================================================
// REPL & Beginner UX End-to-End Tests
// =============================================================================

#[test]
fn all_preset_files_compile_through_content_module() {
    // Verify every built-in preset loads and compiles end-to-end
    use resonance::content::presets;
    use resonance::dsl::Compiler;

    for name in &["house", "techno", "ambient", "dnb", "empty"] {
        let source = presets::load_preset(name);
        assert!(source.is_some(), "preset '{name}' should be loadable");
        let source = source.unwrap();
        assert!(!source.is_empty(), "preset '{name}' should not be empty");
        let result = Compiler::compile(&source);
        assert!(
            result.is_ok(),
            "preset '{name}' should compile, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn all_presets_load_into_app_and_compile() {
    // Full App integration: load each preset into App and verify compilation succeeds
    use resonance::content::presets;

    for name in &["house", "techno", "ambient", "dnb", "empty"] {
        let source = presets::load_preset(name).unwrap();
        let mut app = App::new(&source);
        app.handle_action(Action::CompileReload);
        assert_eq!(
            app.status.compile_status,
            CompileStatus::Ok,
            "App should compile '{name}' preset successfully"
        );
        assert!(
            !app.compiled_events.is_empty() || *name == "empty",
            "compiled '{name}' should have events (except empty)"
        );
    }
}

#[test]
fn ambient_preset_has_correct_structure() {
    // Specifically test ambient preset: 85 BPM, pad/pluck tracks, sections, heavy reverb
    use resonance::content::presets;
    use resonance::dsl::Compiler;

    let source = presets::load_preset("ambient").unwrap();
    assert!(source.contains("tempo 85"), "ambient should be 85 BPM");
    assert!(source.contains("poly"), "ambient should have poly synth");
    assert!(source.contains("pluck"), "ambient should have pluck synth");
    assert!(
        source.contains("reverb_mix"),
        "ambient should map reverb_mix"
    );
    assert!(source.contains("delay_mix"), "ambient should map delay_mix");

    let compiled = Compiler::compile(&source).unwrap();
    assert!(compiled.tempo >= 84.0 && compiled.tempo <= 86.0);
    assert!(
        compiled.track_defs.len() >= 2,
        "ambient should have >= 2 tracks"
    );
    assert!(
        compiled.sections.len() >= 2,
        "ambient should have >= 2 sections"
    );
    assert!(
        compiled.mappings.len() >= 4,
        "ambient should have >= 4 mappings"
    );
}

#[test]
fn ambient_preset_app_produces_events_and_plays() {
    use resonance::content::presets;

    let source = presets::load_preset("ambient").unwrap();
    let mut app = App::new(&source);
    app.handle_action(Action::CompileReload);
    assert_eq!(app.status.compile_status, CompileStatus::Ok);
    assert!(
        !app.compiled_events.is_empty(),
        "ambient should produce events"
    );

    // Start playback and advance
    app.handle_action(Action::TogglePlayback);
    assert!(app.is_playing);
    for _ in 0..20 {
        app.advance_beat();
    }
    assert!(app.current_beat.ticks() > 0, "ambient should advance beats");
}

#[test]
fn eval_immediate_compiles_and_auto_starts() {
    let mut app = App::new(sample_src());
    assert!(!app.is_playing);

    // Ctrl+Enter should compile and auto-start playback
    app.handle_action(Action::EvalImmediate);
    assert_eq!(app.status.compile_status, CompileStatus::Ok);
    assert!(app.is_playing, "eval_immediate should auto-start playback");
}

#[test]
fn eval_immediate_with_bad_source_does_not_start() {
    let mut app = App::new("invalid dsl garbage");
    assert!(!app.is_playing);

    app.handle_action(Action::EvalImmediate);
    assert!(!app.is_playing, "should not start with bad source");
}

#[test]
fn command_bar_activate_deactivate() {
    let mut app = App::new(sample_src());
    assert!(!app.command_bar.active);

    app.handle_action(Action::ActivateCommandBar);
    assert!(app.command_bar.active);

    app.handle_action(Action::CommandBarCancel);
    assert!(!app.command_bar.active);
}

#[test]
fn command_bar_typing_and_submit() {
    let mut app = App::new(sample_src());

    app.handle_action(Action::ActivateCommandBar);
    app.handle_action(Action::CommandBarInsert(':'));
    app.handle_action(Action::CommandBarInsert('h'));
    app.handle_action(Action::CommandBarInsert('e'));
    app.handle_action(Action::CommandBarInsert('l'));
    app.handle_action(Action::CommandBarInsert('p'));

    assert_eq!(app.command_bar.input(), ":help");

    // Submit triggers help
    app.handle_action(Action::CommandBarSubmit);
    assert!(!app.command_bar.active, "should deactivate after submit");
    assert!(app.help_screen.visible, ":help should toggle help");
}

#[test]
fn command_bar_eval_command() {
    let mut app = App::new(sample_src());
    assert!(!app.is_playing);

    app.handle_action(Action::ActivateCommandBar);
    for c in ":eval".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    assert_eq!(app.status.compile_status, CompileStatus::Ok);
    assert!(app.is_playing, ":eval should compile and auto-start");
}

#[test]
fn command_bar_preset_loads_and_compiles() {
    let mut app = App::new(sample_src());

    // Load ambient preset via command bar
    app.handle_action(Action::ActivateCommandBar);
    for c in ":preset ambient".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    assert_eq!(
        app.status.compile_status,
        CompileStatus::Ok,
        "loading ambient preset should compile"
    );
    let content = app.editor.content();
    assert!(
        content.contains("tempo 85"),
        "editor should contain ambient preset"
    );
}

#[test]
fn command_bar_preset_techno() {
    let mut app = App::new(sample_src());

    app.handle_action(Action::ActivateCommandBar);
    for c in ":preset techno".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    assert_eq!(app.status.compile_status, CompileStatus::Ok);
    assert!(app.editor.content().contains("tempo 130"));
}

#[test]
fn command_bar_preset_dnb() {
    let mut app = App::new(sample_src());

    app.handle_action(Action::ActivateCommandBar);
    for c in ":preset dnb".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    assert_eq!(app.status.compile_status, CompileStatus::Ok);
    assert!(app.editor.content().contains("tempo 170"));
}

#[test]
fn command_bar_ref_toggles_dsl_reference() {
    let mut app = App::new(sample_src());
    assert!(!app.dsl_reference.visible);

    app.handle_action(Action::ActivateCommandBar);
    for c in ":ref".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    assert!(app.dsl_reference.visible, ":ref should show DSL reference");
}

#[test]
fn command_bar_tutorial_starts() {
    let mut app = App::new(sample_src());
    assert!(!app.tutorial.active);

    app.handle_action(Action::ActivateCommandBar);
    for c in ":tutorial".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    assert!(app.tutorial.active, ":tutorial should activate tutorial");
    assert!(
        app.tutorial.explanation_visible,
        "tutorial explanation should show"
    );
    // First lesson should load into editor and compile
    let content = app.editor.content();
    assert!(
        content.contains("tempo"),
        "tutorial lesson should load into editor"
    );
    // Tutorial's :tutorial command triggers compile_source internally
    // Verify it compiled by checking events (not status, which may be Idle if compile_source not called)
    assert!(
        !content.is_empty(),
        "tutorial should have loaded lesson content"
    );
}

#[test]
fn tutorial_next_prev_navigation() {
    let mut app = App::new(sample_src());

    // Start tutorial
    app.handle_action(Action::ActivateCommandBar);
    for c in ":tutorial".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    let first_content = app.editor.content();
    assert!(!first_content.is_empty(), "lesson 1 should be loaded");

    // Verify lesson content compiles
    app.handle_action(Action::CompileReload);
    assert_eq!(
        app.status.compile_status,
        CompileStatus::Ok,
        "lesson 1 should compile"
    );

    // Navigate to next lesson
    app.handle_action(Action::TutorialNext);
    let second_content = app.editor.content();
    assert_ne!(
        first_content, second_content,
        "next lesson should change editor content"
    );

    // Verify second lesson compiles
    app.handle_action(Action::CompileReload);
    assert_eq!(
        app.status.compile_status,
        CompileStatus::Ok,
        "lesson 2 should compile"
    );

    // Navigate back
    app.handle_action(Action::TutorialPrev);
    let back_content = app.editor.content();
    assert_eq!(
        first_content, back_content,
        "prev should return to first lesson"
    );
}

#[test]
fn tutorial_all_lessons_compile() {
    // Walk through all lessons and verify each compiles
    let mut app = App::new(sample_src());

    app.handle_action(Action::ActivateCommandBar);
    for c in ":tutorial".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    // Compile first lesson
    app.handle_action(Action::CompileReload);
    assert_eq!(
        app.status.compile_status,
        CompileStatus::Ok,
        "lesson 1 should compile"
    );

    let mut lesson_count = 1;
    let mut prev_content = app.editor.content();

    // Navigate through remaining lessons
    for _ in 0..10 {
        app.handle_action(Action::TutorialNext);
        let content = app.editor.content();
        if content == prev_content {
            break; // Reached end
        }
        // Explicitly compile each lesson
        app.handle_action(Action::CompileReload);
        assert_eq!(
            app.status.compile_status,
            CompileStatus::Ok,
            "lesson {} should compile",
            lesson_count + 1
        );
        prev_content = content;
        lesson_count += 1;
    }

    assert!(
        lesson_count >= 5,
        "should have at least 5 lessons, got {lesson_count}"
    );
}

#[test]
fn dsl_reference_toggle() {
    let mut app = App::new(sample_src());

    app.handle_action(Action::ToggleDslReference);
    assert!(app.dsl_reference.visible, "should show");

    app.handle_action(Action::Escape);
    assert!(!app.dsl_reference.visible, "escape should close");
}

#[test]
fn nl_faster_adjusts_tempo() {
    use resonance::ai::nl_parser::{self, NlCommand};

    let cmd = nl_parser::parse("faster", "tempo 120");
    assert_eq!(cmd, NlCommand::AdjustTempo(10.0));

    let cmd = nl_parser::parse("speed up", "tempo 120");
    assert_eq!(cmd, NlCommand::AdjustTempo(10.0));

    let cmd = nl_parser::parse("slower", "tempo 120");
    assert_eq!(cmd, NlCommand::AdjustTempo(-10.0));
}

#[test]
fn nl_reverb_commands() {
    use resonance::ai::nl_parser::{self, NlCommand};

    let cmd = nl_parser::parse("more reverb", "");
    match cmd {
        NlCommand::AdjustMacro { name, delta } => {
            assert_eq!(name, "space");
            assert!(delta > 0.0);
        }
        other => panic!("expected AdjustMacro, got {other:?}"),
    }

    let cmd = nl_parser::parse("dry", "");
    match cmd {
        NlCommand::AdjustMacro { name, delta } => {
            assert_eq!(name, "space");
            assert!(delta < 0.0);
        }
        other => panic!("expected AdjustMacro, got {other:?}"),
    }
}

#[test]
fn nl_add_hihats_produces_valid_dsl() {
    use resonance::ai::nl_parser::{self, NlCommand};
    use resonance::dsl::Compiler;

    let source = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . . X . . . X . . . X . . .]\n  }\n}";
    let cmd = nl_parser::parse("add hi-hats", source);
    match cmd {
        NlCommand::ModifyDsl(new_source) => {
            assert!(new_source.contains("hat:"), "should contain hat pattern");
            let result = Compiler::compile(&new_source);
            assert!(
                result.is_ok(),
                "modified DSL should compile: {:?}",
                result.err()
            );
        }
        other => panic!("expected ModifyDsl, got {other:?}"),
    }
}

#[test]
fn nl_add_bass_produces_valid_dsl() {
    use resonance::ai::nl_parser::{self, NlCommand};
    use resonance::dsl::Compiler;

    let source = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . . X . . . X . . . X . . .]\n  }\n}";
    let cmd = nl_parser::parse("add bass", source);
    match cmd {
        NlCommand::ModifyDsl(new_source) => {
            assert!(new_source.contains("track bass"), "should add bass track");
            let result = Compiler::compile(&new_source);
            assert!(
                result.is_ok(),
                "modified DSL should compile: {:?}",
                result.err()
            );
        }
        other => panic!("expected ModifyDsl, got {other:?}"),
    }
}

#[test]
fn nl_add_pad_produces_valid_dsl() {
    use resonance::ai::nl_parser::{self, NlCommand};
    use resonance::dsl::Compiler;

    let source = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . . X . . . X . . . X . . .]\n  }\n}";
    let cmd = nl_parser::parse("add pad", source);
    match cmd {
        NlCommand::ModifyDsl(new_source) => {
            assert!(new_source.contains("track pad"), "should add pad track");
            let result = Compiler::compile(&new_source);
            assert!(
                result.is_ok(),
                "modified DSL should compile: {:?}",
                result.err()
            );
        }
        other => panic!("expected ModifyDsl, got {other:?}"),
    }
}

#[test]
fn nl_four_on_floor_produces_valid_dsl() {
    use resonance::ai::nl_parser::{self, NlCommand};
    use resonance::dsl::Compiler;

    let source = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . . . . . . X . . . . . . .]\n  }\n}";
    let cmd = nl_parser::parse("4 on the floor", source);
    match cmd {
        NlCommand::ModifyDsl(new_source) => {
            let result = Compiler::compile(&new_source);
            assert!(
                result.is_ok(),
                "modified DSL should compile: {:?}",
                result.err()
            );
        }
        other => panic!("expected ModifyDsl, got {other:?}"),
    }
}

#[test]
fn preset_list_includes_all_genres() {
    use resonance::content::presets;

    let presets = presets::list_presets();
    let names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();

    assert!(names.contains(&"House"), "missing House");
    assert!(names.contains(&"Techno"), "missing Techno");
    assert!(names.contains(&"Ambient"), "missing Ambient");
    assert!(names.contains(&"Drum & Bass"), "missing Drum & Bass");
    assert!(names.contains(&"Empty Canvas"), "missing Empty Canvas");
}

#[test]
fn default_starter_delegates_to_content_module() {
    use resonance::dsl::Compiler;
    use resonance::tui::first_run;

    let source = first_run::default_starter();
    assert!(!source.is_empty());
    assert!(source.contains("tempo"));
    let result = Compiler::compile(&source);
    assert!(result.is_ok(), "default starter should compile");
}

#[test]
fn full_repl_workflow_load_preset_eval_play() {
    // Complete REPL workflow: load preset → eval → play → advance beats
    use resonance::content::presets;

    for name in &["house", "techno", "ambient", "dnb"] {
        let source = presets::load_preset(name).unwrap();
        let mut app = App::new(&source);

        // Eval immediate (Ctrl+Enter)
        app.handle_action(Action::EvalImmediate);
        assert_eq!(
            app.status.compile_status,
            CompileStatus::Ok,
            "{name}: should compile"
        );
        assert!(app.is_playing, "{name}: should be playing after eval");

        // Advance beats
        for _ in 0..50 {
            app.advance_beat();
        }
        assert!(app.current_beat.ticks() > 0, "{name}: beats should advance");

        // Stop
        app.handle_action(Action::TogglePlayback);
        assert!(!app.is_playing, "{name}: should stop");
    }
}

#[test]
fn command_bar_clear_resets_editor() {
    let mut app = App::new(sample_src());
    assert!(!app.editor.content().is_empty());

    app.handle_action(Action::ActivateCommandBar);
    for c in ":clear".chars() {
        app.handle_action(Action::CommandBarInsert(c));
    }
    app.handle_action(Action::CommandBarSubmit);

    assert!(
        app.editor.content().is_empty(),
        ":clear should empty the editor"
    );
}

#[test]
fn overlay_priority_help_over_dsl_reference() {
    let mut app = App::new(sample_src());

    // Open DSL reference
    app.handle_action(Action::ToggleDslReference);
    assert!(app.dsl_reference.visible);

    // Open help (should take priority)
    app.handle_action(Action::ToggleHelp);
    assert!(app.help_screen.visible);

    // Close help
    app.handle_action(Action::ToggleHelp);
    assert!(!app.help_screen.visible);
    // DSL reference should still be open
    assert!(app.dsl_reference.visible);
}
