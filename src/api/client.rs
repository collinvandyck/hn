use std::collections::HashMap;
use std::time::Duration;

use tracing::{debug, info, instrument, warn};

use super::error::ApiError;
use super::types::{AlgoliaItem, Comment, Feed, HnItem, Story};
use crate::storage::{StorableComment, StorableStory, Storage};
use crate::time::now_unix;

const DEFAULT_FIREBASE_API: &str = "https://hacker-news.firebaseio.com/v0";
const DEFAULT_ALGOLIA_API: &str = "https://hn.algolia.com/api/v1";
const PAGE_SIZE: usize = 30;

/// Stories with their fetch timestamp from storage.
pub struct FetchedStories {
    pub stories: Vec<Story>,
    pub fetched_at: u64,
}
/// Comments with their fetch timestamp from storage.
pub struct FetchedComments {
    pub comments: Vec<Comment>,
    pub fetched_at: u64,
}

#[derive(Clone)]
pub struct HnClient {
    http: reqwest::Client,
    storage: Storage,
    firebase_api: String,
    algolia_api: String,
}

impl HnClient {
    pub fn new(storage: Storage) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            storage,
            firebase_api: DEFAULT_FIREBASE_API.to_string(),
            algolia_api: DEFAULT_ALGOLIA_API.to_string(),
        }
    }

    #[cfg(test)]
    pub fn with_api_urls(storage: Storage, firebase_api: &str, algolia_api: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            storage,
            firebase_api: firebase_api.to_string(),
            algolia_api: algolia_api.to_string(),
        }
    }

    pub const fn storage(&self) -> &Storage {
        &self.storage
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
        let endpoint = feed
            .endpoint()
            .expect("fetch_feed_ids called on feed without API endpoint");
        let url = format!("{}/{}.json", self.firebase_api, endpoint);
        self.get_json(&url).await
    }

    async fn fetch_item(&self, id: u64) -> Result<HnItem, ApiError> {
        let url = format!("{}/item/{}.json", self.firebase_api, id);
        self.get_json(&url).await
    }

    async fn fetch_algolia_item(&self, id: u64) -> Result<AlgoliaItem, ApiError> {
        let url = format!("{}/items/{}", self.algolia_api, id);
        self.get_json(&url).await
    }

    #[instrument(skip(self), fields(feed = %feed.label(), page))]
    pub async fn fetch_stories(
        &self,
        feed: Feed,
        page: usize,
        force_refresh: bool,
    ) -> Result<FetchedStories, ApiError> {
        info!("fetching stories");
        // Check for cached feed (page 0 only, unless force refresh)
        let (ids, fetched_at) = if page == 0 && !force_refresh {
            if let Ok(Some(cached)) = self.storage.get_fresh_feed(feed).await {
                info!(source = "cache", "using cached feed");
                (cached.ids, cached.fetched_at)
            } else {
                let ids = self.fetch_feed_ids(feed).await?;
                let fetched_at = now_unix();
                self.storage.save_feed(feed, &ids).await?;
                (ids, fetched_at)
            }
        } else if page == 0 {
            // Force refresh on page 0
            let ids = self.fetch_feed_ids(feed).await?;
            let fetched_at = now_unix();
            self.storage.save_feed(feed, &ids).await?;
            (ids, fetched_at)
        } else {
            // For page > 0, we need the feed IDs but timestamp comes from page 0
            // Just fetch IDs, timestamp will be ignored by caller for pagination
            (self.fetch_feed_ids(feed).await?, now_unix())
        };
        let start = page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(ids.len());
        if start >= ids.len() {
            return Ok(FetchedStories {
                stories: vec![],
                fetched_at,
            });
        }
        let page_ids = &ids[start..end];
        let stories = self.fetch_stories_by_ids(page_ids, force_refresh).await?;
        info!(count = stories.len(), "fetched stories");
        Ok(FetchedStories {
            stories,
            fetched_at,
        })
    }

    pub async fn fetch_stories_by_ids(
        &self,
        ids: &[u64],
        force_refresh: bool,
    ) -> Result<Vec<Story>, ApiError> {
        let mut stories: Vec<Story> = Vec::with_capacity(ids.len());
        let mut to_fetch = Vec::new();
        // Check storage for cached stories (unless forcing refresh)
        if force_refresh {
            to_fetch.extend_from_slice(ids);
        } else {
            for &id in ids {
                if let Ok(Some(cached)) = self.storage.get_fresh_story(id).await {
                    debug!(story_id = id, "cache hit");
                    stories.push(cached.into());
                } else {
                    debug!(story_id = id, "cache miss");
                    to_fetch.push(id);
                }
            }
        }
        // Fetch remaining from API
        if !to_fetch.is_empty() {
            let futures: Vec<_> = to_fetch.iter().map(|&id| self.fetch_item(id)).collect();
            let results = futures::future::join_all(futures).await;
            let fetched: Vec<Story> = results
                .into_iter()
                .filter_map(std::result::Result::ok)
                .filter_map(Story::from_item)
                .collect();
            // Write-through to storage, using returned row to get preserved read_at
            for story in fetched {
                let saved = self
                    .storage
                    .save_story(&StorableStory::from(&story))
                    .await?;
                stories.push(saved.into());
            }
        }
        // Re-sort by original id order
        let id_positions: HashMap<u64, usize> =
            ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        stories.sort_by_key(|s| id_positions.get(&s.id).copied().unwrap_or(usize::MAX));
        Ok(stories)
    }

    /// Fetches comments for a story, trying Algolia first then falling back to Firebase.
    #[instrument(skip(self, story), fields(story_id = story.id))]
    pub async fn fetch_comments_flat(
        &self,
        story: &Story,
        force_refresh: bool,
    ) -> Result<FetchedComments, ApiError> {
        info!("fetching comments");
        // Check storage for cached comments (unless forcing refresh)
        if !force_refresh
            && let Ok(Some((cached, fetched_at))) = self.storage.get_fresh_comments(story.id).await
        {
            info!(count = cached.len(), source = "cache", "loaded comments");
            let comments: Vec<Comment> = cached.into_iter().map(std::convert::Into::into).collect();
            return Ok(FetchedComments {
                comments: order_cached_comments(comments, &story.kids),
                fetched_at,
            });
        }
        let fetched_at = now_unix();
        // Try Algolia first (single request for all comments)
        match self.fetch_comments_algolia(story.id).await {
            Ok(comments) => {
                self.save_comments(story.id, &comments).await?;
                info!(
                    count = comments.len(),
                    source = "algolia",
                    "fetched comments"
                );
                return Ok(FetchedComments {
                    comments,
                    fetched_at,
                });
            }
            Err(e) => {
                warn!(source = "algolia", error = %e, "fetch failed, falling back to Firebase");
            }
        }
        // Fall back to Firebase (BFS, no depth limit)
        let comments = self.fetch_comments_firebase(story).await?;
        self.save_comments(story.id, &comments).await?;
        info!(
            count = comments.len(),
            source = "firebase",
            "fetched comments"
        );
        Ok(FetchedComments {
            comments,
            fetched_at,
        })
    }

    /// Fetches all comments via Algolia's single-request endpoint.
    async fn fetch_comments_algolia(&self, story_id: u64) -> Result<Vec<Comment>, ApiError> {
        let item = self.fetch_algolia_item(story_id).await?;
        Ok(flatten_algolia_tree(&item, 0))
    }

    /// Fetches comments via Firebase API (one request per comment, BFS).
    async fn fetch_comments_firebase(&self, story: &Story) -> Result<Vec<Comment>, ApiError> {
        use std::collections::HashSet;

        let mut items: HashMap<u64, HnItem> = HashMap::new();
        let mut attempted: HashSet<u64> = HashSet::new();
        let mut to_fetch: Vec<u64> = story.kids.clone();

        while !to_fetch.is_empty() {
            let futures: Vec<_> = to_fetch.iter().map(|&id| self.fetch_item(id)).collect();
            let results = futures::future::join_all(futures).await;

            let mut next_fetch = Vec::new();
            for (id, result) in to_fetch.into_iter().zip(results) {
                attempted.insert(id);
                let Ok(item) = result else { continue };
                if item.deleted.unwrap_or(false) || item.dead.unwrap_or(false) {
                    continue;
                }
                next_fetch.extend(&item.kids);
                items.insert(id, item);
            }
            to_fetch = next_fetch;
        }

        Ok(build_comment_tree(items, &attempted, &story.kids))
    }

    /// Saves comments to storage.
    async fn save_comments(&self, story_id: u64, comments: &[Comment]) -> Result<(), ApiError> {
        let storable: Vec<StorableComment> = comments
            .iter()
            .map(|c| StorableComment::from_comment(c, story_id, find_parent_id(comments, c.id)))
            .collect();
        self.storage.save_comments(story_id, &storable).await?;
        Ok(())
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
/// Takes items in a `HashMap`, traverses from `root_kids` in DFS order,
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

/// Builds a DFS-ordered comment tree from fetched `HnItems`.
///
/// Pre-filters kids that were attempted but not fetched (deleted/dead), while
/// keeping kids that were never attempted (beyond `max_depth`) so UI shows
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

/// Flattens nested Algolia response into DFS-ordered comments.
fn flatten_algolia_tree(item: &AlgoliaItem, depth: usize) -> Vec<Comment> {
    let mut comments = Vec::new();
    for child in &item.children {
        if child.item_type.as_deref() == Some("comment")
            && let Some(comment) = algolia_to_comment(child, depth)
        {
            comments.push(comment);
            comments.extend(flatten_algolia_tree(child, depth + 1));
        }
    }
    comments
}

/// Converts an Algolia item to a Comment.
fn algolia_to_comment(item: &AlgoliaItem, depth: usize) -> Option<Comment> {
    let text = item.text.as_ref()?;
    Some(Comment {
        id: item.id,
        text: html_escape::decode_html_entities(text).to_string(),
        by: item.author.clone().unwrap_or_else(|| "[deleted]".into()),
        time: item.created_at_i.unwrap_or(0),
        depth,
        kids: item.children.iter().map(|c| c.id).collect(),
        favorited_at: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Storage, StorageLocation};
    use std::collections::HashSet;

    fn test_storage() -> Storage {
        Storage::open(StorageLocation::InMemory).unwrap()
    }

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
        let client = HnClient::new(test_storage());
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

    /// Verifies that children beyond `max_depth` (never attempted) are kept in
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
    /// 1. Build tree from `HnItems` (fresh fetch path)
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
            read_at: None,
            favorited_at: None,
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
        let cached_as_comments: Vec<Comment> =
            cached.into_iter().map(std::convert::Into::into).collect();
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
            assert_eq!(fresh.id, cached.id, "ID mismatch at position {i}");
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
            read_at: None,
            favorited_at: None,
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
        let cached_as_comments: Vec<Comment> =
            cached.into_iter().map(std::convert::Into::into).collect();
        let cached_comments = order_cached_comments(cached_as_comments, &story_kids);

        // Verify DFS order: 10, 11, 20, 21, 22
        let expected_order = vec![10, 11, 20, 21, 22];
        let fresh_order: Vec<u64> = fresh_comments.iter().map(|c| c.id).collect();
        let cached_order: Vec<u64> = cached_comments.iter().map(|c| c.id).collect();

        assert_eq!(fresh_order, expected_order, "Fresh order incorrect");
        assert_eq!(cached_order, expected_order, "Cached order incorrect");
    }

    mod algolia {
        use super::*;
        use crate::api::types::AlgoliaItem;

        /// Real Algolia response for story 1 (Y Combinator).
        /// Linear nesting: story → comment 15 → comment 17 → comment 1079
        const FIXTURE_STORY_1: &str = r#"{
            "author": "pg",
            "children": [{
                "author": "sama",
                "children": [{
                    "author": "pg",
                    "children": [{
                        "author": "dmon",
                        "children": [],
                        "created_at_i": 1172441903,
                        "id": 1079,
                        "text": "sure",
                        "type": "comment"
                    }],
                    "created_at_i": 1160423565,
                    "id": 17,
                    "text": "Is there anywhere to eat on Sandhill Road?",
                    "type": "comment"
                }],
                "created_at_i": 1160423461,
                "id": 15,
                "text": "&#34;the rising star of venture capital&#34; -unknown VC eating lunch on SHR",
                "type": "comment"
            }],
            "created_at_i": 1160418111,
            "id": 1,
            "type": "story"
        }"#;

        /// Real Algolia response for story 121003 (Ask HN: The Arc Effect).
        /// Multiple top-level comments with varying nesting.
        const FIXTURE_STORY_121003: &str = r#"{
            "author": "pg",
            "children": [
                {
                    "author": "pg",
                    "children": [
                        {"author": "byrneseyeview", "children": [], "created_at_i": 1203364041, "id": 121026, "text": "neat", "type": "comment"},
                        {"author": "Tichy", "children": [], "created_at_i": 1203364696, "id": 121035, "text": "reply to pg", "type": "comment"}
                    ],
                    "created_at_i": 1203362760,
                    "id": 121016,
                    "text": "You can see two spikes in unique visitors due to Arc",
                    "type": "comment"
                },
                {
                    "author": "yters",
                    "children": [
                        {"author": "far33d", "children": [], "created_at_i": 1203395863, "id": 121171, "text": "response to yters", "type": "comment"}
                    ],
                    "created_at_i": 1203370876,
                    "id": 121109,
                    "text": "Does having 3 main hacker sites dilute quality?",
                    "type": "comment"
                },
                {
                    "author": "andreyf",
                    "children": [],
                    "created_at_i": 1203396157,
                    "id": 121168,
                    "text": "Why not just scale the weight given to a vote?",
                    "type": "comment"
                }
            ],
            "created_at_i": 1203361853,
            "id": 121003,
            "type": "story"
        }"#;

        #[test]
        fn test_flatten_linear_nesting() {
            let item: AlgoliaItem = serde_json::from_str(FIXTURE_STORY_1).unwrap();
            let comments = flatten_algolia_tree(&item, 0);

            assert_eq!(comments.len(), 3);

            // Verify DFS order and depths
            assert_eq!(comments[0].id, 15);
            assert_eq!(comments[0].depth, 0);
            assert_eq!(comments[0].by, "sama");

            assert_eq!(comments[1].id, 17);
            assert_eq!(comments[1].depth, 1);
            assert_eq!(comments[1].by, "pg");

            assert_eq!(comments[2].id, 1079);
            assert_eq!(comments[2].depth, 2);
            assert_eq!(comments[2].by, "dmon");
        }

        #[test]
        fn test_flatten_multiple_branches() {
            let item: AlgoliaItem = serde_json::from_str(FIXTURE_STORY_121003).unwrap();
            let comments = flatten_algolia_tree(&item, 0);

            // 3 top-level + 2 replies under first + 1 reply under second = 6 total
            assert_eq!(comments.len(), 6);

            // Verify DFS order: branch 1 fully explored before branch 2
            let ids: Vec<u64> = comments.iter().map(|c| c.id).collect();
            assert_eq!(ids, vec![121016, 121026, 121035, 121109, 121171, 121168]);

            // Verify depths
            assert_eq!(comments[0].depth, 0); // 121016
            assert_eq!(comments[1].depth, 1); // 121026 (child of 121016)
            assert_eq!(comments[2].depth, 1); // 121035 (child of 121016)
            assert_eq!(comments[3].depth, 0); // 121109
            assert_eq!(comments[4].depth, 1); // 121171 (child of 121109)
            assert_eq!(comments[5].depth, 0); // 121168
        }

        #[test]
        fn test_html_entity_decoding() {
            let item: AlgoliaItem = serde_json::from_str(FIXTURE_STORY_1).unwrap();
            let comments = flatten_algolia_tree(&item, 0);

            // Comment 15 has &#34; which should be decoded to "
            assert!(comments[0].text.contains('"'));
            assert!(!comments[0].text.contains("&#34;"));
        }

        #[test]
        fn test_kids_array_populated() {
            let item: AlgoliaItem = serde_json::from_str(FIXTURE_STORY_1).unwrap();
            let comments = flatten_algolia_tree(&item, 0);

            // Comment 15 has child 17
            assert_eq!(comments[0].kids, vec![17]);
            // Comment 17 has child 1079
            assert_eq!(comments[1].kids, vec![1079]);
            // Comment 1079 has no children
            assert!(comments[2].kids.is_empty());
        }

        #[test]
        fn test_missing_author_defaults_to_deleted() {
            let json = r#"{
                "children": [{
                    "children": [],
                    "created_at_i": 1234567890,
                    "id": 999,
                    "text": "orphaned comment",
                    "type": "comment"
                }],
                "id": 1,
                "type": "story"
            }"#;
            let item: AlgoliaItem = serde_json::from_str(json).unwrap();
            let comments = flatten_algolia_tree(&item, 0);

            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].by, "[deleted]");
        }

        #[test]
        fn test_skips_items_without_text() {
            let json = r#"{
                "children": [
                    {"author": "user1", "children": [], "created_at_i": 1, "id": 1, "text": "has text", "type": "comment"},
                    {"author": "user2", "children": [], "created_at_i": 2, "id": 2, "type": "comment"},
                    {"author": "user3", "children": [], "created_at_i": 3, "id": 3, "text": null, "type": "comment"}
                ],
                "id": 100,
                "type": "story"
            }"#;
            let item: AlgoliaItem = serde_json::from_str(json).unwrap();
            let comments = flatten_algolia_tree(&item, 0);

            // Only the comment with text should be included
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].id, 1);
        }

        #[test]
        fn test_skips_non_comment_types() {
            let json = r#"{
                "children": [
                    {"author": "user1", "children": [], "created_at_i": 1, "id": 1, "text": "comment", "type": "comment"},
                    {"author": "user2", "children": [], "created_at_i": 2, "id": 2, "text": "poll", "type": "pollopt"}
                ],
                "id": 100,
                "type": "story"
            }"#;
            let item: AlgoliaItem = serde_json::from_str(json).unwrap();
            let comments = flatten_algolia_tree(&item, 0);

            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].id, 1);
        }
    }

    mod fallback {
        use super::*;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        #[allow(clippy::cast_possible_truncation)] // test helper; kids.len() is small
        fn make_story(id: u64, kids: Vec<u64>) -> Story {
            Story {
                id,
                title: "Test Story".to_string(),
                url: Some("https://example.com".to_string()),
                score: 100,
                by: "testuser".to_string(),
                time: 1700000000,
                descendants: kids.len() as u32,
                kids,
                read_at: None,
                favorited_at: None,
            }
        }

        /// Verifies that when Algolia returns 503, we fall back to Firebase.
        #[tokio::test]
        async fn test_falls_back_to_firebase_on_algolia_error() {
            let algolia_server = MockServer::start().await;
            let firebase_server = MockServer::start().await;
            // Algolia returns 503
            Mock::given(method("GET"))
                .and(path("/items/999"))
                .respond_with(ResponseTemplate::new(503))
                .mount(&algolia_server)
                .await;
            // Firebase returns the comment
            Mock::given(method("GET"))
                .and(path("/item/1.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": 1,
                    "type": "comment",
                    "by": "firebase_user",
                    "time": 1700000000,
                    "text": "Comment from Firebase",
                    "kids": []
                })))
                .mount(&firebase_server)
                .await;
            let storage = test_storage();
            let story = make_story(999, vec![1]);
            // Save story first so comments can reference it
            storage
                .save_story(&StorableStory::from(&story))
                .await
                .unwrap();
            let client =
                HnClient::with_api_urls(storage, &firebase_server.uri(), &algolia_server.uri());
            let fetched = client.fetch_comments_flat(&story, false).await.unwrap();
            let comments = fetched.comments;
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].id, 1);
            assert_eq!(comments[0].by, "firebase_user");
            assert_eq!(comments[0].text, "Comment from Firebase");
        }

        /// Verifies that Algolia is used when it succeeds.
        #[tokio::test]
        async fn test_uses_algolia_when_available() {
            let algolia_server = MockServer::start().await;
            let firebase_server = MockServer::start().await;
            // Algolia returns the comment tree
            Mock::given(method("GET"))
                .and(path("/items/999"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": 999,
                    "type": "story",
                    "children": [{
                        "id": 1,
                        "type": "comment",
                        "author": "algolia_user",
                        "created_at_i": 1700000000,
                        "text": "Comment from Algolia",
                        "children": []
                    }]
                })))
                .mount(&algolia_server)
                .await;
            // Firebase should NOT be called (no mock needed, will fail if called)
            let storage = test_storage();
            let story = make_story(999, vec![1]);
            storage
                .save_story(&StorableStory::from(&story))
                .await
                .unwrap();
            let client =
                HnClient::with_api_urls(storage, &firebase_server.uri(), &algolia_server.uri());
            let fetched = client.fetch_comments_flat(&story, false).await.unwrap();
            let comments = fetched.comments;
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].id, 1);
            assert_eq!(comments[0].by, "algolia_user");
            assert_eq!(comments[0].text, "Comment from Algolia");
        }

        /// Verifies fallback handles nested Firebase comments correctly.
        #[tokio::test]
        async fn test_firebase_fallback_handles_nested_comments() {
            let algolia_server = MockServer::start().await;
            let firebase_server = MockServer::start().await;
            // Algolia returns 500
            Mock::given(method("GET"))
                .and(path("/items/999"))
                .respond_with(ResponseTemplate::new(500))
                .mount(&algolia_server)
                .await;
            // Firebase returns nested comments
            Mock::given(method("GET"))
                .and(path("/item/1.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": 1,
                    "type": "comment",
                    "by": "parent",
                    "time": 1700000000,
                    "text": "Parent comment",
                    "kids": [2]
                })))
                .mount(&firebase_server)
                .await;
            Mock::given(method("GET"))
                .and(path("/item/2.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": 2,
                    "type": "comment",
                    "by": "child",
                    "time": 1700000001,
                    "text": "Child comment",
                    "kids": []
                })))
                .mount(&firebase_server)
                .await;
            let storage = test_storage();
            let story = make_story(999, vec![1]);
            storage
                .save_story(&StorableStory::from(&story))
                .await
                .unwrap();
            let client =
                HnClient::with_api_urls(storage, &firebase_server.uri(), &algolia_server.uri());
            let fetched = client.fetch_comments_flat(&story, false).await.unwrap();
            let comments = fetched.comments;
            assert_eq!(comments.len(), 2);
            assert_eq!(comments[0].id, 1);
            assert_eq!(comments[0].depth, 0);
            assert_eq!(comments[1].id, 2);
            assert_eq!(comments[1].depth, 1);
        }
    }
}
