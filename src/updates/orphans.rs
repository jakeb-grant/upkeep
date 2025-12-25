use super::installed::{get_foreign_packages, InstalledPackage};
use super::types::PackageSource;
use std::process::Command;

pub fn get_orphan_packages() -> Vec<InstalledPackage> {
    // pacman -Qdt lists packages installed as deps but no longer required
    let output = Command::new("pacman").args(["-Qdt"]).output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    // Exit code 1 with empty output means no orphans (not an error)
    if !output.status.success() && output.stdout.is_empty() {
        return Vec::new();
    }

    // Get foreign (AUR) packages to determine source
    let foreign = get_foreign_packages();

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let source = if foreign.contains(&name) {
                    PackageSource::Aur
                } else {
                    PackageSource::Pacman
                };
                Some(InstalledPackage::new(name, parts[1].to_string(), source))
            } else {
                None
            }
        })
        .collect()
}
