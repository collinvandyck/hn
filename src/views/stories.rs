use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::api::{Feed, Story};
use crate::app::{App, StorySort};
use crate::help::stories_help;
use crate::keys::{global_keymap, stories_keymap};

use crate::theme::ResolvedTheme;
use crate::time::{Clock, format_relative};
use crate::views::common::{render_error, render_with_timestamp};
use crate::views::status_bar::StatusBar;

#[derive(Debug, Clone, Copy, Default)]
struct ColumnWidths {
    score: usize,
    author: usize,
    comments: usize,
}

impl ColumnWidths {
    const MIN_SCORE: usize = 3;
    const MIN_AUTHOR: usize = 4;
    const MIN_COMMENTS: usize = 3;
    const MAX_AUTHOR: usize = 15;

    fn from_stories(stories: &[Story]) -> Self {
        Self {
            score: stories
                .iter()
                .map(|s| digit_count(s.score))
                .max()
                .unwrap_or(1)
                .max(Self::MIN_SCORE),
            author: stories
                .iter()
                .map(|s| s.by.len().min(Self::MAX_AUTHOR))
                .max()
                .unwrap_or(1)
                .max(Self::MIN_AUTHOR),
            comments: stories
                .iter()
                .map(|s| digit_count(s.descendants))
                .max()
                .unwrap_or(1)
                .max(Self::MIN_COMMENTS),
        }
    }
}

const fn digit_count(mut n: u32) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    while n > 0 {
        count += 1;
        n /= 10;
    }
    count
}

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

    // Show sort indicator when not using default sort
    if app.story_sort != StorySort::Position {
        spans.push(Span::styled(
            format!("[sort: {}]", app.story_sort.label()),
            theme.active_tab_style(),
        ));
    }

    let tabs_line = Line::from(spans);
    render_with_timestamp(
        frame,
        tabs_line,
        app.stories_fetched_at,
        app.clock.now(),
        theme,
        area,
    );
}

fn render_story_list(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    if let Some(err) = &app.load.error {
        render_error(frame, err, theme, area);
        return;
    }

    let widths = ColumnWidths::from_stories(&app.stories);

    let items: Vec<ListItem> = app
        .stories
        .iter()
        .enumerate()
        .map(|(i, story)| {
            let is_selected = i == app.selected_index;
            story_to_list_item(
                story,
                theme,
                &app.clock,
                app.feed,
                widths,
                is_selected,
                app.story_sort,
            )
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(theme.border_style()),
        )
        .highlight_style(theme.selection_style())
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    frame.render_stateful_widget(list, area, &mut state);
}

fn story_to_list_item(
    story: &Story,
    theme: &ResolvedTheme,
    clock: &Arc<dyn Clock>,
    feed: Feed,
    widths: ColumnWidths,
    is_selected: bool,
    sort: StorySort,
) -> ListItem<'static> {
    use ratatui::style::Modifier;

    let theme = if story.is_read() && feed != Feed::Favorites && !is_selected {
        theme.dimmed()
    } else {
        theme.clone()
    };
    let mut title_spans = vec![Span::styled(story.title.clone(), theme.story_title_style())];
    if story.is_favorited() {
        title_spans.push(Span::styled(
            " \u{2728}",
            ratatui::style::Style::default().fg(theme.warning),
        ));
    }
    title_spans.push(Span::styled(
        format!(" ({})", story.domain()),
        theme.story_domain_style(),
    ));
    let title_line = Line::from(title_spans);
    let author_display = if story.by.len() > ColumnWidths::MAX_AUTHOR {
        format!("{}...", &story.by[..ColumnWidths::MAX_AUTHOR - 3])
    } else {
        story.by.clone()
    };
    // Apply bold to the sorted field
    let score_style = if sort == StorySort::ScoreDesc {
        theme.story_score_style().add_modifier(Modifier::BOLD)
    } else {
        theme.story_score_style()
    };
    let comments_style = if sort == StorySort::CommentsDesc {
        theme.story_comments_style().add_modifier(Modifier::BOLD)
    } else {
        theme.story_comments_style()
    };
    let time_style = if sort == StorySort::TimeDesc {
        theme.story_time_style().add_modifier(Modifier::BOLD)
    } else {
        theme.story_time_style()
    };
    let meta_line = Line::from(vec![
        Span::styled(
            format!("▲ {:>width$}", story.score, width = widths.score),
            score_style,
        ),
        Span::styled(" | ", theme.dim_style()),
        Span::styled(
            format!("{:<width$}", author_display, width = widths.author),
            theme.story_author_style(),
        ),
        Span::styled(" | ", theme.dim_style()),
        Span::styled(
            format!(
                "{:>width$} comments",
                story.descendants,
                width = widths.comments
            ),
            comments_style,
        ),
        Span::styled(" | ", theme.dim_style()),
        Span::styled(format_relative(story.time, clock.now()), time_style),
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

    #[test]
    fn test_stories_view_with_fresh_timestamp() {
        use crate::test_utils::TEST_NOW;

        // 2 minutes ago (fresh, < 5 minutes)
        let fetched_at = (TEST_NOW - 120) as u64;

        let app = TestAppBuilder::new()
            .with_stories(sample_stories())
            .selected(0)
            .stories_fetched_at(fetched_at)
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_stories_view_with_stale_timestamp() {
        use crate::test_utils::TEST_NOW;

        // 10 minutes ago (stale, >= 5 minutes)
        let fetched_at = (TEST_NOW - 600) as u64;

        let app = TestAppBuilder::new()
            .with_stories(sample_stories())
            .selected(0)
            .stories_fetched_at(fetched_at)
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_stories_view_aligned_columns() {
        use crate::test_utils::StoryBuilder;

        // Stories with varying field widths to verify alignment
        let stories = vec![
            StoryBuilder::new()
                .id(1)
                .title("Small numbers")
                .url("https://example.com/1")
                .score(1)
                .author("a")
                .comments(0)
                .build(),
            StoryBuilder::new()
                .id(2)
                .title("Large numbers")
                .url("https://example.com/2")
                .score(9999)
                .author("verylongname")
                .comments(12345)
                .build(),
            StoryBuilder::new()
                .id(3)
                .title("Medium numbers")
                .url("https://example.com/3")
                .score(100)
                .author("pg")
                .comments(50)
                .build(),
        ];

        let app = TestAppBuilder::new()
            .with_stories(stories)
            .selected(0)
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }
}
