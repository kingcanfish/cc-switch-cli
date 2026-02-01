use serde_json::{json, Value};
use std::collections::HashMap;

use crate::app_config::{McpApps, McpServer, MultiAppConfig};
use crate::error::AppError;
use crate::mcp::validate_server_spec;
use crate::opencode_config;

/// OpenCode MCP 后端实现
pub struct OpenCodeBackend;

pub static OPENCODE_BACKEND: OpenCodeBackend = OpenCodeBackend;

// ============================================================================
// Helper
// ============================================================================

fn should_sync_opencode_mcp() -> bool {
    opencode_config::opencode_dir_exists()
}

// ============================================================================
// Format Conversion: CC Switch → OpenCode
// ============================================================================

fn convert_to_opencode_format(spec: &Value) -> Result<Value, AppError> {
    let obj = spec
        .as_object()
        .ok_or_else(|| AppError::McpValidation("MCP spec must be a JSON object".into()))?;

    let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");

    let mut result = serde_json::Map::new();

    match typ {
        "stdio" => {
            result.insert("type".into(), json!("local"));

            let cmd = obj.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let mut command_arr = vec![json!(cmd)];

            if let Some(args) = obj.get("args").and_then(|v| v.as_array()) {
                for arg in args {
                    command_arr.push(arg.clone());
                }
            }
            result.insert("command".into(), Value::Array(command_arr));

            if let Some(env) = obj.get("env") {
                if env.is_object() && !env.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    result.insert("environment".into(), env.clone());
                }
            }

            result.insert("enabled".into(), json!(true));
        }
        "sse" | "http" => {
            result.insert("type".into(), json!("remote"));

            if let Some(url) = obj.get("url") {
                result.insert("url".into(), url.clone());
            }

            if let Some(headers) = obj.get("headers") {
                if headers.is_object() && !headers.as_object().map(|o| o.is_empty()).unwrap_or(true)
                {
                    result.insert("headers".into(), headers.clone());
                }
            }

            result.insert("enabled".into(), json!(true));
        }
        _ => {
            return Err(AppError::McpValidation(format!("Unknown MCP type: {typ}")));
        }
    }

    Ok(Value::Object(result))
}

// ============================================================================
// Format Conversion: OpenCode → CC Switch
// ============================================================================

fn convert_from_opencode_format(spec: &Value) -> Result<Value, AppError> {
    let obj = spec
        .as_object()
        .ok_or_else(|| AppError::McpValidation("OpenCode MCP spec must be a JSON object".into()))?;

    let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("local");

    let mut result = serde_json::Map::new();

    match typ {
        "local" => {
            result.insert("type".into(), json!("stdio"));

            if let Some(cmd_arr) = obj.get("command").and_then(|v| v.as_array()) {
                if !cmd_arr.is_empty() {
                    if let Some(cmd) = cmd_arr.first().and_then(|v| v.as_str()) {
                        result.insert("command".into(), json!(cmd));
                    }

                    if cmd_arr.len() > 1 {
                        let args: Vec<Value> = cmd_arr[1..].to_vec();
                        result.insert("args".into(), Value::Array(args));
                    }
                }
            }

            if let Some(env) = obj.get("environment") {
                if env.is_object() && !env.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    result.insert("env".into(), env.clone());
                }
            }
        }
        "remote" => {
            result.insert("type".into(), json!("sse"));

            if let Some(url) = obj.get("url") {
                result.insert("url".into(), url.clone());
            }

            if let Some(headers) = obj.get("headers") {
                if headers.is_object() && !headers.as_object().map(|o| o.is_empty()).unwrap_or(true)
                {
                    result.insert("headers".into(), headers.clone());
                }
            }
        }
        _ => {
            return Err(AppError::McpValidation(format!(
                "Unknown OpenCode MCP type: {typ}"
            )));
        }
    }

    Ok(Value::Object(result))
}

// ============================================================================
// McpBackend impl
// ============================================================================

impl crate::mcp::McpBackend for OpenCodeBackend {
    fn import_into(&self, config: &mut MultiAppConfig) -> Result<usize, AppError> {
        let mcp_map = opencode_config::get_mcp_servers()?;
        if mcp_map.is_empty() {
            return Ok(0);
        }

        let servers = config.mcp.servers.get_or_insert_with(HashMap::new);

        let mut changed = 0;
        let mut errors = Vec::new();

        for (id, spec) in mcp_map {
            let enabled = spec
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            if !enabled {
                if let Some(existing) = servers.get_mut(&id) {
                    if existing.apps.opencode {
                        existing.apps.opencode = false;
                        changed += 1;
                    }
                }
                continue;
            }

            let unified_spec = match convert_from_opencode_format(&spec) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Skip invalid OpenCode MCP server '{id}': {e}");
                    errors.push(format!("{id}: {e}"));
                    continue;
                }
            };

            if let Err(e) = validate_server_spec(&unified_spec) {
                log::warn!("Skip invalid MCP server '{id}' after conversion: {e}");
                errors.push(format!("{id}: {e}"));
                continue;
            }

            if let Some(existing) = servers.get_mut(&id) {
                if !existing.apps.opencode {
                    existing.apps.opencode = true;
                    changed += 1;
                }
            } else {
                servers.insert(
                    id.clone(),
                    McpServer {
                        id: id.clone(),
                        name: id.clone(),
                        server: unified_spec,
                        apps: McpApps {
                            claude: false,
                            codex: false,
                            gemini: false,
                            opencode: true,
                        },
                        description: None,
                        homepage: None,
                        docs: None,
                        tags: Vec::new(),
                    },
                );
                changed += 1;
            }
        }

        if !errors.is_empty() {
            log::warn!(
                "Import completed with {} failures: {:?}",
                errors.len(),
                errors
            );
        }

        Ok(changed)
    }

    fn sync_enabled(&self, config: &MultiAppConfig) -> Result<(), AppError> {
        if !should_sync_opencode_mcp() {
            return Ok(());
        }

        let Some(servers) = &config.mcp.servers else {
            return Ok(());
        };

        for (id, server) in servers {
            if server.apps.opencode {
                let opencode_spec = convert_to_opencode_format(&server.server)?;
                opencode_config::set_mcp_server(id, opencode_spec)?;
            }
        }

        Ok(())
    }

    fn sync_single(&self, id: &str, server_spec: &Value) -> Result<(), AppError> {
        if !should_sync_opencode_mcp() {
            return Ok(());
        }

        let opencode_spec = convert_to_opencode_format(server_spec)?;
        opencode_config::set_mcp_server(id, opencode_spec)
    }

    fn remove(&self, id: &str) -> Result<(), AppError> {
        if !should_sync_opencode_mcp() {
            return Ok(());
        }

        opencode_config::remove_mcp_server(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_stdio_to_local() {
        let spec = json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-filesystem"],
            "env": { "HOME": "/Users/test" }
        });

        let result = convert_to_opencode_format(&spec).unwrap();
        assert_eq!(result["type"], "local");
        assert_eq!(result["command"][0], "npx");
        assert_eq!(result["command"][1], "-y");
        assert_eq!(
            result["command"][2],
            "@modelcontextprotocol/server-filesystem"
        );
        assert_eq!(result["environment"]["HOME"], "/Users/test");
        assert_eq!(result["enabled"], true);
    }

    #[test]
    fn test_convert_sse_to_remote() {
        let spec = json!({
            "type": "sse",
            "url": "https://example.com/mcp",
            "headers": { "Authorization": "Bearer xxx" }
        });

        let result = convert_to_opencode_format(&spec).unwrap();
        assert_eq!(result["type"], "remote");
        assert_eq!(result["url"], "https://example.com/mcp");
        assert_eq!(result["headers"]["Authorization"], "Bearer xxx");
        assert_eq!(result["enabled"], true);
    }

    #[test]
    fn test_convert_local_to_stdio() {
        let spec = json!({
            "type": "local",
            "command": ["npx", "-y", "@modelcontextprotocol/server-filesystem"],
            "environment": { "HOME": "/Users/test" }
        });

        let result = convert_from_opencode_format(&spec).unwrap();
        assert_eq!(result["type"], "stdio");
        assert_eq!(result["command"], "npx");
        assert_eq!(result["args"][0], "-y");
        assert_eq!(result["args"][1], "@modelcontextprotocol/server-filesystem");
        assert_eq!(result["env"]["HOME"], "/Users/test");
    }

    #[test]
    fn test_convert_remote_to_sse() {
        let spec = json!({
            "type": "remote",
            "url": "https://example.com/mcp",
            "headers": { "Authorization": "Bearer xxx" }
        });

        let result = convert_from_opencode_format(&spec).unwrap();
        assert_eq!(result["type"], "sse");
        assert_eq!(result["url"], "https://example.com/mcp");
        assert_eq!(result["headers"]["Authorization"], "Bearer xxx");
    }
}
