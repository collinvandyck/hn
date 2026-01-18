use std::collections::HashMap;
use std::time::Duration;

use tracing::{debug, info, instrument, warn};

use super::error::ApiError;
use super::types::{Comment, Feed, HnItem, Story};
use crate::storage::{StorableComment, StorableStory, Storage};

const API_BASE: &str = "https://hacker-news.firebaseio.com/v0";
const PAGE_SIZE: usize = 30;

pub struct HnClient {
    http: reqwest::Client,
    storage: Option<Storage>,
}

impl HnClient {
    pub fn new(storage: Option<Storage>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            storage,
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        let response = self.http.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            warn!(status = %status, url, "http error");
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
        let url = format!("{}/item/{}.json", API_BASE, id);
        self.get_json(&url).await
    }

    #[instrument(skip(self), fields(feed = %feed.label(), page))]
    pub async fn fetch_stories(
        &self,
        feed: Feed,
        page: usize,
        force_refresh: bool,
    ) -> Result<Vec<Story>, ApiError> {
        info!("fetching stories");
        let ids = self.fetch_feed_ids(feed).await?;
        let start = page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(ids.len());

        if start >= ids.len() {
            return Ok(vec![]);
        }

        let page_ids = &ids[start..end];
        let stories = self.fetch_stories_by_ids(page_ids, force_refresh).await?;
        info!(count = stories.len(), "fetched stories");
        Ok(stories)
    }

    pub async fn fetch_stories_by_ids(
        &self,
        ids: &[u64],
        force_refresh: bool,
    ) -> Result<Vec<Story>, ApiError> {
        let mut stories = Vec::with_capacity(ids.len());
        let mut to_fetch = Vec::new();

        // Check storage for cached stories (unless forcing refresh)
        if !force_refresh {
            if let Some(storage) = &self.storage {
                for &id in ids {
                    if let Ok(Some(cached)) = storage.get_fresh_story(id).await {
                        debug!(story_id = id, "cache hit");
                        stories.push(cached.into());
                    } else {
                        debug!(story_id = id, "cache miss");
                        to_fetch.push(id);
                    }
                }
            } else {
                to_fetch.extend_from_slice(ids);
            }
        } else {
            to_fetch.extend_from_slice(ids);
        }

        // Fetch remaining from API
        if !to_fetch.is_empty() {
            let futures: Vec<_> = to_fetch.iter().map(|&id| self.fetch_item(id)).collect();
            let results = futures::future::join_all(futures).await;

            let fetched: Vec<Story> = results
                .into_iter()
                .filter_map(|r| r.ok())
                .filter_map(Story::from_item)
                .collect();

            // Write-through to storage
            if let Some(storage) = &self.storage {
                for story in &fetched {
                    storage.save_story(&StorableStory::from(story)).await?;
                }
            }

            stories.extend(fetched);
        }

        // Re-sort by original id order
        let id_positions: HashMap<u64, usize> =
            ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        stories.sort_by_key(|s| id_positions.get(&s.id).copied().unwrap_or(usize::MAX));

        Ok(stories)
    }

    /// Fetches comments using BFS for parallelism, then reorders to DFS for display
    #[instrument(skip(self, story), fields(story_id = story.id, max_depth))]
    pub async fn fetch_comments_flat(
        &self,
        story: &Story,
        max_depth: usize,
        force_refresh: bool,
    ) -> Result<Vec<Comment>, ApiError> {
        use std::collections::HashSet;

        info!("fetching comments");

        // Check storage for cached comments (unless forcing refresh)
        if !force_refresh
            && let Some(storage) = &self.storage
            && let Ok(Some(cached)) = storage.get_fresh_comments(story.id).await
        {
            info!(count = cached.len(), "comments cache hit");
            let comments: Vec<Comment> = cached.into_iter().map(|c| c.into()).collect();
            return Ok(order_cached_comments(comments, &story.kids));
        }

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

        let comments = build_comment_tree(items, &attempted, &story.kids);

        // Write-through to storage
        if let Some(storage) = &self.storage {
            let storable: Vec<StorableComment> = comments
                .iter()
                .map(|c| {
                    StorableComment::from_comment(c, story.id, find_parent_id(&comments, c.id))
                })
                .collect();
            storage.save_comments(story.id, &storable).await?;
        }

        info!(count = comments.len(), "fetched comments");
        Ok(comments)
    }
}

fn find_parent_id(comments: &[Comment], comment_id: u64) -> Option<u64> {
    comments
        .iter()
        .find(|c| c.kids.contains(&comment_id))
        .map(|c| c.id)
}

/// Core DFS tree builder - the single implementation for ordering comments.
///
/// Takes items in a HashMap, traverses from root_kids in DFS order,
/// and converts each item to a Comment using the provided closure.
fn build_tree<T, K, F>(
    mut items: HashMap<u64, T>,
    root_kids: &[u64],
    get_kids: K,
    mut to_comment: F,
) -> Vec<Comment>
where
    K: Fn(&T) -> &[u64],
    F: FnMut(T, usize) -> Option<Comment>,
{
    let mut result = Vec::new();
    let mut stack: Vec<(u64, usize)> = root_kids.iter().rev().map(|&id| (id, 0)).collect();

    while let Some((id, depth)) = stack.pop() {
        if let Some(item) = items.remove(&id) {
            for &kid_id in get_kids(&item).iter().rev() {
                stack.push((kid_id, depth + 1));
            }
            if let Some(comment) = to_comment(item, depth) {
                result.push(comment);
            }
        }
    }

    result
}

/// Builds a DFS-ordered comment tree from fetched HnItems.
///
/// Pre-filters kids that were attempted but not fetched (deleted/dead), while
/// keeping kids that were never attempted (beyond max_depth) so UI shows
/// they have replies even if we can't display them.
pub fn build_comment_tree(
    mut items: HashMap<u64, HnItem>,
    attempted: &std::collections::HashSet<u64>,
    root_kids: &[u64],
) -> Vec<Comment> {
    // Pre-filter kids: remove attempted-but-missing (deleted/dead)
    // Build set of present IDs first to avoid borrow conflict
    let present: std::collections::HashSet<u64> = items.keys().copied().collect();
    for item in items.values_mut() {
        item.kids
            .retain(|kid_id| !attempted.contains(kid_id) || present.contains(kid_id));
    }

    build_tree(items, root_kids, |item| &item.kids, Comment::from_item)
}

/// Orders cached comments into DFS tree order using stored kids arrays.
fn order_cached_comments(cached: Vec<Comment>, root_kids: &[u64]) -> Vec<Comment> {
    let by_id: HashMap<u64, Comment> = cached.into_iter().map(|c| (c.id, c)).collect();

    build_tree(by_id, root_kids, |c| &c.kids, |c, _depth| Some(c))
}

impl Default for HnClient {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Clone for HnClient {
    fn clone(&self) -> Self {
        Self {
            http: self.http.clone(),
            storage: self.storage.clone(),
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
        let client = HnClient::new(None);
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

    /// Verifies that fresh fetch and cached load produce identical tree ordering.
    ///
    /// This tests the full round-trip:
    /// 1. Build tree from HnItems (fresh fetch path)
    /// 2. Save to storage
    /// 3. Load from storage and rebuild tree (cached path)
    /// 4. Assert both produce identical results
    #[tokio::test]
    async fn test_cached_comments_match_fresh_tree_order() {
        use crate::storage::{StorableStory, Storage, StorageLocation};

        // Build a complex tree structure:
        //   1 (root)
        //   ├── 2
        //   │   ├── 4
        //   │   └── 5
        //   └── 3
        //       └── 6
        //           └── 7
        let mut items: HashMap<u64, HnItem> = HashMap::new();
        items.insert(
            1,
            make_comment_item(1, "user1", "Root comment 1", vec![2, 3]),
        );
        items.insert(2, make_comment_item(2, "user2", "Child of 1", vec![4, 5]));
        items.insert(3, make_comment_item(3, "user3", "Child of 1", vec![6]));
        items.insert(4, make_comment_item(4, "user4", "Child of 2", vec![]));
        items.insert(5, make_comment_item(5, "user5", "Child of 2", vec![]));
        items.insert(6, make_comment_item(6, "user6", "Child of 3", vec![7]));
        items.insert(7, make_comment_item(7, "user7", "Child of 6", vec![]));

        let story_kids = vec![1];
        let attempted: HashSet<u64> = items.keys().copied().collect();

        // Fresh path: build tree from HnItems
        let fresh_comments = build_comment_tree(items, &attempted, &story_kids);

        // Save to storage (simulating cache write)
        let storage = Storage::open(StorageLocation::InMemory).unwrap();
        let story_id = 12345u64;

        // Must save story first (foreign key constraint)
        let story = StorableStory {
            id: story_id,
            title: "Test".to_string(),
            url: None,
            score: 1,
            by: "user".to_string(),
            time: 1700000000,
            descendants: 7,
            kids: story_kids.clone(),
            fetched_at: 1700000000,
        };
        storage.save_story(&story).await.unwrap();

        let storable: Vec<StorableComment> = fresh_comments
            .iter()
            .map(|c| {
                StorableComment::from_comment(c, story_id, find_parent_id(&fresh_comments, c.id))
            })
            .collect();
        storage.save_comments(story_id, &storable).await.unwrap();

        // Cached path: load from storage and rebuild tree
        let cached = storage.get_comments(story_id).await.unwrap();
        let cached_as_comments: Vec<Comment> = cached.into_iter().map(|c| c.into()).collect();
        let cached_comments = order_cached_comments(cached_as_comments, &story_kids);

        // Both paths must produce identical results
        assert_eq!(
            fresh_comments.len(),
            cached_comments.len(),
            "Comment count mismatch"
        );

        for (i, (fresh, cached)) in fresh_comments
            .iter()
            .zip(cached_comments.iter())
            .enumerate()
        {
            assert_eq!(fresh.id, cached.id, "ID mismatch at position {}", i);
            assert_eq!(
                fresh.depth, cached.depth,
                "Depth mismatch at position {} (id={})",
                i, fresh.id
            );
            assert_eq!(
                fresh.kids, cached.kids,
                "Kids mismatch at position {} (id={})",
                i, fresh.id
            );
        }
    }

    /// Verifies tree ordering with multiple root comments.
    #[tokio::test]
    async fn test_cached_comments_multiple_roots() {
        use crate::storage::{StorableStory, Storage, StorageLocation};

        // Two separate root comment threads
        //   10 (root 1)
        //   └── 11
        //   20 (root 2)
        //   └── 21
        //       └── 22
        let mut items: HashMap<u64, HnItem> = HashMap::new();
        items.insert(10, make_comment_item(10, "a", "Root 1", vec![11]));
        items.insert(11, make_comment_item(11, "b", "Child of 10", vec![]));
        items.insert(20, make_comment_item(20, "c", "Root 2", vec![21]));
        items.insert(21, make_comment_item(21, "d", "Child of 20", vec![22]));
        items.insert(22, make_comment_item(22, "e", "Child of 21", vec![]));

        let story_kids = vec![10, 20];
        let attempted: HashSet<u64> = items.keys().copied().collect();

        let fresh_comments = build_comment_tree(items, &attempted, &story_kids);

        let storage = Storage::open(StorageLocation::InMemory).unwrap();
        let story_id = 99999u64;

        // Must save story first (foreign key constraint)
        let story = StorableStory {
            id: story_id,
            title: "Test".to_string(),
            url: None,
            score: 1,
            by: "user".to_string(),
            time: 1700000000,
            descendants: 5,
            kids: story_kids.clone(),
            fetched_at: 1700000000,
        };
        storage.save_story(&story).await.unwrap();

        let storable = fresh_comments
            .iter()
            .map(|c| {
                StorableComment::from_comment(c, story_id, find_parent_id(&fresh_comments, c.id))
            })
            .collect::<Vec<_>>();
        storage.save_comments(story_id, &storable).await.unwrap();

        let cached = storage.get_comments(story_id).await.unwrap();
        let cached_as_comments: Vec<Comment> = cached.into_iter().map(|c| c.into()).collect();
        let cached_comments = order_cached_comments(cached_as_comments, &story_kids);

        // Verify DFS order: 10, 11, 20, 21, 22
        let expected_order = vec![10, 11, 20, 21, 22];
        let fresh_order: Vec<u64> = fresh_comments.iter().map(|c| c.id).collect();
        let cached_order: Vec<u64> = cached_comments.iter().map(|c| c.id).collect();

        assert_eq!(fresh_order, expected_order, "Fresh order incorrect");
        assert_eq!(cached_order, expected_order, "Cached order incorrect");
    }
}
