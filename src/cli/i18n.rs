use crate::settings::{get_settings, update_settings};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::RwLock;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Chinese,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => {
                text_for_language(Language::English, "language_display_name_english")
            }
            Language::Chinese => {
                text_for_language(Language::Chinese, "language_display_name_chinese")
            }
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "zh" | "zh-cn" | "zh-tw" | "chinese" => Language::Chinese,
            _ => Language::English,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Global language state
fn language_store() -> &'static RwLock<Language> {
    static STORE: OnceLock<RwLock<Language>> = OnceLock::new();
    STORE.get_or_init(|| {
        let settings = get_settings();
        let lang = settings
            .language
            .as_deref()
            .map(Language::from_code)
            .unwrap_or(Language::English);
        RwLock::new(lang)
    })
}

/// Get current language
pub fn current_language() -> Language {
    *language_store().read().expect("Failed to read language")
}

/// Set current language and persist
pub fn set_language(lang: Language) -> Result<(), crate::error::AppError> {
    // Update runtime state
    {
        let mut guard = language_store().write().expect("Failed to write language");
        *guard = lang;
    }

    // Persist to settings
    let mut settings = get_settings();
    settings.language = Some(lang.code().to_string());
    update_settings(settings)
}

// Check if current language is Chinese
// ============================================================================
// Locale loading and lookup
// ============================================================================

struct LocaleStore {
    en: HashMap<String, &'static str>,
    zh: HashMap<String, &'static str>,
}

fn load_locale(yaml: &str) -> HashMap<String, &'static str> {
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(yaml).expect("Failed to parse locale YAML");
    let mut flat: HashMap<String, String> = HashMap::new();
    flatten_locale_value(None, &parsed, &mut flat);
    flat.into_iter()
        .map(|(k, v)| {
            let value: &'static str = Box::leak(v.into_boxed_str());
            (k, value)
        })
        .collect()
}

fn flatten_locale_value(
    prefix: Option<&str>,
    value: &serde_yaml::Value,
    out: &mut HashMap<String, String>,
) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (key, value) in map {
                let key = key
                    .as_str()
                    .unwrap_or_else(|| panic!("Locale keys must be strings: {key:?}"));
                let next = if let Some(prefix_value) = prefix {
                    format!("{prefix_value}_{key}")
                } else {
                    key.to_string()
                };
                flatten_locale_value(Some(&next), value, out);
            }
        }
        serde_yaml::Value::String(value) => {
            let key = prefix.unwrap_or_else(|| panic!("Locale root must be a mapping, got string"));
            out.insert(key.to_string(), value.clone());
        }
        other => {
            let key = prefix.unwrap_or("<root>");
            panic!("Locale value for '{key}' must be a string or mapping, got {other:?}");
        }
    }
}

fn locale_store() -> &'static LocaleStore {
    static STORE: OnceLock<LocaleStore> = OnceLock::new();
    STORE.get_or_init(|| {
        let en = load_locale(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/locales/en.yaml"
        )));
        let zh = load_locale(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/locales/zh.yaml"
        )));
        LocaleStore { en, zh }
    })
}

fn lookup_with_store(locales: &LocaleStore, lang: Language, key: &'static str) -> &'static str {
    let primary = match lang {
        Language::Chinese => &locales.zh,
        Language::English => &locales.en,
    };
    if let Some(value) = primary.get(key) {
        value
    } else if let Some(value) = locales.en.get(key) {
        value
    } else {
        key
    }
}

fn text_for_language(lang: Language, key: &'static str) -> &'static str {
    lookup_with_store(locale_store(), lang, key)
}

pub fn text(key: &'static str) -> &'static str {
    text_for_language(current_language(), key)
}

pub fn text_with_args(key: &'static str, args: &[(&str, &str)]) -> String {
    let mut value = text(key).to_string();
    for (name, replacement) in args {
        let placeholder = format!("{{{}}}", name);
        value = value.replace(&placeholder, replacement);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_selects_locale() {
        let mut en = HashMap::new();
        let mut zh = HashMap::new();
        en.insert("greeting".to_string(), "Hello");
        zh.insert("greeting".to_string(), "你好");
        let store = LocaleStore { en, zh };

        assert_eq!(
            lookup_with_store(&store, Language::Chinese, "greeting"),
            "你好"
        );
    }

    #[test]
    fn lookup_falls_back_to_english() {
        let mut en = HashMap::new();
        let zh = HashMap::new();
        en.insert("greeting".to_string(), "Hello");
        let store = LocaleStore { en, zh };

        assert_eq!(
            lookup_with_store(&store, Language::Chinese, "greeting"),
            "Hello"
        );
    }

    #[test]
    fn locale_en_yaml_is_valid() {
        let yaml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/locales/en.yaml"));
        load_locale(yaml);
    }

    #[test]
    fn locale_zh_yaml_is_valid() {
        let yaml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/locales/zh.yaml"));
        load_locale(yaml);
    }
}

// ============================================================================
// Common UI Texts
// ============================================================================

pub mod texts {
    use super::{text, text_with_args};

    pub fn entity_provider() -> &'static str {
        text("entity_provider")
    }

    pub fn entity_server() -> &'static str {
        text("entity_server")
    }

    pub fn entity_prompt() -> &'static str {
        text("entity_prompt")
    }

    pub fn entity_added_success(entity_type: &str, name: &str) -> String {
        text_with_args(
            "entity_added_success",
            &[("entity_type", entity_type), ("name", name)],
        )
    }

    pub fn entity_updated_success(entity_type: &str, name: &str) -> String {
        text_with_args(
            "entity_updated_success",
            &[("entity_type", entity_type), ("name", name)],
        )
    }

    pub fn entity_deleted_success(entity_type: &str, name: &str) -> String {
        text_with_args(
            "entity_deleted_success",
            &[("entity_type", entity_type), ("name", name)],
        )
    }

    pub fn entity_not_found(entity_type: &str, id: &str) -> String {
        text_with_args(
            "entity_not_found",
            &[("entity_type", entity_type), ("id", id)],
        )
    }

    pub fn confirm_create_entity(entity_type: &str) -> String {
        text_with_args("confirm_create_entity", &[("entity_type", entity_type)])
    }

    pub fn confirm_update_entity(entity_type: &str) -> String {
        text_with_args("confirm_update_entity", &[("entity_type", entity_type)])
    }

    pub fn confirm_delete_entity(entity_type: &str, name: &str) -> String {
        text_with_args(
            "confirm_delete_entity",
            &[("entity_type", entity_type), ("name", name)],
        )
    }

    pub fn select_to_delete_entity(entity_type: &str) -> String {
        text_with_args("select_to_delete_entity", &[("entity_type", entity_type)])
    }

    pub fn no_entities_to_delete(entity_type: &str) -> String {
        text_with_args("no_entities_to_delete", &[("entity_type", entity_type)])
    }

    pub fn welcome_title() -> &'static str {
        text("welcome_title")
    }

    pub fn application() -> &'static str {
        text("application")
    }

    pub fn goodbye() -> &'static str {
        text("goodbye")
    }

    pub fn main_menu_prompt(app: &str) -> String {
        text_with_args("main_menu_prompt", &[("app", app)])
    }

    pub fn main_menu_help() -> &'static str {
        text("main_menu_help")
    }

    pub fn main_menu_search_prompt() -> &'static str {
        text("main_menu_search_prompt")
    }

    pub fn main_menu_filtering(query: &str) -> String {
        text_with_args("main_menu_filtering", &[("query", query)])
    }

    pub fn main_menu_no_matches() -> &'static str {
        text("main_menu_no_matches")
    }

    pub fn tui_list_help() -> &'static str {
        text("tui_list_help")
    }

    pub fn tui_text_help() -> &'static str {
        text("tui_text_help")
    }

    pub fn tui_confirm_help() -> &'static str {
        text("tui_confirm_help")
    }

    pub fn tui_empty_list() -> &'static str {
        text("tui_empty_list")
    }

    pub fn tui_yes() -> &'static str {
        text("tui_yes")
    }

    pub fn tui_no() -> &'static str {
        text("tui_no")
    }

    pub fn menu_manage_providers() -> &'static str {
        text("menu_manage_providers")
    }

    pub fn menu_manage_mcp() -> &'static str {
        text("menu_manage_mcp")
    }

    pub fn menu_manage_prompts() -> &'static str {
        text("menu_manage_prompts")
    }

    pub fn menu_manage_skills() -> &'static str {
        text("menu_manage_skills")
    }

    pub fn menu_manage_config() -> &'static str {
        text("menu_manage_config")
    }

    pub fn menu_view_config() -> &'static str {
        text("menu_view_config")
    }

    pub fn menu_switch_app() -> &'static str {
        text("menu_switch_app")
    }

    pub fn menu_settings() -> &'static str {
        text("menu_settings")
    }

    pub fn menu_exit() -> &'static str {
        text("menu_exit")
    }

    pub fn skills_management() -> &'static str {
        text("skills_management")
    }

    pub fn skills_list() -> &'static str {
        text("skills_list")
    }

    pub fn skills_search() -> &'static str {
        text("skills_search")
    }

    pub fn skills_install() -> &'static str {
        text("skills_install")
    }

    pub fn skills_uninstall() -> &'static str {
        text("skills_uninstall")
    }

    pub fn skills_info() -> &'static str {
        text("skills_info")
    }

    pub fn skills_repos() -> &'static str {
        text("skills_repos")
    }

    pub fn skills_installed_header() -> &'static str {
        text("skills_installed_header")
    }

    pub fn skills_available_header() -> &'static str {
        text("skills_available_header")
    }

    pub fn skills_none_installed() -> &'static str {
        text("skills_none_installed")
    }

    pub fn skills_none_found() -> &'static str {
        text("skills_none_found")
    }

    pub fn skills_loading() -> &'static str {
        text("skills_loading")
    }

    pub fn skills_fetch_timeout() -> &'static str {
        text("skills_fetch_timeout")
    }

    pub fn skills_showing_local_only() -> &'static str {
        text("skills_showing_local_only")
    }

    pub fn skills_search_prompt() -> &'static str {
        text("skills_search_prompt")
    }

    pub fn skills_select_prompt() -> &'static str {
        text("skills_select_prompt")
    }

    pub fn skills_name_prompt() -> &'static str {
        text("skills_name_prompt")
    }

    pub fn skills_not_found(name: &str) -> String {
        text_with_args("skills_not_found", &[("name", name)])
    }

    pub fn skills_not_installed(name: &str) -> String {
        text_with_args("skills_not_installed", &[("name", name)])
    }

    pub fn skills_install_missing_repo() -> &'static str {
        text("skills_install_missing_repo")
    }

    pub fn skills_installed(name: &str) -> String {
        text_with_args("skills_installed", &[("name", name)])
    }

    pub fn skills_uninstalled(name: &str) -> String {
        text_with_args("skills_uninstalled", &[("name", name)])
    }

    pub fn skills_info_header() -> &'static str {
        text("skills_info_header")
    }

    pub fn skills_label_name() -> &'static str {
        text("skills_label_name")
    }

    pub fn skills_label_directory() -> &'static str {
        text("skills_label_directory")
    }

    pub fn skills_label_description() -> &'static str {
        text("skills_label_description")
    }

    pub fn skills_label_readme() -> &'static str {
        text("skills_label_readme")
    }

    pub fn skills_label_installed() -> &'static str {
        text("skills_label_installed")
    }

    pub fn skills_label_apps() -> &'static str {
        text("skills_label_apps")
    }

    pub fn skills_repos_header() -> &'static str {
        text("skills_repos_header")
    }

    pub fn skills_repos_list() -> &'static str {
        text("skills_repos_list")
    }

    pub fn skills_repos_add() -> &'static str {
        text("skills_repos_add")
    }

    pub fn skills_repos_remove() -> &'static str {
        text("skills_repos_remove")
    }

    pub fn skills_repos_empty() -> &'static str {
        text("skills_repos_empty")
    }

    pub fn skills_repo_prompt() -> &'static str {
        text("skills_repo_prompt")
    }

    pub fn skills_repo_invalid_url() -> &'static str {
        text("skills_repo_invalid_url")
    }

    pub fn skills_repo_added() -> &'static str {
        text("skills_repo_added")
    }

    pub fn skills_repo_removed() -> &'static str {
        text("skills_repo_removed")
    }

    pub fn provider_management() -> &'static str {
        text("provider_management")
    }

    pub fn no_providers() -> &'static str {
        text("no_providers")
    }

    pub fn view_current_provider() -> &'static str {
        text("view_current_provider")
    }

    pub fn switch_provider() -> &'static str {
        text("switch_provider")
    }

    pub fn add_provider() -> &'static str {
        text("add_provider")
    }

    pub fn add_official_provider() -> &'static str {
        text("add_official_provider")
    }

    pub fn add_third_party_provider() -> &'static str {
        text("add_third_party_provider")
    }

    pub fn select_provider_add_mode() -> &'static str {
        text("select_provider_add_mode")
    }

    pub fn delete_provider() -> &'static str {
        text("delete_provider")
    }

    pub fn back_to_main() -> &'static str {
        text("back_to_main")
    }

    pub fn choose_action() -> &'static str {
        text("choose_action")
    }

    pub fn esc_to_go_back_help() -> &'static str {
        text("esc_to_go_back_help")
    }

    pub fn select_filter_help() -> &'static str {
        text("select_filter_help")
    }

    pub fn current_provider_details() -> &'static str {
        text("current_provider_details")
    }

    pub fn only_one_provider() -> &'static str {
        text("only_one_provider")
    }

    pub fn no_other_providers() -> &'static str {
        text("no_other_providers")
    }

    pub fn select_provider_to_switch() -> &'static str {
        text("select_provider_to_switch")
    }

    pub fn switched_to_provider(id: &str) -> String {
        text_with_args("switched_to_provider", &[("id", id)])
    }

    pub fn restart_note() -> &'static str {
        text("restart_note")
    }

    pub fn no_deletable_providers() -> &'static str {
        text("no_deletable_providers")
    }

    pub fn select_provider_to_delete() -> &'static str {
        text("select_provider_to_delete")
    }

    pub fn confirm_delete(id: &str) -> String {
        text_with_args("confirm_delete", &[("id", id)])
    }

    pub fn cancelled() -> &'static str {
        text("cancelled")
    }

    pub fn deleted_provider(id: &str) -> String {
        text_with_args("deleted_provider", &[("id", id)])
    }

    pub fn provider_name_label() -> &'static str {
        text("provider_name_label")
    }

    pub fn provider_name_help() -> &'static str {
        text("provider_name_help")
    }

    pub fn provider_name_help_edit() -> &'static str {
        text("provider_name_help_edit")
    }

    pub fn provider_name_placeholder() -> &'static str {
        text("provider_name_placeholder")
    }

    pub fn provider_name_empty_error() -> &'static str {
        text("provider_name_empty_error")
    }

    pub fn website_url_label() -> &'static str {
        text("website_url_label")
    }

    pub fn website_url_help() -> &'static str {
        text("website_url_help")
    }

    pub fn website_url_help_edit() -> &'static str {
        text("website_url_help_edit")
    }

    pub fn website_url_placeholder() -> &'static str {
        text("website_url_placeholder")
    }

    pub fn no_providers_hint() -> &'static str {
        text("no_providers_hint")
    }

    pub fn app_config_not_found(app: &str) -> String {
        text_with_args("app_config_not_found", &[("app", app)])
    }

    pub fn provider_not_found(id: &str) -> String {
        text_with_args("provider_not_found", &[("id", id)])
    }

    pub fn generated_id(id: &str) -> String {
        text_with_args("generated_id", &[("id", id)])
    }

    pub fn configure_optional_fields_prompt() -> &'static str {
        text("configure_optional_fields_prompt")
    }

    pub fn current_config_header() -> &'static str {
        text("current_config_header")
    }

    pub fn modify_provider_config_prompt() -> &'static str {
        text("modify_provider_config_prompt")
    }

    pub fn modify_optional_fields_prompt() -> &'static str {
        text("modify_optional_fields_prompt")
    }

    pub fn current_provider_synced_warning() -> &'static str {
        text("current_provider_synced_warning")
    }

    pub fn no_current_provider() -> &'static str {
        text("no_current_provider")
    }

    pub fn syncing_to_live_config() -> &'static str {
        text("syncing_to_live_config")
    }

    pub fn synced_live_config() -> &'static str {
        text("synced_live_config")
    }

    pub fn synced_opencode_live_config() -> &'static str {
        text("synced_opencode_live_config")
    }

    pub fn invalid_toml_syntax() -> &'static str {
        text("invalid_toml_syntax")
    }

    pub fn input_failed_error(err: &str) -> String {
        text_with_args("input_failed_error", &[("err", err)])
    }

    pub fn cannot_delete_current_provider() -> &'static str {
        text("cannot_delete_current_provider")
    }

    pub fn provider_name_prompt() -> &'static str {
        text("provider_name_prompt")
    }

    pub fn config_claude_header() -> &'static str {
        text("config_claude_header")
    }

    pub fn api_key_label() -> &'static str {
        text("api_key_label")
    }

    pub fn api_key_help() -> &'static str {
        text("api_key_help")
    }

    pub fn base_url_label() -> &'static str {
        text("base_url_label")
    }

    pub fn base_url_placeholder() -> &'static str {
        text("base_url_placeholder")
    }

    pub fn configure_model_names_prompt() -> &'static str {
        text("configure_model_names_prompt")
    }

    pub fn model_default_label() -> &'static str {
        text("model_default_label")
    }

    pub fn model_default_help() -> &'static str {
        text("model_default_help")
    }

    pub fn model_haiku_label() -> &'static str {
        text("model_haiku_label")
    }

    pub fn model_haiku_placeholder() -> &'static str {
        text("model_haiku_placeholder")
    }

    pub fn model_sonnet_label() -> &'static str {
        text("model_sonnet_label")
    }

    pub fn model_sonnet_placeholder() -> &'static str {
        text("model_sonnet_placeholder")
    }

    pub fn model_opus_label() -> &'static str {
        text("model_opus_label")
    }

    pub fn model_opus_placeholder() -> &'static str {
        text("model_opus_placeholder")
    }

    pub fn config_codex_header() -> &'static str {
        text("config_codex_header")
    }

    pub fn openai_api_key_label() -> &'static str {
        text("openai_api_key_label")
    }

    pub fn anthropic_api_key_label() -> &'static str {
        text("anthropic_api_key_label")
    }

    pub fn config_toml_label() -> &'static str {
        text("config_toml_label")
    }

    pub fn config_toml_help() -> &'static str {
        text("config_toml_help")
    }

    pub fn config_toml_placeholder() -> &'static str {
        text("config_toml_placeholder")
    }

    pub fn codex_auth_mode_info() -> &'static str {
        text("codex_auth_mode_info")
    }

    pub fn codex_auth_mode_label() -> &'static str {
        text("codex_auth_mode_label")
    }

    pub fn codex_auth_mode_help() -> &'static str {
        text("codex_auth_mode_help")
    }

    pub fn codex_auth_mode_openai() -> &'static str {
        text("codex_auth_mode_openai")
    }

    pub fn codex_auth_mode_env_var() -> &'static str {
        text("codex_auth_mode_env_var")
    }

    pub fn codex_official_provider_tip() -> &'static str {
        text("codex_official_provider_tip")
    }

    pub fn codex_env_key_info() -> &'static str {
        text("codex_env_key_info")
    }

    pub fn codex_env_key_label() -> &'static str {
        text("codex_env_key_label")
    }

    pub fn codex_env_key_help() -> &'static str {
        text("codex_env_key_help")
    }

    pub fn codex_wire_api_label() -> &'static str {
        text("codex_wire_api_label")
    }

    pub fn codex_wire_api_help() -> &'static str {
        text("codex_wire_api_help")
    }

    pub fn codex_env_reminder(env_key: &str) -> String {
        text_with_args("codex_env_reminder", &[("env_key", env_key)])
    }

    pub fn codex_openai_auth_info() -> &'static str {
        text("codex_openai_auth_info")
    }

    pub fn codex_base_url_help() -> &'static str {
        text("codex_base_url_help")
    }

    pub fn codex_model_help() -> &'static str {
        text("codex_model_help")
    }

    pub fn codex_dual_write_info(env_key: &str, _api_key: &str) -> String {
        text_with_args(
            "codex_dual_write_info",
            &[("env_key", env_key), ("_api_key", _api_key)],
        )
    }

    pub fn use_current_config_prompt() -> &'static str {
        text("use_current_config_prompt")
    }

    pub fn use_current_config_help() -> &'static str {
        text("use_current_config_help")
    }

    pub fn input_toml_config() -> &'static str {
        text("input_toml_config")
    }

    pub fn direct_enter_to_finish() -> &'static str {
        text("direct_enter_to_finish")
    }

    pub fn current_config_label() -> &'static str {
        text("current_config_label")
    }

    pub fn config_toml_header() -> &'static str {
        text("config_toml_header")
    }

    pub fn config_gemini_header() -> &'static str {
        text("config_gemini_header")
    }

    pub fn config_opencode_header() -> &'static str {
        text("config_opencode_header")
    }

    pub fn auth_type_label() -> &'static str {
        text("auth_type_label")
    }

    pub fn auth_type_api_key() -> &'static str {
        text("auth_type_api_key")
    }

    pub fn auth_type_service_account() -> &'static str {
        text("auth_type_service_account")
    }

    pub fn gemini_api_key_label() -> &'static str {
        text("gemini_api_key_label")
    }

    pub fn gemini_base_url_label() -> &'static str {
        text("gemini_base_url_label")
    }

    pub fn gemini_base_url_help() -> &'static str {
        text("gemini_base_url_help")
    }

    pub fn gemini_base_url_placeholder() -> &'static str {
        text("gemini_base_url_placeholder")
    }

    pub fn opencode_npm_label() -> &'static str {
        text("opencode_npm_label")
    }

    pub fn opencode_npm_help() -> &'static str {
        text("opencode_npm_help")
    }

    pub fn opencode_npm_required_error() -> &'static str {
        text("opencode_npm_required_error")
    }

    pub fn opencode_base_url_label() -> &'static str {
        text("opencode_base_url_label")
    }

    pub fn opencode_base_url_help() -> &'static str {
        text("opencode_base_url_help")
    }

    pub fn opencode_api_key_label() -> &'static str {
        text("opencode_api_key_label")
    }

    pub fn opencode_api_key_help() -> &'static str {
        text("opencode_api_key_help")
    }

    pub fn opencode_models_label() -> &'static str {
        text("opencode_models_label")
    }

    pub fn opencode_models_help() -> &'static str {
        text("opencode_models_help")
    }

    pub fn opencode_models_count_label() -> &'static str {
        text("opencode_models_count_label")
    }

    pub fn npm_display_label() -> &'static str {
        text("npm_display_label")
    }

    pub fn adc_project_id_label() -> &'static str {
        text("adc_project_id_label")
    }

    pub fn adc_location_label() -> &'static str {
        text("adc_location_label")
    }

    pub fn adc_location_placeholder() -> &'static str {
        text("adc_location_placeholder")
    }

    pub fn google_oauth_official() -> &'static str {
        text("google_oauth_official")
    }

    pub fn packycode_api_key() -> &'static str {
        text("packycode_api_key")
    }

    pub fn generic_api_key() -> &'static str {
        text("generic_api_key")
    }

    pub fn select_auth_method_help() -> &'static str {
        text("select_auth_method_help")
    }

    pub fn use_google_oauth_warning() -> &'static str {
        text("use_google_oauth_warning")
    }

    pub fn packycode_api_key_help() -> &'static str {
        text("packycode_api_key_help")
    }

    pub fn packycode_endpoint_help() -> &'static str {
        text("packycode_endpoint_help")
    }

    pub fn generic_api_key_help() -> &'static str {
        text("generic_api_key_help")
    }

    pub fn notes_label() -> &'static str {
        text("notes_label")
    }

    pub fn notes_placeholder() -> &'static str {
        text("notes_placeholder")
    }

    pub fn sort_index_label() -> &'static str {
        text("sort_index_label")
    }

    pub fn sort_index_help() -> &'static str {
        text("sort_index_help")
    }

    pub fn sort_index_placeholder() -> &'static str {
        text("sort_index_placeholder")
    }

    pub fn invalid_sort_index() -> &'static str {
        text("invalid_sort_index")
    }

    pub fn optional_fields_config() -> &'static str {
        text("optional_fields_config")
    }

    pub fn notes_example_placeholder() -> &'static str {
        text("notes_example_placeholder")
    }

    pub fn notes_help_edit() -> &'static str {
        text("notes_help_edit")
    }

    pub fn notes_help_new() -> &'static str {
        text("notes_help_new")
    }

    pub fn sort_index_help_edit() -> &'static str {
        text("sort_index_help_edit")
    }

    pub fn sort_index_help_new() -> &'static str {
        text("sort_index_help_new")
    }

    pub fn invalid_sort_index_number() -> &'static str {
        text("invalid_sort_index_number")
    }

    pub fn provider_config_summary() -> &'static str {
        text("provider_config_summary")
    }

    pub fn id_label() -> &'static str {
        text("id_label")
    }

    pub fn website_label() -> &'static str {
        text("website_label")
    }

    pub fn core_config_label() -> &'static str {
        text("core_config_label")
    }

    pub fn model_label() -> &'static str {
        text("model_label")
    }

    pub fn config_toml_lines(count: usize) -> String {
        let count = count.to_string();
        text_with_args("config_toml_lines", &[("count", &count)])
    }

    pub fn optional_fields_label() -> &'static str {
        text("optional_fields_label")
    }

    pub fn notes_label_colon() -> &'static str {
        text("notes_label_colon")
    }

    pub fn sort_index_label_colon() -> &'static str {
        text("sort_index_label_colon")
    }

    pub fn id_label_colon() -> &'static str {
        text("id_label_colon")
    }

    pub fn url_label_colon() -> &'static str {
        text("url_label_colon")
    }

    pub fn api_url_label_colon() -> &'static str {
        text("api_url_label_colon")
    }

    pub fn endpoint_label_colon() -> &'static str {
        text("endpoint_label_colon")
    }

    pub fn no_api_url_configured() -> &'static str {
        text("no_api_url_configured")
    }

    pub fn summary_divider() -> &'static str {
        text("summary_divider")
    }

    pub fn basic_info_header() -> &'static str {
        text("basic_info_header")
    }

    pub fn name_display_label() -> &'static str {
        text("name_display_label")
    }

    pub fn app_display_label() -> &'static str {
        text("app_display_label")
    }

    pub fn notes_display_label() -> &'static str {
        text("notes_display_label")
    }

    pub fn sort_index_display_label() -> &'static str {
        text("sort_index_display_label")
    }

    pub fn config_info_header() -> &'static str {
        text("config_info_header")
    }

    pub fn api_key_display_label() -> &'static str {
        text("api_key_display_label")
    }

    pub fn base_url_display_label() -> &'static str {
        text("base_url_display_label")
    }

    pub fn model_config_header() -> &'static str {
        text("model_config_header")
    }

    pub fn default_model_display() -> &'static str {
        text("default_model_display")
    }

    pub fn haiku_model_display() -> &'static str {
        text("haiku_model_display")
    }

    pub fn sonnet_model_display() -> &'static str {
        text("sonnet_model_display")
    }

    pub fn opus_model_display() -> &'static str {
        text("opus_model_display")
    }

    pub fn auth_type_display_label() -> &'static str {
        text("auth_type_display_label")
    }

    pub fn project_id_display_label() -> &'static str {
        text("project_id_display_label")
    }

    pub fn location_display_label() -> &'static str {
        text("location_display_label")
    }

    pub fn edit_provider_menu() -> &'static str {
        text("edit_provider_menu")
    }

    pub fn no_editable_providers() -> &'static str {
        text("no_editable_providers")
    }

    pub fn select_provider_to_edit() -> &'static str {
        text("select_provider_to_edit")
    }

    pub fn choose_edit_mode() -> &'static str {
        text("choose_edit_mode")
    }

    pub fn edit_mode_interactive() -> &'static str {
        text("edit_mode_interactive")
    }

    pub fn edit_mode_json_editor() -> &'static str {
        text("edit_mode_json_editor")
    }

    pub fn cancel() -> &'static str {
        text("cancel")
    }

    pub fn opening_external_editor() -> &'static str {
        text("opening_external_editor")
    }

    pub fn invalid_json_syntax() -> &'static str {
        text("invalid_json_syntax")
    }

    pub fn invalid_provider_structure() -> &'static str {
        text("invalid_provider_structure")
    }

    pub fn provider_id_cannot_be_changed() -> &'static str {
        text("provider_id_cannot_be_changed")
    }

    pub fn retry_editing() -> &'static str {
        text("retry_editing")
    }

    pub fn no_changes_detected() -> &'static str {
        text("no_changes_detected")
    }

    pub fn provider_summary() -> &'static str {
        text("provider_summary")
    }

    pub fn confirm_save_changes() -> &'static str {
        text("confirm_save_changes")
    }

    pub fn editor_failed() -> &'static str {
        text("editor_failed")
    }

    pub fn invalid_selection_format() -> &'static str {
        text("invalid_selection_format")
    }

    pub fn basic_info_section_header() -> &'static str {
        text("basic_info_section_header")
    }

    pub fn name_label_with_colon() -> &'static str {
        text("name_label_with_colon")
    }

    pub fn app_label_with_colon() -> &'static str {
        text("app_label_with_colon")
    }

    pub fn api_config_section_header() -> &'static str {
        text("api_config_section_header")
    }

    pub fn model_config_section_header() -> &'static str {
        text("model_config_section_header")
    }

    pub fn main_model_label_with_colon() -> &'static str {
        text("main_model_label_with_colon")
    }

    pub fn updated_config_header() -> &'static str {
        text("updated_config_header")
    }

    pub fn generated_id_message(id: &str) -> String {
        text_with_args("generated_id_message", &[("id", id)])
    }

    pub fn edit_fields_instruction() -> &'static str {
        text("edit_fields_instruction")
    }

    pub fn mcp_management() -> &'static str {
        text("mcp_management")
    }

    pub fn no_mcp_servers() -> &'static str {
        text("no_mcp_servers")
    }

    pub fn sync_all_servers() -> &'static str {
        text("sync_all_servers")
    }

    pub fn synced_successfully() -> &'static str {
        text("synced_successfully")
    }

    pub fn prompts_management() -> &'static str {
        text("prompts_management")
    }

    pub fn no_prompts() -> &'static str {
        text("no_prompts")
    }

    pub fn switch_active_prompt() -> &'static str {
        text("switch_active_prompt")
    }

    pub fn no_prompts_available() -> &'static str {
        text("no_prompts_available")
    }

    pub fn select_prompt_to_activate() -> &'static str {
        text("select_prompt_to_activate")
    }

    pub fn activated_prompt(id: &str) -> String {
        text_with_args("activated_prompt", &[("id", id)])
    }

    pub fn deactivated_prompt(id: &str) -> String {
        text_with_args("deactivated_prompt", &[("id", id)])
    }

    pub fn prompt_cleared_note() -> &'static str {
        text("prompt_cleared_note")
    }

    pub fn prompt_synced_note() -> &'static str {
        text("prompt_synced_note")
    }

    pub fn current_configuration() -> &'static str {
        text("current_configuration")
    }

    pub fn provider_label() -> &'static str {
        text("provider_label")
    }

    pub fn mcp_servers_label() -> &'static str {
        text("mcp_servers_label")
    }

    pub fn prompts_label() -> &'static str {
        text("prompts_label")
    }

    pub fn total() -> &'static str {
        text("total")
    }

    pub fn enabled() -> &'static str {
        text("enabled")
    }

    pub fn active() -> &'static str {
        text("active")
    }

    pub fn none() -> &'static str {
        text("none")
    }

    pub fn settings_title() -> &'static str {
        text("settings_title")
    }

    pub fn change_language() -> &'static str {
        text("change_language")
    }

    pub fn current_language_label() -> &'static str {
        text("current_language_label")
    }

    pub fn select_language() -> &'static str {
        text("select_language")
    }

    pub fn language_changed() -> &'static str {
        text("language_changed")
    }

    pub fn select_application() -> &'static str {
        text("select_application")
    }

    pub fn switched_to_app(app: &str) -> String {
        text_with_args("switched_to_app", &[("app", app)])
    }

    pub fn press_enter() -> &'static str {
        text("press_enter")
    }

    pub fn error_prefix() -> &'static str {
        text("error_prefix")
    }

    pub fn header_name() -> &'static str {
        text("header_name")
    }

    pub fn header_category() -> &'static str {
        text("header_category")
    }

    pub fn header_description() -> &'static str {
        text("header_description")
    }

    pub fn config_management() -> &'static str {
        text("config_management")
    }

    pub fn config_export() -> &'static str {
        text("config_export")
    }

    pub fn config_import() -> &'static str {
        text("config_import")
    }

    pub fn config_backup() -> &'static str {
        text("config_backup")
    }

    pub fn config_restore() -> &'static str {
        text("config_restore")
    }

    pub fn config_validate() -> &'static str {
        text("config_validate")
    }

    pub fn config_common_snippet() -> &'static str {
        text("config_common_snippet")
    }

    pub fn config_reset() -> &'static str {
        text("config_reset")
    }

    pub fn config_show_full() -> &'static str {
        text("config_show_full")
    }

    pub fn config_show_path() -> &'static str {
        text("config_show_path")
    }

    pub fn enter_export_path() -> &'static str {
        text("enter_export_path")
    }

    pub fn enter_import_path() -> &'static str {
        text("enter_import_path")
    }

    pub fn enter_restore_path() -> &'static str {
        text("enter_restore_path")
    }

    pub fn confirm_import() -> &'static str {
        text("confirm_import")
    }

    pub fn confirm_reset() -> &'static str {
        text("confirm_reset")
    }

    pub fn common_config_snippet_editor_prompt(app: &str) -> String {
        text_with_args("common_config_snippet_editor_prompt", &[("app", app)])
    }

    pub fn common_config_snippet_invalid_json(err: &str) -> String {
        text_with_args("common_config_snippet_invalid_json", &[("err", err)])
    }

    pub fn common_config_snippet_not_object() -> &'static str {
        text("common_config_snippet_not_object")
    }

    pub fn common_config_snippet_saved() -> &'static str {
        text("common_config_snippet_saved")
    }

    pub fn common_config_snippet_cleared() -> &'static str {
        text("common_config_snippet_cleared")
    }

    pub fn common_config_snippet_apply_now() -> &'static str {
        text("common_config_snippet_apply_now")
    }

    pub fn common_config_snippet_no_current_provider() -> &'static str {
        text("common_config_snippet_no_current_provider")
    }

    pub fn common_config_snippet_applied() -> &'static str {
        text("common_config_snippet_applied")
    }

    pub fn common_config_snippet_apply_hint() -> &'static str {
        text("common_config_snippet_apply_hint")
    }

    pub fn confirm_restore() -> &'static str {
        text("confirm_restore")
    }

    pub fn exported_to(path: &str) -> String {
        text_with_args("exported_to", &[("path", path)])
    }

    pub fn imported_from(path: &str) -> String {
        text_with_args("imported_from", &[("path", path)])
    }

    pub fn backup_created(id: &str) -> String {
        text_with_args("backup_created", &[("id", id)])
    }

    pub fn backup_use_custom_name_confirm() -> &'static str {
        text("backup_use_custom_name_confirm")
    }

    pub fn backup_name_prompt() -> &'static str {
        text("backup_name_prompt")
    }

    pub fn backup_name_help() -> &'static str {
        text("backup_name_help")
    }

    pub fn backup_location(path: &str) -> String {
        text_with_args("backup_location", &[("path", path)])
    }

    pub fn no_backups_available() -> &'static str {
        text("restore_no_backups_available")
    }

    pub fn backups_create_hint() -> &'static str {
        text("restore_create_backup_hint")
    }

    pub fn select_backup_to_restore() -> &'static str {
        text("restore_select_backup")
    }

    pub fn invalid_backup_selection() -> &'static str {
        text("restore_invalid_selection")
    }

    pub fn restore_warning_title() -> &'static str {
        text("restore_warning_title")
    }

    pub fn restore_warning_replace_current() -> &'static str {
        text("restore_warning_replace_current")
    }

    pub fn restore_warning_auto_backup() -> &'static str {
        text("restore_warning_auto_backup")
    }

    pub fn restore_pre_backup_created(id: &str) -> String {
        text_with_args("restore_pre_backup_created", &[("id", id)])
    }

    pub fn restored_from(path: &str) -> String {
        text_with_args("restored_from", &[("path", path)])
    }

    pub fn config_valid() -> &'static str {
        text("config_valid")
    }

    pub fn config_reset_done() -> &'static str {
        text("config_reset_done")
    }

    pub fn file_overwrite_confirm(path: &str) -> String {
        text_with_args("file_overwrite_confirm", &[("path", path)])
    }

    pub fn mcp_delete_server() -> &'static str {
        text("mcp_delete_server")
    }

    pub fn mcp_enable_server() -> &'static str {
        text("mcp_enable_server")
    }

    pub fn mcp_disable_server() -> &'static str {
        text("mcp_disable_server")
    }

    pub fn mcp_import_servers() -> &'static str {
        text("mcp_import_servers")
    }

    pub fn mcp_validate_command() -> &'static str {
        text("mcp_validate_command")
    }

    pub fn select_server_to_delete() -> &'static str {
        text("select_server_to_delete")
    }

    pub fn select_server_to_enable() -> &'static str {
        text("select_server_to_enable")
    }

    pub fn select_server_to_disable() -> &'static str {
        text("select_server_to_disable")
    }

    pub fn select_apps_to_enable() -> &'static str {
        text("select_apps_to_enable")
    }

    pub fn mcp_enable_apps_help() -> &'static str {
        text("mcp_enable_apps_help")
    }

    pub fn select_apps_to_disable() -> &'static str {
        text("select_apps_to_disable")
    }

    pub fn enter_command_to_validate() -> &'static str {
        text("enter_command_to_validate")
    }

    pub fn server_deleted(id: &str) -> String {
        text_with_args("server_deleted", &[("id", id)])
    }

    pub fn server_enabled(id: &str) -> String {
        text_with_args("server_enabled", &[("id", id)])
    }

    pub fn server_disabled(id: &str) -> String {
        text_with_args("server_disabled", &[("id", id)])
    }

    pub fn servers_imported(count: usize) -> String {
        let count = count.to_string();
        text_with_args("servers_imported", &[("count", &count)])
    }

    pub fn command_valid(cmd: &str) -> String {
        text_with_args("command_valid", &[("cmd", cmd)])
    }

    pub fn command_invalid(cmd: &str) -> String {
        text_with_args("command_invalid", &[("cmd", cmd)])
    }

    pub fn prompts_show_content() -> &'static str {
        text("prompts_show_content")
    }

    pub fn prompts_delete() -> &'static str {
        text("prompts_delete")
    }

    pub fn prompts_view_current() -> &'static str {
        text("prompts_view_current")
    }

    pub fn select_prompt_to_view() -> &'static str {
        text("select_prompt_to_view")
    }

    pub fn select_prompt_to_delete() -> &'static str {
        text("select_prompt_to_delete")
    }

    pub fn prompt_deleted(id: &str) -> String {
        text_with_args("prompt_deleted", &[("id", id)])
    }

    pub fn no_active_prompt() -> &'static str {
        text("no_active_prompt")
    }

    pub fn cannot_delete_active() -> &'static str {
        text("cannot_delete_active")
    }

    pub fn no_servers_to_delete() -> &'static str {
        text("no_servers_to_delete")
    }

    pub fn no_prompts_to_delete() -> &'static str {
        text("no_prompts_to_delete")
    }

    pub fn speedtest_endpoint() -> &'static str {
        text("speedtest_endpoint")
    }

    pub fn duplicating_provider(id: &str) -> String {
        text_with_args("duplicating_provider", &[("id", id)])
    }

    pub fn provider_duplication_not_implemented() -> &'static str {
        text("provider_duplication_not_implemented")
    }

    pub fn testing_provider(name: &str) -> String {
        text_with_args("testing_provider", &[("name", name)])
    }

    pub fn speedtest_failed() -> &'static str {
        text("speedtest_failed")
    }

    pub fn speedtest_timeout() -> &'static str {
        text("speedtest_timeout")
    }

    pub fn speedtest_completed_success() -> &'static str {
        text("speedtest_completed_success")
    }

    pub fn latency_label() -> &'static str {
        text("latency_label")
    }

    pub fn status_label() -> &'static str {
        text("status_label")
    }

    pub fn not_applicable() -> &'static str {
        text("not_applicable")
    }

    pub fn async_runtime_create_failed(err: &str) -> String {
        text_with_args("async_runtime_create_failed", &[("err", err)])
    }

    pub fn opencode_additive_mode_notice() -> &'static str {
        text("opencode_additive_mode_notice")
    }

    pub fn opencode_no_current_provider() -> &'static str {
        text("opencode_no_current_provider")
    }

    pub fn opencode_switch_not_supported() -> &'static str {
        text("opencode_switch_not_supported")
    }

    pub fn back() -> &'static str {
        text("back")
    }
}
