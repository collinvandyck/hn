use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::api::{Feed, Story};
use crate::app::App;
use crate::theme::ResolvedTheme;
use crate::time::{Clock, format_relative};
use crate::views::common::render_error;
use crate::views::status_bar::StatusBar;

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
    use super::spinner::spinner_frame;

    let theme = &app.theme;
    let mut spans: Vec<Span> = Feed::all()
        .iter()
        .enumerate()
        .flat_map(|(i, feed)| {
            let style = if *feed == app.feed {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground_dim)
            };
            vec![
                Span::styled(
                    format!("[{}]", i + 1),
                    Style::default().fg(theme.foreground_dim),
                ),
                Span::styled(feed.label(), style),
                Span::raw("  "),
            ]
        })
        .collect();

    if app.load.should_show_spinner() {
        spans.push(Span::styled(
            spinner_frame(app.load.loading_start),
            Style::default().fg(theme.spinner),
        ));
    }

    let tabs_line = Line::from(spans);
    frame.render_widget(Paragraph::new(tabs_line), area);
}

fn render_story_list(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    if let Some(err) = &app.load.error {
        render_error(frame, err, theme, area);
        return;
    }

    let items: Vec<ListItem> = app
        .stories
        .iter()
        .enumerate()
        .map(|(i, story)| {
            let is_read = app.is_story_read(story.id);
            story_to_list_item(story, i + 1, theme, &app.clock, is_read)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(format!("{} Stories", app.feed.label())),
        )
        .highlight_style(
            Style::default()
                .bg(theme.selection_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    frame.render_stateful_widget(list, area, &mut state);
}

fn story_to_list_item(
    story: &Story,
    rank: usize,
    theme: &ResolvedTheme,
    clock: &Arc<dyn Clock>,
    is_read: bool,
) -> ListItem<'static> {
    let style = |color| {
        let base = Style::default().fg(color);
        if is_read {
            base.add_modifier(Modifier::DIM)
        } else {
            base
        }
    };
    let title_line = Line::from(vec![
        Span::styled(format!("{:>3}. ", rank), style(theme.foreground_dim)),
        Span::styled(story.title.clone(), style(theme.story_title)),
        Span::styled(format!(" ({})", story.domain()), style(theme.story_domain)),
    ]);
    let meta_line = Line::from(vec![
        Span::styled("     ", style(theme.foreground_dim)),
        Span::styled(format!("▲ {}", story.score), style(theme.story_score)),
        Span::styled(" | ", style(theme.foreground_dim)),
        Span::styled(story.by.clone(), style(theme.story_author)),
        Span::styled(" | ", style(theme.foreground_dim)),
        Span::styled(
            format!("{} comments", story.descendants),
            style(theme.story_comments),
        ),
        Span::styled(" | ", style(theme.foreground_dim)),
        Span::styled(
            format_relative(story.time, clock.now()),
            style(theme.story_time),
        ),
    ]);
    ListItem::new(vec![title_line, meta_line])
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.show_help {
        "j/k:nav  g/G:top/bottom  H/L:feeds  o:open  y:copy  l:comments  1-6:feeds  r:refresh  t:themes  `:debug  q:quit  ?:hide"
    } else {
        "H/L:feeds  ?:help  q:quit"
    };

    StatusBar::new(&app.theme)
        .label(app.feed.label())
        .position(app.selected_index + 1, app.stories.len())
        .help(help_text)
        .flash(app.flash_text())
        .render(frame, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{TestAppBuilder, sample_stories};
    use crate::views::tests::render_to_string;

    #[test]
    fn test_stories_view_renders_list() {
        let app = TestAppBuilder::new()
            .with_stories(sample_stories())
            .selected(0)
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_stories_view_selection_highlight() {
        let app = TestAppBuilder::new()
            .with_stories(sample_stories())
            .selected(2) // Third story selected
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_stories_view_error_state() {
        let app = TestAppBuilder::new()
            .error("Failed to fetch stories: connection timeout")
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        assert!(output.contains("connection timeout"));
    }

    #[test]
    fn test_stories_view_empty_list() {
        let app = TestAppBuilder::new().build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_stories_view_with_help() {
        let app = TestAppBuilder::new()
            .with_stories(sample_stories())
            .show_help()
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        assert!(output.contains("j/k:nav"));
    }

    #[test]
    fn test_stories_view_read_stories() {
        let stories = sample_stories();
        // Mark stories 1 and 3 as read
        let read_ids = vec![stories[0].id, stories[2].id];
        let app = TestAppBuilder::new()
            .with_stories(stories)
            .read_story_ids(read_ids)
            .selected(1) // Select an unread story
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }
}
