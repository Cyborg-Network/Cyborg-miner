use crate::config;
use crate::error::{Error, Result};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tracing_appender::non_blocking;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

static LOG_GUARD: Lazy<Mutex<Option<WorkerGuard>>> = Lazy::new(|| Mutex::new(None));

pub fn init_logger() -> Result<()> {
    eprintln!("Running as UID: {:?}", nix::unistd::getuid());

    let log_file_path = &config::PATHS.get()
        .ok_or(Error::config_paths_not_initialized())?
        .log_path;

    let log_dir_path = log_file_path
        .parent()
        .ok_or(Error::Custom(
            "Could not get parent of log path".to_string()
        ))?;

    let log_file = log_file_path
        .file_name()
        .ok_or(Error::Custom(
            "Could not get file name of log path".to_string()
        ))?;

    let file_appender = tracing_appender::rolling::never(log_dir_path, log_file);

    let (non_blocking_writer, guard) = non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(BoxMakeWriter::new(non_blocking_writer))
        .with_ansi(false)
        .with_level(true)
        .init();

    *LOG_GUARD.lock().map_err(|_| Error::Custom("Failed to lock log guard => poisoned?".to_string()))? = Some(guard);

    Ok(())
}

fn reset_log_file() -> Result<()> {
    *LOG_GUARD.lock().map_err(|_| Error::Custom("Failed to lock log guard => poisoned".to_string()))? = None;

    let log_file_path = &config::PATHS.get()
        .ok_or(Error::config_paths_not_initialized())?
        .log_path;

    std::fs::remove_file(log_file_path)?;

    Ok(())
}
