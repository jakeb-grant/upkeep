use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_aur_helper")]
    pub aur_helper: String,
}

fn default_aur_helper() -> String {
    "yay".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aur_helper: default_aur_helper(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            // Create default config
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir)?;

        let content = format!(
            r#"# Upkeep configuration

# AUR helper to use for updates (default: yay)
# Alternatives: paru, pikaur, etc.
aur_helper = "{}"
"#,
            self.aur_helper
        );

        std::fs::write(config_path(), content)?;
        Ok(())
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("upkeep")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}
