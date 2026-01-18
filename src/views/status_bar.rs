use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::theme::ResolvedTheme;

/// Builder for rendering a consistent status bar across views.
///
/// The status bar has a standard layout:
/// `[Label] [Loading?] Position | Help Text`
pub struct StatusBar<'a> {
    theme: &'a ResolvedTheme,
    label: &'a str,
    loading_text: Option<&'a str>,
    position: Option<(usize, usize)>,
    help_text: &'a str,
    flash_text: Option<&'a str>,
}

impl<'a> StatusBar<'a> {
    pub fn new(theme: &'a ResolvedTheme) -> Self {
        Self {
            theme,
            label: "",
            loading_text: None,
            position: None,
            help_text: "",
            flash_text: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    pub fn loading(mut self, text: &'a str) -> Self {
        self.loading_text = Some(text);
        self
    }

    pub fn position(mut self, current: usize, total: usize) -> Self {
        self.position = Some((current, total));
        self
    }

    pub fn help(mut self, text: &'a str) -> Self {
        self.help_text = text;
        self
    }

    pub fn flash(mut self, text: Option<&'a str>) -> Self {
        self.flash_text = text;
        self
    }

    pub fn render(self, frame: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled(
                format!(" {} ", self.label),
                Style::default()
                    .bg(self.theme.status_bar_bg)
                    .fg(self.theme.status_bar_fg),
            ),
            Span::raw(" "),
        ];

        if let Some(loading) = self.loading_text {
            spans.push(Span::styled(
                loading.to_string(),
                Style::default().fg(self.theme.spinner),
            ));
            spans.push(Span::raw(" | "));
        }

        if let Some((current, total)) = self.position {
            spans.push(Span::styled(
                format!("{}/{}", current, total),
                Style::default().fg(self.theme.foreground_dim),
            ));
            spans.push(Span::raw(" | "));
        }

        if let Some(flash) = self.flash_text {
            spans.push(Span::styled(
                flash.to_string(),
                Style::default().fg(self.theme.foreground),
            ));
        } else {
            spans.push(Span::styled(
                self.help_text.to_string(),
                Style::default().fg(self.theme.foreground_dim),
            ));
        }

        let status = Line::from(spans);
        frame.render_widget(Paragraph::new(status), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ThemeVariant, default_for_variant};
    use ratatui::{Terminal, backend::TestBackend};

    fn render_to_string<F>(width: u16, height: u16, render_fn: F) -> String
    where
        F: FnOnce(&mut Frame),
    {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_fn(frame)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut output = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                output.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
            output.push('\n');
        }
        output
    }

    #[test]
    fn test_status_bar_full() {
        let theme = default_for_variant(ThemeVariant::Dark);
        let output = render_to_string(60, 1, |frame| {
            StatusBar::new(&theme)
                .label("Stories")
                .position(5, 100)
                .help("j/k:nav  ?:help")
                .render(frame, frame.area());
        });

        assert!(output.contains("Stories"));
        assert!(output.contains("5/100"));
        assert!(output.contains("j/k:nav"));
    }

    #[test]
    fn test_status_bar_with_loading() {
        let theme = default_for_variant(ThemeVariant::Dark);
        let output = render_to_string(60, 1, |frame| {
            StatusBar::new(&theme)
                .label("Comments")
                .loading("⠋ Loading...")
                .position(1, 50)
                .help("?:help")
                .render(frame, frame.area());
        });

        assert!(output.contains("Comments"));
        assert!(output.contains("Loading"));
        assert!(output.contains("1/50"));
    }

    #[test]
    fn test_status_bar_minimal() {
        let theme = default_for_variant(ThemeVariant::Dark);
        let output = render_to_string(40, 1, |frame| {
            StatusBar::new(&theme)
                .label("Test")
                .help("q:quit")
                .render(frame, frame.area());
        });

        assert!(output.contains("Test"));
        assert!(output.contains("q:quit"));
    }
}
