use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "lima-hn")]
#[command(about = "A terminal UI for Hacker News", long_about = None)]
pub struct Cli {
    /// Theme name or path to a TOML theme file
    #[arg(short, long)]
    pub theme: Option<String>,

    /// Force dark mode (overrides auto-detection)
    #[arg(long, conflicts_with = "light")]
    pub dark: bool,

    /// Force light mode (overrides auto-detection)
    #[arg(long, conflicts_with = "dark")]
    pub light: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage themes
    Theme(ThemeArgs),
}

#[derive(Args, Debug)]
pub struct ThemeArgs {
    #[command(subcommand)]
    pub command: ThemeCommands,
}

#[derive(Subcommand, Debug)]
pub enum ThemeCommands {
    /// List available themes
    List {
        /// Show detailed information about each theme
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show a theme's configuration
    Show {
        /// Theme name to show
        name: String,

        /// Output format (toml or json)
        #[arg(short, long, default_value = "toml")]
        format: OutputFormat,
    },
    /// Show path where custom themes can be placed
    Path,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Toml,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "toml" => Ok(OutputFormat::Toml),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Invalid format: {}. Use 'toml' or 'json'", s)),
        }
    }
}

pub fn custom_themes_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("lima-hn").join("themes"))
}
