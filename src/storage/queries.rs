use rusqlite::{Connection, params, params_from_iter};

use crate::api::Feed;
use crate::time::now_unix;

use super::StorageError;
use super::types::{CachedFeed, StorableComment, StorableStory};

fn kids_to_json(kids: &[u64]) -> String {
    serde_json::to_string(kids).unwrap_or_else(|_| "[]".to_string())
}

fn json_to_kids(json: &str) -> Vec<u64> {
    serde_json::from_str(json).unwrap_or_default()
}

pub fn save_story(conn: &Connection, story: &StorableStory) -> Result<StorableStory, StorageError> {
    // Use INSERT ... ON CONFLICT to preserve read_at, returning the saved row
    let mut stmt = conn.prepare(
        "INSERT INTO stories (id, title, url, score, by, time, descendants, kids, fetched_at, read_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            url = excluded.url,
            score = excluded.score,
            by = excluded.by,
            time = excluded.time,
            descendants = excluded.descendants,
            kids = excluded.kids,
            fetched_at = excluded.fetched_at,
            read_at = COALESCE(stories.read_at, excluded.read_at)
         RETURNING id, title, url, score, by, time, descendants, kids, fetched_at, read_at",
    )?;
    let saved = stmt.query_row(
        params![
            story.id as i64,
            story.title,
            story.url,
            story.score as i64,
            story.by,
            story.time as i64,
            story.descendants as i64,
            kids_to_json(&story.kids),
            story.fetched_at as i64,
            story.read_at.map(|t| t as i64),
        ],
        |row| {
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
            })
        },
    )?;
    Ok(saved)
}

pub fn get_story(conn: &Connection, id: u64) -> Result<Option<StorableStory>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, url, score, by, time, descendants, kids, fetched_at, read_at
         FROM stories WHERE id = ?1",
    )?;

    let result = stmt.query_row(params![id as i64], |row| {
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
        })
    });

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

    let mut stmt = tx.prepare(
        "INSERT OR REPLACE INTO comments (id, story_id, parent_id, text, by, time, depth, kids, fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
        ])?;
    }
    drop(stmt);

    let ids: Vec<i64> = comments.iter().map(|c| c.id as i64).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let delete_sql = format!(
        "DELETE FROM comments WHERE story_id = ?1 AND id NOT IN ({})",
        placeholders
    );
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
        "SELECT id, story_id, parent_id, text, by, time, depth, kids, fetched_at
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
        })
    })?;

    let mut comments = Vec::new();
    for row in rows {
        comments.push(row?);
    }
    Ok(comments)
}

fn feed_type_str(feed: Feed) -> &'static str {
    match feed {
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
        "top" => Feed::Top,
        "new" => Feed::New,
        "best" => Feed::Best,
        "ask" => Feed::Ask,
        "show" => Feed::Show,
        "jobs" => Feed::Jobs,
        _ => Feed::Top,
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
    // Get story IDs in order
    let mut stmt =
        conn.prepare("SELECT story_id FROM feed_stories WHERE feed_id = ?1 ORDER BY position")?;
    let rows = stmt.query_map(params![feed_id], |row| row.get::<_, i64>(0))?;
    let ids: Vec<u64> = rows.filter_map(|r| r.ok()).map(|id| id as u64).collect();
    if ids.is_empty() {
        return Ok(None);
    }
    Ok(Some(CachedFeed {
        feed: str_to_feed(feed_type),
        ids,
        fetched_at: fetched_at as u64,
    }))
}

pub fn mark_story_read(conn: &Connection, id: u64) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE stories SET read_at = ?1 WHERE id = ?2 AND read_at IS NULL",
        params![now_unix() as i64, id as i64],
    )?;
    Ok(())
}
