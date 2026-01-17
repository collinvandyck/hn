use super::{ResolvedTheme, Theme, ThemeColor, ThemeColors, ThemeMeta, ThemeVariant};

pub fn all_themes() -> Vec<Theme> {
    vec![
        default_dark(),
        default_light(),
        monokai(),
        dracula(),
        nord(),
        gruvbox_dark(),
        gruvbox_light(),
        solarized_dark(),
        solarized_light(),
        catppuccin_mocha(),
        catppuccin_latte(),
        tokyo_night(),
    ]
}

pub fn by_name(name: &str) -> Option<Theme> {
    all_themes().into_iter().find(|t| t.name == name)
}

pub fn default_for_variant(variant: ThemeVariant) -> ResolvedTheme {
    match variant {
        ThemeVariant::Dark => default_dark().into(),
        ThemeVariant::Light => default_light().into(),
    }
}

fn named(s: &str) -> ThemeColor {
    ThemeColor::Named(s.to_string())
}

fn hex(s: &str) -> ThemeColor {
    ThemeColor::Hex(s.to_string())
}

pub fn default_dark() -> Theme {
    Theme {
        name: "default-dark".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Default dark theme using terminal colors".to_string()),
            variant: ThemeVariant::Dark,
        },
        colors: ThemeColors {
            foreground: named("white"),
            foreground_dim: named("darkgray"),
            border: named("darkgray"),
            selection_bg: named("darkgray"),
            primary: named("yellow"),
            success: named("green"),
            warning: named("yellow"),
            error: named("red"),
            info: named("cyan"),
            story_title: named("white"),
            story_domain: named("darkgray"),
            story_score: named("yellow"),
            story_author: named("cyan"),
            story_comments: named("green"),
            story_time: named("darkgray"),
            comment_text: named("white"),
            comment_depth_colors: vec![
                named("cyan"),
                named("green"),
                named("yellow"),
                named("magenta"),
                named("blue"),
                named("red"),
            ],
            status_bar_bg: named("blue"),
            status_bar_fg: named("white"),
            spinner: named("yellow"),
        },
    }
}

pub fn default_light() -> Theme {
    Theme {
        name: "default-light".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Default light theme using terminal colors".to_string()),
            variant: ThemeVariant::Light,
        },
        colors: ThemeColors {
            foreground: named("black"),
            foreground_dim: named("darkgray"),
            border: named("darkgray"),
            selection_bg: named("lightblue"),
            primary: named("blue"),
            success: named("green"),
            warning: named("yellow"),
            error: named("red"),
            info: named("blue"),
            story_title: named("black"),
            story_domain: named("darkgray"),
            story_score: named("yellow"),
            story_author: named("blue"),
            story_comments: named("green"),
            story_time: named("darkgray"),
            comment_text: named("black"),
            comment_depth_colors: vec![
                named("blue"),
                named("green"),
                named("magenta"),
                named("cyan"),
                named("red"),
                named("yellow"),
            ],
            status_bar_bg: named("blue"),
            status_bar_fg: named("white"),
            spinner: named("blue"),
        },
    }
}

pub fn monokai() -> Theme {
    Theme {
        name: "monokai".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Classic Monokai dark theme".to_string()),
            variant: ThemeVariant::Dark,
        },
        colors: ThemeColors {
            foreground: hex("#F8F8F2"),
            foreground_dim: hex("#75715E"),
            border: hex("#75715E"),
            selection_bg: hex("#49483E"),
            primary: hex("#A6E22E"),
            success: hex("#A6E22E"),
            warning: hex("#E6DB74"),
            error: hex("#F92672"),
            info: hex("#66D9EF"),
            story_title: hex("#F8F8F2"),
            story_domain: hex("#75715E"),
            story_score: hex("#E6DB74"),
            story_author: hex("#66D9EF"),
            story_comments: hex("#A6E22E"),
            story_time: hex("#75715E"),
            comment_text: hex("#F8F8F2"),
            comment_depth_colors: vec![
                hex("#66D9EF"),
                hex("#A6E22E"),
                hex("#E6DB74"),
                hex("#AE81FF"),
                hex("#FD971F"),
                hex("#F92672"),
            ],
            status_bar_bg: hex("#A6E22E"),
            status_bar_fg: hex("#272822"),
            spinner: hex("#E6DB74"),
        },
    }
}

pub fn dracula() -> Theme {
    Theme {
        name: "dracula".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Dracula dark theme".to_string()),
            variant: ThemeVariant::Dark,
        },
        colors: ThemeColors {
            foreground: hex("#F8F8F2"),
            foreground_dim: hex("#6272A4"),
            border: hex("#6272A4"),
            selection_bg: hex("#44475A"),
            primary: hex("#BD93F9"),
            success: hex("#50FA7B"),
            warning: hex("#F1FA8C"),
            error: hex("#FF5555"),
            info: hex("#8BE9FD"),
            story_title: hex("#F8F8F2"),
            story_domain: hex("#6272A4"),
            story_score: hex("#F1FA8C"),
            story_author: hex("#8BE9FD"),
            story_comments: hex("#50FA7B"),
            story_time: hex("#6272A4"),
            comment_text: hex("#F8F8F2"),
            comment_depth_colors: vec![
                hex("#8BE9FD"),
                hex("#50FA7B"),
                hex("#F1FA8C"),
                hex("#BD93F9"),
                hex("#FFB86C"),
                hex("#FF79C6"),
            ],
            status_bar_bg: hex("#BD93F9"),
            status_bar_fg: hex("#282A36"),
            spinner: hex("#F1FA8C"),
        },
    }
}

pub fn nord() -> Theme {
    Theme {
        name: "nord".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Arctic, bluish color palette".to_string()),
            variant: ThemeVariant::Dark,
        },
        colors: ThemeColors {
            foreground: hex("#ECEFF4"),
            foreground_dim: hex("#4C566A"),
            border: hex("#4C566A"),
            selection_bg: hex("#434C5E"),
            primary: hex("#88C0D0"),
            success: hex("#A3BE8C"),
            warning: hex("#EBCB8B"),
            error: hex("#BF616A"),
            info: hex("#81A1C1"),
            story_title: hex("#ECEFF4"),
            story_domain: hex("#4C566A"),
            story_score: hex("#EBCB8B"),
            story_author: hex("#81A1C1"),
            story_comments: hex("#A3BE8C"),
            story_time: hex("#4C566A"),
            comment_text: hex("#ECEFF4"),
            comment_depth_colors: vec![
                hex("#88C0D0"),
                hex("#A3BE8C"),
                hex("#EBCB8B"),
                hex("#B48EAD"),
                hex("#81A1C1"),
                hex("#BF616A"),
            ],
            status_bar_bg: hex("#5E81AC"),
            status_bar_fg: hex("#ECEFF4"),
            spinner: hex("#EBCB8B"),
        },
    }
}

pub fn gruvbox_dark() -> Theme {
    Theme {
        name: "gruvbox-dark".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Gruvbox dark theme".to_string()),
            variant: ThemeVariant::Dark,
        },
        colors: ThemeColors {
            foreground: hex("#EBDBB2"),
            foreground_dim: hex("#928374"),
            border: hex("#928374"),
            selection_bg: hex("#504945"),
            primary: hex("#B8BB26"),
            success: hex("#B8BB26"),
            warning: hex("#FABD2F"),
            error: hex("#FB4934"),
            info: hex("#83A598"),
            story_title: hex("#EBDBB2"),
            story_domain: hex("#928374"),
            story_score: hex("#FABD2F"),
            story_author: hex("#83A598"),
            story_comments: hex("#B8BB26"),
            story_time: hex("#928374"),
            comment_text: hex("#EBDBB2"),
            comment_depth_colors: vec![
                hex("#83A598"),
                hex("#B8BB26"),
                hex("#FABD2F"),
                hex("#D3869B"),
                hex("#8EC07C"),
                hex("#FE8019"),
            ],
            status_bar_bg: hex("#458588"),
            status_bar_fg: hex("#EBDBB2"),
            spinner: hex("#FABD2F"),
        },
    }
}

pub fn gruvbox_light() -> Theme {
    Theme {
        name: "gruvbox-light".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Gruvbox light theme".to_string()),
            variant: ThemeVariant::Light,
        },
        colors: ThemeColors {
            foreground: hex("#3C3836"),
            foreground_dim: hex("#928374"),
            border: hex("#928374"),
            selection_bg: hex("#D5C4A1"),
            primary: hex("#79740E"),
            success: hex("#79740E"),
            warning: hex("#B57614"),
            error: hex("#9D0006"),
            info: hex("#076678"),
            story_title: hex("#3C3836"),
            story_domain: hex("#928374"),
            story_score: hex("#B57614"),
            story_author: hex("#076678"),
            story_comments: hex("#79740E"),
            story_time: hex("#928374"),
            comment_text: hex("#3C3836"),
            comment_depth_colors: vec![
                hex("#076678"),
                hex("#79740E"),
                hex("#B57614"),
                hex("#8F3F71"),
                hex("#427B58"),
                hex("#AF3A03"),
            ],
            status_bar_bg: hex("#076678"),
            status_bar_fg: hex("#FBF1C7"),
            spinner: hex("#B57614"),
        },
    }
}

pub fn solarized_dark() -> Theme {
    Theme {
        name: "solarized-dark".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Solarized dark theme".to_string()),
            variant: ThemeVariant::Dark,
        },
        colors: ThemeColors {
            foreground: hex("#839496"),
            foreground_dim: hex("#586E75"),
            border: hex("#586E75"),
            selection_bg: hex("#073642"),
            primary: hex("#268BD2"),
            success: hex("#859900"),
            warning: hex("#B58900"),
            error: hex("#DC322F"),
            info: hex("#2AA198"),
            story_title: hex("#93A1A1"),
            story_domain: hex("#586E75"),
            story_score: hex("#B58900"),
            story_author: hex("#2AA198"),
            story_comments: hex("#859900"),
            story_time: hex("#586E75"),
            comment_text: hex("#839496"),
            comment_depth_colors: vec![
                hex("#268BD2"),
                hex("#859900"),
                hex("#B58900"),
                hex("#D33682"),
                hex("#2AA198"),
                hex("#CB4B16"),
            ],
            status_bar_bg: hex("#268BD2"),
            status_bar_fg: hex("#FDF6E3"),
            spinner: hex("#B58900"),
        },
    }
}

pub fn solarized_light() -> Theme {
    Theme {
        name: "solarized-light".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Solarized light theme".to_string()),
            variant: ThemeVariant::Light,
        },
        colors: ThemeColors {
            foreground: hex("#657B83"),
            foreground_dim: hex("#93A1A1"),
            border: hex("#93A1A1"),
            selection_bg: hex("#EEE8D5"),
            primary: hex("#268BD2"),
            success: hex("#859900"),
            warning: hex("#B58900"),
            error: hex("#DC322F"),
            info: hex("#2AA198"),
            story_title: hex("#586E75"),
            story_domain: hex("#93A1A1"),
            story_score: hex("#B58900"),
            story_author: hex("#2AA198"),
            story_comments: hex("#859900"),
            story_time: hex("#93A1A1"),
            comment_text: hex("#657B83"),
            comment_depth_colors: vec![
                hex("#268BD2"),
                hex("#859900"),
                hex("#B58900"),
                hex("#D33682"),
                hex("#2AA198"),
                hex("#CB4B16"),
            ],
            status_bar_bg: hex("#268BD2"),
            status_bar_fg: hex("#FDF6E3"),
            spinner: hex("#B58900"),
        },
    }
}

pub fn catppuccin_mocha() -> Theme {
    Theme {
        name: "catppuccin-mocha".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Catppuccin Mocha (darkest variant)".to_string()),
            variant: ThemeVariant::Dark,
        },
        colors: ThemeColors {
            foreground: hex("#CDD6F4"),
            foreground_dim: hex("#6C7086"),
            border: hex("#6C7086"),
            selection_bg: hex("#45475A"),
            primary: hex("#CBA6F7"),
            success: hex("#A6E3A1"),
            warning: hex("#F9E2AF"),
            error: hex("#F38BA8"),
            info: hex("#89DCEB"),
            story_title: hex("#CDD6F4"),
            story_domain: hex("#6C7086"),
            story_score: hex("#F9E2AF"),
            story_author: hex("#89DCEB"),
            story_comments: hex("#A6E3A1"),
            story_time: hex("#6C7086"),
            comment_text: hex("#CDD6F4"),
            comment_depth_colors: vec![
                hex("#89DCEB"),
                hex("#A6E3A1"),
                hex("#F9E2AF"),
                hex("#CBA6F7"),
                hex("#FAB387"),
                hex("#F38BA8"),
            ],
            status_bar_bg: hex("#CBA6F7"),
            status_bar_fg: hex("#1E1E2E"),
            spinner: hex("#F9E2AF"),
        },
    }
}

pub fn catppuccin_latte() -> Theme {
    Theme {
        name: "catppuccin-latte".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Catppuccin Latte (light variant)".to_string()),
            variant: ThemeVariant::Light,
        },
        colors: ThemeColors {
            foreground: hex("#4C4F69"),
            foreground_dim: hex("#9CA0B0"),
            border: hex("#9CA0B0"),
            selection_bg: hex("#DCE0E8"),
            primary: hex("#8839EF"),
            success: hex("#40A02B"),
            warning: hex("#DF8E1D"),
            error: hex("#D20F39"),
            info: hex("#04A5E5"),
            story_title: hex("#4C4F69"),
            story_domain: hex("#9CA0B0"),
            story_score: hex("#DF8E1D"),
            story_author: hex("#04A5E5"),
            story_comments: hex("#40A02B"),
            story_time: hex("#9CA0B0"),
            comment_text: hex("#4C4F69"),
            comment_depth_colors: vec![
                hex("#04A5E5"),
                hex("#40A02B"),
                hex("#DF8E1D"),
                hex("#8839EF"),
                hex("#FE640B"),
                hex("#D20F39"),
            ],
            status_bar_bg: hex("#8839EF"),
            status_bar_fg: hex("#EFF1F5"),
            spinner: hex("#DF8E1D"),
        },
    }
}

pub fn tokyo_night() -> Theme {
    Theme {
        name: "tokyo-night".to_string(),
        meta: ThemeMeta {
            author: Some("lima-hn".to_string()),
            description: Some("Tokyo Night dark theme".to_string()),
            variant: ThemeVariant::Dark,
        },
        colors: ThemeColors {
            foreground: hex("#A9B1D6"),
            foreground_dim: hex("#565F89"),
            border: hex("#565F89"),
            selection_bg: hex("#343B58"),
            primary: hex("#7AA2F7"),
            success: hex("#9ECE6A"),
            warning: hex("#E0AF68"),
            error: hex("#F7768E"),
            info: hex("#7DCFFF"),
            story_title: hex("#A9B1D6"),
            story_domain: hex("#565F89"),
            story_score: hex("#E0AF68"),
            story_author: hex("#7DCFFF"),
            story_comments: hex("#9ECE6A"),
            story_time: hex("#565F89"),
            comment_text: hex("#A9B1D6"),
            comment_depth_colors: vec![
                hex("#7DCFFF"),
                hex("#9ECE6A"),
                hex("#E0AF68"),
                hex("#BB9AF7"),
                hex("#7AA2F7"),
                hex("#F7768E"),
            ],
            status_bar_bg: hex("#7AA2F7"),
            status_bar_fg: hex("#1A1B26"),
            spinner: hex("#E0AF68"),
        },
    }
}
