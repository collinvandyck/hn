//! Test data builders for view testing.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::api::{Comment, Feed, HnClient, Story};
use crate::app::{App, DebugState, LoadState, View};
use crate::comment_tree::CommentTree;
use crate::storage::{Storage, StorageLocation};
use crate::theme::{ResolvedTheme, ThemeVariant, default_for_variant};
use crate::time::{Clock, fixed_clock};

/// Fixed timestamp for deterministic tests: 2023-11-15 00:00:00 UTC
/// This is 1 day after the base timestamp (1700000000) used in sample data,
/// so stories/comments will show as "1d ago".
pub const TEST_NOW: i64 = 1700092800;

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
    read_at: Option<u64>,
    favorited_at: Option<u64>,
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
            read_at: None,
            favorited_at: None,
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

    pub fn read(mut self) -> Self {
        self.read_at = Some(1700000000);
        self
    }

    pub fn favorited(mut self) -> Self {
        self.favorited_at = Some(1700000000);
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
            read_at: self.read_at,
            favorited_at: self.favorited_at,
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
    favorited_at: Option<u64>,
}

impl Default for CommentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl CommentBuilder {
    pub fn new() -> Self {
        Self {
            id: 1,
            text: "Test comment".to_string(),
            by: "commenter".to_string(),
            time: 1700000000,
            depth: 0,
            kids: vec![],
            favorited_at: None,
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

    pub fn favorited(mut self) -> Self {
        self.favorited_at = Some(1700000000);
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
            favorited_at: self.favorited_at,
        }
    }
}

#[allow(dead_code)]
pub struct TestAppBuilder {
    view: View,
    feed: Feed,
    stories: Vec<Story>,
    comments: Vec<Comment>,
    expanded_ids: Vec<u64>,
    selected_index: usize,
    loading: bool,
    loading_start: Option<Instant>,
    loading_more: bool,
    current_page: usize,
    has_more: bool,
    error: Option<String>,
    help_overlay: bool,
    scroll_offset: usize,
    theme: ResolvedTheme,
    clock: Arc<dyn Clock>,
    viewport_height: Option<u16>,
    config_dir: Option<PathBuf>,
    stories_fetched_at: Option<u64>,
    comments_fetched_at: Option<u64>,
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
            expanded_ids: Vec::new(),
            selected_index: 0,
            loading: false,
            loading_start: None,
            loading_more: false,
            current_page: 0,
            has_more: true,
            error: None,
            help_overlay: false,
            scroll_offset: 0,
            theme: default_for_variant(ThemeVariant::Dark),
            clock: fixed_clock(TEST_NOW),
            viewport_height: None,
            config_dir: None,
            stories_fetched_at: None,
            comments_fetched_at: None,
        }
    }

    pub fn config_dir(mut self, path: PathBuf) -> Self {
        self.config_dir = Some(path);
        self
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
        self.expanded_ids = ids;
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

    pub fn help_overlay(mut self) -> Self {
        self.help_overlay = true;
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

    pub fn viewport_height(mut self, height: u16) -> Self {
        self.viewport_height = Some(height);
        self
    }

    pub fn loading_more(mut self, loading: bool) -> Self {
        self.loading_more = loading;
        self
    }

    pub fn has_more(mut self, has_more: bool) -> Self {
        self.has_more = has_more;
        self
    }

    pub fn stories_fetched_at(mut self, ts: u64) -> Self {
        self.stories_fetched_at = Some(ts);
        self
    }

    pub fn comments_fetched_at(mut self, ts: u64) -> Self {
        self.comments_fetched_at = Some(ts);
        self
    }

    pub fn build(self) -> App {
        let (result_tx, result_rx) = mpsc::channel(10);

        // Build comment tree with expansion state
        let mut comment_tree = CommentTree::new();
        comment_tree.set(self.comments);
        for id in self.expanded_ids {
            comment_tree.expand(id);
        }

        // Build load state
        let load = LoadState {
            loading: self.loading,
            loading_start: self.loading_start,
            loading_more: self.loading_more,
            current_page: self.current_page,
            has_more: self.has_more,
            error: self.error,
        };

        App {
            view: self.view,
            feed: self.feed,
            stories: self.stories,
            comment_tree,
            selected_index: self.selected_index,
            load,
            should_quit: false,
            help_overlay: self.help_overlay,
            client: HnClient::new(Storage::open(StorageLocation::InMemory).unwrap()),
            scroll_offset: self.scroll_offset,
            theme: self.theme,
            clock: self.clock,
            result_tx,
            result_rx,
            generation: 0,
            debug: DebugState::new(),
            viewport_height: self.viewport_height,
            theme_picker: None,
            config_dir: self.config_dir,
            flash_message: None,
            stories_fetched_at: self.stories_fetched_at,
            comments_fetched_at: self.comments_fetched_at,
        }
    }
}

pub fn sample_stories() -> Vec<Story> {
    vec![
        StoryBuilder::new()
            .id(1)
            .title("Show HN: I built a terminal UI for Hacker News")
            .url("https://github.com/user/hn")
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
