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
use tokio::time::{Interval, interval};
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
    tick_interval: Interval,
}

impl EventHandler {
    pub fn with_tick_every(every: Duration) -> Self {
        Self {
            event_stream: EventStream::new(),
            tick_interval: interval(every),
        }
    }

    pub async fn next(&mut self) -> Result<Event> {
        loop {
            tokio::select! {
                _ = self.tick_interval.tick() => {
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
