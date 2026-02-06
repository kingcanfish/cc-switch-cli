use crate::app_config::AppType;
use crate::cli::i18n::texts;
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::TextViewScreen;
use crate::cli::ui::current_tui_app;
use crate::error::AppError;
use crate::services::PromptService;
use crate::store::AppState;

use super::utils::{get_state, prompt_confirm, prompt_select, run_tui_screen};

pub fn manage_prompts_menu(app_type: &AppType) -> Result<(), AppError> {
    loop {
        let state = get_state()?;
        let prompts = PromptService::get_prompts(&state, app_type.clone())?;

        let choices = vec![
            texts::prompts_view_current().to_string(),
            texts::switch_active_prompt().to_string(),
            texts::prompts_show_content().to_string(),
            texts::prompts_delete().to_string(),
            texts::back_to_main().to_string(),
        ];

        let Some(choice) = prompt_select(texts::prompts_management(), choices)? else {
            break;
        };

        if choice == texts::prompts_view_current() {
            view_current_prompt_interactive(&state, &prompts)?;
        } else if choice == texts::switch_active_prompt() {
            switch_prompt_interactive(&state, app_type, &prompts)?;
        } else if choice == texts::prompts_show_content() {
            show_prompt_content_interactive(&prompts)?;
        } else if choice == texts::prompts_delete() {
            delete_prompt_interactive(&state, app_type, &prompts)?;
        } else {
            break;
        }
    }

    Ok(())
}

fn view_current_prompt_interactive(
    _state: &AppState,
    prompts: &std::collections::HashMap<String, crate::prompt::Prompt>,
) -> Result<(), AppError> {
    let active = prompts.iter().find(|(_, p)| p.enabled);

    if let Some((id, prompt)) = active {
        let mut lines = Vec::new();
        lines.push(format!("ID: {}", id));
        lines.push(format!("Name: {}", prompt.name));
        if let Some(desc) = &prompt.description {
            lines.push(format!("Description: {}", desc));
        }
        lines.push(String::new());
        lines.push("Content:".to_string());
        lines.extend(prompt.content.lines().map(|line| line.to_string()));
        tui_show_text(
            texts::prompts_view_current().trim_start_matches("📋 "),
            lines,
        )?;
    } else {
        tui_show_text(
            texts::prompts_view_current(),
            vec![texts::no_active_prompt().to_string()],
        )?;
    }

    Ok(())
}

fn show_prompt_content_interactive(
    prompts: &std::collections::HashMap<String, crate::prompt::Prompt>,
) -> Result<(), AppError> {
    if prompts.is_empty() {
        tui_show_text(
            texts::prompts_show_content(),
            vec![texts::no_prompts_available().to_string()],
        )?;
        return Ok(());
    }

    let prompt_choices: Vec<_> = prompts
        .iter()
        .map(|(id, p)| format!("{} ({})", p.name, id))
        .collect();

    let Some(selected) = prompt_select(texts::select_prompt_to_view(), prompt_choices)? else {
        return Ok(());
    };

    let prompt_id = selected
        .split('(')
        .nth(1)
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AppError::Message("Invalid selection".to_string()))?;

    if let Some(prompt) = prompts.get(prompt_id) {
        let mut lines = Vec::new();
        if let Some(desc) = &prompt.description {
            lines.push(format!("Description: {}", desc));
            lines.push(String::new());
        }
        lines.extend(prompt.content.lines().map(|line| line.to_string()));
        tui_show_text(&prompt.name, lines)?;
    }

    Ok(())
}

fn delete_prompt_interactive(
    state: &AppState,
    app_type: &AppType,
    prompts: &std::collections::HashMap<String, crate::prompt::Prompt>,
) -> Result<(), AppError> {
    let deletable: Vec<_> = prompts
        .iter()
        .filter(|(_, p)| !p.enabled)
        .map(|(id, p)| format!("{} ({})", p.name, id))
        .collect();

    if deletable.is_empty() {
        let mut lines = vec![texts::no_prompts_to_delete().to_string()];
        if prompts.iter().any(|(_, p)| p.enabled) {
            lines.push(texts::cannot_delete_active().to_string());
        }
        tui_show_text(texts::prompts_delete(), lines)?;
        return Ok(());
    }

    let Some(selected) = prompt_select(texts::select_prompt_to_delete(), deletable)? else {
        return Ok(());
    };

    let prompt_id = selected
        .split('(')
        .nth(1)
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AppError::Message("Invalid selection".to_string()))?;

    let confirm_prompt = texts::confirm_delete(prompt_id);
    let Some(confirm) = prompt_confirm(&confirm_prompt, false)? else {
        return Ok(());
    };

    if !confirm {
        tui_show_text(
            texts::prompts_delete(),
            vec![texts::cancelled().to_string()],
        )?;
        return Ok(());
    }

    PromptService::delete_prompt(state, app_type.clone(), prompt_id)?;
    tui_show_text(
        texts::prompts_delete(),
        vec![texts::prompt_deleted(prompt_id).to_string()],
    )?;
    Ok(())
}

fn switch_prompt_interactive(
    state: &AppState,
    app_type: &AppType,
    prompts: &std::collections::HashMap<String, crate::prompt::Prompt>,
) -> Result<(), AppError> {
    if prompts.is_empty() {
        tui_show_text(
            texts::switch_active_prompt(),
            vec![texts::no_prompts_available().to_string()],
        )?;
        return Ok(());
    }

    let prompt_choices: Vec<_> = prompts
        .iter()
        .map(|(id, p)| format!("{} ({})", p.name, id))
        .collect();

    let Some(choice) = prompt_select(texts::select_prompt_to_activate(), prompt_choices)? else {
        return Ok(());
    };

    let id = choice
        .split('(')
        .nth(1)
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AppError::Message("Invalid choice".to_string()))?;

    if let Some(prompt) = prompts.get(id) {
        if prompt.enabled {
            PromptService::disable_prompt(state, app_type.clone(), id)?;
            tui_show_text(
                texts::switch_active_prompt(),
                vec![
                    texts::deactivated_prompt(id).to_string(),
                    texts::prompt_cleared_note().to_string(),
                ],
            )?;
            return Ok(());
        }
    }

    PromptService::enable_prompt(state, app_type.clone(), id)?;
    tui_show_text(
        texts::switch_active_prompt(),
        vec![
            texts::activated_prompt(id).to_string(),
            texts::prompt_synced_note().to_string(),
        ],
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
