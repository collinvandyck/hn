use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{Event as CrosstermEvent, EventStream},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::time::interval;
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

pub struct EventHandler {
    event_stream: EventStream,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            event_stream: EventStream::new(),
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    pub async fn next(&mut self) -> Result<Event> {
        let mut tick_interval = interval(self.tick_rate);

        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    return Ok(Event::Tick);
                }
                event = self.event_stream.next() => {
                    if let Some(Ok(event)) = event {
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
    }
}
