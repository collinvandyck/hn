use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use textwrap;

use crate::api::Comment;
use crate::app::{App, View};
use crate::theme::ResolvedTheme;
use crate::time::{Clock, format_relative};
use crate::views::html::strip_html;
use crate::views::status_bar::StatusBar;
use crate::views::tree::{
    build_empty_line_prefix, build_meta_tree_prefix, build_text_prefix, compute_tree_context,
};

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
    render_header(frame, &story_title, chunks[0], theme);
    render_comment_list(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, title: &str, area: Rect, theme: &ResolvedTheme) {
    let header = Paragraph::new(title).style(
        Style::default()
            .fg(theme.story_title)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header, area);
}

fn render_comment_list(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    if app.loading {
        let loading = Paragraph::new("Loading comments...")
            .style(Style::default().fg(theme.warning))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title("Comments"),
            );
        frame.render_widget(loading, area);
        return;
    }

    if let Some(err) = &app.error {
        let error = Paragraph::new(err.as_str())
            .style(Style::default().fg(theme.error))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title("Error"),
            );
        frame.render_widget(error, area);
        return;
    }

    if app.comments.is_empty() {
        let empty = Paragraph::new("No comments yet")
            .style(Style::default().fg(theme.foreground_dim))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title("Comments"),
            );
        frame.render_widget(empty, area);
        return;
    }

    let content_width = area.width.saturating_sub(4) as usize;
    let visible_indices = app.visible_comment_indices();
    let tree_context = compute_tree_context(&app.comments, &visible_indices);
    let items: Vec<ListItem> = visible_indices
        .iter()
        .enumerate()
        .map(|(vis_idx, &i)| {
            let comment = &app.comments[i];
            let is_expanded = app.expanded_comments.contains(&comment.id);
            let has_more = &tree_context[vis_idx];
            comment_to_list_item(
                comment,
                content_width,
                is_expanded,
                theme,
                has_more,
                &app.clock,
            )
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(format!("Comments ({})", app.comments.len())),
        )
        .highlight_style(Style::default().bg(theme.selection_bg))
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    let visible_count = visible_indices.len();
    let visible_items = (area.height.saturating_sub(2) / 4).max(1) as usize;
    let half = visible_items / 2;
    let max_offset = visible_count.saturating_sub(visible_items);
    let offset = app.selected_index.saturating_sub(half).min(max_offset);
    *state.offset_mut() = offset;

    frame.render_stateful_widget(list, area, &mut state);
}

fn comment_to_list_item(
    comment: &Comment,
    max_width: usize,
    is_expanded: bool,
    theme: &ResolvedTheme,
    has_more_at_depth: &[bool],
    clock: &Arc<dyn Clock>,
) -> ListItem<'static> {
    let color = theme.depth_color(comment.depth);
    let depth = comment.depth;
    let has_children = !comment.kids.is_empty();

    let depth_marker = build_meta_tree_prefix(depth, has_more_at_depth, color);

    let expand_indicator = if has_children {
        if is_expanded {
            Span::styled("[-] ", Style::default().fg(theme.foreground_dim))
        } else {
            Span::styled("[+] ", Style::default().fg(theme.warning))
        }
    } else {
        Span::styled("[ ] ", Style::default().fg(theme.foreground_dim))
    };

    let child_info = if has_children {
        vec![
            Span::styled(" · ", Style::default().fg(theme.foreground_dim)),
            Span::styled(
                format!("{} replies", comment.kids.len()),
                Style::default().fg(theme.foreground_dim),
            ),
        ]
    } else {
        vec![]
    };

    let mut meta_spans = vec![
        depth_marker,
        expand_indicator,
        Span::styled(
            comment.by.clone(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(theme.foreground_dim)),
        Span::styled(
            format_relative(comment.time, clock.now()),
            Style::default().fg(theme.foreground_dim),
        ),
    ];
    meta_spans.extend(child_info);
    let meta_line = Line::from(meta_spans);

    let text = strip_html(&comment.text);
    // Only show children connector if children are visible (expanded)
    let show_children_connector = has_children && is_expanded;
    let text_prefix = build_text_prefix(depth, has_more_at_depth, show_children_connector, color);
    let prefix_width = text_prefix.content.len();
    let available_width = max_width.saturating_sub(prefix_width).max(20);
    let wrapped_lines = wrap_text(&text, available_width);

    let mut lines = vec![meta_line];

    for wrapped_line in wrapped_lines {
        lines.push(Line::from(vec![
            text_prefix.clone(),
            Span::styled(wrapped_line, Style::default().fg(theme.comment_text)),
        ]));
    }

    let empty_prefix =
        build_empty_line_prefix(depth, has_more_at_depth, show_children_connector, color);
    lines.push(Line::from(vec![empty_prefix]));

    ListItem::new(lines)
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

    let help_text = if app.show_help {
        "j/k:nav  l/h:expand  L/H:subtree  +/-:thread  o:story  c:link  Esc:back  r:refresh  `:debug  q:quit  ?:hide"
    } else {
        "l/h:expand  L/H:subtree  +/-:thread  Esc:back  ?:help"
    };

    let loading_text = if app.loading {
        Some(format!("{} Loading...", spinner_frame(app.loading_start)))
    } else {
        None
    };

    let mut status_bar = StatusBar::new(&app.theme)
        .label("Comments")
        .position(app.selected_index + 1, app.comments.len())
        .help(help_text);

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
}
