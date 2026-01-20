//! Help overlay view showing keybindings.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
};

use crate::app::{App, View};
use crate::help::{HelpItem, comments_overlay_items, stories_overlay_items};
use crate::keys::{Keymap, comments_keymap, global_keymap, stories_keymap};

#[allow(clippy::cast_possible_truncation)] // popup dimensions fit in u16
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    if !app.help_overlay {
        return;
    }

    // Dim the underlying content
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = &mut buf[(x, y)];
            cell.set_style(cell.style().add_modifier(Modifier::DIM));
        }
    }

    let theme = &app.theme;

    // Get view-specific items and keymap
    let (items, keymap): (Vec<HelpItem>, Keymap) = match &app.view {
        View::Stories => (
            stories_overlay_items(),
            global_keymap().extend(stories_keymap()),
        ),
        View::Comments { .. } => (
            comments_overlay_items(),
            global_keymap().extend(comments_keymap()),
        ),
    };

    // Format items for display
    let formatted: Vec<(String, &str)> = items
        .iter()
        .filter_map(|item| item.format_for_overlay(&keymap))
        .collect();

    // Calculate dimensions
    let key_width = formatted.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    let label_width = formatted.iter().map(|(_, l)| l.len()).max().unwrap_or(0);
    let content_width = key_width + 2 + label_width; // 2 for column spacing
    let padding = 2; // 1 char padding on each side
    let popup_width = (content_width + 2 + padding * 2) as u16; // 2 for borders
    let popup_height = (formatted.len() + 2 + 2) as u16; // 2 for borders, 2 for vertical padding

    // Ensure popup fits in area
    let popup_width = popup_width.min(area.width.saturating_sub(4));
    let popup_height = popup_height.min(area.height.saturating_sub(4));
    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Alternating row background
    let alt_row_style = Style::default().bg(theme.selection_bg);

    // Build content lines with alternating backgrounds
    let lines: Vec<Line> = formatted
        .iter()
        .enumerate()
        .map(|(i, (keys, label))| {
            let base_style = if i % 2 == 1 {
                alt_row_style
            } else {
                Style::default()
            };
            // Pad the line to fill the content width for consistent background
            let key_span = Span::styled(
                format!("{keys:>key_width$}"),
                theme.dim_style().patch(base_style),
            );
            let spacer = Span::styled("  ", base_style);
            let label_span = Span::styled(
                format!("{label:<label_width$}"),
                theme.story_title_style().patch(base_style),
            );
            Line::from(vec![key_span, spacer, label_span])
        })
        .collect();

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style())
            .title("Help")
            .title_style(theme.active_tab_style())
            .padding(Padding::uniform(1)),
    );

    frame.render_widget(paragraph, popup_area);
}

const fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::View;
    use crate::test_utils::TestAppBuilder;
    use crate::views::tests::render_to_string;

    #[test]
    fn test_help_overlay_stories() {
        let app = TestAppBuilder::new().help_overlay().build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_help_overlay_comments() {
        let app = TestAppBuilder::new()
            .view(View::Comments {
                story_id: 1,
                story_title: "Test".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .help_overlay()
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_help_overlay_hidden_when_closed() {
        let app = TestAppBuilder::new().build(); // help_overlay is false

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        // Should be empty since overlay is closed
        assert!(output.trim().is_empty());
    }
}
