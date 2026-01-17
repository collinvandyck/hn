mod api;
mod app;
mod cli;
mod event;
mod keys;
mod theme;
mod tui;
mod views;

#[cfg(test)]
mod test_utils;

use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use ratatui::Frame;

use app::{App, View};
use cli::{Cli, Commands, OutputFormat, ThemeArgs, ThemeCommands};
use event::Event;
use theme::{
    ResolvedTheme, ThemeVariant, all_themes, by_name, default_for_variant, detect_terminal_theme,
    load_theme_file,
};
use tui::EventHandler;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Theme(theme_args)) = cli.command {
        return handle_theme_command(theme_args);
    }
    run_tui(cli).await
}

fn handle_theme_command(args: ThemeArgs) -> Result<()> {
    match args.command {
        ThemeCommands::List { verbose } => {
            let themes = all_themes();
            if verbose {
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
            let theme = by_name(&name).with_context(|| format!("Theme '{}' not found", name))?;

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
            if let Some(path) = cli::custom_themes_dir() {
                println!("{}", path.display());
            } else {
                eprintln!("Could not determine config directory");
            }
        }
    }
    Ok(())
}

fn resolve_theme(cli: &Cli) -> Result<ResolvedTheme> {
    let variant = if cli.dark {
        ThemeVariant::Dark
    } else if cli.light {
        ThemeVariant::Light
    } else {
        detect_terminal_theme()
    };

    if let Some(theme_arg) = &cli.theme {
        let path = Path::new(theme_arg);
        if path.exists() && path.extension().map(|e| e == "toml").unwrap_or(false) {
            let theme = load_theme_file(path)?;
            return Ok(theme.into());
        }

        if let Some(theme) = by_name(theme_arg) {
            return Ok(theme.into());
        }

        if let Some(custom_dir) = cli::custom_themes_dir() {
            let custom_path = custom_dir.join(format!("{}.toml", theme_arg));
            if custom_path.exists() {
                let theme = load_theme_file(&custom_path)?;
                return Ok(theme.into());
            }
        }

        anyhow::bail!(
            "Theme '{}' not found. Use 'lima-hn theme list' to see available themes.",
            theme_arg
        );
    }

    Ok(default_for_variant(variant))
}

async fn run_tui(cli: Cli) -> Result<()> {
    let resolved_theme = resolve_theme(&cli)?;
    let mut terminal = tui::init()?;
    let mut app = App::new(resolved_theme);
    let mut events = EventHandler::new(250);

    app.load_stories();

    loop {
        terminal.draw(|frame| render(&app, frame))?;

        // Poll async results (non-blocking)
        while let Ok(result) = app.result_rx.try_recv() {
            app.handle_async_result(result);
        }

        match events.next().await? {
            Event::Key(key) => {
                if let Some(msg) = keys::handle_key(key, &app) {
                    app.update(msg);
                }
            }
            Event::Tick | Event::Resize | Event::Mouse => {}
        }

        if app.should_quit {
            break;
        }
    }

    tui::restore()?;
    Ok(())
}

fn render(app: &App, frame: &mut Frame) {
    use ratatui::layout::{Constraint, Layout};

    let area = frame.area();

    // Split area for debug pane if visible
    let (main_area, debug_area) = if app.debug_visible {
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
}
