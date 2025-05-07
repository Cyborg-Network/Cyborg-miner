use std::path::PathBuf;
use std::fs;
use crate::error::Result;
use crate::types::Miner;

pub fn update_config_file(miner: &Miner, path: &PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, content)?;

    Ok(())
}