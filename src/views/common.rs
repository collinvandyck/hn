use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
};

use crate::theme::ResolvedTheme;

/// Render an error message in a bordered block.
pub fn render_error(frame: &mut Frame, error: &str, theme: &ResolvedTheme, area: Rect) {
    let widget = Paragraph::new(error)
        .style(Style::default().fg(theme.error))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title("Error"),
        );
    frame.render_widget(widget, area);
}

/// Render a loading message in a bordered block.
pub fn render_loading(
    frame: &mut Frame,
    message: &str,
    title: &str,
    theme: &ResolvedTheme,
    area: Rect,
) {
    let widget = Paragraph::new(message)
        .style(Style::default().fg(theme.warning))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(title),
        );
    frame.render_widget(widget, area);
}
