use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::app::App;
use crate::help::theme_picker_help;
use crate::keys::theme_picker_keymap;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let Some(picker) = &app.theme_picker else {
        return;
    };

    let theme = &app.theme;

    // Calculate centered popup size
    let popup_width = 40.min(area.width.saturating_sub(4));
    let popup_height = 16.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Split popup into list and help areas
    let chunks = Layout::vertical([
        Constraint::Min(0),    // Theme list
        Constraint::Length(1), // Help line
    ])
    .split(popup_area);

    // Build theme list items
    let items: Vec<ListItem> = picker
        .themes
        .iter()
        .map(|t| ListItem::new(t.name.clone()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style())
                .title("Theme"),
        )
        .highlight_style(theme.selection_style())
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(picker.selected));
    frame.render_stateful_widget(list, chunks[0], &mut state);

    // Help line
    let keymap = theme_picker_keymap();
    let help_text = theme_picker_help().format(&keymap, true);
    let help = Paragraph::new(help_text).style(theme.dim_style());
    frame.render_widget(help, chunks[1]);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestAppBuilder;
    use crate::views::tests::render_to_string;

    #[test]
    fn test_theme_picker_renders() {
        let mut app = TestAppBuilder::new().build();
        app.update(crate::app::Message::OpenThemePicker);

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_theme_picker_selection() {
        let mut app = TestAppBuilder::new().build();
        app.update(crate::app::Message::OpenThemePicker);
        app.update(crate::app::Message::ThemePickerDown);
        app.update(crate::app::Message::ThemePickerDown);

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }
}
