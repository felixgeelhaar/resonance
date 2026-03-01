//! TUI interface — ratatui panels: editor, tracks, grid, macros, intent console.
//!
//! The App struct holds all TUI state and drives the event loop.

pub mod command_bar;
pub mod crash_log;
pub mod diff_preview;
pub mod dsl_reference;
pub mod editor;
pub mod external_input;
pub mod first_run;
pub mod grid;
pub mod help;
pub mod intent_console;
pub mod keybindings;
pub mod layers;
pub mod layout;
pub mod macros;
pub mod status;
pub mod theme;
pub mod tracks;
pub mod tutorial;

pub use command_bar::CommandBar;
pub use crash_log::CrashLog;
pub use diff_preview::DiffPreview;
pub use dsl_reference::DslReference;
pub use editor::Editor;
pub use grid::{project_events, GridCell, GridZoom, TrackGrid};
pub use help::HelpScreen;
pub use intent_console::IntentConsole;
pub use keybindings::{map_key, Action};
pub use layers::LayerPanel;
pub use layout::{AppMode, FocusPanel};
pub use macros::MacroPanel;
pub use status::{CompileStatus, StatusInfo};
pub use tracks::TrackList;
pub use tutorial::TutorialMode;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use ratatui::Frame;

/// Debounce delay before auto-recompiling after an edit (milliseconds).
const COMPILE_DEBOUNCE_MS: u64 = 300;

use crate::audio::AudioEngine;
use crate::dsl::Compiler;
use crate::event::types::ParamId;
use crate::event::{Beat, EventScheduler, RenderFn};
use crate::instrument::InstrumentRouter;
use crate::intent::{IntentProcessor, PerformanceIntent, StructuralIntentProcessor};
use crate::macro_engine::history::MacroHistory;
use crate::macro_engine::{MacroEngine, Mapping};
use crate::section::{Section, SectionController};

/// The main TUI application state.
pub struct App {
    pub editor: Editor,
    pub mode: AppMode,
    pub focus: FocusPanel,
    pub track_list: TrackList,
    pub macro_panel: MacroPanel,
    pub layer_panel: LayerPanel,
    pub diff_preview: DiffPreview,
    pub help_screen: HelpScreen,
    pub intent_console: IntentConsole,
    pub status: StatusInfo,
    pub macro_engine: MacroEngine,
    pub intent_processor: IntentProcessor,
    pub section_controller: SectionController,
    pub compiled_events: Vec<crate::event::types::Event>,
    pub should_quit: bool,
    pub is_playing: bool,
    pub current_beat: Beat,
    pub crash_log: CrashLog,
    pub crash_log_visible: bool,
    pub macro_history: MacroHistory,
    pub grid_zoom: GridZoom,
    pub command_bar: CommandBar,
    pub tutorial: TutorialMode,
    pub dsl_reference: DslReference,
    pub structural_intent_processor: StructuralIntentProcessor,
    #[cfg(feature = "llm")]
    llm_client: Option<crate::ai::llm::LlmClient>,
    external_rx: external_input::ExternalInputReceiver,
    external_tx: external_input::ExternalInputSender,
    // Kept alive to maintain the MIDI connection; messages flow via external_rx.
    #[allow(dead_code)]
    midi_input: Option<crate::midi::MidiInput>,
    // Kept alive to maintain the OSC listener thread; messages flow via external_rx.
    #[allow(dead_code)]
    osc_listener: Option<crate::osc::OscListener>,
    last_tick: Option<Instant>,
    last_device_check: Option<Instant>,
    audio_engine: Option<AudioEngine>,
    pub scheduler: Option<EventScheduler>,
    render_fn: Option<RenderFn>,
    dirty: bool,
    last_edit: Option<Instant>,
    pub theme: theme::Theme,
    available_themes: Vec<theme::Theme>,
}

impl App {
    /// Create a new App with initial DSL source.
    pub fn new(source: &str) -> Self {
        let audio_engine = AudioEngine::new().ok();
        let (external_tx, external_rx) = external_input::external_channel();

        // Attempt MIDI connection
        let midi_config = crate::midi::MidiConfig::load().unwrap_or_default();
        let midi_input = crate::midi::MidiInput::start(&midi_config, external_tx.clone()).ok();

        // Attempt OSC listener — only if config file exists
        let osc_listener = crate::osc::OscConfig::load()
            .and_then(|config| crate::osc::OscListener::start(&config, external_tx.clone()).ok());

        let loaded_theme = theme::load_theme();
        let available_themes = theme::builtin::all_builtins();

        Self {
            editor: Editor::new(source),
            mode: AppMode::Edit,
            focus: FocusPanel::Editor,
            track_list: TrackList::default(),
            macro_panel: MacroPanel::default(),
            layer_panel: LayerPanel::default(),
            diff_preview: DiffPreview::default(),
            help_screen: HelpScreen::default(),
            intent_console: IntentConsole::new(50),
            status: StatusInfo::default(),
            macro_engine: MacroEngine::new(),
            intent_processor: IntentProcessor::new(1),
            section_controller: SectionController::default(),
            compiled_events: Vec::new(),
            should_quit: false,
            is_playing: false,
            current_beat: Beat::ZERO,
            crash_log: CrashLog::default(),
            crash_log_visible: false,
            macro_history: MacroHistory::new(),
            grid_zoom: GridZoom::default(),
            command_bar: CommandBar::default(),
            tutorial: TutorialMode::default(),
            dsl_reference: DslReference::default(),
            structural_intent_processor: StructuralIntentProcessor::new(),
            #[cfg(feature = "llm")]
            llm_client: crate::ai::config::load_config()
                .and_then(|c| crate::ai::llm::LlmClient::from_config(&c)),
            external_rx,
            external_tx,
            midi_input,
            osc_listener,
            last_tick: None,
            last_device_check: None,
            audio_engine,
            scheduler: None,
            render_fn: None,
            dirty: false,
            last_edit: None,
            theme: loaded_theme,
            available_themes,
        }
    }

    /// Process an action.
    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::TogglePlayback => {
                self.is_playing = !self.is_playing;
                self.status.is_playing = self.is_playing;
                if self.is_playing {
                    if let Some(ref mut scheduler) = self.scheduler {
                        scheduler.play();
                    }
                    if let Some(ref engine) = self.audio_engine {
                        let _ = engine.play();
                    }
                } else {
                    if let Some(ref mut scheduler) = self.scheduler {
                        scheduler.stop();
                    }
                    if let Some(ref mut engine) = self.audio_engine {
                        let _ = engine.stop();
                    }
                    self.last_tick = None;
                }
            }
            Action::CompileReload => self.compile_source(),
            Action::ToggleMode => {
                self.mode = self.mode.toggle();
                self.status.is_edit_mode = self.mode == AppMode::Edit;
            }
            Action::CycleFocus => self.focus = self.focus.next(),
            Action::JumpSection(idx) => {
                if self
                    .section_controller
                    .schedule_transition_by_index(idx, self.current_beat)
                {
                    self.intent_console.log(
                        format!("jump to section {}", idx + 1),
                        self.current_beat.as_beats_f64(),
                    );
                }
            }
            Action::AdjustMacro(idx, delta)
            | Action::AdjustMacroFine(idx, delta)
            | Action::AdjustMacroCoarse(idx, delta) => {
                let names: Vec<String> = {
                    let mut n: Vec<String> = self.macro_engine.macros().keys().cloned().collect();
                    n.sort();
                    n
                };
                if let Some(name) = names.get(idx) {
                    // Record current value before change
                    if let Some(current) = self.macro_engine.get_macro(name) {
                        self.macro_history.record(idx, current);
                    }
                    self.macro_engine.adjust_macro(name, delta);
                    self.macro_panel.update(self.macro_engine.macros());
                    let step_label = match &action {
                        Action::AdjustMacroFine(_, _) => "fine",
                        Action::AdjustMacroCoarse(_, _) => "coarse",
                        _ => "",
                    };
                    let msg = if step_label.is_empty() {
                        format!("adjust {} {delta:+.2}", name)
                    } else {
                        format!("adjust {} {delta:+.2} ({step_label})", name)
                    };
                    self.intent_console
                        .log(msg, self.current_beat.as_beats_f64());
                }
            }
            Action::MacroUndo => {
                // Try undo for all macros — find the most recently changed
                // For simplicity, we iterate all macro indices and try undo
                let names: Vec<String> = {
                    let mut n: Vec<String> = self.macro_engine.macros().keys().cloned().collect();
                    n.sort();
                    n
                };
                let mut undone = false;
                for (idx, name) in names.iter().enumerate() {
                    if let Some(prev) = self.macro_history.undo(idx) {
                        self.macro_engine.set_macro(name, prev);
                        self.macro_panel.update(self.macro_engine.macros());
                        self.intent_console.log(
                            format!("undo {} -> {prev:.2}", name),
                            self.current_beat.as_beats_f64(),
                        );
                        undone = true;
                        break;
                    }
                }
                if !undone {
                    self.intent_console
                        .log("nothing to undo", self.current_beat.as_beats_f64());
                }
            }
            Action::MacroRedo => {
                let names: Vec<String> = {
                    let mut n: Vec<String> = self.macro_engine.macros().keys().cloned().collect();
                    n.sort();
                    n
                };
                let mut redone = false;
                for (idx, name) in names.iter().enumerate() {
                    if let Some(val) = self.macro_history.redo(idx) {
                        self.macro_engine.set_macro(name, val);
                        self.macro_panel.update(self.macro_engine.macros());
                        self.intent_console.log(
                            format!("redo {} -> {val:.2}", name),
                            self.current_beat.as_beats_f64(),
                        );
                        redone = true;
                        break;
                    }
                }
                if !redone {
                    self.intent_console
                        .log("nothing to redo", self.current_beat.as_beats_f64());
                }
            }
            Action::ToggleLayer(idx) => {
                if let Some(name) = self.layer_panel.name_at(idx).map(String::from) {
                    if self.section_controller.toggle_layer(&name) {
                        self.update_layer_panel();
                        self.intent_console.log(
                            format!("toggle layer {}", name),
                            self.current_beat.as_beats_f64(),
                        );
                    }
                }
            }
            Action::AcceptDiff => {
                // Apply structural intent if pending
                if let Some(proposed_source) = self.structural_intent_processor.accept_pending() {
                    self.editor.set_content(&proposed_source);
                    self.compile_source();
                }
                self.diff_preview.hide();
                self.focus = FocusPanel::Editor;
                self.intent_console
                    .log("diff accepted", self.current_beat.as_beats_f64());
            }
            Action::RejectDiff => {
                self.structural_intent_processor.reject_pending();
                self.diff_preview.hide();
                self.focus = FocusPanel::Editor;
                self.intent_console
                    .log("diff rejected", self.current_beat.as_beats_f64());
            }
            Action::DiffScrollUp => {
                self.diff_preview.scroll_up();
            }
            Action::DiffScrollDown => {
                self.diff_preview.scroll_down(20);
            }
            Action::EditorInsert(c) => {
                self.editor.insert_char(c);
                self.dirty = true;
                self.last_edit = Some(Instant::now());
            }
            Action::EditorBackspace => {
                self.editor.backspace();
                self.dirty = true;
                self.last_edit = Some(Instant::now());
            }
            Action::EditorDelete => {
                self.editor.delete();
                self.dirty = true;
                self.last_edit = Some(Instant::now());
            }
            Action::EditorLeft => self.editor.move_left(),
            Action::EditorRight => self.editor.move_right(),
            Action::EditorUp => self.editor.move_up(),
            Action::EditorDown => self.editor.move_down(),
            Action::EditorNewline => {
                self.editor.newline();
                self.dirty = true;
                self.last_edit = Some(Instant::now());
            }
            Action::EditorHome => self.editor.home(),
            Action::EditorEnd => self.editor.end(),
            Action::ToggleHelp => {
                self.help_screen.toggle();
            }
            Action::ToggleCrashLog => {
                self.crash_log_visible = !self.crash_log_visible;
            }
            Action::Escape => {
                if self.crash_log_visible {
                    self.crash_log_visible = false;
                } else if self.help_screen.visible {
                    self.help_screen.hide();
                } else if self.dsl_reference.visible {
                    self.dsl_reference.hide();
                } else if self.tutorial.active && self.tutorial.explanation_visible {
                    self.tutorial.toggle_explanation();
                } else if self.focus != FocusPanel::Editor {
                    self.focus = FocusPanel::Editor;
                }
            }
            Action::GridZoomIn => {
                self.grid_zoom = self.grid_zoom.zoom_in();
            }
            Action::GridZoomOut => {
                self.grid_zoom = self.grid_zoom.zoom_out();
            }
            Action::CycleTheme => {
                self.theme = theme::cycle_theme(&self.theme, &self.available_themes);
                self.intent_console.log(
                    format!("theme: {}", self.theme.name),
                    self.current_beat.as_beats_f64(),
                );
            }
            Action::PanelNavigate(_key_code) => {
                // Panel-specific navigation — currently a no-op for content scrolling.
                // Future: scroll track list, grid cursor, etc.
            }
            Action::EvalImmediate => self.eval_immediate(),
            Action::ActivateCommandBar => {
                self.command_bar.activate();
            }
            Action::CommandBarInsert(c) => {
                self.command_bar.insert_char(c);
            }
            Action::CommandBarSubmit => {
                let input = self.command_bar.submit();
                if !input.is_empty() {
                    self.process_command(&input);
                }
            }
            Action::CommandBarCancel => {
                self.command_bar.deactivate();
            }
            Action::CommandBarBackspace => {
                self.command_bar.backspace();
            }
            Action::CommandBarLeft => {
                self.command_bar.move_left();
            }
            Action::CommandBarRight => {
                self.command_bar.move_right();
            }
            Action::CommandBarHistoryUp => {
                self.command_bar.history_up();
            }
            Action::CommandBarHistoryDown => {
                self.command_bar.history_down();
            }
            Action::TutorialNext => {
                if self.tutorial.next_lesson() {
                    if let Some(lesson) = self.tutorial.current_lesson() {
                        let code = lesson.code.trim().to_string();
                        self.editor.set_content(&code);
                    }
                    self.intent_console.log(
                        format!(
                            "tutorial: lesson {}/{}",
                            self.tutorial.current_index() + 1,
                            self.tutorial.total_lessons()
                        ),
                        self.current_beat.as_beats_f64(),
                    );
                }
            }
            Action::TutorialPrev => {
                if self.tutorial.prev_lesson() {
                    if let Some(lesson) = self.tutorial.current_lesson() {
                        let code = lesson.code.trim().to_string();
                        self.editor.set_content(&code);
                    }
                }
            }
            Action::ToggleDslReference => {
                self.dsl_reference.toggle();
            }
            Action::ReconnectAudio => {
                self.reconnect_audio_device();
            }
        }
    }

    /// Advance the current beat — renders audio when pipeline is connected,
    /// falls back to wall-clock advancement for visual-only mode.
    /// Wrapped in catch_unwind to prevent panics from crashing the UI.
    pub fn advance_beat(&mut self) {
        if !self.is_playing {
            self.last_tick = None;
            return;
        }

        // Try real audio rendering if the full pipeline is available
        if self.scheduler.is_some() && self.render_fn.is_some() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let scheduler = self.scheduler.as_mut().unwrap();
                let render_fn = self.render_fn.as_mut().unwrap();
                let macro_engine = &self.macro_engine;

                if let Some(samples) =
                    scheduler.render_block_with(render_fn, |e| macro_engine.apply_to_event(e))
                {
                    if let Some(ref mut engine) = self.audio_engine {
                        let _ = engine.send_samples(samples);

                        // Send resolved FX params to audio engine
                        let resolved = macro_engine.resolve_params();
                        for param_name in
                            &["reverb_mix", "reverb_decay", "delay_mix", "delay_feedback"]
                        {
                            let key = ParamId(param_name.to_string());
                            if let Some(&val) = resolved.get(&key) {
                                let _ = engine.send_effect_param(param_name.to_string(), val);
                            }
                        }
                    }

                    let pos = scheduler.transport().position();
                    Some(pos)
                } else {
                    None
                }
            }));

            match result {
                Ok(Some(pos)) => {
                    self.current_beat = pos;
                    let total_beats = self.current_beat.as_beats_f64();
                    self.status.position_bars = (total_beats / 4.0).floor() as u64;
                    self.status.position_beats = (total_beats % 4.0).floor() as u64;
                }
                Ok(None) => {}
                Err(panic_info) => {
                    let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        format!("audio panic: {s}")
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        format!("audio panic: {s}")
                    } else {
                        "audio panic: unknown".to_string()
                    };
                    self.crash_log.push(msg.clone());
                    self.intent_console
                        .log(msg, self.current_beat.as_beats_f64());
                    // Stop playback on panic to avoid repeated crashes
                    self.is_playing = false;
                    self.status.is_playing = false;
                    self.last_tick = None;
                }
            }
            return;
        }

        // Wall-clock fallback: visual-only mode (no scheduler)
        let now = Instant::now();
        if let Some(last) = self.last_tick {
            let elapsed = now.duration_since(last);
            let beats_per_second = self.status.bpm / 60.0;
            let ticks_per_second = beats_per_second * 960.0; // 960 PPQN
            let delta_ticks = (ticks_per_second * elapsed.as_secs_f64()).round() as u64;
            if delta_ticks > 0 {
                self.current_beat = Beat::from_ticks(self.current_beat.ticks() + delta_ticks);
                let total_beats = self.current_beat.as_beats_f64();
                self.status.position_bars = (total_beats / 4.0).floor() as u64;
                self.status.position_beats = (total_beats % 4.0).floor() as u64;
            }
        }
        self.last_tick = Some(now);
    }

    /// Set the last tick time (for testing beat advancement).
    pub fn set_last_tick(&mut self, instant: Instant) {
        self.last_tick = Some(instant);
    }

    /// Compile the editor content and update state.
    /// Errors are caught and logged to the crash log instead of propagating.
    fn compile_source(&mut self) {
        let source = self.editor.content();
        match Compiler::compile(&source) {
            Ok(song) => {
                // Clamp BPM to valid range
                self.status.bpm = song.tempo.clamp(20.0, 999.0);
                self.status.compile_status = CompileStatus::Ok;
                self.macro_engine = MacroEngine::from_compiled(&song.macros, &song.mappings);
                self.macro_panel.update(self.macro_engine.macros());

                let track_defs: Vec<(String, String)> = song
                    .track_defs
                    .iter()
                    .map(|(_, td)| {
                        let inst = format!("{:?}", td.instrument);
                        (td.name.clone(), inst)
                    })
                    .collect();
                self.track_list = TrackList::from_defs(&track_defs);

                // Populate SectionController from compiled sections
                let sections: Vec<Section> = song
                    .sections
                    .iter()
                    .map(|cs| Section {
                        name: cs.name.clone(),
                        length_in_bars: cs.length_in_bars,
                        mapping_overrides: cs
                            .mapping_overrides
                            .iter()
                            .map(|o| Mapping {
                                macro_name: o.macro_name.clone(),
                                target_param: ParamId(o.target_param.clone()),
                                range: o.range,
                                curve: o.curve,
                            })
                            .collect(),
                    })
                    .collect();
                self.section_controller = SectionController::new(sections);

                // Populate layers from compiled song
                for layer_def in &song.layers {
                    let layer = crate::section::Layer {
                        name: layer_def.name.clone(),
                        mapping_additions: layer_def
                            .mappings
                            .iter()
                            .map(|m| Mapping {
                                macro_name: m.macro_name.clone(),
                                target_param: ParamId(m.target_param.clone()),
                                range: m.range,
                                curve: m.curve,
                            })
                            .collect(),
                        enabled: layer_def.enabled_by_default,
                    };
                    self.section_controller.add_layer(layer);
                }
                self.update_layer_panel();

                // Store compiled events for grid visualization
                self.compiled_events = song.events.clone();

                // Build audio pipeline: scheduler + instrument router
                let seed = 42u64;
                let (sample_rate, channels) = match &self.audio_engine {
                    Some(engine) => (engine.sample_rate(), engine.channels()),
                    None => (44100, 2),
                };
                let router = InstrumentRouter::from_track_defs_with_kits(
                    &song.track_defs,
                    sample_rate,
                    seed,
                );
                let mut scheduler =
                    EventScheduler::new(song.tempo, sample_rate, channels, 1024, seed);
                scheduler.timeline_mut().insert_batch(song.events.clone());
                if self.is_playing {
                    scheduler.play();
                }
                self.scheduler = Some(scheduler);
                self.render_fn = Some(router.into_render_fn());

                self.intent_console
                    .log("compiled OK", self.current_beat.as_beats_f64());
            }
            Err(e) => {
                self.status.compile_status = CompileStatus::Error(e.to_string());
                self.intent_console.log(
                    format!("compile error: {e}"),
                    self.current_beat.as_beats_f64(),
                );
            }
        }
    }

    /// Update the layer panel from the section controller's layers.
    fn update_layer_panel(&mut self) {
        // We need to get layer states from the section controller.
        // The active_mappings method gives us active layers, but we need names + enabled state.
        // For now we track via the layer_panel itself — populated during compile.
        // After toggle, we re-read states.
        let layer_states: Vec<(String, bool)> = self
            .section_controller
            .layer_states()
            .iter()
            .map(|(n, e)| (n.clone(), *e))
            .collect();
        self.layer_panel.update(&layer_states);
    }

    /// Reconnect to the default audio output device.
    ///
    /// Drops the current engine, creates a new one on the default device,
    /// and re-compiles if the sample rate or channel count changed.
    fn reconnect_audio_device(&mut self) {
        let old_config = self
            .audio_engine
            .as_ref()
            .map(|e| (e.sample_rate(), e.channels()));

        // Drop old engine
        self.audio_engine = None;

        match AudioEngine::new() {
            Ok(engine) => {
                let name = engine.device_name().to_string();
                let sr = engine.sample_rate();
                let ch = engine.channels();

                self.audio_engine = Some(engine);

                self.intent_console.log(
                    format!("audio: reconnected to '{name}' ({sr}Hz, {ch}ch)"),
                    self.current_beat.as_beats_f64(),
                );

                // Re-compile if audio config changed
                if old_config.is_some_and(|(osr, och)| osr != sr || och != ch) {
                    self.compile_source();
                }

                // Resume playback if we were playing
                if self.is_playing {
                    if let Some(ref engine) = self.audio_engine {
                        let _ = engine.play();
                    }
                }
            }
            Err(e) => {
                let msg = format!("audio: reconnect failed — {e}");
                self.crash_log.push(msg.clone());
                self.intent_console
                    .log(msg, self.current_beat.as_beats_f64());
            }
        }
    }

    /// Best-effort auto-detection of audio device changes.
    ///
    /// Polls the default device every 2 seconds and reconnects if the
    /// device name, sample rate, or channel count changed.
    /// Note: built-in headphone jack may share name+config with speakers,
    /// so `Ctrl+D` remains the fallback for that case.
    fn check_audio_device(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_device_check {
            if now.duration_since(last) < Duration::from_secs(2) {
                return;
            }
        }
        self.last_device_check = Some(now);

        let current = self
            .audio_engine
            .as_ref()
            .map(|e| (e.device_name().to_string(), e.sample_rate(), e.channels()));

        if let Ok((name, sr, ch)) = AudioEngine::default_device_info() {
            if let Some((cur_name, cur_sr, cur_ch)) = current {
                if name != cur_name || sr != cur_sr || ch != cur_ch {
                    self.reconnect_audio_device();
                }
            }
        }
    }

    /// Evaluate code immediately (REPL mode). Compiles and auto-starts playback.
    fn eval_immediate(&mut self) {
        self.compile_source();
        self.dirty = false;
        self.last_edit = None;
        if !self.is_playing && self.status.compile_status == CompileStatus::Ok {
            self.handle_action(Action::TogglePlayback);
        }
    }

    /// Process a command from the command bar.
    fn process_command(&mut self, input: &str) {
        let trimmed = input.trim();

        // : commands
        if let Some(cmd) = trimmed.strip_prefix(':') {
            let cmd = cmd.trim();
            match cmd {
                "tutorial" | "learn" => {
                    self.tutorial.start();
                    if let Some(lesson) = self.tutorial.current_lesson() {
                        let code = lesson.code.trim().to_string();
                        self.editor.set_content(&code);
                    }
                    self.intent_console
                        .log("tutorial: started", self.current_beat.as_beats_f64());
                }
                "next" => {
                    self.handle_action(Action::TutorialNext);
                }
                "prev" => {
                    self.handle_action(Action::TutorialPrev);
                }
                "ref" | "reference" => {
                    self.dsl_reference.toggle();
                }
                "help" => {
                    self.help_screen.toggle();
                }
                "eval" => {
                    self.eval_immediate();
                }
                "clear" => {
                    self.editor.set_content("");
                    self.intent_console
                        .log("editor cleared", self.current_beat.as_beats_f64());
                }
                "audio" | "reconnect" => {
                    self.reconnect_audio_device();
                }
                "presets" => {
                    let presets = crate::content::presets::list_presets();
                    for p in &presets {
                        self.intent_console.log(
                            format!("preset: {} — {}", p.name, p.description),
                            self.current_beat.as_beats_f64(),
                        );
                    }
                }
                "themes" => {
                    for t in &self.available_themes {
                        self.intent_console.log(
                            format!("theme: {}", t.name),
                            self.current_beat.as_beats_f64(),
                        );
                    }
                }
                _ if cmd.starts_with("preset ") => {
                    let name = cmd.strip_prefix("preset ").unwrap().trim();
                    if let Some(source) = crate::content::presets::load_preset(name) {
                        self.editor.set_content(&source);
                        self.compile_source();
                        self.intent_console.log(
                            format!("loaded preset: {name}"),
                            self.current_beat.as_beats_f64(),
                        );
                    } else {
                        self.intent_console.log(
                            format!("preset not found: {name}"),
                            self.current_beat.as_beats_f64(),
                        );
                    }
                }
                _ if cmd.starts_with("theme ") => {
                    let name = cmd.strip_prefix("theme ").unwrap().trim();
                    if let Some(t) = self
                        .available_themes
                        .iter()
                        .find(|t| t.name.to_lowercase() == name.to_lowercase())
                    {
                        self.theme = t.clone();
                        self.intent_console.log(
                            format!("theme: {}", self.theme.name),
                            self.current_beat.as_beats_f64(),
                        );
                    } else {
                        self.intent_console.log(
                            format!("theme not found: {name}"),
                            self.current_beat.as_beats_f64(),
                        );
                    }
                }
                _ if cmd.starts_with("save ") => {
                    let path = cmd.strip_prefix("save ").unwrap().trim();
                    match std::fs::write(path, self.editor.content()) {
                        Ok(()) => {
                            self.intent_console
                                .log(format!("saved to {path}"), self.current_beat.as_beats_f64());
                        }
                        Err(e) => {
                            self.intent_console
                                .log(format!("save error: {e}"), self.current_beat.as_beats_f64());
                        }
                    }
                }
                _ if cmd.starts_with("load ") => {
                    let path = cmd.strip_prefix("load ").unwrap().trim();
                    match std::fs::read_to_string(path) {
                        Ok(content) => {
                            self.editor.set_content(&content);
                            self.compile_source();
                            self.intent_console
                                .log(format!("loaded {path}"), self.current_beat.as_beats_f64());
                        }
                        Err(e) => {
                            self.intent_console
                                .log(format!("load error: {e}"), self.current_beat.as_beats_f64());
                        }
                    }
                }
                _ => {
                    self.intent_console.log(
                        format!("unknown command: :{cmd}"),
                        self.current_beat.as_beats_f64(),
                    );
                }
            }
            return;
        }

        // Natural language input
        let nl_cmd = crate::ai::nl_parser::parse(trimmed, &self.editor.content());
        match nl_cmd {
            crate::ai::nl_parser::NlCommand::SetTempo(bpm) => {
                self.status.bpm = bpm.clamp(20.0, 999.0);
                self.intent_console.log(
                    format!("tempo: {:.0}", self.status.bpm),
                    self.current_beat.as_beats_f64(),
                );
            }
            crate::ai::nl_parser::NlCommand::AdjustTempo(delta) => {
                // Handle half time / double time special cases
                if delta == -0.5 {
                    self.status.bpm = (self.status.bpm * 0.5).clamp(20.0, 999.0);
                } else if delta == 2.0 {
                    self.status.bpm = (self.status.bpm * 2.0).clamp(20.0, 999.0);
                } else {
                    self.status.bpm = (self.status.bpm + delta).clamp(20.0, 999.0);
                }
                self.intent_console.log(
                    format!("tempo: {:.0}", self.status.bpm),
                    self.current_beat.as_beats_f64(),
                );
            }
            crate::ai::nl_parser::NlCommand::AdjustMacro { name, delta } => {
                self.macro_engine.adjust_macro(&name, delta);
                self.macro_panel.update(self.macro_engine.macros());
                self.intent_console.log(
                    format!("adjust {name} {delta:+.2}"),
                    self.current_beat.as_beats_f64(),
                );
            }
            crate::ai::nl_parser::NlCommand::TogglePlayback => {
                self.handle_action(Action::TogglePlayback);
            }
            crate::ai::nl_parser::NlCommand::ModifyDsl(proposed_source) => {
                // Structural change — show diff preview
                let current_source = self.editor.content();
                if let (Ok(old_ast), Ok(new_ast)) = (
                    Compiler::parse(&current_source),
                    Compiler::parse(&proposed_source),
                ) {
                    let diff = crate::dsl::diff::AstDiff::diff(&old_ast, &new_ast);
                    let diff_lines: Vec<diff_preview::DiffLine> = diff
                        .changes
                        .iter()
                        .map(|change| {
                            let text = format!("{change:?}");
                            diff_preview::DiffLine {
                                text,
                                kind: diff_preview::DiffLineKind::Modification,
                            }
                        })
                        .collect();

                    if diff_lines.is_empty() {
                        self.intent_console
                            .log("no changes detected", self.current_beat.as_beats_f64());
                    } else {
                        let _id = self.structural_intent_processor.propose(
                            trimmed.to_string(),
                            diff,
                            proposed_source,
                        );
                        self.diff_preview.show(diff_lines);
                        self.intent_console.log(
                            "proposed change — Enter/Esc",
                            self.current_beat.as_beats_f64(),
                        );
                    }
                } else {
                    // Fallback: apply directly if parse fails for diff
                    self.editor.set_content(&proposed_source);
                    self.compile_source();
                    self.intent_console
                        .log("applied change", self.current_beat.as_beats_f64());
                }
            }
            crate::ai::nl_parser::NlCommand::LoadPreset(name) => {
                self.process_command(&format!(":preset {name}"));
            }
            crate::ai::nl_parser::NlCommand::StartTutorial => {
                self.process_command(":tutorial");
            }
            crate::ai::nl_parser::NlCommand::ShowHelp => {
                self.help_screen.toggle();
            }
            crate::ai::nl_parser::NlCommand::ShowReference => {
                self.dsl_reference.toggle();
            }
            crate::ai::nl_parser::NlCommand::Unknown(text) => {
                #[cfg(feature = "llm")]
                {
                    if self.llm_client.is_some() {
                        self.intent_console
                            .log("AI thinking...", self.current_beat.as_beats_f64());
                        let source = self.editor.content();
                        let input = text.clone();
                        let tx = self.external_tx.clone();
                        std::thread::spawn(move || {
                            // Re-load config and create client in thread
                            if let Some(config) = crate::ai::config::load_config() {
                                if let Some(client) =
                                    crate::ai::llm::LlmClient::from_config(&config)
                                {
                                    match client.transform(&input, &source) {
                                        Ok(proposed) => {
                                            let _ = tx.send(
                                                external_input::ExternalEvent::AiResponse {
                                                    input,
                                                    proposed_source: proposed,
                                                },
                                            );
                                        }
                                        Err(e) => {
                                            eprintln!("LLM error: {e}");
                                        }
                                    }
                                }
                            }
                        });
                        return;
                    }
                }
                self.intent_console
                    .log(format!("unknown: {text}"), self.current_beat.as_beats_f64());
            }
        }
    }

    /// Draw the UI.
    pub fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Determine if command bar is visible (needs extra row)
        let cmd_bar_height = if self.command_bar.active { 1 } else { 0 };

        // Main vertical layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),         // Editor + Tracks
                Constraint::Percentage(30),         // Grid
                Constraint::Percentage(20),         // Macros + Intent Console
                Constraint::Length(cmd_bar_height), // Command bar (conditional)
                Constraint::Length(1),              // Status bar
            ])
            .split(size);

        // Top row: Editor + Tracks
        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(chunks[0]);

        self.draw_editor(frame, top[0]);
        self.draw_tracks(frame, top[1]);

        // Middle: Grid
        self.draw_grid(frame, chunks[1]);

        // Bottom row: Macros + Intent Console
        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        self.draw_macros(frame, bottom[0]);
        self.draw_intent_console(frame, bottom[1]);

        // Command bar (conditional)
        if self.command_bar.active {
            self.draw_command_bar(frame, chunks[3]);
        }

        // Status bar
        self.draw_status(frame, chunks[4]);

        // Overlay priority: help > dsl_reference > tutorial > crash_log > diff_preview
        if self.diff_preview.visible {
            self.draw_diff_preview(frame, size);
        }

        if self.crash_log_visible {
            self.draw_crash_log(frame, size);
        }

        if self.tutorial.active && self.tutorial.explanation_visible {
            self.draw_tutorial_explanation(frame, size);
        }

        if self.dsl_reference.visible {
            self.draw_dsl_reference(frame, size);
        }

        // Help overlay (rendered on top of everything)
        if self.help_screen.visible {
            self.draw_help(frame, size);
        }
    }

    fn draw_editor(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::Editor;
        let border_style = if focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        // Compute inner height (subtract 2 for top+bottom borders)
        let inner_height = area.height.saturating_sub(2) as usize;
        self.editor.set_viewport_height(inner_height);
        let scroll_offset = self.editor.scroll_offset();

        let line_num_color = self.theme.editor_line_number;
        let lines: Vec<Line> = self
            .editor
            .lines()
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(inner_height)
            .map(|(i, line)| {
                let num =
                    Span::styled(format!("{:3} ", i + 1), Style::default().fg(line_num_color));
                let content = Span::raw(line.as_str());
                Line::from(vec![num, content])
            })
            .collect();

        let block = Block::default()
            .title(format!(
                " Code [{}] ",
                if self.mode == AppMode::Edit {
                    "EDIT"
                } else {
                    "VIEW"
                }
            ))
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);

        // Show cursor in edit mode
        if focused && self.mode == AppMode::Edit {
            let (row, col) = self.editor.cursor();
            // +1 for border, +4 for line number; adjust row by scroll_offset
            let x = area.x + 1 + 4 + col as u16;
            let y = area.y + 1 + (row - scroll_offset) as u16;
            if x < area.x + area.width && y < area.y + area.height {
                frame.set_cursor_position((x, y));
            }
        }
    }

    fn draw_tracks(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::Tracks;
        let border_style = if focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        let items: Vec<ListItem> = self
            .track_list
            .tracks
            .iter()
            .map(|t| {
                let mute_indicator = if t.muted { "[M]" } else { "   " };
                ListItem::new(format!(
                    "{} {} ({})",
                    mute_indicator, t.name, t.instrument_type
                ))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Tracks ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_widget(list, area);
    }

    fn draw_grid(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::Grid;
        let border_style = if focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        let zoom_label = self.grid_zoom.label();
        let block = Block::default()
            .title(format!(" Grid [{zoom_label}] "))
            .borders(Borders::ALL)
            .border_style(border_style);

        if self.compiled_events.is_empty() {
            let paragraph = Paragraph::new("(no events — compile with Ctrl-R)").block(block);
            frame.render_widget(paragraph, area);
            return;
        }

        let cursor = if self.is_playing {
            Some(self.current_beat)
        } else {
            None
        };
        let steps_per_bar = self.grid_zoom.steps_per_bar();
        let grids = grid::project_events(&self.compiled_events, 2, steps_per_bar, cursor);

        let theme = &self.theme;
        let lines: Vec<Line> = grids
            .iter()
            .map(|tg| {
                let tc = grid::track_color(&tg.track_name, &theme.grid_palette);
                let mut spans = vec![Span::styled(
                    format!("{:>8} ", tg.track_name),
                    Style::default().fg(tc),
                )];
                for cell in &tg.cells {
                    let (text, color) = match cell {
                        GridCell::Empty => (".", theme.grid_empty),
                        GridCell::Hit(v) => {
                            let c = grid::velocity_color(
                                *v,
                                tc,
                                theme.grid_hit_bright,
                                theme.grid_hit_dim,
                            );
                            if *v > 0.6 {
                                ("X", c)
                            } else {
                                ("x", c)
                            }
                        }
                        GridCell::Note(_) => ("N", tc),
                        GridCell::Cursor => ("|", theme.grid_playhead),
                    };
                    spans.push(Span::styled(format!("{text} "), Style::default().fg(color)));
                }
                Line::from(spans)
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn draw_macros(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::Macros;
        let border_style = if focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        let block = Block::default()
            .title(" Macros ")
            .borders(Borders::ALL)
            .border_style(border_style);

        if self.macro_panel.is_empty() {
            let paragraph = Paragraph::new("(no macros)").block(block);
            frame.render_widget(paragraph, area);
        } else {
            let inner = block.inner(area);
            frame.render_widget(block, area);

            // Draw each macro as a gauge
            let gauge_height = 1;
            for (i, meter) in self.macro_panel.meters.iter().enumerate() {
                let y = inner.y + i as u16 * gauge_height;
                if y >= inner.y + inner.height {
                    break;
                }
                let gauge_area = Rect::new(inner.x, y, inner.width, gauge_height);
                let gauge = Gauge::default()
                    .label(format!("{}: {:.0}%", meter.name, meter.value * 100.0))
                    .ratio(meter.value.clamp(0.0, 1.0))
                    .gauge_style(Style::default().fg(self.theme.macro_bar));
                frame.render_widget(gauge, gauge_area);
            }
        }
    }

    fn draw_intent_console(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::IntentConsole;
        let border_style = if focused {
            Style::default().fg(self.theme.border_focused)
        } else {
            Style::default().fg(self.theme.border)
        };

        let items: Vec<ListItem> = self
            .intent_console
            .entries()
            .iter()
            .rev()
            .map(|e| ListItem::new(format!("[{:.1}] {}", e.timestamp_beats, e.message)))
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(" Intent Console ")
                .borders(Borders::ALL)
                .border_style(border_style),
        );

        frame.render_widget(list, area);
    }

    fn draw_diff_preview(&self, frame: &mut Frame, area: Rect) {
        use crate::tui::diff_preview::DiffLineKind;

        // Centered overlay: 60% width, 60% height
        let width = (area.width * 60 / 100).max(40);
        let height = (area.height * 60 / 100).max(10);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let overlay = Rect::new(x, y, width, height);

        // Clear the background
        let clear = Block::default()
            .style(Style::default().bg(Color::Black))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.title))
            .title(" Diff Preview ");
        let inner = clear.inner(overlay);
        frame.render_widget(clear, overlay);

        // Render visible lines
        let max_lines = inner.height as usize;
        let visible = self.diff_preview.visible_lines(max_lines);
        let theme = &self.theme;
        let lines: Vec<Line> = visible
            .iter()
            .map(|dl| {
                let color = match dl.kind {
                    DiffLineKind::Header => theme.title,
                    DiffLineKind::Addition => theme.diff_add,
                    DiffLineKind::Removal => theme.diff_remove,
                    DiffLineKind::Modification => theme.border_focused,
                    DiffLineKind::Context => theme.editor_line_number,
                };
                Line::from(Span::styled(&dl.text, Style::default().fg(color)))
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let width = (area.width * 70 / 100).max(50);
        let height = (area.height * 70 / 100).max(15);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let overlay = Rect::new(x, y, width, height);

        let block = Block::default()
            .style(Style::default().bg(Color::Black))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_focused))
            .title(" Help — Press ? or Esc to close ");
        let inner = block.inner(overlay);
        frame.render_widget(block, overlay);

        let lines: Vec<Line> = self
            .help_screen
            .lines()
            .iter()
            .skip(self.help_screen.scroll_offset)
            .take(inner.height as usize)
            .map(|hl| {
                let color = if hl.is_header {
                    self.theme.help_key
                } else {
                    self.theme.help_desc
                };
                Line::from(Span::styled(&hl.text, Style::default().fg(color)))
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    fn draw_crash_log(&self, frame: &mut Frame, area: Rect) {
        let width = (area.width * 70 / 100).max(50);
        let height = (area.height * 50 / 100).max(10);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let overlay = Rect::new(x, y, width, height);

        let block = Block::default()
            .style(Style::default().bg(Color::Black))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.diff_remove))
            .title(" Crash Log — Press Ctrl-L or Esc to close ");
        let inner = block.inner(overlay);
        frame.render_widget(block, overlay);

        if self.crash_log.is_empty() {
            let paragraph = Paragraph::new("(no errors recorded)")
                .style(Style::default().fg(self.theme.editor_line_number));
            frame.render_widget(paragraph, inner);
        } else {
            let theme = &self.theme;
            let lines: Vec<Line> = self
                .crash_log
                .entries()
                .map(|entry| {
                    let elapsed = entry
                        .timestamp
                        .elapsed()
                        .map(|d| format!("{:.0}s ago", d.as_secs_f64()))
                        .unwrap_or_else(|_| "?".to_string());
                    Line::from(vec![
                        Span::styled(
                            format!("[{elapsed}] "),
                            Style::default().fg(theme.editor_line_number),
                        ),
                        Span::styled(&entry.message, Style::default().fg(theme.diff_remove)),
                    ])
                })
                .collect();
            let paragraph = Paragraph::new(lines);
            frame.render_widget(paragraph, inner);
        }
    }

    fn draw_command_bar(&self, frame: &mut Frame, area: Rect) {
        let theme = &self.theme;
        let input = self.command_bar.input();
        let line = Line::from(vec![
            Span::styled(
                " > ",
                Style::default()
                    .fg(theme.border_focused)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(input),
        ]);
        let paragraph =
            Paragraph::new(line).style(Style::default().bg(theme.status_bg).fg(theme.status_fg));
        frame.render_widget(paragraph, area);

        // Show cursor in command bar
        let x = area.x + 3 + self.command_bar.cursor_pos() as u16;
        let y = area.y;
        if x < area.x + area.width {
            frame.set_cursor_position((x, y));
        }
    }

    fn draw_tutorial_explanation(&self, frame: &mut Frame, area: Rect) {
        let width = (area.width * 60 / 100).max(40);
        let height = (area.height * 50 / 100).max(10);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let overlay = Rect::new(x, y, width, height);

        let lesson_info = if let Some(lesson) = self.tutorial.current_lesson() {
            format!(
                " Tutorial: {} ({}/{}) — Esc to dismiss ",
                lesson.title,
                self.tutorial.current_index() + 1,
                self.tutorial.total_lessons()
            )
        } else {
            " Tutorial ".to_string()
        };

        let block = Block::default()
            .style(Style::default().bg(Color::Black))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_focused))
            .title(lesson_info);
        let inner = block.inner(overlay);
        frame.render_widget(block, overlay);

        if let Some(lesson) = self.tutorial.current_lesson() {
            let theme = &self.theme;
            let mut all_lines: Vec<Line> = Vec::new();

            for text in &lesson.explanation {
                all_lines.push(Line::from(Span::styled(
                    text.as_str(),
                    Style::default().fg(theme.help_desc),
                )));
            }

            if !lesson.hints.is_empty() {
                all_lines.push(Line::from(""));
                all_lines.push(Line::from(Span::styled(
                    "Hints:",
                    Style::default()
                        .fg(theme.help_key)
                        .add_modifier(Modifier::BOLD),
                )));
                for hint in &lesson.hints {
                    all_lines.push(Line::from(Span::styled(
                        format!("  - {hint}"),
                        Style::default().fg(theme.editor_line_number),
                    )));
                }
            }

            let visible: Vec<Line> = all_lines
                .into_iter()
                .skip(self.tutorial.scroll_offset)
                .take(inner.height as usize)
                .collect();

            let paragraph = Paragraph::new(visible);
            frame.render_widget(paragraph, inner);
        }
    }

    fn draw_dsl_reference(&self, frame: &mut Frame, area: Rect) {
        let width = (area.width * 70 / 100).max(50);
        let height = (area.height * 70 / 100).max(15);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let overlay = Rect::new(x, y, width, height);

        let block = Block::default()
            .style(Style::default().bg(Color::Black))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_focused))
            .title(" DSL Reference — Shift+? or Esc to close ");
        let inner = block.inner(overlay);
        frame.render_widget(block, overlay);

        let lines: Vec<Line> = self
            .dsl_reference
            .lines()
            .iter()
            .skip(self.dsl_reference.scroll_offset)
            .take(inner.height as usize)
            .map(|hl| {
                let color = if hl.is_header {
                    self.theme.help_key
                } else {
                    self.theme.help_desc
                };
                Line::from(Span::styled(&hl.text, Style::default().fg(color)))
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    /// Context-sensitive hint for the status bar.
    pub fn context_hint(&self) -> &str {
        if self.command_bar.active {
            return "> type command or natural language | Esc:cancel";
        }
        if self.crash_log_visible {
            return "Ctrl-L/Esc:close crash log";
        }
        if self.help_screen.visible {
            return "?/Esc:close help  Up/Down:scroll";
        }
        if self.dsl_reference.visible {
            return "Shift-?/Esc:close reference  Up/Down:scroll";
        }
        if self.tutorial.active && self.tutorial.explanation_visible {
            return "Esc:dismiss  Ctrl-Right:next  Ctrl-Left:prev";
        }
        if self.diff_preview.visible {
            return "Enter:accept  Esc:reject  Up/Down:scroll";
        }
        match self.mode {
            AppMode::Edit => match self.focus {
                FocusPanel::Editor => "Ctrl+Enter:eval | Ctrl+;:command | Tab:focus | ?:help",
                _ => "Tab:focus  Esc:back to editor  Ctrl-R:compile  ?:help",
            },
            AppMode::Perform => "Space:play 1-9:section Shift+1-9:layer F1-F8:macro ?:help",
        }
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let theme = &self.theme;
        let compile_indicator = match &self.status.compile_status {
            CompileStatus::Ok => Span::styled(" OK ", Style::default().fg(theme.diff_add)),
            CompileStatus::Error(_) => {
                Span::styled(" ERR ", Style::default().fg(theme.diff_remove))
            }
            CompileStatus::Idle => {
                Span::styled(" -- ", Style::default().fg(theme.editor_line_number))
            }
        };

        let audio_device_indicator = match &self.audio_engine {
            Some(engine) => {
                let name = engine.device_name();
                let char_count = name.chars().count();
                let label = if char_count > 12 {
                    let mut t: String = name.chars().take(11).collect();
                    t.push('\u{2026}'); // ellipsis
                    t
                } else {
                    name.to_string()
                };
                Span::styled(format!(" {label} "), Style::default().fg(theme.diff_add))
            }
            None => Span::styled(" NO AUDIO ", Style::default().fg(theme.diff_remove)),
        };

        // Only show MIDI/OSC indicators when connected (save status bar space)
        let midi_indicator = if self.midi_input.is_some() {
            Span::styled(" MIDI", Style::default().fg(theme.diff_add))
        } else {
            Span::raw("")
        };
        let osc_indicator = if self.osc_listener.is_some() {
            Span::styled(" OSC", Style::default().fg(theme.diff_add))
        } else {
            Span::raw("")
        };

        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", self.status.playback_display()),
                Style::default()
                    .fg(if self.status.is_playing {
                        theme.diff_add
                    } else {
                        theme.diff_remove
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " BPM:{:.0} | {} | {} | Z:{} ",
                self.status.bpm,
                self.status.position_display(),
                self.status.mode_display(),
                self.grid_zoom.label(),
            )),
            compile_indicator,
            audio_device_indicator,
            midi_indicator,
            osc_indicator,
            Span::styled(
                format!(" {} ", self.context_hint()),
                Style::default().fg(theme.editor_line_number),
            ),
        ]);

        let paragraph =
            Paragraph::new(line).style(Style::default().bg(theme.status_bg).fg(theme.status_fg));
        frame.render_widget(paragraph, area);
    }

    /// Get a clone of the external input sender for MIDI/OSC threads.
    pub fn external_sender(&self) -> external_input::ExternalInputSender {
        self.external_tx.clone()
    }

    /// Process external events from MIDI/OSC/etc.
    fn process_external_events(&mut self) {
        let events = self.external_rx.drain();
        for event in events {
            match event {
                external_input::ExternalEvent::MacroSet { name, value } => {
                    self.macro_engine.set_macro(&name, value);
                    self.macro_panel.update(self.macro_engine.macros());
                    self.intent_console.log(
                        format!("ext: set {} = {value:.2}", name),
                        self.current_beat.as_beats_f64(),
                    );
                }
                external_input::ExternalEvent::SectionJump(idx) => {
                    self.handle_action(Action::JumpSection(idx));
                }
                external_input::ExternalEvent::LayerToggle(idx) => {
                    self.handle_action(Action::ToggleLayer(idx));
                }
                external_input::ExternalEvent::BpmSet(bpm) => {
                    self.status.bpm = bpm.clamp(20.0, 999.0);
                    self.intent_console.log(
                        format!("ext: BPM = {:.0}", self.status.bpm),
                        self.current_beat.as_beats_f64(),
                    );
                }
                external_input::ExternalEvent::PlayStop => {
                    self.handle_action(Action::TogglePlayback);
                }
                external_input::ExternalEvent::AiResponse {
                    input,
                    proposed_source,
                } => {
                    // AI suggested a DSL modification — show as structural intent
                    let current_source = self.editor.content();
                    if let (Ok(old_ast), Ok(new_ast)) = (
                        Compiler::parse(&current_source),
                        Compiler::parse(&proposed_source),
                    ) {
                        let diff = crate::dsl::diff::AstDiff::diff(&old_ast, &new_ast);
                        let diff_lines: Vec<diff_preview::DiffLine> = diff
                            .changes
                            .iter()
                            .map(|change| diff_preview::DiffLine {
                                text: format!("{change:?}"),
                                kind: diff_preview::DiffLineKind::Modification,
                            })
                            .collect();

                        if !diff_lines.is_empty() {
                            self.structural_intent_processor
                                .propose(input, diff, proposed_source);
                            self.diff_preview.show(diff_lines);
                            self.intent_console.log(
                                "AI proposed change — Enter/Esc",
                                self.current_beat.as_beats_f64(),
                            );
                        } else {
                            self.intent_console
                                .log("AI: no changes detected", self.current_beat.as_beats_f64());
                        }
                    } else {
                        self.intent_console.log(
                            "AI response failed to parse",
                            self.current_beat.as_beats_f64(),
                        );
                    }
                }
                external_input::ExternalEvent::NoteOn { .. }
                | external_input::ExternalEvent::NoteOff { .. }
                | external_input::ExternalEvent::CC { .. } => {
                    // Future: route to instrument engine
                }
            }
        }
    }

    /// Run the TUI event loop.
    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>,
    ) -> io::Result<()> {
        while !self.should_quit {
            terminal
                .draw(|frame| self.draw(frame))
                .map_err(|e| io::Error::other(e.to_string()))?;

            // Poll for input with a short timeout (5ms for responsive audio)
            if event::poll(Duration::from_millis(5))? {
                if let CrosstermEvent::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        let is_edit = self.mode == AppMode::Edit;
                        let diff_visible = self.diff_preview.visible;
                        let cmd_bar_active = self.command_bar.active;
                        let tutorial_active = self.tutorial.active;
                        if let Some(action) = keybindings::map_key_full(
                            key,
                            is_edit,
                            diff_visible,
                            self.focus,
                            cmd_bar_active,
                            tutorial_active,
                        ) {
                            self.handle_action(action);
                        }
                    }
                }
            }

            // Auto-recompile after debounce period
            if self.dirty {
                if let Some(last) = self.last_edit {
                    if last.elapsed() >= Duration::from_millis(COMPILE_DEBOUNCE_MS) {
                        self.compile_source();
                        self.dirty = false;
                        self.last_edit = None;
                    }
                }
            }

            // Process external input (MIDI, OSC, etc.)
            self.process_external_events();

            // Check for audio device changes (best-effort, every 2s)
            self.check_audio_device();

            // Advance beat when playing
            self.advance_beat();

            // Process pending intents
            let ready_intents = self.intent_processor.drain_ready(self.current_beat);
            for intent in ready_intents {
                match intent {
                    PerformanceIntent::SetMacro { name, value } => {
                        self.macro_engine.set_macro(&name, value);
                    }
                    PerformanceIntent::AdjustMacro { name, delta } => {
                        self.macro_engine.adjust_macro(&name, delta);
                    }
                    PerformanceIntent::ToggleLayer { name } => {
                        self.section_controller.toggle_layer(&name);
                    }
                    PerformanceIntent::JumpToSection { name } => {
                        self.section_controller
                            .schedule_transition(&name, self.current_beat);
                    }
                    PerformanceIntent::SetTempo(bpm) => {
                        self.status.bpm = bpm;
                    }
                }
                self.macro_panel.update(self.macro_engine.macros());
            }

            // Update section controller
            self.section_controller.update(self.current_beat);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_creation() {
        let app = App::new("tempo 128");
        assert_eq!(app.mode, AppMode::Edit);
        assert_eq!(app.focus, FocusPanel::Editor);
        assert!(!app.should_quit);
        assert!(!app.is_playing);
    }

    #[test]
    fn handle_quit() {
        let mut app = App::new("");
        app.handle_action(Action::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn handle_toggle_playback() {
        let mut app = App::new("");
        assert!(!app.is_playing);
        app.handle_action(Action::TogglePlayback);
        assert!(app.is_playing);
        app.handle_action(Action::TogglePlayback);
        assert!(!app.is_playing);
    }

    #[test]
    fn handle_toggle_mode() {
        let mut app = App::new("");
        assert_eq!(app.mode, AppMode::Edit);
        app.handle_action(Action::ToggleMode);
        assert_eq!(app.mode, AppMode::Perform);
        app.handle_action(Action::ToggleMode);
        assert_eq!(app.mode, AppMode::Edit);
    }

    #[test]
    fn handle_cycle_focus() {
        let mut app = App::new("");
        assert_eq!(app.focus, FocusPanel::Editor);
        app.handle_action(Action::CycleFocus);
        assert_eq!(app.focus, FocusPanel::Tracks);
    }

    #[test]
    fn handle_compile_valid() {
        let src = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        assert_eq!(app.status.compile_status, CompileStatus::Ok);
        assert!((app.status.bpm - 128.0).abs() < f64::EPSILON);
        assert_eq!(app.track_list.len(), 1);
    }

    #[test]
    fn handle_compile_error() {
        let mut app = App::new("invalid source {{{");
        app.handle_action(Action::CompileReload);
        assert!(matches!(app.status.compile_status, CompileStatus::Error(_)));
    }

    #[test]
    fn handle_editor_insert() {
        let mut app = App::new("");
        app.handle_action(Action::EditorInsert('a'));
        app.handle_action(Action::EditorInsert('b'));
        assert_eq!(app.editor.content(), "ab");
    }

    #[test]
    fn compile_updates_macros() {
        let src = "macro filter = 0.5\nmap filter -> cutoff (0.0..1.0) linear\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        assert_eq!(app.macro_panel.len(), 1);
    }

    #[test]
    fn diff_preview_accept_hides() {
        let mut app = App::new("");
        app.diff_preview.show(vec![diff_preview::DiffLine {
            text: "test".to_string(),
            kind: diff_preview::DiffLineKind::Context,
        }]);
        app.focus = FocusPanel::DiffPreview;
        assert!(app.diff_preview.visible);

        app.handle_action(Action::AcceptDiff);
        assert!(!app.diff_preview.visible);
        assert_eq!(app.focus, FocusPanel::Editor);
    }

    #[test]
    fn diff_preview_reject_hides() {
        let mut app = App::new("");
        app.diff_preview.show(vec![diff_preview::DiffLine {
            text: "test".to_string(),
            kind: diff_preview::DiffLineKind::Context,
        }]);
        app.handle_action(Action::RejectDiff);
        assert!(!app.diff_preview.visible);
    }

    #[test]
    fn diff_scroll_actions() {
        let mut app = App::new("");
        let lines: Vec<diff_preview::DiffLine> = (0..30)
            .map(|i| diff_preview::DiffLine {
                text: format!("line {i}"),
                kind: diff_preview::DiffLineKind::Context,
            })
            .collect();
        app.diff_preview.show(lines);

        app.handle_action(Action::DiffScrollDown);
        assert_eq!(app.diff_preview.scroll_offset, 1);
        app.handle_action(Action::DiffScrollUp);
        assert_eq!(app.diff_preview.scroll_offset, 0);
    }

    #[test]
    fn compile_populates_layers() {
        let src = "layer fx {\n  filter -> cutoff (0.0..1.0) linear\n}\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        assert_eq!(app.layer_panel.len(), 1);
        assert_eq!(app.layer_panel.entries[0].name, "fx");
        assert!(!app.layer_panel.entries[0].enabled);
    }

    #[test]
    fn toggle_layer_action() {
        let src = "layer fx {\n  filter -> cutoff (0.0..1.0) linear\n}\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        assert!(!app.layer_panel.entries[0].enabled);

        app.handle_action(Action::ToggleLayer(0));
        assert!(app.layer_panel.entries[0].enabled);

        app.handle_action(Action::ToggleLayer(0));
        assert!(!app.layer_panel.entries[0].enabled);
    }

    #[test]
    fn toggle_layer_out_of_range_no_panic() {
        let mut app = App::new("");
        // No layers — should not panic
        app.handle_action(Action::ToggleLayer(5));
    }

    // --- Focus routing tests ---

    #[test]
    fn focus_routing_editor_only_when_focused() {
        let mut app = App::new("");
        app.mode = AppMode::Edit;

        // Editor focused: insert works
        app.focus = FocusPanel::Editor;
        app.handle_action(Action::EditorInsert('x'));
        assert_eq!(app.editor.content(), "x");

        // Switch to Tracks: the keybinding mapper should not produce EditorInsert
        // (this tests the mapper, not handle_action directly)
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        let key = KeyEvent {
            code: KeyCode::Char('y'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let action = keybindings::map_key_with_diff(key, true, false, FocusPanel::Tracks);
        assert_eq!(action, None); // 'y' should not produce any action when Tracks focused
    }

    #[test]
    fn compile_populates_events_for_grid() {
        let src = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        assert!(!app.compiled_events.is_empty());
    }

    // --- Beat advancement tests ---

    #[test]
    fn beat_does_not_advance_when_stopped() {
        let mut app = App::new("");
        app.is_playing = false;
        app.advance_beat();
        assert_eq!(app.current_beat, Beat::ZERO);
    }

    #[test]
    fn beat_advances_when_playing() {
        let mut app = App::new("");
        app.is_playing = true;
        app.status.bpm = 120.0;

        // First call initializes last_tick
        app.advance_beat();
        let first_beat = app.current_beat;

        // Simulate time passing by setting last_tick in the past
        app.last_tick = Some(Instant::now() - Duration::from_millis(500));
        app.advance_beat();

        // After 500ms at 120BPM, should have advanced ~1 beat
        assert!(app.current_beat.ticks() > first_beat.ticks());
    }

    #[test]
    fn status_updates_during_playback() {
        let mut app = App::new("");
        app.is_playing = true;
        app.status.bpm = 120.0;

        // Simulate 2.5 seconds of playback at 120 BPM = 5 beats
        app.last_tick = Some(Instant::now() - Duration::from_millis(2500));
        app.advance_beat();

        assert!(app.status.position_bars > 0 || app.status.position_beats > 0);
    }

    // --- Help screen tests ---

    #[test]
    fn help_toggle_action() {
        let mut app = App::new("");
        assert!(!app.help_screen.visible);
        app.handle_action(Action::ToggleHelp);
        assert!(app.help_screen.visible);
        app.handle_action(Action::ToggleHelp);
        assert!(!app.help_screen.visible);
    }

    #[test]
    fn escape_closes_help() {
        let mut app = App::new("");
        app.help_screen.show();
        assert!(app.help_screen.visible);
        app.handle_action(Action::Escape);
        assert!(!app.help_screen.visible);
    }

    #[test]
    fn escape_returns_focus_to_editor() {
        let mut app = App::new("");
        app.focus = FocusPanel::Tracks;
        app.handle_action(Action::Escape);
        assert_eq!(app.focus, FocusPanel::Editor);
    }

    #[test]
    fn context_hint_changes_by_mode() {
        let mut app = App::new("");
        app.mode = AppMode::Edit;
        app.focus = FocusPanel::Editor;
        assert!(app.context_hint().contains("Ctrl+Enter"));

        app.mode = AppMode::Perform;
        assert!(app.context_hint().contains("Space:play"));
    }

    #[test]
    fn context_hint_changes_by_focus() {
        let mut app = App::new("");
        app.mode = AppMode::Edit;
        app.focus = FocusPanel::Tracks;
        assert!(app.context_hint().contains("Esc:back to editor"));
    }

    // --- Stability hardening tests ---

    #[test]
    fn crash_log_toggle_action() {
        let mut app = App::new("");
        assert!(!app.crash_log_visible);
        app.handle_action(Action::ToggleCrashLog);
        assert!(app.crash_log_visible);
        app.handle_action(Action::ToggleCrashLog);
        assert!(!app.crash_log_visible);
    }

    #[test]
    fn escape_closes_crash_log() {
        let mut app = App::new("");
        app.crash_log_visible = true;
        app.handle_action(Action::Escape);
        assert!(!app.crash_log_visible);
    }

    #[test]
    fn escape_closes_crash_log_before_help() {
        let mut app = App::new("");
        app.crash_log_visible = true;
        app.help_screen.show();
        app.handle_action(Action::Escape);
        // Crash log should close first
        assert!(!app.crash_log_visible);
        assert!(app.help_screen.visible);
    }

    #[test]
    fn compile_error_does_not_crash() {
        let mut app = App::new("invalid source {{{");
        app.handle_action(Action::CompileReload);
        assert!(matches!(app.status.compile_status, CompileStatus::Error(_)));
        // App should still be functional
        assert!(!app.should_quit);
    }

    #[test]
    fn bpm_clamped_low() {
        let src = "tempo 5\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        assert!(app.status.bpm >= 20.0);
    }

    #[test]
    fn bpm_clamped_high() {
        let src = "tempo 10000\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        assert!(app.status.bpm <= 999.0);
    }

    #[test]
    fn context_hint_crash_log_visible() {
        let mut app = App::new("");
        app.crash_log_visible = true;
        assert!(app.context_hint().contains("crash log"));
    }

    // --- External input tests ---

    #[test]
    fn external_sender_clone_works() {
        let app = App::new("");
        let _tx = app.external_sender();
    }

    #[test]
    fn external_macro_set_updates_engine() {
        let src = "macro filter = 0.5\nmap filter -> cutoff (0.0..1.0) linear\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);

        let tx = app.external_sender();
        tx.send(external_input::ExternalEvent::MacroSet {
            name: "filter".to_string(),
            value: 0.9,
        })
        .unwrap();
        app.process_external_events();

        let val = app.macro_engine.get_macro("filter").unwrap();
        assert!((val - 0.9).abs() < f64::EPSILON);
    }

    // --- Grid zoom tests ---

    #[test]
    fn grid_zoom_in_out() {
        let mut app = App::new("");
        assert_eq!(app.grid_zoom, GridZoom::Beat);
        app.handle_action(Action::GridZoomOut);
        assert_eq!(app.grid_zoom, GridZoom::HalfBar);
        app.handle_action(Action::GridZoomIn);
        assert_eq!(app.grid_zoom, GridZoom::Beat);
    }

    // --- Auto-recompile tests ---

    #[test]
    fn dirty_flag_set_on_editor_insert() {
        let mut app = App::new("");
        assert!(!app.dirty);
        app.handle_action(Action::EditorInsert('x'));
        assert!(app.dirty);
        assert!(app.last_edit.is_some());
    }

    #[test]
    fn dirty_flag_set_on_editor_backspace() {
        let mut app = App::new("ab");
        app.editor.move_right();
        app.handle_action(Action::EditorBackspace);
        assert!(app.dirty);
    }

    #[test]
    fn dirty_flag_set_on_editor_newline() {
        let mut app = App::new("hello");
        app.handle_action(Action::EditorNewline);
        assert!(app.dirty);
    }

    #[test]
    fn app_has_theme() {
        let app = App::new("");
        assert!(!app.theme.name.is_empty());
        assert!(!app.available_themes.is_empty());
    }

    #[test]
    fn cycle_theme_action() {
        let mut app = App::new("");
        let first_name = app.theme.name.clone();
        app.handle_action(Action::CycleTheme);
        assert_ne!(app.theme.name, first_name);
    }

    #[test]
    fn dirty_flag_set_on_editor_delete() {
        let mut app = App::new("ab");
        app.handle_action(Action::EditorDelete);
        assert!(app.dirty);
    }

    #[test]
    fn manual_compile_still_works() {
        let src = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        assert_eq!(app.status.compile_status, CompileStatus::Ok);
    }

    // --- REPL eval tests ---

    #[test]
    fn eval_immediate_compiles_and_autoplays() {
        let src = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        assert!(!app.is_playing);
        app.handle_action(Action::EvalImmediate);
        assert_eq!(app.status.compile_status, CompileStatus::Ok);
        assert!(app.is_playing); // Auto-started
    }

    #[test]
    fn eval_immediate_error_does_not_autoplay() {
        let mut app = App::new("invalid {{{");
        app.handle_action(Action::EvalImmediate);
        assert!(matches!(app.status.compile_status, CompileStatus::Error(_)));
        assert!(!app.is_playing);
    }

    // --- Command bar tests ---

    #[test]
    fn command_bar_activate_deactivate() {
        let mut app = App::new("");
        assert!(!app.command_bar.active);
        app.handle_action(Action::ActivateCommandBar);
        assert!(app.command_bar.active);
        app.handle_action(Action::CommandBarCancel);
        assert!(!app.command_bar.active);
    }

    #[test]
    fn command_bar_insert_and_submit() {
        let mut app = App::new("");
        app.handle_action(Action::ActivateCommandBar);
        app.handle_action(Action::CommandBarInsert(':'));
        app.handle_action(Action::CommandBarInsert('h'));
        app.handle_action(Action::CommandBarInsert('e'));
        app.handle_action(Action::CommandBarInsert('l'));
        app.handle_action(Action::CommandBarInsert('p'));
        app.handle_action(Action::CommandBarSubmit);
        assert!(!app.command_bar.active);
        // :help toggles help screen
        assert!(app.help_screen.visible);
    }

    #[test]
    fn command_preset_loads() {
        let mut app = App::new("");
        app.process_command(":preset techno");
        assert!(app.editor.content().contains("tempo 130"));
    }

    #[test]
    fn command_clear_clears_editor() {
        let mut app = App::new("tempo 120");
        app.process_command(":clear");
        assert!(app.editor.content().is_empty());
    }

    #[test]
    fn command_eval_compiles() {
        let src = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.process_command(":eval");
        assert_eq!(app.status.compile_status, CompileStatus::Ok);
    }

    #[test]
    fn nl_command_faster() {
        let mut app = App::new("");
        app.status.bpm = 120.0;
        app.process_command("faster");
        assert!((app.status.bpm - 130.0).abs() < f64::EPSILON);
    }

    #[test]
    fn nl_command_more_reverb() {
        let src = "macro space = 0.3\nmap space -> reverb_mix (0.0..0.5) linear\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let mut app = App::new(src);
        app.handle_action(Action::CompileReload);
        let before = app.macro_engine.get_macro("space").unwrap();
        app.process_command("more reverb");
        let after = app.macro_engine.get_macro("space").unwrap();
        assert!(after > before);
    }

    // --- Tutorial tests ---

    #[test]
    fn tutorial_start_via_command() {
        let mut app = App::new("");
        app.process_command(":tutorial");
        assert!(app.tutorial.active);
        assert!(app.tutorial.explanation_visible);
        assert!(!app.editor.content().is_empty());
    }

    #[test]
    fn tutorial_next_prev() {
        let mut app = App::new("");
        app.process_command(":tutorial");
        let first_code = app.editor.content();
        app.handle_action(Action::TutorialNext);
        let second_code = app.editor.content();
        assert_ne!(first_code, second_code);
        app.handle_action(Action::TutorialPrev);
        assert_eq!(app.editor.content(), first_code);
    }

    // --- DSL reference tests ---

    #[test]
    fn dsl_reference_toggle() {
        let mut app = App::new("");
        assert!(!app.dsl_reference.visible);
        app.handle_action(Action::ToggleDslReference);
        assert!(app.dsl_reference.visible);
        app.handle_action(Action::ToggleDslReference);
        assert!(!app.dsl_reference.visible);
    }

    #[test]
    fn escape_closes_dsl_reference() {
        let mut app = App::new("");
        app.dsl_reference.show();
        app.handle_action(Action::Escape);
        assert!(!app.dsl_reference.visible);
    }

    // --- Context hint for new features ---

    #[test]
    fn context_hint_command_bar() {
        let mut app = App::new("");
        app.command_bar.activate();
        assert!(app.context_hint().contains("command"));
    }

    #[test]
    fn context_hint_dsl_reference() {
        let mut app = App::new("");
        app.dsl_reference.show();
        assert!(app.context_hint().contains("reference"));
    }

    // --- Audio reconnection tests ---

    #[test]
    fn handle_reconnect_audio() {
        let mut app = App::new("");
        // Should not panic regardless of audio device availability
        app.handle_action(Action::ReconnectAudio);
        // Intent console should have a log entry about the reconnection
        assert!(app
            .intent_console
            .entries()
            .iter()
            .any(|e| e.message.contains("audio:")));
    }

    #[test]
    fn command_bar_audio_reconnect() {
        let mut app = App::new("");
        app.process_command(":audio");
        assert!(app
            .intent_console
            .entries()
            .iter()
            .any(|e| e.message.contains("audio:")));
    }

    #[test]
    fn check_audio_device_debounce() {
        let mut app = App::new("");
        // First call should set last_device_check
        app.check_audio_device();
        assert!(app.last_device_check.is_some());

        // Second immediate call should be debounced (no reconnection)
        let check_time = app.last_device_check;
        app.check_audio_device();
        assert_eq!(app.last_device_check, check_time);
    }
}
