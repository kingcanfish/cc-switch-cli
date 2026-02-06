use std::cell::RefCell;
use std::io::{self, IsTerminal, Write};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::RwLock;

use crate::app_config::AppType;
use crate::app_config::MultiAppConfig;
use crate::cli::i18n::texts;
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::{
    ConfirmScreen, ListScreen, MultiSelectScreen, Screen, TextInputScreen, TuiRuntime,
};
use crate::cli::ui::current_tui_app;
use crate::error::AppError;
use crate::store::AppState;

const STATUS_SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
const STATUS_TICK_MS: u64 = 120;

fn render_status_line(message: &str, spinner_frame: usize) {
    let Ok((width, height)) = crossterm::terminal::size() else {
        return;
    };

    let mut line = format!(
        "{} {}",
        STATUS_SPINNER_FRAMES[spinner_frame % STATUS_SPINNER_FRAMES.len()],
        message
    );
    if width > 0 && line.chars().count() > width as usize {
        line = line.chars().take(width as usize).collect();
    }

    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::cursor::MoveTo(0, height.saturating_sub(1)),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        crossterm::style::Print(line)
    );
    let _ = stdout.flush();
}

fn clear_status_line() {
    let Ok((_, height)) = crossterm::terminal::size() else {
        return;
    };

    let mut stdout = io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::cursor::MoveTo(0, height.saturating_sub(1)),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
    );
    let _ = stdout.flush();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSwitchDirection {
    Previous,
    Next,
}

pub fn cycle_app_type(current: &AppType, direction: AppSwitchDirection) -> AppType {
    match (current, direction) {
        (AppType::Claude, AppSwitchDirection::Next) => AppType::Codex,
        (AppType::Codex, AppSwitchDirection::Next) => AppType::Gemini,
        (AppType::Gemini, AppSwitchDirection::Next) => AppType::OpenCode,
        (AppType::OpenCode, AppSwitchDirection::Next) => AppType::Claude,
        (AppType::Claude, AppSwitchDirection::Previous) => AppType::OpenCode,
        (AppType::Codex, AppSwitchDirection::Previous) => AppType::Claude,
        (AppType::Gemini, AppSwitchDirection::Previous) => AppType::Codex,
        (AppType::OpenCode, AppSwitchDirection::Previous) => AppType::Gemini,
    }
}

pub fn app_switch_direction_from_key(key: &console::Key) -> Option<AppSwitchDirection> {
    match key {
        console::Key::ArrowLeft => Some(AppSwitchDirection::Previous),
        console::Key::ArrowRight => Some(AppSwitchDirection::Next),
        _ => None,
    }
}

pub fn get_state() -> Result<AppState, AppError> {
    let config = MultiAppConfig::load()?;
    Ok(AppState {
        config: RwLock::new(config),
    })
}

pub fn clear_screen() {
    if !io::stdout().is_terminal() {
        return;
    }

    let term = console::Term::stdout();
    let _ = term.clear_screen();
    let _ = io::stdout().flush();
}

struct TuiSession {
    runtime: TuiRuntime,
    page_stack: Vec<String>,
}

impl TuiSession {
    fn new() -> Result<Self, AppError> {
        Ok(Self {
            runtime: TuiRuntime::new()?,
            page_stack: Vec::new(),
        })
    }
}

thread_local! {
    static TUI_SESSION: RefCell<Option<TuiSession>> = const { RefCell::new(None) };
}

pub fn init_tui_session() -> Result<(), AppError> {
    TUI_SESSION.with(|session| {
        if session.borrow().is_none() {
            *session.borrow_mut() = Some(TuiSession::new()?);
        }
        Ok(())
    })
}

pub fn shutdown_tui_session() {
    TUI_SESSION.with(|session| {
        *session.borrow_mut() = None;
    });
}

pub fn run_with_tui_suspended<T, F>(task: F) -> Result<T, AppError>
where
    F: FnOnce() -> Result<T, AppError>,
{
    let had_session = TUI_SESSION.with(|session| session.borrow().is_some());

    if had_session {
        shutdown_tui_session();
    }

    let result = task();

    if had_session {
        if let Err(init_err) = init_tui_session() {
            return match result {
                Ok(_) => Err(init_err),
                Err(task_err) => {
                    log::warn!(
                        "Failed to reinitialize TUI session after external operation: {}",
                        init_err
                    );
                    Err(task_err)
                }
            };
        }
    }

    result
}

pub fn run_tui_screen<S: Screen>(title: &str, screen: &mut S) -> Result<S::Output, AppError> {
    TUI_SESSION.with(|session| {
        if session.borrow().is_none() {
            *session.borrow_mut() = Some(TuiSession::new()?);
        }

        let mut borrowed = session.borrow_mut();
        let active = borrowed
            .as_mut()
            .ok_or_else(|| AppError::Message("TUI session is not initialized".to_string()))?;
        active.page_stack.push(title.to_string());
        let result = active.runtime.run(screen);
        active.page_stack.pop();
        result
    })
}

pub fn prompt_select<T>(message: &str, options: Vec<T>) -> Result<Option<T>, AppError>
where
    T: Clone + std::fmt::Display,
{
    prompt_select_with_help(message, options, texts::tui_list_help())
}

pub fn prompt_select_with_help<T>(
    message: &str,
    options: Vec<T>,
    help_message: &str,
) -> Result<Option<T>, AppError>
where
    T: Clone + std::fmt::Display,
{
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let labels: Vec<String> = options.iter().map(|opt| opt.to_string()).collect();
    let mut screen = ListScreen::new(
        message,
        labels,
        help_message,
        texts::tui_empty_list(),
        accent,
    );
    let selection = run_tui_screen(message, &mut screen)?;
    Ok(selection.map(|idx| options[idx].clone()))
}

pub fn prompt_select_with_help_and_default<T>(
    message: &str,
    options: Vec<T>,
    help_message: &str,
    default_index: usize,
) -> Result<Option<T>, AppError>
where
    T: Clone + std::fmt::Display,
{
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let labels: Vec<String> = options.iter().map(|opt| opt.to_string()).collect();
    let mut screen = ListScreen::new(
        message,
        labels,
        help_message,
        texts::tui_empty_list(),
        accent,
    )
    .with_selected_idx(default_index);
    let selection = run_tui_screen(message, &mut screen)?;
    Ok(selection.map(|idx| options[idx].clone()))
}

pub fn prompt_multiselect_with_help<T>(
    message: &str,
    options: Vec<T>,
    help_message: &str,
) -> Result<Option<Vec<T>>, AppError>
where
    T: Clone + std::fmt::Display,
{
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let labels: Vec<String> = options.iter().map(|opt| opt.to_string()).collect();
    let mut screen = MultiSelectScreen::new(
        message,
        labels,
        help_message,
        texts::tui_empty_list(),
        accent,
    );
    let selection = run_tui_screen(message, &mut screen)?;
    Ok(selection.map(|indices| {
        indices
            .into_iter()
            .filter_map(|idx| options.get(idx).cloned())
            .collect()
    }))
}

pub fn prompt_confirm(message: &str, default: bool) -> Result<Option<bool>, AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = ConfirmScreen::new(
        message,
        texts::tui_yes(),
        texts::tui_no(),
        texts::tui_confirm_help(),
        default,
        accent,
    );
    run_tui_screen(message, &mut screen)
}

pub fn prompt_text(message: &str) -> Result<Option<String>, AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = TextInputScreen::new(message, "", texts::tui_text_help(), accent);
    run_tui_screen(message, &mut screen)
}

pub fn prompt_text_with_default(message: &str, default: &str) -> Result<Option<String>, AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = TextInputScreen::new(message, default, texts::tui_text_help(), accent);
    run_tui_screen(message, &mut screen)
}

pub fn prompt_text_with_help(
    message: &str,
    help_message: &str,
) -> Result<Option<String>, AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = TextInputScreen::new(message, "", help_message, accent);
    run_tui_screen(message, &mut screen)
}

pub fn prompt_text_with_default_and_help(
    message: &str,
    default: &str,
    help_message: &str,
) -> Result<Option<String>, AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = TextInputScreen::new(message, default, help_message, accent);
    run_tui_screen(message, &mut screen)
}

pub fn run_with_tui_loading<T, F>(
    _title: &str,
    message: &str,
    disconnected_error: &str,
    task: F,
) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = task().map_err(|e| e.to_string());
        let _ = tx.send(result);
    });

    let mut spinner_frame = 0usize;

    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(STATUS_TICK_MS)) {
            Ok(Ok(result)) => {
                clear_status_line();
                return Ok(result);
            }
            Ok(Err(err)) => {
                clear_status_line();
                return Err(AppError::Message(err));
            }
            Err(RecvTimeoutError::Timeout) => {
                render_status_line(message, spinner_frame);
                spinner_frame = (spinner_frame + 1) % STATUS_SPINNER_FRAMES.len();
            }
            Err(RecvTimeoutError::Disconnected) => {
                clear_status_line();
                return Err(AppError::Message(disconnected_error.to_string()));
            }
        }
    }
}

pub fn pause() {
    print!("{} ", texts::press_enter());
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::Key;

    #[test]
    fn cycle_app_type_next_wraps() {
        assert_eq!(
            cycle_app_type(&AppType::Claude, AppSwitchDirection::Next),
            AppType::Codex
        );
        assert_eq!(
            cycle_app_type(&AppType::Codex, AppSwitchDirection::Next),
            AppType::Gemini
        );
        assert_eq!(
            cycle_app_type(&AppType::Gemini, AppSwitchDirection::Next),
            AppType::OpenCode
        );
        assert_eq!(
            cycle_app_type(&AppType::OpenCode, AppSwitchDirection::Next),
            AppType::Claude
        );
    }

    #[test]
    fn cycle_app_type_previous_wraps() {
        assert_eq!(
            cycle_app_type(&AppType::Claude, AppSwitchDirection::Previous),
            AppType::OpenCode
        );
        assert_eq!(
            cycle_app_type(&AppType::Codex, AppSwitchDirection::Previous),
            AppType::Claude
        );
        assert_eq!(
            cycle_app_type(&AppType::Gemini, AppSwitchDirection::Previous),
            AppType::Codex
        );
        assert_eq!(
            cycle_app_type(&AppType::OpenCode, AppSwitchDirection::Previous),
            AppType::Gemini
        );
    }

    #[test]
    fn app_switch_direction_from_key_maps_arrows() {
        assert_eq!(
            app_switch_direction_from_key(&Key::ArrowLeft),
            Some(AppSwitchDirection::Previous)
        );
        assert_eq!(
            app_switch_direction_from_key(&Key::ArrowRight),
            Some(AppSwitchDirection::Next)
        );
        assert_eq!(app_switch_direction_from_key(&Key::Enter), None);
    }
}
