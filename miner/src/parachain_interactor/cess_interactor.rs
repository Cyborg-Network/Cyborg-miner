use crate::error::{Error, Result};
use crate::traits::ParachainInteractor;
use crate::types::{Miner, TaskType};
use reqwest::get;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

pub async fn download_model_archive(
    miner: &mut Miner,
    cess_fid: &str,
) -> Result<()> {
    //TODO the extraction of the archive will be left up to the individual runtimes, as they might treat it differently
    println!("Starting download model archive: {}", cess_fid);

    miner.write_log(format!("Retrieving model archive with fid: {}...", &cess_fid).as_str());

    let url = format!("{}/{}", miner.cess_gateway, cess_fid);

    let response = get(&url).await?;

    if !response.status().is_success() {
        eprintln!("Error: {}", response.status());
        return Err(Error::Custom(format!(
            "Failed to download model archive, CESS responded with {}",
            response.status()
        )));
    }

    if let Some(parent) = &miner.task_path.parent() {
        match fs::create_dir_all(parent) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Failed to create directory: {}", e);
                return Err(Error::Io(e));
            }
        }
    }

    let mut file = File::create(&miner.task_path)?;

    let response_bytes = response.bytes().await?;

    println!(
        "Downloaded {} bytes from IPFS gateway.",
        response_bytes.len()
    );

    file.write_all(&response_bytes)?;

    // File needs to be dropped, else there will be a race condition and the file will not be executable
    drop(file);

    let mut perms = fs::metadata(&miner.task_path)?.permissions();

    perms.set_mode(perms.mode() | 0o111);

    fs::set_permissions(&miner.task_path, perms)?;

    miner.write_log("Work package retrieved!");

    miner.write_log("Executing work package...");

    let execution = Command::new(&miner.task_path)
        .stdout(Stdio::piped())
        .spawn()?;

    // TODO: This only permits the execution of tasks with one ouput - need to establish a standard for measuring intermittent results
    if let Some(output) = execution.wait_with_output().ok() {
        miner.write_log("Work package executed!");
        return Ok(());
    } else {
        return Err(Error::Custom("Failed to execute work package".to_string()));
    }
}
