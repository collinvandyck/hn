use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, TimeZone, Utc};

/// Returns the current Unix timestamp in seconds.
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub fn system_clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}

#[cfg(test)]
pub struct FixedClock(pub DateTime<Utc>);

#[cfg(test)]
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

#[cfg(test)]
pub fn fixed_clock(timestamp: i64) -> Arc<dyn Clock> {
    Arc::new(FixedClock(Utc.timestamp_opt(timestamp, 0).unwrap()))
}

#[allow(clippy::cast_possible_wrap)] // Unix timestamps won't exceed i64::MAX until year 292 billion
pub fn format_relative(timestamp: u64, now: DateTime<Utc>) -> String {
    Utc.timestamp_opt(timestamp as i64, 0).single().map_or_else(
        || "?".to_string(),
        |t| {
            let diff = now.signed_duration_since(t);
            if diff.num_hours() < 1 {
                format!("{}m ago", diff.num_minutes())
            } else if diff.num_hours() < 24 {
                format!("{}h ago", diff.num_hours())
            } else {
                format!("{}d ago", diff.num_days())
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_relative_minutes() {
        let now = Utc.timestamp_opt(1700000000, 0).unwrap();
        let timestamp = 1700000000 - 30 * 60; // 30 minutes ago
        assert_eq!(format_relative(timestamp, now), "30m ago");
    }

    #[test]
    fn format_relative_hours() {
        let now = Utc.timestamp_opt(1700000000, 0).unwrap();
        let timestamp = 1700000000 - 5 * 3600; // 5 hours ago
        assert_eq!(format_relative(timestamp, now), "5h ago");
    }

    #[test]
    fn format_relative_days() {
        let now = Utc.timestamp_opt(1700000000, 0).unwrap();
        let timestamp = 1700000000 - 3 * 86400; // 3 days ago
        assert_eq!(format_relative(timestamp, now), "3d ago");
    }
}
