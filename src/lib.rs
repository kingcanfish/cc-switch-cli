// Core modules
mod app_config;
mod claude_mcp;
mod codex_config;
mod config;
mod error;
mod gemini_config;
mod gemini_mcp;
mod mcp;
mod prompt;
mod prompt_files;
mod provider;
mod provider_defaults;
mod services;
mod settings;
mod store;
mod usage_script;

// CLI module
pub mod cli;

// Public exports
pub use app_config::{AppType, McpApps, McpServer, MultiAppConfig};
pub use codex_config::{get_codex_auth_path, get_codex_config_path, write_codex_live_atomic};
pub use config::{get_claude_mcp_path, get_claude_settings_path, read_json_file};
pub use error::AppError;
pub use mcp::{
    import_from_claude, import_from_codex, import_from_gemini, remove_server_from_claude,
    remove_server_from_codex, remove_server_from_gemini, sync_enabled_to_claude,
    sync_enabled_to_codex, sync_enabled_to_gemini, sync_single_server_to_claude,
    sync_single_server_to_codex, sync_single_server_to_gemini,
};
pub use provider::{Provider, ProviderMeta};
pub use services::{
    ConfigService, EndpointLatency, McpService, PromptService, ProviderService, SkillService,
    SpeedtestService,
};
pub use settings::{update_settings, AppSettings};
pub use store::AppState;
