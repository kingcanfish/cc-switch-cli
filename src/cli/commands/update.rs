use crate::cli::i18n as texts;
use crate::cli::ui::colors::{info, success, warning};
use crate::error::AppError;
use crate::services::self_update::{
    extract_version_from_output, select_asset, Os, Platform, SelfUpdateService, Version,
};
use clap::CommandFactory;

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

    let version_output = crate::cli::Cli::command().render_version();
    let (current_version_str, current_version) = extract_version_from_output(&version_output)?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| AppError::Message(format!("Failed to start runtime: {e}")))?;

    runtime.block_on(async move {
        let service = SelfUpdateService::new()?;
        println!("{}", info(texts::text("update_checking")));

        let release = service.fetch_latest_release().await?;
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
