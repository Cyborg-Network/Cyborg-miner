use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::sync::watch;

pub static SHUTDOWN_SENDER: Lazy<Mutex<Option<watch::Sender<bool>>>> =
    Lazy::new(|| Mutex::new(None));
