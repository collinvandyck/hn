mod builtin;
mod detect;
pub mod loader;

pub use builtin::{all_themes, by_name, default_for_variant};
pub use detect::detect_terminal_theme;
pub use loader::load_theme_file;

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    #[serde(default)]
    pub meta: ThemeMeta,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeMeta {
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub variant: ThemeVariant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeColor {
    Named(String),
    Hex(String),
    Rgb { r: u8, g: u8, b: u8 },
    Indexed(u8),
}

impl ThemeColor {
    pub fn to_color(&self) -> Color {
        match self {
            ThemeColor::Named(name) => Self::parse_named(name),
            ThemeColor::Hex(hex) => Self::parse_hex(hex),
            ThemeColor::Rgb { r, g, b } => Color::Rgb(*r, *g, *b),
            ThemeColor::Indexed(idx) => Color::Indexed(*idx),
        }
    }

    fn parse_named(name: &str) -> Color {
        match name.to_lowercase().as_str() {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "gray" | "grey" => Color::Gray,
            "darkgray" | "darkgrey" | "dark_gray" => Color::DarkGray,
            "lightred" | "light_red" => Color::LightRed,
            "lightgreen" | "light_green" => Color::LightGreen,
            "lightyellow" | "light_yellow" => Color::LightYellow,
            "lightblue" | "light_blue" => Color::LightBlue,
            "lightmagenta" | "light_magenta" => Color::LightMagenta,
            "lightcyan" | "light_cyan" => Color::LightCyan,
            "white" => Color::White,
            "reset" | "default" => Color::Reset,
            _ => Color::Reset,
        }
    }

    fn parse_hex(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color::Rgb(r, g, b);
            }
        }
        Color::Reset
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub foreground: ThemeColor,
    pub foreground_dim: ThemeColor,
    pub border: ThemeColor,
    pub selection_bg: ThemeColor,
    pub primary: ThemeColor,
    pub success: ThemeColor,
    pub warning: ThemeColor,
    pub error: ThemeColor,
    pub info: ThemeColor,
    pub story_title: ThemeColor,
    pub story_domain: ThemeColor,
    pub story_score: ThemeColor,
    pub story_author: ThemeColor,
    pub story_comments: ThemeColor,
    pub story_time: ThemeColor,
    pub comment_text: ThemeColor,
    pub comment_depth_colors: Vec<ThemeColor>,
    pub status_bar_bg: ThemeColor,
    pub status_bar_fg: ThemeColor,
    pub spinner: ThemeColor,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResolvedTheme {
    pub name: String,
    pub variant: ThemeVariant,
    pub foreground: Color,
    pub foreground_dim: Color,
    pub border: Color,
    pub selection_bg: Color,
    pub primary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub story_title: Color,
    pub story_domain: Color,
    pub story_score: Color,
    pub story_author: Color,
    pub story_comments: Color,
    pub story_time: Color,
    pub comment_text: Color,
    pub comment_depth_colors: Vec<Color>,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub spinner: Color,
}

impl ResolvedTheme {
    pub fn depth_color(&self, depth: usize) -> Color {
        if self.comment_depth_colors.is_empty() {
            return self.primary;
        }
        self.comment_depth_colors[depth % self.comment_depth_colors.len()]
    }
}

impl From<Theme> for ResolvedTheme {
    fn from(theme: Theme) -> Self {
        let c = theme.colors;
        ResolvedTheme {
            name: theme.name,
            variant: theme.meta.variant,
            foreground: c.foreground.to_color(),
            foreground_dim: c.foreground_dim.to_color(),
            border: c.border.to_color(),
            selection_bg: c.selection_bg.to_color(),
            primary: c.primary.to_color(),
            success: c.success.to_color(),
            warning: c.warning.to_color(),
            error: c.error.to_color(),
            info: c.info.to_color(),
            story_title: c.story_title.to_color(),
            story_domain: c.story_domain.to_color(),
            story_score: c.story_score.to_color(),
            story_author: c.story_author.to_color(),
            story_comments: c.story_comments.to_color(),
            story_time: c.story_time.to_color(),
            comment_text: c.comment_text.to_color(),
            comment_depth_colors: c.comment_depth_colors.iter().map(|c| c.to_color()).collect(),
            status_bar_bg: c.status_bar_bg.to_color(),
            status_bar_fg: c.status_bar_fg.to_color(),
            spinner: c.spinner.to_color(),
        }
    }
}
