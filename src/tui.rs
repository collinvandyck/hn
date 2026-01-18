use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::{
    event::{Event as CrosstermEvent, EventStream},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use tracing::debug;

use crate::event::Event;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    debug!("entering alternate screen");
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    debug!("leaving alternate screen");
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

pub struct CrosstermEvents {
    event_stream: EventStream,
}

impl CrosstermEvents {
    pub fn new() -> Self {
        Self {
            event_stream: EventStream::new(),
        }
    }

    pub async fn next(&mut self) -> Result<Event> {
        loop {
            if let Some(Ok(event)) = self.event_stream.next().await {
                match event {
                    CrosstermEvent::Key(key) => {
                        if key.kind == crossterm::event::KeyEventKind::Press {
                            return Ok(Event::Key(key));
                        }
                    }
                    CrosstermEvent::Resize(_, _) => {
                        return Ok(Event::Resize);
                    }
                    _ => {}
                }
            }
        }
    }
}
