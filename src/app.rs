use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use tokio::sync::mpsc;

use crate::api::{Comment, Feed, HnClient, Story};
use crate::theme::ResolvedTheme;

pub enum AsyncResult {
    Stories {
        generation: u64,
        task_id: u64,
        result: Result<Vec<Story>, String>,
    },
    MoreStories {
        generation: u64,
        task_id: u64,
        result: Result<Vec<Story>, String>,
    },
    Comments {
        story_id: u64,
        task_id: u64,
        result: Result<Vec<Comment>, String>,
    },
}

pub struct TaskInfo {
    pub id: u64,
    pub description: String,
    pub started_at: Instant,
}

pub struct LogEntry {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum View {
    #[default]
    Stories,
    Comments {
        story_id: u64,
        story_title: String,
        story_index: usize,
        story_scroll: usize,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectNext,
    SelectPrev,
    SelectFirst,
    SelectLast,
    PageDown,
    PageUp,
    OpenUrl,
    OpenComments,
    OpenCommentsUrl,
    ExpandComment,
    CollapseComment,
    Back,
    Quit,
    Refresh,
    ToggleHelp,
    ToggleDebug,
    SwitchFeed(Feed),
    NextFeed,
    PrevFeed,
    UpdateViewportHeight(u16),
}

pub struct App {
    pub view: View,
    pub feed: Feed,
    pub stories: Vec<Story>,
    pub comments: Vec<Comment>,
    pub expanded_comments: HashSet<u64>,
    pub selected_index: usize,
    pub loading: bool,
    pub loading_start: Option<Instant>,
    pub loading_more: bool,
    pub current_page: usize,
    pub has_more: bool,
    pub error: Option<String>,
    pub should_quit: bool,
    pub show_help: bool,
    pub client: HnClient,
    pub scroll_offset: usize,
    pub theme: ResolvedTheme,
    // Async task management
    pub result_tx: mpsc::Sender<AsyncResult>,
    pub result_rx: mpsc::Receiver<AsyncResult>,
    pub generation: u64,
    // Debug pane
    pub debug_visible: bool,
    pub running_tasks: Vec<TaskInfo>,
    pub debug_log: VecDeque<LogEntry>,
    pub next_task_id: u64,
    // Viewport tracking for dynamic story loading
    pub viewport_height: Option<u16>,
}

impl App {
    pub fn new(theme: ResolvedTheme) -> Self {
        let (result_tx, result_rx) = mpsc::channel(10);
        Self {
            view: View::default(),
            feed: Feed::default(),
            stories: Vec::new(),
            comments: Vec::new(),
            expanded_comments: HashSet::new(),
            selected_index: 0,
            loading: false,
            loading_start: None,
            loading_more: false,
            current_page: 0,
            has_more: true,
            error: None,
            should_quit: false,
            show_help: false,
            client: HnClient::new(),
            scroll_offset: 0,
            theme,
            result_tx,
            result_rx,
            generation: 0,
            debug_visible: false,
            running_tasks: Vec::new(),
            debug_log: VecDeque::new(),
            next_task_id: 0,
            viewport_height: None,
        }
    }

    fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        if loading {
            self.loading_start = Some(Instant::now());
        }
        // Don't clear loading_start when done - used for minimum spinner duration
    }

    pub fn should_show_spinner(&self) -> bool {
        const MIN_SPINNER_DURATION: std::time::Duration = std::time::Duration::from_millis(500);
        if let Some(start) = self.loading_start {
            self.loading || start.elapsed() < MIN_SPINNER_DURATION
        } else {
            false
        }
    }

    pub fn log_debug(&mut self, msg: impl Into<String>) {
        self.debug_log.push_back(LogEntry {
            message: msg.into(),
        });
        if self.debug_log.len() > 50 {
            self.debug_log.pop_front();
        }
    }

    fn start_task(&mut self, description: impl Into<String>) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;
        let desc = description.into();
        self.log_debug(format!("Started: {}", desc));
        self.running_tasks.push(TaskInfo {
            id,
            description: desc,
            started_at: Instant::now(),
        });
        id
    }

    fn end_task(&mut self, id: u64, outcome: &str) {
        if let Some(pos) = self.running_tasks.iter().position(|t| t.id == id) {
            let task = self.running_tasks.remove(pos);
            let elapsed = task.started_at.elapsed();
            self.log_debug(format!("{} {}: {:.2?}", task.description, outcome, elapsed));
        }
    }

    pub fn handle_async_result(&mut self, result: AsyncResult) {
        match result {
            AsyncResult::Stories {
                generation,
                task_id,
                result,
            } => {
                if generation != self.generation {
                    self.end_task(task_id, "discarded (stale)");
                    return;
                }
                self.end_task(
                    task_id,
                    if result.is_ok() {
                        "completed"
                    } else {
                        "failed"
                    },
                );
                match result {
                    Ok(stories) => {
                        self.stories = stories;
                        self.set_loading(false);
                        self.selected_index = 0;
                        self.scroll_offset = 0;
                        if self.should_fill_viewport() {
                            self.load_more();
                        }
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to load stories: {}", e));
                        self.set_loading(false);
                    }
                }
            }
            AsyncResult::MoreStories {
                generation,
                task_id,
                result,
            } => {
                if generation != self.generation {
                    self.end_task(task_id, "discarded (stale)");
                    return;
                }
                self.end_task(
                    task_id,
                    if result.is_ok() {
                        "completed"
                    } else {
                        "failed"
                    },
                );
                match result {
                    Ok(stories) => {
                        if stories.is_empty() {
                            self.has_more = false;
                        } else {
                            self.stories.extend(stories);
                            self.current_page += 1;
                        }
                        self.loading_more = false;
                        if self.should_fill_viewport() {
                            self.load_more();
                        }
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to load more: {}", e));
                        self.loading_more = false;
                    }
                }
            }
            AsyncResult::Comments {
                story_id,
                task_id,
                result,
            } => {
                let is_current =
                    matches!(&self.view, View::Comments { story_id: id, .. } if *id == story_id);
                if !is_current {
                    self.end_task(task_id, "discarded (wrong view)");
                    return;
                }
                self.end_task(
                    task_id,
                    if result.is_ok() {
                        "completed"
                    } else {
                        "failed"
                    },
                );
                match result {
                    Ok(comments) => {
                        self.comments = comments;
                        self.set_loading(false);
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to load comments: {}", e));
                        self.set_loading(false);
                    }
                }
            }
        }
    }

    pub fn update(&mut self, msg: Message) {
        self.error = None;

        match msg {
            Message::SelectNext => {
                self.select_next();
                if self.should_load_more() {
                    self.load_more();
                }
            }
            Message::SelectPrev => self.select_prev(),
            Message::SelectFirst => self.select_first(),
            Message::SelectLast => {
                self.select_last();
                if self.should_load_more() {
                    self.load_more();
                }
            }
            Message::PageDown => {
                self.page_down();
                if self.should_load_more() {
                    self.load_more();
                }
            }
            Message::PageUp => self.page_up(),
            Message::OpenUrl => self.open_url(),
            Message::OpenComments => self.open_comments(),
            Message::OpenCommentsUrl => self.open_comments_url(),
            Message::ExpandComment => self.expand_comment(),
            Message::CollapseComment => self.collapse_comment(),
            Message::Back => self.go_back(),
            Message::Quit => self.should_quit = true,
            Message::Refresh => self.refresh(),
            Message::ToggleHelp => self.show_help = !self.show_help,
            Message::ToggleDebug => self.debug_visible = !self.debug_visible,
            Message::SwitchFeed(feed) => self.switch_feed(feed),
            Message::NextFeed => self.cycle_feed(1),
            Message::PrevFeed => self.cycle_feed(-1),
            Message::UpdateViewportHeight(height) => {
                let old_height = self.viewport_height;
                self.viewport_height = Some(height);
                if old_height.is_none_or(|h| height > h) && self.should_fill_viewport() {
                    self.load_more();
                }
            }
        }
    }

    pub fn visible_comment_indices(&self) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut parent_visible_at_depth: Vec<bool> = vec![true];

        for (i, comment) in self.comments.iter().enumerate() {
            parent_visible_at_depth.truncate(comment.depth + 1);

            let is_visible = parent_visible_at_depth
                .get(comment.depth)
                .copied()
                .unwrap_or(false);

            if is_visible {
                visible.push(i);

                let children_visible = self.expanded_comments.contains(&comment.id);
                if parent_visible_at_depth.len() <= comment.depth + 1 {
                    parent_visible_at_depth.push(children_visible);
                } else {
                    parent_visible_at_depth[comment.depth + 1] = children_visible;
                }
            }
        }

        visible
    }

    fn actual_comment_index(&self, visible_index: usize) -> Option<usize> {
        self.visible_comment_indices().get(visible_index).copied()
    }

    pub fn selected_comment(&self) -> Option<&Comment> {
        let actual_idx = self.actual_comment_index(self.selected_index)?;
        self.comments.get(actual_idx)
    }

    fn expand_comment(&mut self) {
        if let View::Comments { .. } = self.view
            && let Some(comment) = self.selected_comment()
            && !comment.kids.is_empty()
        {
            let id = comment.id;
            self.expanded_comments.insert(id);
        }
    }

    fn collapse_comment(&mut self) {
        if let View::Comments { .. } = self.view {
            let (id, depth) = match self.selected_comment() {
                Some(c) => (c.id, c.depth),
                None => return,
            };

            self.expanded_comments.remove(&id);

            if depth > 0 && self.selected_index > 0 {
                let visible = self.visible_comment_indices();
                for i in (0..self.selected_index).rev() {
                    if let Some(&actual_idx) = visible.get(i)
                        && self.comments[actual_idx].depth < depth
                    {
                        self.selected_index = i;
                        return;
                    }
                }
            }
        }
    }

    fn item_count(&self) -> usize {
        match self.view {
            View::Stories => self.stories.len(),
            View::Comments { .. } => self.visible_comment_indices().len(),
        }
    }

    fn select_next(&mut self) {
        let count = self.item_count();
        if count > 0 && self.selected_index < count - 1 {
            self.selected_index += 1;
        }
    }

    fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn select_first(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    fn select_last(&mut self) {
        let count = self.item_count();
        if count > 0 {
            self.selected_index = count - 1;
        }
    }

    fn page_down(&mut self) {
        let count = self.item_count();
        if count > 0 {
            self.selected_index = (self.selected_index + 10).min(count - 1);
        }
    }

    fn page_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(10);
    }

    fn open_url(&self) {
        let story = match &self.view {
            View::Stories => self.stories.get(self.selected_index),
            View::Comments { story_index, .. } => self.stories.get(*story_index),
        };
        if let Some(story) = story {
            let url = story
                .url
                .clone()
                .unwrap_or_else(|| format!("https://news.ycombinator.com/item?id={}", story.id));
            let _ = open::that(url);
        }
    }

    fn open_comments_url(&self) {
        match &self.view {
            View::Stories => {
                if let Some(story) = self.stories.get(self.selected_index) {
                    let url = format!("https://news.ycombinator.com/item?id={}", story.id);
                    let _ = open::that(url);
                }
            }
            View::Comments { .. } => {
                if let Some(comment) = self.selected_comment() {
                    let url = format!("https://news.ycombinator.com/item?id={}", comment.id);
                    let _ = open::that(url);
                }
            }
        }
    }

    fn open_comments(&mut self) {
        if let View::Stories = self.view
            && let Some(story) = self.stories.get(self.selected_index).cloned()
        {
            let story_index = self.selected_index;
            let story_scroll = self.scroll_offset;
            let story_id = story.id;

            self.view = View::Comments {
                story_id,
                story_title: story.title.clone(),
                story_index,
                story_scroll,
            };
            self.set_loading(true);
            self.comments.clear();
            self.expanded_comments.clear();
            self.selected_index = 0;
            self.scroll_offset = 0;

            let task_id = self.start_task(format!("Load comments for {}", story_id));
            let client = self.client.clone();
            let tx = self.result_tx.clone();

            tokio::spawn(async move {
                let result = client
                    .fetch_comments_flat(&story, 5)
                    .await
                    .map_err(|e| e.to_string());
                let _ = tx
                    .send(AsyncResult::Comments {
                        story_id,
                        task_id,
                        result,
                    })
                    .await;
            });
        }
    }

    fn go_back(&mut self) {
        if let View::Comments {
            story_index,
            story_scroll,
            ..
        } = self.view
        {
            self.view = View::Stories;
            self.comments.clear();
            self.selected_index = story_index;
            self.scroll_offset = story_scroll;
        }
    }

    fn refresh(&mut self) {
        match &self.view {
            View::Stories => {
                // Clear cache and reload stories (keep existing stories visible during load)
                let client = self.client.clone();
                let feed = self.feed;
                let tx = self.result_tx.clone();

                self.generation += 1;
                self.set_loading(true);
                self.current_page = 0;
                self.has_more = true;

                let generation = self.generation;
                let task_id = self.start_task(format!("Refresh {} stories", feed.label()));

                tokio::spawn(async move {
                    client.clear_cache().await;
                    let result = client
                        .fetch_stories(feed, 0)
                        .await
                        .map_err(|e| e.to_string());
                    let _ = tx
                        .send(AsyncResult::Stories {
                            generation,
                            task_id,
                            result,
                        })
                        .await;
                });
            }
            View::Comments { story_id, .. } => {
                let story_id = *story_id;
                if let Some(story) = self.stories.iter().find(|s| s.id == story_id).cloned() {
                    self.set_loading(true);

                    let client = self.client.clone();
                    let tx = self.result_tx.clone();
                    let task_id = self.start_task(format!("Refresh comments for {}", story_id));

                    tokio::spawn(async move {
                        client.clear_cache().await;
                        let result = client
                            .fetch_comments_flat(&story, 5)
                            .await
                            .map_err(|e| e.to_string());
                        let _ = tx
                            .send(AsyncResult::Comments {
                                story_id,
                                task_id,
                                result,
                            })
                            .await;
                    });
                }
            }
        }
    }

    fn switch_feed(&mut self, feed: Feed) {
        if self.feed != feed {
            self.feed = feed;
            self.view = View::Stories;
            self.load_stories();
        }
    }

    fn cycle_feed(&mut self, direction: i32) {
        let feeds = Feed::all();
        let current_idx = feeds.iter().position(|&f| f == self.feed).unwrap_or(0);
        let new_idx = (current_idx as i32 + direction).rem_euclid(feeds.len() as i32) as usize;
        self.switch_feed(feeds[new_idx]);
    }

    pub fn load_stories(&mut self) {
        self.generation += 1;
        self.set_loading(true);
        self.error = None;
        self.stories.clear();
        self.current_page = 0;
        self.has_more = true;

        let client = self.client.clone();
        let feed = self.feed;
        let tx = self.result_tx.clone();
        let generation = self.generation;
        let task_id = self.start_task(format!("Load {} stories", feed.label()));

        tokio::spawn(async move {
            let result = client
                .fetch_stories(feed, 0)
                .await
                .map_err(|e| e.to_string());
            let _ = tx
                .send(AsyncResult::Stories {
                    generation,
                    task_id,
                    result,
                })
                .await;
        });
    }

    fn should_load_more(&self) -> bool {
        const THRESHOLD: usize = 5;
        matches!(self.view, View::Stories)
            && !self.loading
            && !self.loading_more
            && self.has_more
            && !self.stories.is_empty()
            && self.selected_index + THRESHOLD >= self.stories.len()
    }

    pub fn visible_story_capacity(&self) -> usize {
        const LAYOUT_OVERHEAD: u16 = 4; // 1 tabs + 1 status bar + 2 borders
        const STORY_HEIGHT: u16 = 2; // title + metadata

        self.viewport_height
            .map(|h| (h.saturating_sub(LAYOUT_OVERHEAD) / STORY_HEIGHT) as usize)
            .unwrap_or(0)
    }

    fn should_fill_viewport(&self) -> bool {
        matches!(self.view, View::Stories)
            && !self.loading
            && !self.loading_more
            && self.has_more
            && !self.stories.is_empty()
            && self.stories.len() < self.visible_story_capacity()
    }

    fn load_more(&mut self) {
        if self.loading_more || !self.has_more {
            return;
        }

        self.loading_more = true;
        let next_page = self.current_page + 1;

        let client = self.client.clone();
        let feed = self.feed;
        let tx = self.result_tx.clone();
        let generation = self.generation;
        let task_id = self.start_task(format!("Load {} page {}", feed.label(), next_page));

        tokio::spawn(async move {
            let result = client
                .fetch_stories(feed, next_page)
                .await
                .map_err(|e| e.to_string());
            let _ = tx
                .send(AsyncResult::MoreStories {
                    generation,
                    task_id,
                    result,
                })
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{StoryBuilder, TestAppBuilder, sample_stories};
    use crate::theme::{ThemeVariant, default_for_variant};

    fn test_app() -> App {
        App::new(default_for_variant(ThemeVariant::Dark))
    }

    #[test]
    fn test_new_app() {
        let app = test_app();
        assert_eq!(app.view, View::Stories);
        assert_eq!(app.feed, Feed::Top);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_navigation() {
        let mut app = test_app();
        app.stories = vec![
            Story {
                id: 1,
                title: "A".to_string(),
                url: None,
                score: 1,
                by: "u".to_string(),
                time: 0,
                descendants: 0,
                kids: vec![],
            },
            Story {
                id: 2,
                title: "B".to_string(),
                url: None,
                score: 1,
                by: "u".to_string(),
                time: 0,
                descendants: 0,
                kids: vec![],
            },
        ];

        assert_eq!(app.selected_index, 0);
        app.select_next();
        assert_eq!(app.selected_index, 1);
        app.select_next(); // Should not go past last
        assert_eq!(app.selected_index, 1);
        app.select_prev();
        assert_eq!(app.selected_index, 0);
        app.select_prev(); // Should not go below 0
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_go_back_restores_state() {
        let mut app = test_app();
        app.view = View::Comments {
            story_id: 1,
            story_title: "Test".to_string(),
            story_index: 5,
            story_scroll: 10,
        };
        app.selected_index = 3; // Comment selection
        app.go_back();
        assert_eq!(app.view, View::Stories);
        assert_eq!(app.selected_index, 5);
        assert_eq!(app.scroll_offset, 10);
    }

    #[test]
    fn visible_story_capacity_with_no_viewport() {
        let app = TestAppBuilder::new().build();
        assert_eq!(app.visible_story_capacity(), 0);
    }

    #[test]
    fn visible_story_capacity_with_small_terminal() {
        // 24 lines: (24 - 4) / 2 = 10 stories
        let app = TestAppBuilder::new().viewport_height(24).build();
        assert_eq!(app.visible_story_capacity(), 10);
    }

    #[test]
    fn visible_story_capacity_with_large_terminal() {
        // 80 lines: (80 - 4) / 2 = 38 stories
        let app = TestAppBuilder::new().viewport_height(80).build();
        assert_eq!(app.visible_story_capacity(), 38);
    }

    #[test]
    fn visible_story_capacity_with_minimum_height() {
        // 4 lines (just overhead): (4 - 4) / 2 = 0 stories
        let app = TestAppBuilder::new().viewport_height(4).build();
        assert_eq!(app.visible_story_capacity(), 0);
    }

    #[test]
    fn should_fill_viewport_when_stories_below_capacity() {
        let stories = sample_stories(); // 5 stories
        let app = TestAppBuilder::new()
            .with_stories(stories)
            .viewport_height(50) // capacity = 23
            .has_more(true)
            .build();
        assert!(app.should_fill_viewport());
    }

    #[test]
    fn should_not_fill_viewport_when_stories_at_capacity() {
        let stories: Vec<_> = (0..25).map(|i| StoryBuilder::new().id(i).build()).collect();
        let app = TestAppBuilder::new()
            .with_stories(stories) // 25 stories
            .viewport_height(50) // capacity = 23
            .has_more(true)
            .build();
        assert!(!app.should_fill_viewport());
    }

    #[test]
    fn should_not_fill_viewport_when_no_more_stories() {
        let stories = sample_stories();
        let app = TestAppBuilder::new()
            .with_stories(stories)
            .viewport_height(50)
            .has_more(false)
            .build();
        assert!(!app.should_fill_viewport());
    }

    #[test]
    fn should_not_fill_viewport_when_loading() {
        let stories = sample_stories();
        let app = TestAppBuilder::new()
            .with_stories(stories)
            .viewport_height(50)
            .has_more(true)
            .loading()
            .build();
        assert!(!app.should_fill_viewport());
    }

    #[test]
    fn should_not_fill_viewport_when_loading_more() {
        let stories = sample_stories();
        let app = TestAppBuilder::new()
            .with_stories(stories)
            .viewport_height(50)
            .has_more(true)
            .loading_more(true)
            .build();
        assert!(!app.should_fill_viewport());
    }

    #[test]
    fn should_not_fill_viewport_in_comments_view() {
        let stories = sample_stories();
        let app = TestAppBuilder::new()
            .with_stories(stories)
            .viewport_height(50)
            .has_more(true)
            .view(View::Comments {
                story_id: 1,
                story_title: "Test".to_string(),
                story_index: 0,
                story_scroll: 0,
            })
            .build();
        assert!(!app.should_fill_viewport());
    }

    #[tokio::test]
    async fn update_viewport_height_triggers_load_when_needed() {
        let stories = sample_stories(); // 5 stories
        let mut app = TestAppBuilder::new()
            .with_stories(stories)
            .has_more(true)
            .build();

        app.update(Message::UpdateViewportHeight(50)); // capacity = 23

        assert_eq!(app.viewport_height, Some(50));
        assert!(app.loading_more); // should have triggered load_more
    }

    #[test]
    fn update_viewport_height_no_load_when_shrinking() {
        let stories = sample_stories();
        let mut app = TestAppBuilder::new()
            .with_stories(stories)
            .viewport_height(50)
            .has_more(true)
            .build();

        app.update(Message::UpdateViewportHeight(24)); // shrink

        assert_eq!(app.viewport_height, Some(24));
        assert!(!app.loading_more); // should NOT trigger load
    }
}
