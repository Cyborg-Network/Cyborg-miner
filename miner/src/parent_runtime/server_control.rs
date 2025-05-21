use std::sync::Mutex;
use tokio::sync::watch;
use once_cell::sync::Lazy;

pub static SHUTDOWN_SENDER: Lazy<Mutex<Option<watch::Sender<bool>>>> =
    Lazy::new(|| Mutex::new(None));
