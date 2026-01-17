use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::api::Comment;
use crate::app::{App, View};

/// Render the comments view
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let story_title = match &app.view {
        View::Comments { story_title, .. } => story_title.clone(),
        _ => String::new(),
    };

    let chunks = Layout::vertical([
        Constraint::Length(2), // Story title
        Constraint::Min(0),    // Comments
        Constraint::Length(1), // Status bar
    ])
    .split(area);

    render_header(frame, &story_title, chunks[0]);
    render_comment_list(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, title: &str, area: Rect) {
    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        .wrap(Wrap { trim: true });
    frame.render_widget(header, area);
}

fn render_comment_list(frame: &mut Frame, app: &App, area: Rect) {
    if app.loading {
        let loading = Paragraph::new("Loading comments...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Comments"));
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

    if app.comments.is_empty() {
        let empty = Paragraph::new("No comments yet")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Comments"));
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .comments
        .iter()
        .map(comment_to_list_item)
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Comments ({})", app.comments.len())),
        )
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

fn comment_to_list_item(comment: &Comment) -> ListItem<'static> {
    // Depth indicator with visual indentation
    let indent = "  ".repeat(comment.depth);
    let depth_marker = if comment.depth > 0 {
        format!("{}└─ ", indent)
    } else {
        String::new()
    };

    // Author and time
    let meta_line = Line::from(vec![
        Span::raw(depth_marker.clone()),
        Span::styled(comment.by.clone(), Style::default().fg(Color::Cyan)),
        Span::raw(" • "),
        Span::styled(
            format_time(comment.time),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    // Comment text - strip HTML and truncate
    let text = strip_html(&comment.text);
    let truncated = if text.len() > 200 {
        format!("{}...", &text[..200])
    } else {
        text
    };

    let text_line = Line::from(vec![
        Span::raw(format!("{}  ", "  ".repeat(comment.depth))),
        Span::styled(truncated, Style::default().fg(Color::White)),
    ]);

    ListItem::new(vec![meta_line, text_line, Line::from("")])
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.show_help {
        "j/k:nav  g/G:top/bottom  h/Esc:back  r:refresh  q:quit  ?:hide help"
    } else {
        "h:back  ?:help  q:quit"
    };

    let status = Line::from(vec![
        Span::styled(
            " Comments ",
            Style::default().bg(Color::Green).fg(Color::Black),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{}/{}", app.selected_index + 1, app.comments.len()),
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

fn strip_html(html: &str) -> String {
    // Simple HTML stripping - replace common tags and entities
    html.replace("<p>", "\n")
        .replace("</p>", "")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<i>", "")
        .replace("</i>", "")
        .replace("<b>", "")
        .replace("</b>", "")
        .replace("<code>", "`")
        .replace("</code>", "`")
        .replace("<pre>", "")
        .replace("</pre>", "")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("<a href=\"", "[")
        .replace("\" rel=\"nofollow\">", "](")
        .replace("</a>", ")")
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}
