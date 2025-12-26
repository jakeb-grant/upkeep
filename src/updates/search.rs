use super::util::url_encode;
use serde::Deserialize;
use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: String,
    pub installed: bool,
    pub selected: bool,
}

/// AUR RPC search response
#[derive(Debug, Deserialize)]
struct AurSearchResponse {
    results: Vec<AurSearchResult>,
}

/// AUR package from search results
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AurSearchResult {
    name: String,
    version: String,
    description: Option<String>,
}

/// Search for packages in official repos using pacman -Ss
fn search_pacman(query: &str) -> Vec<SearchResult> {
    if query.len() < 2 {
        return Vec::new();
    }

    let output = Command::new("pacman")
        .args(["-Ss", query])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pacman_search(&stdout)
}

/// Parse pacman -Ss output
/// Format:
/// repo/name version [installed]
///     Description text
fn parse_pacman_search(output: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let mut lines = output.lines().peekable();

    while let Some(line) = lines.next() {
        // Package line: repo/name version [installed]
        if !line.starts_with(' ') && line.contains('/') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let repo_name: Vec<&str> = parts[0].splitn(2, '/').collect();
                if repo_name.len() == 2 {
                    let repository = repo_name[0].to_string();
                    let name = repo_name[1].to_string();
                    let version = parts[1].to_string();
                    let installed = line.contains("[installed");

                    // Get description from next line
                    let description = if let Some(desc_line) = lines.peek() {
                        if desc_line.starts_with("    ") {
                            let desc = desc_line.trim().to_string();
                            lines.next();
                            desc
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    results.push(SearchResult {
                        name,
                        version,
                        description,
                        repository,
                        installed,
                        selected: false,
                    });
                }
            }
        }
    }

    results
}

/// Search for packages in AUR using RPC API
fn search_aur(query: &str) -> Vec<SearchResult> {
    if query.len() < 2 {
        return Vec::new();
    }

    // Use curl to fetch from AUR RPC
    let url = format!(
        "https://aur.archlinux.org/rpc/?v=5&type=search&arg={}",
        url_encode(query)
    );

    let output = Command::new("curl")
        .args(["-s", "-m", "5", &url])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let json = String::from_utf8_lossy(&output.stdout);
    let response: AurSearchResponse = match serde_json::from_str(&json) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    response
        .results
        .into_iter()
        .map(|pkg| SearchResult {
            name: pkg.name,
            version: pkg.version,
            description: pkg.description.unwrap_or_default(),
            repository: "AUR".to_string(),
            installed: false, // Will be checked separately
            selected: false,
        })
        .collect()
}

/// Get list of installed package names for checking
fn get_installed_names() -> HashSet<String> {
    let output = Command::new("pacman")
        .args(["-Qq"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect()
        }
        _ => HashSet::new(),
    }
}

/// Search for packages in both official repos and AUR
pub fn search_packages(query: &str) -> Vec<SearchResult> {
    if query.len() < 2 {
        return Vec::new();
    }

    let installed = get_installed_names();

    // Search official repos
    let mut results = search_pacman(query);

    // Search AUR
    let mut aur_results = search_aur(query);

    // Mark AUR packages as installed if they are
    for result in &mut aur_results {
        result.installed = installed.contains(&result.name);
    }

    // Deduplicate (prefer official repos over AUR)
    let repo_names: HashSet<String> = results.iter().map(|r| r.name.clone()).collect();
    aur_results.retain(|r| !repo_names.contains(&r.name));

    results.extend(aur_results);

    // Sort: installed last, then alphabetically
    results.sort_by(|a, b| {
        match (a.installed, b.installed) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.name.cmp(&b.name),
        }
    });

    results
}
