mod api;
mod app;
mod cli;
mod comment_tree;
mod event;
mod help;
mod keys;
mod logging;
mod settings;
mod storage;
mod theme;
mod time;
mod tui;
mod views;
mod widgets;

#[cfg(test)]
mod test_utils;

use anyhow::{Context, Result, bail};
use app::{App, Message, View};
use clap::Parser;
use cli::{Cli, Commands, OutputFormat, ThemeArgs, ThemeCommands};
use event::Event;
use ratatui::Frame;
use settings::Settings;
use std::path::{Path, PathBuf};
use std::time::Duration;
use storage::{Storage, StorageLocation};
use theme::{
    ResolvedTheme, ThemeVariant, all_themes, by_name, default_for_variant, detect_terminal_theme,
    load_theme_file,
};
use tokio::time::interval;
use tui::CrosstermEvents;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Theme(theme_args)) = &cli.command {
        return handle_theme_command(theme_args, cli.config_dir.as_ref());
    }

    // Initialize logging before entering alt screen
    let config_dir = settings::config_dir(cli.config_dir.as_ref());
    let _guard = config_dir.as_ref().and_then(|dir| {
        let log_path = settings::log_path(dir);
        logging::init(&log_path, cli.verbose)
    });

    tracing::info!("starting");

    let terminal = tui::init()?;
    let result = run_tui(cli, terminal).await;
    tui::restore()?;

    tracing::info!("exiting");
    result
}

fn handle_theme_command(args: &ThemeArgs, custom_config_dir: Option<&PathBuf>) -> Result<()> {
    match &args.command {
        ThemeCommands::List { verbose } => {
            let themes = all_themes();
            if *verbose {
                for theme in themes {
                    println!(
                        "{:<20} {:?}  {}",
                        theme.name,
                        theme.meta.variant,
                        theme.meta.description.as_deref().unwrap_or("")
                    );
                }
            } else {
                for theme in themes {
                    println!("{}", theme.name);
                }
            }
        }
        ThemeCommands::Show { name, format } => {
            let theme = by_name(name).with_context(|| format!("Theme '{}' not found", name))?;

            match format {
                OutputFormat::Toml => {
                    let toml = theme::loader::theme_to_toml(&theme)
                        .context("Failed to serialize theme")?;
                    println!("{}", toml);
                }
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&theme)
                        .context("Failed to serialize theme to JSON")?;
                    println!("{}", json);
                }
            }
        }
        ThemeCommands::Path => {
            let config_dir = settings::config_dir(custom_config_dir)
                .context("Could not determine config directory")?;
            println!("{}", settings::themes_dir(&config_dir).display());
        }
    }
    Ok(())
}

fn resolve_theme(
    cli: &Cli,
    settings: &Settings,
    config_dir: Option<&PathBuf>,
) -> Result<ResolvedTheme> {
    let variant = if cli.dark {
        ThemeVariant::Dark
    } else if cli.light {
        ThemeVariant::Light
    } else {
        detect_terminal_theme()
    };

    // Priority: CLI --theme > settings file > default
    let theme_name = cli.theme.as_ref().or(settings.theme.as_ref());

    if let Some(theme_arg) = theme_name {
        let path = Path::new(theme_arg);
        if path.exists() && path.extension().map(|e| e == "toml").unwrap_or(false) {
            let theme = load_theme_file(path)?;
            return Ok(theme.into());
        }

        if let Some(theme) = by_name(theme_arg) {
            return Ok(theme.into());
        }

        if let Some(config_dir) = config_dir {
            let custom_path = settings::themes_dir(config_dir).join(format!("{}.toml", theme_arg));
            if custom_path.exists() {
                let theme = load_theme_file(&custom_path)?;
                return Ok(theme.into());
            }
        }

        anyhow::bail!(
            "Theme '{}' not found. Use 'hn theme list' to see available themes.",
            theme_arg
        );
    }

    Ok(default_for_variant(variant))
}

async fn run_tui(cli: Cli, mut terminal: tui::Tui) -> Result<()> {
    let config_dir = settings::config_dir(cli.config_dir.as_ref());
    let (settings, storage) = match config_dir.as_ref() {
        Some(dir) => {
            let path = settings::settings_path(dir);
            let settings = Settings::load(&path)
                .with_context(|| format!("Failed to load settings from {}", path.display()))?;
            let storage = Storage::open(StorageLocation::Path(settings::db_path(dir)))
                .context("Failed to open storage database")?;
            (settings, Some(storage))
        }
        None => (Settings::default(), None),
    };
    let resolved_theme = resolve_theme(&cli, &settings, config_dir.as_ref())?;
    let mut app = App::new(resolved_theme, config_dir, storage);
    let mut events = CrosstermEvents::new();
    let mut tick = interval(Duration::from_millis(16));
    let mut last_height: Option<u16> = None;

    app.load_stories();

    loop {
        terminal.draw(|frame| render(&app, frame))?;

        // Track viewport height changes for dynamic story loading
        let current_height = terminal.size()?.height;
        if last_height != Some(current_height) {
            last_height = Some(current_height);
            app.update(Message::UpdateViewportHeight(current_height));
        }

        if app.should_quit {
            break;
        }

        tokio::select! {
            event = events.next() => {
                match event? {
                    Event::Key(key) => {
                        if let Some(msg) = keys::handle_key(key, &app) {
                            app.update(msg);
                        }
                    }
                    Event::Resize => {}
                }
            }
            result = app.result_rx.recv() => {
                if let Some(result) = result {
                    app.handle_async_result(result);
                }
            }
            _ = tick.tick() => {}
        }
    }

    if let Some(err) = app.load.error.as_ref() {
        bail!("{err}")
    }
    Ok(())
}

fn render(app: &App, frame: &mut Frame) {
    use ratatui::layout::{Constraint, Layout};

    let area = frame.area();

    // Split area for debug pane if visible
    let (main_area, debug_area) = if app.debug.visible {
        let chunks = Layout::vertical([
            Constraint::Min(0),     // Main content
            Constraint::Length(10), // Debug pane
        ])
        .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    match &app.view {
        View::Stories => views::stories::render(frame, app, main_area),
        View::Comments { .. } => views::comments::render(frame, app, main_area),
    }

    if let Some(debug_area) = debug_area {
        views::debug::render(frame, app, debug_area);
    }

    // Render theme picker overlay if open
    if app.theme_picker.is_some() {
        views::theme_picker::render(frame, app, area);
    }

    // Render help overlay if open (but not if theme picker is open)
    if app.help_overlay && app.theme_picker.is_none() {
        views::help_overlay::render(frame, app, area);
    }
}
