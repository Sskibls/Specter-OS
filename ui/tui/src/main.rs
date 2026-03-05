// PhantomKernel OS - Terminal User Interface
// Secure terminal dashboard for managing PhantomKernel OS

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
    Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::Duration;

mod app;
mod ui;

use app::{App, AppResult};

fn main() -> AppResult<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle result
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            Err(e)
        }
    }
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> AppResult<()> {
    loop {
        // Draw UI
        terminal.draw(|frame| ui::render(frame, app))?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('p') => app.activate_panic(),
                        KeyCode::Char('m') => app.toggle_mask_mode(),
                        KeyCode::Char('t') => app.toggle_travel_mode(),
                        KeyCode::Char('k') => app.toggle_kill_switch(),
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.previous_tab(),
                        KeyCode::Char('1') => app.set_tab(0),
                        KeyCode::Char('2') => app.set_tab(1),
                        KeyCode::Char('3') => app.set_tab(2),
                        KeyCode::Char('4') => app.set_tab(3),
                        KeyCode::Char('r') => app.refresh(),
                        _ => {}
                    }
                }
            }
        }

        // Auto-refresh
        app.tick();
    }
}
