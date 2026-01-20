#![allow(clippy::cast_possible_wrap)]
// SQLite uses i64; timestamps are u64 but well within i64 range

use rusqlite::Connection;

use super::StorageError;
use crate::time::now_unix;

struct Migration {
    version: i64,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("sql/001_initial.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("sql/002_normalize_feeds.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("sql/003_feeds_synthetic_id.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("sql/004_feeds_age_view.sql"),
    },
    Migration {
        version: 5,
        sql: include_str!("sql/005_favorites.sql"),
    },
];

pub fn run_migrations(conn: &Connection) -> Result<(), StorageError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _schema (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        [],
    )?;

    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM _schema", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    for migration in MIGRATIONS {
        if migration.version > current {
            conn.execute_batch(migration.sql)
                .map_err(|e| StorageError::Migration {
                    version: migration.version,
                    error: e.to_string(),
                })?;

            conn.execute(
                "INSERT INTO _schema (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![migration.version, now_unix() as i64],
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_run_on_fresh_db() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='stories'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap(); // Should not fail
    }

    #[test]
    fn test_schema_version_tracked() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM _schema", [], |r| r.get(0))
            .unwrap();
        let expected = MIGRATIONS.last().unwrap().version;
        assert_eq!(version, expected);
    }
}
