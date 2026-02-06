use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Color;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use super::keymap;
use super::{Screen, ScreenResult, TuiEvent};

pub struct ListScreen {
    title: String,
    help: String,
    empty_message: String,
    header_lines: Vec<String>,
    items: Vec<String>,
    selected_idx: usize,
    filter_query: String,
    accent: Color,
}

impl ListScreen {
    pub fn new(
        title: impl Into<String>,
        items: Vec<String>,
        help: impl Into<String>,
        empty_message: impl Into<String>,
        accent: Color,
    ) -> Self {
        Self {
            title: title.into(),
            help: help.into(),
            empty_message: empty_message.into(),
            header_lines: Vec::new(),
            items,
            selected_idx: 0,
            filter_query: String::new(),
            accent,
        }
    }

    pub fn with_selected_idx(mut self, selected_idx: usize) -> Self {
        self.selected_idx = selected_idx;
        self
    }

    pub fn with_header_lines(mut self, lines: Vec<String>) -> Self {
        self.header_lines = lines;
        self
    }

    fn visible_indices(&self) -> Vec<usize> {
        let query = self.filter_query.trim();
        if query.is_empty() {
            return (0..self.items.len()).collect();
        }

        let needle = query.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                if item.to_lowercase().contains(&needle) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    fn clamp(&mut self, visible_len: usize) {
        if visible_len == 0 || self.selected_idx >= visible_len {
            self.selected_idx = 0;
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenResult<Option<usize>> {
        if keymap::is_escape(&key) {
            if !self.filter_query.trim().is_empty() {
                self.filter_query.clear();
                self.selected_idx = 0;
                return ScreenResult::Continue;
            }
            return ScreenResult::Exit(None);
        }

        if keymap::is_backspace(&key) {
            if !self.filter_query.is_empty() {
                self.filter_query.pop();
                self.selected_idx = 0;
            }
            return ScreenResult::Continue;
        }

        if let Some(ch) = keymap::char_from_key(&key) {
            if !ch.is_control() {
                self.filter_query.push(ch);
                self.selected_idx = 0;
                return ScreenResult::Continue;
            }
        }

        let visible = self.visible_indices();
        let visible_len = visible.len();

        if keymap::is_up(&key) {
            self.clamp(visible_len);
            if visible_len > 0 {
                self.selected_idx = self.selected_idx.checked_sub(1).unwrap_or(visible_len - 1);
            }
            return ScreenResult::Continue;
        }

        if keymap::is_down(&key) {
            self.clamp(visible_len);
            if visible_len > 0 {
                self.selected_idx = (self.selected_idx + 1) % visible_len;
            }
            return ScreenResult::Continue;
        }

        if keymap::is_enter(&key) {
            if visible_len == 0 {
                return ScreenResult::Exit(None);
            }
            return ScreenResult::Exit(Some(visible[self.selected_idx]));
        }

        ScreenResult::Continue
    }
}

impl Screen for ListScreen {
    type Output = Option<usize>;

    fn draw(&mut self, frame: &mut ratatui::prelude::Frame) {
        let size = frame.area();
        let header_height = std::cmp::max(3, 1 + self.header_lines.len());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(header_height as u16),
                Constraint::Min(3),
                Constraint::Length(2),
            ])
            .split(size);

        let mut header_lines = Vec::new();
        header_lines.push(Line::from(Span::styled(
            self.title.as_str(),
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD),
        )));
        header_lines.extend(
            self.header_lines
                .iter()
                .map(|line| Line::from(line.as_str())),
        );
        let header = Paragraph::new(header_lines);
        frame.render_widget(header, chunks[0]);

        let visible = self.visible_indices();
        let visible_len = visible.len();
        self.clamp(visible_len);

        if visible_len == 0 {
            let empty = Paragraph::new(self.empty_message.as_str())
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(empty, chunks[1]);
        } else {
            let items: Vec<ListItem> = visible
                .iter()
                .filter_map(|&idx| self.items.get(idx))
                .map(|item| ListItem::new(item.clone()))
                .collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(
                    Style::default()
                        .fg(self.accent)
                        .add_modifier(Modifier::BOLD),
                );

            let mut state = ListState::default();
            state.select(Some(self.selected_idx));
            frame.render_stateful_widget(list, chunks[1], &mut state);
        }

        let footer_text = if self.filter_query.trim().is_empty() {
            self.help.clone()
        } else {
            format!("{}  /{}", self.help, self.filter_query)
        };
        let footer = Paragraph::new(footer_text);
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

pub struct MultiSelectScreen {
    title: String,
    help: String,
    empty_message: String,
    items: Vec<String>,
    selected: Vec<bool>,
    cursor: usize,
    filter_query: String,
    accent: Color,
}

impl MultiSelectScreen {
    pub fn new(
        title: impl Into<String>,
        items: Vec<String>,
        help: impl Into<String>,
        empty_message: impl Into<String>,
        accent: Color,
    ) -> Self {
        let selected = vec![false; items.len()];
        Self {
            title: title.into(),
            help: help.into(),
            empty_message: empty_message.into(),
            items,
            selected,
            cursor: 0,
            filter_query: String::new(),
            accent,
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        let query = self.filter_query.trim();
        if query.is_empty() {
            return (0..self.items.len()).collect();
        }

        let needle = query.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                if item.to_lowercase().contains(&needle) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    fn clamp(&mut self, visible_len: usize) {
        if visible_len == 0 || self.cursor >= visible_len {
            self.cursor = 0;
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenResult<Option<Vec<usize>>> {
        if keymap::is_escape(&key) {
            if !self.filter_query.trim().is_empty() {
                self.filter_query.clear();
                self.cursor = 0;
                return ScreenResult::Continue;
            }
            return ScreenResult::Exit(None);
        }

        if keymap::is_backspace(&key) {
            if !self.filter_query.is_empty() {
                self.filter_query.pop();
                self.cursor = 0;
            }
            return ScreenResult::Continue;
        }

        if let Some(' ') = keymap::char_from_key(&key) {
            let visible = self.visible_indices();
            if !visible.is_empty() {
                let idx = visible[self.cursor];
                let current = self.selected.get_mut(idx);
                if let Some(value) = current {
                    *value = !*value;
                }
            }
            return ScreenResult::Continue;
        }

        if let Some(ch) = keymap::char_from_key(&key) {
            if !ch.is_control() {
                self.filter_query.push(ch);
                self.cursor = 0;
                return ScreenResult::Continue;
            }
        }

        let visible = self.visible_indices();
        let visible_len = visible.len();

        if keymap::is_up(&key) {
            self.clamp(visible_len);
            if visible_len > 0 {
                self.cursor = self.cursor.checked_sub(1).unwrap_or(visible_len - 1);
            }
            return ScreenResult::Continue;
        }

        if keymap::is_down(&key) {
            self.clamp(visible_len);
            if visible_len > 0 {
                self.cursor = (self.cursor + 1) % visible_len;
            }
            return ScreenResult::Continue;
        }

        if keymap::is_left(&key) {
            self.selected.fill(false);
            return ScreenResult::Continue;
        }

        if keymap::is_right(&key) {
            self.selected.fill(true);
            return ScreenResult::Continue;
        }

        if keymap::is_enter(&key) {
            let selected = self
                .selected
                .iter()
                .enumerate()
                .filter_map(|(idx, selected)| if *selected { Some(idx) } else { None })
                .collect::<Vec<_>>();
            return ScreenResult::Exit(Some(selected));
        }

        ScreenResult::Continue
    }
}

impl Screen for MultiSelectScreen {
    type Output = Option<Vec<usize>>;

    fn draw(&mut self, frame: &mut ratatui::prelude::Frame) {
        let size = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(2),
            ])
            .split(size);

        let header = Paragraph::new(Line::from(Span::styled(
            self.title.as_str(),
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(header, chunks[0]);

        let visible = self.visible_indices();
        let visible_len = visible.len();
        self.clamp(visible_len);

        if visible_len == 0 {
            let empty = Paragraph::new(self.empty_message.as_str())
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(empty, chunks[1]);
        } else {
            let items: Vec<ListItem> = visible
                .iter()
                .filter_map(|&idx| self.items.get(idx).map(|item| (idx, item)))
                .map(|(idx, item)| {
                    let prefix = if *self.selected.get(idx).unwrap_or(&false) {
                        "[x] "
                    } else {
                        "[ ] "
                    };
                    ListItem::new(format!("{prefix}{item}"))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(
                    Style::default()
                        .fg(self.accent)
                        .add_modifier(Modifier::BOLD),
                );

            let mut state = ListState::default();
            state.select(Some(self.cursor));
            frame.render_stateful_widget(list, chunks[1], &mut state);
        }

        let footer_text = if self.filter_query.trim().is_empty() {
            self.help.clone()
        } else {
            format!("{}  /{}", self.help, self.filter_query)
        };
        let footer = Paragraph::new(footer_text);
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

pub struct TextInputScreen {
    title: String,
    help: String,
    value: String,
    accent: Color,
}

impl TextInputScreen {
    pub fn new(
        title: impl Into<String>,
        initial: impl Into<String>,
        help: impl Into<String>,
        accent: Color,
    ) -> Self {
        Self {
            title: title.into(),
            help: help.into(),
            value: initial.into(),
            accent,
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenResult<Option<String>> {
        if keymap::is_escape(&key) {
            return ScreenResult::Exit(None);
        }

        if keymap::is_enter(&key) {
            return ScreenResult::Exit(Some(self.value.clone()));
        }

        if keymap::is_backspace(&key) {
            self.value.pop();
            return ScreenResult::Continue;
        }

        if let Some(ch) = keymap::char_from_key(&key) {
            if !ch.is_control() {
                self.value.push(ch);
            }
        }

        ScreenResult::Continue
    }
}

impl Screen for TextInputScreen {
    type Output = Option<String>;

    fn draw(&mut self, frame: &mut ratatui::prelude::Frame) {
        let size = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(size);

        let header = Paragraph::new(Line::from(Span::styled(
            self.title.as_str(),
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(header, chunks[0]);

        let input =
            Paragraph::new(self.value.as_str()).block(Block::default().borders(Borders::ALL));
        frame.render_widget(input, chunks[1]);

        let footer = Paragraph::new(self.help.as_str());
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

pub struct ConfirmScreen {
    title: String,
    yes_label: String,
    no_label: String,
    help: String,
    selected: usize,
    accent: Color,
}

impl ConfirmScreen {
    pub fn new(
        title: impl Into<String>,
        yes_label: impl Into<String>,
        no_label: impl Into<String>,
        help: impl Into<String>,
        default_yes: bool,
        accent: Color,
    ) -> Self {
        Self {
            title: title.into(),
            yes_label: yes_label.into(),
            no_label: no_label.into(),
            help: help.into(),
            selected: if default_yes { 0 } else { 1 },
            accent,
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenResult<Option<bool>> {
        if keymap::is_escape(&key) {
            return ScreenResult::Exit(None);
        }

        if keymap::is_left(&key) || keymap::is_up(&key) {
            self.selected = 0;
            return ScreenResult::Continue;
        }

        if keymap::is_right(&key) || keymap::is_down(&key) {
            self.selected = 1;
            return ScreenResult::Continue;
        }

        if keymap::is_enter(&key) {
            return ScreenResult::Exit(Some(self.selected == 0));
        }

        ScreenResult::Continue
    }
}

impl Screen for ConfirmScreen {
    type Output = Option<bool>;

    fn draw(&mut self, frame: &mut ratatui::prelude::Frame) {
        let size = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .split(size);

        let header = Paragraph::new(Line::from(Span::styled(
            self.title.as_str(),
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(header, chunks[0]);

        let labels = [self.yes_label.clone(), self.no_label.clone()];
        let items: Vec<ListItem> = labels
            .iter()
            .map(|label| ListItem::new(label.clone()))
            .collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .fg(self.accent)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(self.selected));
        frame.render_stateful_widget(list, chunks[1], &mut state);

        let footer = Paragraph::new(self.help.as_str());
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

pub struct TextViewScreen {
    title: String,
    lines: Vec<String>,
    help: String,
    accent: Color,
}

impl TextViewScreen {
    pub fn new(
        title: impl Into<String>,
        lines: Vec<String>,
        help: impl Into<String>,
        accent: Color,
    ) -> Self {
        Self {
            title: title.into(),
            lines,
            help: help.into(),
            accent,
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenResult<()> {
        if keymap::is_enter(&key) || keymap::is_escape(&key) {
            return ScreenResult::Exit(());
        }

        ScreenResult::Continue
    }
}

impl Screen for TextViewScreen {
    type Output = ();

    fn draw(&mut self, frame: &mut ratatui::prelude::Frame) {
        let size = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(2),
            ])
            .split(size);

        let header = Paragraph::new(Line::from(Span::styled(
            self.title.as_str(),
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(header, chunks[0]);

        let body_lines: Vec<Line> = self
            .lines
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect();
        let body = Paragraph::new(body_lines).block(Block::default().borders(Borders::ALL));
        frame.render_widget(body, chunks[1]);

        let footer = Paragraph::new(self.help.as_str());
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
