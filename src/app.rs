use std::collections::HashSet;
use std::time::Instant;

use crate::api::{Comment, Feed, HnClient, Story};
use crate::theme::ResolvedTheme;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Stories,
    Comments {
        story_id: u64,
        story_title: String,
        story_index: usize,
        story_scroll: usize,
    },
}

impl Default for View {
    fn default() -> Self {
        View::Stories
    }
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
    SwitchFeed(Feed),
    NextFeed,
    PrevFeed,
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
    pub error: Option<String>,
    pub should_quit: bool,
    pub show_help: bool,
    pub client: HnClient,
    pub scroll_offset: usize,
    pub theme: ResolvedTheme,
}

impl App {
    pub fn new(theme: ResolvedTheme) -> Self {
        Self {
            view: View::default(),
            feed: Feed::default(),
            stories: Vec::new(),
            comments: Vec::new(),
            expanded_comments: HashSet::new(),
            selected_index: 0,
            loading: false,
            loading_start: None,
            error: None,
            should_quit: false,
            show_help: false,
            client: HnClient::new(),
            scroll_offset: 0,
            theme,
        }
    }

    fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        self.loading_start = if loading { Some(Instant::now()) } else { None };
    }

    pub async fn update(&mut self, msg: Message) {
        self.error = None;

        match msg {
            Message::SelectNext => self.select_next(),
            Message::SelectPrev => self.select_prev(),
            Message::SelectFirst => self.select_first(),
            Message::SelectLast => self.select_last(),
            Message::PageDown => self.page_down(),
            Message::PageUp => self.page_up(),
            Message::OpenUrl => self.open_url(),
            Message::OpenComments => self.open_comments().await,
            Message::OpenCommentsUrl => self.open_comments_url(),
            Message::ExpandComment => self.expand_comment(),
            Message::CollapseComment => self.collapse_comment(),
            Message::Back => self.go_back(),
            Message::Quit => self.should_quit = true,
            Message::Refresh => self.refresh().await,
            Message::ToggleHelp => self.show_help = !self.show_help,
            Message::SwitchFeed(feed) => self.switch_feed(feed).await,
            Message::NextFeed => self.cycle_feed(1).await,
            Message::PrevFeed => self.cycle_feed(-1).await,
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
        if let View::Comments { .. } = self.view {
            if let Some(comment) = self.selected_comment() {
                if !comment.kids.is_empty() {
                    let id = comment.id;
                    self.expanded_comments.insert(id);
                }
            }
        }
    }

    fn collapse_comment(&mut self) {
        if let View::Comments { .. } = self.view {
            let (id, depth) = match self.selected_comment() {
                Some(c) => (c.id, c.depth),
                None => return,
            };

            self.expanded_comments.remove(&id);

            // If child comment, move selection to parent
            if depth > 0 && self.selected_index > 0 {
                let visible = self.visible_comment_indices();
                for i in (0..self.selected_index).rev() {
                    if let Some(&actual_idx) = visible.get(i) {
                        if self.comments[actual_idx].depth < depth {
                            self.selected_index = i;
                            return;
                        }
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

    async fn open_comments(&mut self) {
        if let View::Stories = self.view {
            if let Some(story) = self.stories.get(self.selected_index).cloned() {
                let story_index = self.selected_index;
                let story_scroll = self.scroll_offset;

                self.view = View::Comments {
                    story_id: story.id,
                    story_title: story.title.clone(),
                    story_index,
                    story_scroll,
                };
                self.set_loading(true);
                self.comments.clear();
                self.expanded_comments.clear();
                self.selected_index = 0;
                self.scroll_offset = 0;

                match self.client.fetch_comments_flat(&story, 5).await {
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

    fn go_back(&mut self) {
        if let View::Comments { story_index, story_scroll, .. } = self.view {
            self.view = View::Stories;
            self.comments.clear();
            self.selected_index = story_index;
            self.scroll_offset = story_scroll;
        }
    }

    async fn refresh(&mut self) {
        self.client.clear_cache().await;

        match &self.view {
            View::Stories => {
                self.load_stories().await;
            }
            View::Comments { story_id, .. } => {
                let story_id = *story_id;
                if let Some(story) = self.stories.iter().find(|s| s.id == story_id).cloned() {
                    self.set_loading(true);
                    match self.client.fetch_comments_flat(&story, 5).await {
                        Ok(comments) => {
                            self.comments = comments;
                            self.set_loading(false);
                        }
                        Err(e) => {
                            self.error = Some(format!("Failed to refresh comments: {}", e));
                            self.set_loading(false);
                        }
                    }
                }
            }
        }
    }

    async fn switch_feed(&mut self, feed: Feed) {
        if self.feed != feed {
            self.feed = feed;
            self.view = View::Stories;
            self.load_stories().await;
        }
    }

    async fn cycle_feed(&mut self, direction: i32) {
        let feeds = Feed::all();
        let current_idx = feeds.iter().position(|&f| f == self.feed).unwrap_or(0);
        let new_idx = (current_idx as i32 + direction).rem_euclid(feeds.len() as i32) as usize;
        self.switch_feed(feeds[new_idx]).await;
    }

    pub async fn load_stories(&mut self) {
        self.set_loading(true);
        self.error = None;
        self.stories.clear();

        match self.client.fetch_stories(self.feed, 0).await {
            Ok(stories) => {
                self.stories = stories;
                self.set_loading(false);
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load stories: {}", e));
                self.set_loading(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{default_for_variant, ThemeVariant};

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
}
