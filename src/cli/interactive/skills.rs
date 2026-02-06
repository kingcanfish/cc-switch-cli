use chrono::Utc;
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::cli::i18n::texts;
use crate::cli::tui::theme::accent_color;
use crate::cli::tui::TextViewScreen;
use crate::cli::ui::current_tui_app;
use crate::error::AppError;
use crate::services::skill::{Skill, SkillApps, SkillRepo, SkillService, SkillState, SkillStore};

use super::utils::{get_state, prompt_select, prompt_text, run_tui_screen, run_with_tui_loading};

pub fn manage_skills_menu() -> Result<(), AppError> {
    loop {
        let choices = vec![
            texts::skills_list().to_string(),
            texts::skills_search().to_string(),
            texts::skills_install().to_string(),
            texts::skills_uninstall().to_string(),
            texts::skills_info().to_string(),
            texts::skills_repos().to_string(),
            texts::back_to_main().to_string(),
        ];

        let Some(choice) = prompt_select(texts::skills_management(), choices)? else {
            break;
        };

        if choice == texts::skills_list() {
            list_skills_interactive()?;
        } else if choice == texts::skills_search() {
            search_skills_interactive()?;
        } else if choice == texts::skills_install() {
            install_skill_interactive()?;
        } else if choice == texts::skills_uninstall() {
            uninstall_skill_interactive()?;
        } else if choice == texts::skills_info() {
            show_skill_info_interactive()?;
        } else if choice == texts::skills_repos() {
            manage_repos_menu()?;
        } else {
            break;
        }
    }

    Ok(())
}

fn read_skill_store(state: &crate::store::AppState) -> Result<SkillStore, AppError> {
    let config = state.config.read()?;
    Ok(config.skills.clone())
}

fn reload_search_skills_after_mutation_with_loader<F>(
    state: &crate::store::AppState,
    query: &str,
    loader: F,
) -> Result<(Vec<Skill>, bool), AppError>
where
    F: FnOnce(&str, &SkillStore) -> Result<(Vec<Skill>, bool), AppError>,
{
    let refreshed_store = read_skill_store(state)?;
    loader(query, &refreshed_store)
}

fn reload_search_skills_after_mutation(
    state: &crate::store::AppState,
    query: &str,
) -> Result<(Vec<Skill>, bool), AppError> {
    reload_search_skills_after_mutation_with_loader(
        state,
        query,
        load_skills_for_search_with_feedback,
    )
}

fn list_skills_interactive() -> Result<(), AppError> {
    let state = get_state()?;
    let store = read_skill_store(&state)?;

    let mut installed: Vec<_> = store
        .skills
        .iter()
        .filter(|(_, state)| state.installed)
        .map(|(directory, state)| (directory.clone(), state.apps.clone()))
        .collect();
    installed.sort_by(|a, b| a.0.cmp(&b.0));

    let mut lines = Vec::new();
    if installed.is_empty() {
        lines.push(texts::skills_none_installed().to_string());
    } else {
        for (directory, apps) in installed {
            lines.push(format!("- {} [{}]", directory, format_enabled_apps(&apps)));
        }
    }
    tui_show_text(texts::skills_installed_header(), lines)
}

fn search_skills_interactive() -> Result<(), AppError> {
    let Some(query) = prompt_text(texts::skills_search_prompt())? else {
        return Ok(());
    };

    let query = query.trim().to_string();

    let state = get_state()?;
    let store = read_skill_store(&state)?;

    let (mut skills, mut timed_out) = load_skills_for_search_with_feedback(&query, &store)?;

    if skills.is_empty() {
        let mut lines = Vec::new();
        if timed_out {
            lines.push(texts::skills_fetch_timeout().to_string());
        }
        lines.push(texts::skills_none_found().to_string());
        tui_show_text(texts::skills_available_header(), lines)?;
        return Ok(());
    }

    #[derive(Clone)]
    struct SkillChoice {
        label: String,
        directory: String,
    }

    impl std::fmt::Display for SkillChoice {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.label)
        }
    }

    loop {
        let choices: Vec<SkillChoice> = skills
            .iter()
            .map(|skill| SkillChoice {
                label: format!(
                    "[{}] {} ({})",
                    if skill.installed { "✓" } else { " " },
                    skill.name,
                    skill.directory
                ),
                directory: skill.directory.clone(),
            })
            .collect();

        let select_prompt = if timed_out {
            format!(
                "{} ({})",
                texts::skills_select_prompt(),
                texts::skills_showing_local_only()
            )
        } else {
            texts::skills_select_prompt().to_string()
        };

        let Some(selected) = prompt_select(&select_prompt, choices)? else {
            return Ok(());
        };

        let Some(skill) = skills.iter().find(|s| s.directory == selected.directory) else {
            continue;
        };

        loop {
            let mut actions = Vec::new();
            if skill.installed {
                actions.push(texts::skills_uninstall());
            } else {
                actions.push(texts::skills_install());
            }
            actions.push(texts::skills_info());
            actions.push(texts::back());

            let Some(action) = prompt_select(texts::choose_action(), actions)? else {
                return Ok(());
            };

            if action == texts::back() {
                break;
            }

            if action == texts::skills_install() {
                install_skill_entry(&state, skill)?;
                let (updated_skills, did_timeout) =
                    reload_search_skills_after_mutation(&state, &query)?;
                skills = updated_skills;
                timed_out = did_timeout;
                break;
            } else if action == texts::skills_uninstall() {
                uninstall_skill_entry(&state, skill)?;
                let (updated_skills, did_timeout) =
                    reload_search_skills_after_mutation(&state, &query)?;
                skills = updated_skills;
                timed_out = did_timeout;
                break;
            } else if action == texts::skills_info() {
                let refreshed_store = read_skill_store(&state)?;
                show_skill_info_entry(skill, &refreshed_store)?;
            }
        }
    }
}

fn install_skill_interactive() -> Result<(), AppError> {
    let Some(name) = prompt_text(texts::skills_name_prompt())? else {
        return Ok(());
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(());
    }

    let state = get_state()?;
    let store = read_skill_store(&state)?;

    let (skills, timed_out) = load_skills_for_search_with_feedback("", &store)?;

    let Some(skill) = skills.iter().find(|s| {
        s.key.eq_ignore_ascii_case(&name)
            || s.directory.eq_ignore_ascii_case(&name)
            || s.name.eq_ignore_ascii_case(&name)
    }) else {
        let mut lines = Vec::new();
        if timed_out {
            lines.push(texts::skills_fetch_timeout().to_string());
        }
        lines.push(texts::skills_not_found(&name));
        tui_show_text(texts::skills_install(), lines)?;
        return Ok(());
    };

    install_skill_entry(&state, skill)
}

fn uninstall_skill_interactive() -> Result<(), AppError> {
    let Some(name) = prompt_text(texts::skills_name_prompt())? else {
        return Ok(());
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(());
    }

    let state = get_state()?;
    let store = read_skill_store(&state)?;

    let (skills, timed_out) = load_skills_for_search_with_feedback("", &store)?;

    let Some(skill) = skills.iter().find(|s| {
        s.installed
            && (s.key.eq_ignore_ascii_case(&name)
                || s.directory.eq_ignore_ascii_case(&name)
                || s.name.eq_ignore_ascii_case(&name))
    }) else {
        let mut lines = Vec::new();
        if timed_out {
            lines.push(texts::skills_fetch_timeout().to_string());
        }
        lines.push(texts::skills_not_installed(&name));
        tui_show_text(texts::skills_uninstall(), lines)?;
        return Ok(());
    };

    let directory = skill.directory.clone();
    let apps = {
        let config = state.config.read()?;
        config
            .skills
            .skills
            .get(&directory)
            .map(|s| s.apps.clone())
            .unwrap_or_else(SkillApps::set_all_enabled)
    };

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    service
        .uninstall_skill(directory.clone())
        .map_err(|e| AppError::Message(e.to_string()))?;
    service
        .remove_skill_from_apps(&directory, &apps)
        .map_err(|e| AppError::Message(e.to_string()))?;

    {
        let mut config = state.config.write()?;
        config.skills.skills.remove(&directory);
    }
    state.save()?;

    tui_show_text(
        texts::skills_uninstall(),
        vec![texts::skills_uninstalled(&directory).to_string()],
    )?;
    Ok(())
}

fn show_skill_info_interactive() -> Result<(), AppError> {
    let Some(name) = prompt_text(texts::skills_name_prompt())? else {
        return Ok(());
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(());
    }

    let state = get_state()?;
    let store = read_skill_store(&state)?;

    let (skills, timed_out) = load_skills_for_search_with_feedback("", &store)?;

    let Some(skill) = skills.iter().find(|s| {
        s.key.eq_ignore_ascii_case(&name)
            || s.directory.eq_ignore_ascii_case(&name)
            || s.name.eq_ignore_ascii_case(&name)
    }) else {
        let mut lines = Vec::new();
        if timed_out {
            lines.push(texts::skills_fetch_timeout().to_string());
        }
        lines.push(texts::skills_not_found(&name));
        tui_show_text(texts::skills_info_header(), lines)?;
        return Ok(());
    };

    let lines = build_skill_info_lines(skill, &store);
    tui_show_text(texts::skills_info_header(), lines)
}

fn install_skill_entry(state: &crate::store::AppState, skill: &Skill) -> Result<(), AppError> {
    let (owner, repo_name) = match (skill.repo_owner.as_deref(), skill.repo_name.as_deref()) {
        (Some(owner), Some(repo_name)) if !owner.is_empty() && !repo_name.is_empty() => {
            (owner.to_string(), repo_name.to_string())
        }
        _ => {
            tui_show_text(
                texts::skills_install(),
                vec![texts::skills_install_missing_repo().to_string()],
            )?;
            return Ok(());
        }
    };

    let repo = SkillRepo {
        owner,
        name: repo_name,
        branch: skill
            .repo_branch
            .clone()
            .unwrap_or_else(|| "main".to_string()),
        enabled: true,
        skills_path: skill.skills_path.clone(),
    };

    let install_dir = skill.directory.clone();
    run_with_tui_loading(
        texts::skills_install(),
        texts::skills_loading(),
        texts::skills_fetch_timeout(),
        move || {
            let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
            let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
            runtime
                .block_on(async { service.install_skill(install_dir.clone(), repo).await })
                .map_err(|e| AppError::Message(e.to_string()))
        },
    )?;

    {
        let mut config = state.config.write()?;
        let entry = config
            .skills
            .skills
            .entry(skill.directory.clone())
            .or_insert(SkillState {
                installed: true,
                installed_at: Utc::now(),
                apps: SkillApps::set_all_enabled(),
            });
        entry.installed = true;
        entry.installed_at = Utc::now();
        if entry.apps.enabled_apps().is_empty() {
            entry.apps = SkillApps::set_all_enabled();
        }
    }
    state.save()?;

    let apps = {
        let config = state.config.read()?;
        config
            .skills
            .skills
            .get(&skill.directory)
            .map(|s| s.apps.clone())
            .unwrap_or_else(SkillApps::set_all_enabled)
    };

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    service
        .sync_skill_to_apps(&skill.directory, &apps)
        .map_err(|e| AppError::Message(e.to_string()))?;

    tui_show_text(
        texts::skills_install(),
        vec![texts::skills_installed(&skill.name).to_string()],
    )?;
    Ok(())
}

fn uninstall_skill_entry(state: &crate::store::AppState, skill: &Skill) -> Result<(), AppError> {
    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;

    let apps = {
        let config = state.config.read()?;
        config
            .skills
            .skills
            .get(&skill.directory)
            .map(|s| s.apps.clone())
            .unwrap_or_else(SkillApps::set_all_enabled)
    };

    service
        .uninstall_skill(skill.directory.clone())
        .map_err(|e| AppError::Message(e.to_string()))?;
    service
        .remove_skill_from_apps(&skill.directory, &apps)
        .map_err(|e| AppError::Message(e.to_string()))?;

    {
        let mut config = state.config.write()?;
        config.skills.skills.remove(&skill.directory);
    }
    state.save()?;

    tui_show_text(
        texts::skills_uninstall(),
        vec![texts::skills_uninstalled(&skill.directory).to_string()],
    )?;
    Ok(())
}

fn show_skill_info_entry(
    skill: &Skill,
    store: &crate::services::skill::SkillStore,
) -> Result<(), AppError> {
    let lines = build_skill_info_lines(skill, store);
    tui_show_text(texts::skills_info_header(), lines)
}

fn manage_repos_menu() -> Result<(), AppError> {
    loop {
        let choices = vec![
            texts::skills_repos_list().to_string(),
            texts::skills_repos_add().to_string(),
            texts::skills_repos_remove().to_string(),
            texts::back_to_main().to_string(),
        ];

        let Some(choice) = prompt_select(texts::skills_repos_header(), choices)? else {
            break;
        };

        if choice == texts::skills_repos_list() {
            list_repos_interactive()?;
        } else if choice == texts::skills_repos_add() {
            add_repo_interactive()?;
        } else if choice == texts::skills_repos_remove() {
            remove_repo_interactive()?;
        } else {
            break;
        }
    }

    Ok(())
}

fn list_repos_interactive() -> Result<(), AppError> {
    let state = get_state()?;
    let store = read_skill_store(&state)?;

    let mut lines = Vec::new();
    if store.repos.is_empty() {
        lines.push(texts::skills_repos_empty().to_string());
    } else {
        for repo in &store.repos {
            lines.push(format!(
                "- {}/{} ({}) {}",
                repo.owner,
                repo.name,
                repo.branch,
                if repo.enabled { "enabled" } else { "disabled" }
            ));
        }
    }
    tui_show_text(texts::skills_repos_header(), lines)
}

fn add_repo_interactive() -> Result<(), AppError> {
    let Some(url) = prompt_text(texts::skills_repo_prompt())? else {
        return Ok(());
    };
    let url = url.trim();
    if url.is_empty() {
        return Ok(());
    }

    let repo = match parse_repo_url(url) {
        Ok(repo) => repo,
        Err(err) => {
            tui_show_text(texts::skills_repos_add(), vec![err.to_string()])?;
            return Ok(());
        }
    };

    run_with_tui_loading(
        texts::skills_repos_add(),
        texts::skills_loading(),
        texts::skills_fetch_timeout(),
        move || {
            let state = get_state()?;
            let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
            {
                let mut config = state.config.write()?;
                service
                    .add_repo(&mut config.skills, repo)
                    .map_err(|e| AppError::Message(e.to_string()))?;
            }
            state.save()?;
            Ok(())
        },
    )?;

    tui_show_text(
        texts::skills_repos_add(),
        vec![texts::skills_repo_added().to_string()],
    )?;
    Ok(())
}

fn remove_repo_interactive() -> Result<(), AppError> {
    let Some(url) = prompt_text(texts::skills_repo_prompt())? else {
        return Ok(());
    };
    let url = url.trim();
    if url.is_empty() {
        return Ok(());
    }

    let repo = match parse_repo_url(url) {
        Ok(repo) => repo,
        Err(err) => {
            tui_show_text(texts::skills_repos_remove(), vec![err.to_string()])?;
            return Ok(());
        }
    };

    let owner = repo.owner;
    let name = repo.name;
    run_with_tui_loading(
        texts::skills_repos_remove(),
        texts::skills_loading(),
        texts::skills_fetch_timeout(),
        move || {
            let state = get_state()?;
            let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
            {
                let mut config = state.config.write()?;
                service
                    .remove_repo(&mut config.skills, owner, name)
                    .map_err(|e| AppError::Message(e.to_string()))?;
            }
            state.save()?;
            Ok(())
        },
    )?;

    tui_show_text(
        texts::skills_repos_remove(),
        vec![texts::skills_repo_removed().to_string()],
    )?;
    Ok(())
}

fn load_skills_for_search(query: &str, store: &SkillStore) -> Result<(Vec<Skill>, bool), AppError> {
    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;

    let fetch = runtime.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(12),
            service.list_skills(store.repos.clone()),
        )
        .await
    });

    let timed_out = fetch.is_err();
    let mut skills = match fetch {
        Ok(result) => result.map_err(|e| AppError::Message(e.to_string()))?,
        Err(_) => local_skills_fallback(store),
    };

    if !query.is_empty() {
        skills.retain(|s| s.matches_query(query));
    }

    Ok((skills, timed_out))
}

fn load_skills_for_search_with_feedback(
    query: &str,
    store: &SkillStore,
) -> Result<(Vec<Skill>, bool), AppError> {
    let query_owned = query.to_string();
    let store_owned = store.clone();
    run_with_tui_loading(
        texts::skills_available_header(),
        texts::skills_loading(),
        texts::skills_fetch_timeout(),
        move || load_skills_for_search(&query_owned, &store_owned),
    )
}

fn local_skills_fallback(store: &SkillStore) -> Vec<Skill> {
    let mut skills: Vec<Skill> = store
        .skills
        .iter()
        .filter(|(_, state)| state.installed)
        .map(|(directory, _)| Skill {
            key: format!("local:{}", directory),
            name: directory.clone(),
            description: texts::skills_showing_local_only().to_string(),
            directory: directory.clone(),
            readme_url: None,
            installed: true,
            repo_owner: None,
            repo_name: None,
            repo_branch: None,
            skills_path: None,
        })
        .collect();

    skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    skills
}

fn build_skill_info_lines(
    skill: &Skill,
    store: &crate::services::skill::SkillStore,
) -> Vec<String> {
    let mut lines = vec![
        format!("{}: {}", texts::skills_label_name(), skill.name),
        format!("{}: {}", texts::skills_label_directory(), skill.directory),
        format!(
            "{}: {}",
            texts::skills_label_description(),
            skill.description
        ),
    ];
    if let Some(url) = &skill.readme_url {
        lines.push(format!("{}: {}", texts::skills_label_readme(), url));
    }
    lines.push(format!(
        "{}: {}",
        texts::skills_label_installed(),
        if skill.installed { "yes" } else { "no" }
    ));

    if skill.installed {
        let apps = store
            .skills
            .get(&skill.directory)
            .map(|s| s.apps.clone())
            .unwrap_or_else(SkillApps::set_all_enabled);
        lines.push(format!(
            "{}: {}",
            texts::skills_label_apps(),
            format_enabled_apps(&apps)
        ));
    }

    lines
}

fn tui_show_text(title: &str, lines: Vec<String>) -> Result<(), AppError> {
    let accent = current_tui_app()
        .map(|app| accent_color(&app))
        .unwrap_or(ratatui::style::Color::Blue);
    let mut screen = TextViewScreen::new(title, lines, texts::press_enter(), accent);
    run_tui_screen(title, &mut screen)?;
    Ok(())
}

fn parse_repo_url(url: &str) -> Result<SkillRepo, AppError> {
    let trimmed = url.trim().trim_end_matches('/');
    let without_prefix = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .unwrap_or(trimmed);
    let parts: Vec<&str> = without_prefix.split('/').collect();
    if parts.len() < 2 {
        return Err(AppError::InvalidInput(
            texts::skills_repo_invalid_url().to_string(),
        ));
    }
    Ok(SkillRepo {
        owner: parts[0].to_string(),
        name: parts[1].to_string(),
        branch: "main".to_string(),
        enabled: true,
        skills_path: None,
    })
}

fn format_enabled_apps(apps: &SkillApps) -> String {
    let mut names = Vec::new();
    if apps.claude {
        names.push("claude");
    }
    if apps.codex {
        names.push("codex");
    }
    if apps.gemini {
        names.push("gemini");
    }
    if apps.opencode {
        names.push("opencode");
    }
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::RwLock;

    use crate::app_config::MultiAppConfig;
    use crate::store::AppState;

    #[test]
    fn reload_search_after_mutation_uses_latest_store_snapshot() {
        let state = AppState {
            config: RwLock::new(MultiAppConfig::default()),
        };

        let stale_store = read_skill_store(&state).expect("should read initial store");

        {
            let mut config = state.config.write().expect("state lock should be writable");
            config.skills.skills.insert(
                "new-skill".to_string(),
                SkillState {
                    installed: true,
                    installed_at: Utc::now(),
                    apps: SkillApps::set_all_enabled(),
                },
            );
        }

        let (refreshed_skills, timed_out) =
            reload_search_skills_after_mutation_with_loader(&state, "", |_query, store| {
                Ok((local_skills_fallback(store), true))
            })
            .expect("reload should succeed");

        assert!(timed_out);
        assert!(
            refreshed_skills
                .iter()
                .any(|skill| skill.directory == "new-skill"),
            "reloaded search should use refreshed state"
        );
        assert!(
            !local_skills_fallback(&stale_store)
                .iter()
                .any(|skill| skill.directory == "new-skill"),
            "stale snapshot should not include mutated entry"
        );
    }
}
