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
use resonance::dsl::Compiler;
use resonance::event::EventScheduler;
use resonance::instrument::{build_default_kit, InstrumentRouter};
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
    // Determine initial source
    let initial_source = if first_run::is_first_run() {
        // Create config directory on first run
        if let Err(e) = first_run::create_config_dir() {
            eprintln!("warning: could not create config dir: {e}");
        }
        first_run::default_starter()
    } else {
        first_run::default_starter()
    };

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

fn headless_play(file: &PathBuf, duration: Option<f64>) -> io::Result<()> {
    let source = std::fs::read_to_string(file)?;

    let song = Compiler::compile(&source).map_err(|e| io::Error::other(e.to_string()))?;

    let mut engine =
        AudioEngine::new().map_err(|e| io::Error::other(format!("audio init failed: {e}")))?;

    let sample_rate = engine.sample_rate();
    let channels = engine.channels();
    let seed = 42u64;
    let bpm = song.tempo.clamp(20.0, 999.0);

    let bank = build_default_kit(sample_rate, seed);
    let router = InstrumentRouter::from_track_defs(&song.track_defs, bank, seed);
    let mut render_fn = router.into_render_fn();

    let mut scheduler = EventScheduler::new(bpm, sample_rate, channels, 1024, seed);
    scheduler.timeline_mut().insert_batch(song.events);
    scheduler.play();

    let _ = engine.play();

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        stop_clone.store(true, Ordering::SeqCst);
    })
    .map_err(|e| io::Error::other(format!("failed to set Ctrl-C handler: {e}")))?;

    eprintln!(
        "Playing {} at {:.0} BPM... (Ctrl-C to stop)",
        file.display(),
        bpm
    );

    let start = Instant::now();
    let timeout = duration.map(Duration::from_secs_f64);

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        if let Some(t) = timeout {
            if start.elapsed() >= t {
                break;
            }
        }

        if let Some(samples) = scheduler.render_block(&mut render_fn) {
            if engine.send_samples(samples).is_err() {
                std::thread::sleep(Duration::from_millis(1));
            }
        } else {
            break;
        }
    }

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
