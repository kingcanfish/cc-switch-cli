use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::app_config::MultiAppConfig;
use crate::config::atomic_write;
use crate::config::gemini::get_gemini_settings_path;
use crate::error::AppError;

use super::{collect_enabled_servers, validate_server_spec, McpBackend};

pub struct GeminiBackend;

pub(crate) static GEMINI_BACKEND: GeminiBackend = GeminiBackend;

impl McpBackend for GeminiBackend {
    fn import_into(&self, config: &mut MultiAppConfig) -> Result<usize, AppError> {
        import_from(config)
    }

    fn sync_enabled(&self, config: &MultiAppConfig) -> Result<(), AppError> {
        sync_enabled(config)
    }

    fn sync_single(&self, id: &str, server_spec: &Value) -> Result<(), AppError> {
        sync_single(id, server_spec)
    }

    fn remove(&self, id: &str) -> Result<(), AppError> {
        remove(id)
    }
}

/// 获取 Gemini MCP 配置文件路径（~/.gemini/settings.json）
fn user_config_path() -> PathBuf {
    get_gemini_settings_path()
}

fn read_json_value(path: &Path) -> Result<Value, AppError> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    let value: Value = serde_json::from_str(&content).map_err(|e| AppError::json(path, e))?;
    Ok(value)
}

fn write_json_value(path: &Path, value: &Value) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let json =
        serde_json::to_string_pretty(value).map_err(|e| AppError::JsonSerialize { source: e })?;
    atomic_write(path, json.as_bytes())
}

/// 读取 Gemini MCP 配置文件的完整 JSON 文本
fn read_mcp_json() -> Result<Option<String>, AppError> {
    let path = user_config_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    Ok(Some(content))
}

/// 读取 Gemini settings.json 中的 mcpServers 映射
fn read_mcp_servers_map() -> Result<std::collections::HashMap<String, Value>, AppError> {
    let path = user_config_path();
    if !path.exists() {
        return Ok(std::collections::HashMap::new());
    }

    let root = read_json_value(&path)?;
    let servers = root
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    Ok(servers)
}

/// 将给定的启用 MCP 服务器映射写入到 Gemini settings.json 的 mcpServers 字段
/// 仅覆盖 mcpServers，其他字段保持不变
fn set_mcp_servers_map(servers: &std::collections::HashMap<String, Value>) -> Result<(), AppError> {
    let path = user_config_path();
    let mut root = if path.exists() {
        read_json_value(&path)?
    } else {
        serde_json::json!({})
    };

    // 构建 mcpServers 对象：移除 UI 辅助字段（enabled/source），仅保留实际 MCP 规范
    let mut out: Map<String, Value> = Map::new();
    for (id, spec) in servers.iter() {
        let mut obj = if let Some(map) = spec.as_object() {
            map.clone()
        } else {
            return Err(AppError::McpValidation(format!(
                "MCP 服务器 '{id}' 不是对象"
            )));
        };

        // 提取 server 字段（如果存在）
        if let Some(server_val) = obj.remove("server") {
            let server_obj = server_val.as_object().cloned().ok_or_else(|| {
                AppError::McpValidation(format!("MCP 服务器 '{id}' server 字段不是对象"))
            })?;
            obj = server_obj;
        }

        // Gemini CLI 格式转换：
        // - Gemini 不使用 "type" 字段（从字段名推断传输类型）
        // - HTTP 使用 "httpUrl" 字段，SSE 使用 "url" 字段
        let transport_type = obj.get("type").and_then(|v| v.as_str());
        if transport_type == Some("http") {
            // HTTP streaming: 将 "url" 重命名为 "httpUrl"
            if let Some(url_value) = obj.remove("url") {
                obj.insert("httpUrl".to_string(), url_value);
            }
        }
        // SSE 保持 "url" 字段不变

        // 移除 UI 辅助字段和 type 字段（Gemini 不需要）
        obj.remove("type");
        obj.remove("enabled");
        obj.remove("source");
        obj.remove("id");
        obj.remove("name");
        obj.remove("description");
        obj.remove("tags");
        obj.remove("homepage");
        obj.remove("docs");

        out.insert(id.clone(), Value::Object(obj));
    }

    {
        let obj = root
            .as_object_mut()
            .ok_or_else(|| AppError::Config("~/.gemini/settings.json 根必须是对象".into()))?;
        obj.insert("mcpServers".into(), Value::Object(out));
    }

    write_json_value(&path, &root)?;
    Ok(())
}

/// 将 config.json 中 enabled==true 的项投影写入 ~/.gemini/settings.json
fn sync_enabled(config: &MultiAppConfig) -> Result<(), AppError> {
    let enabled = collect_enabled_servers(&config.mcp.gemini);
    set_mcp_servers_map(&enabled)
}

/// 从 ~/.gemini/settings.json 导入 mcpServers 到统一结构（v3.7.0+）
/// 已存在的服务器将启用 Gemini 应用，不覆盖其他字段和应用状态
fn import_from(config: &mut MultiAppConfig) -> Result<usize, AppError> {
    use crate::app_config::{McpApps, McpServer};

    let text_opt = read_mcp_json()?;
    let Some(text) = text_opt else { return Ok(0) };

    let v: Value = serde_json::from_str(&text)
        .map_err(|e| AppError::McpValidation(format!("解析 ~/.gemini/settings.json 失败: {e}")))?;
    let Some(map) = v.get("mcpServers").and_then(|x| x.as_object()) else {
        return Ok(0);
    };

    // 确保新结构存在
    if config.mcp.servers.is_none() {
        config.mcp.servers = Some(std::collections::HashMap::new());
    }
    let servers = config.mcp.servers.as_mut().unwrap();

    let mut changed = 0;
    let mut errors = Vec::new();

    for (id, spec) in map.iter() {
        // 校验：单项失败不中止，收集错误继续处理
        if let Err(e) = validate_server_spec(spec) {
            log::warn!("跳过无效 MCP 服务器 '{id}': {e}");
            errors.push(format!("{id}: {e}"));
            continue;
        }

        if let Some(existing) = servers.get_mut(id) {
            // 已存在：仅启用 Gemini 应用
            if !existing.apps.gemini {
                existing.apps.gemini = true;
                changed += 1;
                log::info!("MCP 服务器 '{id}' 已启用 Gemini 应用");
            }
        } else {
            // 新建服务器：默认仅启用 Gemini
            servers.insert(
                id.clone(),
                McpServer {
                    id: id.clone(),
                    name: id.clone(),
                    server: spec.clone(),
                    apps: McpApps {
                        claude: false,
                        codex: false,
                        gemini: true,
                        opencode: false,
                    },
                    description: None,
                    homepage: None,
                    docs: None,
                    tags: Vec::new(),
                },
            );
            changed += 1;
            log::info!("导入新 MCP 服务器 '{id}'");
        }
    }

    if !errors.is_empty() {
        log::warn!("导入完成，但有 {} 项失败: {:?}", errors.len(), errors);
    }

    Ok(changed)
}

/// 将单个 MCP 服务器同步到 Gemini live 配置
fn sync_single(id: &str, server_spec: &Value) -> Result<(), AppError> {
    // 读取现有的 MCP 配置
    let current = read_mcp_servers_map()?;

    // 创建新的 HashMap，包含现有的所有服务器 + 当前要同步的服务器
    let mut updated = current;
    updated.insert(id.to_string(), server_spec.clone());

    // 写回
    set_mcp_servers_map(&updated)
}

/// 从 Gemini live 配置中移除单个 MCP 服务器
fn remove(id: &str) -> Result<(), AppError> {
    // 读取现有的 MCP 配置
    let mut current = read_mcp_servers_map()?;

    // 移除指定服务器
    current.remove(id);

    // 写回
    set_mcp_servers_map(&current)
}
