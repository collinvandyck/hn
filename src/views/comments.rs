use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use textwrap;

use crate::api::Comment;
use crate::app::{App, View};
use crate::theme::ResolvedTheme;

/// Render the comments view
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
    let header = Paragraph::new(title)
        .style(
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
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)).title("Comments"));
        frame.render_widget(loading, area);
        return;
    }

    if let Some(err) = &app.error {
        let error = Paragraph::new(err.as_str())
            .style(Style::default().fg(theme.error))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)).title("Error"));
        frame.render_widget(error, area);
        return;
    }

    if app.comments.is_empty() {
        let empty = Paragraph::new("No comments yet")
            .style(Style::default().fg(theme.foreground_dim))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)).title("Comments"));
        frame.render_widget(empty, area);
        return;
    }

    // Calculate available width for text (account for borders and indent)
    let content_width = area.width.saturating_sub(4) as usize; // 2 for borders, 2 for padding

    // Get only visible comments based on expansion state
    let visible_indices = app.visible_comment_indices();
    let items: Vec<ListItem> = visible_indices
        .iter()
        .map(|&i| {
            let comment = &app.comments[i];
            let is_expanded = app.expanded_comments.contains(&comment.id);
            comment_to_list_item(comment, content_width, is_expanded, theme)
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

    // Center the selected item (scrolloff behavior)
    // Estimate ~4 lines per comment on average for visible item calculation
    let visible_count = visible_indices.len();
    let visible_items = (area.height.saturating_sub(2) / 4).max(1) as usize;
    let half = visible_items / 2;
    let max_offset = visible_count.saturating_sub(visible_items);
    let offset = app.selected_index.saturating_sub(half).min(max_offset);
    *state.offset_mut() = offset;

    frame.render_stateful_widget(list, area, &mut state);
}

fn comment_to_list_item(comment: &Comment, max_width: usize, is_expanded: bool, theme: &ResolvedTheme) -> ListItem<'static> {
    let color = theme.depth_color(comment.depth);
    let indent_width = comment.depth * 2;
    let indent = " ".repeat(indent_width);
    let has_children = !comment.kids.is_empty();

    // Depth marker with color
    let depth_marker = if comment.depth > 0 {
        Span::styled(
            format!("{}├─ ", &indent[..indent_width.saturating_sub(3)]),
            Style::default().fg(color),
        )
    } else {
        Span::raw("")
    };

    // Collapse/expand indicator (fixed width for alignment)
    let expand_indicator = if has_children {
        if is_expanded {
            Span::styled("[-] ", Style::default().fg(theme.foreground_dim))
        } else {
            Span::styled("[+] ", Style::default().fg(theme.warning))
        }
    } else {
        Span::styled("[ ] ", Style::default().fg(theme.foreground_dim))
    };

    // Child count for RHS (only show if has children)
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

    // Author line with colored marker
    let mut meta_spans = vec![
        depth_marker,
        expand_indicator,
        Span::styled(
            comment.by.clone(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(theme.foreground_dim)),
        Span::styled(format_time(comment.time), Style::default().fg(theme.foreground_dim)),
    ];
    meta_spans.extend(child_info);
    let meta_line = Line::from(meta_spans);

    // If collapsed with children, show only meta line
    if has_children && !is_expanded {
        return ListItem::new(vec![meta_line, Line::from("")]);
    }

    // Process and wrap comment text
    let text = strip_html(&comment.text);
    let text_indent = indent.clone() + "      "; // Extra indent for text body (accounts for expand indicator)
    let available_width = max_width.saturating_sub(text_indent.len()).max(20);

    // Wrap text to fit available width
    let wrapped_lines = wrap_text(&text, available_width);

    // Build text lines with proper indentation
    let mut lines = vec![meta_line];

    for wrapped_line in wrapped_lines {
        lines.push(Line::from(vec![
            Span::styled(text_indent.clone(), Style::default().fg(theme.foreground_dim)),
            Span::styled(wrapped_line, Style::default().fg(theme.comment_text)),
        ]));
    }

    // Add empty line for spacing between comments
    lines.push(Line::from(""));

    ListItem::new(lines)
}

/// Wrap text to specified width, preserving words
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

    let theme = &app.theme;
    let help_text = if app.show_help {
        "j/k:nav  l:expand  h:collapse  o:story  c:link  Esc:back  r:refresh  q:quit  ?:hide"
    } else {
        "l/h:expand/collapse  Esc:back  ?:help"
    };

    let mut spans = vec![
        Span::styled(
            " Comments ",
            Style::default().bg(theme.status_bar_bg).fg(theme.status_bar_fg),
        ),
        Span::raw(" "),
    ];

    // Show spinner when loading
    if app.loading {
        spans.push(Span::styled(
            format!("{} Loading... ", spinner_frame(app.loading_start)),
            Style::default().fg(theme.spinner),
        ));
        spans.push(Span::raw("| "));
    }

    spans.extend([
        Span::styled(
            format!("{}/{}", app.selected_index + 1, app.comments.len()),
            Style::default().fg(theme.foreground_dim),
        ),
        Span::raw(" | "),
        Span::styled(help_text, Style::default().fg(theme.foreground_dim)),
    ]);

    let status = Line::from(spans);
    frame.render_widget(Paragraph::new(status), area);
}

fn format_time(timestamp: u64) -> String {
    use chrono::{TimeZone, Utc};

    let now = Utc::now();
    let then = Utc.timestamp_opt(timestamp as i64, 0).single();

    match then {
        Some(t) => {
            let diff = now.signed_duration_since(t);
            if diff.num_hours() < 1 {
                format!("{}m ago", diff.num_minutes())
            } else if diff.num_hours() < 24 {
                format!("{}h ago", diff.num_hours())
            } else {
                format!("{}d ago", diff.num_days())
            }
        }
        None => "?".to_string(),
    }
}

fn strip_html(html: &str) -> String {
    // Convert HTML to readable text
    html.replace("<p>", "\n\n")
        .replace("</p>", "")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<i>", "_")
        .replace("</i>", "_")
        .replace("<b>", "*")
        .replace("</b>", "*")
        .replace("<code>", "`")
        .replace("</code>", "`")
        .replace("<pre>", "\n```\n")
        .replace("</pre>", "\n```\n")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&#x2F;", "/")
        // Strip links but keep text
        .split("<a ")
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.to_string()
            } else {
                // Find the link text between > and </a>
                if let Some(start) = part.find('>') {
                    if let Some(end) = part.find("</a>") {
                        let link_text = &part[start + 1..end];
                        let rest = &part[end + 4..];
                        return format!("{}{}", link_text, rest);
                    }
                }
                part.to_string()
            }
        })
        .collect::<String>()
        // Clean up whitespace
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::View;
    use crate::test_utils::{sample_comments, CommentBuilder, TestAppBuilder};
    use crate::views::tests::render_to_string;

    #[test]
    fn test_strip_html_basic_tags() {
        assert_eq!(strip_html("<p>Hello</p><p>World</p>"), "Hello World");
        assert_eq!(strip_html("Line1<br>Line2"), "Line1 Line2");
    }

    #[test]
    fn test_strip_html_formatting() {
        assert_eq!(strip_html("<i>italic</i>"), "_italic_");
        assert_eq!(strip_html("<b>bold</b>"), "*bold*");
        assert_eq!(strip_html("<code>code</code>"), "`code`");
    }

    #[test]
    fn test_strip_html_entities() {
        assert_eq!(strip_html("&lt;tag&gt;"), "<tag>");
        assert_eq!(strip_html("&amp;&quot;&#x27;"), "&\"'");
        assert_eq!(strip_html("path&#x2F;to&#x2F;file"), "path/to/file");
    }

    #[test]
    fn test_strip_html_links() {
        let html = r#"Check <a href="https://example.com">this link</a> out"#;
        assert_eq!(strip_html(html), "Check this link out");
    }

    #[test]
    fn test_strip_html_collapses_whitespace() {
        assert_eq!(strip_html("  too   many    spaces  "), "too many spaces");
        assert_eq!(strip_html("<p>  \n\n  </p>text"), "text");
    }

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
        // Create a deep comment tree to test indentation
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
            .expanded(vec![1, 2, 3]) // Expand all to show full thread
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

        // Don't expand comment 1, so replies should be hidden
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

        // Should show [+] indicator and "2 replies" but not the reply text
        assert!(output.contains("[+]"));
        assert!(output.contains("2 replies"));
        // The reply text should not be visible
        assert!(!output.contains("Hidden reply"));
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
