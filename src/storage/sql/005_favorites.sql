CREATE TABLE IF NOT EXISTS favorites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id INTEGER NOT NULL,
    item_type TEXT NOT NULL CHECK(item_type IN ('story', 'comment')),
    favorited_at INTEGER NOT NULL,
    UNIQUE(item_id, item_type)
);

CREATE INDEX IF NOT EXISTS idx_favorites_type_time ON favorites(item_type, favorited_at DESC);
