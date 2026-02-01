use crate::error::AppError;
use flate2::read::GzDecoder;
use reqwest::Client;
use semver::Version as SemverVersion;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;

const RELEASES_API_URL: &str =
    "https://api.github.com/repos/kingcanfish/cc-switch-cli/releases/latest";
const BINARY_NAME: &str = "cc-switch-cli";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Os {
    Macos,
    Linux,
    Windows,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arch {
    X64,
    Arm64,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub fn detect() -> Self {
        let os = match env::consts::OS {
            "macos" => Os::Macos,
            "linux" => Os::Linux,
            "windows" => Os::Windows,
            other => Os::Other(other.to_string()),
        };

        let arch = match env::consts::ARCH {
            "x86_64" => Arch::X64,
            "aarch64" => Arch::Arm64,
            other => Arch::Other(other.to_string()),
        };

        Self { os, arch }
    }

    pub fn arch_label(&self) -> String {
        match &self.arch {
            Arch::X64 => "x64".to_string(),
            Arch::Arm64 => "arm64".to_string(),
            Arch::Other(value) => value.clone(),
        }
    }

    pub fn os_label(&self) -> String {
        match &self.os {
            Os::Macos => "macos".to_string(),
            Os::Linux => "linux".to_string(),
            Os::Windows => "windows".to_string(),
            Os::Other(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub SemverVersion);

impl Version {
    pub fn parse(raw: &str) -> Result<Self, AppError> {
        let trimmed = raw.trim().trim_start_matches('v');
        let parsed = SemverVersion::parse(trimmed)
            .map_err(|_| AppError::Message(format!("Invalid version: {raw}")))?;
        Ok(Self(parsed))
    }
}

pub fn extract_version_from_output(output: &str) -> Result<(String, Version), AppError> {
    let version = extract_version_token(output)
        .ok_or_else(|| AppError::Message(format!("Unable to parse version from: {output}")))?;
    let parsed = Version::parse(&version)?;
    Ok((version, parsed))
}

fn extract_version_token(output: &str) -> Option<String> {
    let re = regex::Regex::new(r"v?\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?").ok()?;
    re.find(output).map(|m| m.as_str().to_string())
}

pub fn select_asset(release: &Release, platform: &Platform) -> Option<ReleaseAsset> {
    let tag = release.tag_name.as_str();
    let find_asset = |name: String| {
        release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .cloned()
    };

    match platform.os {
        Os::Macos => {
            let arch_asset = match platform.arch {
                Arch::Arm64 => Some(format!("cc-switch-cli-{tag}-darwin-arm64.tar.gz")),
                Arch::X64 => Some(format!("cc-switch-cli-{tag}-darwin-x64.tar.gz")),
                Arch::Other(_) => None,
            };
            if let Some(name) = arch_asset {
                if let Some(asset) = find_asset(name) {
                    return Some(asset);
                }
            }

            find_asset(format!("cc-switch-cli-{tag}-darwin-universal.tar.gz"))
        }
        Os::Linux => {
            let arch = match platform.arch {
                Arch::Arm64 => "arm64",
                Arch::X64 => "x64",
                Arch::Other(_) => return None,
            };
            let musl = format!("cc-switch-cli-{tag}-linux-{arch}-musl.tar.gz");
            if let Some(asset) = find_asset(musl) {
                return Some(asset);
            }
            find_asset(format!("cc-switch-cli-{tag}-linux-{arch}.tar.gz"))
        }
        _ => None,
    }
}

pub struct SelfUpdateService {
    client: Client,
}

impl SelfUpdateService {
    pub fn new() -> Result<Self, AppError> {
        let client = Client::builder()
            .user_agent("cc-switch-cli")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| AppError::Message(format!("Failed to build HTTP client: {e}")))?;
        Ok(Self { client })
    }

    pub async fn fetch_latest_release(&self) -> Result<Release, AppError> {
        let response = self
            .client
            .get(RELEASES_API_URL)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| AppError::Message(format!("Failed to fetch release: {e}")))?;
        let response = response
            .error_for_status()
            .map_err(|e| AppError::Message(format!("Failed to fetch release: {e}")))?;
        response
            .json::<Release>()
            .await
            .map_err(|e| AppError::Message(format!("Failed to parse release: {e}")))
    }

    pub async fn download_asset(
        &self,
        url: &str,
        dest_dir: &Path,
        name: &str,
    ) -> Result<PathBuf, AppError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::Message(format!("Download failed: {e}")))?;
        let response = response
            .error_for_status()
            .map_err(|e| AppError::Message(format!("Download failed: {e}")))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::Message(format!("Download failed: {e}")))?;

        let archive_path = dest_dir.join(name);
        fs::write(&archive_path, &bytes).map_err(|e| AppError::io(&archive_path, e))?;
        Ok(archive_path)
    }

    pub fn extract_binary(
        &self,
        archive_path: &Path,
        dest_dir: &Path,
    ) -> Result<PathBuf, AppError> {
        let file = fs::File::open(archive_path).map_err(|e| AppError::io(archive_path, e))?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        for entry in archive
            .entries()
            .map_err(|e| AppError::Message(format!("Failed to read archive: {e}")))?
        {
            let mut entry =
                entry.map_err(|e| AppError::Message(format!("Failed to read archive: {e}")))?;
            let path = entry
                .path()
                .map_err(|e| AppError::Message(format!("Failed to read archive: {e}")))?;
            if path.file_name().and_then(|n| n.to_str()) == Some(BINARY_NAME) {
                let output_path = dest_dir.join(BINARY_NAME);
                entry
                    .unpack(&output_path)
                    .map_err(|e| AppError::io(&output_path, e))?;
                let mut perms = fs::metadata(&output_path)
                    .map_err(|e| AppError::io(&output_path, e))?
                    .permissions();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    perms.set_mode(0o755);
                }
                fs::set_permissions(&output_path, perms)
                    .map_err(|e| AppError::io(&output_path, e))?;
                return Ok(output_path);
            }
        }

        Err(AppError::Message(
            "Failed to locate binary in archive".to_string(),
        ))
    }

    pub fn install_binary(&self, new_binary: &Path) -> Result<(), AppError> {
        let current = env::current_exe()
            .map_err(|e| AppError::Message(format!("Failed to locate current executable: {e}")))?;

        let backup = current.with_extension("bak");
        if backup.exists() {
            fs::remove_file(&backup).map_err(|e| AppError::io(&backup, e))?;
        }

        if let Err(err) = fs::rename(&current, &backup) {
            if err.kind() == io::ErrorKind::PermissionDenied {
                return Err(AppError::io(&current, err));
            }
            return Err(AppError::io(&current, err));
        }

        if let Err(err) = fs::copy(new_binary, &current) {
            let _ = fs::rename(&backup, &current);
            return Err(AppError::io(&current, err));
        }

        if let Err(err) = fs::remove_file(&backup) {
            log::warn!("Failed to remove backup: {}", err);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> ReleaseAsset {
        ReleaseAsset {
            name: name.to_string(),
            download_url: format!("https://example.com/{name}"),
        }
    }

    #[test]
    fn parse_version_from_output_basic() {
        let (version_str, version) = extract_version_from_output("cc-switch-cli 1.2.3").unwrap();
        assert_eq!(version_str, "1.2.3");
        assert_eq!(version.0, SemverVersion::new(1, 2, 3));
    }

    #[test]
    fn parse_version_preserves_prerelease() {
        let parsed = Version::parse("v2.10.4-beta.1").unwrap();
        assert_eq!(parsed.0, SemverVersion::parse("2.10.4-beta.1").unwrap());
    }

    #[test]
    fn parse_version_from_output_with_prerelease() {
        let (version_str, version) =
            extract_version_from_output("cc-switch-cli 0.0.2-beta.1+build.7").unwrap();
        assert_eq!(version_str, "0.0.2-beta.1+build.7");
        assert_eq!(
            version.0,
            SemverVersion::parse("0.0.2-beta.1+build.7").unwrap()
        );
    }

    #[test]
    fn select_asset_prefers_macos_arch() {
        let release = Release {
            tag_name: "v1.2.3".to_string(),
            assets: vec![
                asset("cc-switch-cli-v1.2.3-darwin-universal.tar.gz"),
                asset("cc-switch-cli-v1.2.3-darwin-arm64.tar.gz"),
            ],
        };
        let platform = Platform {
            os: Os::Macos,
            arch: Arch::Arm64,
        };
        let selected = select_asset(&release, &platform).unwrap();
        assert_eq!(selected.name, "cc-switch-cli-v1.2.3-darwin-arm64.tar.gz");
    }

    #[test]
    fn select_asset_prefers_linux_musl() {
        let release = Release {
            tag_name: "v1.2.3".to_string(),
            assets: vec![
                asset("cc-switch-cli-v1.2.3-linux-x64.tar.gz"),
                asset("cc-switch-cli-v1.2.3-linux-x64-musl.tar.gz"),
            ],
        };
        let platform = Platform {
            os: Os::Linux,
            arch: Arch::X64,
        };
        let selected = select_asset(&release, &platform).unwrap();
        assert_eq!(selected.name, "cc-switch-cli-v1.2.3-linux-x64-musl.tar.gz");
    }

    #[test]
    fn parse_release_fixture() {
        let json = r#"
        {
            "tag_name": "v1.2.3",
            "assets": [
                {"name": "cc-switch-cli-v1.2.3-linux-x64-musl.tar.gz", "browser_download_url": "https://example.com/a"}
            ]
        }
        "#;
        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.2.3");
        assert_eq!(release.assets.len(), 1);
    }
}
