use crossterm::event::KeyEvent;

/// Application events
#[derive(Debug, Clone)]
pub enum Event {
    /// Terminal tick for periodic updates
    Tick,
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input (ignored)
    Mouse,
    /// Terminal resize (ignored)
    Resize,
}
