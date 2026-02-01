use serde_json::json;
use std::fs;
use std::sync::RwLock;

use cc_switch_lib::{
    update_settings, AppSettings, AppState, AppType, MultiAppConfig, PromptService, Provider,
    ProviderService, SkillApps, SkillService,
};

#[path = "support.rs"]
mod support;
use support::{ensure_test_home, reset_test_fs, test_mutex};

#[test]
fn opencode_add_is_additive_and_writes_live() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let state = AppState {
        config: RwLock::new(MultiAppConfig::default()),
    };

    let provider = Provider::with_id(
        "demo".to_string(),
        "Demo".to_string(),
        json!({
            "npm": "@ai-sdk/openai-compatible"
        }),
        None,
    );

    ProviderService::add(&state, AppType::OpenCode, provider).expect("add opencode provider");

    let current = ProviderService::current(&state, AppType::OpenCode).expect("current provider");
    assert!(
        current.is_empty(),
        "OpenCode should not have current provider"
    );

    let config_path = home.join(".config").join("opencode").join("opencode.json");
    let raw = fs::read_to_string(&config_path).expect("read opencode.json");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("parse opencode.json");
    assert_eq!(
        value.pointer("/provider/demo/npm").and_then(|v| v.as_str()),
        Some("@ai-sdk/openai-compatible")
    );
}

#[test]
fn opencode_override_dir_used_for_live_settings() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let override_dir = home.join("custom-opencode");
    fs::create_dir_all(&override_dir).expect("create override dir");

    update_settings(AppSettings {
        opencode_config_dir: Some(override_dir.to_string_lossy().to_string()),
        ..Default::default()
    })
    .expect("update settings");

    let override_path = override_dir.join("opencode.json");
    fs::write(
        &override_path,
        json!({
            "provider": {
                "override": {
                    "npm": "@ai-sdk/override"
                }
            }
        })
        .to_string(),
    )
    .expect("write override opencode.json");

    let default_dir = home.join(".config").join("opencode");
    fs::create_dir_all(&default_dir).expect("create default opencode dir");
    fs::write(
        default_dir.join("opencode.json"),
        json!({
            "provider": {
                "default": {
                    "npm": "@ai-sdk/default"
                }
            }
        })
        .to_string(),
    )
    .expect("write default opencode.json");

    let value = ProviderService::read_live_settings(AppType::OpenCode).expect("read live settings");

    assert_eq!(
        value
            .pointer("/provider/override/npm")
            .and_then(|v| v.as_str()),
        Some("@ai-sdk/override")
    );
    assert!(
        value.pointer("/provider/default").is_none(),
        "should read from override dir, not default dir"
    );
}

#[test]
fn opencode_prompt_import_uses_override_dir() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let override_dir = home.join("prompt-opencode");
    fs::create_dir_all(&override_dir).expect("create override dir");

    update_settings(AppSettings {
        opencode_config_dir: Some(override_dir.to_string_lossy().to_string()),
        ..Default::default()
    })
    .expect("update settings");

    let override_prompt = override_dir.join("AGENTS.md");
    fs::write(&override_prompt, "override prompt").expect("write override prompt");

    let default_prompt = home.join(".config").join("opencode").join("AGENTS.md");
    if let Some(parent) = default_prompt.parent() {
        fs::create_dir_all(parent).expect("create default prompt dir");
    }
    fs::write(&default_prompt, "default prompt").expect("write default prompt");

    let state = AppState {
        config: RwLock::new(MultiAppConfig::default()),
    };

    PromptService::import_from_file(&state, AppType::OpenCode).expect("import opencode prompt");

    let prompts =
        PromptService::get_prompts(&state, AppType::OpenCode).expect("get opencode prompts");
    assert!(
        prompts.values().any(|p| p.content == "override prompt"),
        "should import prompt from override dir"
    );
}

#[test]
fn skills_sync_to_all_apps() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let service = SkillService::new().expect("create skill service");

    let skill_dir = home
        .join(".cc-switch-cli")
        .join("skills")
        .join("demo-skill");
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    fs::write(skill_dir.join("SKILL.md"), "# Demo Skill").expect("write skill file");

    let apps = SkillApps::set_all_enabled();
    service
        .sync_skill_to_apps("demo-skill", &apps)
        .expect("sync skill to apps");

    let expected_paths = [
        home.join(".claude/skills/demo-skill/SKILL.md"),
        home.join(".codex/skills/demo-skill/SKILL.md"),
        home.join(".gemini/skills/demo-skill/SKILL.md"),
        home.join(".config/opencode/skills/demo-skill/SKILL.md"),
    ];

    for path in expected_paths {
        assert!(path.exists(), "skill file missing at {}", path.display());
    }
}
