use std::process::Command;

use crate::app_config::AppType;
use crate::cli::i18n::texts;
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::{ListScreen, TextViewScreen};
use crate::cli::ui::create_table;
use crate::cli::ui::current_tui_app;
use crate::error::AppError;
use crate::services::McpService;
use crate::store::AppState;

use super::utils::{
    get_state, prompt_confirm, prompt_multiselect_with_help, prompt_select, prompt_text,
    run_tui_screen,
};

pub fn manage_mcp_menu(_app_type: &AppType) -> Result<(), AppError> {
    loop {
        let state = get_state()?;
        let servers = McpService::get_all_servers(&state)?;

        let choices = vec![
            texts::sync_all_servers().to_string(),
            texts::mcp_enable_server().to_string(),
            texts::mcp_disable_server().to_string(),
            texts::mcp_delete_server().to_string(),
            texts::mcp_import_servers().to_string(),
            texts::mcp_validate_command().to_string(),
            texts::back_to_main().to_string(),
        ];

        let header_lines = if servers.is_empty() {
            vec![texts::no_mcp_servers().to_string()]
        } else {
            let mut table = create_table();
            table.set_header(vec![
                texts::header_name(),
                "Claude",
                "Codex",
                "Gemini",
                "OpenCode",
            ]);

            let mut server_list: Vec<_> = servers.iter().collect();
            server_list.sort_by_key(|(id, _)| *id);

            for (_, server) in &server_list {
                table.add_row(vec![
                    server.name.clone(),
                    if server.apps.claude { "✓" } else { " " }.to_string(),
                    if server.apps.codex { "✓" } else { " " }.to_string(),
                    if server.apps.gemini { "✓" } else { " " }.to_string(),
                    if server.apps.opencode { "✓" } else { " " }.to_string(),
                ]);
            }

            table
                .to_string()
                .lines()
                .map(|line| line.to_string())
                .collect()
        };

        let accent = current_tui_app()
            .map(|app| accent_color(&app))
            .unwrap_or(ratatui::style::Color::Blue);
        let mut screen = ListScreen::new(
            texts::mcp_management(),
            choices.clone(),
            texts::tui_list_help(),
            texts::tui_empty_list(),
            accent,
        )
        .with_header_lines(header_lines);

        let Some(selection) = run_tui_screen(texts::mcp_management(), &mut screen)? else {
            break;
        };
        let choice = choices[selection].as_str();

        if choice == texts::sync_all_servers() {
            McpService::sync_all_enabled(&state)?;
            tui_show_text(
                texts::mcp_management(),
                vec![texts::synced_successfully().to_string()],
            )?;
        } else if choice == texts::mcp_enable_server() {
            mcp_enable_server_interactive(&state)?;
        } else if choice == texts::mcp_disable_server() {
            mcp_disable_server_interactive(&state)?;
        } else if choice == texts::mcp_delete_server() {
            mcp_delete_server_interactive(&state)?;
        } else if choice == texts::mcp_import_servers() {
            mcp_import_servers_interactive(&state)?;
        } else if choice == texts::mcp_validate_command() {
            mcp_validate_command_interactive()?;
        } else {
            break;
        }
    }

    Ok(())
}

fn mcp_enable_server_interactive(state: &AppState) -> Result<(), AppError> {
    let servers = McpService::get_all_servers(state)?;
    if servers.is_empty() {
        tui_show_text(
            texts::mcp_enable_server(),
            vec![texts::no_mcp_servers().to_string()],
        )?;
        return Ok(());
    }

    let server_choices: Vec<_> = servers
        .iter()
        .map(|(id, s)| format!("{} ({})", s.name, id))
        .collect();

    let Some(selected) = prompt_select(texts::select_server_to_enable(), server_choices)? else {
        return Ok(());
    };

    let server_id = selected
        .split('(')
        .nth(1)
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AppError::Message("Invalid selection".to_string()))?;

    let app_choices = vec!["Claude", "Codex", "Gemini", "OpenCode"];
    let Some(selected_apps) = prompt_multiselect_with_help(
        texts::select_apps_to_enable(),
        app_choices,
        texts::mcp_enable_apps_help(),
    )?
    else {
        return Ok(());
    };

    let apps: Vec<AppType> = selected_apps
        .iter()
        .filter_map(|&s| match s {
            "Claude" => Some(AppType::Claude),
            "Codex" => Some(AppType::Codex),
            "Gemini" => Some(AppType::Gemini),
            "OpenCode" => Some(AppType::OpenCode),
            _ => None,
        })
        .collect();

    for app in apps {
        McpService::toggle_app(state, server_id, app, true)?;
    }

    tui_show_text(
        texts::mcp_enable_server(),
        vec![texts::server_enabled(server_id).to_string()],
    )?;
    Ok(())
}

fn mcp_disable_server_interactive(state: &AppState) -> Result<(), AppError> {
    let servers = McpService::get_all_servers(state)?;
    if servers.is_empty() {
        tui_show_text(
            texts::mcp_disable_server(),
            vec![texts::no_mcp_servers().to_string()],
        )?;
        return Ok(());
    }

    let server_choices: Vec<_> = servers
        .iter()
        .map(|(id, s)| format!("{} ({})", s.name, id))
        .collect();

    let Some(selected) = prompt_select(texts::select_server_to_disable(), server_choices)? else {
        return Ok(());
    };

    let server_id = selected
        .split('(')
        .nth(1)
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AppError::Message("Invalid selection".to_string()))?;

    let app_choices = vec!["Claude", "Codex", "Gemini", "OpenCode"];
    let Some(selected_apps) = prompt_multiselect_with_help(
        texts::select_apps_to_disable(),
        app_choices,
        texts::mcp_enable_apps_help(),
    )?
    else {
        return Ok(());
    };

    let apps: Vec<AppType> = selected_apps
        .iter()
        .filter_map(|&s| match s {
            "Claude" => Some(AppType::Claude),
            "Codex" => Some(AppType::Codex),
            "Gemini" => Some(AppType::Gemini),
            "OpenCode" => Some(AppType::OpenCode),
            _ => None,
        })
        .collect();

    for app in apps {
        McpService::toggle_app(state, server_id, app, false)?;
    }

    tui_show_text(
        texts::mcp_disable_server(),
        vec![texts::server_disabled(server_id).to_string()],
    )?;
    Ok(())
}

fn mcp_delete_server_interactive(state: &AppState) -> Result<(), AppError> {
    let servers = McpService::get_all_servers(state)?;
    if servers.is_empty() {
        tui_show_text(
            texts::mcp_delete_server(),
            vec![texts::no_servers_to_delete().to_string()],
        )?;
        return Ok(());
    }

    let server_choices: Vec<_> = servers
        .iter()
        .map(|(id, s)| format!("{} ({})", s.name, id))
        .collect();

    let Some(selected) = prompt_select(texts::select_server_to_delete(), server_choices)? else {
        return Ok(());
    };

    let server_id = selected
        .split('(')
        .nth(1)
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AppError::Message("Invalid selection".to_string()))?;

    let confirm_prompt = texts::confirm_delete(server_id);
    let Some(confirm) = prompt_confirm(&confirm_prompt, false)? else {
        return Ok(());
    };

    if !confirm {
        tui_show_text(
            texts::mcp_delete_server(),
            vec![texts::cancelled().to_string()],
        )?;
        return Ok(());
    }

    McpService::delete_server(state, server_id)?;
    tui_show_text(
        texts::mcp_delete_server(),
        vec![texts::server_deleted(server_id).to_string()],
    )?;
    Ok(())
}

fn mcp_import_servers_interactive(state: &AppState) -> Result<(), AppError> {
    let mut total = 0;
    total += McpService::import_from_app(state, AppType::Claude)?;
    total += McpService::import_from_app(state, AppType::Codex)?;
    total += McpService::import_from_app(state, AppType::Gemini)?;
    total += McpService::import_from_app(state, AppType::OpenCode)?;

    tui_show_text(
        texts::mcp_import_servers(),
        vec![texts::servers_imported(total).to_string()],
    )?;
    Ok(())
}

fn mcp_validate_command_interactive() -> Result<(), AppError> {
    let Some(command) = prompt_text(texts::enter_command_to_validate())? else {
        return Ok(());
    };

    let is_valid = if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(&command)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        Command::new("which")
            .arg(&command)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    let message = if is_valid {
        texts::command_valid(&command).to_string()
    } else {
        texts::command_invalid(&command).to_string()
    };
    tui_show_text(texts::mcp_validate_command(), vec![message])?;
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
