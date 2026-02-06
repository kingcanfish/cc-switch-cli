use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum TuiEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
}
