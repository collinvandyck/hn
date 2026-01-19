mod db;
mod migrations;
mod queries;
mod types;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};

pub use types::{CachedFeed, StorableComment, StorableStory};

use crate::api::Feed;

const CACHE_TTL: Duration = Duration::from_secs(60);

pub enum StorageLocation {
    Path(PathBuf),
    #[cfg(test)]
    InMemory,
}

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    Channel(String),
    Migration { version: i64, error: String },
    NoDbPathParent,
    IO(io::Error),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Sqlite(e) => write!(f, "Database error: {}", e),
            StorageError::Channel(msg) => write!(f, "Channel error: {}", msg),
            StorageError::Migration { version, error } => {
                write!(f, "Migration {} failed: {}", version, error)
            }
            StorageError::NoDbPathParent => write!(f, "db path did not have a parent dir"),
            StorageError::IO(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        StorageError::Sqlite(e)
    }
}

impl<T> From<mpsc::error::SendError<T>> for StorageError {
    fn from(e: mpsc::error::SendError<T>) -> Self {
        StorageError::Channel(e.to_string())
    }
}

impl From<oneshot::error::RecvError> for StorageError {
    fn from(e: oneshot::error::RecvError) -> Self {
        StorageError::Channel(e.to_string())
    }
}

impl StorageError {
    #[allow(dead_code)] // Used by future features
    pub fn is_fatal(&self) -> bool {
        matches!(self, StorageError::Migration { .. })
    }
}

pub(crate) enum StorageCommand {
    SaveStory {
        story: StorableStory,
        reply: oneshot::Sender<Result<(), StorageError>>,
    },
    GetStory {
        id: u64,
        reply: oneshot::Sender<Result<Option<StorableStory>, StorageError>>,
    },
    SaveComments {
        story_id: u64,
        comments: Vec<StorableComment>,
        reply: oneshot::Sender<Result<(), StorageError>>,
    },
    GetComments {
        story_id: u64,
        reply: oneshot::Sender<Result<Vec<StorableComment>, StorageError>>,
    },
    #[allow(dead_code)] // Used by future features
    SaveFeed {
        feed: Feed,
        ids: Vec<u64>,
        reply: oneshot::Sender<Result<(), StorageError>>,
    },
    #[allow(dead_code)] // Used by future features
    GetFeed {
        feed: Feed,
        reply: oneshot::Sender<Result<Option<CachedFeed>, StorageError>>,
    },
    MarkStoryRead {
        id: u64,
        reply: oneshot::Sender<Result<(), StorageError>>,
    },
}

#[derive(Clone)]
pub struct Storage {
    cmd_tx: mpsc::Sender<StorageCommand>,
}

impl Storage {
    pub fn open(location: StorageLocation) -> Result<Self, StorageError> {
        let (cmd_tx, cmd_rx) = mpsc::channel(64);

        let conn = match location {
            StorageLocation::Path(path) => {
                let parent = path.parent().ok_or(StorageError::NoDbPathParent)?;
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(StorageError::IO)?;
                }
                Connection::open(&path)?
            }
            #[cfg(test)]
            StorageLocation::InMemory => Connection::open_in_memory()?,
        };

        db::run_migrations(&conn)?;
        std::thread::spawn(move || {
            db::run_worker(conn, cmd_rx);
        });

        Ok(Self { cmd_tx })
    }

    pub async fn save_story(&self, story: &StorableStory) -> Result<(), StorageError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(StorageCommand::SaveStory {
                story: story.clone(),
                reply: tx,
            })
            .await?;
        rx.await?
    }

    pub async fn get_story(&self, id: u64) -> Result<Option<StorableStory>, StorageError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(StorageCommand::GetStory { id, reply: tx })
            .await?;
        rx.await?
    }

    pub async fn get_fresh_story(&self, id: u64) -> Result<Option<StorableStory>, StorageError> {
        let story = self.get_story(id).await?;
        Ok(story.filter(|s| s.is_fresh(CACHE_TTL)))
    }

    pub async fn save_comments(
        &self,
        story_id: u64,
        comments: &[StorableComment],
    ) -> Result<(), StorageError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(StorageCommand::SaveComments {
                story_id,
                comments: comments.to_vec(),
                reply: tx,
            })
            .await?;
        rx.await?
    }

    pub async fn get_comments(&self, story_id: u64) -> Result<Vec<StorableComment>, StorageError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(StorageCommand::GetComments {
                story_id,
                reply: tx,
            })
            .await?;
        rx.await?
    }

    pub async fn get_fresh_comments(
        &self,
        story_id: u64,
    ) -> Result<Option<Vec<StorableComment>>, StorageError> {
        let comments = self.get_comments(story_id).await?;
        if comments.is_empty() {
            return Ok(None);
        }
        // Check if first comment is fresh (all were fetched together)
        if comments[0].is_fresh(CACHE_TTL) {
            Ok(Some(comments))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)] // Used by future features
    pub async fn save_feed(&self, feed: Feed, ids: &[u64]) -> Result<(), StorageError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(StorageCommand::SaveFeed {
                feed,
                ids: ids.to_vec(),
                reply: tx,
            })
            .await?;
        rx.await?
    }

    #[allow(dead_code)] // Used by future features
    pub async fn get_feed(&self, feed: Feed) -> Result<Option<CachedFeed>, StorageError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(StorageCommand::GetFeed { feed, reply: tx })
            .await?;
        rx.await?
    }

    #[allow(dead_code)] // Used by future features
    pub async fn get_fresh_feed(&self, feed: Feed) -> Result<Option<CachedFeed>, StorageError> {
        let cached = self.get_feed(feed).await?;
        Ok(cached.filter(|f| f.is_fresh(CACHE_TTL)))
    }

    pub async fn mark_story_read(&self, id: u64) -> Result<(), StorageError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(StorageCommand::MarkStoryRead { id, reply: tx })
            .await?;
        rx.await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_unix() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[tokio::test]
    async fn test_story_round_trip() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();

        let story = StorableStory {
            id: 123,
            title: "Test Story".to_string(),
            url: Some("https://example.com".to_string()),
            score: 100,
            by: "testuser".to_string(),
            time: 1700000000,
            descendants: 50,
            kids: vec![1, 2, 3],
            fetched_at: now_unix(),
            read_at: None,
        };

        storage.save_story(&story).await.unwrap();

        let loaded = storage.get_story(123).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, 123);
        assert_eq!(loaded.title, "Test Story");
        assert_eq!(loaded.url, Some("https://example.com".to_string()));
        assert_eq!(loaded.kids, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_story_freshness() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();

        let old_story = StorableStory {
            id: 456,
            title: "Old Story".to_string(),
            url: None,
            score: 50,
            by: "olduser".to_string(),
            time: 1700000000,
            descendants: 10,
            kids: vec![],
            fetched_at: now_unix() - 120, // 2 minutes ago
            read_at: None,
        };

        storage.save_story(&old_story).await.unwrap();

        // Regular get returns the story
        let loaded = storage.get_story(456).await.unwrap();
        assert!(loaded.is_some());

        // Fresh get returns None (story is stale)
        let fresh = storage.get_fresh_story(456).await.unwrap();
        assert!(fresh.is_none());
    }

    #[tokio::test]
    async fn test_comments_round_trip() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();

        // First save the story (comments have a foreign key to stories)
        let story = StorableStory {
            id: 123,
            title: "Test Story".to_string(),
            url: None,
            score: 100,
            by: "testuser".to_string(),
            time: 1700000000,
            descendants: 2,
            kids: vec![1001],
            fetched_at: now_unix(),
            read_at: None,
        };
        storage.save_story(&story).await.unwrap();

        let comments = vec![
            StorableComment {
                id: 1001,
                story_id: 123,
                parent_id: None,
                text: "Top level comment".to_string(),
                by: "user1".to_string(),
                time: 1700000000,
                depth: 0,
                kids: vec![1002],
                fetched_at: now_unix(),
            },
            StorableComment {
                id: 1002,
                story_id: 123,
                parent_id: Some(1001),
                text: "Reply".to_string(),
                by: "user2".to_string(),
                time: 1700000100,
                depth: 1,
                kids: vec![],
                fetched_at: now_unix(),
            },
        ];

        storage.save_comments(123, &comments).await.unwrap();

        let loaded = storage.get_comments(123).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, 1001);
        assert_eq!(loaded[0].parent_id, None);
        assert_eq!(loaded[1].id, 1002);
        assert_eq!(loaded[1].parent_id, Some(1001));
    }

    #[tokio::test]
    async fn test_feed_round_trip() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();

        let ids = vec![100, 101, 102, 103, 104];
        storage.save_feed(Feed::Top, &ids).await.unwrap();

        let loaded = storage.get_feed(Feed::Top).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.ids, vec![100, 101, 102, 103, 104]);
    }

    #[tokio::test]
    async fn test_nonexistent_story_returns_none() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();
        let loaded = storage.get_story(999999).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_comments_upsert_updates_existing() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();
        let story = StorableStory {
            id: 123,
            title: "Test".to_string(),
            url: None,
            score: 1,
            by: "u".to_string(),
            time: 1700000000,
            descendants: 1,
            kids: vec![1001],
            fetched_at: now_unix(),
            read_at: None,
        };
        storage.save_story(&story).await.unwrap();

        let v1 = vec![StorableComment {
            id: 1001,
            story_id: 123,
            parent_id: None,
            text: "Original".to_string(),
            by: "user".to_string(),
            time: 1700000000,
            depth: 0,
            kids: vec![],
            fetched_at: now_unix(),
        }];
        storage.save_comments(123, &v1).await.unwrap();

        let v2 = vec![StorableComment {
            id: 1001,
            story_id: 123,
            parent_id: None,
            text: "Updated".to_string(),
            by: "user".to_string(),
            time: 1700000000,
            depth: 0,
            kids: vec![],
            fetched_at: now_unix(),
        }];
        storage.save_comments(123, &v2).await.unwrap();

        let loaded = storage.get_comments(123).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "Updated");
    }

    #[tokio::test]
    async fn test_comments_upsert_deletes_orphans() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();
        let story = StorableStory {
            id: 123,
            title: "Test".to_string(),
            url: None,
            score: 1,
            by: "u".to_string(),
            time: 1700000000,
            descendants: 2,
            kids: vec![1001],
            fetched_at: now_unix(),
            read_at: None,
        };
        storage.save_story(&story).await.unwrap();

        let v1 = vec![
            StorableComment {
                id: 1001,
                story_id: 123,
                parent_id: None,
                text: "First".to_string(),
                by: "user".to_string(),
                time: 1700000000,
                depth: 0,
                kids: vec![],
                fetched_at: now_unix(),
            },
            StorableComment {
                id: 1002,
                story_id: 123,
                parent_id: None,
                text: "Second".to_string(),
                by: "user".to_string(),
                time: 1700000000,
                depth: 0,
                kids: vec![],
                fetched_at: now_unix(),
            },
        ];
        storage.save_comments(123, &v1).await.unwrap();

        let v2 = vec![StorableComment {
            id: 1001,
            story_id: 123,
            parent_id: None,
            text: "First".to_string(),
            by: "user".to_string(),
            time: 1700000000,
            depth: 0,
            kids: vec![],
            fetched_at: now_unix(),
        }];
        storage.save_comments(123, &v2).await.unwrap();

        let loaded = storage.get_comments(123).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, 1001);
    }

    #[tokio::test]
    async fn test_mark_story_read() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();

        let story = StorableStory {
            id: 123,
            title: "Test Story".to_string(),
            url: None,
            score: 100,
            by: "testuser".to_string(),
            time: 1700000000,
            descendants: 0,
            kids: vec![],
            fetched_at: now_unix(),
            read_at: None,
        };
        storage.save_story(&story).await.unwrap();

        // Initially not read
        let loaded = storage.get_story(123).await.unwrap().unwrap();
        assert!(loaded.read_at.is_none());

        // Mark as read
        storage.mark_story_read(123).await.unwrap();

        // Now it should have read_at set
        let loaded = storage.get_story(123).await.unwrap().unwrap();
        assert!(loaded.read_at.is_some());
    }

    #[tokio::test]
    async fn test_save_story_preserves_read_at() {
        let storage = Storage::open(StorageLocation::InMemory).unwrap();

        // Save story and mark as read
        let story = StorableStory {
            id: 456,
            title: "Original".to_string(),
            url: None,
            score: 1,
            by: "u".to_string(),
            time: 1700000000,
            descendants: 0,
            kids: vec![],
            fetched_at: now_unix(),
            read_at: None,
        };
        storage.save_story(&story).await.unwrap();
        storage.mark_story_read(456).await.unwrap();

        // Save updated version (simulating refresh from API)
        let updated = StorableStory {
            id: 456,
            title: "Updated".to_string(),
            url: None,
            score: 10,
            by: "u".to_string(),
            time: 1700000000,
            descendants: 5,
            kids: vec![],
            fetched_at: now_unix(),
            read_at: None, // API doesn't know about read_at
        };
        storage.save_story(&updated).await.unwrap();

        // read_at should be preserved
        let loaded = storage.get_story(456).await.unwrap().unwrap();
        assert!(loaded.read_at.is_some());
        assert_eq!(loaded.title, "Updated");
        assert_eq!(loaded.score, 10);
    }
}
