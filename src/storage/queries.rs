#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
// SQLite uses i64 for all integers; HN IDs and timestamps are u64 but well within i64 range

use rusqlite::{Connection, params, params_from_iter};

use crate::api::Feed;
use crate::time::now_unix;

use super::StorageError;
use super::types::{CachedFeed, StorableComment, StorableStory, StorySort};

fn kids_to_json(kids: &[u64]) -> String {
    serde_json::to_string(kids).unwrap_or_else(|_| "[]".to_string())
}

fn json_to_kids(json: &str) -> Vec<u64> {
    serde_json::from_str(json).unwrap_or_default()
}

const fn sort_to_str(sort: StorySort) -> &'static str {
    match sort {
        StorySort::Position => "position",
        StorySort::ScoreDesc => "score",
        StorySort::CommentsDesc => "comments",
        StorySort::TimeDesc => "time",
    }
}

fn str_to_sort(s: &str) -> StorySort {
    match s {
        "score" => StorySort::ScoreDesc,
        "comments" => StorySort::CommentsDesc,
        "time" => StorySort::TimeDesc,
        _ => StorySort::Position,
    }
}

fn story_from_row(row: &rusqlite::Row) -> rusqlite::Result<StorableStory> {
    let kids_json: String = row.get(7)?;
    Ok(StorableStory {
        id: row.get::<_, i64>(0)? as u64,
        title: row.get(1)?,
        url: row.get(2)?,
        score: row.get::<_, i64>(3)? as u32,
        by: row.get(4)?,
        time: row.get::<_, i64>(5)? as u64,
        descendants: row.get::<_, i64>(6)? as u32,
        kids: json_to_kids(&kids_json),
        fetched_at: row.get::<_, i64>(8)? as u64,
        read_at: row.get::<_, Option<i64>>(9)?.map(|t| t as u64),
        favorited_at: row.get::<_, Option<i64>>(10)?.map(|t| t as u64),
    })
}

pub fn save_story(conn: &Connection, story: &StorableStory) -> Result<StorableStory, StorageError> {
    // Use INSERT ... ON CONFLICT to preserve read_at and favorited_at, returning the saved row
    let mut stmt = conn.prepare(
        "INSERT INTO stories (id, title, url, score, by, time, descendants, kids, fetched_at, read_at, favorited_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            url = excluded.url,
            score = excluded.score,
            by = excluded.by,
            time = excluded.time,
            descendants = excluded.descendants,
            kids = excluded.kids,
            fetched_at = excluded.fetched_at,
            read_at = COALESCE(stories.read_at, excluded.read_at),
            favorited_at = COALESCE(stories.favorited_at, excluded.favorited_at)
         RETURNING id, title, url, score, by, time, descendants, kids, fetched_at, read_at, favorited_at",
    )?;
    let saved = stmt.query_row(
        params![
            story.id as i64,
            story.title,
            story.url,
            i64::from(story.score),
            story.by,
            story.time as i64,
            i64::from(story.descendants),
            kids_to_json(&story.kids),
            story.fetched_at as i64,
            story.read_at.map(|t| t as i64),
            story.favorited_at.map(|t| t as i64),
        ],
        story_from_row,
    )?;
    Ok(saved)
}

pub fn get_story(conn: &Connection, id: u64) -> Result<Option<StorableStory>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, url, score, by, time, descendants, kids, fetched_at, read_at, favorited_at
         FROM stories WHERE id = ?1",
    )?;

    let result = stmt.query_row(params![id as i64], story_from_row);

    match result {
        Ok(story) => Ok(Some(story)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn save_comments(
    conn: &Connection,
    story_id: u64,
    comments: &[StorableComment],
) -> Result<(), StorageError> {
    if comments.is_empty() {
        conn.execute(
            "DELETE FROM comments WHERE story_id = ?1",
            params![story_id as i64],
        )?;
        return Ok(());
    }

    let tx = conn.unchecked_transaction()?;

    // Use INSERT ... ON CONFLICT to preserve favorited_at
    let mut stmt = tx.prepare(
        "INSERT INTO comments (id, story_id, parent_id, text, by, time, depth, kids, fetched_at, favorited_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
            story_id = excluded.story_id,
            parent_id = excluded.parent_id,
            text = excluded.text,
            by = excluded.by,
            time = excluded.time,
            depth = excluded.depth,
            kids = excluded.kids,
            fetched_at = excluded.fetched_at,
            favorited_at = COALESCE(comments.favorited_at, excluded.favorited_at)",
    )?;
    for comment in comments {
        stmt.execute(params![
            comment.id as i64,
            comment.story_id as i64,
            comment.parent_id.map(|id| id as i64),
            comment.text,
            comment.by,
            comment.time as i64,
            comment.depth as i64,
            kids_to_json(&comment.kids),
            comment.fetched_at as i64,
            comment.favorited_at.map(|t| t as i64),
        ])?;
    }
    drop(stmt);

    let ids: Vec<i64> = comments.iter().map(|c| c.id as i64).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let delete_sql =
        format!("DELETE FROM comments WHERE story_id = ?1 AND id NOT IN ({placeholders})");
    tx.execute(
        &delete_sql,
        params_from_iter(std::iter::once(story_id as i64).chain(ids)),
    )?;

    tx.commit()?;
    Ok(())
}

pub fn get_comments(
    conn: &Connection,
    story_id: u64,
) -> Result<Vec<StorableComment>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, story_id, parent_id, text, by, time, depth, kids, fetched_at, favorited_at
         FROM comments WHERE story_id = ?1",
    )?;

    let rows = stmt.query_map(params![story_id as i64], |row| {
        let kids_json: String = row.get(7)?;
        Ok(StorableComment {
            id: row.get::<_, i64>(0)? as u64,
            story_id: row.get::<_, i64>(1)? as u64,
            parent_id: row.get::<_, Option<i64>>(2)?.map(|id| id as u64),
            text: row.get(3)?,
            by: row.get(4)?,
            time: row.get::<_, i64>(5)? as u64,
            depth: row.get::<_, i64>(6)? as usize,
            kids: json_to_kids(&kids_json),
            fetched_at: row.get::<_, i64>(8)? as u64,
            favorited_at: row.get::<_, Option<i64>>(9)?.map(|t| t as u64),
        })
    })?;

    let mut comments = Vec::new();
    for row in rows {
        comments.push(row?);
    }
    Ok(comments)
}

const fn feed_type_str(feed: Feed) -> &'static str {
    match feed {
        Feed::Favorites => "favorites",
        Feed::Top => "top",
        Feed::New => "new",
        Feed::Best => "best",
        Feed::Ask => "ask",
        Feed::Show => "show",
        Feed::Jobs => "jobs",
    }
}

fn str_to_feed(s: &str) -> Feed {
    match s {
        "favorites" => Feed::Favorites,
        "new" => Feed::New,
        "best" => Feed::Best,
        "ask" => Feed::Ask,
        "show" => Feed::Show,
        "jobs" => Feed::Jobs,
        _ => Feed::Top, // "top" or unknown
    }
}

pub fn save_feed(conn: &Connection, feed: Feed, ids: &[u64]) -> Result<(), StorageError> {
    let feed_type = feed_type_str(feed);
    let now = now_unix() as i64;
    let tx = conn.unchecked_transaction()?;
    // Upsert feed metadata and get the ID
    tx.execute(
        "INSERT INTO feeds (feed_type, fetched_at) VALUES (?1, ?2)
         ON CONFLICT(feed_type) DO UPDATE SET fetched_at = excluded.fetched_at",
        params![feed_type, now],
    )?;
    let feed_id: i64 = tx.query_row(
        "SELECT id FROM feeds WHERE feed_type = ?1",
        params![feed_type],
        |row| row.get(0),
    )?;
    // Clear existing feed_stories for this feed
    tx.execute(
        "DELETE FROM feed_stories WHERE feed_id = ?1",
        params![feed_id],
    )?;
    if !ids.is_empty() {
        let mut stmt = tx.prepare(
            "INSERT INTO feed_stories (feed_id, position, story_id) VALUES (?1, ?2, ?3)",
        )?;
        for (position, &story_id) in ids.iter().enumerate() {
            stmt.execute(params![feed_id, position as i64, story_id as i64])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn get_feed(conn: &Connection, feed: Feed) -> Result<Option<CachedFeed>, StorageError> {
    let feed_type = feed_type_str(feed);
    // Get feed metadata including sort
    let row: Option<(i64, i64, String)> = conn
        .query_row(
            "SELECT id, fetched_at, sort FROM feeds WHERE feed_type = ?1",
            params![feed_type],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();
    let Some((feed_id, fetched_at, sort_str)) = row else {
        return Ok(None);
    };
    // Get story IDs in order
    let mut stmt =
        conn.prepare("SELECT story_id FROM feed_stories WHERE feed_id = ?1 ORDER BY position")?;
    let rows = stmt.query_map(params![feed_id], |row| row.get::<_, i64>(0))?;
    let ids: Vec<u64> = rows
        .filter_map(std::result::Result::ok)
        .map(|id| id as u64)
        .collect();
    if ids.is_empty() {
        return Ok(None);
    }
    Ok(Some(CachedFeed {
        feed: str_to_feed(feed_type),
        ids,
        fetched_at: fetched_at as u64,
        sort: str_to_sort(&sort_str),
    }))
}

/// Update the sort preference for a feed. Creates the feed record if it doesn't exist.
pub fn set_feed_sort(conn: &Connection, feed: Feed, sort: StorySort) -> Result<(), StorageError> {
    let feed_type = feed_type_str(feed);
    conn.execute(
        "INSERT INTO feeds (feed_type, fetched_at, sort) VALUES (?1, 0, ?2)
         ON CONFLICT(feed_type) DO UPDATE SET sort = excluded.sort",
        params![feed_type, sort_to_str(sort)],
    )?;
    Ok(())
}

/// Get just the sort preference for a feed, without loading story IDs.
pub fn get_feed_sort(conn: &Connection, feed: Feed) -> Option<StorySort> {
    let feed_type = feed_type_str(feed);
    conn.query_row(
        "SELECT sort FROM feeds WHERE feed_type = ?1",
        params![feed_type],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .map(|s| str_to_sort(&s))
}

pub fn mark_story_read(conn: &Connection, id: u64) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE stories SET read_at = ?1 WHERE id = ?2 AND read_at IS NULL",
        params![now_unix() as i64, id as i64],
    )?;
    Ok(())
}

/// Toggle favorite status for a story. Returns the new `favorited_at` value (Some if favorited, None if unfavorited).
pub fn toggle_story_favorite(conn: &Connection, id: u64) -> Result<Option<u64>, StorageError> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT favorited_at FROM stories WHERE id = ?1",
            params![id as i64],
            |row| row.get(0),
        )
        .ok()
        .flatten();
    if existing.is_some() {
        conn.execute(
            "UPDATE stories SET favorited_at = NULL WHERE id = ?1",
            params![id as i64],
        )?;
        Ok(None)
    } else {
        let now = now_unix();
        conn.execute(
            "UPDATE stories SET favorited_at = ?1 WHERE id = ?2",
            params![now as i64, id as i64],
        )?;
        Ok(Some(now))
    }
}

/// Toggle favorite status for a comment. Returns the new `favorited_at` value (Some if favorited, None if unfavorited).
pub fn toggle_comment_favorite(conn: &Connection, id: u64) -> Result<Option<u64>, StorageError> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT favorited_at FROM comments WHERE id = ?1",
            params![id as i64],
            |row| row.get(0),
        )
        .ok()
        .flatten();
    if existing.is_some() {
        conn.execute(
            "UPDATE comments SET favorited_at = NULL WHERE id = ?1",
            params![id as i64],
        )?;
        Ok(None)
    } else {
        let now = now_unix();
        conn.execute(
            "UPDATE comments SET favorited_at = ?1 WHERE id = ?2",
            params![now as i64, id as i64],
        )?;
        Ok(Some(now))
    }
}

/// Get all favorited stories with optional sorting.
pub fn get_favorited_stories_sorted(
    conn: &Connection,
    sort: StorySort,
) -> Result<Vec<StorableStory>, StorageError> {
    let order_clause = match sort {
        StorySort::Position => "favorited_at DESC", // Default: most recently favorited first
        StorySort::ScoreDesc => "score DESC, favorited_at DESC",
        StorySort::CommentsDesc => "descendants DESC, favorited_at DESC",
        StorySort::TimeDesc => "time DESC, favorited_at DESC",
    };
    let sql = format!(
        "SELECT id, title, url, score, by, time, descendants, kids, fetched_at, read_at, favorited_at
         FROM stories WHERE favorited_at IS NOT NULL ORDER BY {order_clause}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], story_from_row)?;
    let mut stories = Vec::new();
    for row in rows {
        stories.push(row?);
    }
    Ok(stories)
}

/// Get all favorited stories, ordered by most recently favorited first.
pub fn get_favorited_stories(conn: &Connection) -> Result<Vec<StorableStory>, StorageError> {
    get_favorited_stories_sorted(conn, StorySort::Position)
}

/// Result from [`get_feed_stories_sorted`].
pub struct SortedFeedResult {
    pub stories: Vec<StorableStory>,
    pub fetched_at: u64,
}

/// Get stories for a feed with sorting applied at the DB level.
pub fn get_feed_stories_sorted(
    conn: &Connection,
    feed: Feed,
    sort: StorySort,
) -> Result<Option<SortedFeedResult>, StorageError> {
    let feed_type = feed_type_str(feed);
    // Get feed metadata
    let row: Option<(i64, i64)> = conn
        .query_row(
            "SELECT id, fetched_at FROM feeds WHERE feed_type = ?1",
            params![feed_type],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    let Some((feed_id, fetched_at)) = row else {
        return Ok(None);
    };
    // Build ORDER BY clause based on sort
    let order_clause = match sort {
        StorySort::Position => "fs.position ASC",
        StorySort::ScoreDesc => "s.score DESC, fs.position ASC",
        StorySort::CommentsDesc => "s.descendants DESC, fs.position ASC",
        StorySort::TimeDesc => "s.time DESC, fs.position ASC",
    };
    let sql = format!(
        "SELECT s.id, s.title, s.url, s.score, s.by, s.time, s.descendants, s.kids,
                s.fetched_at, s.read_at, s.favorited_at
         FROM feed_stories fs
         JOIN stories s ON fs.story_id = s.id
         WHERE fs.feed_id = ?1
         ORDER BY {order_clause}"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![feed_id], story_from_row)?;
    let mut stories = Vec::new();
    for row in rows {
        stories.push(row?);
    }
    if stories.is_empty() {
        return Ok(None);
    }
    Ok(Some(SortedFeedResult {
        stories,
        fetched_at: fetched_at as u64,
    }))
}
