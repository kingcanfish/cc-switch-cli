use crate::app_config::AppType;
use crate::cli::i18n::texts;
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::{ListScreen, TextViewScreen};
use crate::error::AppError;
use crate::services::{ProviderService, SpeedtestService};
use crate::store::AppState;

use super::utils::{
    get_state, prompt_confirm, prompt_select, run_tui_screen, run_with_tui_loading,
    run_with_tui_suspended,
};

pub fn manage_providers_menu(app_type: &AppType) -> Result<(), AppError> {
    loop {
        let choices = vec![
            texts::view_current_provider().to_string(),
            texts::switch_provider().to_string(),
            texts::add_provider().to_string(),
            texts::edit_provider_menu().to_string(),
            texts::delete_provider().to_string(),
            texts::back_to_main().to_string(),
        ];

        let Some(selection) = tui_select(texts::provider_management(), choices, app_type)? else {
            break;
        };

        match selection {
            0 => {
                let state = get_state()?;
                let current_id = ProviderService::current(&state, app_type.clone())?;
                view_provider_detail_tui(&state, app_type, &current_id)?;
            }
            1 => {
                let state = get_state()?;
                let providers = ProviderService::list(&state, app_type.clone())?;
                let current_id = ProviderService::current(&state, app_type.clone())?;
                switch_provider_interactive(&state, app_type, &providers, &current_id)?;
            }
            2 => add_provider_interactive(app_type)?,
            3 => {
                let state = get_state()?;
                let providers = ProviderService::list(&state, app_type.clone())?;
                edit_provider_interactive(app_type, &providers)?;
            }
            4 => {
                let state = get_state()?;
                let providers = ProviderService::list(&state, app_type.clone())?;
                let current_id = ProviderService::current(&state, app_type.clone())?;
                delete_provider_interactive(&state, app_type, &providers, &current_id)?;
            }
            _ => break,
        }
    }

    Ok(())
}

fn view_provider_detail_tui(
    state: &AppState,
    app_type: &AppType,
    current_id: &str,
) -> Result<(), AppError> {
    if matches!(app_type, AppType::OpenCode) && current_id.trim().is_empty() {
        tui_show_text(
            texts::current_provider_details(),
            vec![texts::opencode_no_current_provider().to_string()],
            app_type,
        )?;
        return Ok(());
    }

    loop {
        let providers = ProviderService::list(state, app_type.clone())?;
        let current_id = current_id.trim();
        let Some(provider) = providers.get(current_id) else {
            let message = if current_id.is_empty() {
                texts::no_current_provider().to_string()
            } else {
                texts::provider_not_found(current_id)
            };
            tui_show_text(texts::current_provider_details(), vec![message], app_type)?;
            break;
        };

        let mut lines = Vec::new();
        lines.push(texts::basic_info_section_header().to_string());
        lines.push(format!("ID: {}", current_id));
        lines.push(format!(
            "{} {}",
            texts::name_label_with_colon(),
            provider.name
        ));
        lines.push(format!(
            "{} {}",
            texts::app_label_with_colon(),
            app_type.as_str()
        ));

        if matches!(app_type, AppType::Claude) {
            let config = extract_claude_config(&provider.settings_config);
            lines.push(String::new());
            lines.push(texts::api_config_section_header().to_string());
            lines.push(format!(
                "Base URL: {}",
                config.base_url.unwrap_or_else(|| "N/A".to_string())
            ));
            lines.push(format!(
                "API Key: {}",
                config.api_key.unwrap_or_else(|| "N/A".to_string())
            ));
            lines.push(String::new());
            lines.push(texts::model_config_section_header().to_string());
            lines.push(format!(
                "{} {}",
                texts::main_model_label_with_colon(),
                config.model.unwrap_or_else(|| "default".to_string())
            ));
            lines.push(format!(
                "Haiku: {}",
                config.haiku_model.unwrap_or_else(|| "default".to_string())
            ));
            lines.push(format!(
                "Sonnet: {}",
                config.sonnet_model.unwrap_or_else(|| "default".to_string())
            ));
            lines.push(format!(
                "Opus: {}",
                config.opus_model.unwrap_or_else(|| "default".to_string())
            ));
        } else {
            lines.push(String::new());
            lines.push(texts::api_config_section_header().to_string());
            let api_url = extract_api_url(&provider.settings_config, app_type)
                .unwrap_or_else(|| "N/A".to_string());
            lines.push(format!("API URL: {}", api_url));
        }

        tui_show_text(texts::current_provider_details(), lines, app_type)?;

        let actions = vec![
            texts::speedtest_endpoint().to_string(),
            texts::back().to_string(),
        ];
        let Some(action) = tui_select(texts::choose_action(), actions, app_type)? else {
            break;
        };
        if action == 0 {
            speedtest_provider_tui(app_type, provider)?;
        } else {
            break;
        }
    }

    Ok(())
}

fn speedtest_provider_tui(
    app_type: &AppType,
    provider: &crate::provider::Provider,
) -> Result<(), AppError> {
    let api_url = extract_api_url(&provider.settings_config, app_type);

    if api_url.is_none() {
        tui_show_text(
            texts::speedtest_endpoint(),
            vec![texts::no_api_url_configured().to_string()],
            app_type,
        )?;
        return Ok(());
    }

    let api_url = api_url.unwrap();

    let speedtest_url = api_url.clone();
    let loading_message = format!("{} {}", texts::endpoint_label_colon(), api_url);
    let results = run_with_tui_loading(
        texts::speedtest_endpoint(),
        &loading_message,
        texts::cancelled(),
        move || {
            let runtime = tokio::runtime::Runtime::new()
                .map_err(|e| AppError::Message(format!("Failed to create async runtime: {}", e)))?;
            runtime.block_on(async {
                SpeedtestService::test_endpoints(vec![speedtest_url], None).await
            })
        },
    )?;

    let mut lines = Vec::new();
    lines.push(format!("{} {}", texts::endpoint_label_colon(), api_url));

    if let Some(result) = results.first() {
        let latency_str = if let Some(latency) = result.latency {
            format!("{} ms", latency)
        } else if result.error.is_some() {
            "Failed".to_string()
        } else {
            "Timeout".to_string()
        };

        let status_str = result
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        lines.push(format!("Latency: {}", latency_str));
        lines.push(format!("Status: {}", status_str));

        if let Some(err) = &result.error {
            lines.push(format!("Error: {}", err));
        }
    }

    tui_show_text(texts::speedtest_endpoint(), lines, app_type)?;
    Ok(())
}

fn tui_select(
    title: &str,
    items: Vec<String>,
    app_type: &AppType,
) -> Result<Option<usize>, AppError> {
    let accent = accent_color(app_type);
    let mut screen = ListScreen::new(
        title,
        items,
        texts::tui_list_help(),
        texts::tui_empty_list(),
        accent,
    );
    run_tui_screen(title, &mut screen)
}

fn tui_show_text(title: &str, lines: Vec<String>, app_type: &AppType) -> Result<(), AppError> {
    let accent = accent_color(app_type);
    let mut screen = TextViewScreen::new(title, lines, texts::press_enter(), accent);
    run_tui_screen(title, &mut screen)?;
    Ok(())
}

pub fn extract_api_url(settings_config: &serde_json::Value, app_type: &AppType) -> Option<String> {
    match app_type {
        AppType::Claude => settings_config
            .get("env")?
            .get("ANTHROPIC_BASE_URL")?
            .as_str()
            .map(|s| s.to_string()),
        AppType::Codex => {
            if let Some(config_str) = settings_config.get("config")?.as_str() {
                for line in config_str.lines() {
                    let line = line.trim();
                    if line.starts_with("base_url") {
                        if let Some(url_part) = line.split('=').nth(1) {
                            let url = url_part.trim().trim_matches('"').trim_matches('\'');
                            return Some(url.to_string());
                        }
                    }
                }
            }
            None
        }
        AppType::Gemini => settings_config
            .get("env")?
            .get("GEMINI_BASE_URL")
            .or_else(|| settings_config.get("env")?.get("BASE_URL"))?
            .as_str()
            .map(|s| s.to_string()),
        AppType::OpenCode => settings_config
            .get("options")?
            .get("baseURL")?
            .as_str()
            .map(|s| s.to_string()),
    }
}

fn switch_provider_interactive(
    _state: &AppState,
    app_type: &AppType,
    providers: &std::collections::HashMap<String, crate::provider::Provider>,
    current_id: &str,
) -> Result<(), AppError> {
    if matches!(app_type, AppType::OpenCode) {
        tui_show_text(
            texts::switch_provider(),
            vec![texts::opencode_switch_not_supported().to_string()],
            app_type,
        )?;
        return Ok(());
    }

    if providers.len() <= 1 {
        tui_show_text(
            texts::switch_provider(),
            vec![texts::only_one_provider().to_string()],
            app_type,
        )?;
        return Ok(());
    }

    let mut provider_choices: Vec<_> = providers
        .iter()
        .filter(|(id, _)| *id != current_id)
        .map(|(id, p)| format!("{} ({})", p.name, id))
        .collect();
    provider_choices.sort();

    if provider_choices.is_empty() {
        tui_show_text(
            texts::switch_provider(),
            vec![texts::no_other_providers().to_string()],
            app_type,
        )?;
        return Ok(());
    }

    let Some(choice) = prompt_select(texts::select_provider_to_switch(), provider_choices)? else {
        return Ok(());
    };

    let id = choice
        .split('(')
        .nth(1)
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AppError::Message("Invalid choice".to_string()))?;

    let app = app_type.clone();
    let id_owned = id.to_string();
    run_with_tui_loading(
        texts::switch_provider(),
        texts::syncing_to_live_config(),
        texts::cancelled(),
        move || {
            let state = get_state()?;
            ProviderService::switch(&state, app, &id_owned)
        },
    )?;

    tui_show_text(
        texts::switch_provider(),
        vec![
            texts::switched_to_provider(id),
            texts::restart_note().to_string(),
        ],
        app_type,
    )?;

    Ok(())
}

fn delete_provider_interactive(
    _state: &AppState,
    app_type: &AppType,
    providers: &std::collections::HashMap<String, crate::provider::Provider>,
    current_id: &str,
) -> Result<(), AppError> {
    let deletable: Vec<_> = providers
        .iter()
        .filter(|(id, _)| *id != current_id)
        .map(|(id, p)| format!("{} ({})", p.name, id))
        .collect();

    if deletable.is_empty() {
        tui_show_text(
            texts::delete_provider(),
            vec![texts::no_deletable_providers().to_string()],
            app_type,
        )?;
        return Ok(());
    }

    let Some(choice) = prompt_select(texts::select_provider_to_delete(), deletable)? else {
        return Ok(());
    };

    let id = choice
        .split('(')
        .nth(1)
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AppError::Message("Invalid choice".to_string()))?;

    let confirm_prompt = texts::confirm_delete(id);
    let Some(confirm) = prompt_confirm(&confirm_prompt, false)? else {
        return Ok(());
    };

    if !confirm {
        tui_show_text(
            texts::delete_provider(),
            vec![texts::cancelled().to_string()],
            app_type,
        )?;
        return Ok(());
    }

    let app = app_type.clone();
    let id_owned = id.to_string();
    run_with_tui_loading(
        texts::delete_provider(),
        texts::delete_provider(),
        texts::cancelled(),
        move || {
            let state = get_state()?;
            ProviderService::delete(&state, app, &id_owned)
        },
    )?;
    tui_show_text(
        texts::delete_provider(),
        vec![texts::deleted_provider(id)],
        app_type,
    )?;

    Ok(())
}

fn add_provider_interactive(app_type: &AppType) -> Result<(), AppError> {
    crate::cli::commands::provider::execute(
        crate::cli::commands::provider::ProviderCommand::Add,
        Some(app_type.clone()),
    )
}

/// Edit mode choices for provider editing
#[derive(Debug, Clone)]
enum EditMode {
    Interactive,
    JsonEditor,
    Cancel,
}

impl std::fmt::Display for EditMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interactive => write!(f, "{}", texts::edit_mode_interactive()),
            Self::JsonEditor => write!(f, "{}", texts::edit_mode_json_editor()),
            Self::Cancel => write!(f, "{}", texts::cancel()),
        }
    }
}

/// Codex config file choices for JSON editing
#[derive(Debug, Clone)]
enum CodexConfigFile {
    Auth,   // auth.json
    Config, // config.toml
}

impl std::fmt::Display for CodexConfigFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth => write!(f, "auth.json"),
            Self::Config => write!(f, "config.toml"),
        }
    }
}

fn edit_provider_interactive(
    app_type: &AppType,
    providers: &std::collections::HashMap<String, crate::provider::Provider>,
) -> Result<(), AppError> {
    if providers.is_empty() {
        tui_show_text(
            texts::edit_provider_menu(),
            vec![texts::no_editable_providers().to_string()],
            app_type,
        )?;
        return Ok(());
    }

    let mut provider_list: Vec<_> = providers.iter().collect();
    provider_list.sort_by(|(_, a), (_, b)| match (a.sort_index, b.sort_index) {
        (Some(idx_a), Some(idx_b)) => idx_a.cmp(&idx_b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.created_at.cmp(&b.created_at),
    });

    let choices: Vec<String> = provider_list
        .iter()
        .map(|(id, provider)| format!("{} ({})", provider.name, id))
        .collect();

    let Some(selection) = prompt_select(texts::select_provider_to_edit(), choices)? else {
        return Ok(());
    };

    let selected_id = selection
        .rsplit_once('(')
        .and_then(|(_, id_part)| id_part.strip_suffix(')'))
        .ok_or_else(|| AppError::Message(texts::invalid_selection_format().to_string()))?
        .to_string();

    let edit_mode_choices = vec![
        EditMode::Interactive,
        EditMode::JsonEditor,
        EditMode::Cancel,
    ];

    let Some(edit_mode) = prompt_select(texts::choose_edit_mode(), edit_mode_choices)? else {
        return Ok(());
    };

    match edit_mode {
        EditMode::Interactive => {
            crate::cli::commands::provider::execute(
                crate::cli::commands::provider::ProviderCommand::Edit { id: selected_id },
                Some(app_type.clone()),
            )?;
        }
        EditMode::JsonEditor => {
            let original = providers
                .get(&selected_id)
                .ok_or_else(|| AppError::Message("Provider not found".to_string()))?;
            edit_provider_with_json_editor(app_type, &selected_id, original)?;
        }
        EditMode::Cancel => {
            tui_show_text(
                texts::edit_provider_menu(),
                vec![texts::cancelled().to_string()],
                app_type,
            )?;
        }
    }

    Ok(())
}

/// Edit provider using external JSON editor (per-file editing)
fn edit_provider_with_json_editor(
    app_type: &AppType,
    id: &str,
    original: &crate::provider::Provider,
) -> Result<(), AppError> {
    let (field_name, content_to_edit, is_toml) = match app_type {
        AppType::Claude => {
            let json_str = serde_json::to_string_pretty(&original.settings_config)
                .map_err(|e| AppError::JsonSerialize { source: e })?;
            ("settings_config", json_str, false)
        }
        AppType::Codex => {
            let Some(file_choice) = prompt_select(
                "Select config file to edit:",
                vec![CodexConfigFile::Auth, CodexConfigFile::Config],
            )?
            else {
                return Ok(());
            };

            match file_choice {
                CodexConfigFile::Auth => {
                    let auth_value = original.settings_config.get("auth").ok_or_else(|| {
                        AppError::Message("Missing 'auth' field in settings_config".to_string())
                    })?;

                    let json_str = serde_json::to_string_pretty(auth_value)
                        .map_err(|e| AppError::JsonSerialize { source: e })?;

                    ("settings_config.auth", json_str, false)
                }
                CodexConfigFile::Config => {
                    let config_str = original
                        .settings_config
                        .get("config")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            AppError::Message(
                                "Missing or invalid 'config' field in settings_config".to_string(),
                            )
                        })?;

                    ("settings_config.config", config_str.to_string(), true)
                }
            }
        }
        AppType::Gemini => {
            let json_str = serde_json::to_string_pretty(&original.settings_config)
                .map_err(|e| AppError::JsonSerialize { source: e })?;
            ("settings_config", json_str, false)
        }
        AppType::OpenCode => {
            let json_str = serde_json::to_string_pretty(&original.settings_config)
                .map_err(|e| AppError::JsonSerialize { source: e })?;
            ("settings_config", json_str, false)
        }
    };

    loop {
        tui_show_text(
            texts::edit_provider_menu(),
            vec![format!(
                "{} ({})",
                texts::opening_external_editor(),
                field_name
            )],
            app_type,
        )?;

        let edited_content = match open_external_editor(&content_to_edit) {
            Ok(content) => content,
            Err(e) => {
                tui_show_text(texts::edit_provider_menu(), vec![e.to_string()], app_type)?;
                return Ok(());
            }
        };

        if edited_content.trim() == content_to_edit.trim() {
            tui_show_text(
                texts::edit_provider_menu(),
                vec![texts::no_changes_detected().to_string()],
                app_type,
            )?;
            return Ok(());
        }

        let validated_value = if is_toml {
            match toml::from_str::<toml::Value>(&edited_content) {
                Ok(_) => serde_json::Value::String(edited_content.clone()),
                Err(e) => {
                    tui_show_text(
                        texts::edit_provider_menu(),
                        vec![format!("{}: {}", texts::invalid_toml_syntax(), e)],
                        app_type,
                    )?;

                    if !retry_prompt()? {
                        return Ok(());
                    }
                    continue;
                }
            }
        } else {
            match serde_json::from_str::<serde_json::Value>(&edited_content) {
                Ok(v) => v,
                Err(e) => {
                    tui_show_text(
                        texts::edit_provider_menu(),
                        vec![format!("{}: {}", texts::invalid_json_syntax(), e)],
                        app_type,
                    )?;

                    if !retry_prompt()? {
                        return Ok(());
                    }
                    continue;
                }
            }
        };

        let mut updated_provider = original.clone();

        match app_type {
            AppType::Claude => {
                updated_provider.settings_config = validated_value;
            }
            AppType::Codex => {
                if let Some(settings_obj) = updated_provider.settings_config.as_object_mut() {
                    if field_name == "settings_config.auth" {
                        settings_obj.insert("auth".to_string(), validated_value);
                    } else {
                        settings_obj.insert("config".to_string(), validated_value);
                    }
                }
            }
            AppType::Gemini => {
                updated_provider.settings_config = validated_value;
            }
            AppType::OpenCode => {
                updated_provider.settings_config = validated_value;
            }
        }

        display_provider_summary(&updated_provider, app_type)?;

        let Some(confirm) = prompt_confirm(texts::confirm_save_changes(), false)? else {
            return Ok(());
        };

        if !confirm {
            tui_show_text(
                texts::edit_provider_menu(),
                vec![texts::cancelled().to_string()],
                app_type,
            )?;
            return Ok(());
        }

        let app = app_type.clone();
        let provider_id = id.to_string();
        run_with_tui_loading(
            texts::edit_provider_menu(),
            texts::syncing_to_live_config(),
            texts::cancelled(),
            move || {
                let state = get_state()?;
                ProviderService::update(&state, app.clone(), updated_provider)?;

                if matches!(app, AppType::OpenCode) {
                    ProviderService::sync_opencode_to_live(&state)?;
                } else {
                    ProviderService::switch(&state, app, &provider_id)?;
                }
                Ok(())
            },
        )?;

        let updated_message = texts::entity_updated_success(texts::entity_provider(), id);
        let sync_message = if matches!(app_type, AppType::OpenCode) {
            texts::synced_opencode_live_config()
        } else {
            texts::synced_live_config()
        };

        tui_show_text(
            texts::edit_provider_menu(),
            vec![
                updated_message.to_string(),
                texts::syncing_to_live_config().to_string(),
                sync_message.to_string(),
                texts::restart_note().to_string(),
            ],
            app_type,
        )?;

        break;
    }

    Ok(())
}

/// Helper function to prompt for retry
fn retry_prompt() -> Result<bool, AppError> {
    Ok(prompt_confirm(texts::retry_editing(), true)?.unwrap_or(false))
}

/// Open external editor for content editing
fn open_external_editor(initial_content: &str) -> Result<String, AppError> {
    run_with_tui_suspended(|| {
        edit::edit(initial_content)
            .map_err(|e| AppError::Message(format!("{}: {}", texts::editor_failed(), e)))
    })
}

/// Display provider summary (used by JSON editor)
fn display_provider_summary(
    provider: &crate::provider::Provider,
    app_type: &AppType,
) -> Result<(), AppError> {
    let mut lines = Vec::new();
    lines.push(format!("{}: {}", texts::id_label_colon(), provider.id));
    lines.push(format!(
        "{}: {}",
        texts::name_label_with_colon(),
        provider.name
    ));
    if let Some(url) = &provider.website_url {
        lines.push(format!("{}: {}", texts::url_label_colon(), url));
    }
    if let Some(notes) = &provider.notes {
        lines.push(format!("{}: {}", texts::notes_label_colon(), notes));
    }
    if let Some(sort_index) = provider.sort_index {
        lines.push(format!(
            "{}: {}",
            texts::sort_index_label_colon(),
            sort_index
        ));
    }
    if let Some(api_url) = extract_api_url(&provider.settings_config, app_type) {
        lines.push(format!("{}: {}", texts::api_url_label_colon(), api_url));
    }
    tui_show_text(texts::provider_summary(), lines, app_type)?;
    Ok(())
}

/// Claude 配置信息
#[derive(Default)]
struct ClaudeConfig {
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    haiku_model: Option<String>,
    sonnet_model: Option<String>,
    opus_model: Option<String>,
}

/// 提取 Claude 配置信息
fn extract_claude_config(settings_config: &serde_json::Value) -> ClaudeConfig {
    let env = settings_config.get("env").and_then(|v| v.as_object());

    if let Some(env) = env {
        ClaudeConfig {
            api_key: env
                .get("ANTHROPIC_AUTH_TOKEN")
                .or_else(|| env.get("ANTHROPIC_API_KEY"))
                .and_then(|v| v.as_str())
                .map(mask_api_key),
            base_url: env
                .get("ANTHROPIC_BASE_URL")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            model: env
                .get("ANTHROPIC_MODEL")
                .and_then(|v| v.as_str())
                .map(simplify_model_name),
            haiku_model: env
                .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
                .and_then(|v| v.as_str())
                .map(simplify_model_name),
            sonnet_model: env
                .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
                .and_then(|v| v.as_str())
                .map(simplify_model_name),
            opus_model: env
                .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
                .and_then(|v| v.as_str())
                .map(simplify_model_name),
        }
    } else {
        ClaudeConfig::default()
    }
}

/// 将 API Key 脱敏显示（显示前8位 + ...）
fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}...", &key[..8])
    } else {
        key.to_string()
    }
}

/// 简化模型名称（去掉日期后缀）
/// 例如：claude-3-5-sonnet-20241022 -> claude-3-5-sonnet
fn simplify_model_name(name: &str) -> String {
    // 移除末尾的日期格式（8位数字）
    if let Some(pos) = name.rfind('-') {
        let suffix = &name[pos + 1..];
        if suffix.len() == 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
            return name[..pos].to_string();
        }
    }
    name.to_string()
}
