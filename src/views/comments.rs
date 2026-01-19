use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use textwrap;
use unicode_width::UnicodeWidthStr;

use crate::api::Comment;
use crate::app::{App, View};
use crate::help::comments_help;
use crate::keys::{comments_keymap, global_keymap};
use crate::theme::ResolvedTheme;
use crate::time::{Clock, format_relative};
use crate::views::common::{render_error, render_with_timestamp};
use crate::views::html::strip_html;
use crate::views::status_bar::StatusBar;
use crate::views::tree::{
    build_empty_line_prefix, build_meta_tree_prefix, build_text_prefix, compute_tree_context,
};
use crate::widgets::{CommentList, CommentListItem, CommentListState};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let story_title = match &app.view {
        View::Comments { story_title, .. } => story_title.clone(),
        _ => String::new(),
    };

    let chunks = Layout::vertical([
        Constraint::Length(2), // Story title
        Constraint::Min(0),    // Comments
        Constraint::Length(1), // Status bar
    ])
    .split(area);

    let theme = &app.theme;
    render_header(frame, app, &story_title, chunks[0], theme);
    render_comment_list(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, title: &str, area: Rect, theme: &ResolvedTheme) {
    use super::spinner::spinner_frame;

    let mut spans = vec![Span::styled(
        title,
        Style::default()
            .fg(theme.story_title)
            .add_modifier(Modifier::BOLD),
    )];

    if app.load.should_show_spinner() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            spinner_frame(app.load.loading_start),
            theme.spinner_style(),
        ));
    }

    let title_line = Line::from(spans);
    render_with_timestamp(
        frame,
        title_line,
        app.comments_fetched_at,
        app.clock.now(),
        theme,
        area,
    );
}

fn render_comment_list(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    if let Some(err) = &app.load.error {
        render_error(frame, err, theme, area);
        return;
    }

    if app.comment_tree.is_empty() {
        let empty = Paragraph::new("No comments yet")
            .style(theme.dim_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border_style())
                    .title("Comments"),
            );
        frame.render_widget(empty, area);
        return;
    }

    let content_width = area.width.saturating_sub(4) as usize;
    let visible_indices = app.visible_comment_indices();
    let tree_context = compute_tree_context(app.comment_tree.comments(), &visible_indices);

    let items: Vec<CommentListItem> = visible_indices
        .iter()
        .enumerate()
        .map(|(vis_idx, &i)| {
            let comment = app.comment_tree.get(i).unwrap();
            let is_expanded = app.comment_tree.is_expanded(comment.id);
            let has_more = &tree_context[vis_idx];
            let lines = comment_to_lines(
                comment,
                content_width,
                is_expanded,
                theme,
                has_more,
                &app.clock,
            );
            CommentListItem::new(lines)
        })
        .collect();

    let list = CommentList::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style())
                .title(format!("Comments ({})", app.comment_tree.len())),
        )
        .highlight_style(Style::default().bg(theme.selection_bg))
        .highlight_symbol("▶ ");

    let mut state = CommentListState::new();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

fn comment_to_lines(
    comment: &Comment,
    max_width: usize,
    is_expanded: bool,
    theme: &ResolvedTheme,
    has_more_at_depth: &[bool],
    clock: &Arc<dyn Clock>,
) -> Vec<Line<'static>> {
    let color = theme.depth_color(comment.depth);
    let has_children = !comment.kids.is_empty();
    let show_children_connector = has_children && is_expanded;

    let meta_line = build_meta_line(comment, is_expanded, has_more_at_depth, theme, clock, color);
    let text_lines = build_text_lines(
        &comment.text,
        comment.depth,
        has_more_at_depth,
        show_children_connector,
        max_width,
        theme,
        color,
    );
    let separator = build_empty_line_prefix(
        comment.depth,
        has_more_at_depth,
        show_children_connector,
        color,
    );

    let mut lines = vec![meta_line];
    lines.extend(text_lines);
    lines.push(Line::from(vec![separator]));
    lines
}

fn build_meta_line(
    comment: &Comment,
    is_expanded: bool,
    has_more_at_depth: &[bool],
    theme: &ResolvedTheme,
    clock: &Arc<dyn Clock>,
    color: ratatui::style::Color,
) -> Line<'static> {
    let has_children = !comment.kids.is_empty();
    let tree_prefix = build_meta_tree_prefix(comment.depth, has_more_at_depth, color);

    let expand_indicator = if has_children {
        if is_expanded {
            Span::styled("[-] ", Style::default().fg(theme.foreground_dim))
        } else {
            Span::styled("[+] ", Style::default().fg(theme.warning))
        }
    } else {
        Span::styled("[ ] ", Style::default().fg(theme.foreground_dim))
    };

    let mut spans = vec![
        tree_prefix,
        expand_indicator,
        Span::styled(
            comment.by.clone(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", theme.dim_style()),
        Span::styled(
            format_relative(comment.time, clock.now()),
            theme.dim_style(),
        ),
    ];

    if has_children {
        spans.push(Span::styled(" · ", theme.dim_style()));
        spans.push(Span::styled(
            format!("{} replies", comment.kids.len()),
            theme.dim_style(),
        ));
    }

    if comment.is_favorited() {
        spans.push(Span::styled(
            " \u{2728}",
            Style::default().fg(theme.warning),
        ));
    }

    Line::from(spans)
}

fn build_text_lines(
    text: &str,
    depth: usize,
    has_more_at_depth: &[bool],
    show_children_connector: bool,
    max_width: usize,
    theme: &ResolvedTheme,
    color: ratatui::style::Color,
) -> Vec<Line<'static>> {
    let text = strip_html(text);
    let prefix = build_text_prefix(depth, has_more_at_depth, show_children_connector, color);
    let prefix_width = prefix.content.width();
    let available_width = max_width.saturating_sub(prefix_width).max(20);

    wrap_text(&text, available_width)
        .into_iter()
        .map(|line| {
            Line::from(vec![
                prefix.clone(),
                Span::styled(line, theme.comment_text_style()),
            ])
        })
        .collect()
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    textwrap::wrap(text, width)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect()
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    use super::spinner::spinner_frame;

    let keymap = global_keymap().extend(comments_keymap());
    let help_text = comments_help().format(&keymap, false);

    let loading_text = if app.load.loading {
        Some(format!(
            "{} Loading...",
            spinner_frame(app.load.loading_start)
        ))
    } else {
        None
    };

    let mut status_bar = StatusBar::new(&app.theme)
        .label("Comments")
        .position(app.selected_index + 1, app.comment_tree.len())
        .help(&help_text)
        .flash(app.flash_text());

    if let Some(ref text) = loading_text {
        status_bar = status_bar.loading(text);
    }

    status_bar.render(frame, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::View;
    use crate::test_utils::{CommentBuilder, TestAppBuilder, sample_comments};
    use crate::views::tests::render_to_string;

    #[test]
    fn test_comments_view_renders_thread() {
        let app = TestAppBuilder::new()
            .with_comments(sample_comments())
            .view(View::Comments {
                story_id: 1,
                story_title: "Test Story Title".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .expanded(vec![100]) // Expand first comment to show replies
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_comments_view_depth_indentation() {
        let comments = vec![
            CommentBuilder::new()
                .id(1)
                .text("Top level comment")
                .author("user1")
                .depth(0)
                .kids(vec![2])
                .build(),
            CommentBuilder::new()
                .id(2)
                .text("First reply")
                .author("user2")
                .depth(1)
                .kids(vec![3])
                .build(),
            CommentBuilder::new()
                .id(3)
                .text("Nested reply")
                .author("user3")
                .depth(2)
                .kids(vec![4])
                .build(),
            CommentBuilder::new()
                .id(4)
                .text("Deep nested")
                .author("user4")
                .depth(3)
                .kids(vec![])
                .build(),
        ];

        let app = TestAppBuilder::new()
            .with_comments(comments)
            .view(View::Comments {
                story_id: 1,
                story_title: "Deep Thread".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .expanded(vec![1, 2, 3])
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_comments_view_collapsed_state() {
        let comments = vec![
            CommentBuilder::new()
                .id(1)
                .text("Parent comment with replies")
                .author("parent")
                .depth(0)
                .kids(vec![2, 3])
                .build(),
            CommentBuilder::new()
                .id(2)
                .text("Hidden reply 1")
                .author("child1")
                .depth(1)
                .build(),
            CommentBuilder::new()
                .id(3)
                .text("Hidden reply 2")
                .author("child2")
                .depth(1)
                .build(),
        ];

        let app = TestAppBuilder::new()
            .with_comments(comments)
            .view(View::Comments {
                story_id: 1,
                story_title: "Collapsed Test".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        assert!(output.contains("[+]"));
        assert!(output.contains("2 replies"));
        assert!(!output.contains("Hidden reply"));
    }

    #[test]
    fn test_comments_view_top_level_collapsed_no_connectors() {
        let comments = vec![
            CommentBuilder::new()
                .id(1)
                .text("First top-level comment")
                .author("alice")
                .depth(0)
                .kids(vec![10])
                .build(),
            CommentBuilder::new()
                .id(2)
                .text("Second top-level comment")
                .author("bob")
                .depth(0)
                .kids(vec![20])
                .build(),
            CommentBuilder::new()
                .id(3)
                .text("Third top-level comment")
                .author("carol")
                .depth(0)
                .build(),
        ];

        let app = TestAppBuilder::new()
            .with_comments(comments)
            .view(View::Comments {
                story_id: 1,
                story_title: "Multiple Top-Level".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_comments_view_collapsed_children_show_text() {
        let comments = vec![
            CommentBuilder::new()
                .id(1)
                .text("Parent comment expanded")
                .author("parent")
                .depth(0)
                .kids(vec![2, 3])
                .build(),
            CommentBuilder::new()
                .id(2)
                .text("Child with no replies")
                .author("child_leaf")
                .depth(1)
                .build(),
            CommentBuilder::new()
                .id(3)
                .text("Child with hidden grandchildren")
                .author("child_parent")
                .depth(1)
                .kids(vec![4])
                .build(),
            CommentBuilder::new()
                .id(4)
                .text("Hidden grandchild")
                .author("grandchild")
                .depth(2)
                .build(),
        ];

        let app = TestAppBuilder::new()
            .with_comments(comments)
            .view(View::Comments {
                story_id: 1,
                story_title: "Nested Collapse Test".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .expanded(vec![1])
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_comments_view_empty() {
        let app = TestAppBuilder::new()
            .view(View::Comments {
                story_id: 1,
                story_title: "Empty Story".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        assert!(output.contains("No comments yet"));
    }

    #[test]
    fn test_comments_text_wrapping_consistent_when_expanded() {
        // Regression test: text should wrap at the same width whether collapsed or expanded.
        // The tree glyph (│) and spaces have the same display width, so wrapping shouldn't change.
        let long_text = "This is a long comment that needs to wrap. It should wrap at exactly the same position whether the comment is collapsed or expanded because the tree glyphs and spaces have identical display widths.";
        let comments = vec![
            CommentBuilder::new()
                .id(1)
                .text(long_text)
                .author("alice")
                .depth(0)
                .kids(vec![2])
                .build(),
            CommentBuilder::new()
                .id(2)
                .text("Child comment")
                .author("bob")
                .depth(1)
                .build(),
        ];

        // Render collapsed
        let collapsed_app = TestAppBuilder::new()
            .with_comments(comments.clone())
            .view(View::Comments {
                story_id: 1,
                story_title: "Wrap Test".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .build();

        let collapsed_output = render_to_string(80, 16, |frame| {
            render(frame, &collapsed_app, frame.area());
        });

        // Render expanded
        let expanded_app = TestAppBuilder::new()
            .with_comments(comments)
            .view(View::Comments {
                story_id: 1,
                story_title: "Wrap Test".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .expanded(vec![1])
            .build();

        let expanded_output = render_to_string(80, 16, |frame| {
            render(frame, &expanded_app, frame.area());
        });

        insta::assert_snapshot!("text_wrap_collapsed", collapsed_output);
        insta::assert_snapshot!("text_wrap_expanded", expanded_output);
    }

    #[test]
    fn test_comments_view_loading() {
        let app = TestAppBuilder::new()
            .view(View::Comments {
                story_id: 1,
                story_title: "Loading Story".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .loading()
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        assert!(output.contains("Loading"));
    }

    #[test]
    fn test_comments_view_error() {
        let app = TestAppBuilder::new()
            .view(View::Comments {
                story_id: 1,
                story_title: "Error Story".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .error("Network error: connection refused")
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        assert!(output.contains("connection refused"));
    }

    #[test]
    fn test_comments_view_fills_viewport_with_partial_comments() {
        let comments = vec![
            CommentBuilder::new()
                .id(1)
                .text("Short comment")
                .author("user1")
                .depth(0)
                .build(),
            CommentBuilder::new()
                .id(2)
                .text("Another short comment")
                .author("user2")
                .depth(0)
                .build(),
            CommentBuilder::new()
                .id(3)
                .text("This is a much longer comment that will wrap to multiple lines and should be partially visible at the bottom of the viewport instead of being skipped entirely causing blank space")
                .author("user3")
                .depth(0)
                .build(),
        ];

        let app = TestAppBuilder::new()
            .with_comments(comments)
            .view(View::Comments {
                story_id: 1,
                story_title: "Partial Render Test".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .selected(0)
            .build();

        // Height chosen to cause partial rendering of last comment
        let output = render_to_string(80, 15, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_comments_view_with_timestamp() {
        use crate::test_utils::TEST_NOW;

        // 3 minutes ago (fresh)
        let fetched_at = (TEST_NOW - 180) as u64;

        let app = TestAppBuilder::new()
            .with_comments(sample_comments())
            .view(View::Comments {
                story_id: 1,
                story_title: "Test Story with Timestamp".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .comments_fetched_at(fetched_at)
            .selected(0)
            .build();

        let output = render_to_string(80, 24, |frame| {
            render(frame, &app, frame.area());
        });

        insta::assert_snapshot!(output);
    }
}
