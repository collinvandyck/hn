use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::api::{Feed, Story};
use crate::app::App;

/// Render the stories list view
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Feed tabs
        Constraint::Min(0),    // Story list
        Constraint::Length(1), // Status bar
    ])
    .split(area);

    render_feed_tabs(frame, app, chunks[0]);
    render_story_list(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);
}

fn render_feed_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tabs: Vec<Span> = Feed::all()
        .iter()
        .enumerate()
        .flat_map(|(i, feed)| {
            let style = if *feed == app.feed {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            vec![
                Span::styled(format!("[{}]", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(feed.label(), style),
                Span::raw("  "),
            ]
        })
        .collect();

    let tabs_line = Line::from(tabs);
    frame.render_widget(Paragraph::new(tabs_line), area);
}

fn render_story_list(frame: &mut Frame, app: &App, area: Rect) {
    if app.loading {
        let loading = Paragraph::new("Loading...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Stories"));
        frame.render_widget(loading, area);
        return;
    }

    if let Some(err) = &app.error {
        let error = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title("Error"));
        frame.render_widget(error, area);
        return;
    }

    let items: Vec<ListItem> = app
        .stories
        .iter()
        .enumerate()
        .map(|(i, story)| story_to_list_item(story, i + 1))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "{} Stories",
            app.feed.label()
        )))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    frame.render_stateful_widget(list, area, &mut state);
}

fn story_to_list_item(story: &Story, rank: usize) -> ListItem<'static> {
    let title_line = Line::from(vec![
        Span::styled(
            format!("{:>3}. ", rank),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(story.title.clone(), Style::default().fg(Color::White)),
        Span::styled(
            format!(" ({})", story.domain()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let meta_line = Line::from(vec![
        Span::raw("     "),
        Span::styled(
            format!("▲ {}", story.score),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" | "),
        Span::styled(story.by.clone(), Style::default().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::styled(
            format!("{} comments", story.descendants),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" | "),
        Span::styled(
            format_time(story.time),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    ListItem::new(vec![title_line, meta_line])
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.show_help {
        "j/k:nav  g/G:top/bottom  o:open  l:comments  1-6:feeds  r:refresh  q:quit  ?:hide help"
    } else {
        "?:help  q:quit"
    };

    let status = Line::from(vec![
        Span::styled(
            format!(" {} ", app.feed.label()),
            Style::default().bg(Color::Blue).fg(Color::White),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{}/{}", app.selected_index + 1, app.stories.len()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" | "),
        Span::styled(help_text, Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(status), area);
}

fn format_time(timestamp: u64) -> String {
    use chrono::{TimeZone, Utc};

    let now = Utc::now();
    let then = Utc.timestamp_opt(timestamp as i64, 0).single();

    match then {
        Some(t) => {
            let diff = now.signed_duration_since(t);
            if diff.num_hours() < 1 {
                format!("{}m ago", diff.num_minutes())
            } else if diff.num_hours() < 24 {
                format!("{}h ago", diff.num_hours())
            } else {
                format!("{}d ago", diff.num_days())
            }
        }
        None => "?".to_string(),
    }
}
