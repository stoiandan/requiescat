#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod windows {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use semver::Version;
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    const MANIFEST_URL: &str =
        "https://github.com/stoiandan/requiescat/releases/latest/download/release-manifest.json";

    #[derive(Deserialize)]
    struct ReleaseManifest {
        version: String,
        assets: HashMap<String, ReleaseAsset>,
    }

    #[derive(Deserialize)]
    struct ReleaseAsset {
        url: String,
        sha256: String,
        file_name: String,
    }

    pub fn run() -> Result<(), String> {
        let arguments = std::env::args().collect::<Vec<_>>();
        if arguments.iter().any(|argument| argument == "--launch-app") {
            return launch_application();
        }
        if let Some(index) = arguments
            .iter()
            .position(|argument| argument == "--install-msi")
        {
            let installer = arguments
                .get(index + 1)
                .ok_or_else(|| "The MSI path is missing.".to_owned())?;
            return launch_installer(Path::new(installer));
        }

        match find_update() {
            Ok(Some(update)) => download_update(&update)
                .and_then(|installer| launch_installer(&installer))
                .or_else(|_| launch_application()),
            Ok(None) | Err(_) => launch_application(),
        }
    }

    fn find_update() -> Result<Option<ReleaseAsset>, String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(concat!("requiescat-updater/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| error.to_string())?;
        let manifest = client
            .get(MANIFEST_URL)
            .send()
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<ReleaseManifest>()
            .map_err(|error| error.to_string())?;
        let available = parse_version(&manifest.version)?;
        let current = parse_version(env!("CARGO_PKG_VERSION"))?;

        if available <= current {
            return Ok(None);
        }

        manifest
            .assets
            .into_iter()
            .find(|(key, _)| key == "windows-msi-x86_64")
            .map(|(_, asset)| asset)
            .ok_or_else(|| "The release has no Windows MSI package.".to_owned())
            .map(Some)
    }

    fn download_update(asset: &ReleaseAsset) -> Result<PathBuf, String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(concat!("requiescat-updater/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| error.to_string())?;
        let bytes = client
            .get(&asset.url)
            .send()
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .bytes()
            .map_err(|error| error.to_string())?;
        let checksum = format!("{:x}", Sha256::digest(&bytes));
        if !checksum.eq_ignore_ascii_case(asset.sha256.trim()) {
            return Err("The downloaded MSI failed checksum verification.".to_owned());
        }

        let directory = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| "LOCALAPPDATA is unavailable.".to_owned())?
            .join("Requiescat")
            .join("updates");
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let installer = directory.join(&asset.file_name);
        fs::write(&installer, bytes).map_err(|error| error.to_string())?;
        Ok(installer)
    }

    fn launch_installer(installer: &Path) -> Result<(), String> {
        use std::os::windows::process::CommandExt;

        Command::new("msiexec.exe")
            .arg("/i")
            .arg(installer)
            .args(["/passive", "/norestart", "REQUIESCAT_LAUNCH=1"])
            .creation_flags(0x0000_0008 | 0x0800_0000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn launch_application() -> Result<(), String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let application = executable
            .parent()
            .ok_or_else(|| "The installation directory is unavailable.".to_owned())?
            .join("requiescat.exe");
        Command::new(application)
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn parse_version(value: &str) -> Result<Version, String> {
        Version::parse(value.trim().trim_start_matches('v')).map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "windows")]
fn main() {
    if let Err(error) = windows::run() {
        let _ = std::process::Command::new("msg.exe")
            .arg("*")
            .arg(format!("Requiescat update failed: {error}"))
            .spawn();
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("requiescat-updater is only used by the Windows MSI package.");
}
