use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

pub fn render_to_string<F>(width: u16, height: u16, render_fn: F) -> String
where
    F: FnOnce(&mut Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(render_fn).unwrap();
    buffer_to_string(terminal.backend().buffer())
}

fn buffer_to_string(buffer: &Buffer) -> String {
    let mut lines = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            line.push_str(cell.symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod test {
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
