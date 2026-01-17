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

    pub async fn clear_cache(&self) {
        self.item_cache.write().await.clear();
    }

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

    async fn fetch_item(&self, id: u64) -> Result<HnItem> {
        {
            let cache = self.item_cache.read().await;
            if let Some(entry) = cache.get(&id)
                && entry.is_fresh()
            {
                return Ok(entry.data.clone());
            }
        }

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

    /// Fetches comments using BFS for parallelism, then reorders to DFS for display
    pub async fn fetch_comments_flat(
        &self,
        story: &Story,
        max_depth: usize,
    ) -> Result<Vec<Comment>> {
        let mut items: HashMap<u64, HnItem> = HashMap::new();
        let mut to_fetch: Vec<u64> = story.kids.clone();
        let mut depth = 0;

        while !to_fetch.is_empty() && depth <= max_depth {
            let futures: Vec<_> = to_fetch.iter().map(|&id| self.fetch_item(id)).collect();
            let results = futures::future::join_all(futures).await;

            let mut next_fetch = Vec::new();
            for (id, result) in to_fetch.into_iter().zip(results) {
                if let Ok(item) = result {
                    if item.deleted.unwrap_or(false) || item.dead.unwrap_or(false) {
                        continue;
                    }
                    if depth < max_depth {
                        next_fetch.extend(&item.kids);
                    }
                    items.insert(id, item);
                }
            }
            to_fetch = next_fetch;
            depth += 1;
        }

        let mut comments = Vec::new();
        let mut stack: Vec<(u64, usize)> = story.kids.iter().rev().map(|&id| (id, 0)).collect();

        while let Some((id, depth)) = stack.pop() {
            if let Some(item) = items.remove(&id) {
                // Reverse order so first child is processed first
                for &kid_id in item.kids.iter().rev() {
                    if items.contains_key(&kid_id) {
                        stack.push((kid_id, depth + 1));
                    }
                }
                if let Some(comment) = Comment::from_item(item, depth) {
                    comments.push(comment);
                }
            }
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
