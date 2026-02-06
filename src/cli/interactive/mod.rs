mod config;
mod mcp;
mod prompts;
mod provider;
mod settings;
mod skills;
mod tui;
pub mod utils;

use std::io::IsTerminal;

use crate::app_config::AppType;
use crate::cli::i18n::texts;
use crate::cli::tui as tui_runtime;
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::TextViewScreen;
use crate::cli::ui::current_tui_app;
use crate::cli::ui::set_tui_theme_app;
use crate::error::AppError;
use crate::services::{McpService, PromptService, ProviderService};
use crate::settings as app_settings;

use utils::{init_tui_session, prompt_select, run_tui_screen, shutdown_tui_session};

pub fn run(app: Option<AppType>) -> Result<(), AppError> {
    // Disable bracketed paste mode to work around inquire dropping paste events
    crate::cli::terminal::disable_bracketed_paste_mode_best_effort();

    let mut app_type = app
        .as_ref()
        .cloned()
        .unwrap_or_else(app_settings::default_app);
    set_tui_theme_app(Some(app_type.clone()));

    if let Some(app_type) = app.as_ref() {
        if let Err(err) = app_settings::set_last_app(app_type) {
            log::warn!("Failed to persist last app: {}", err);
        }
    }

    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(AppError::Message(
            "Interactive mode requires a TTY with TUI support".to_string(),
        ));
    }

    tui_runtime::set_tui_active(true);
    init_tui_session()?;

    let result = (|| -> Result<(), AppError> {
        loop {
            let outcome = tui::show_main_menu_tui(app_type.clone())?;
            app_type = outcome.app_type;
            let choice = outcome.choice;

            set_tui_theme_app(Some(app_type.clone()));

            match choice {
                MainMenuChoice::ManageProviders => {
                    if let Err(e) = provider::manage_providers_menu(&app_type) {
                        show_interactive_error(&e)?;
                    }
                }
                MainMenuChoice::ManageMCP => {
                    if let Err(e) = mcp::manage_mcp_menu(&app_type) {
                        show_interactive_error(&e)?;
                    }
                }
                MainMenuChoice::ManagePrompts => {
                    if let Err(e) = prompts::manage_prompts_menu(&app_type) {
                        show_interactive_error(&e)?;
                    }
                }
                MainMenuChoice::ManageSkills => {
                    if let Err(e) = skills::manage_skills_menu() {
                        show_interactive_error(&e)?;
                    }
                }
                MainMenuChoice::ManageConfig => {
                    if let Err(e) = config::manage_config_menu(&app_type) {
                        show_interactive_error(&e)?;
                    }
                }
                MainMenuChoice::ViewCurrentConfig => {
                    if let Err(e) = view_current_config(&app_type) {
                        show_interactive_error(&e)?;
                    }
                }
                MainMenuChoice::SwitchApp => {
                    if let Ok(new_app) = select_app() {
                        app_type = new_app;
                        if let Err(err) = app_settings::set_last_app(&app_type) {
                            log::warn!("Failed to persist last app: {}", err);
                        }
                    }
                }
                MainMenuChoice::Settings => {
                    if let Err(e) = settings::settings_menu() {
                        show_interactive_error(&e)?;
                    }
                }
                MainMenuChoice::Exit => {
                    break;
                }
            }
        }

        Ok(())
    })();

    shutdown_tui_session();
    tui_runtime::set_tui_active(false);
    set_tui_theme_app(None);

    result
}

#[derive(Debug, Clone)]
enum MainMenuChoice {
    ManageProviders,
    ManageMCP,
    ManagePrompts,
    ManageSkills,
    ManageConfig,
    ViewCurrentConfig,
    SwitchApp,
    Settings,
    Exit,
}

impl std::fmt::Display for MainMenuChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManageProviders => write!(f, "{}", texts::menu_manage_providers()),
            Self::ManageMCP => write!(f, "{}", texts::menu_manage_mcp()),
            Self::ManagePrompts => write!(f, "{}", texts::menu_manage_prompts()),
            Self::ManageSkills => write!(f, "{}", texts::menu_manage_skills()),
            Self::ManageConfig => write!(f, "{}", texts::menu_manage_config()),
            Self::ViewCurrentConfig => write!(f, "{}", texts::menu_view_config()),
            Self::SwitchApp => write!(f, "{}", texts::menu_switch_app()),
            Self::Settings => write!(f, "{}", texts::menu_settings()),
            Self::Exit => write!(f, "{}", texts::menu_exit()),
        }
    }
}

fn select_app() -> Result<AppType, AppError> {
    let apps = vec![
        AppType::Claude,
        AppType::Codex,
        AppType::Gemini,
        AppType::OpenCode,
    ];

    let Some(app) = prompt_select(texts::select_application(), apps)? else {
        return Err(AppError::Message("Selection cancelled".to_string()));
    };

    tui_show_text(
        texts::select_application(),
        vec![texts::switched_to_app(app.as_str()).to_string()],
    )?;

    Ok(app)
}

fn view_current_config(app_type: &AppType) -> Result<(), AppError> {
    use utils::get_state;

    let app = app_type.clone();
    let state = get_state()?;
    let mut lines = Vec::new();

    let current_provider = ProviderService::current(&state, app.clone())?;
    let providers = ProviderService::list(&state, app.clone())?;
    if let Some(provider) = providers.get(&current_provider) {
        lines.push(texts::provider_label().to_string());
        lines.push(format!(
            "  {}     {}",
            texts::name_label_with_colon(),
            provider.name
        ));
        let api_url = provider::extract_api_url(&provider.settings_config, &app)
            .unwrap_or_else(|| "N/A".to_string());
        lines.push(format!("  API URL:  {}", api_url));
    }

    let mcp_servers = McpService::get_all_servers(&state)?;
    let enabled_count = mcp_servers
        .values()
        .filter(|s| s.apps.is_enabled_for(&app))
        .count();
    lines.push(String::new());
    lines.push(texts::mcp_servers_label().to_string());
    lines.push(format!("  {}:     {}", texts::total(), mcp_servers.len()));
    lines.push(format!("  {}:     {}", texts::enabled(), enabled_count));

    let prompts = PromptService::get_prompts(&state, app)?;
    let active_prompt = prompts.iter().find(|(_, p)| p.enabled);
    lines.push(String::new());
    lines.push(texts::prompts_label().to_string());
    lines.push(format!("  {}:     {}", texts::total(), prompts.len()));
    if let Some((_, p)) = active_prompt {
        lines.push(format!("  {}:     {}", texts::active(), p.name));
    } else {
        lines.push(format!("  {}:     {}", texts::active(), texts::none()));
    }

    tui_show_text(texts::current_configuration(), lines)?;

    Ok(())
}

fn show_interactive_error(err: &AppError) -> Result<(), AppError> {
    let message = format!("{}: {}", texts::error_prefix(), err);
    tui_show_text(texts::error_prefix(), vec![message])
}

fn tui_show_text(title: &str, lines: Vec<String>) -> Result<(), AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = TextViewScreen::new(title, lines, texts::press_enter(), accent);
    run_tui_screen(title, &mut screen)?;
    Ok(())
}
