use super::config::RebuildCheck;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Check passed — no rebuild needed
    Ok,
    /// Check triggered — rebuild needed
    Triggered,
}

#[derive(Debug, Clone)]
pub struct RebuildIssue {
    pub name: String,
    pub rebuild_command: String,
    pub detail: Option<String>,
    pub status: CheckStatus,
    pub selected: bool,
}

pub fn check_rebuilds(checks: &[RebuildCheck]) -> Vec<RebuildIssue> {
    checks
        .iter()
        .filter_map(|check| {
            if let Some(ref tracks) = check.tracks {
                Some(check_version_track(check, tracks))
            } else if check.command.is_some() && check.error_patterns.is_some() {
                let triggered = has_rebuild_issue(check);
                Some(RebuildIssue {
                    name: check.name.clone(),
                    rebuild_command: check.rebuild.clone(),
                    detail: None,
                    status: if triggered { CheckStatus::Triggered } else { CheckStatus::Ok },
                    selected: false,
                })
            } else {
                None
            }
        })
        .collect()
}

fn has_rebuild_issue(check: &RebuildCheck) -> bool {
    let command = match check.command.as_ref() {
        Some(cmd) if !cmd.is_empty() => cmd,
        _ => return false,
    };
    let error_patterns = match check.error_patterns.as_ref() {
        Some(p) => p,
        None => return false,
    };

    let result = Command::new(&command[0])
        .args(&command[1..])
        .output();

    let output = match result {
        Ok(o) => o,
        Err(_) => return false,
    };

    let stderr = String::from_utf8_lossy(&output.stderr);

    for pattern in error_patterns {
        if stderr.contains(pattern) {
            return true;
        }
    }

    false
}

fn check_version_track(check: &RebuildCheck, tracks: &str) -> RebuildIssue {
    let upstream_version = match get_repo_version(tracks) {
        Some(v) => v,
        None => {
            return RebuildIssue {
                name: check.name.clone(),
                rebuild_command: check.rebuild.clone(),
                detail: Some(format!("tracks {} (repo unavailable)", tracks)),
                status: CheckStatus::Ok,
                selected: false,
            };
        }
    };

    let installed_version = match get_installed_version(&check.name) {
        Some(v) => v,
        None => {
            return RebuildIssue {
                name: check.name.clone(),
                rebuild_command: check.rebuild.clone(),
                detail: Some(format!("not installed (upstream: {})", upstream_version)),
                status: CheckStatus::Triggered,
                selected: false,
            };
        }
    };

    // vercmp returns < 0 if first version is older
    let cmp = Command::new("vercmp")
        .args([&installed_version, &upstream_version])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<i32>().ok())
        .unwrap_or(0);

    if cmp < 0 {
        RebuildIssue {
            name: check.name.clone(),
            rebuild_command: check.rebuild.clone(),
            detail: Some(format!("{} → {}", installed_version, upstream_version)),
            status: CheckStatus::Triggered,
            selected: false,
        }
    } else {
        RebuildIssue {
            name: check.name.clone(),
            rebuild_command: check.rebuild.clone(),
            detail: Some(format!("{} (up to date)", installed_version)),
            status: CheckStatus::Ok,
            selected: false,
        }
    }
}

/// Get installed version via `pacman -Q <name>`
fn get_installed_version(name: &str) -> Option<String> {
    let output = Command::new("pacman")
        .args(["-Q", name])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Output format: "package-name 1:1.8.4-1"
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.split_whitespace().nth(1).map(|s| s.to_string())
}

/// Get repo version via `pacman -Si <name>`
fn get_repo_version(name: &str) -> Option<String> {
    let output = Command::new("pacman")
        .args(["-Si", name])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Find "Version" line: "Version         : 1:1.8.4-1"
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("Version") {
            if let Some(version) = rest.trim().strip_prefix(':') {
                return Some(version.trim().to_string());
            }
        }
    }
    None
}
