//! View testing utilities for snapshot testing with ratatui's TestBackend.

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Frame;
use ratatui::Terminal;

/// Render a view to a string for snapshot testing.
///
/// This creates a virtual terminal of the specified dimensions,
/// runs the render function, and returns the buffer contents as a string.
///
/// # Example
///
/// ```ignore
/// let output = render_to_string(80, 24, |frame| {
///     views::stories::render(frame, &app, frame.area());
/// });
/// insta::assert_snapshot!(output);
/// ```
pub fn render_to_string<F>(width: u16, height: u16, render_fn: F) -> String
where
    F: FnOnce(&mut Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(render_fn).unwrap();
    buffer_to_string(terminal.backend().buffer())
}

/// Convert a ratatui Buffer to a readable string.
///
/// Each row becomes a line, with trailing whitespace trimmed for cleaner snapshots.
fn buffer_to_string(buffer: &Buffer) -> String {
    let mut lines = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            line.push_str(cell.symbol());
        }
        // Trim trailing whitespace for cleaner snapshots
        lines.push(line.trim_end().to_string());
    }
    // Remove trailing empty lines
    while lines.last().map_or(false, |l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::widgets::Paragraph;

    #[test]
    fn test_render_to_string_basic() {
        let output = render_to_string(20, 3, |frame| {
            let paragraph = Paragraph::new("Hello, world!");
            frame.render_widget(paragraph, frame.area());
        });
        assert!(output.contains("Hello, world!"));
    }
}
