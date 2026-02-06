use crossterm::event::{KeyCode, KeyEvent};

pub fn is_up(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Up)
}

pub fn is_down(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Down)
}

pub fn is_left(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Left)
}

pub fn is_right(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Right)
}

pub fn is_enter(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Enter)
}

pub fn is_escape(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc)
}

pub fn is_backspace(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Backspace)
}

pub fn char_from_key(key: &KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(ch) => Some(ch),
        _ => None,
    }
}
