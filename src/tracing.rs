use anyhow::Result;
use dirs;
use std::io;
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, writer::MakeWriterExt},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

fn determine_log_path(app_name: &str) -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        let log_dir = home.join(".local/share").join(app_name).join("logs");
        return log_dir;
    }

    PathBuf::from("logs")
}

pub fn init_tracing() -> Result<WorkerGuard> {
    let log_path = determine_log_path(env!("CARGO_PKG_NAME"));
    let file_appender =
        tracing_appender::rolling::daily(log_path, format!("{}.log", env!("CARGO_PKG_NAME")));
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    let stdout_layer = fmt::layer()
        .with_writer(io::stdout.with_max_level(tracing::Level::INFO))
        .with_ansi(true)
        .with_target(true)
        .with_timer(fmt::time::ChronoLocal::rfc_3339())
        .pretty();

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "debug,tower_http=info,umem_mcp=info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    Ok(guard)
}
