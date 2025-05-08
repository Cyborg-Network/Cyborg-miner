use crate::error::Result;
use crate::types::Miner;
use std::fs;
use std::path::PathBuf;

pub fn update_config_file(miner: &Miner, path: &PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, content)?;

    Ok(())
}
