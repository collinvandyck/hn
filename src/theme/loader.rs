use std::path::Path;

use anyhow::{Context, Result};

use super::Theme;

/// Load a theme from a TOML file
pub fn load_theme_file(path: &Path) -> Result<Theme> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read theme file: {}", path.display()))?;

    let theme: Theme = toml::from_str(&content)
        .with_context(|| format!("Failed to parse theme file: {}", path.display()))?;

    Ok(theme)
}

/// Serialize a theme to TOML format
pub fn theme_to_toml(theme: &Theme) -> Result<String> {
    toml::to_string_pretty(theme).context("Failed to serialize theme to TOML")
}
