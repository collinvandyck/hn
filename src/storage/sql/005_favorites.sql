-- Drop the separate favorites table (if it exists from prior implementation)
DROP TABLE IF EXISTS favorites;

-- Add favorited_at column to stories table
ALTER TABLE stories ADD COLUMN favorited_at INTEGER;

-- Add favorited_at column to comments table
ALTER TABLE comments ADD COLUMN favorited_at INTEGER;

-- Index for querying favorited stories ordered by favorited_at
CREATE INDEX IF NOT EXISTS idx_stories_favorited ON stories(favorited_at DESC) WHERE favorited_at IS NOT NULL;

-- Index for querying favorited comments ordered by favorited_at
CREATE INDEX IF NOT EXISTS idx_comments_favorited ON comments(favorited_at DESC) WHERE favorited_at IS NOT NULL;
