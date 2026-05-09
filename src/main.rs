use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

mod ai;
mod api;
mod app;
mod config;
mod event;
mod theme;
mod ui;

use app::{App, AppResult};
use event::{Event, EventHandler};
use ui::render;

#[tokio::main]
async fn main() -> AppResult<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    io::stdout().execute(EnableMouseCapture)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new();

    // Auto-search if player name/id provided as argument
    if let Some(query) = std::env::args().nth(1) {
        if !query.is_empty() {
            app.input = query;
            app.fetch_stats().await?;
            app.input_mode = app::InputMode::Normal;
        }
    }

    let mut event_handler = EventHandler::new(250);

    let result = run_app(&mut terminal, &mut app, &mut event_handler).await;

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    io::stdout().execute(DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &mut EventHandler,
) -> AppResult<()>
where
    B::Error: Send + Sync + 'static,
{
    loop {
        terminal.draw(|f| render(f, app))?;

        match events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => {
                if app.handle_key_event(key_event).await? {
                    return Ok(());
                }
            }
            Event::Mouse(mouse_event) => {
                app.handle_mouse_event(mouse_event);
            }
            Event::Resize(_, _) => {
                app.input_rect = None;
                app.player_card_rect = None;
                app.last_content_rect = None;
                let _ = terminal.clear();
            }
        }
    }
}
