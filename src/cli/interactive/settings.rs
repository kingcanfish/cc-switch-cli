use crate::cli::i18n::{current_language, set_language, texts, Language};
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::TextViewScreen;
use crate::cli::ui::current_tui_app;
use crate::error::AppError;

use super::utils::{prompt_select, run_tui_screen};

pub fn settings_menu() -> Result<(), AppError> {
    loop {
        let lang = current_language();
        let prompt = format!(
            "{} - {}: {}",
            texts::settings_title(),
            texts::current_language_label(),
            lang.display_name()
        );

        let choices = vec![
            texts::change_language().to_string(),
            texts::back_to_main().to_string(),
        ];

        let Some(choice) = prompt_select(&prompt, choices)? else {
            break;
        };

        if choice == texts::change_language() {
            change_language_interactive()?;
        } else {
            break;
        }
    }

    Ok(())
}

fn change_language_interactive() -> Result<(), AppError> {
    let languages = vec![Language::English, Language::Chinese];

    let Some(selected) = prompt_select(texts::select_language(), languages)? else {
        return Ok(());
    };

    set_language(selected)?;

    tui_show_text(
        texts::change_language(),
        vec![texts::language_changed().to_string()],
    )?;

    Ok(())
}

fn tui_show_text(title: &str, lines: Vec<String>) -> Result<(), AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = TextViewScreen::new(title, lines, texts::press_enter(), accent);
    run_tui_screen(title, &mut screen)?;
    Ok(())
}
