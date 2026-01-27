use crate::app_config::MultiAppConfig;
use crate::error::AppError;
use std::sync::RwLock;

/// 全局应用状态
pub struct AppState {
    pub config: RwLock<MultiAppConfig>,
}

impl AppState {
    /// 创建新的应用状态
    /// 注意：仅在配置成功加载时返回；不会在失败时回退默认值。
    pub fn try_new() -> Result<Self, AppError> {
        let config = MultiAppConfig::load()?;
        Ok(Self {
            config: RwLock::new(config),
        })
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), AppError> {
        let config = self.config.read().map_err(AppError::from)?;

        config.save()
    }
}
