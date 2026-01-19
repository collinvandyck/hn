-- View showing how stale each feed's cache is
CREATE VIEW IF NOT EXISTS feeds_age AS
SELECT
    id,
    feed_type,
    fetched_at,
    strftime('%s', 'now') - fetched_at AS seconds_ago,
    CASE
        WHEN strftime('%s', 'now') - fetched_at < 60
            THEN (strftime('%s', 'now') - fetched_at) || 's ago'
        WHEN strftime('%s', 'now') - fetched_at < 3600
            THEN ((strftime('%s', 'now') - fetched_at) / 60) || 'm ago'
        ELSE ((strftime('%s', 'now') - fetched_at) / 3600) || 'h ago'
    END AS age
FROM feeds;
