use std::path::PathBuf;

use super::AppConfigPaths;

pub struct OpenCodeConfig;

impl AppConfigPaths for OpenCodeConfig {
    fn override_dir() -> Option<PathBuf> {
        crate::settings::get_opencode_override_dir()
    }

    fn default_dir() -> PathBuf {
        dirs::home_dir()
            .expect("无法获取用户主目录")
            .join(".config")
            .join("opencode")
    }
}

/// 获取 OpenCode 配置目录路径（支持设置覆盖）
pub fn get_opencode_dir() -> PathBuf {
    OpenCodeConfig::config_dir()
}

/// 获取 OpenCode 配置文件路径
pub fn get_opencode_config_path() -> PathBuf {
    get_opencode_dir().join("opencode.json")
}

/// 获取 OpenCode 环境变量文件路径（如存在）
#[allow(dead_code)]
pub fn get_opencode_env_path() -> PathBuf {
    get_opencode_dir().join(".env")
}
