use super::config::RebuildCheck;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct RebuildIssue {
    pub name: String,
    pub rebuild_command: String,
    pub selected: bool,
}

pub fn check_rebuilds(checks: &[RebuildCheck]) -> Vec<RebuildIssue> {
    checks
        .iter()
        .filter_map(|check| {
            if has_rebuild_issue(check) {
                Some(RebuildIssue {
                    name: check.name.clone(),
                    rebuild_command: check.rebuild.clone(),
                    selected: false,
                })
            } else {
                None
            }
        })
        .collect()
}

fn has_rebuild_issue(check: &RebuildCheck) -> bool {
    if check.command.is_empty() {
        return false;
    }

    let result = Command::new(&check.command[0])
        .args(&check.command[1..])
        .output();

    let output = match result {
        Ok(o) => o,
        Err(_) => return false,
    };

    let stderr = String::from_utf8_lossy(&output.stderr);

    for pattern in &check.error_patterns {
        if stderr.contains(pattern) {
            return true;
        }
    }

    false
}
