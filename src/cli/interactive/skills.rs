use chrono::Utc;
use tokio::runtime::Runtime;

use crate::cli::i18n::texts;
use crate::cli::ui::{error, highlight, info, success};
use crate::error::AppError;
use crate::services::skill::{Skill, SkillApps, SkillRepo, SkillService, SkillState};

use super::utils::{clear_screen, get_state, pause, prompt_select, prompt_text};

pub fn manage_skills_menu() -> Result<(), AppError> {
    loop {
        clear_screen();
        println!("\n{}", highlight(texts::skills_management()));
        println!("{}", "─".repeat(60));

        let choices = vec![
            texts::skills_list(),
            texts::skills_search(),
            texts::skills_install(),
            texts::skills_uninstall(),
            texts::skills_info(),
            texts::skills_repos(),
            texts::back_to_main(),
        ];

        let Some(choice) = prompt_select(texts::choose_action(), choices)? else {
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

fn list_skills_interactive() -> Result<(), AppError> {
    clear_screen();
    let state = get_state()?;
    let store = {
        let config = state.config.read()?;
        config.skills.clone()
    };

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    let installed: Vec<_> = skills.into_iter().filter(|s| s.installed).collect();
    println!("\n{}", highlight(texts::skills_installed_header()));
    println!("{}", "─".repeat(60));

    if installed.is_empty() {
        println!("{}", info(texts::skills_none_installed()));
        pause();
        return Ok(());
    }

    for skill in installed {
        let apps = store
            .skills
            .get(&skill.directory)
            .map(|s| s.apps.clone())
            .unwrap_or_else(SkillApps::set_all_enabled);
        println!(
            "- {} ({}) [{}]",
            skill.name,
            skill.directory,
            format_enabled_apps(&apps)
        );
    }

    pause();
    Ok(())
}

fn search_skills_interactive() -> Result<(), AppError> {
    clear_screen();
    let Some(query) = prompt_text(texts::skills_search_prompt())? else {
        return Ok(());
    };

    let query = query.trim().to_string();

    let state = get_state()?;
    let store = {
        let config = state.config.read()?;
        config.skills.clone()
    };

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;

    let load_skills = || -> Result<Vec<Skill>, AppError> {
        let mut skills = runtime
            .block_on(async { service.list_skills(store.repos.clone()).await })
            .map_err(|e| AppError::Message(e.to_string()))?;
        if !query.is_empty() {
            skills.retain(|s| s.matches_query(&query));
        }
        Ok(skills)
    };

    let mut skills = load_skills()?;

    if skills.is_empty() {
        println!("\n{}", highlight(texts::skills_available_header()));
        println!("{}", "─".repeat(60));
        println!("{}", info(texts::skills_none_found()));
        pause();
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
        println!("\n{}", highlight(texts::skills_available_header()));
        println!("{}", "─".repeat(60));

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

        let Some(selected) = prompt_select(texts::skills_select_prompt(), choices)? else {
            return Ok(());
        };

        let Some(skill) = skills.iter().find(|s| s.directory == selected.directory) else {
            continue;
        };

        loop {
            clear_screen();
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
                clear_screen();
                break;
            }

            if action == texts::skills_install() {
                install_skill_entry(&state, skill)?;
                skills = load_skills()?;
                break;
            } else if action == texts::skills_uninstall() {
                uninstall_skill_entry(&state, skill)?;
                skills = load_skills()?;
                break;
            } else if action == texts::skills_info() {
                show_skill_info_entry(skill, &store)?;
                pause();
            }
        }
    }
}

fn install_skill_interactive() -> Result<(), AppError> {
    clear_screen();
    let Some(name) = prompt_text(texts::skills_name_prompt())? else {
        return Ok(());
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(());
    }

    let state = get_state()?;
    let store = {
        let config = state.config.read()?;
        config.skills.clone()
    };

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    let Some(skill) = skills.iter().find(|s| {
        s.key.eq_ignore_ascii_case(&name)
            || s.directory.eq_ignore_ascii_case(&name)
            || s.name.eq_ignore_ascii_case(&name)
    }) else {
        println!("{}", error(&texts::skills_not_found(&name)));
        pause();
        return Ok(());
    };

    let (owner, repo_name) = match (skill.repo_owner.as_deref(), skill.repo_name.as_deref()) {
        (Some(owner), Some(repo_name)) if !owner.is_empty() && !repo_name.is_empty() => {
            (owner.to_string(), repo_name.to_string())
        }
        _ => {
            println!("{}", error(texts::skills_install_missing_repo()));
            pause();
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

    runtime
        .block_on(async { service.install_skill(skill.directory.clone(), repo).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

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

    service
        .sync_skill_to_apps(&skill.directory, &apps)
        .map_err(|e| AppError::Message(e.to_string()))?;

    println!("{}", success(&texts::skills_installed(&skill.name)));
    pause();
    Ok(())
}

fn uninstall_skill_interactive() -> Result<(), AppError> {
    clear_screen();
    let Some(name) = prompt_text(texts::skills_name_prompt())? else {
        return Ok(());
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(());
    }

    let state = get_state()?;
    let store = {
        let config = state.config.read()?;
        config.skills.clone()
    };

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    let Some(skill) = skills.iter().find(|s| {
        s.installed
            && (s.key.eq_ignore_ascii_case(&name)
                || s.directory.eq_ignore_ascii_case(&name)
                || s.name.eq_ignore_ascii_case(&name))
    }) else {
        println!("{}", error(&texts::skills_not_installed(&name)));
        pause();
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

    println!("{}", success(&texts::skills_uninstalled(&directory)));
    pause();
    Ok(())
}

fn show_skill_info_interactive() -> Result<(), AppError> {
    clear_screen();
    let Some(name) = prompt_text(texts::skills_name_prompt())? else {
        return Ok(());
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(());
    }

    let state = get_state()?;
    let store = {
        let config = state.config.read()?;
        config.skills.clone()
    };

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    let Some(skill) = skills.iter().find(|s| {
        s.key.eq_ignore_ascii_case(&name)
            || s.directory.eq_ignore_ascii_case(&name)
            || s.name.eq_ignore_ascii_case(&name)
    }) else {
        println!("{}", error(&texts::skills_not_found(&name)));
        pause();
        return Ok(());
    };

    println!("\n{}", highlight(texts::skills_info_header()));
    println!("{}", "─".repeat(60));
    println!("{}: {}", texts::skills_label_name(), skill.name);
    println!("{}: {}", texts::skills_label_directory(), skill.directory);
    println!(
        "{}: {}",
        texts::skills_label_description(),
        skill.description
    );
    if let Some(url) = &skill.readme_url {
        println!("{}: {}", texts::skills_label_readme(), url);
    }
    println!(
        "{}: {}",
        texts::skills_label_installed(),
        if skill.installed { "yes" } else { "no" }
    );

    if skill.installed {
        let apps = store
            .skills
            .get(&skill.directory)
            .map(|s| s.apps.clone())
            .unwrap_or_else(SkillApps::set_all_enabled);
        println!(
            "{}: {}",
            texts::skills_label_apps(),
            format_enabled_apps(&apps)
        );
    }

    pause();
    Ok(())
}

fn install_skill_entry(state: &crate::store::AppState, skill: &Skill) -> Result<(), AppError> {
    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;

    let (owner, repo_name) = match (skill.repo_owner.as_deref(), skill.repo_name.as_deref()) {
        (Some(owner), Some(repo_name)) if !owner.is_empty() && !repo_name.is_empty() => {
            (owner.to_string(), repo_name.to_string())
        }
        _ => {
            println!("{}", error(texts::skills_install_missing_repo()));
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

    runtime
        .block_on(async { service.install_skill(skill.directory.clone(), repo).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

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

    service
        .sync_skill_to_apps(&skill.directory, &apps)
        .map_err(|e| AppError::Message(e.to_string()))?;

    println!("{}", success(&texts::skills_installed(&skill.name)));
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

    println!("{}", success(&texts::skills_uninstalled(&skill.directory)));
    Ok(())
}

fn show_skill_info_entry(
    skill: &Skill,
    store: &crate::services::skill::SkillStore,
) -> Result<(), AppError> {
    println!("\n{}", highlight(texts::skills_info_header()));
    println!("{}", "─".repeat(60));
    println!("{}: {}", texts::skills_label_name(), skill.name);
    println!("{}: {}", texts::skills_label_directory(), skill.directory);
    println!(
        "{}: {}",
        texts::skills_label_description(),
        skill.description
    );
    if let Some(url) = &skill.readme_url {
        println!("{}: {}", texts::skills_label_readme(), url);
    }
    println!(
        "{}: {}",
        texts::skills_label_installed(),
        if skill.installed { "yes" } else { "no" }
    );

    if skill.installed {
        let apps = store
            .skills
            .get(&skill.directory)
            .map(|s| s.apps.clone())
            .unwrap_or_else(SkillApps::set_all_enabled);
        println!(
            "{}: {}",
            texts::skills_label_apps(),
            format_enabled_apps(&apps)
        );
    }

    Ok(())
}

fn manage_repos_menu() -> Result<(), AppError> {
    loop {
        clear_screen();
        println!("\n{}", highlight(texts::skills_repos_header()));
        println!("{}", "─".repeat(60));

        let choices = vec![
            texts::skills_repos_list(),
            texts::skills_repos_add(),
            texts::skills_repos_remove(),
            texts::back_to_main(),
        ];

        let Some(choice) = prompt_select(texts::choose_action(), choices)? else {
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
    clear_screen();
    let state = get_state()?;
    let store = {
        let config = state.config.read()?;
        config.skills.clone()
    };

    println!("\n{}", highlight(texts::skills_repos_header()));
    println!("{}", "─".repeat(60));

    if store.repos.is_empty() {
        println!("{}", info(texts::skills_repos_empty()));
        pause();
        return Ok(());
    }

    for repo in &store.repos {
        println!(
            "- {}/{} ({}) {}",
            repo.owner,
            repo.name,
            repo.branch,
            if repo.enabled { "enabled" } else { "disabled" }
        );
    }

    pause();
    Ok(())
}

fn add_repo_interactive() -> Result<(), AppError> {
    clear_screen();
    let Some(url) = prompt_text(texts::skills_repo_prompt())? else {
        return Ok(());
    };
    let url = url.trim();
    if url.is_empty() {
        return Ok(());
    }

    let repo = parse_repo_url(url)?;
    let state = get_state()?;
    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    {
        let mut config = state.config.write()?;
        service
            .add_repo(&mut config.skills, repo)
            .map_err(|e| AppError::Message(e.to_string()))?;
    }
    state.save()?;

    println!("{}", success(texts::skills_repo_added()));
    pause();
    Ok(())
}

fn remove_repo_interactive() -> Result<(), AppError> {
    clear_screen();
    let Some(url) = prompt_text(texts::skills_repo_prompt())? else {
        return Ok(());
    };
    let url = url.trim();
    if url.is_empty() {
        return Ok(());
    }

    let repo = parse_repo_url(url)?;
    let state = get_state()?;
    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    {
        let mut config = state.config.write()?;
        service
            .remove_repo(&mut config.skills, repo.owner, repo.name)
            .map_err(|e| AppError::Message(e.to_string()))?;
    }
    state.save()?;

    println!("{}", success(texts::skills_repo_removed()));
    pause();
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
