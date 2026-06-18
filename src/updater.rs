use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use flate2::read::GzDecoder;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tar::Archive;
#[cfg(not(target_os = "macos"))]
use zip::ZipArchive;

const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/stoiandan/requiescat/releases/latest/download/release-manifest.json";
const INSTALL_MODE_ARGUMENT: &str = "--install-update";

#[derive(Debug)]
struct AvailableUpdate {
    version: String,
    asset: ReleaseAsset,
}

#[derive(Debug)]
struct StagedUpdate {
    archive: PathBuf,
    format: ArchiveFormat,
}

#[derive(Debug)]
pub enum UpdateError {
    Http(reqwest::Error),
    Io(io::Error),
    Archive(String),
    InvalidManifest(String),
    UnsupportedPlatform,
    ChecksumMismatch,
}

#[derive(Debug, Clone)]
pub enum LauncherProgress {
    CheckingForUpdates,
    UpToDate,
    Downloading { version: String },
    Installing,
    CheckFailed { error: String },
    LaunchingApplication,
}

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Archive(message) | Self::InvalidManifest(message) => formatter.write_str(message),
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

impl From<io::Error> for UpdateError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Deserialize)]
struct ReleaseManifest {
    version: String,
    assets: std::collections::HashMap<String, ReleaseAsset>,
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
    Msi,
    Flatpak,
}

impl ArchiveFormat {
    fn as_argument(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::TarGz => "tar-gz",
            Self::Msi => "msi",
            Self::Flatpak => "flatpak",
        }
    }

    fn from_argument(value: &str) -> Result<Self, UpdateError> {
        match value {
            "zip" => Ok(Self::Zip),
            "tar-gz" => Ok(Self::TarGz),
            "msi" => Ok(Self::Msi),
            "flatpak" => Ok(Self::Flatpak),
            _ => Err(UpdateError::InvalidManifest(format!(
                "Unsupported update archive format: {value}"
            ))),
        }
    }
}

struct InstallRequest {
    archive: PathBuf,
    target: PathBuf,
    format: ArchiveFormat,
    parent_pid: u32,
}

fn check_for_update_blocking() -> Result<Option<AvailableUpdate>, UpdateError> {
    let manifest_url =
        option_env!("REQUIESCAT_UPDATE_MANIFEST_URL").unwrap_or(DEFAULT_MANIFEST_URL);
    let current_version = installed_application_version().unwrap_or(package_version());
    let client = http_client()?;
    let manifest = client
        .get(manifest_url)
        .send()?
        .error_for_status()?
        .json::<ReleaseManifest>()?;

    update_from_manifest(manifest, &current_version)
}

fn download_update_blocking(update: AvailableUpdate) -> Result<StagedUpdate, UpdateError> {
    let client = http_client()?;
    let bytes = client
        .get(&update.asset.url)
        .send()?
        .error_for_status()?
        .bytes()?;

    stage_update(update, &bytes)
}

fn http_client() -> Result<reqwest::blocking::Client, UpdateError> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent(concat!("requiescat-updater/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

fn update_from_manifest(
    manifest: ReleaseManifest,
    current_version: &Version,
) -> Result<Option<AvailableUpdate>, UpdateError> {
    let available_version = parse_version(&manifest.version)?;
    if available_version <= *current_version {
        return Ok(None);
    }

    let asset = manifest
        .assets
        .get(platform_key())
        .cloned()
        .ok_or(UpdateError::UnsupportedPlatform)?;

    Ok(Some(AvailableUpdate {
        version: available_version.to_string(),
        asset,
    }))
}

fn stage_update(update: AvailableUpdate, bytes: &[u8]) -> Result<StagedUpdate, UpdateError> {
    let checksum = format!("{:x}", Sha256::digest(bytes));
    if !checksum.eq_ignore_ascii_case(update.asset.sha256.trim()) {
        return Err(UpdateError::ChecksumMismatch);
    }

    let directory = application_data_directory()?
        .join("updates")
        .join(&update.version);
    fs::create_dir_all(&directory)?;
    let archive = directory.join(&update.asset.file_name);
    fs::write(&archive, bytes)?;

    Ok(StagedUpdate {
        archive,
        format: update.asset.format,
    })
}

fn install_update_package(update: &StagedUpdate) -> Result<(), UpdateError> {
    match update.format {
        ArchiveFormat::Msi => install_msi_update(&update.archive),
        ArchiveFormat::Flatpak => install_flatpak_update(&update.archive),
        ArchiveFormat::Zip | ArchiveFormat::TarGz => install_archive_update(update),
    }
}

#[cfg(target_os = "windows")]
fn install_msi_update(installer: &Path) -> Result<(), UpdateError> {
    let updater = std::env::current_exe()?
        .parent()
        .ok_or_else(|| {
            UpdateError::InvalidManifest(
                "The Windows installation directory is unavailable.".to_owned(),
            )
        })?
        .join("requiescat-updater.exe");

    Command::new(updater)
        .arg("--install-msi")
        .arg(installer)
        .spawn()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn install_msi_update(_installer: &Path) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn install_flatpak_update(bundle: &Path) -> Result<(), UpdateError> {
    // Flatpak apps cannot replace files under /app from inside the sandbox.
    // Hand the bundle to the desktop/Flatpak tooling instead.
    Command::new("xdg-open").arg(bundle).spawn()?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn install_flatpak_update(_bundle: &Path) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedPlatform)
}

fn install_archive_update(update: &StagedUpdate) -> Result<(), UpdateError> {
    let executable = std::env::current_exe()?;
    let target = installation_target(&executable)?;
    ensure_installation_writable(&target)?;

    let helper = helper_path()?;
    if helper.exists() {
        fs::remove_file(&helper)?;
    }

    // On macOS the running executable lives inside Requiescat.app, so it cannot
    // replace the bundle directly. Copy the updater outside the bundle and let
    // that helper wait for this process before swapping the app atomically.
    fs::copy(&executable, &helper)?;

    let mut command = Command::new(&helper);
    command
        .arg(INSTALL_MODE_ARGUMENT)
        .arg(&update.archive)
        .arg(&target)
        .arg(update.format.as_argument())
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008 | 0x0800_0000);
    }

    command.spawn()?;
    Ok(())
}

pub fn run_launcher_mode() -> Result<(), UpdateError> {
    run_launcher_mode_with_progress(|_| {})
}

pub fn run_launcher_mode_with_progress(
    mut report: impl FnMut(LauncherProgress),
) -> Result<(), UpdateError> {
    if complete_installation_if_requested(&mut report)? {
        return Ok(());
    }

    remove_stale_helper();

    match LauncherAction::from_arguments(std::env::args_os().collect::<Vec<_>>())? {
        LauncherAction::InstallLatest => match install_latest_update(&mut report) {
            Ok(true) => Ok(()),
            Ok(false) => launch_application(&mut report),
            Err(error) => {
                report(LauncherProgress::CheckFailed {
                    error: error.to_string(),
                });
                launch_application(&mut report)
            }
        },
        LauncherAction::LaunchApp => launch_application(&mut report),
        LauncherAction::InstallMsi(installer) => {
            report(LauncherProgress::Installing);
            launch_msi_installer(&installer)
        }
    }
}

fn install_latest_update(report: &mut impl FnMut(LauncherProgress)) -> Result<bool, UpdateError> {
    report(LauncherProgress::CheckingForUpdates);

    match check_for_update_blocking() {
        Ok(Some(update)) => {
            let version = update.version.clone();
            report(LauncherProgress::Downloading { version });
            let update = download_update_blocking(update)?;
            report(LauncherProgress::Installing);
            install_update_package(&update)?;
            Ok(true)
        }
        Ok(None) => {
            report(LauncherProgress::UpToDate);
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn launch_application(report: &mut impl FnMut(LauncherProgress)) -> Result<(), UpdateError> {
    report(LauncherProgress::LaunchingApplication);
    launch_installed_application()
}

enum LauncherAction {
    InstallLatest,
    LaunchApp,
    InstallMsi(PathBuf),
}

impl LauncherAction {
    fn from_arguments(arguments: Vec<std::ffi::OsString>) -> Result<Self, UpdateError> {
        if arguments.iter().any(|argument| argument == "--launch-app") {
            return Ok(Self::LaunchApp);
        }

        if let Some(index) = arguments
            .iter()
            .position(|argument| argument == "--install-msi")
        {
            let installer = arguments.get(index + 1).ok_or_else(|| {
                UpdateError::InvalidManifest("The MSI path is missing.".to_owned())
            })?;
            return Ok(Self::InstallMsi(installer.into()));
        }

        Ok(Self::InstallLatest)
    }
}

pub fn launch_installed_application() -> Result<(), UpdateError> {
    Command::new(installed_application_path()?).spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_msi_installer(installer: &Path) -> Result<(), UpdateError> {
    use std::os::windows::process::CommandExt;

    Command::new("msiexec.exe")
        .arg("/i")
        .arg(installer)
        .args(["/passive", "/norestart", "REQUIESCAT_LAUNCH=1"])
        .creation_flags(0x0000_0008 | 0x0800_0000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn launch_msi_installer(_installer: &Path) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedPlatform)
}

fn complete_installation_if_requested(
    report: &mut impl FnMut(LauncherProgress),
) -> Result<bool, UpdateError> {
    let request = match install_request_from_arguments(std::env::args_os()) {
        Ok(Some(request)) => request,
        Ok(None) => return Ok(false),
        Err(error) => {
            record_installer_failure(&error);
            return Err(error);
        }
    };

    report(LauncherProgress::Installing);

    if let Err(error) = install_update(request) {
        record_installer_failure(&error);
        return Err(error);
    }

    Ok(true)
}

fn install_request_from_arguments(
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Option<InstallRequest>, UpdateError> {
    let mut arguments = arguments.into_iter();
    let _executable = arguments.next();
    let Some(mode) = arguments.next() else {
        return Ok(None);
    };
    if mode != INSTALL_MODE_ARGUMENT {
        return Ok(None);
    }

    parse_install_request(arguments.collect()).map(Some)
}

fn remove_stale_helper() {
    let Ok(helper) = helper_path() else {
        return;
    };
    if helper != std::env::current_exe().unwrap_or_default() {
        let _ = fs::remove_file(helper);
    }
}

fn record_installer_failure(error: &UpdateError) {
    let Ok(directory) = application_data_directory() else {
        return;
    };
    let directory = directory.join("updates");
    if fs::create_dir_all(&directory).is_ok() {
        let _ = fs::write(
            directory.join("last-install-error.txt"),
            format!("{error}\n"),
        );
    }
}

fn parse_install_request(
    arguments: Vec<std::ffi::OsString>,
) -> Result<InstallRequest, UpdateError> {
    let [archive, target, format, parent_pid]: [std::ffi::OsString; 4] =
        arguments.try_into().map_err(|arguments: Vec<_>| {
            UpdateError::InvalidManifest(format!(
                "Expected four update installer arguments, received {}.",
                arguments.len()
            ))
        })?;
    let format = format
        .into_string()
        .map_err(|_| UpdateError::InvalidManifest("Invalid archive format argument.".to_owned()))?;
    let parent_pid = parent_pid
        .into_string()
        .map_err(|_| UpdateError::InvalidManifest("Invalid parent process ID.".to_owned()))?
        .parse()
        .map_err(|_| UpdateError::InvalidManifest("Invalid parent process ID.".to_owned()))?;

    Ok(InstallRequest {
        archive: archive.into(),
        target: target.into(),
        format: ArchiveFormat::from_argument(&format)?,
        parent_pid,
    })
}

fn install_update(request: InstallRequest) -> Result<(), UpdateError> {
    wait_for_process_exit(request.parent_pid);

    let staging = std::env::temp_dir().join(format!(
        "requiescat-update-{}-{}",
        request.parent_pid,
        std::process::id()
    ));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let result = (|| {
        extract_archive(&request.archive, request.format, &staging)?;
        replace_installation(&staging, &request.target)?;
        restart_application(&request.target)
    })();

    let _ = fs::remove_dir_all(staging);
    let _ = fs::remove_file(request.archive);
    result
}

fn wait_for_process_exit(process_id: u32) {
    let mut system = System::new();
    let pid = Pid::from_u32(process_id);
    loop {
        system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        if system.process(pid).is_none() {
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
}

fn extract_archive(
    archive: &Path,
    format: ArchiveFormat,
    destination: &Path,
) -> Result<(), UpdateError> {
    match format {
        ArchiveFormat::Zip => extract_zip(archive, destination),
        ArchiveFormat::TarGz => extract_tar_gz(archive, destination),
        ArchiveFormat::Msi | ArchiveFormat::Flatpak => Err(UpdateError::Archive(
            "This package format is installed by the operating system.".to_owned(),
        )),
    }
}

fn extract_zip(archive: &Path, destination: &Path) -> Result<(), UpdateError> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("/usr/bin/ditto")
            .args(["-x", "-k"])
            .arg(archive)
            .arg(destination)
            .status()?;
        if !status.success() {
            return Err(UpdateError::Archive(format!(
                "ditto failed to extract the update archive with status {status}."
            )));
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let file = File::open(archive)?;
        let mut archive =
            ZipArchive::new(file).map_err(|error| UpdateError::Archive(error.to_string()))?;

        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| UpdateError::Archive(error.to_string()))?;
            let Some(relative_path) = entry.enclosed_name() else {
                return Err(UpdateError::Archive(
                    "The update archive contains an unsafe path.".to_owned(),
                ));
            };
            let output = destination.join(relative_path);

            if entry.is_dir() {
                fs::create_dir_all(&output)?;
                continue;
            }

            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output_file = File::create(&output)?;
            io::copy(&mut entry, &mut output_file)?;

            #[cfg(unix)]
            if let Some(mode) = entry.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&output, fs::Permissions::from_mode(mode))?;
            }
        }

        Ok(())
    }
}

fn extract_tar_gz(archive: &Path, destination: &Path) -> Result<(), UpdateError> {
    let file = File::open(archive)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(destination)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_installation(staging: &Path, target: &Path) -> Result<(), UpdateError> {
    let replacement = staging.join("requiescat").join("requiescat.exe");
    replace_file(&replacement, target)
}

#[cfg(target_os = "linux")]
fn replace_installation(staging: &Path, target: &Path) -> Result<(), UpdateError> {
    let replacement = staging.join("requiescat").join("requiescat");
    replace_file(&replacement, target)
}

#[cfg(target_os = "macos")]
fn replace_installation(staging: &Path, target: &Path) -> Result<(), UpdateError> {
    let replacement = staging.join("Requiescat.app");
    let previous = target.with_extension("app.previous");

    if previous.exists() {
        fs::remove_dir_all(&previous)?;
    }
    fs::rename(target, &previous)?;
    match fs::rename(&replacement, target) {
        Ok(()) => {
            fs::remove_dir_all(previous)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&previous, target);
            Err(error.into())
        }
    }
}

#[cfg(target_os = "linux")]
fn replace_file(replacement: &Path, target: &Path) -> Result<(), UpdateError> {
    let next = target.with_extension("new");
    fs::copy(replacement, &next)?;

    let permissions = fs::metadata(replacement)?.permissions();
    fs::set_permissions(&next, permissions)?;

    fs::rename(next, target)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_file(replacement: &Path, target: &Path) -> Result<(), UpdateError> {
    let next = target.with_extension("new");
    let previous = target.with_extension("previous");
    fs::copy(replacement, &next)?;

    if previous.exists() {
        fs::remove_file(&previous)?;
    }
    fs::rename(target, &previous)?;
    match fs::rename(&next, target) {
        Ok(()) => {
            fs::remove_file(previous)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&previous, target);
            Err(error.into())
        }
    }
}

#[cfg(target_os = "macos")]
fn restart_application(target: &Path) -> Result<(), UpdateError> {
    Command::new("open").arg(target).spawn()?;
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn restart_application(target: &Path) -> Result<(), UpdateError> {
    Command::new(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn installation_target(executable: &Path) -> Result<PathBuf, UpdateError> {
    #[cfg(target_os = "macos")]
    {
        macos_app_bundle(executable).map(Path::to_owned)
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        Ok(executable.to_owned())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = executable;
        Err(UpdateError::UnsupportedPlatform)
    }
}

fn helper_path() -> Result<PathBuf, UpdateError> {
    let directory = application_data_directory()?.join("updates");
    fs::create_dir_all(&directory)?;
    Ok(directory.join(if cfg!(target_os = "windows") {
        "requiescat-updater.exe"
    } else {
        "requiescat-updater"
    }))
}

fn application_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "requiescat.exe"
    } else {
        "requiescat"
    }
}

fn installed_application_path() -> Result<PathBuf, UpdateError> {
    let executable = std::env::current_exe()?;
    let directory = executable.parent().ok_or_else(|| {
        UpdateError::InvalidManifest("The installation directory is unavailable.".to_owned())
    })?;

    Ok(directory.join(application_executable_name()))
}

fn installed_application_version() -> Result<Version, UpdateError> {
    application_version_from(installed_application_path()?)
}

fn package_version() -> Version {
    parse_version(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION must be valid semver")
}

fn application_version_from(application: impl AsRef<Path>) -> Result<Version, UpdateError> {
    let output = Command::new(application.as_ref())
        .arg("--version")
        .stdin(Stdio::null())
        .output()?;

    if !output.status.success() {
        return Err(UpdateError::InvalidManifest(format!(
            "Could not read application version: {}",
            output.status
        )));
    }

    let version = String::from_utf8(output.stdout).map_err(|_| {
        UpdateError::InvalidManifest("Application version is not UTF-8.".to_owned())
    })?;
    parse_version(&version)
}

fn application_data_directory() -> Result<PathBuf, UpdateError> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME").ok_or_else(|| {
            UpdateError::InvalidManifest("The HOME directory is unavailable".to_owned())
        })?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Requiescat"))
    }

    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var_os("APPDATA").ok_or_else(|| {
            UpdateError::InvalidManifest("The APPDATA directory is unavailable".to_owned())
        })?;
        Ok(PathBuf::from(app_data).join("Requiescat"))
    }

    #[cfg(target_os = "linux")]
    {
        linux_application_data_directory(
            std::env::var_os("XDG_DATA_HOME"),
            std::env::var_os("HOME"),
        )
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(UpdateError::UnsupportedPlatform)
    }
}

#[cfg(target_os = "linux")]
fn linux_application_data_directory(
    data_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, UpdateError> {
    if let Some(data_home) = data_home.filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(data_home).join("requiescat"));
    }

    let home = home.filter(|path| !path.is_empty()).ok_or_else(|| {
        UpdateError::InvalidManifest(
            "Neither XDG_DATA_HOME nor HOME is available on Linux.".to_owned(),
        )
    })?;

    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("requiescat"))
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
        "windows-msi-x86_64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-aarch64"
    }
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    {
        "macos-unsupported"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-flatpak-x86_64"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_versions_with_or_without_a_v_prefix() {
        assert_eq!(parse_version("v1.2.3").unwrap(), Version::new(1, 2, 3));
        assert_eq!(parse_version("1.2.3").unwrap(), Version::new(1, 2, 3));
    }

    #[test]
    fn parses_installer_arguments() {
        let request = parse_install_request(vec![
            "update.zip".into(),
            "Requiescat.app".into(),
            "zip".into(),
            "42".into(),
        ])
        .unwrap();

        assert_eq!(request.archive, PathBuf::from("update.zip"));
        assert_eq!(request.target, PathBuf::from("Requiescat.app"));
        assert!(matches!(request.format, ArchiveFormat::Zip));
        assert_eq!(request.parent_pid, 42);
    }

    #[test]
    fn detects_installer_mode_from_process_arguments() {
        let request = install_request_from_arguments(vec![
            "requiescat".into(),
            INSTALL_MODE_ARGUMENT.into(),
            "update.zip".into(),
            "Requiescat.app".into(),
            "zip".into(),
            "42".into(),
        ])
        .unwrap()
        .unwrap();

        assert_eq!(request.archive, PathBuf::from("update.zip"));
        assert_eq!(request.target, PathBuf::from("Requiescat.app"));
        assert!(matches!(request.format, ArchiveFormat::Zip));
        assert_eq!(request.parent_pid, 42);
    }

    #[test]
    fn ignores_non_installer_process_arguments() {
        assert!(
            install_request_from_arguments(vec!["requiescat".into()])
                .unwrap()
                .is_none()
        );
        assert!(
            install_request_from_arguments(vec!["requiescat".into(), "--launch-app".into()])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn parses_launcher_arguments() {
        assert!(matches!(
            LauncherAction::from_arguments(vec!["requiescat-updater".into()]).unwrap(),
            LauncherAction::InstallLatest
        ));

        assert!(matches!(
            LauncherAction::from_arguments(vec![
                "requiescat-updater".into(),
                "--launch-app".into()
            ])
            .unwrap(),
            LauncherAction::LaunchApp
        ));

        match LauncherAction::from_arguments(vec![
            "requiescat-updater".into(),
            "--install-msi".into(),
            "update.msi".into(),
        ])
        .unwrap()
        {
            LauncherAction::InstallMsi(path) => assert_eq!(path, PathBuf::from("update.msi")),
            _ => panic!("expected MSI launcher action"),
        }
    }

    #[test]
    fn manifest_update_is_compared_to_application_version() {
        let manifest = release_manifest("2.0.0");
        let current_version = Version::new(1, 5, 0);

        let update = update_from_manifest(manifest, &current_version).unwrap();

        assert_eq!(update.unwrap().version, "2.0.0");
    }

    #[test]
    fn manifest_without_newer_version_is_ignored() {
        let manifest = release_manifest("2.0.0");
        let current_version = Version::new(2, 0, 0);

        assert!(
            update_from_manifest(manifest, &current_version)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn platform_has_a_release_key() {
        assert!(!platform_key().is_empty());
    }

    fn release_manifest(version: &str) -> ReleaseManifest {
        ReleaseManifest {
            version: version.to_owned(),
            assets: std::collections::HashMap::from([(
                platform_key().to_owned(),
                ReleaseAsset {
                    url: "https://example.com/update.zip".to_owned(),
                    sha256: "checksum".to_owned(),
                    file_name: "update.zip".to_owned(),
                    format: ArchiveFormat::Zip,
                },
            )]),
        }
    }
}
