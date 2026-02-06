use ratatui::style::Color;

use crate::app_config::AppType;

pub fn accent_color(app_type: &AppType) -> Color {
    match app_type {
        AppType::Codex => Color::Green,
        AppType::Claude => Color::Cyan,
        AppType::Gemini => Color::Magenta,
        AppType::OpenCode => Color::Blue,
    }
}
