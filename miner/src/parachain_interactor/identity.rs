use crate::error::Result;
use std::fs;
use std::path::PathBuf;

pub fn update_identity_file(path: &str, content: &str) -> Result<()> {
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, content)?;

    Ok(())
}