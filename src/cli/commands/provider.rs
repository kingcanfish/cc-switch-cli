use clap::Subcommand;
use std::sync::RwLock;

use crate::app_config::{AppType, MultiAppConfig};
use crate::cli::commands::provider_input::{
    current_timestamp, display_provider_summary, generate_provider_id, prompt_basic_fields,
    prompt_optional_fields, prompt_settings_config, prompt_settings_config_for_add, OptionalFields,
    ProviderAddMode,
};
use crate::cli::i18n::texts;
use crate::cli::interactive::utils::{
    prompt_confirm, prompt_select, prompt_text_with_help, run_tui_screen,
};
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::{is_tui_active, TextViewScreen};
use crate::cli::ui::{create_table, error, highlight, info, success, warning};
use crate::error::AppError;
use crate::provider::Provider;
use crate::services::{ProviderService, SpeedtestService};
use crate::store::AppState;

fn supports_official_provider(app_type: &AppType) -> bool {
    matches!(app_type, AppType::Codex)
}

fn tui_show_text(app_type: &AppType, title: &str, lines: Vec<String>) -> Result<(), AppError> {
    if !is_tui_active() {
        return Ok(());
    }
    let accent = accent_color(app_type);
    let mut screen = TextViewScreen::new(title, lines, texts::press_enter(), accent);
    run_tui_screen(title, &mut screen)?;
    Ok(())
}

#[derive(Subcommand)]
pub enum ProviderCommand {
    /// List all providers
    List,
    /// Show current provider
    Current,
    /// Switch to a provider
    Switch {
        /// Provider ID to switch to
        id: String,
    },
    /// Add a new provider (interactive)
    Add,
    /// Edit a provider
    Edit {
        /// Provider ID to edit
        id: String,
    },
    /// Delete a provider
    Delete {
        /// Provider ID to delete
        id: String,
    },
    /// Duplicate a provider
    Duplicate {
        /// Provider ID to duplicate
        id: String,
    },
    /// Test provider endpoint speed
    Speedtest {
        /// Provider ID to test
        id: String,
    },
}

pub fn execute(cmd: ProviderCommand, app: Option<AppType>) -> Result<(), AppError> {
    let app_type = app.unwrap_or(AppType::Claude);

    match cmd {
        ProviderCommand::List => list_providers(app_type),
        ProviderCommand::Current => show_current(app_type),
        ProviderCommand::Switch { id } => switch_provider(app_type, &id),
        ProviderCommand::Add => add_provider(app_type),
        ProviderCommand::Edit { id } => edit_provider(app_type, &id),
        ProviderCommand::Delete { id } => delete_provider(app_type, &id),
        ProviderCommand::Duplicate { id } => duplicate_provider(app_type, &id),
        ProviderCommand::Speedtest { id } => speedtest_provider(app_type, &id),
    }
}

fn get_state() -> Result<AppState, AppError> {
    let config = MultiAppConfig::load()?;
    Ok(AppState {
        config: RwLock::new(config),
    })
}

fn list_providers(app_type: AppType) -> Result<(), AppError> {
    let state = get_state()?;
    let app_str = app_type.as_str().to_string();
    let providers = ProviderService::list(&state, app_type.clone())?;
    let current_id = ProviderService::current(&state, app_type.clone())?;

    if providers.is_empty() {
        println!("{}", info(texts::no_providers()));
        println!("{}", texts::no_providers_hint());
        return Ok(());
    }

    // 创建表格
    let mut table = create_table();
    table.set_header(vec![
        "",
        texts::id_label(),
        texts::name_display_label(),
        texts::api_url_label_colon(),
    ]);

    // 按创建时间排序
    let mut provider_list: Vec<_> = providers.into_iter().collect();
    provider_list.sort_by(|(_, a), (_, b)| {
        // 先按 sort_index，再按创建时间
        match (a.sort_index, b.sort_index) {
            (Some(idx_a), Some(idx_b)) => idx_a.cmp(&idx_b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.created_at.cmp(&b.created_at),
        }
    });

    for (id, provider) in provider_list {
        let current_marker = if id == current_id { "✓" } else { " " };
        let api_url = extract_api_url(&provider.settings_config, &app_type)
            .unwrap_or_else(|| texts::not_applicable().to_string());

        table.add_row(vec![
            current_marker.to_string(),
            id.clone(),
            provider.name.clone(),
            api_url,
        ]);
    }

    println!("{}", table);
    println!("\n{} {}: {}", info("ℹ"), texts::application(), app_str);
    if matches!(app_type, AppType::OpenCode) {
        println!("{}", info(texts::opencode_additive_mode_notice()));
    } else {
        println!(
            "{} {}: {}",
            info("→"),
            texts::active(),
            highlight(&current_id)
        );
    }

    Ok(())
}

fn show_current(app_type: AppType) -> Result<(), AppError> {
    if matches!(app_type, AppType::OpenCode) {
        println!("{}", info(texts::opencode_no_current_provider()));
        return Ok(());
    }
    let state = get_state()?;
    let current_id = ProviderService::current(&state, app_type.clone())?;
    let providers = ProviderService::list(&state, app_type.clone())?;

    let provider = providers
        .get(&current_id)
        .ok_or_else(|| AppError::Message(texts::provider_not_found(&current_id)))?;

    println!("{}", highlight(texts::current_provider_details()));
    println!("{}", "═".repeat(60));

    // 基本信息
    println!("\n{}", highlight(texts::basic_info_section_header()));
    println!("  {}:       {}", texts::id_label(), current_id);
    println!(
        "  {}:     {}",
        texts::name_label_with_colon(),
        provider.name
    );
    println!(
        "  {}:     {}",
        texts::app_label_with_colon(),
        app_type.as_str()
    );

    // 仅 Claude 应用显示详细配置
    if matches!(app_type, AppType::Claude) {
        let config = extract_claude_config(&provider.settings_config);

        // API 配置
        println!("\n{}", highlight(texts::api_config_section_header()));
        println!(
            "  {}: {}",
            texts::base_url_display_label(),
            config
                .base_url
                .unwrap_or_else(|| texts::not_applicable().to_string())
        );
        println!(
            "  {}:  {}",
            texts::api_key_display_label(),
            config
                .api_key
                .unwrap_or_else(|| texts::not_applicable().to_string())
        );

        // 模型配置
        println!("\n{}", highlight(texts::model_config_section_header()));
        println!(
            "  {}:   {}",
            texts::main_model_label_with_colon(),
            config
                .model
                .unwrap_or_else(|| texts::default_model_display().to_string())
        );
        println!(
            "  {}:    {}",
            texts::haiku_model_display(),
            config
                .haiku_model
                .unwrap_or_else(|| texts::default_model_display().to_string())
        );
        println!(
            "  {}:   {}",
            texts::sonnet_model_display(),
            config
                .sonnet_model
                .unwrap_or_else(|| texts::default_model_display().to_string())
        );
        println!(
            "  {}:     {}",
            texts::opus_model_display(),
            config
                .opus_model
                .unwrap_or_else(|| texts::default_model_display().to_string())
        );
    } else {
        // Codex/Gemini 应用只显示 API URL
        println!("\n{}", highlight(texts::api_config_section_header()));
        let api_url = extract_api_url(&provider.settings_config, &app_type)
            .unwrap_or_else(|| texts::not_applicable().to_string());
        println!("  {}:  {}", texts::api_url_label_colon(), api_url);
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}

fn switch_provider(app_type: AppType, id: &str) -> Result<(), AppError> {
    if matches!(app_type, AppType::OpenCode) {
        return Err(AppError::Message(
            texts::opencode_switch_not_supported().to_string(),
        ));
    }
    let state = get_state()?;
    let app_str = app_type.as_str().to_string();

    // 检查 provider 是否存在
    let providers = ProviderService::list(&state, app_type.clone())?;
    if !providers.contains_key(id) {
        return Err(AppError::Message(texts::provider_not_found(id)));
    }

    // 执行切换
    ProviderService::switch(&state, app_type, id)?;

    println!("{}", success(&texts::switched_to_provider(id)));
    println!(
        "{}",
        info(&format!("  {}: {}", texts::application(), app_str))
    );
    println!("\n{}", info(texts::restart_note()));

    Ok(())
}

fn delete_provider(app_type: AppType, id: &str) -> Result<(), AppError> {
    let state = get_state()?;

    // 检查是否是当前 provider
    if !matches!(app_type, AppType::OpenCode) {
        let current_id = ProviderService::current(&state, app_type.clone())?;
        if id == current_id {
            return Err(AppError::Message(
                texts::cannot_delete_current_provider().to_string(),
            ));
        }
    }

    // 确认删除
    let Some(confirm) = prompt_confirm(&texts::confirm_delete(id), false)? else {
        if is_tui_active() {
            tui_show_text(
                &app_type,
                texts::delete_provider(),
                vec![texts::cancelled().to_string()],
            )?;
        } else {
            println!("{}", info(texts::cancelled()));
        }
        return Ok(());
    };

    if !confirm {
        if is_tui_active() {
            tui_show_text(
                &app_type,
                texts::delete_provider(),
                vec![texts::cancelled().to_string()],
            )?;
        } else {
            println!("{}", info(texts::cancelled()));
        }
        return Ok(());
    }

    // 执行删除
    ProviderService::delete(&state, app_type.clone(), id)?;

    if is_tui_active() {
        tui_show_text(
            &app_type,
            texts::delete_provider(),
            vec![texts::deleted_provider(id)],
        )?;
    } else {
        println!("{}", success(&texts::deleted_provider(id)));
    }

    Ok(())
}

fn add_provider(app_type: AppType) -> Result<(), AppError> {
    // Disable bracketed paste mode to work around inquire dropping paste events
    crate::cli::terminal::disable_bracketed_paste_mode_best_effort();

    if !is_tui_active() {
        println!("{}", highlight(texts::add_provider()));
        println!("{}", "=".repeat(50));
    }

    let add_mode = if supports_official_provider(&app_type) {
        let choices = vec![
            texts::add_official_provider(),
            texts::add_third_party_provider(),
        ];
        let Some(selected) = prompt_select(texts::select_provider_add_mode(), choices.clone())?
        else {
            if is_tui_active() {
                tui_show_text(
                    &app_type,
                    texts::add_provider(),
                    vec![texts::cancelled().to_string()],
                )?;
            } else {
                println!("{}", info(texts::cancelled()));
            }
            return Ok(());
        };
        if selected == texts::add_official_provider() {
            ProviderAddMode::Official
        } else {
            ProviderAddMode::ThirdParty
        }
    } else {
        ProviderAddMode::ThirdParty
    };

    // 1. 加载配置和状态
    let state = AppState {
        config: RwLock::new(MultiAppConfig::load()?),
    };
    let config = state.config.read().unwrap();
    let manager = config
        .get_manager(&app_type)
        .ok_or_else(|| AppError::Message(texts::app_config_not_found(app_type.as_str())))?;
    let existing_ids: Vec<String> = manager.providers.keys().cloned().collect();
    drop(config);

    // 2. 收集基本字段
    let (name, website_url) = match (app_type.clone(), add_mode) {
        (AppType::Codex, ProviderAddMode::Official) => {
            let Some(name) =
                prompt_text_with_help(texts::provider_name_label(), texts::provider_name_help())?
            else {
                if is_tui_active() {
                    tui_show_text(
                        &app_type,
                        texts::add_provider(),
                        vec![texts::cancelled().to_string()],
                    )?;
                } else {
                    println!("{}", info(texts::cancelled()));
                }
                return Ok(());
            };
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(AppError::InvalidInput(
                    texts::provider_name_empty_error().to_string(),
                ));
            }
            (name, Some("https://openai.com".to_string()))
        }
        _ => prompt_basic_fields(None)?,
    };
    let id = generate_provider_id(&name, &existing_ids);
    if is_tui_active() {
        tui_show_text(
            &app_type,
            texts::add_provider(),
            vec![texts::generated_id_message(&id)],
        )?;
    } else {
        println!("{}", info(&texts::generated_id_message(&id)));
    }

    // 3. 收集配置
    let settings_config = prompt_settings_config_for_add(&app_type, add_mode)?;

    // 4. 询问是否配置可选字段
    let Some(configure_optional) =
        prompt_confirm(texts::configure_optional_fields_prompt(), false)?
    else {
        if is_tui_active() {
            tui_show_text(
                &app_type,
                texts::add_provider(),
                vec![texts::cancelled().to_string()],
            )?;
        } else {
            println!("{}", info(texts::cancelled()));
        }
        return Ok(());
    };
    let optional = if configure_optional {
        prompt_optional_fields(None)?
    } else {
        OptionalFields::default()
    };

    // 5. 构建 Provider 对象
    let provider = Provider {
        id: id.clone(),
        name,
        settings_config,
        website_url,
        category: None,
        created_at: Some(current_timestamp()),
        sort_index: optional.sort_index,
        notes: optional.notes,
        icon: None,
        icon_color: None,
        meta: None,
    };

    // 6. 显示摘要并确认
    display_provider_summary(&provider, &app_type)?;
    let Some(confirm) = prompt_confirm(
        &texts::confirm_create_entity(texts::entity_provider()),
        false,
    )?
    else {
        println!("{}", info(texts::cancelled()));
        return Ok(());
    };
    if !confirm {
        if is_tui_active() {
            tui_show_text(
                &app_type,
                texts::add_provider(),
                vec![texts::cancelled().to_string()],
            )?;
        } else {
            println!("{}", info(texts::cancelled()));
        }
        return Ok(());
    }

    // 7. 调用 Service 层
    ProviderService::add(&state, app_type.clone(), provider)?;

    // 8. 成功消息
    if is_tui_active() {
        tui_show_text(
            &app_type,
            texts::add_provider(),
            vec![texts::entity_added_success(texts::entity_provider(), &id)],
        )?;
    } else {
        println!(
            "\n{}",
            success(&texts::entity_added_success(texts::entity_provider(), &id))
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_official_provider_is_codex_only() {
        assert!(supports_official_provider(&AppType::Codex));
        assert!(!supports_official_provider(&AppType::Claude));
        assert!(!supports_official_provider(&AppType::Gemini));
        assert!(!supports_official_provider(&AppType::OpenCode));
    }
}

fn edit_provider(app_type: AppType, id: &str) -> Result<(), AppError> {
    // Disable bracketed paste mode to work around inquire dropping paste events
    crate::cli::terminal::disable_bracketed_paste_mode_best_effort();

    if is_tui_active() {
        tui_show_text(
            &app_type,
            texts::edit_provider_menu(),
            vec![format!("{} {}", texts::id_label_colon(), id)],
        )?;
    } else {
        println!("{}", highlight(texts::edit_provider_menu()));
        println!("{}: {}", texts::id_label_colon(), id);
        println!("{}", "=".repeat(50));
    }

    // 1. 加载并验证供应商存在
    let state = AppState {
        config: RwLock::new(MultiAppConfig::load()?),
    };
    let config = state.config.read().unwrap();
    let manager = config
        .get_manager(&app_type)
        .ok_or_else(|| AppError::Message(texts::app_config_not_found(app_type.as_str())))?;
    let original = manager
        .providers
        .get(id)
        .ok_or_else(|| {
            let msg = texts::entity_not_found(texts::entity_provider(), id);
            AppError::localized("provider.not_found", msg.clone(), msg)
        })?
        .clone();
    let is_current = manager.current == id;
    drop(config);

    // 2. 显示当前配置
    if is_tui_active() {
        tui_show_text(&app_type, texts::current_config_header(), Vec::new())?;
    } else {
        println!("\n{}", highlight(texts::current_config_header()));
    }
    display_provider_summary(&original, &app_type)?;
    if !is_tui_active() {
        println!();
    }

    // 3. 全量编辑各字段（使用当前值作为默认）
    if is_tui_active() {
        tui_show_text(
            &app_type,
            texts::edit_provider_menu(),
            vec![texts::edit_fields_instruction().to_string()],
        )?;
    } else {
        println!("{}", info(texts::edit_fields_instruction()));
    }

    // 调用 prompt_basic_fields 来处理基本字段输入（自动使用 initial_value）
    let (name, website_url) = prompt_basic_fields(Some(&original))?;

    // 4. 询问是否修改配置
    let Some(modify_config) = prompt_confirm(texts::modify_provider_config_prompt(), false)? else {
        if is_tui_active() {
            tui_show_text(
                &app_type,
                texts::edit_provider_menu(),
                vec![texts::cancelled().to_string()],
            )?;
        } else {
            println!("{}", info(texts::cancelled()));
        }
        return Ok(());
    };
    let settings_config = if modify_config {
        prompt_settings_config(&app_type, Some(&original.settings_config))?
    } else {
        original.settings_config.clone()
    };

    // 5. 询问是否修改可选字段
    let Some(modify_optional) = prompt_confirm(texts::modify_optional_fields_prompt(), false)?
    else {
        if is_tui_active() {
            tui_show_text(
                &app_type,
                texts::edit_provider_menu(),
                vec![texts::cancelled().to_string()],
            )?;
        } else {
            println!("{}", info(texts::cancelled()));
        }
        return Ok(());
    };
    let optional = if modify_optional {
        prompt_optional_fields(Some(&original))?
    } else {
        OptionalFields::from_provider(&original)
    };

    // 6. 构建更新后的 Provider（保留 meta 和 created_at）
    let updated = Provider {
        id: id.to_string(),
        name: name.trim().to_string(),
        settings_config,
        website_url,
        category: None,
        created_at: original.created_at,
        sort_index: optional.sort_index,
        notes: optional.notes,
        icon: None,
        icon_color: None,
        meta: original.meta, // 保留元数据
    };

    // 7. 显示修改摘要并确认
    if is_tui_active() {
        tui_show_text(&app_type, texts::updated_config_header(), Vec::new())?;
    } else {
        println!("\n{}", highlight(texts::updated_config_header()));
    }
    display_provider_summary(&updated, &app_type)?;
    let Some(confirm) = prompt_confirm(
        &texts::confirm_update_entity(texts::entity_provider()),
        false,
    )?
    else {
        if is_tui_active() {
            tui_show_text(
                &app_type,
                texts::edit_provider_menu(),
                vec![texts::cancelled().to_string()],
            )?;
        } else {
            println!("{}", info(texts::cancelled()));
        }
        return Ok(());
    };
    if !confirm {
        if is_tui_active() {
            tui_show_text(
                &app_type,
                texts::edit_provider_menu(),
                vec![texts::cancelled().to_string()],
            )?;
        } else {
            println!("{}", info(texts::cancelled()));
        }
        return Ok(());
    }

    // 8. 调用 Service 层
    ProviderService::update(&state, app_type.clone(), updated)?;

    // 9. 成功消息
    if is_tui_active() {
        let mut lines = vec![texts::entity_updated_success(texts::entity_provider(), id)];
        if is_current {
            lines.push(texts::current_provider_synced_warning().to_string());
        }
        tui_show_text(&app_type, texts::edit_provider_menu(), lines)?;
    } else {
        println!(
            "\n{}",
            success(&texts::entity_updated_success(texts::entity_provider(), id))
        );
        if is_current {
            println!("{}", warning(texts::current_provider_synced_warning()));
        }
    }

    Ok(())
}

fn duplicate_provider(_app_type: AppType, id: &str) -> Result<(), AppError> {
    println!("{}", info(&texts::duplicating_provider(id)));
    println!("{}", error(texts::provider_duplication_not_implemented()));
    Ok(())
}

fn speedtest_provider(app_type: AppType, id: &str) -> Result<(), AppError> {
    let state = get_state()?;

    // Get provider by ID
    let providers = ProviderService::list(&state, app_type.clone())?;
    let provider = providers
        .get(id)
        .ok_or_else(|| AppError::Message(texts::provider_not_found(id)))?;

    // Extract API URL
    let api_url = extract_api_url(&provider.settings_config, &app_type)
        .ok_or_else(|| AppError::Message(texts::no_api_url_configured().to_string()))?;

    println!("{}", info(&texts::testing_provider(&provider.name)));
    println!(
        "{}",
        info(&format!("{}: {}", texts::endpoint_label_colon(), api_url))
    );
    println!();

    // Run speedtest asynchronously
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| AppError::Message(texts::async_runtime_create_failed(&e.to_string())))?;

    let results = runtime
        .block_on(async { SpeedtestService::test_endpoints(vec![api_url.clone()], None).await })?;

    // Display results
    if let Some(result) = results.first() {
        let mut table = create_table();
        table.set_header(vec![
            texts::endpoint_label_colon(),
            texts::latency_label(),
            texts::status_label(),
        ]);

        let latency_str = if let Some(latency) = result.latency {
            format!("{} ms", latency)
        } else if result.error.is_some() {
            texts::speedtest_failed().to_string()
        } else {
            texts::speedtest_timeout().to_string()
        };

        let status_str = result
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| texts::not_applicable().to_string());

        table.add_row(vec![result.url.clone(), latency_str, status_str]);

        println!("{}", table);

        // Show error details if any
        if let Some(err) = &result.error {
            println!(
                "\n{}",
                error(&format!("{}: {}", texts::error_prefix(), err))
            );
        } else if result.latency.is_some() {
            println!("\n{}", success(texts::speedtest_completed_success()));
        }
    }

    Ok(())
}

fn extract_api_url(settings_config: &serde_json::Value, app_type: &AppType) -> Option<String> {
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
