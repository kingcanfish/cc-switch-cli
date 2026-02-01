use crate::error::AppError;
use crate::services::skill::{SkillApps, SkillRepo, SkillService, SkillState};
use crate::store::AppState;
use crate::MultiAppConfig;
use chrono::Utc;
use clap::Subcommand;
use std::sync::RwLock;
use tokio::runtime::Runtime;

#[derive(Subcommand)]
pub enum SkillsCommand {
    /// List installed skills
    List,
    /// Search for available skills
    Search {
        /// Search query (optional)
        query: Option<String>,
    },
    /// Install a skill
    Install {
        /// Skill name or URL
        name: String,
    },
    /// Uninstall a skill
    Uninstall {
        /// Skill name
        name: String,
    },
    /// Show skill information
    Info {
        /// Skill name
        name: String,
    },
    /// Manage skill repositories
    #[command(subcommand)]
    Repos(SkillReposCommand),
}

#[derive(Subcommand)]
pub enum SkillReposCommand {
    /// List all repositories
    List,
    /// Add a repository
    Add {
        /// Repository URL
        url: String,
    },
    /// Remove a repository
    Remove {
        /// Repository URL
        url: String,
    },
    /// Update repository index
    Update,
}

pub fn execute(cmd: SkillsCommand) -> Result<(), AppError> {
    match cmd {
        SkillsCommand::List => list_skills(),
        SkillsCommand::Search { query } => search_skills(query.as_deref()),
        SkillsCommand::Install { name } => install_skill(&name),
        SkillsCommand::Uninstall { name } => uninstall_skill(&name),
        SkillsCommand::Info { name } => show_skill_info(&name),
        SkillsCommand::Repos(repos_cmd) => execute_repos(repos_cmd),
    }
}

fn get_state() -> Result<AppState, AppError> {
    let config = MultiAppConfig::load()?;
    Ok(AppState {
        config: RwLock::new(config),
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

fn list_skills() -> Result<(), AppError> {
    let state = get_state()?;
    let config = state.config.read()?;
    let store = config.skills.clone();
    drop(config);

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    let installed: Vec<_> = skills.into_iter().filter(|s| s.installed).collect();
    if installed.is_empty() {
        println!("No skills installed.");
        return Ok(());
    }

    println!("Installed skills:");
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
    Ok(())
}

fn search_skills(_query: Option<&str>) -> Result<(), AppError> {
    let state = get_state()?;
    let config = state.config.read()?;
    let store = config.skills.clone();
    drop(config);

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let mut skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    if let Some(query) = _query {
        skills.retain(|s| s.matches_query(query));
    }

    if skills.is_empty() {
        println!("No skills found.");
        return Ok(());
    }

    println!("Skills:");
    for skill in skills {
        let installed_marker = if skill.installed { "✓" } else { " " };
        println!(
            "[{}] {} ({})",
            installed_marker, skill.name, skill.directory
        );
    }
    Ok(())
}

fn install_skill(_name: &str) -> Result<(), AppError> {
    let state = get_state()?;
    let config = state.config.write()?;
    let store = config.skills.clone();
    drop(config);

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    let target = skills.iter().find(|s| {
        s.key.eq_ignore_ascii_case(_name)
            || s.directory.eq_ignore_ascii_case(_name)
            || s.name.eq_ignore_ascii_case(_name)
    });

    let Some(skill) = target else {
        return Err(AppError::Message(format!("Skill not found: {}", _name)));
    };

    let (owner, name) = match (skill.repo_owner.as_deref(), skill.repo_name.as_deref()) {
        (Some(owner), Some(name)) if !owner.is_empty() && !name.is_empty() => {
            (owner.to_string(), name.to_string())
        }
        _ => {
            return Err(AppError::Message(
                "Skill is local or missing repo metadata; cannot install.".to_string(),
            ))
        }
    };

    let repo = SkillRepo {
        owner,
        name,
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
        let mut guard = state.config.write()?;
        let entry = guard
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

    let guard = state.config.read()?;
    let apps = guard
        .skills
        .skills
        .get(&skill.directory)
        .map(|s| s.apps.clone())
        .unwrap_or_else(SkillApps::set_all_enabled);
    drop(guard);

    service
        .sync_skill_to_apps(&skill.directory, &apps)
        .map_err(|e| AppError::Message(e.to_string()))?;

    println!("Installed skill: {}", skill.name);
    Ok(())
}

fn uninstall_skill(_name: &str) -> Result<(), AppError> {
    let state = get_state()?;
    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;

    let config = state.config.read()?;
    let store = config.skills.clone();
    drop(config);

    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    let target = skills.iter().find(|s| {
        s.installed
            && (s.key.eq_ignore_ascii_case(_name)
                || s.directory.eq_ignore_ascii_case(_name)
                || s.name.eq_ignore_ascii_case(_name))
    });

    let Some(skill) = target else {
        return Err(AppError::Message(format!(
            "Installed skill not found: {}",
            _name
        )));
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

    println!("Uninstalled skill: {}", directory);
    Ok(())
}

fn show_skill_info(_name: &str) -> Result<(), AppError> {
    let state = get_state()?;
    let config = state.config.read()?;
    let store = config.skills.clone();
    drop(config);

    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    let runtime = Runtime::new().map_err(|e| AppError::Message(e.to_string()))?;
    let skills = runtime
        .block_on(async { service.list_skills(store.repos.clone()).await })
        .map_err(|e| AppError::Message(e.to_string()))?;

    let Some(skill) = skills.iter().find(|s| {
        s.key.eq_ignore_ascii_case(_name)
            || s.directory.eq_ignore_ascii_case(_name)
            || s.name.eq_ignore_ascii_case(_name)
    }) else {
        return Err(AppError::Message(format!("Skill not found: {}", _name)));
    };

    println!("Name: {}", skill.name);
    println!("Directory: {}", skill.directory);
    println!("Description: {}", skill.description);
    if let Some(url) = &skill.readme_url {
        println!("Readme: {}", url);
    }
    println!("Installed: {}", if skill.installed { "yes" } else { "no" });
    Ok(())
}

fn execute_repos(cmd: SkillReposCommand) -> Result<(), AppError> {
    match cmd {
        SkillReposCommand::List => list_repos(),
        SkillReposCommand::Add { url } => add_repo(&url),
        SkillReposCommand::Remove { url } => remove_repo(&url),
        SkillReposCommand::Update => update_repos(),
    }
}

fn list_repos() -> Result<(), AppError> {
    let state = get_state()?;
    let config = state.config.read()?;
    if config.skills.repos.is_empty() {
        println!("No repositories configured.");
        return Ok(());
    }

    println!("Repositories:");
    for repo in &config.skills.repos {
        println!(
            "- {}/{} ({}) {}",
            repo.owner,
            repo.name,
            repo.branch,
            if repo.enabled { "enabled" } else { "disabled" }
        );
    }
    Ok(())
}

fn add_repo(_url: &str) -> Result<(), AppError> {
    let repo = parse_repo_url(_url)?;
    let state = get_state()?;
    let mut config = state.config.write()?;
    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    service
        .add_repo(&mut config.skills, repo)
        .map_err(|e| AppError::Message(e.to_string()))?;
    drop(config);
    state.save()?;
    println!("Repository added.");
    Ok(())
}

fn remove_repo(_url: &str) -> Result<(), AppError> {
    let repo = parse_repo_url(_url)?;
    let state = get_state()?;
    let mut config = state.config.write()?;
    let service = SkillService::new().map_err(|e| AppError::Message(e.to_string()))?;
    service
        .remove_repo(&mut config.skills, repo.owner, repo.name)
        .map_err(|e| AppError::Message(e.to_string()))?;
    drop(config);
    state.save()?;
    println!("Repository removed.");
    Ok(())
}

fn update_repos() -> Result<(), AppError> {
    let state = get_state()?;
    let config = state.config.read()?;
    if config.skills.repos.is_empty() {
        println!("No repositories configured.");
        return Ok(());
    }
    println!("Repositories updated.");
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
        return Err(AppError::InvalidInput("Invalid repository URL".to_string()));
    }
    Ok(SkillRepo {
        owner: parts[0].to_string(),
        name: parts[1].to_string(),
        branch: "main".to_string(),
        enabled: true,
        skills_path: None,
    })
}
