//! Resonance — terminal-native live coding music instrument.
//!
//! Launches the TUI interface for writing DSL code, compiling patterns,
//! and performing live with macros and section transitions.
//!
//! Also supports headless playback via the `play` subcommand:
//!   resonance play file.dsl [--duration 10]

use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use resonance::audio::AudioEngine;
use resonance::content::presets;
use resonance::dsl::Compiler;
use resonance::event::types::ParamId;
use resonance::event::EventScheduler;
use resonance::instrument::InstrumentRouter;
use resonance::macro_engine::MacroEngine;
use resonance::tui::first_run;
use resonance::tui::App;

#[derive(Parser)]
#[command(
    name = "resonance",
    about = "Terminal-native live coding music instrument"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Play a .dsl file headlessly (no TUI)
    Play {
        /// Path to a .dsl source file
        file: PathBuf,
        /// Stop after this many seconds (omit for indefinite playback)
        #[arg(short, long)]
        duration: Option<f64>,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Play { file, duration }) => headless_play(&file, duration),
        None => run_tui(),
    }
}

fn run_tui() -> io::Result<()> {
    // Create config directory on first run
    let is_first = first_run::is_first_run();
    if is_first {
        if let Err(e) = first_run::create_config_dir() {
            eprintln!("warning: could not create config dir: {e}");
        }
    }

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // On first run, show genre selection wizard
    let initial_source = if is_first {
        let selected = run_first_run_wizard(&mut terminal)?;
        presets::load_preset(&selected).unwrap_or_else(presets::default_preset)
    } else {
        presets::default_preset()
    };

    // Run the app
    let mut app = App::new(&initial_source);
    let result = app.run(&mut terminal);

    // Terminal restore
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Show a simple genre-selection wizard on first run.
/// Returns the selected genre name.
fn run_first_run_wizard(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<String> {
    use crossterm::event::{self as ev, Event as EvEvent, KeyCode, KeyEventKind};
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let genres = [
        ("house", "Classic 4/4 — 124 BPM, offbeat hats, bass groove"),
        ("techno", "Driving — 130 BPM, minimal, industrial feel"),
        ("ambient", "Textural — 85 BPM, pads, heavy reverb"),
        ("dnb", "Fast breaks — 170 BPM, syncopated, driving bass"),
        ("empty", "Empty canvas — just tempo, start from scratch"),
    ];
    let mut selected: usize = 0;

    loop {
        terminal
            .draw(|frame| {
                let size = frame.area();

                // Center the wizard
                let width = 60u16.min(size.width.saturating_sub(4));
                let height = (genres.len() as u16 + 8).min(size.height.saturating_sub(4));
                let x = (size.width.saturating_sub(width)) / 2;
                let y = (size.height.saturating_sub(height)) / 2;
                let area = Rect::new(x, y, width, height);

                frame.render_widget(Clear, area);
                let block = Block::default()
                    .style(Style::default().bg(Color::Black))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" Welcome to Resonance ");
                let inner = block.inner(area);
                frame.render_widget(block, area);

                let mut lines = vec![
                    Line::from(Span::styled(
                        "What vibe are you going for?",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                ];

                for (i, (name, desc)) in genres.iter().enumerate() {
                    let marker = if i == selected { "> " } else { "  " };
                    let style = if i == selected {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("{marker}{name:<10} {desc}"),
                        style,
                    )));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Up/Down to select, Enter to confirm, Esc to skip",
                    Style::default().fg(Color::DarkGray),
                )));

                let paragraph = Paragraph::new(lines);
                frame.render_widget(paragraph, inner);
            })
            .map_err(|e| io::Error::other(e.to_string()))?;

        if ev::poll(Duration::from_millis(100))? {
            if let EvEvent::Key(key) = ev::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Up => {
                            selected = selected.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            if selected + 1 < genres.len() {
                                selected += 1;
                            }
                        }
                        KeyCode::Enter => {
                            return Ok(genres[selected].0.to_string());
                        }
                        KeyCode::Esc => {
                            return Ok("house".to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn headless_play(file: &PathBuf, duration: Option<f64>) -> io::Result<()> {
    use resonance::event::Beat;

    let source = std::fs::read_to_string(file)?;
    let song = Compiler::compile(&source).map_err(|e| io::Error::other(e.to_string()))?;

    let mut engine =
        AudioEngine::new().map_err(|e| io::Error::other(format!("audio init failed: {e}")))?;

    let sample_rate = engine.sample_rate();
    let channels = engine.channels();
    let seed = 42u64;
    let bpm = song.tempo.clamp(20.0, 999.0);

    // Compute song loop length from sections (round up to bar boundary)
    let song_length_bars: u32 = song
        .sections
        .iter()
        .map(|s| s.length_in_bars)
        .sum::<u32>()
        .max(1);
    let loop_length = Beat::from_bars(song_length_bars);

    let macro_engine = MacroEngine::from_compiled(&song.macros, &song.mappings);

    let plugin_registry = resonance::plugin::registry::PluginRegistry::scan_default();
    let router = InstrumentRouter::from_track_defs_with_kits(
        &song.track_defs,
        sample_rate,
        seed,
        &plugin_registry,
    );
    let mut render_fn = router.into_render_fn();

    let block_size: u32 = 1024;
    let mut scheduler = EventScheduler::new(bpm, sample_rate, channels, block_size, seed);
    scheduler.timeline_mut().insert_batch(song.events);
    scheduler.play();

    // Send initial FX params from macro defaults
    let resolved = macro_engine.resolve_params();
    for param_name in &["reverb_mix", "reverb_decay", "delay_mix", "delay_feedback"] {
        let key = ParamId(param_name.to_string());
        if let Some(&val) = resolved.get(&key) {
            let _ = engine.send_effect_param(param_name.to_string(), val);
        }
    }

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        stop_clone.store(true, Ordering::SeqCst);
    })
    .map_err(|e| io::Error::other(format!("failed to set Ctrl-C handler: {e}")))?;

    eprintln!(
        "Playing {} at {:.0} BPM ({} bar loop)... Ctrl-C to stop",
        file.display(),
        bpm,
        song_length_bars
    );
    let start = Instant::now();
    let timeout = duration.map(Duration::from_secs_f64);
    let block_duration = Duration::from_secs_f64(block_size as f64 / sample_rate as f64);
    let mut blocks_sent = 0u64;

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        if let Some(t) = timeout {
            if start.elapsed() >= t {
                break;
            }
        }

        // Loop: reset scheduler when past the song boundary
        if scheduler.transport().position() >= loop_length {
            scheduler.reset();
            scheduler.play();
        }

        if let Some(samples) = scheduler.render_block(&mut render_fn) {
            match engine.send_samples(samples) {
                Ok(()) => blocks_sent += 1,
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }

        // Pace to real-time
        let expected = Duration::from_secs_f64(blocks_sent as f64 * block_duration.as_secs_f64());
        let elapsed = start.elapsed();
        if elapsed < expected {
            std::thread::sleep(expected - elapsed);
        }
    }

    // Let buffered audio drain before shutting down
    std::thread::sleep(Duration::from_millis(500));
    let _ = engine.stop();
    eprintln!("Stopped.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parse_no_args() {
        let cli = Cli::try_parse_from(["resonance"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_parse_play_subcommand() {
        let cli = Cli::try_parse_from(["resonance", "play", "test.dsl"]).unwrap();
        match cli.command {
            Some(Commands::Play { file, duration }) => {
                assert_eq!(file, PathBuf::from("test.dsl"));
                assert!(duration.is_none());
            }
            _ => panic!("expected Play command"),
        }
    }

    #[test]
    fn cli_parse_play_with_duration() {
        let cli =
            Cli::try_parse_from(["resonance", "play", "test.dsl", "--duration", "5.0"]).unwrap();
        match cli.command {
            Some(Commands::Play { file, duration }) => {
                assert_eq!(file, PathBuf::from("test.dsl"));
                assert!((duration.unwrap() - 5.0).abs() < f64::EPSILON);
            }
            _ => panic!("expected Play command"),
        }
    }

    #[test]
    fn headless_compile_only() {
        // Test compilation without audio device
        let source = "tempo 128\ntrack drums {\n  kit: default\n  section main [1 bars] {\n    kick: [X . . .]\n  }\n}";
        let song = Compiler::compile(source).unwrap();
        assert!((song.tempo - 128.0).abs() < f64::EPSILON);
        assert!(!song.events.is_empty());
    }
}
