//! TUI interface — ratatui panels: editor, tracks, grid, macros, intent console.
//!
//! The App struct holds all TUI state and drives the event loop.

pub mod editor;
pub mod first_run;
pub mod grid;
pub mod intent_console;
pub mod keybindings;
pub mod layout;
pub mod macros;
pub mod status;
pub mod tracks;

pub use editor::Editor;
pub use grid::{project_events, GridCell, TrackGrid};
pub use intent_console::IntentConsole;
pub use keybindings::{map_key, Action};
pub use layout::{AppMode, FocusPanel};
pub use macros::MacroPanel;
pub use status::{CompileStatus, StatusInfo};
pub use tracks::TrackList;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::dsl::Compiler;
use crate::event::Beat;
use crate::intent::{IntentProcessor, PerformanceIntent};
use crate::macro_engine::MacroEngine;
use crate::section::SectionController;

/// The main TUI application state.
pub struct App {
    pub editor: Editor,
    pub mode: AppMode,
    pub focus: FocusPanel,
    pub track_list: TrackList,
    pub macro_panel: MacroPanel,
    pub intent_console: IntentConsole,
    pub status: StatusInfo,
    pub macro_engine: MacroEngine,
    pub intent_processor: IntentProcessor,
    pub section_controller: SectionController,
    pub should_quit: bool,
    pub is_playing: bool,
    current_beat: Beat,
}

impl App {
    /// Create a new App with initial DSL source.
    pub fn new(source: &str) -> Self {
        Self {
            editor: Editor::new(source),
            mode: AppMode::Edit,
            focus: FocusPanel::Editor,
            track_list: TrackList::default(),
            macro_panel: MacroPanel::default(),
            intent_console: IntentConsole::new(50),
            status: StatusInfo::default(),
            macro_engine: MacroEngine::new(),
            intent_processor: IntentProcessor::new(1),
            section_controller: SectionController::default(),
            should_quit: false,
            is_playing: false,
            current_beat: Beat::ZERO,
        }
    }

    /// Process an action.
    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::TogglePlayback => {
                self.is_playing = !self.is_playing;
                self.status.is_playing = self.is_playing;
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
            Action::AdjustMacro(idx, delta) => {
                let names: Vec<String> = self.macro_engine.macros().keys().cloned().collect();
                if let Some(name) = names.get(idx) {
                    self.macro_engine.adjust_macro(name, delta);
                    self.macro_panel.update(self.macro_engine.macros());
                    self.intent_console.log(
                        format!("adjust {} +{:.2}", name, delta),
                        self.current_beat.as_beats_f64(),
                    );
                }
            }
            Action::EditorInsert(c) => self.editor.insert_char(c),
            Action::EditorBackspace => self.editor.backspace(),
            Action::EditorDelete => self.editor.delete(),
            Action::EditorLeft => self.editor.move_left(),
            Action::EditorRight => self.editor.move_right(),
            Action::EditorUp => self.editor.move_up(),
            Action::EditorDown => self.editor.move_down(),
            Action::EditorNewline => self.editor.newline(),
            Action::EditorHome => self.editor.home(),
            Action::EditorEnd => self.editor.end(),
        }
    }

    /// Compile the editor content and update state.
    fn compile_source(&mut self) {
        let source = self.editor.content();
        match Compiler::compile(&source) {
            Ok(song) => {
                self.status.bpm = song.tempo;
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

    /// Draw the UI.
    pub fn draw(&self, frame: &mut Frame) {
        let size = frame.area();

        // Main vertical layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Editor + Tracks
                Constraint::Percentage(30), // Grid
                Constraint::Percentage(20), // Macros + Intent Console
                Constraint::Length(1),      // Status bar
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

        // Status bar
        self.draw_status(frame, chunks[3]);
    }

    fn draw_editor(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::Editor;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let lines: Vec<Line> = self
            .editor
            .lines()
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let num = Span::styled(
                    format!("{:3} ", i + 1),
                    Style::default().fg(Color::DarkGray),
                );
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
            // +1 for border, +4 for line number
            let x = area.x + 1 + 4 + col as u16;
            let y = area.y + 1 + row as u16;
            if x < area.x + area.width && y < area.y + area.height {
                frame.set_cursor_position((x, y));
            }
        }
    }

    fn draw_tracks(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::Tracks;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
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
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .title(" Grid ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph = Paragraph::new("(grid visualization)").block(block);
        frame.render_widget(paragraph, area);
    }

    fn draw_macros(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::Macros;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
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
                    .gauge_style(Style::default().fg(Color::Green));
                frame.render_widget(gauge, gauge_area);
            }
        }
    }

    fn draw_intent_console(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusPanel::IntentConsole;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
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

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let compile_indicator = match &self.status.compile_status {
            CompileStatus::Ok => Span::styled(" OK ", Style::default().fg(Color::Green)),
            CompileStatus::Error(_) => Span::styled(" ERR ", Style::default().fg(Color::Red)),
            CompileStatus::Idle => Span::styled(" -- ", Style::default().fg(Color::DarkGray)),
        };

        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", self.status.playback_display()),
                Style::default()
                    .fg(if self.status.is_playing {
                        Color::Green
                    } else {
                        Color::Red
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " BPM:{:.0} | {} | {} ",
                self.status.bpm,
                self.status.position_display(),
                self.status.mode_display(),
            )),
            compile_indicator,
            Span::styled(
                " Ctrl-Q:quit Tab:focus Ctrl-R:compile Ctrl-P:mode Space:play ",
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        let paragraph =
            Paragraph::new(line).style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_widget(paragraph, area);
    }

    /// Run the TUI event loop.
    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>,
    ) -> io::Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;

            // Poll for input with a short timeout (5ms for responsive audio)
            if event::poll(Duration::from_millis(5))? {
                if let CrosstermEvent::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        let is_edit = self.mode == AppMode::Edit;
                        if let Some(action) = map_key(key, is_edit) {
                            self.handle_action(action);
                        }
                    }
                }
            }

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
}
