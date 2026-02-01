mod config;
mod mcp;
mod prompts;
mod provider;
mod settings;
mod skills;
mod utils;

use std::io::IsTerminal;

use crate::app_config::AppType;
use crate::cli::i18n::texts;
use crate::cli::ui::{error, highlight, info, set_tui_theme_app, success};
use crate::error::AppError;
use crate::services::{McpService, PromptService, ProviderService};
use crate::settings as app_settings;

use utils::{
    app_switch_direction_from_key, clear_screen, cycle_app_type, pause, prompt_select,
    prompt_text_with_default,
};

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

    loop {
        match show_main_menu(&mut app_type)? {
            MainMenuChoice::ManageProviders => {
                if let Err(e) = provider::manage_providers_menu(&app_type) {
                    println!("\n{}", error(&format!("{}: {}", texts::error_prefix(), e)));
                    pause();
                }
            }
            MainMenuChoice::ManageMCP => {
                if let Err(e) = mcp::manage_mcp_menu(&app_type) {
                    println!("\n{}", error(&format!("{}: {}", texts::error_prefix(), e)));
                    pause();
                }
            }
            MainMenuChoice::ManagePrompts => {
                if let Err(e) = prompts::manage_prompts_menu(&app_type) {
                    println!("\n{}", error(&format!("{}: {}", texts::error_prefix(), e)));
                    pause();
                }
            }
            MainMenuChoice::ManageSkills => {
                if let Err(e) = skills::manage_skills_menu() {
                    println!("\n{}", error(&format!("{}: {}", texts::error_prefix(), e)));
                    pause();
                }
            }
            MainMenuChoice::ManageConfig => {
                if let Err(e) = config::manage_config_menu(&app_type) {
                    println!("\n{}", error(&format!("{}: {}", texts::error_prefix(), e)));
                    pause();
                }
            }
            MainMenuChoice::ViewCurrentConfig => {
                if let Err(e) = view_current_config(&app_type) {
                    println!("\n{}", error(&format!("{}: {}", texts::error_prefix(), e)));
                    pause();
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
                    println!("\n{}", error(&format!("{}: {}", texts::error_prefix(), e)));
                    pause();
                }
            }
            MainMenuChoice::Exit => {
                clear_screen();
                println!("{}\n", success(texts::goodbye()));
                set_tui_theme_app(None);
                break;
            }
        }
    }

    Ok(())
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

fn print_welcome(app_type: &AppType) {
    println!("\n{}", "═".repeat(60));
    println!("{}", highlight(texts::welcome_title()));
    println!("{}", "═".repeat(60));
    println!(
        "{} {}: {}",
        info("📱"),
        texts::application(),
        highlight(app_type.as_str())
    );
    println!("{}", "─".repeat(60));
    println!();
}

fn show_main_menu(app_type: &mut AppType) -> Result<MainMenuChoice, AppError> {
    let choices = vec![
        MainMenuChoice::ManageProviders,
        MainMenuChoice::ManageMCP,
        MainMenuChoice::ManagePrompts,
        MainMenuChoice::ManageSkills,
        MainMenuChoice::ManageConfig,
        MainMenuChoice::ViewCurrentConfig,
        MainMenuChoice::SwitchApp,
        MainMenuChoice::Settings,
        MainMenuChoice::Exit,
    ];

    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Ok(
            prompt_select(&texts::main_menu_prompt(app_type.as_str()), choices)?
                .unwrap_or(MainMenuChoice::Exit),
        );
    }

    let term = console::Term::stdout();
    let mut selected_idx: usize = 0;
    let mut filter_query: Option<String> = None;

    loop {
        // Determine active filter query (non-empty trimmed string)
        let active_query = filter_query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty());

        // Filter choices based on query
        let visible_choices: Vec<MainMenuChoice> = if let Some(query) = active_query {
            let query_lower = query.to_lowercase();
            choices
                .iter()
                .filter(|choice| choice.to_string().to_lowercase().contains(&query_lower))
                .cloned()
                .collect()
        } else {
            choices.clone()
        };

        // Reset selection if out of bounds
        if visible_choices.is_empty() || selected_idx >= visible_choices.len() {
            selected_idx = 0;
        }

        // Render menu
        clear_screen();
        set_tui_theme_app(Some(app_type.clone()));
        print_welcome(app_type);

        println!("{}", texts::main_menu_prompt(app_type.as_str()));
        println!("{}", "─".repeat(60));

        // Show filter status if active
        if let Some(query) = active_query {
            println!("{}", info(&texts::main_menu_filtering(query)));
        }

        // Show menu items or no matches message
        if visible_choices.is_empty() {
            println!("  {}", info(texts::main_menu_no_matches()));
        } else {
            for (idx, choice) in visible_choices.iter().enumerate() {
                if idx == selected_idx {
                    println!("{} {}", highlight("➤"), highlight(&choice.to_string()));
                } else {
                    println!("  {}", choice);
                }
            }
        }

        println!("{}", "─".repeat(60));
        println!("{}", texts::main_menu_help());

        // Read keyboard input
        let key = term
            .read_key()
            .map_err(|e| AppError::Message(e.to_string()))?;

        // Handle app switching (left/right arrows)
        if let Some(direction) = app_switch_direction_from_key(&key) {
            *app_type = cycle_app_type(app_type, direction);
            if let Err(err) = app_settings::set_last_app(app_type) {
                log::warn!("Failed to persist last app: {}", err);
            }
            continue;
        }

        // Handle keyboard actions
        match key {
            console::Key::Char('/') => {
                // Enter search mode with current query as default
                clear_screen();
                let current_query = filter_query.as_deref().unwrap_or("");
                let query =
                    prompt_text_with_default(texts::main_menu_search_prompt(), current_query)?
                        .unwrap_or_default();
                let query = query.trim();

                if query.is_empty() {
                    filter_query = None;
                } else {
                    filter_query = Some(query.to_string());
                }

                selected_idx = 0;
            }
            console::Key::ArrowUp => {
                if !visible_choices.is_empty() {
                    selected_idx = selected_idx
                        .checked_sub(1)
                        .unwrap_or(visible_choices.len() - 1);
                }
            }
            console::Key::ArrowDown => {
                if !visible_choices.is_empty() {
                    selected_idx = (selected_idx + 1) % visible_choices.len();
                }
            }
            console::Key::Enter => {
                if !visible_choices.is_empty() {
                    return Ok(visible_choices[selected_idx].clone());
                }
            }
            console::Key::Escape => {
                // If filter is active, clear it; otherwise exit
                if filter_query
                    .as_deref()
                    .map(str::trim)
                    .filter(|q| !q.is_empty())
                    .is_some()
                {
                    filter_query = None;
                    selected_idx = 0;
                    continue;
                }
                return Ok(MainMenuChoice::Exit);
            }
            console::Key::Unknown => return Ok(MainMenuChoice::Exit),
            _ => {}
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

    println!("\n{}", success(&texts::switched_to_app(app.as_str())));
    pause();

    Ok(app)
}

fn view_current_config(app_type: &AppType) -> Result<(), AppError> {
    use utils::get_state;

    println!("\n{}", highlight(texts::current_configuration()));
    println!("{}", "═".repeat(60));

    let state = get_state()?;

    // Provider info
    let current_provider = ProviderService::current(&state, app_type.clone())?;
    let providers = ProviderService::list(&state, app_type.clone())?;
    if let Some(provider) = providers.get(&current_provider) {
        println!("\n{}", highlight(texts::provider_label()));
        println!("  名称:     {}", provider.name);
        let api_url = provider::extract_api_url(&provider.settings_config, app_type)
            .unwrap_or_else(|| "N/A".to_string());
        println!("  API URL:  {}", api_url);
    }

    // MCP servers count
    let mcp_servers = McpService::get_all_servers(&state)?;
    let enabled_count = mcp_servers
        .values()
        .filter(|s| s.apps.is_enabled_for(app_type))
        .count();
    println!("\n{}", highlight(texts::mcp_servers_label()));
    println!("  总计:     {}", mcp_servers.len());
    println!("  启用:     {}", enabled_count);

    // Prompts
    let prompts = PromptService::get_prompts(&state, app_type.clone())?;
    let active_prompt = prompts.iter().find(|(_, p)| p.enabled);
    println!("\n{}", highlight(texts::prompts_label()));
    println!("  总计:     {}", prompts.len());
    if let Some((_, p)) = active_prompt {
        println!("  活动:     {}", p.name);
    } else {
        println!("  活动:     {}", texts::none());
    }

    println!("\n{}", "─".repeat(60));
    pause();

    Ok(())
}
