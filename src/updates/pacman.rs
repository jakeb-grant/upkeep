use super::types::{Package, PackageSource};
use std::process::Command;

pub fn check_pacman_updates() -> Result<Vec<Package>, String> {
    let output = Command::new("checkupdates")
        .arg("--nocolor")
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "checkupdates not found (install pacman-contrib)".to_string()
            } else {
                format!("failed to run checkupdates: {}", e)
            }
        })?;

    // checkupdates exit codes: 0 = updates available, 2 = no updates, 1 = error
    match output.status.code() {
        Some(0) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(parse_updates(&stdout))
        }
        Some(2) => Ok(Vec::new()),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr.lines().next().unwrap_or("").trim().to_string();
            if msg.is_empty() {
                Err("checkupdates failed".to_string())
            } else {
                Err(msg)
            }
        }
    }
}

fn parse_updates(output: &str) -> Vec<Package> {
    output
        .lines()
        .filter_map(|line| {
            // Format: "package old_version -> new_version"
            if !line.contains(" -> ") {
                return None;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let old_ver = parts[1].to_string();
                let new_ver = parts[3].to_string();
                Some(Package::new(name, old_ver, new_ver, PackageSource::Pacman))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_updates() {
        let output = "firefox 115.0-1 -> 116.0-1\nlinux 6.4.10-1 -> 6.4.12-1\n";
        let packages = parse_updates(output);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "firefox");
        assert_eq!(packages[0].old_version, "115.0-1");
        assert_eq!(packages[0].new_version, "116.0-1");
    }
}
