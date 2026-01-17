use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::types::{Comment, Feed, HnItem, Story};

const API_BASE: &str = "https://hacker-news.firebaseio.com/v0";
const CACHE_TTL: Duration = Duration::from_secs(60);
const PAGE_SIZE: usize = 30;

struct CacheEntry<T> {
    data: T,
    fetched_at: Instant,
}

impl<T> CacheEntry<T> {
    fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < CACHE_TTL
    }
}

/// Async client for the Hacker News API
pub struct HnClient {
    http: reqwest::Client,
    item_cache: Arc<RwLock<HashMap<u64, CacheEntry<HnItem>>>>,
}

impl HnClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            item_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Fetch story IDs for a feed
    pub async fn fetch_feed_ids(&self, feed: Feed) -> Result<Vec<u64>> {
        let url = format!("{}/{}.json", API_BASE, feed.endpoint());
        let ids: Vec<u64> = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch feed")?
            .json()
            .await
            .context("Failed to parse feed IDs")?;
        Ok(ids)
    }

    /// Fetch a single item by ID
    async fn fetch_item(&self, id: u64) -> Result<HnItem> {
        // Check cache first
        {
            let cache = self.item_cache.read().await;
            if let Some(entry) = cache.get(&id) {
                if entry.is_fresh() {
                    return Ok(entry.data.clone());
                }
            }
        }

        // Fetch from API
        let url = format!("{}/item/{}.json", API_BASE, id);
        let item: HnItem = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch item")?
            .json()
            .await
            .context("Failed to parse item")?;

        // Cache the result
        {
            let mut cache = self.item_cache.write().await;
            cache.insert(
                id,
                CacheEntry {
                    data: item.clone(),
                    fetched_at: Instant::now(),
                },
            );
        }

        Ok(item)
    }

    /// Fetch a page of stories from a feed
    pub async fn fetch_stories(&self, feed: Feed, page: usize) -> Result<Vec<Story>> {
        let ids = self.fetch_feed_ids(feed).await?;
        let start = page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(ids.len());

        if start >= ids.len() {
            return Ok(vec![]);
        }

        let page_ids = &ids[start..end];
        self.fetch_stories_by_ids(page_ids).await
    }

    /// Fetch stories by their IDs concurrently
    pub async fn fetch_stories_by_ids(&self, ids: &[u64]) -> Result<Vec<Story>> {
        let futures: Vec<_> = ids.iter().map(|&id| self.fetch_item(id)).collect();
        let results = futures::future::join_all(futures).await;

        let stories: Vec<Story> = results
            .into_iter()
            .filter_map(|r| r.ok())
            .filter_map(Story::from_item)
            .collect();

        Ok(stories)
    }

    /// Fetch comments for a story in flat order with depth tracking
    /// Uses parallel fetching at each depth level for performance
    pub async fn fetch_comments_flat(
        &self,
        story: &Story,
        max_depth: usize,
    ) -> Result<Vec<Comment>> {
        let mut comments = Vec::new();
        let mut current_level: Vec<(u64, usize)> =
            story.kids.iter().map(|&id| (id, 0)).collect();

        while !current_level.is_empty() {
            // Fetch all comments at current level in parallel
            let futures: Vec<_> = current_level
                .iter()
                .map(|&(id, _)| self.fetch_item(id))
                .collect();
            let results = futures::future::join_all(futures).await;

            // Process results and collect next level's IDs
            let mut next_level = Vec::new();
            for ((_, depth), result) in current_level.into_iter().zip(results) {
                if let Ok(item) = result {
                    if let Some(comment) = Comment::from_item(item.clone(), depth) {
                        // Collect children for next level if within depth limit
                        if depth < max_depth {
                            for &kid_id in &item.kids {
                                next_level.push((kid_id, depth + 1));
                            }
                        }
                        comments.push(comment);
                    }
                }
            }
            current_level = next_level;
        }

        Ok(comments)
    }
}

impl Default for HnClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for HnClient {
    fn clone(&self) -> Self {
        Self {
            http: self.http.clone(),
            item_cache: Arc::clone(&self.item_cache),
        }
    }
}

// Implement Clone for HnItem to enable caching
impl Clone for HnItem {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            item_type: self.item_type.clone(),
            by: self.by.clone(),
            time: self.time,
            text: self.text.clone(),
            url: self.url.clone(),
            score: self.score,
            title: self.title.clone(),
            descendants: self.descendants,
            kids: self.kids.clone(),
            parent: self.parent,
            deleted: self.deleted,
            dead: self.dead,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = HnClient::new();
        // Just verify it doesn't panic
        drop(client);
    }
}
