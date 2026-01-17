use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Debug ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.foreground_dim));

    let mut lines = Vec::new();

    // Running tasks
    let task_count = app.running_tasks.len();
    lines.push(Line::from(vec![
        Span::styled("Tasks: ", Style::default().fg(theme.foreground_dim)),
        Span::styled(
            task_count.to_string(),
            Style::default().fg(if task_count > 0 {
                theme.story_score
            } else {
                theme.foreground
            }),
        ),
    ]));

    for task in &app.running_tasks {
        let elapsed = task.started_at.elapsed();
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("[{}] ", task.id),
                Style::default().fg(theme.foreground_dim),
            ),
            Span::styled(&task.description, Style::default().fg(theme.foreground)),
            Span::styled(
                format!(" ({:.1?})", elapsed),
                Style::default().fg(theme.story_time),
            ),
        ]));
    }

    // Separator
    if !app.running_tasks.is_empty() {
        lines.push(Line::from(""));
    }

    // Recent log entries (newest first, limit to fit area)
    let available_lines = area.height.saturating_sub(3) as usize; // 3 for border + tasks header
    let log_lines = available_lines.saturating_sub(app.running_tasks.len() + 1);

    for entry in app.debug_log.iter().rev().take(log_lines) {
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", entry.message),
            Style::default().fg(theme.foreground_dim),
        )]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}
