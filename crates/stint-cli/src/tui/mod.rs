//! Interactive TUI dashboard for Stint.

mod app;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use stint_core::storage::sqlite::SqliteStorage;

use self::app::App;

/// Runs the interactive dashboard.
///
/// Opens the database, enters the alternate screen, and runs the event loop
/// until the user quits. Restores terminal state on exit (including panics).
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = SqliteStorage::default_path();
    let storage = SqliteStorage::open(&path)?;

    // Set up terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Install panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    let mut app = App::new(storage);

    // Event loop
    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Poll for events with 1-second timeout (drives the live timer tick)
        if event::poll(Duration::from_secs(1))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => app.should_quit = true,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.should_quit = true,
                    (KeyCode::Tab, _) => {
                        app.selected_panel = app.selected_panel.next();
                    }
                    (KeyCode::BackTab, _) => {
                        app.selected_panel = app.selected_panel.next();
                    }
                    (KeyCode::Up, _) => app.scroll_up(),
                    (KeyCode::Down, _) => app.scroll_down(),
                    _ => {}
                }
            }
        } else {
            // Timeout — refresh data
            app.refresh();
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
