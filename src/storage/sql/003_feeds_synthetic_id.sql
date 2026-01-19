-- Add synthetic ID to feeds table and update foreign key in feed_stories

-- Create new feeds table with synthetic ID
CREATE TABLE IF NOT EXISTS feeds_new (
    id INTEGER PRIMARY KEY,
    feed_type TEXT NOT NULL UNIQUE,
    fetched_at INTEGER NOT NULL
);

-- Create new feed_stories table with FK to feeds.id
CREATE TABLE IF NOT EXISTS feed_stories_new (
    feed_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    story_id INTEGER NOT NULL,
    PRIMARY KEY (feed_id, position),
    FOREIGN KEY (feed_id) REFERENCES feeds_new(id)
);

CREATE INDEX IF NOT EXISTS idx_feed_stories_new_story ON feed_stories_new(story_id);

-- Migrate data
INSERT INTO feeds_new (feed_type, fetched_at)
SELECT feed_type, fetched_at FROM feeds;

INSERT INTO feed_stories_new (feed_id, position, story_id)
SELECT f.id, fs.position, fs.story_id
FROM feed_stories fs
JOIN feeds_new f ON f.feed_type = fs.feed_type;

-- Drop old tables and rename
DROP TABLE IF EXISTS feed_stories;
DROP TABLE IF EXISTS feeds;

ALTER TABLE feeds_new RENAME TO feeds;
ALTER TABLE feed_stories_new RENAME TO feed_stories;

-- Recreate index with correct name
DROP INDEX IF EXISTS idx_feed_stories_new_story;
CREATE INDEX IF NOT EXISTS idx_feed_stories_story ON feed_stories(story_id);
