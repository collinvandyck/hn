use std::time::Duration;

use crate::api::{Comment, Feed, Story};
use crate::time::now_unix;

#[derive(Debug, Clone)]
pub struct StorableStory {
    pub id: u64,
    pub title: String,
    pub url: Option<String>,
    pub score: u32,
    pub by: String,
    pub time: u64,
    pub descendants: u32,
    pub kids: Vec<u64>,
    pub fetched_at: u64,
    pub read_at: Option<u64>,
}

impl StorableStory {
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        let now = now_unix();
        now.saturating_sub(self.fetched_at) < ttl.as_secs()
    }
}

impl From<&Story> for StorableStory {
    fn from(story: &Story) -> Self {
        StorableStory {
            id: story.id,
            title: story.title.clone(),
            url: story.url.clone(),
            score: story.score,
            by: story.by.clone(),
            time: story.time,
            descendants: story.descendants,
            kids: story.kids.clone(),
            fetched_at: now_unix(),
            read_at: story.read_at,
        }
    }
}

impl From<StorableStory> for Story {
    fn from(stored: StorableStory) -> Self {
        Story {
            id: stored.id,
            title: stored.title,
            url: stored.url,
            score: stored.score,
            by: stored.by,
            time: stored.time,
            descendants: stored.descendants,
            kids: stored.kids,
            read_at: stored.read_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorableComment {
    pub id: u64,
    pub story_id: u64,
    pub parent_id: Option<u64>,
    pub text: String,
    pub by: String,
    pub time: u64,
    pub depth: usize,
    pub kids: Vec<u64>,
    pub fetched_at: u64,
}

impl StorableComment {
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        let now = now_unix();
        now.saturating_sub(self.fetched_at) < ttl.as_secs()
    }

    pub fn from_comment(comment: &Comment, story_id: u64, parent_id: Option<u64>) -> Self {
        StorableComment {
            id: comment.id,
            story_id,
            parent_id,
            text: comment.text.clone(),
            by: comment.by.clone(),
            time: comment.time,
            depth: comment.depth,
            kids: comment.kids.clone(),
            fetched_at: now_unix(),
        }
    }
}

impl From<StorableComment> for Comment {
    fn from(stored: StorableComment) -> Self {
        Comment {
            id: stored.id,
            text: stored.text,
            by: stored.by,
            time: stored.time,
            depth: stored.depth,
            kids: stored.kids,
        }
    }
}

#[allow(dead_code)] // Used by future features
#[derive(Debug, Clone)]
pub struct CachedFeed {
    pub feed: Feed,
    pub ids: Vec<u64>,
    pub fetched_at: u64,
}

#[allow(dead_code)] // Used by future features
impl CachedFeed {
    pub fn new(feed: Feed, ids: Vec<u64>) -> Self {
        CachedFeed {
            feed,
            ids,
            fetched_at: now_unix(),
        }
    }

    pub fn is_fresh(&self, ttl: Duration) -> bool {
        let now = now_unix();
        now.saturating_sub(self.fetched_at) < ttl.as_secs()
    }
}
