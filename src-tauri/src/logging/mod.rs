use std::fs;
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes structured daily rotating file logging plus stderr logging.
/// Returns a WorkerGuard that must be kept alive for the lifetime of the app.
pub fn init_logging(log_dir: &Path) -> Option<WorkerGuard> {
    if let Err(e) = fs::create_dir_all(log_dir) {
        eprintln!("Warning: could not create log directory {}: {e}", log_dir.display());
        return None;
    }

    let file_appender = tracing_appender::rolling::daily(log_dir, "dns-changer.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,dns_changer_lib=debug"));

    let file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_target(true)
        .with_thread_ids(true);

    let stdout_layer = fmt::layer()
        .compact()
        .with_writer(std::io::stderr);

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .try_init();

    Some(guard)
}
