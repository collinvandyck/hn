//! Test data builders for view testing.

use std::collections::HashSet;
use std::time::Instant;

use crate::api::{Comment, Feed, HnClient, Story};
use crate::app::{App, View};
use crate::theme::{ResolvedTheme, ThemeVariant, default_for_variant};

#[allow(dead_code)]
pub struct StoryBuilder {
    id: u64,
    title: String,
    url: Option<String>,
    score: u32,
    by: String,
    time: u64,
    descendants: u32,
    kids: Vec<u64>,
}

impl Default for StoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl StoryBuilder {
    pub fn new() -> Self {
        Self {
            id: 1,
            title: "Test Story".to_string(),
            url: Some("https://example.com".to_string()),
            score: 100,
            by: "testuser".to_string(),
            time: 1700000000,
            descendants: 10,
            kids: vec![],
        }
    }

    pub fn id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn no_url(mut self) -> Self {
        self.url = None;
        self
    }

    pub fn score(mut self, score: u32) -> Self {
        self.score = score;
        self
    }

    pub fn author(mut self, author: &str) -> Self {
        self.by = author.to_string();
        self
    }

    pub fn comments(mut self, count: u32) -> Self {
        self.descendants = count;
        self
    }

    pub fn time(mut self, time: u64) -> Self {
        self.time = time;
        self
    }

    pub fn kids(mut self, kids: Vec<u64>) -> Self {
        self.kids = kids;
        self
    }

    pub fn build(self) -> Story {
        Story {
            id: self.id,
            title: self.title,
            url: self.url,
            score: self.score,
            by: self.by,
            time: self.time,
            descendants: self.descendants,
            kids: self.kids,
        }
    }
}

pub struct CommentBuilder {
    id: u64,
    text: String,
    by: String,
    time: u64,
    depth: usize,
    kids: Vec<u64>,
}

impl Default for CommentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CommentBuilder {
    pub fn new() -> Self {
        Self {
            id: 1,
            text: "Test comment".to_string(),
            by: "commenter".to_string(),
            time: 1700000000,
            depth: 0,
            kids: vec![],
        }
    }

    pub fn id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }

    pub fn author(mut self, author: &str) -> Self {
        self.by = author.to_string();
        self
    }

    pub fn time(mut self, time: u64) -> Self {
        self.time = time;
        self
    }

    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    pub fn kids(mut self, kids: Vec<u64>) -> Self {
        self.kids = kids;
        self
    }

    pub fn build(self) -> Comment {
        Comment {
            id: self.id,
            text: self.text,
            by: self.by,
            time: self.time,
            depth: self.depth,
            kids: self.kids,
        }
    }
}

#[allow(dead_code)]
pub struct TestAppBuilder {
    view: View,
    feed: Feed,
    stories: Vec<Story>,
    comments: Vec<Comment>,
    expanded_comments: HashSet<u64>,
    selected_index: usize,
    loading: bool,
    loading_start: Option<Instant>,
    error: Option<String>,
    show_help: bool,
    scroll_offset: usize,
    theme: ResolvedTheme,
}

impl Default for TestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl TestAppBuilder {
    pub fn new() -> Self {
        Self {
            view: View::Stories,
            feed: Feed::Top,
            stories: Vec::new(),
            comments: Vec::new(),
            expanded_comments: HashSet::new(),
            selected_index: 0,
            loading: false,
            loading_start: None,
            error: None,
            show_help: false,
            scroll_offset: 0,
            theme: default_for_variant(ThemeVariant::Dark),
        }
    }

    pub fn view(mut self, view: View) -> Self {
        self.view = view;
        self
    }

    pub fn feed(mut self, feed: Feed) -> Self {
        self.feed = feed;
        self
    }

    pub fn with_stories(mut self, stories: Vec<Story>) -> Self {
        self.stories = stories;
        self
    }

    pub fn with_comments(mut self, comments: Vec<Comment>) -> Self {
        self.comments = comments;
        self
    }

    pub fn expanded(mut self, ids: Vec<u64>) -> Self {
        self.expanded_comments = ids.into_iter().collect();
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    pub fn loading(mut self) -> Self {
        self.loading = true;
        self.loading_start = Some(Instant::now());
        self
    }

    pub fn error(mut self, msg: &str) -> Self {
        self.error = Some(msg.to_string());
        self
    }

    pub fn show_help(mut self) -> Self {
        self.show_help = true;
        self
    }

    pub fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn theme(mut self, theme: ResolvedTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn build(self) -> App {
        App {
            view: self.view,
            feed: self.feed,
            stories: self.stories,
            comments: self.comments,
            expanded_comments: self.expanded_comments,
            selected_index: self.selected_index,
            loading: self.loading,
            loading_start: self.loading_start,
            error: self.error,
            should_quit: false,
            show_help: self.show_help,
            client: HnClient::new(),
            scroll_offset: self.scroll_offset,
            theme: self.theme,
        }
    }
}

pub fn sample_stories() -> Vec<Story> {
    vec![
        StoryBuilder::new()
            .id(1)
            .title("Show HN: I built a terminal UI for Hacker News")
            .url("https://github.com/user/lima-hn")
            .score(142)
            .author("dang")
            .comments(47)
            .time(1700000000)
            .build(),
        StoryBuilder::new()
            .id(2)
            .title("Why Rust is the Future of Systems Programming")
            .url("https://example.com/rust-future")
            .score(89)
            .author("pg")
            .comments(23)
            .time(1699990000)
            .build(),
        StoryBuilder::new()
            .id(3)
            .title("Ask HN: What are you working on?")
            .no_url()
            .score(56)
            .author("sama")
            .comments(128)
            .time(1699980000)
            .build(),
        StoryBuilder::new()
            .id(4)
            .title("The unreasonable effectiveness of simple HTML")
            .url("https://blog.example.com/simple-html")
            .score(234)
            .author("tptacek")
            .comments(89)
            .time(1699970000)
            .build(),
        StoryBuilder::new()
            .id(5)
            .title("A Deep Dive into Linux Kernel Networking")
            .url("https://lwn.net/kernel-networking")
            .score(167)
            .author("patio11")
            .comments(34)
            .time(1699960000)
            .build(),
    ]
}

pub fn sample_comments() -> Vec<Comment> {
    vec![
        CommentBuilder::new()
            .id(100)
            .text("This is a great project! I love the vim keybindings.")
            .author("commenter1")
            .depth(0)
            .kids(vec![101, 103])
            .time(1700000000)
            .build(),
        CommentBuilder::new()
            .id(101)
            .text("Agreed, the vim bindings are really nice. Would love to see more themes.")
            .author("commenter2")
            .depth(1)
            .kids(vec![102])
            .time(1700001000)
            .build(),
        CommentBuilder::new()
            .id(102)
            .text("Themes are already supported! Check the --theme flag.")
            .author("author")
            .depth(2)
            .kids(vec![])
            .time(1700002000)
            .build(),
        CommentBuilder::new()
            .id(103)
            .text("Does this work on Windows?")
            .author("windowsuser")
            .depth(1)
            .kids(vec![])
            .time(1700003000)
            .build(),
        CommentBuilder::new()
            .id(104)
            .text("Nice work! Any plans for search functionality?")
            .author("searcher")
            .depth(0)
            .kids(vec![])
            .time(1700004000)
            .build(),
    ]
}
