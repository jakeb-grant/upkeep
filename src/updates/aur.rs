use super::types::{Package, PackageSource};
use super::util::url_encode;
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

pub fn check_aur_updates(aur_helper: &str) -> Result<Vec<Package>, String> {
    let local_packages = get_local_aur_packages();
    if local_packages.is_empty() {
        return Ok(Vec::new());
    }

    // Try AUR API first
    match query_aur_api(&local_packages) {
        Ok(aur_versions) => Ok(find_updates(&local_packages, &aur_versions)),
        Err(api_err) => {
            // Fall back to configured AUR helper
            check_aur_updates_fallback(aur_helper)
                .map_err(|helper_err| format!("{}; {}", api_err, helper_err))
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

fn query_aur_api(packages: &[(String, String)]) -> Result<HashMap<String, String>, String> {
    let mut results = HashMap::new();

    for batch in packages.chunks(BATCH_SIZE) {
        let args: Vec<String> = batch
            .iter()
            .map(|(name, _)| format!("arg[]={}", url_encode(name)))
            .collect();
        let url = format!("{}?{}", AUR_API_URL, args.join("&"));

        // -g disables curl URL globbing so the arg[] brackets pass through literally
        let output = Command::new("curl")
            .args(["-sg", "-m", "30", &url])
            .output()
            .map_err(|e| format!("failed to run curl: {}", e))?;

        if !output.status.success() {
            return Err(format!("AUR request failed (curl {})", output.status));
        }

        let json = String::from_utf8_lossy(&output.stdout);
        let response: AurResponse =
            serde_json::from_str(&json).map_err(|e| format!("unexpected AUR response: {}", e))?;

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

fn check_aur_updates_fallback(aur_helper: &str) -> Result<Vec<Package>, String> {
    let output = Command::new(aur_helper).arg("-Qua").output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("{} not found", aur_helper)
        } else {
            format!("failed to run {}: {}", aur_helper, e)
        }
    })?;

    // AUR helpers exit non-zero with empty output when there are no updates,
    // so parse whatever stdout we got rather than treating that as an error
    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = stdout
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
        .collect();

    Ok(packages)
}
