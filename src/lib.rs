// Core modules
mod app_config;
mod config;
mod error;
mod mcp;
mod opencode_config;
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
pub use config::claude::{get_claude_mcp_path, get_claude_settings_path};
pub use config::codex::{get_codex_auth_path, get_codex_config_path, write_codex_live_atomic};
pub use config::read_json_file;
pub use error::AppError;
pub use mcp::{import_from, remove_server_from, sync_enabled_to, sync_single_server_to};
pub use provider::{Provider, ProviderMeta};
pub use services::{
    ConfigService, EndpointLatency, McpService, PromptService, ProviderService, SkillApps,
    SkillService, SpeedtestService,
};
pub use settings::{update_settings, AppSettings};
pub use store::AppState;
