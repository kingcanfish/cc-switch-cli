use std::path::Path;

use crate::app_config::{AppType, MultiAppConfig};
use crate::cli::i18n::texts;
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::TextViewScreen;
use crate::cli::ui::current_tui_app;
use crate::config::get_app_config_path;
use crate::error::AppError;
use crate::services::ConfigService;
use crate::services::ProviderService;

use super::utils::{
    get_state, prompt_confirm, prompt_select, prompt_text, prompt_text_with_default,
    prompt_text_with_help, run_tui_screen, run_with_tui_loading, run_with_tui_suspended,
};

pub fn manage_config_menu(app_type: &AppType) -> Result<(), AppError> {
    loop {
        let choices = vec![
            texts::config_show_path().to_string(),
            texts::config_show_full().to_string(),
            texts::config_export().to_string(),
            texts::config_import().to_string(),
            texts::config_backup().to_string(),
            texts::config_restore().to_string(),
            texts::config_validate().to_string(),
            texts::config_common_snippet().to_string(),
            texts::config_reset().to_string(),
            texts::back_to_main().to_string(),
        ];

        let Some(choice) = prompt_select(texts::config_management(), choices)? else {
            break;
        };

        if choice == texts::config_show_path() {
            show_config_path_interactive()?;
        } else if choice == texts::config_show_full() {
            show_full_config_interactive()?;
        } else if choice == texts::config_export() {
            let Some(path) =
                prompt_text_with_default(texts::enter_export_path(), "./config-export.json")?
            else {
                continue;
            };
            export_config_interactive(&path)?;
        } else if choice == texts::config_import() {
            let Some(path) = prompt_text(texts::enter_import_path())? else {
                continue;
            };
            import_config_interactive(&path)?;
        } else if choice == texts::config_backup() {
            backup_config_interactive()?;
        } else if choice == texts::config_restore() {
            restore_config_interactive()?;
        } else if choice == texts::config_validate() {
            validate_config_interactive()?;
        } else if choice == texts::config_common_snippet() {
            edit_common_config_snippet_interactive(app_type)?;
        } else if choice == texts::config_reset() {
            reset_config_interactive()?;
        } else {
            break;
        }
    }

    Ok(())
}

fn edit_common_config_snippet_interactive(app_type: &AppType) -> Result<(), AppError> {
    let state = get_state()?;
    let current = {
        let cfg = state.config.read()?;
        cfg.common_config_snippets.get(app_type).cloned()
    }
    .unwrap_or_default();

    let initial = if current.trim().is_empty() {
        "{}\n".to_string()
    } else {
        current
    };

    loop {
        let edited = match open_external_editor(&initial) {
            Ok(content) => content,
            Err(e) => {
                tui_show_text(texts::config_common_snippet(), vec![e.to_string()])?;
                return Ok(());
            }
        };

        if edited.trim() == initial.trim() {
            tui_show_text(
                texts::config_common_snippet(),
                vec![texts::no_changes_detected().to_string()],
            )?;
            return Ok(());
        }

        let edited = edited.trim().to_string();
        let (next_snippet, action_label) = if edited.is_empty() {
            (None, texts::common_config_snippet_cleared())
        } else {
            let value: serde_json::Value = match serde_json::from_str(&edited) {
                Ok(v) => v,
                Err(e) => {
                    tui_show_text(
                        texts::config_common_snippet(),
                        vec![format!("{}: {}", texts::invalid_json_syntax(), e)],
                    )?;
                    if !retry_prompt()? {
                        return Ok(());
                    }
                    continue;
                }
            };

            if !value.is_object() {
                tui_show_text(
                    texts::config_common_snippet(),
                    vec![texts::common_config_snippet_not_object().to_string()],
                )?;
                if !retry_prompt()? {
                    return Ok(());
                }
                continue;
            }

            let pretty = serde_json::to_string_pretty(&value)
                .map_err(|e| AppError::Message(format!("Failed to serialize JSON: {}", e)))?;

            let lines = pretty.lines().map(|line| line.to_string()).collect();
            tui_show_text(texts::config_common_snippet(), lines)?;

            let Some(confirm) = prompt_confirm(texts::confirm_save_changes(), false)? else {
                return Ok(());
            };

            if !confirm {
                tui_show_text(
                    texts::config_common_snippet(),
                    vec![texts::cancelled().to_string()],
                )?;
                return Ok(());
            }

            (Some(pretty), texts::common_config_snippet_saved())
        };

        {
            let mut cfg = state.config.write()?;
            cfg.common_config_snippets.set(app_type, next_snippet);
        }
        state.save()?;

        tui_show_text(
            texts::config_common_snippet(),
            vec![action_label.to_string()],
        )?;
        break;
    }

    let Some(apply) = prompt_confirm(texts::common_config_snippet_apply_now(), true)? else {
        return Ok(());
    };

    if apply {
        if matches!(app_type, AppType::OpenCode) {
            run_with_tui_loading(
                texts::config_common_snippet(),
                texts::syncing_to_live_config(),
                texts::cancelled(),
                move || {
                    let state = get_state()?;
                    ProviderService::sync_opencode_to_live(&state)
                },
            )?;
            tui_show_text(
                texts::config_common_snippet(),
                vec![texts::common_config_snippet_applied().to_string()],
            )?;
        } else {
            let current_id = ProviderService::current(&state, app_type.clone())?;
            if current_id.trim().is_empty() {
                tui_show_text(
                    texts::config_common_snippet(),
                    vec![texts::common_config_snippet_no_current_provider().to_string()],
                )?;
            } else {
                let app = app_type.clone();
                let current_id_owned = current_id.clone();
                run_with_tui_loading(
                    texts::config_common_snippet(),
                    texts::syncing_to_live_config(),
                    texts::cancelled(),
                    move || {
                        let state = get_state()?;
                        ProviderService::switch(&state, app, &current_id_owned)
                    },
                )?;
                tui_show_text(
                    texts::config_common_snippet(),
                    vec![texts::common_config_snippet_applied().to_string()],
                )?;
            }
        }
    } else {
        tui_show_text(
            texts::config_common_snippet(),
            vec![texts::common_config_snippet_apply_hint().to_string()],
        )?;
    }

    Ok(())
}

fn retry_prompt() -> Result<bool, AppError> {
    Ok(prompt_confirm(texts::retry_editing(), true)?.unwrap_or(false))
}

fn open_external_editor(initial_content: &str) -> Result<String, AppError> {
    run_with_tui_suspended(|| {
        edit::edit(initial_content)
            .map_err(|e| AppError::Message(format!("{}: {}", texts::editor_failed(), e)))
    })
}

fn tui_show_text(title: &str, lines: Vec<String>) -> Result<(), AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = TextViewScreen::new(title, lines, texts::press_enter(), accent);
    run_tui_screen(title, &mut screen)?;
    Ok(())
}

fn show_config_path_interactive() -> Result<(), AppError> {
    let config_path = get_app_config_path();
    let config_dir = config_path.parent().unwrap_or(&config_path);

    let mut lines = vec![
        format!("Config file: {}", config_path.display()),
        format!("Config dir:  {}", config_dir.display()),
    ];

    if config_path.exists() {
        if let Ok(metadata) = std::fs::metadata(&config_path) {
            lines.push(format!("File size:   {} bytes", metadata.len()));
        }
    } else {
        lines.push("Status:      File does not exist".to_string());
    }

    let backup_dir = config_dir.join("backups");
    if backup_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&backup_dir) {
            let count = entries.filter(|e| e.is_ok()).count();
            lines.push(format!(
                "Backups:     {} files in {}",
                count,
                backup_dir.display()
            ));
        }
    }

    tui_show_text(texts::config_show_path().trim_start_matches("📍 "), lines)
}

fn show_full_config_interactive() -> Result<(), AppError> {
    let config = MultiAppConfig::load()?;
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| AppError::Message(format!("Failed to serialize config: {}", e)))?;

    let lines = json.lines().map(|line| line.to_string()).collect();
    tui_show_text(texts::config_show_full().trim_start_matches("👁️ "), lines)
}

fn export_config_interactive(path: &str) -> Result<(), AppError> {
    let target_path = Path::new(path);

    if target_path.exists() {
        let overwrite_prompt = texts::file_overwrite_confirm(path);
        let Some(confirm) = prompt_confirm(&overwrite_prompt, false)? else {
            return Ok(());
        };

        if !confirm {
            tui_show_text(texts::config_export(), vec![texts::cancelled().to_string()])?;
            return Ok(());
        }
    }

    let export_path = path.to_string();
    run_with_tui_loading(
        texts::config_export(),
        texts::config_export(),
        texts::cancelled(),
        move || {
            let target_path = Path::new(&export_path);
            ConfigService::export_config_to_path(target_path)
        },
    )?;

    tui_show_text(
        texts::config_export(),
        vec![texts::exported_to(path).to_string()],
    )?;
    Ok(())
}

fn import_config_interactive(path: &str) -> Result<(), AppError> {
    let file_path = Path::new(path);

    if !file_path.exists() {
        return Err(AppError::Message(format!("File not found: {}", path)));
    }

    let Some(confirm) = prompt_confirm(texts::confirm_import(), false)? else {
        return Ok(());
    };

    if !confirm {
        tui_show_text(texts::config_import(), vec![texts::cancelled().to_string()])?;
        return Ok(());
    }

    let import_path = path.to_string();
    let backup_id = run_with_tui_loading(
        texts::config_import(),
        texts::config_import(),
        texts::cancelled(),
        move || {
            let state = get_state()?;
            let file_path = Path::new(&import_path);
            ConfigService::import_config_from_path(file_path, &state)
        },
    )?;

    tui_show_text(
        texts::config_import(),
        vec![
            texts::imported_from(path).to_string(),
            texts::backup_created(&backup_id),
        ],
    )?;
    Ok(())
}

fn backup_config_interactive() -> Result<(), AppError> {
    let Some(use_custom_name) = prompt_confirm(texts::backup_use_custom_name_confirm(), false)?
    else {
        return Ok(());
    };

    let custom_name = if use_custom_name {
        let Some(input) =
            prompt_text_with_help(texts::backup_name_prompt(), texts::backup_name_help())?
        else {
            return Ok(());
        };

        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    } else {
        None
    };

    let config_path = get_app_config_path();
    let backup_id = run_with_tui_loading(
        texts::config_backup(),
        texts::config_backup(),
        texts::cancelled(),
        move || ConfigService::create_backup(&config_path, custom_name),
    )?;

    let backup_dir = get_app_config_path().parent().unwrap().join("backups");
    let backup_file = backup_dir.join(format!("{}.json", backup_id));
    tui_show_text(
        texts::config_backup(),
        vec![
            texts::backup_created(&backup_id).to_string(),
            texts::backup_location(&backup_file.display().to_string()),
        ],
    )?;
    Ok(())
}

fn restore_config_interactive() -> Result<(), AppError> {
    let config_path = get_app_config_path();
    let backups = run_with_tui_loading(
        texts::config_restore(),
        texts::config_restore(),
        texts::cancelled(),
        move || ConfigService::list_backups(&config_path),
    )?;

    if backups.is_empty() {
        tui_show_text(
            texts::config_restore(),
            vec![
                texts::no_backups_available().to_string(),
                texts::backups_create_hint().to_string(),
            ],
        )?;
        return Ok(());
    }

    let choices: Vec<String> = backups
        .iter()
        .map(|b| format!("{} - {}", b.display_name, b.id))
        .collect();

    let Some(selection) = prompt_select(texts::select_backup_to_restore(), choices)? else {
        return Ok(());
    };

    let selected_backup = backups
        .iter()
        .find(|b| selection.contains(&b.id))
        .ok_or_else(|| AppError::Message(texts::invalid_backup_selection().to_string()))?;

    tui_show_text(
        texts::restore_warning_title(),
        vec![
            texts::restore_warning_replace_current().to_string(),
            texts::restore_warning_auto_backup().to_string(),
        ],
    )?;

    let Some(confirm) = prompt_confirm(texts::confirm_restore(), false)? else {
        return Ok(());
    };

    if !confirm {
        tui_show_text(
            texts::config_restore(),
            vec![texts::cancelled().to_string()],
        )?;
        return Ok(());
    }

    let restore_id = selected_backup.id.clone();
    let pre_restore_backup = run_with_tui_loading(
        texts::config_restore(),
        texts::config_restore(),
        texts::cancelled(),
        move || {
            let state = get_state()?;
            ConfigService::restore_from_backup_id(&restore_id, &state)
        },
    )?;

    let mut lines = vec![texts::restored_from(&selected_backup.display_name)];
    if !pre_restore_backup.is_empty() {
        lines.push(texts::restore_pre_backup_created(&pre_restore_backup));
    }
    lines.push(texts::restart_note().to_string());
    tui_show_text(texts::config_restore(), lines)?;
    Ok(())
}

fn validate_config_interactive() -> Result<(), AppError> {
    let config_path = get_app_config_path();

    if !config_path.exists() {
        return Err(AppError::Message("Config file does not exist".to_string()));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| AppError::Message(format!("Failed to read config: {}", e)))?;

    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| AppError::Message(format!("Invalid JSON: {}", e)))?;

    let config: MultiAppConfig = serde_json::from_str(&content)
        .map_err(|e| AppError::Message(format!("Invalid config structure: {}", e)))?;

    let claude_count = config
        .apps
        .get("claude")
        .map(|m| m.providers.len())
        .unwrap_or(0);
    let codex_count = config
        .apps
        .get("codex")
        .map(|m| m.providers.len())
        .unwrap_or(0);
    let gemini_count = config
        .apps
        .get("gemini")
        .map(|m| m.providers.len())
        .unwrap_or(0);
    let opencode_count = config
        .apps
        .get("opencode")
        .map(|m| m.providers.len())
        .unwrap_or(0);
    let mcp_count = config.mcp.servers.as_ref().map(|s| s.len()).unwrap_or(0);

    tui_show_text(
        texts::config_validate().trim_start_matches("✓ "),
        vec![
            texts::config_valid().to_string(),
            format!("Claude providers: {}", claude_count),
            format!("Codex providers:  {}", codex_count),
            format!("Gemini providers: {}", gemini_count),
            format!("OpenCode providers: {}", opencode_count),
            format!("MCP servers:      {}", mcp_count),
        ],
    )?;
    Ok(())
}

fn reset_config_interactive() -> Result<(), AppError> {
    let Some(confirm) = prompt_confirm(texts::confirm_reset(), false)? else {
        return Ok(());
    };

    if !confirm {
        tui_show_text(texts::config_reset(), vec![texts::cancelled().to_string()])?;
        return Ok(());
    }

    let backup_id = run_with_tui_loading(
        texts::config_reset(),
        texts::config_reset(),
        texts::cancelled(),
        move || {
            let config_path = get_app_config_path();
            let backup_id = ConfigService::create_backup(&config_path, None)?;

            if config_path.exists() {
                std::fs::remove_file(&config_path)
                    .map_err(|e| AppError::Message(format!("Failed to delete config: {}", e)))?;
            }

            let _ = MultiAppConfig::load()?;
            Ok(backup_id)
        },
    )?;

    tui_show_text(
        texts::config_reset(),
        vec![
            texts::config_reset_done().to_string(),
            format!("Previous config backed up: {}", backup_id),
        ],
    )?;
    Ok(())
}
