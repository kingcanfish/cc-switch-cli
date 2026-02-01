use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// SSOT 模式：不再写供应商副本文件

/// 供应商结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    #[serde(rename = "settingsConfig")]
    pub settings_config: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sortIndex")]
    pub sort_index: Option<usize>,
    /// 备注信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// 供应商元数据（不写入 live 配置，仅存于 ~/.cc-switch-cli/config.json）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProviderMeta>,
    /// 图标名称（如 "openai", "anthropic"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// 图标颜色（Hex 格式，如 "#00A67E"）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "iconColor")]
    pub icon_color: Option<String>,
}

impl Provider {
    /// 从现有ID创建供应商
    pub fn with_id(
        id: String,
        name: String,
        settings_config: Value,
        website_url: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            settings_config,
            website_url,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
        }
    }
}

/// 供应商管理器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderManager {
    pub providers: HashMap<String, Provider>,
    pub current: String,
}

/// 用量查询脚本配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageScript {
    pub enabled: bool,
    pub language: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// 用量查询专用的 API Key（通用模板使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    /// 用量查询专用的 Base URL（通用和 NewAPI 模板使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    /// 访问令牌（用于需要登录的接口，NewAPI 模板使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "accessToken")]
    pub access_token: Option<String>,
    /// 用户ID（用于需要用户标识的接口，NewAPI 模板使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    /// 自动查询间隔（单位：分钟，0 表示禁用自动查询）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "autoQueryInterval")]
    pub auto_query_interval: Option<u64>,
}

/// 用量数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "planName")]
    pub plan_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isValid")]
    pub is_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "invalidMessage")]
    pub invalid_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// 用量查询结果（支持多套餐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<UsageData>>, // 支持返回多个套餐
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 供应商元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderMeta {
    /// 自定义端点列表（按 URL 去重存储）
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_endpoints: HashMap<String, crate::settings::CustomEndpoint>,
    /// 用量查询脚本配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_script: Option<UsageScript>,
    /// 合作伙伴标记（前端使用 isPartner，保持字段名一致）
    #[serde(rename = "isPartner", skip_serializing_if = "Option::is_none")]
    pub is_partner: Option<bool>,
    /// 合作伙伴促销 key，用于识别 PackyCode 等特殊供应商
    #[serde(
        rename = "partnerPromotionKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub partner_promotion_key: Option<String>,
}

// ============================================================================
// OpenCode 供应商配置结构
// ============================================================================

/// OpenCode 供应商的 settings_config 结构
///
/// OpenCode 使用 AI SDK 包名来指定供应商类型，与其他应用的配置格式不同。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeProviderConfig {
    /// AI SDK 包名，如 "@ai-sdk/openai-compatible", "@ai-sdk/anthropic"
    pub npm: String,

    /// 供应商名称（可选，用于显示）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// 供应商选项（API 密钥、基础 URL 等）
    #[serde(default)]
    pub options: OpenCodeProviderOptions,

    /// 模型定义映射
    #[serde(default)]
    pub models: HashMap<String, OpenCodeModel>,
}

impl Default for OpenCodeProviderConfig {
    fn default() -> Self {
        Self {
            npm: "@ai-sdk/openai-compatible".to_string(),
            name: None,
            options: OpenCodeProviderOptions::default(),
            models: HashMap::new(),
        }
    }
}

/// OpenCode 供应商选项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenCodeProviderOptions {
    /// API 基础 URL
    #[serde(rename = "baseURL", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// API 密钥（支持环境变量引用，如 "{env:API_KEY}"）
    #[serde(rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// 自定义请求头
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,

    /// 额外选项（timeout, setCacheKey 等）
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

/// OpenCode 模型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeModel {
    /// 模型显示名称
    pub name: String,

    /// 模型限制（上下文和输出 token 数）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<OpenCodeModelLimit>,

    /// 模型额外选项（provider 路由等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, Value>>,

    /// 额外字段（cost、modalities、thinking、variants 等）
    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

/// OpenCode 模型限制
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenCodeModelLimit {
    /// 上下文 token 限制
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<u64>,

    /// 输出 token 限制
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
}

impl ProviderManager {
    /// 获取所有供应商
    pub fn get_all_providers(&self) -> &HashMap<String, Provider> {
        &self.providers
    }
}
