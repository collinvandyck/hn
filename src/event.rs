use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Resize,
}
