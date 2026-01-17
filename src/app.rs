use crate::api::{Comment, Feed, HnClient, Story};

/// Current view in the application
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Stories,
    Comments {
        story_id: u64,
        story_title: String,
        /// Saved story view state for restoration on back
        story_index: usize,
        story_scroll: usize,
    },
}

impl Default for View {
    fn default() -> Self {
        View::Stories
    }
}

/// Messages that can update the application state
#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    SelectNext,
    SelectPrev,
    SelectFirst,
    SelectLast,
    PageDown,
    PageUp,

    // Actions
    OpenUrl,
    OpenComments,
    OpenCommentsUrl,
    Back,
    Quit,
    Refresh,
    ToggleHelp,

    // Feed switching
    SwitchFeed(Feed),
    NextFeed,
    PrevFeed,
}

/// Application state
pub struct App {
    pub view: View,
    pub feed: Feed,
    pub stories: Vec<Story>,
    pub comments: Vec<Comment>,
    pub selected_index: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub should_quit: bool,
    pub show_help: bool,
    pub client: HnClient,
    pub scroll_offset: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            view: View::default(),
            feed: Feed::default(),
            stories: Vec::new(),
            comments: Vec::new(),
            selected_index: 0,
            loading: false,
            error: None,
            should_quit: false,
            show_help: false,
            client: HnClient::new(),
            scroll_offset: 0,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the app state based on a message
    pub async fn update(&mut self, msg: Message) {
        // Clear any previous error on new actions
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
            Message::Back => self.go_back(),
            Message::Quit => self.should_quit = true,
            Message::Refresh => self.refresh().await,
            Message::ToggleHelp => self.show_help = !self.show_help,
            Message::SwitchFeed(feed) => self.switch_feed(feed).await,
            Message::NextFeed => self.cycle_feed(1).await,
            Message::PrevFeed => self.cycle_feed(-1).await,
        }
    }

    fn item_count(&self) -> usize {
        match self.view {
            View::Stories => self.stories.len(),
            View::Comments { .. } => self.comments.len(),
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
            // Use story URL if available, otherwise fall back to HN discussion page
            let url = story.url.clone().unwrap_or_else(|| {
                format!("https://news.ycombinator.com/item?id={}", story.id)
            });
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
                // Open permalink for the selected comment
                if let Some(comment) = self.comments.get(self.selected_index) {
                    let url = format!("https://news.ycombinator.com/item?id={}", comment.id);
                    let _ = open::that(url);
                }
            }
        }
    }

    async fn open_comments(&mut self) {
        if let View::Stories = self.view {
            if let Some(story) = self.stories.get(self.selected_index).cloned() {
                // Save story view state before switching
                let story_index = self.selected_index;
                let story_scroll = self.scroll_offset;

                self.view = View::Comments {
                    story_id: story.id,
                    story_title: story.title.clone(),
                    story_index,
                    story_scroll,
                };
                self.loading = true;
                self.comments.clear();
                self.selected_index = 0;
                self.scroll_offset = 0;

                match self.client.fetch_comments_flat(&story, 5).await {
                    Ok(comments) => {
                        self.comments = comments;
                        self.loading = false;
                    }
                    Err(e) => {
                        self.error = Some(format!("Failed to load comments: {}", e));
                        self.loading = false;
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
        match &self.view {
            View::Stories => {
                self.load_stories().await;
            }
            View::Comments { story_id, .. } => {
                let story_id = *story_id;
                if let Some(story) = self.stories.iter().find(|s| s.id == story_id).cloned() {
                    self.loading = true;
                    match self.client.fetch_comments_flat(&story, 5).await {
                        Ok(comments) => {
                            self.comments = comments;
                            self.loading = false;
                        }
                        Err(e) => {
                            self.error = Some(format!("Failed to refresh comments: {}", e));
                            self.loading = false;
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

    /// Load stories for the current feed
    pub async fn load_stories(&mut self) {
        self.loading = true;
        self.error = None;
        self.stories.clear();

        match self.client.fetch_stories(self.feed, 0).await {
            Ok(stories) => {
                self.stories = stories;
                self.loading = false;
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load stories: {}", e));
                self.loading = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_app() {
        let app = App::default();
        assert_eq!(app.view, View::Stories);
        assert_eq!(app.feed, Feed::Top);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_navigation() {
        let mut app = App::default();
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
        let mut app = App::default();
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
