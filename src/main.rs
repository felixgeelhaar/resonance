//! Resonance — terminal-native live coding music instrument.
//!
//! Launches the TUI interface for writing DSL code, compiling patterns,
//! and performing live with macros and section transitions.

use std::io;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use resonance::tui::first_run;
use resonance::tui::App;

fn main() -> io::Result<()> {
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
