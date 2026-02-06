use crossterm::event::KeyEvent;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app_config::AppType;
use crate::cli::i18n::texts;
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::{keymap, Screen, ScreenResult, TuiEvent};
use crate::error::AppError;
use crate::settings as app_settings;

use super::utils::{cycle_app_type, run_tui_screen, AppSwitchDirection};
use super::MainMenuChoice;

pub(super) struct MainMenuOutcome {
    pub(super) choice: MainMenuChoice,
    pub(super) app_type: AppType,
}

pub fn show_main_menu_tui(app_type: AppType) -> Result<MainMenuOutcome, AppError> {
    let mut screen = MainMenuScreen::new(app_type);
    run_tui_screen(texts::welcome_title(), &mut screen)
}

struct MainMenuScreen {
    app_type: AppType,
    selected_idx: usize,
    filter_query: String,
    filter_mode: bool,
    choices: Vec<MainMenuChoice>,
}

impl MainMenuScreen {
    fn new(app_type: AppType) -> Self {
        Self {
            app_type,
            selected_idx: 0,
            filter_query: String::new(),
            filter_mode: false,
            choices: vec![
                MainMenuChoice::ManageProviders,
                MainMenuChoice::ManageMCP,
                MainMenuChoice::ManagePrompts,
                MainMenuChoice::ManageSkills,
                MainMenuChoice::ManageConfig,
                MainMenuChoice::ViewCurrentConfig,
                MainMenuChoice::SwitchApp,
                MainMenuChoice::Settings,
                MainMenuChoice::Exit,
            ],
        }
    }

    fn visible_choices(&self) -> Vec<MainMenuChoice> {
        let query = self.filter_query.trim();
        if query.is_empty() {
            return self.choices.clone();
        }

        let query_lower = query.to_lowercase();
        self.choices
            .iter()
            .filter(|choice| choice.to_string().to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }

    fn clamp_selection(&mut self, visible_len: usize) {
        if visible_len == 0 || self.selected_idx >= visible_len {
            self.selected_idx = 0;
        }
    }

    fn exit(&self, choice: MainMenuChoice) -> ScreenResult<MainMenuOutcome> {
        ScreenResult::Exit(MainMenuOutcome {
            choice,
            app_type: self.app_type.clone(),
        })
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult<MainMenuOutcome> {
        if self.filter_mode {
            if keymap::is_escape(&key) {
                if self.filter_query.trim().is_empty() {
                    self.filter_mode = false;
                } else {
                    self.filter_query.clear();
                    self.filter_mode = false;
                }
                self.selected_idx = 0;
                return ScreenResult::Continue;
            }

            if keymap::is_enter(&key) {
                self.filter_mode = false;
                self.selected_idx = 0;
                return ScreenResult::Continue;
            }

            if keymap::is_backspace(&key) {
                self.filter_query.pop();
                self.selected_idx = 0;
                return ScreenResult::Continue;
            }

            if let Some(ch) = keymap::char_from_key(&key) {
                if !ch.is_control() {
                    self.filter_query.push(ch);
                    self.selected_idx = 0;
                }
            }

            return ScreenResult::Continue;
        }

        if keymap::is_left(&key) {
            self.app_type = cycle_app_type(&self.app_type, AppSwitchDirection::Previous);
            if let Err(err) = app_settings::set_last_app(&self.app_type) {
                log::warn!("Failed to persist last app: {}", err);
            }
            return ScreenResult::Continue;
        }

        if keymap::is_right(&key) {
            self.app_type = cycle_app_type(&self.app_type, AppSwitchDirection::Next);
            if let Err(err) = app_settings::set_last_app(&self.app_type) {
                log::warn!("Failed to persist last app: {}", err);
            }
            return ScreenResult::Continue;
        }

        if keymap::is_escape(&key) {
            if self.filter_query.trim().is_empty() {
                return self.exit(MainMenuChoice::Exit);
            }

            self.filter_query.clear();
            self.selected_idx = 0;
            return ScreenResult::Continue;
        }

        if keymap::is_up(&key) {
            let visible = self.visible_choices();
            self.clamp_selection(visible.len());
            if !visible.is_empty() {
                self.selected_idx = self
                    .selected_idx
                    .checked_sub(1)
                    .unwrap_or(visible.len() - 1);
            }
            return ScreenResult::Continue;
        }

        if keymap::is_down(&key) {
            let visible = self.visible_choices();
            self.clamp_selection(visible.len());
            if !visible.is_empty() {
                self.selected_idx = (self.selected_idx + 1) % visible.len();
            }
            return ScreenResult::Continue;
        }

        if keymap::is_enter(&key) {
            let visible = self.visible_choices();
            if let Some(choice) = visible.get(self.selected_idx).cloned() {
                return self.exit(choice);
            }
            return ScreenResult::Continue;
        }

        if let Some('/') = keymap::char_from_key(&key) {
            self.filter_mode = true;
            return ScreenResult::Continue;
        }

        ScreenResult::Continue
    }
}

impl Screen for MainMenuScreen {
    type Output = MainMenuOutcome;

    fn draw(&mut self, frame: &mut ratatui::prelude::Frame) {
        let size = frame.area();
        let accent = accent_color(&self.app_type);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(size);

        let mut header_lines = Vec::new();
        header_lines.push(Line::from(Span::styled(
            texts::welcome_title(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )));
        header_lines.push(Line::from(vec![
            Span::raw(format!("{}: ", texts::application())),
            Span::styled(
                self.app_type.as_str(),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
        ]));
        header_lines.push(Line::from(texts::main_menu_prompt(self.app_type.as_str())));

        if !self.filter_query.trim().is_empty() || self.filter_mode {
            let mut query = self.filter_query.clone();
            if self.filter_mode {
                query.push('_');
            }
            header_lines.push(Line::from(texts::main_menu_filtering(&query)));
        }

        let header = Paragraph::new(header_lines).block(Block::default());
        frame.render_widget(header, chunks[0]);

        let visible = self.visible_choices();
        self.clamp_selection(visible.len());

        if visible.is_empty() {
            let empty = Paragraph::new(texts::main_menu_no_matches()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(accent)),
            );
            frame.render_widget(empty, chunks[1]);
        } else {
            let items: Vec<ListItem> = visible
                .iter()
                .map(|choice| ListItem::new(choice.to_string()))
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(Style::default().fg(accent).add_modifier(Modifier::BOLD));

            let mut state = ListState::default();
            state.select(Some(self.selected_idx));
            frame.render_stateful_widget(list, chunks[1], &mut state);
        }

        let footer_text = if self.filter_mode {
            texts::main_menu_search_prompt()
        } else {
            texts::main_menu_help()
        };
        let footer = Paragraph::new(footer_text).block(Block::default());
        frame.render_widget(footer, chunks[2]);
    }

    fn on_event(&mut self, event: TuiEvent) -> ScreenResult<Self::Output> {
        match event {
            TuiEvent::Key(key) => self.handle_key(key),
            TuiEvent::Resize(_, _) => ScreenResult::Continue,
            TuiEvent::Tick => ScreenResult::Continue,
        }
    }
}
