use ratatui::prelude::Frame;

use super::event::TuiEvent;

pub enum ScreenResult<T> {
    Continue,
    Exit(T),
}

pub trait Screen {
    type Output;

    fn draw(&mut self, frame: &mut Frame);
    fn on_event(&mut self, event: TuiEvent) -> ScreenResult<Self::Output>;
}
