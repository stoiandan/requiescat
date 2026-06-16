#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = requiescat::updater::run_launcher_mode() {
        report_error(&error);
        std::process::exit(1);
    }
}

#[cfg(target_os = "windows")]
fn report_error(error: &requiescat::updater::UpdateError) {
    let _ = std::process::Command::new("msg.exe")
        .arg("*")
        .arg(format!("Requiescat update failed: {error}"))
        .spawn();
}

#[cfg(not(target_os = "windows"))]
fn report_error(error: &requiescat::updater::UpdateError) {
    eprintln!("Requiescat update failed: {error}");
}
