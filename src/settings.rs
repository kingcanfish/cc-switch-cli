use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{OnceLock, RwLock};

use crate::app_config::AppType;
use crate::error::AppError;

/// 自定义端点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEndpoint {
    pub url: String,
    pub added_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAuthSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<SecurityAuthSettings>,
}

/// 应用设置结构，允许覆盖默认配置目录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct AppSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySettings>,
    /// Claude 自定义端点列表
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_endpoints_claude: HashMap<String, CustomEndpoint>,
    /// Codex 自定义端点列表
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_endpoints_codex: HashMap<String, CustomEndpoint>,
}

const SETTINGS_DIR: &str = ".cc-switch-cli";

impl AppSettings {
    fn settings_path() -> PathBuf {
        // settings.json 必须使用固定路径，不能被 app_config_dir 覆盖
        // 否则会造成循环依赖：读取 settings 需要知道路径，但路径在 settings 中
        dirs::home_dir()
            .expect("无法获取用户主目录")
            .join(SETTINGS_DIR)
            .join("settings.json")
    }

    fn read_settings_at(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        match serde_json::from_str::<AppSettings>(&content) {
            Ok(mut settings) => {
                settings.normalize_paths();
                Some(settings)
            }
            Err(err) => {
                log::warn!(
                    "解析设置文件失败，忽略该文件。路径: {}, 错误: {}",
                    path.display(),
                    err
                );
                None
            }
        }
    }

    fn normalize_paths(&mut self) {
        self.claude_config_dir = self
            .claude_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.codex_config_dir = self
            .codex_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.gemini_config_dir = self
            .gemini_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.opencode_config_dir = self
            .opencode_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.language = self
            .language
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| matches!(*s, "en" | "zh"))
            .map(|s| s.to_string());

        self.last_app = self
            .last_app
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| matches!(s.as_str(), "claude" | "codex" | "gemini"))
            .map(|s| s.to_string());
    }

    pub fn load() -> Self {
        let path = Self::settings_path();
        Self::read_settings_at(&path).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), AppError> {
        let mut normalized = self.clone();
        normalized.normalize_paths();
        let path = Self::settings_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let json = serde_json::to_string_pretty(&normalized)
            .map_err(|e| AppError::JsonSerialize { source: e })?;
        fs::write(&path, json).map_err(|e| AppError::io(&path, e))?;
        Ok(())
    }
}

fn settings_store() -> &'static RwLock<AppSettings> {
    static STORE: OnceLock<RwLock<AppSettings>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(AppSettings::load()))
}

fn resolve_override_path(raw: &str) -> PathBuf {
    if raw == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    } else if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    } else if let Some(stripped) = raw.strip_prefix("~\\") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }

    PathBuf::from(raw)
}

pub fn get_settings() -> AppSettings {
    settings_store().read().expect("读取设置锁失败").clone()
}

pub fn update_settings(mut new_settings: AppSettings) -> Result<(), AppError> {
    new_settings.normalize_paths();
    new_settings.save()?;

    let mut guard = settings_store().write().expect("写入设置锁失败");
    *guard = new_settings;
    Ok(())
}

pub fn ensure_security_auth_selected_type(selected_type: &str) -> Result<(), AppError> {
    let mut settings = get_settings();
    let current = settings
        .security
        .as_ref()
        .and_then(|sec| sec.auth.as_ref())
        .and_then(|auth| auth.selected_type.as_deref());

    if current == Some(selected_type) {
        return Ok(());
    }

    let mut security = settings.security.unwrap_or_default();
    let mut auth = security.auth.unwrap_or_default();
    auth.selected_type = Some(selected_type.to_string());
    security.auth = Some(auth);
    settings.security = Some(security);

    update_settings(settings)
}

pub fn default_app() -> AppType {
    get_settings()
        .last_app
        .as_deref()
        .and_then(|value| AppType::from_str(value).ok())
        .unwrap_or(AppType::Claude)
}

pub fn set_last_app(app: &AppType) -> Result<(), AppError> {
    let mut settings = get_settings();
    let next = app.as_str().to_string();

    if settings.last_app.as_deref() == Some(next.as_str()) {
        return Ok(());
    }

    settings.last_app = Some(next);
    update_settings(settings)
}

pub fn get_claude_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .claude_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_codex_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .codex_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_gemini_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .gemini_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_opencode_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .opencode_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}
