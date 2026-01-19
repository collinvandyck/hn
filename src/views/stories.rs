use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::api::{Feed, Story};
use crate::app::App;
use crate::help::stories_help;
use crate::keys::{global_keymap, stories_keymap};

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
                theme.active_tab_style()
            } else {
                theme.dim_style()
            };
            vec![
                Span::styled(format!("[{}]", i + 1), theme.dim_style()),
                Span::styled(feed.label(), style),
                Span::raw("  "),
            ]
        })
        .collect();

    if app.load.should_show_spinner() {
        spans.push(Span::styled(
            spinner_frame(app.load.loading_start),
            theme.spinner_style(),
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
        .map(|(i, story)| story_to_list_item(story, i + 1, theme, &app.clock))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style())
                .title(format!("{} Stories", app.feed.label())),
        )
        .highlight_style(theme.selection_style())
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
) -> ListItem<'static> {
    let theme = if story.is_read() {
        theme.dimmed()
    } else {
        theme.clone()
    };
    let title_line = Line::from(vec![
        Span::styled(format!("{:>3}. ", rank), theme.dim_style()),
        Span::styled(story.title.clone(), theme.story_title_style()),
        Span::styled(format!(" ({})", story.domain()), theme.story_domain_style()),
    ]);
    let meta_line = Line::from(vec![
        Span::styled("     ", theme.dim_style()),
        Span::styled(format!("▲ {}", story.score), theme.story_score_style()),
        Span::styled(" | ", theme.dim_style()),
        Span::styled(story.by.clone(), theme.story_author_style()),
        Span::styled(" | ", theme.dim_style()),
        Span::styled(
            format!("{} comments", story.descendants),
            theme.story_comments_style(),
        ),
        Span::styled(" | ", theme.dim_style()),
        Span::styled(
            format_relative(story.time, clock.now()),
            theme.story_time_style(),
        ),
    ]);
    ListItem::new(vec![title_line, meta_line])
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let keymap = global_keymap().extend(stories_keymap());
    let help_text = stories_help().format(&keymap, false);
    StatusBar::new(&app.theme)
        .label(app.feed.label())
        .position(app.selected_index + 1, app.stories.len())
        .help(&help_text)
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
    fn test_stories_view_status_bar_shows_help_hint() {
        let app = TestAppBuilder::new().with_stories(sample_stories()).build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        // Status bar shows compact help with ?:help
        assert!(output.contains("?:help"));
    }

    #[test]
    fn test_stories_view_read_stories() {
        use crate::test_utils::StoryBuilder;

        // Create stories with some marked as read
        let stories = vec![
            StoryBuilder::new()
                .id(1)
                .title("Read Story One")
                .url("https://example.com/1")
                .score(100)
                .read() // Mark as read
                .build(),
            StoryBuilder::new()
                .id(2)
                .title("Unread Story Two")
                .url("https://example.com/2")
                .score(200)
                .build(),
            StoryBuilder::new()
                .id(3)
                .title("Read Story Three")
                .url("https://example.com/3")
                .score(300)
                .read() // Mark as read
                .build(),
        ];

        let app = TestAppBuilder::new()
            .with_stories(stories)
            .selected(1) // Select an unread story
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }
}
