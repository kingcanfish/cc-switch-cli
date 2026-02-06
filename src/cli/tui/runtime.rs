use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::error::AppError;

use super::event::TuiEvent;
use super::screen::{Screen, ScreenResult};

static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_tui_active(active: bool) {
    TUI_ACTIVE.store(active, Ordering::Relaxed);
}

pub fn is_tui_active() -> bool {
    TUI_ACTIVE.load(Ordering::Relaxed)
}

pub struct TerminalGuard {
    restore: Option<Box<dyn FnOnce()>>,
}

impl TerminalGuard {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(err) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(err);
        }

        let restore = || {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, crossterm::cursor::Show, LeaveAlternateScreen);
            let _ = disable_raw_mode();
        };

        Ok(Self {
            restore: Some(Box::new(restore)),
        })
    }

    #[cfg(test)]
    pub fn new_with_restore<F>(restore: F) -> Self
    where
        F: FnOnce() + 'static,
    {
        Self {
            restore: Some(Box::new(restore)),
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if let Some(restore) = self.restore.take() {
            restore();
        }
    }
}

pub struct TuiRuntime {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    _guard: TerminalGuard,
    tick_rate: Duration,
}

impl TuiRuntime {
    pub fn new() -> Result<Self, AppError> {
        let guard = TerminalGuard::new().map_err(|err| AppError::Message(err.to_string()))?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal =
            Terminal::new(backend).map_err(|err| AppError::Message(err.to_string()))?;
        terminal
            .hide_cursor()
            .map_err(|err| AppError::Message(err.to_string()))?;

        Ok(Self {
            terminal,
            _guard: guard,
            tick_rate: Duration::from_millis(250),
        })
    }

    pub fn run<S: Screen>(&mut self, screen: &mut S) -> Result<S::Output, AppError> {
        let mut last_tick = Instant::now();

        loop {
            self.terminal
                .draw(|frame| screen.draw(frame))
                .map_err(|err| AppError::Message(err.to_string()))?;

            let timeout = self
                .tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::from_millis(0));

            if event::poll(timeout).map_err(|err| AppError::Message(err.to_string()))? {
                match event::read().map_err(|err| AppError::Message(err.to_string()))? {
                    Event::Key(key) => {
                        if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                            if let ScreenResult::Exit(output) = screen.on_event(TuiEvent::Key(key))
                            {
                                return Ok(output);
                            }
                        }
                    }
                    Event::Resize(width, height) => {
                        let _ = self.terminal.clear();
                        if let ScreenResult::Exit(output) =
                            screen.on_event(TuiEvent::Resize(width, height))
                        {
                            return Ok(output);
                        }
                    }
                    _ => {}
                }
            }

            if last_tick.elapsed() >= self.tick_rate {
                if let ScreenResult::Exit(output) = screen.on_event(TuiEvent::Tick) {
                    return Ok(output);
                }
                last_tick = Instant::now();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[test]
    fn terminal_guard_restores_on_drop() {
        let restored = Arc::new(AtomicBool::new(false));
        {
            let restored = Arc::clone(&restored);
            let _guard = TerminalGuard::new_with_restore(move || {
                restored.store(true, Ordering::SeqCst);
            });
        }
        assert!(restored.load(Ordering::SeqCst));
    }
}
