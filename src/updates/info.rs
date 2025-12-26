use super::util::url_encode;
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub size: String,
    pub repository: String,
    pub install_date: Option<String>,
    pub install_reason: Option<String>,
    pub url: Option<String>,
    pub build_date: Option<String>,
    pub maintainer: Option<String>,
    pub votes: Option<u32>,
}

/// AUR RPC API response
#[derive(Debug, Deserialize)]
struct AurResponse {
    resultcount: u32,
    results: Vec<AurPackage>,
}

/// AUR package info from RPC API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AurPackage {
    name: String,
    version: String,
    description: Option<String>,
    #[serde(rename = "URL")]
    url: Option<String>,
    maintainer: Option<String>,
    #[serde(rename = "NumVotes")]
    num_votes: Option<u32>,
    #[serde(rename = "LastModified")]
    last_modified: Option<i64>,
}

/// Check if a package is foreign (AUR/manually installed)
fn is_foreign_package(name: &str) -> bool {
    let output = Command::new("pacman")
        .args(["-Qmq", name])
        .output();
    matches!(output, Ok(o) if o.status.success())
}

/// Format unix timestamp to human-readable date
fn format_timestamp(ts: i64) -> String {
    // Use date command to format - simpler than pulling in chrono
    let output = Command::new("date")
        .args(["-d", &format!("@{}", ts), "+%Y-%m-%d"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => format!("{}", ts), // Fallback to raw timestamp
    }
}

impl PackageInfo {
    /// Fetch info for an installed package using pacman -Qi
    /// Also fetches repository from -Si since -Qi doesn't include it
    /// For AUR packages, fetches additional info from AUR RPC
    pub fn for_installed(name: &str) -> Option<Self> {
        let output = Command::new("pacman")
            .args(["-Qi", name])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut info = Self::parse_pacman_output(&stdout, true)?;

        // -Qi doesn't include Repository, so fetch it from -Si
        if info.repository.is_empty() {
            if let Ok(repo_output) = Command::new("pacman").args(["-Si", name]).output() {
                if repo_output.status.success() {
                    let repo_stdout = String::from_utf8_lossy(&repo_output.stdout);
                    for line in repo_stdout.lines() {
                        if let Some(repo) = line.strip_prefix("Repository") {
                            if let Some(value) = repo.trim().strip_prefix(':') {
                                info.repository = value.trim().to_string();
                                break;
                            }
                        }
                    }
                }
            }
        }

        // If still no repository, check if it's a foreign (AUR) package
        if info.repository.is_empty() && is_foreign_package(name) {
            info.repository = "AUR".to_string();
            // Fetch additional AUR info (maintainer, votes)
            if let Some(aur_info) = Self::fetch_aur_rpc(name) {
                info.maintainer = aur_info.maintainer;
                info.votes = aur_info.votes;
            }
        }

        Some(info)
    }

    /// Fetch info for a repo package using pacman -Si
    pub fn for_repo(name: &str) -> Option<Self> {
        let output = Command::new("pacman")
            .args(["-Si", name])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_pacman_output(&stdout, false)
    }

    /// Fetch info, trying installed first, then repo, then AUR
    pub fn fetch(name: &str) -> Option<Self> {
        Self::for_installed(name)
            .or_else(|| Self::for_repo(name))
            .or_else(|| Self::for_aur(name))
    }

    /// Fetch info for an uninstalled AUR package using AUR RPC
    pub fn for_aur(name: &str) -> Option<Self> {
        Self::fetch_aur_rpc(name)
    }

    /// Fetch package info from AUR RPC API
    fn fetch_aur_rpc(name: &str) -> Option<Self> {
        let url = format!(
            "https://aur.archlinux.org/rpc/?v=5&type=info&arg={}",
            url_encode(name)
        );

        let output = Command::new("curl")
            .args(["-s", "-m", "5", &url])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json = String::from_utf8_lossy(&output.stdout);
        let response: AurResponse = serde_json::from_str(&json).ok()?;

        if response.resultcount != 1 {
            return None;
        }

        let pkg = response.results.into_iter().next()?;
        let build_date = pkg.last_modified.map(format_timestamp);

        Some(Self {
            name: pkg.name,
            version: pkg.version,
            description: pkg.description.unwrap_or_default(),
            size: String::new(),
            repository: "AUR".to_string(),
            install_date: None,
            install_reason: None,
            url: pkg.url,
            build_date,
            maintainer: pkg.maintainer,
            votes: pkg.num_votes,
        })
    }

    fn parse_pacman_output(output: &str, is_installed: bool) -> Option<Self> {
        let mut name = String::new();
        let mut version = String::new();
        let mut description = String::new();
        let mut size = String::new();
        let mut repository = String::new();
        let mut install_date = None;
        let mut install_reason = None;
        let mut url = None;
        let mut build_date = None;

        for line in output.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                // URL contains ":" so we need to reconstruct the full value
                let value = if key == "URL" {
                    line.splitn(2, ':').nth(1).unwrap_or("").trim()
                } else {
                    value.trim()
                };

                match key {
                    "Name" => name = value.to_string(),
                    "Version" => version = value.to_string(),
                    "Description" => description = value.to_string(),
                    "Repository" => repository = value.to_string(),
                    "Installed Size" => size = value.to_string(),
                    "Download Size" if !is_installed => size = value.to_string(),
                    "Install Date" => install_date = Some(value.to_string()),
                    "Install Reason" => install_reason = Some(value.to_string()),
                    "URL" => url = Some(value.to_string()),
                    "Build Date" => build_date = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            return None;
        }

        Some(Self {
            name,
            version,
            description,
            size,
            repository,
            install_date,
            install_reason,
            url,
            build_date,
            maintainer: None, // Only available from AUR RPC
            votes: None,      // Only available from AUR RPC
        })
    }
}
