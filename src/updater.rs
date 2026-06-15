use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/stoiandan/requiescat/releases/latest/download/release-manifest.json";

#[derive(Debug, Clone)]
pub struct AvailableUpdate {
    pub version: String,
    pub notes_url: String,
    asset: ReleaseAsset,
}

#[derive(Debug, Clone)]
pub struct StagedUpdate {
    pub version: String,
    archive: PathBuf,
    format: ArchiveFormat,
}

#[derive(Debug)]
pub enum UpdateError {
    Http(reqwest::Error),
    Io(std::io::Error),
    InvalidManifest(String),
    UnsupportedPlatform,
    ChecksumMismatch,
}

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::InvalidManifest(message) => formatter.write_str(message),
            Self::UnsupportedPlatform => {
                formatter.write_str("No update package is available for this platform.")
            }
            Self::ChecksumMismatch => {
                formatter.write_str("The downloaded update failed checksum verification.")
            }
        }
    }
}

impl From<reqwest::Error> for UpdateError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<std::io::Error> for UpdateError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Deserialize)]
struct ReleaseManifest {
    version: String,
    notes_url: String,
    assets: HashMap<String, ReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseAsset {
    url: String,
    sha256: String,
    file_name: String,
    format: ArchiveFormat,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ArchiveFormat {
    Zip,
    TarGz,
}

pub async fn check_for_update() -> Result<Option<AvailableUpdate>, UpdateError> {
    let manifest_url =
        option_env!("REQUIESCAT_UPDATE_MANIFEST_URL").unwrap_or(DEFAULT_MANIFEST_URL);
    let client = reqwest::Client::builder()
        .user_agent(concat!("requiescat/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let manifest = client
        .get(manifest_url)
        .send()
        .await?
        .error_for_status()?
        .json::<ReleaseManifest>()
        .await?;

    let available_version = parse_version(&manifest.version)?;
    let current_version = parse_version(env!("CARGO_PKG_VERSION"))?;
    if available_version <= current_version {
        return Ok(None);
    }

    let asset = manifest
        .assets
        .get(platform_key())
        .cloned()
        .ok_or(UpdateError::UnsupportedPlatform)?;

    Ok(Some(AvailableUpdate {
        version: available_version.to_string(),
        notes_url: manifest.notes_url,
        asset,
    }))
}

pub async fn download_update(update: AvailableUpdate) -> Result<StagedUpdate, UpdateError> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("requiescat/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let bytes = client
        .get(&update.asset.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let checksum = format!("{:x}", Sha256::digest(&bytes));
    if !checksum.eq_ignore_ascii_case(update.asset.sha256.trim()) {
        return Err(UpdateError::ChecksumMismatch);
    }

    let directory = crate::persistence::application_data_directory()?
        .join("updates")
        .join(&update.version);
    fs::create_dir_all(&directory)?;
    let archive = directory.join(&update.asset.file_name);
    fs::write(&archive, bytes)?;

    Ok(StagedUpdate {
        version: update.version,
        archive,
        format: update.asset.format,
    })
}

pub fn open_release_notes(url: &str) -> Result<(), UpdateError> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler").arg(url);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return Err(UpdateError::UnsupportedPlatform);

    command.spawn()?;
    Ok(())
}

pub fn install_and_restart(update: &StagedUpdate) -> Result<(), UpdateError> {
    let executable = std::env::current_exe()?;
    let script_directory = crate::persistence::application_data_directory()?.join("updates");
    fs::create_dir_all(&script_directory)?;

    #[cfg(target_os = "windows")]
    {
        ensure_installation_writable(&executable)?;
        install_windows(update, &executable, &script_directory)?;
    }

    #[cfg(target_os = "macos")]
    {
        let app_bundle = macos_app_bundle(&executable)?;
        ensure_installation_writable(app_bundle)?;
        install_macos(update, app_bundle, &script_directory)?;
    }

    #[cfg(target_os = "linux")]
    {
        ensure_installation_writable(&executable)?;
        install_linux(update, &executable, &script_directory)?;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return Err(UpdateError::UnsupportedPlatform);

    Ok(())
}

fn ensure_installation_writable(target: &Path) -> Result<(), UpdateError> {
    let directory = target.parent().ok_or_else(|| {
        UpdateError::InvalidManifest("The application directory is unavailable.".to_owned())
    })?;
    let probe = directory.join(format!(".requiescat-update-check-{}", std::process::id()));
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)?;
    fs::remove_file(probe)?;
    Ok(())
}

fn parse_version(value: &str) -> Result<Version, UpdateError> {
    Version::parse(value.trim().trim_start_matches('v')).map_err(|error| {
        UpdateError::InvalidManifest(format!("Invalid release version {value:?}: {error}"))
    })
}

fn platform_key() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows-x86_64"
    }
    #[cfg(target_os = "macos")]
    {
        "macos-universal"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64"
    }
    #[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
    {
        "linux-unsupported"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "unsupported"
    }
}

#[cfg(target_os = "windows")]
fn install_windows(
    update: &StagedUpdate,
    executable: &Path,
    directory: &Path,
) -> Result<(), UpdateError> {
    if !matches!(update.format, ArchiveFormat::Zip) {
        return Err(UpdateError::InvalidManifest(
            "Windows updates must use ZIP archives.".to_owned(),
        ));
    }

    let script = directory.join("install-update.ps1");
    fs::write(
        &script,
        r#"param([string]$Archive, [string]$Executable, [int]$ParentProcessId)
$ErrorActionPreference = "Stop"
while (Get-Process -Id $ParentProcessId -ErrorAction SilentlyContinue) {
    Start-Sleep -Milliseconds 200
}
$staging = Join-Path ([System.IO.Path]::GetTempPath()) ("requiescat-update-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $staging | Out-Null
try {
    Expand-Archive -LiteralPath $Archive -DestinationPath $staging -Force
    $replacement = Join-Path $staging "requiescat\requiescat.exe"
    Copy-Item -LiteralPath $replacement -Destination ($Executable + ".new") -Force
    Move-Item -LiteralPath ($Executable + ".new") -Destination $Executable -Force
    Start-Process -FilePath $Executable
} finally {
    Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Archive -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $PSCommandPath -Force -ErrorAction SilentlyContinue
}
"#,
    )?;

    use std::os::windows::process::CommandExt;
    Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg(&update.archive)
        .arg(executable)
        .arg(std::process::id().to_string())
        .creation_flags(0x0000_0008 | 0x0800_0000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux(
    update: &StagedUpdate,
    executable: &Path,
    directory: &Path,
) -> Result<(), UpdateError> {
    if !matches!(update.format, ArchiveFormat::TarGz) {
        return Err(UpdateError::InvalidManifest(
            "Linux updates must use tar.gz archives.".to_owned(),
        ));
    }

    let script = directory.join("install-update.sh");
    write_unix_script(
        &script,
        r#"#!/bin/sh
set -eu
archive=$1
executable=$2
parent_pid=$3
while kill -0 "$parent_pid" 2>/dev/null; do
    sleep 1
done
staging=$(mktemp -d)
trap 'rm -rf "$staging"; rm -f "$archive" "$0"' EXIT
tar -xzf "$archive" -C "$staging"
cp "$staging/requiescat/requiescat" "$executable.new"
chmod +x "$executable.new"
mv -f "$executable.new" "$executable"
"$executable" >/dev/null 2>&1 &
"#,
    )?;
    spawn_unix_script(&script, &update.archive, executable)
}

#[cfg(target_os = "macos")]
fn install_macos(
    update: &StagedUpdate,
    app_bundle: &Path,
    directory: &Path,
) -> Result<(), UpdateError> {
    if !matches!(update.format, ArchiveFormat::Zip) {
        return Err(UpdateError::InvalidManifest(
            "macOS updates must use ZIP archives.".to_owned(),
        ));
    }

    let script = directory.join("install-update.sh");
    write_unix_script(
        &script,
        r#"#!/bin/sh
set -eu
archive=$1
app=$2
parent_pid=$3
while kill -0 "$parent_pid" 2>/dev/null; do
    sleep 1
done
staging=$(mktemp -d)
old_app="$app.previous"
trap 'rm -rf "$staging"; rm -f "$archive" "$0"' EXIT
/usr/bin/ditto -x -k "$archive" "$staging"
rm -rf "$old_app"
mv "$app" "$old_app"
if mv "$staging/Requiescat.app" "$app"; then
    open "$app"
    rm -rf "$old_app"
else
    mv "$old_app" "$app"
    exit 1
fi
"#,
    )?;
    spawn_unix_script(&script, &update.archive, app_bundle)
}

#[cfg(target_os = "macos")]
fn macos_app_bundle(executable: &Path) -> Result<&Path, UpdateError> {
    executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .ok_or_else(|| {
            UpdateError::InvalidManifest(
                "The running executable is not inside a macOS app bundle.".to_owned(),
            )
        })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn write_unix_script(path: &Path, contents: &str) -> Result<(), UpdateError> {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, contents)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn spawn_unix_script(script: &Path, archive: &Path, target: &Path) -> Result<(), UpdateError> {
    Command::new(script)
        .arg(archive)
        .arg(target)
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

impl From<crate::persistence::PersistenceError> for UpdateError {
    fn from(error: crate::persistence::PersistenceError) -> Self {
        Self::InvalidManifest(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_versions_with_or_without_a_v_prefix() {
        assert_eq!(parse_version("v1.2.3").unwrap(), Version::new(1, 2, 3));
        assert_eq!(parse_version("1.2.3").unwrap(), Version::new(1, 2, 3));
    }

    #[test]
    fn platform_has_a_release_key() {
        assert!(!platform_key().is_empty());
    }
}
