use std::ffi::OsString;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sysinfo::{Pid, ProcessesToUpdate, System};

const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/stoiandan/requiescat/releases/latest/download/release-manifest.json";
const MACOS_INSTALL_MODE_ARGUMENT: &str = "--install-update";

#[derive(Debug)]
struct AvailableUpdate {
    version: String,
    asset: ReleaseAsset,
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
}

struct MacosInstallRequest {
    archive: PathBuf,
    target: PathBuf,
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

fn download_update_blocking(update: &AvailableUpdate) -> Result<PathBuf, UpdateError> {
    let client = http_client()?;
    let bytes = client
        .get(&update.asset.url)
        .send()?
        .error_for_status()?
        .bytes()?;

    let checksum = format!("{:x}", Sha256::digest(&bytes));
    if !checksum.eq_ignore_ascii_case(update.asset.sha256.trim()) {
        return Err(UpdateError::ChecksumMismatch);
    }

    let directory = application_data_directory()?
        .join("updates")
        .join(&update.version);
    fs::create_dir_all(&directory)?;
    let archive = directory.join(&update.asset.file_name);
    fs::write(&archive, bytes)?;

    Ok(archive)
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

#[cfg(target_os = "windows")]
fn install_update_package(installer: &Path) -> Result<(), UpdateError> {
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

#[cfg(target_os = "linux")]
fn install_update_package(bundle: &Path) -> Result<(), UpdateError> {
    // Flatpak apps cannot replace files under /app from inside the sandbox.
    // Hand the bundle to the desktop/Flatpak tooling instead.
    Command::new("xdg-open").arg(bundle).spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn install_update_package(archive: &Path) -> Result<(), UpdateError> {
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
        .arg(MACOS_INSTALL_MODE_ARGUMENT)
        .arg(archive)
        .arg(&target)
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    command.spawn()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn install_update_package(_archive: &Path) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedPlatform)
}

pub fn run_launcher_mode_with_progress(
    report: impl FnMut(LauncherProgress),
) -> Result<(), UpdateError> {
    run_launcher_for_current_platform(report)
}

#[cfg(target_os = "macos")]
fn run_launcher_for_current_platform(
    mut report: impl FnMut(LauncherProgress),
) -> Result<(), UpdateError> {
    let arguments = std::env::args_os().collect::<Vec<_>>();

    match macos_install_request_from_arguments(arguments.iter().cloned()) {
        Ok(Some(install_request)) => {
            report(LauncherProgress::Installing);
            if let Err(error) = complete_macos_app_install(install_request) {
                record_installer_failure(&error);
                return Err(error);
            }
            return Ok(());
        }
        Ok(None) => {}
        Err(error) => {
            record_installer_failure(&error);
            return Err(error);
        }
    }

    remove_stale_helper();
    run_user_launcher_mode(&arguments, &mut report)
}

#[cfg(target_os = "windows")]
fn run_launcher_for_current_platform(
    mut report: impl FnMut(LauncherProgress),
) -> Result<(), UpdateError> {
    let arguments = std::env::args_os().collect::<Vec<_>>();

    if let Some(installer) = windows_msi_install_request_from_arguments(&arguments)? {
        report(LauncherProgress::Installing);
        return complete_windows_msi_install(&installer);
    }

    run_user_launcher_mode(&arguments, &mut report)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn run_launcher_for_current_platform(
    mut report: impl FnMut(LauncherProgress),
) -> Result<(), UpdateError> {
    let arguments = std::env::args_os().collect::<Vec<_>>();
    run_user_launcher_mode(&arguments, &mut report)
}

fn run_user_launcher_mode(
    arguments: &[OsString],
    report: &mut impl FnMut(LauncherProgress),
) -> Result<(), UpdateError> {
    if should_launch_without_update_check(arguments) {
        return launch_application(report);
    }

    update_or_launch(report)
}

fn update_or_launch(report: &mut impl FnMut(LauncherProgress)) -> Result<(), UpdateError> {
    report(LauncherProgress::CheckingForUpdates);

    let update = match check_for_update_blocking() {
        Ok(Some(update)) => update,
        Ok(None) => {
            report(LauncherProgress::UpToDate);
            return launch_application(report);
        }
        Err(error) => {
            report(LauncherProgress::CheckFailed {
                error: error.to_string(),
            });
            return launch_application(report);
        }
    };

    report(LauncherProgress::Downloading {
        version: update.version.clone(),
    });
    let archive = download_update_blocking(&update)?;
    report(LauncherProgress::Installing);
    install_update_package(&archive)?;
    Ok(())
}

fn launch_application(report: &mut impl FnMut(LauncherProgress)) -> Result<(), UpdateError> {
    report(LauncherProgress::LaunchingApplication);
    Command::new(installed_application_path()?).spawn()?;
    Ok(())
}

fn should_launch_without_update_check(arguments: &[OsString]) -> bool {
    arguments.iter().any(|argument| argument == "--launch-app")
}

#[cfg(target_os = "windows")]
fn windows_msi_install_request_from_arguments(
    arguments: &[OsString],
) -> Result<Option<PathBuf>, UpdateError> {
    let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--install-msi")
    else {
        return Ok(None);
    };

    let installer = arguments
        .get(index + 1)
        .ok_or_else(|| UpdateError::InvalidManifest("The MSI path is missing.".to_owned()))?;

    Ok(Some(installer.into()))
}

#[cfg(target_os = "windows")]
fn complete_windows_msi_install(installer: &Path) -> Result<(), UpdateError> {
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

fn macos_install_request_from_arguments(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<Option<MacosInstallRequest>, UpdateError> {
    let mut arguments = arguments.into_iter();
    let _executable = arguments.next();
    let Some(mode) = arguments.next() else {
        return Ok(None);
    };
    if mode != MACOS_INSTALL_MODE_ARGUMENT {
        return Ok(None);
    }

    let [archive, target, parent_pid]: [OsString; 3] = arguments
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|arguments: Vec<_>| {
            UpdateError::InvalidManifest(format!(
                "Expected three update installer arguments, received {}.",
                arguments.len()
            ))
        })?;
    let parent_pid = parent_pid
        .into_string()
        .map_err(|_| UpdateError::InvalidManifest("Invalid parent process ID.".to_owned()))?
        .parse()
        .map_err(|_| UpdateError::InvalidManifest("Invalid parent process ID.".to_owned()))?;

    Ok(Some(MacosInstallRequest {
        archive: archive.into(),
        target: target.into(),
        parent_pid,
    }))
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

#[cfg(target_os = "macos")]
fn complete_macos_app_install(request: MacosInstallRequest) -> Result<(), UpdateError> {
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
        extract_macos_app_archive(&request.archive, &staging)?;
        replace_app_bundle(&staging, &request.target)?;
        restart_application(&request.target)
    })();

    let _ = fs::remove_dir_all(staging);
    let _ = fs::remove_file(request.archive);
    result
}

#[cfg(not(target_os = "macos"))]
fn complete_macos_app_install(_request: MacosInstallRequest) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedPlatform)
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

#[cfg(target_os = "macos")]
fn extract_macos_app_archive(archive: &Path, destination: &Path) -> Result<(), UpdateError> {
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

#[cfg(target_os = "macos")]
fn replace_app_bundle(staging: &Path, target: &Path) -> Result<(), UpdateError> {
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

#[cfg(target_os = "macos")]
fn restart_application(target: &Path) -> Result<(), UpdateError> {
    Command::new("open").arg(target).spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn installation_target(executable: &Path) -> Result<PathBuf, UpdateError> {
    macos_app_bundle(executable).map(Path::to_owned)
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
    fn detects_installer_mode_from_process_arguments() {
        let request = macos_install_request_from_arguments(vec![
            "requiescat".into(),
            MACOS_INSTALL_MODE_ARGUMENT.into(),
            "update.zip".into(),
            "Requiescat.app".into(),
            "42".into(),
        ])
        .unwrap()
        .unwrap();

        assert_eq!(request.archive, PathBuf::from("update.zip"));
        assert_eq!(request.target, PathBuf::from("Requiescat.app"));
        assert_eq!(request.parent_pid, 42);
    }

    #[test]
    fn ignores_non_installer_process_arguments() {
        assert!(
            macos_install_request_from_arguments(vec!["requiescat".into()])
                .unwrap()
                .is_none()
        );
        assert!(
            macos_install_request_from_arguments(vec!["requiescat".into(), "--launch-app".into()])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn detects_launch_app_argument() {
        assert!(!should_launch_without_update_check(&[
            "requiescat-updater".into()
        ]));
        assert!(should_launch_without_update_check(&[
            "requiescat-updater".into(),
            "--launch-app".into()
        ]));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_msi_installer_argument() {
        let installer = windows_msi_install_request_from_arguments(&[
            "requiescat-updater".into(),
            "--install-msi".into(),
            "update.msi".into(),
        ])
        .unwrap()
        .unwrap();

        assert_eq!(installer, PathBuf::from("update.msi"));
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
                },
            )]),
        }
    }
}
