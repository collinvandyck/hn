use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing with file output.
///
/// Returns a guard that must be held for the lifetime of the application
/// to ensure logs are flushed.
pub fn init(log_path: &Path, verbose: bool) -> Option<WorkerGuard> {
    let parent = log_path.parent()?;
    std::fs::create_dir_all(parent).ok()?;
    let file_appender = tracing_appender::rolling::never(parent, log_path.file_name()?);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::new(if verbose { "hn=debug" } else { "hn=info" });
    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true),
        )
        .init();

    Some(guard)
}
