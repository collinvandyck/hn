use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::api::{ApiError, Comment, Feed, HnClient, Story};
use crate::comment_tree::CommentTree;
use crate::settings::{self, Settings};
use crate::storage::Storage;
use crate::theme::{ResolvedTheme, Theme, all_themes};
use crate::time::Clock;

pub enum AsyncResult {
    Stories {
        generation: u64,
        task_id: u64,
        result: Result<Vec<Story>, ApiError>,
    },
    MoreStories {
        generation: u64,
        task_id: u64,
        result: Result<Vec<Story>, ApiError>,
    },
    Comments {
        story_id: u64,
        task_id: u64,
        result: Result<Vec<Comment>, ApiError>,
    },
}

#[derive(Debug)]
pub struct TaskInfo {
    pub id: u64,
    pub description: String,
    pub started_at: Instant,
}

#[derive(Debug)]
pub struct LogEntry {
    pub message: String,
}

/// Debug panel state: task tracking and log messages.
#[derive(Debug, Default)]
pub struct DebugState {
    pub visible: bool,
    pub running_tasks: Vec<TaskInfo>,
    pub log: VecDeque<LogEntry>,
    next_task_id: u64,
}

impl DebugState {
    const MAX_LOG_ENTRIES: usize = 50;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        self.log.push_back(LogEntry {
            message: msg.into(),
        });
        if self.log.len() > Self::MAX_LOG_ENTRIES {
            self.log.pop_front();
        }
    }

    pub fn start_task(&mut self, description: impl Into<String>) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;
        let desc = description.into();
        self.log(format!("Started: {}", desc));
        self.running_tasks.push(TaskInfo {
            id,
            description: desc,
            started_at: Instant::now(),
        });
        id
    }

    pub fn end_task(&mut self, id: u64, outcome: &str) {
        if let Some(pos) = self.running_tasks.iter().position(|t| t.id == id) {
            let task = self.running_tasks.remove(pos);
            let elapsed = task.started_at.elapsed();
            self.log(format!("{} {}: {:.2?}", task.description, outcome, elapsed));
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

/// Loading and pagination state.
#[derive(Debug, Default)]
pub struct LoadState {
    pub loading: bool,
    pub loading_start: Option<Instant>,
    pub loading_more: bool,
    pub current_page: usize,
    pub has_more: bool,
    pub error: Option<String>,
}

impl LoadState {
    pub fn new() -> Self {
        Self {
            has_more: true,
            ..Default::default()
        }
    }

    pub fn set_loading(&mut self, loading: bool) {
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

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }
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

/// State for the theme picker popup.
pub struct ThemePicker {
    pub themes: Vec<Theme>,
    pub selected: usize,
    pub original: ResolvedTheme,
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
    ExpandSubtree,
    CollapseSubtree,
    ExpandThread,
    CollapseThread,
    Back,
    Quit,
    Refresh,
    ToggleHelp,
    ToggleDebug,
    SwitchFeed(Feed),
    NextFeed,
    PrevFeed,
    UpdateViewportHeight(u16),
    // Theme picker
    OpenThemePicker,
    CloseThemePicker,
    ConfirmThemePicker,
    ThemePickerUp,
    ThemePickerDown,
}

pub struct App {
    pub view: View,
    pub feed: Feed,
    pub stories: Vec<Story>,
    pub comment_tree: CommentTree,
    pub selected_index: usize,
    pub load: LoadState,
    pub should_quit: bool,
    pub show_help: bool,
    pub client: HnClient,
    pub scroll_offset: usize,
    pub theme: ResolvedTheme,
    pub clock: Arc<dyn Clock>,
    // Async task management
    pub result_tx: mpsc::Sender<AsyncResult>,
    pub result_rx: mpsc::Receiver<AsyncResult>,
    pub generation: u64,
    // Debug pane
    pub debug: DebugState,
    // Viewport tracking for dynamic story loading
    pub viewport_height: Option<u16>,
    // Theme picker popup
    pub theme_picker: Option<ThemePicker>,
    // Config directory for persisting settings
    pub config_dir: Option<PathBuf>,
}

impl App {
    pub fn new(
        theme: ResolvedTheme,
        config_dir: Option<PathBuf>,
        storage: Option<Storage>,
    ) -> Self {
        let (result_tx, result_rx) = mpsc::channel(10);
        let client = HnClient::new(storage);
        Self {
            view: View::default(),
            feed: Feed::default(),
            stories: Vec::new(),
            comment_tree: CommentTree::new(),
            selected_index: 0,
            load: LoadState::new(),
            should_quit: false,
            show_help: false,
            client,
            scroll_offset: 0,
            theme,
            clock: crate::time::system_clock(),
            result_tx,
            result_rx,
            generation: 0,
            debug: DebugState::new(),
            viewport_height: None,
            theme_picker: None,
            config_dir,
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
                    self.debug.end_task(task_id, "discarded (stale)");
                    return;
                }
                self.debug.end_task(
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
                        self.load.set_loading(false);
                        self.selected_index = 0;
                        self.scroll_offset = 0;
                        if self.should_fill_viewport() {
                            self.load_more();
                        }
                    }
                    Err(e) => {
                        self.load.set_error(e.user_message());
                        self.load.set_loading(false);
                        if e.is_fatal() {
                            self.should_quit = true;
                        }
                    }
                }
            }
            AsyncResult::MoreStories {
                generation,
                task_id,
                result,
            } => {
                if generation != self.generation {
                    self.debug.end_task(task_id, "discarded (stale)");
                    return;
                }
                self.debug.end_task(
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
                            self.load.has_more = false;
                        } else {
                            self.stories.extend(stories);
                            self.load.current_page += 1;
                        }
                        self.load.loading_more = false;
                        if self.should_fill_viewport() {
                            self.load_more();
                        }
                    }
                    Err(e) => {
                        self.load.set_error(e.user_message());
                        self.load.loading_more = false;
                        if e.is_fatal() {
                            self.should_quit = true;
                        }
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
                    self.debug.end_task(task_id, "discarded (wrong view)");
                    return;
                }
                self.debug.end_task(
                    task_id,
                    if result.is_ok() {
                        "completed"
                    } else {
                        "failed"
                    },
                );
                match result {
                    Ok(comments) => {
                        self.comment_tree.set(comments);
                        self.load.set_loading(false);
                    }
                    Err(e) => {
                        self.load.set_error(e.user_message());
                        self.load.set_loading(false);
                        if e.is_fatal() {
                            self.should_quit = true;
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self, msg: Message) {
        self.load.clear_error();

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
            Message::ExpandSubtree => self.expand_subtree(),
            Message::CollapseSubtree => self.collapse_subtree(),
            Message::ExpandThread => self.expand_thread(),
            Message::CollapseThread => self.collapse_thread(),
            Message::Back => self.go_back(),
            Message::Quit => self.should_quit = true,
            Message::Refresh => self.refresh(),
            Message::ToggleHelp => self.show_help = !self.show_help,
            Message::ToggleDebug => self.debug.toggle(),
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
            Message::OpenThemePicker => self.open_theme_picker(),
            Message::CloseThemePicker => self.close_theme_picker(),
            Message::ConfirmThemePicker => self.confirm_theme_picker(),
            Message::ThemePickerUp => self.theme_picker_up(),
            Message::ThemePickerDown => self.theme_picker_down(),
        }
    }

    fn open_theme_picker(&mut self) {
        let themes = all_themes();
        let current_name = &self.theme.name;
        let selected = themes
            .iter()
            .position(|t| &t.name == current_name)
            .unwrap_or(0);
        self.theme_picker = Some(ThemePicker {
            themes,
            selected,
            original: self.theme.clone(),
        });
    }

    fn close_theme_picker(&mut self) {
        if let Some(picker) = self.theme_picker.take() {
            self.theme = picker.original;
        }
    }

    fn confirm_theme_picker(&mut self) {
        if let Some(config_dir) = &self.config_dir {
            let path = settings::settings_path(config_dir);
            match Settings::load(&path) {
                Ok(mut current_settings) => {
                    current_settings.theme = Some(self.theme.name.clone());
                    if let Err(e) = current_settings.save(&path) {
                        self.debug.log(format!("Failed to save settings: {}", e));
                    }
                }
                Err(e) if path.exists() => {
                    self.debug.log(format!("Won't save: {}", e));
                }
                Err(_) => {
                    let settings = Settings {
                        theme: Some(self.theme.name.clone()),
                        ..Default::default()
                    };
                    if let Err(e) = settings.save(&path) {
                        self.debug.log(format!("Failed to save settings: {}", e));
                    }
                }
            }
        }
        self.theme_picker = None;
    }

    fn theme_picker_up(&mut self) {
        if let Some(picker) = &mut self.theme_picker
            && picker.selected > 0
        {
            picker.selected -= 1;
            self.theme = picker.themes[picker.selected].clone().into();
        }
    }

    fn theme_picker_down(&mut self) {
        if let Some(picker) = &mut self.theme_picker
            && picker.selected < picker.themes.len() - 1
        {
            picker.selected += 1;
            self.theme = picker.themes[picker.selected].clone().into();
        }
    }

    pub fn visible_comment_indices(&self) -> Vec<usize> {
        self.comment_tree.visible_indices()
    }

    fn actual_comment_index(&self, visible_index: usize) -> Option<usize> {
        self.visible_comment_indices().get(visible_index).copied()
    }

    pub fn selected_comment(&self) -> Option<&Comment> {
        let actual_idx = self.actual_comment_index(self.selected_index)?;
        self.comment_tree.get(actual_idx)
    }

    fn expand_comment(&mut self) {
        if let View::Comments { .. } = self.view
            && let Some(comment) = self.selected_comment()
            && !comment.kids.is_empty()
        {
            let id = comment.id;
            if self.comment_tree.is_expanded(id) {
                // Already expanded - move to first child
                self.selected_index += 1;
            } else {
                self.comment_tree.expand(id);
            }
        }
    }

    fn collapse_comment(&mut self) {
        if let View::Comments { .. } = self.view {
            let Some(comment) = self.selected_comment() else {
                // No comments - go back to stories
                self.go_back();
                return;
            };

            let (id, depth) = (comment.id, comment.depth);
            let has_children = !comment.kids.is_empty();
            let is_expanded = self.comment_tree.is_expanded(id);

            if depth == 0 {
                // Top-level: collapse if expanded with children, otherwise go back
                if has_children && is_expanded {
                    self.comment_tree.collapse(id);
                } else {
                    self.go_back();
                }
                return;
            }

            self.comment_tree.collapse(id);

            // Navigate to parent
            let visible = self.visible_comment_indices();
            if let Some(parent_idx) = self
                .comment_tree
                .find_parent_visible_index(&visible, self.selected_index)
            {
                self.selected_index = parent_idx;
            }
        }
    }

    fn expand_subtree(&mut self) {
        if let View::Comments { .. } = self.view {
            let Some(start_idx) = self.actual_comment_index(self.selected_index) else {
                return;
            };
            self.comment_tree.expand_subtree(start_idx);
        }
    }

    fn collapse_subtree(&mut self) {
        if let View::Comments { .. } = self.view {
            let visible = self.visible_comment_indices();
            let Some((ancestor_visible_idx, ancestor_actual_idx)) = self
                .comment_tree
                .find_toplevel_ancestor(&visible, self.selected_index)
            else {
                return;
            };

            self.comment_tree.collapse_subtree(ancestor_actual_idx);
            self.selected_index = ancestor_visible_idx;
        }
    }

    fn expand_thread(&mut self) {
        if let View::Comments { .. } = self.view {
            self.comment_tree.expand_all();
        }
    }

    fn collapse_thread(&mut self) {
        if let View::Comments { .. } = self.view {
            self.comment_tree.collapse_all();
        }
    }

    fn item_count(&self) -> usize {
        match self.view {
            View::Stories => self.stories.len(),
            View::Comments { .. } => self.comment_tree.visible_count(),
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
            let _ = open::that(story.content_url());
        }
    }

    fn open_comments_url(&self) {
        match &self.view {
            View::Stories => {
                if let Some(story) = self.stories.get(self.selected_index) {
                    let _ = open::that(story.hn_url());
                }
            }
            View::Comments { .. } => {
                if let Some(comment) = self.selected_comment() {
                    let _ = open::that(comment.hn_url());
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

            self.view = View::Comments {
                story_id: story.id,
                story_title: story.title.clone(),
                story_index,
                story_scroll,
            };
            self.load.set_loading(true);
            self.comment_tree.clear();
            self.selected_index = 0;
            self.scroll_offset = 0;
            self.spawn_comments_fetch(story, false);
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
            self.comment_tree.clear();
            self.selected_index = story_index;
            self.scroll_offset = story_scroll;
        }
    }

    fn refresh(&mut self) {
        match &self.view {
            View::Stories => {
                self.generation += 1;
                self.load.set_loading(true);
                self.load.current_page = 0;
                self.load.has_more = true;
                self.spawn_stories_fetch(0, true, false);
            }
            View::Comments { story_id, .. } => {
                if let Some(story) = self.stories.iter().find(|s| s.id == *story_id).cloned() {
                    self.load.set_loading(true);
                    self.spawn_comments_fetch(story, true);
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
        self.load.set_loading(true);
        self.load.clear_error();
        self.stories.clear();
        self.load.current_page = 0;
        self.load.has_more = true;
        self.spawn_stories_fetch(0, false, false);
    }

    fn should_load_more(&self) -> bool {
        const THRESHOLD: usize = 5;
        matches!(self.view, View::Stories)
            && !self.load.loading
            && !self.load.loading_more
            && self.load.has_more
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
            && !self.load.loading
            && !self.load.loading_more
            && self.load.has_more
            && !self.stories.is_empty()
            && self.stories.len() < self.visible_story_capacity()
    }

    fn load_more(&mut self) {
        if self.load.loading_more || !self.load.has_more {
            return;
        }

        self.load.loading_more = true;
        let next_page = self.load.current_page + 1;
        self.spawn_stories_fetch(next_page, false, true);
    }

    /// Spawn an async task to fetch stories.
    ///
    /// - `page`: Which page to fetch (0 for initial load)
    /// - `force_refresh`: Whether to bypass cache and fetch fresh data
    /// - `is_more`: If true, sends `AsyncResult::MoreStories`; otherwise `AsyncResult::Stories`
    fn spawn_stories_fetch(&mut self, page: usize, force_refresh: bool, is_more: bool) {
        let client = self.client.clone();
        let feed = self.feed;
        let tx = self.result_tx.clone();
        let generation = self.generation;

        let task_desc = if is_more {
            format!("Load {} page {}", feed.label(), page)
        } else if force_refresh {
            format!("Refresh {} stories", feed.label())
        } else {
            format!("Load {} stories", feed.label())
        };
        let task_id = self.debug.start_task(task_desc);

        tokio::spawn(async move {
            let result = client.fetch_stories(feed, page, force_refresh).await;

            let msg = if is_more {
                AsyncResult::MoreStories {
                    generation,
                    task_id,
                    result,
                }
            } else {
                AsyncResult::Stories {
                    generation,
                    task_id,
                    result,
                }
            };
            let _ = tx.send(msg).await;
        });
    }

    /// Spawn an async task to fetch comments for a story.
    ///
    /// - `story`: The story to fetch comments for
    /// - `force_refresh`: Whether to bypass cache and fetch fresh data
    fn spawn_comments_fetch(&mut self, story: Story, force_refresh: bool) {
        let story_id = story.id;
        let client = self.client.clone();
        let tx = self.result_tx.clone();

        let task_desc = if force_refresh {
            format!("Refresh comments for {}", story_id)
        } else {
            format!("Load comments for {}", story_id)
        };
        let task_id = self.debug.start_task(task_desc);

        tokio::spawn(async move {
            let result = client.fetch_comments_flat(&story, 5, force_refresh).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{StoryBuilder, TestAppBuilder, sample_stories};
    use crate::theme::{ThemeVariant, default_for_variant};

    fn test_app() -> App {
        App::new(default_for_variant(ThemeVariant::Dark), None, None)
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
        assert!(app.load.loading_more); // should have triggered load_more
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
        assert!(!app.load.loading_more); // should NOT trigger load
    }
}
