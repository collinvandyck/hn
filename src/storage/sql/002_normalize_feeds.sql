-- Normalize feeds table: split into feeds (metadata) and feed_stories (ordering)

-- Create new normalized tables
CREATE TABLE IF NOT EXISTS feeds_new (
    feed_type TEXT PRIMARY KEY,
    fetched_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS feed_stories (
    feed_type TEXT NOT NULL,
    position INTEGER NOT NULL,
    story_id INTEGER NOT NULL,
    PRIMARY KEY (feed_type, position),
    FOREIGN KEY (feed_type) REFERENCES feeds_new(feed_type)
);

CREATE INDEX IF NOT EXISTS idx_feed_stories_story ON feed_stories(story_id);

-- Migrate data from old feeds table
INSERT OR REPLACE INTO feeds_new (feed_type, fetched_at)
SELECT DISTINCT feed_type, fetched_at FROM feeds;

INSERT OR REPLACE INTO feed_stories (feed_type, position, story_id)
SELECT feed_type, position, story_id FROM feeds;

-- Drop old table and rename new one
DROP TABLE IF EXISTS feeds;
ALTER TABLE feeds_new RENAME TO feeds;

-- Drop old indexes that reference the old feeds table structure
DROP INDEX IF EXISTS idx_feeds_story;
DROP INDEX IF EXISTS idx_feeds_fetched;
