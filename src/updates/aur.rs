use super::types::{Package, PackageSource};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

const AUR_API_URL: &str = "https://aur.archlinux.org/rpc/v5/info";
const BATCH_SIZE: usize = 100;

#[derive(Debug, Deserialize)]
struct AurResponse {
    results: Vec<AurPackage>,
}

#[derive(Debug, Deserialize)]
struct AurPackage {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Version")]
    version: String,
}

pub fn check_aur_updates(aur_helper: &str) -> Vec<Package> {
    let local_packages = get_local_aur_packages();
    if local_packages.is_empty() {
        return Vec::new();
    }

    // Try AUR API first
    match query_aur_api(&local_packages) {
        Ok(aur_versions) => find_updates(&local_packages, &aur_versions),
        Err(_) => {
            // Fall back to configured AUR helper
            check_aur_updates_fallback(aur_helper)
        }
    }
}

fn get_local_aur_packages() -> Vec<(String, String)> {
    let output = Command::new("pacman").arg("-Qm").output();

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

fn query_aur_api(packages: &[(String, String)]) -> Result<HashMap<String, String>, reqwest::Error> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut results = HashMap::new();

    for batch in packages.chunks(BATCH_SIZE) {
        let names: Vec<&str> = batch.iter().map(|(name, _)| name.as_str()).collect();
        let params: Vec<(&str, &str)> = names.iter().map(|n| ("arg[]", *n)).collect();

        let response: AurResponse = client.get(AUR_API_URL).query(&params).send()?.json()?;

        for pkg in response.results {
            results.insert(pkg.name, pkg.version);
        }
    }

    Ok(results)
}

fn find_updates(
    local_packages: &[(String, String)],
    aur_versions: &HashMap<String, String>,
) -> Vec<Package> {
    local_packages
        .iter()
        .filter_map(|(name, local_ver)| {
            let aur_ver = aur_versions.get(name)?;
            if is_newer(aur_ver, local_ver) {
                Some(Package::new(
                    name.clone(),
                    local_ver.clone(),
                    aur_ver.clone(),
                    PackageSource::Aur,
                ))
            } else {
                None
            }
        })
        .collect()
}

fn is_newer(new: &str, old: &str) -> bool {
    if new == old {
        return false;
    }

    let output = Command::new("vercmp").arg(new).arg(old).output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.trim() == "1"
        }
        Err(_) => new != old,
    }
}

fn check_aur_updates_fallback(aur_helper: &str) -> Vec<Package> {
    let output = Command::new(aur_helper).arg("-Qua").output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() && output.stdout.is_empty() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            if !line.contains(" -> ") {
                return None;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let old_ver = parts[1].to_string();
                let new_ver = parts[3].to_string();
                Some(Package::new(name, old_ver, new_ver, PackageSource::Aur))
            } else {
                None
            }
        })
        .collect()
}
