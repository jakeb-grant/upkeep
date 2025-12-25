use super::types::{Filterable, PackageSource};
use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub source: PackageSource,
    pub selected: bool,
}

impl InstalledPackage {
    pub fn new(name: String, version: String, source: PackageSource) -> Self {
        Self {
            name,
            version,
            source,
            selected: false,
        }
    }

    pub fn source_label(&self) -> &'static str {
        match self.source {
            PackageSource::Pacman => "",
            PackageSource::Aur => " (AUR)",
        }
    }
}

impl Filterable for InstalledPackage {
    fn name(&self) -> &str {
        &self.name
    }
}

pub fn get_installed_packages() -> Vec<InstalledPackage> {
    // Get explicitly installed packages
    let explicit = get_explicit_packages();

    // Get AUR/foreign packages to determine source
    let foreign = get_foreign_packages();

    explicit
        .into_iter()
        .map(|(name, version)| {
            let source = if foreign.contains(&name) {
                PackageSource::Aur
            } else {
                PackageSource::Pacman
            };
            InstalledPackage::new(name, version, source)
        })
        .collect()
}

fn get_explicit_packages() -> Vec<(String, String)> {
    let output = Command::new("pacman")
        .args(["-Qe"])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect()
}

pub fn get_foreign_packages() -> HashSet<String> {
    let output = Command::new("pacman")
        .args(["-Qm"])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return HashSet::new(),
    };

    if !output.status.success() {
        return HashSet::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                Some(parts[0].to_string())
            } else {
                None
            }
        })
        .collect()
}
