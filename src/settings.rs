use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const APP_SENTINEL: &str = "5xx.engineer-hn";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "_app")]
    pub app: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            app: APP_SENTINEL.to_string(),
            theme: None,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read settings from {}", path.display()))?;

        let settings: Settings = toml::from_str(&content)
            .with_context(|| format!("Failed to parse settings from {}", path.display()))?;

        settings.validate()?;
        Ok(settings)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let content =
            toml::to_string_pretty(self).with_context(|| "Failed to serialize settings")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write settings to {}", path.display()))?;

        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.app != APP_SENTINEL {
            bail!(
                "Settings file appears to belong to another application (expected _app = '{}', found '{}')",
                APP_SENTINEL,
                self.app
            );
        }
        Ok(())
    }
}

pub fn config_dir(custom: Option<&PathBuf>) -> Option<PathBuf> {
    custom
        .cloned()
        .or_else(|| dirs::home_dir().map(|p| p.join(".config").join("hn")))
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.toml")
}

pub fn themes_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("themes")
}

pub fn db_path(config_dir: &Path) -> PathBuf {
    config_dir.join("data.db")
}

pub fn log_path(config_dir: &Path) -> PathBuf {
    config_dir.join("hn.log")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_missing_file_returns_default() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("settings.toml");

        let settings = Settings::load(&path).unwrap();

        assert_eq!(settings.app, "5xx.engineer-hn");
        assert!(settings.theme.is_none());
    }

    #[test]
    fn load_valid_settings() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("settings.toml");

        fs::write(&path, "_app = \"5xx.engineer-hn\"\ntheme = \"monokai\"\n").unwrap();

        let settings = Settings::load(&path).unwrap();

        assert_eq!(settings.app, "5xx.engineer-hn");
        assert_eq!(settings.theme.as_deref(), Some("monokai"));
    }

    #[test]
    fn wrong_sentinel_returns_error() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("settings.toml");

        fs::write(&path, "_app = \"other-app\"\n").unwrap();

        let result = Settings::load(&path);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("another application"));
    }

    #[test]
    fn save_creates_parent_directories() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nested").join("dir").join("settings.toml");

        let settings = Settings {
            theme: Some("dracula".to_string()),
            ..Default::default()
        };

        settings.save(&path).unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("_app = \"5xx.engineer-hn\""));
        assert!(content.contains("theme = \"dracula\""));
    }

    #[test]
    fn round_trip_serialization() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("settings.toml");

        let original = Settings {
            theme: Some("nord".to_string()),
            ..Default::default()
        };

        original.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();

        assert_eq!(loaded.app, original.app);
        assert_eq!(loaded.theme, original.theme);
    }

    #[test]
    fn config_dir_uses_custom_when_provided() {
        let custom = PathBuf::from("/custom/path");
        let result = config_dir(Some(&custom));
        assert_eq!(result, Some(PathBuf::from("/custom/path")));
    }

    #[test]
    fn config_dir_falls_back_to_default() {
        let result = config_dir(None);
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("hn"));
    }
}
