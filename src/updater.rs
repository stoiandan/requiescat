use std::collections::HashMap;
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

#[derive(Debug, Clone)]
pub struct AvailableUpdate {
    pub version: String,
    notes_url: String,
    descriptions: HashMap<String, String>,
    asset: ReleaseAsset,
}

impl AvailableUpdate {
    pub fn description(&self, language_code: &str) -> &str {
        localized_description(&self.descriptions, language_code)
    }

    pub fn notes_url(&self) -> &str {
        &self.notes_url
    }
}

#[derive(Debug, Clone)]
pub struct StagedUpdate {
    pub version: String,
    notes_url: String,
    descriptions: HashMap<String, String>,
    archive: PathBuf,
    format: ArchiveFormat,
}

impl StagedUpdate {
    pub fn description(&self, language_code: &str) -> &str {
        localized_description(&self.descriptions, language_code)
    }

    pub fn notes_url(&self) -> &str {
        &self.notes_url
    }
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
    notes_url: String,
    descriptions: HashMap<String, String>,
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

pub async fn check_for_update() -> Result<Option<AvailableUpdate>, UpdateError> {
    check_for_update_blocking()
}

pub fn check_for_update_blocking() -> Result<Option<AvailableUpdate>, UpdateError> {
    let manifest_url =
        option_env!("REQUIESCAT_UPDATE_MANIFEST_URL").unwrap_or(DEFAULT_MANIFEST_URL);
    let client = http_client()?;
    let manifest = client
        .get(manifest_url)
        .send()?
        .error_for_status()?
        .json::<ReleaseManifest>()?;

    update_from_manifest(manifest)
}

pub async fn download_update(update: AvailableUpdate) -> Result<StagedUpdate, UpdateError> {
    download_update_blocking(update)
}

pub fn download_update_blocking(update: AvailableUpdate) -> Result<StagedUpdate, UpdateError> {
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

fn update_from_manifest(manifest: ReleaseManifest) -> Result<Option<AvailableUpdate>, UpdateError> {
    let available_version = parse_version(&manifest.version)?;
    let current_version = parse_version(env!("CARGO_PKG_VERSION"))?;
    if available_version <= current_version {
        return Ok(None);
    }
    validate_descriptions(&manifest.descriptions)?;

    let asset = manifest
        .assets
        .get(platform_key())
        .cloned()
        .ok_or(UpdateError::UnsupportedPlatform)?;

    Ok(Some(AvailableUpdate {
        version: available_version.to_string(),
        notes_url: manifest.notes_url,
        descriptions: manifest.descriptions,
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
        version: update.version,
        notes_url: update.notes_url,
        descriptions: update.descriptions,
        archive,
        format: update.asset.format,
    })
}

fn validate_descriptions(descriptions: &HashMap<String, String>) -> Result<(), UpdateError> {
    for language in ["en", "ro"] {
        if descriptions
            .get(language)
            .is_none_or(|description| description.trim().is_empty())
        {
            return Err(UpdateError::InvalidManifest(format!(
                "The release description for {language} is missing."
            )));
        }
    }
    Ok(())
}

fn localized_description<'a>(
    descriptions: &'a HashMap<String, String>,
    language_code: &str,
) -> &'a str {
    descriptions
        .get(language_code)
        .or_else(|| descriptions.get("en"))
        .map(String::as_str)
        .unwrap_or_default()
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
    #[cfg(target_os = "windows")]
    if matches!(update.format, ArchiveFormat::Msi) {
        let executable = std::env::current_exe()?;
        let updater = executable
            .parent()
            .ok_or_else(|| {
                UpdateError::InvalidManifest(
                    "The Windows installation directory is unavailable.".to_owned(),
                )
            })?
            .join("requiescat-updater.exe");
        Command::new(updater)
            .arg("--install-msi")
            .arg(&update.archive)
            .spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    if matches!(update.format, ArchiveFormat::Flatpak) {
        Command::new("xdg-open").arg(&update.archive).spawn()?;
        return Ok(());
    }

    let executable = std::env::current_exe()?;
    let target = installation_target(&executable)?;
    ensure_installation_writable(&target)?;

    let helper = helper_path()?;
    if helper.exists() {
        fs::remove_file(&helper)?;
    }
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
    match LauncherAction::from_arguments(std::env::args_os().collect::<Vec<_>>())? {
        LauncherAction::InstallLatest => match install_latest_update() {
            Ok(true) => Ok(()),
            Ok(false) | Err(_) => launch_installed_application(),
        },
        LauncherAction::LaunchApp => launch_installed_application(),
        LauncherAction::InstallMsi(installer) => launch_msi_installer(&installer),
    }
}

fn install_latest_update() -> Result<bool, UpdateError> {
    match check_for_update_blocking() {
        Ok(Some(update)) => {
            let update = download_update_blocking(update)?;
            install_and_restart(&update)?;
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(error) => Err(error),
    }
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
    let executable = std::env::current_exe()?;
    let application = executable
        .parent()
        .ok_or_else(|| {
            UpdateError::InvalidManifest("The installation directory is unavailable.".to_owned())
        })?
        .join(application_executable_name());
    Command::new(application).spawn()?;
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

pub fn run_installer_mode() -> Option<Result<(), UpdateError>> {
    let mut arguments = std::env::args_os();
    let _executable = arguments.next();
    if arguments.next().as_deref() != Some(INSTALL_MODE_ARGUMENT.as_ref()) {
        return None;
    }

    Some(parse_install_request(arguments.collect()).and_then(install_update))
}

pub fn remove_stale_helper() {
    let Ok(helper) = helper_path() else {
        return;
    };
    if helper != std::env::current_exe().unwrap_or_default() {
        let _ = fs::remove_file(helper);
    }
}

pub fn record_installer_failure(error: &UpdateError) {
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
    fn platform_has_a_release_key() {
        assert!(!platform_key().is_empty());
    }

    #[test]
    fn localized_description_uses_english_as_fallback() {
        let descriptions = HashMap::from([
            ("en".to_owned(), "English notes".to_owned()),
            ("ro".to_owned(), "Note în română".to_owned()),
        ]);

        assert_eq!(localized_description(&descriptions, "ro"), "Note în română");
        assert_eq!(localized_description(&descriptions, "de"), "English notes");
        assert!(validate_descriptions(&descriptions).is_ok());
    }
}
