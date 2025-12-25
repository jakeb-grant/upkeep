use crate::config::config_dir;
use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct RebuildCheck {
    pub name: String,
    pub command: Vec<String>,
    pub error_patterns: Vec<String>,
    pub rebuild: String,
}

#[derive(Debug, Deserialize)]
struct ChecksConfig {
    #[serde(default)]
    check: Vec<RebuildCheck>,
}

pub fn checks_path() -> PathBuf {
    config_dir().join("checks.toml")
}

pub fn load_checks() -> Result<Vec<RebuildCheck>> {
    let path = checks_path();
    if !path.exists() {
        create_default_checks()?;
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)?;
    let config: ChecksConfig = toml::from_str(&content)?;
    Ok(config.check)
}

fn create_default_checks() -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;

    let content = r#"# Upkeep rebuild checks configuration
#
# Each [[check]] block defines an application to monitor for version mismatch issues.
# When the command outputs any of the error_patterns to stderr, the app needs rebuilding.
#
# Fields:
#   name           - Display name for the check
#   command        - Command to run (as array of arguments)
#   error_patterns - Strings to look for in stderr that indicate a rebuild is needed
#   rebuild        - Shell command to run to fix the issue

# Example check (uncomment and modify as needed):
# [[check]]
# name = "elephant"
# command = ["timeout", "3", "elephant"]
# error_patterns = ["plugin was built with a different version"]
# rebuild = "yay -S --rebuild $(pacman -Qqm | grep elephant)"

# [[check]]
# name = "obs-studio"
# command = ["timeout", "3", "obs", "--help"]
# error_patterns = ["ABI mismatch", "symbol lookup error"]
# rebuild = "yay -S --rebuild obs-studio"
"#;

    std::fs::write(checks_path(), content)?;
    Ok(())
}
