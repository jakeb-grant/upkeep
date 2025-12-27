use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Fetches installed packages split into official and AUR
fn fetch_packages() -> Result<(Vec<String>, Vec<String>), String> {
    // Get all explicitly installed packages
    let output = Command::new("pacman")
        .args(["-Qqe"])
        .output()
        .map_err(|e| format!("Failed to run pacman: {}", e))?;

    if !output.status.success() {
        return Err("pacman -Qqe failed".to_string());
    }

    let all_packages: Vec<String> = std::str::from_utf8(&output.stdout)
        .map_err(|e| format!("Invalid UTF-8: {}", e))?
        .lines()
        .map(String::from)
        .collect();

    // Get foreign (AUR) packages
    let foreign_output = Command::new("pacman")
        .args(["-Qqm"])
        .output()
        .map_err(|e| format!("Failed to run pacman -Qqm: {}", e))?;

    // Note: -Qqm returns empty (not error) if no AUR packages
    let foreign: Vec<String> = std::str::from_utf8(&foreign_output.stdout)
        .unwrap_or("")
        .lines()
        .map(String::from)
        .collect();

    // Split into official and AUR
    let official: Vec<String> = all_packages.iter().filter(|p| !foreign.contains(p)).cloned().collect();
    let aur: Vec<String> = all_packages.iter().filter(|p| foreign.contains(p)).cloned().collect();

    Ok((official, aur))
}

pub fn export_packages() -> Result<(PathBuf, PathBuf, usize, usize), String> {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("upkeep")
        .join("backups");

    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create backup directory: {}", e))?;

    let date = Local::now().format("%Y-%m-%d").to_string();
    let pkg_path = dir.join(format!("packages-{}.txt", date));
    let aur_path = dir.join(format!("aur-{}.txt", date));

    let (official, aur) = fetch_packages()?;

    // Write official packages
    let mut pkg_file =
        fs::File::create(&pkg_path).map_err(|e| format!("Failed to create {}: {}", pkg_path.display(), e))?;
    for pkg in &official {
        writeln!(pkg_file, "{}", pkg).map_err(|e| format!("Failed to write: {}", e))?;
    }

    // Write AUR packages
    let mut aur_file =
        fs::File::create(&aur_path).map_err(|e| format!("Failed to create {}: {}", aur_path.display(), e))?;
    for pkg in &aur {
        writeln!(aur_file, "{}", pkg).map_err(|e| format!("Failed to write: {}", e))?;
    }

    Ok((pkg_path, aur_path, official.len(), aur.len()))
}

pub fn get_package_list() -> Result<(String, usize, usize), String> {
    let (official, aur) = fetch_packages()?;

    let mut result = String::new();
    result.push_str("# Official\n");
    for pkg in &official {
        result.push_str(pkg);
        result.push('\n');
    }
    result.push_str("\n# AUR\n");
    for pkg in &aur {
        result.push_str(pkg);
        result.push('\n');
    }

    Ok((result, official.len(), aur.len()))
}

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    // Try wl-copy first (Wayland)
    if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
        if let Some(stdin) = child.stdin.as_mut() {
            if stdin.write_all(text.as_bytes()).is_err() {
                return Err("Failed to write to wl-copy".to_string());
            }
        }
        return child.wait().map(|_| ()).map_err(|e| format!("wl-copy failed: {}", e));
    }

    // Fall back to xclip (X11)
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|_| "Neither wl-copy nor xclip available".to_string())?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes()).map_err(|e| format!("Failed to write to xclip: {}", e))?;
    }
    child.wait().map(|_| ()).map_err(|e| format!("xclip failed: {}", e))
}
