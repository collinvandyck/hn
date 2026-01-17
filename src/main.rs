mod api;
mod app;
mod event;
mod keys;
mod tui;
mod views;

use anyhow::Result;
use ratatui::Frame;

use app::{App, View};
use event::Event;
use tui::EventHandler;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize terminal
    let mut terminal = tui::init()?;

    // Create app and event handler
    let mut app = App::new();
    let mut events = EventHandler::new(250); // 250ms tick rate

    // Load initial stories
    app.load_stories().await;

    // Main loop
    loop {
        // Render
        terminal.draw(|frame| render(&app, frame))?;

        // Handle events
        match events.next().await? {
            Event::Key(key) => {
                if let Some(msg) = keys::handle_key(key, &app) {
                    app.update(msg).await;
                }
            }
            Event::Tick => {
                // Could update timers or check for new data here
            }
            Event::Resize => {
                // Terminal handles resize automatically
            }
            Event::Mouse => {
                // Mouse support can be added later
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    tui::restore()?;

    Ok(())
}

fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();

    match &app.view {
        View::Stories => views::stories::render(frame, app, area),
        View::Comments { .. } => views::comments::render(frame, app, area),
    }
}
