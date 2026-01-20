use chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::theme::ResolvedTheme;
use crate::time::format_relative;

/// Render an error message in a bordered block.
pub fn render_error(frame: &mut Frame, error: &str, theme: &ResolvedTheme, area: Rect) {
    let widget = Paragraph::new(error).style(theme.error_style()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style())
            .title("Error"),
    );
    frame.render_widget(widget, area);
}

/// Render a line with an optional right-aligned timestamp showing when data was fetched.
/// If `fetched_at` is Some, shows "loaded Xm ago" (dimmed if <5m, normal if >=5m).
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // timestamp fits in u64; width fits in u16
pub fn render_with_timestamp(
    frame: &mut Frame,
    content_line: Line,
    fetched_at: Option<u64>,
    now: DateTime<Utc>,
    theme: &ResolvedTheme,
    area: Rect,
) {
    if let Some(ts) = fetched_at {
        let age_text = format!("loaded {}", format_relative(ts, now));
        let now_ts = now.timestamp() as u64;
        let age_secs = now_ts.saturating_sub(ts);
        let is_stale = age_secs >= 300; // 5 minutes
        let style = if is_stale {
            Style::default().fg(theme.foreground)
        } else {
            theme.dim_style()
        };
        let timestamp_span = Span::styled(age_text, style);
        let timestamp_width = timestamp_span.width() as u16;
        let chunks = Layout::horizontal([Constraint::Min(0), Constraint::Length(timestamp_width)])
            .split(area);
        frame.render_widget(Paragraph::new(content_line), chunks[0]);
        frame.render_widget(Paragraph::new(Line::from(timestamp_span)), chunks[1]);
    } else {
        frame.render_widget(Paragraph::new(content_line), area);
    }
}
