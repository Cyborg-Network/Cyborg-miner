use crate::error::Result;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tracing_appender::non_blocking;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

static LOG_GUARD: Lazy<Mutex<Option<WorkerGuard>>> = Lazy::new(|| Mutex::new(None));

pub fn init_logger() {
    let file_appender = tracing_appender::rolling::never("logs", "miner.log");

    let (non_blocking_writer, guard) = non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(BoxMakeWriter::new(non_blocking_writer))
        .with_ansi(false)
        .with_level(true)
        .init();

    *LOG_GUARD.lock().unwrap() = Some(guard);
}

fn reset_log_file() -> Result<()> {
    *LOG_GUARD.lock().unwrap() = None;

    std::fs::remove_file("logs/miner.log")?;

    Ok(())
}
