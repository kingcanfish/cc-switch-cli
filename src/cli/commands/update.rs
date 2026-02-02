use crate::cli::i18n as texts;
use crate::cli::ui::colors::{info, success, warning};
use crate::error::AppError;
use crate::services::self_update::{
    extract_version_from_output, select_asset, Os, Platform, SelfUpdateService, Version,
};
use clap::CommandFactory;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub fn execute() -> Result<(), AppError> {
    let platform = Platform::detect();

    match platform.os {
        Os::Windows => {
            println!("{}", warning(texts::text("update_not_supported_windows")));
            return Ok(());
        }
        Os::Macos | Os::Linux => {}
        Os::Other(os) => {
            return Err(AppError::Message(texts::text_with_args(
                "update_unsupported_platform",
                &[("os", &os)],
            )));
        }
    }

    if is_homebrew_install() {
        println!("{}", warning(texts::text("update_homebrew_hint")));
        return Ok(());
    }

    let version_output = crate::cli::Cli::command().render_version();
    let (current_version_str, current_version) = extract_version_from_output(&version_output)?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| AppError::Message(format!("Failed to start runtime: {e}")))?;

    runtime.block_on(async move {
        let service = SelfUpdateService::new()?;
        println!("{}", info(texts::text("update_checking")));

        let release = match service.fetch_latest_release().await {
            Ok(release) => release,
            Err(err) => {
                return Err(AppError::Message(texts::text_with_args(
                    "update_fetch_failed",
                    &[("error", &err.to_string())],
                )));
            }
        };
        let latest_version = Version::parse(&release.tag_name)?;

        println!(
            "{}",
            info(&texts::text_with_args(
                "update_current_version",
                &[("version", &current_version_str)]
            ))
        );
        println!(
            "{}",
            info(&texts::text_with_args(
                "update_latest_version",
                &[("version", &release.tag_name)]
            ))
        );
        println!(
            "{}",
            info(&texts::text_with_args(
                "update_detected_os",
                &[("os", &platform.os_label())]
            ))
        );
        println!(
            "{}",
            info(&texts::text_with_args(
                "update_detected_arch",
                &[("arch", &platform.arch_label())]
            ))
        );

        if latest_version <= current_version {
            println!("{}", info(texts::text("update_up_to_date")));
            return Ok(());
        }

        let asset = select_asset(&release, &platform).ok_or_else(|| {
            AppError::Message(texts::text_with_args(
                "update_no_asset",
                &[
                    ("os", &platform.os_label()),
                    ("arch", &platform.arch_label()),
                ],
            ))
        })?;

        println!(
            "{}",
            info(&texts::text_with_args(
                "update_downloading",
                &[("asset", &asset.name)]
            ))
        );

        let temp_dir = tempfile::TempDir::new()
            .map_err(|e| AppError::Message(format!("Failed to create temp dir: {e}")))?;
        let archive_path = service
            .download_asset(&asset.download_url, temp_dir.path(), &asset.name)
            .await?;
        let extracted = service.extract_binary(&archive_path, temp_dir.path())?;

        println!("{}", info(texts::text("update_installing")));

        if let Err(err) = service.install_binary(&extracted) {
            if let AppError::Io { path, source } = &err {
                if source.kind() == std::io::ErrorKind::PermissionDenied {
                    return Err(AppError::Message(texts::text_with_args(
                        "update_permission_denied",
                        &[("path", path)],
                    )));
                }
            }
            return Err(err);
        }

        println!(
            "{}",
            success(&texts::text_with_args(
                "update_success",
                &[("version", &release.tag_name)]
            ))
        );
        Ok(())
    })
}

fn is_homebrew_install() -> bool {
    let current = match env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    let mut candidates = vec![current.clone()];
    if let Ok(resolved) = fs::canonicalize(&current) {
        if resolved != current {
            candidates.push(resolved);
        }
    }

    candidates
        .into_iter()
        .any(|path| is_homebrew_cellar_path(&path))
}

fn is_homebrew_cellar_path(path: &Path) -> bool {
    for prefix in homebrew_prefixes() {
        let cellar = prefix.join("Cellar").join("cc-switch-cli");
        if path.starts_with(&cellar) {
            return true;
        }
    }
    false
}

fn homebrew_prefixes() -> Vec<PathBuf> {
    let mut prefixes = Vec::new();
    if let Ok(prefix) = env::var("HOMEBREW_PREFIX") {
        if !prefix.is_empty() {
            prefixes.push(PathBuf::from(prefix));
        }
    }
    prefixes.push(PathBuf::from("/opt/homebrew"));
    prefixes.push(PathBuf::from("/usr/local"));
    prefixes.push(PathBuf::from("/home/linuxbrew/.linuxbrew"));
    prefixes
}
