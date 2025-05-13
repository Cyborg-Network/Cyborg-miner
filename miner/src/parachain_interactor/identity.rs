use crate::error::Result;
use std::fs;
use std::path::PathBuf;

pub fn update_identity_file(path: &PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, content)?;

    Ok(())
}
