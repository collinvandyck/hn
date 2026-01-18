use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::error::ApiError;
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

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        let response = self.http.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::HttpStatus(
                status.as_u16(),
                status.canonical_reason().unwrap_or("").into(),
            ));
        }
        response
            .json()
            .await
            .map_err(|e| ApiError::Parse(e.to_string()))
    }

    pub async fn fetch_feed_ids(&self, feed: Feed) -> Result<Vec<u64>, ApiError> {
        let url = format!("{}/{}.json", API_BASE, feed.endpoint());
        self.get_json(&url).await
    }

    async fn fetch_item(&self, id: u64) -> Result<HnItem, ApiError> {
        {
            let cache = self.item_cache.read().await;
            if let Some(entry) = cache.get(&id)
                && entry.is_fresh()
            {
                return Ok(entry.data.clone());
            }
        }

        let url = format!("{}/item/{}.json", API_BASE, id);
        let item: HnItem = self.get_json(&url).await?;

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

    pub async fn fetch_stories(&self, feed: Feed, page: usize) -> Result<Vec<Story>, ApiError> {
        let ids = self.fetch_feed_ids(feed).await?;
        let start = page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(ids.len());

        if start >= ids.len() {
            return Ok(vec![]);
        }

        let page_ids = &ids[start..end];
        self.fetch_stories_by_ids(page_ids).await
    }

    pub async fn fetch_stories_by_ids(&self, ids: &[u64]) -> Result<Vec<Story>, ApiError> {
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
    ) -> Result<Vec<Comment>, ApiError> {
        use std::collections::HashSet;

        let mut items: HashMap<u64, HnItem> = HashMap::new();
        let mut attempted: HashSet<u64> = HashSet::new();
        let mut to_fetch: Vec<u64> = story.kids.clone();
        let mut depth = 0;

        while !to_fetch.is_empty() && depth <= max_depth {
            let futures: Vec<_> = to_fetch.iter().map(|&id| self.fetch_item(id)).collect();
            let results = futures::future::join_all(futures).await;

            let mut next_fetch = Vec::new();
            for (id, result) in to_fetch.into_iter().zip(results) {
                attempted.insert(id);
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

        Ok(build_comment_tree(items, &attempted, &story.kids))
    }
}

/// Builds a DFS-ordered comment tree from fetched items.
///
/// Filters kids that were attempted but not fetched (deleted/dead), while
/// keeping kids that were never attempted (beyond max_depth) so UI shows
/// they have replies even if we can't display them.
pub fn build_comment_tree(
    mut items: HashMap<u64, HnItem>,
    attempted: &std::collections::HashSet<u64>,
    root_kids: &[u64],
) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut stack: Vec<(u64, usize)> = root_kids.iter().rev().map(|&id| (id, 0)).collect();

    while let Some((id, depth)) = stack.pop() {
        if let Some(mut item) = items.remove(&id) {
            item.kids
                .retain(|kid_id| !attempted.contains(kid_id) || items.contains_key(kid_id));

            for &kid_id in item.kids.iter().rev() {
                stack.push((kid_id, depth + 1));
            }
            if let Some(comment) = Comment::from_item(item, depth) {
                comments.push(comment);
            }
        }
    }

    comments
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
    use std::collections::HashSet;

    fn make_comment_item(id: u64, by: &str, text: &str, kids: Vec<u64>) -> HnItem {
        HnItem {
            id,
            item_type: Some("comment".to_string()),
            by: Some(by.to_string()),
            time: Some(1700000000),
            text: Some(text.to_string()),
            url: None,
            score: None,
            title: None,
            descendants: None,
            kids,
            parent: None,
            deleted: None,
            dead: None,
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = HnClient::new();
        // Just verify it doesn't panic
        drop(client);
    }

    /// Verifies that deleted children (attempted but not fetched) are filtered
    /// out of the kids array.
    #[test]
    fn test_deleted_children_filtered_from_kids() {
        let mut items: HashMap<u64, HnItem> = HashMap::new();
        let mut attempted: HashSet<u64> = HashSet::new();

        // Parent comment with kids [2, 3] - child 3 was attempted but deleted
        items.insert(
            1,
            make_comment_item(1, "parent", "Parent comment", vec![2, 3]),
        );
        items.insert(2, make_comment_item(2, "child", "Child comment", vec![]));

        // Both children were attempted, but child 3 was deleted (not in items)
        attempted.insert(2);
        attempted.insert(3);

        let comments = build_comment_tree(items, &attempted, &[1]);

        assert_eq!(comments.len(), 2);
        let parent = &comments[0];
        assert_eq!(parent.id, 1);
        assert_eq!(parent.kids, vec![2]);
    }

    /// Verifies that a comment whose only child was deleted ends up with
    /// an empty kids array (showing [ ] instead of [+] in the UI).
    #[test]
    fn test_all_children_deleted_results_in_empty_kids() {
        let mut items: HashMap<u64, HnItem> = HashMap::new();
        let mut attempted: HashSet<u64> = HashSet::new();

        items.insert(
            1,
            make_comment_item(1, "author", "Comment with deleted reply", vec![999]),
        );

        // Child 999 was attempted but deleted (not in items)
        attempted.insert(999);

        let comments = build_comment_tree(items, &attempted, &[1]);

        assert_eq!(comments.len(), 1);
        assert!(comments[0].kids.is_empty());
    }

    /// Verifies that children beyond max_depth (never attempted) are kept in
    /// the kids array so the UI shows they have replies.
    #[test]
    fn test_children_beyond_max_depth_kept_in_kids() {
        let mut items: HashMap<u64, HnItem> = HashMap::new();
        let attempted: HashSet<u64> = HashSet::new();

        // Comment at max_depth with a child that was never fetched
        items.insert(
            1,
            make_comment_item(1, "deep_commenter", "Comment at max depth", vec![999]),
        );

        // Child 999 was NOT attempted (beyond max_depth)
        let comments = build_comment_tree(items, &attempted, &[1]);

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].kids, vec![999]);
    }
}
