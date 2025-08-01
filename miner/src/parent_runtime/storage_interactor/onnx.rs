use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom};
use reqwest::blocking::Client;
use reqwest::header::{RANGE, CONTENT_LENGTH};
use std::path::Path;

use crate::config;
use crate::error::{Result, Error};
use crate::substrate_interface::api::runtime_types::cyborg_primitives::task::OnnxTask;

const CHUNK_SIZE: u64 = 100 * 1024 * 1024;

pub async fn download_onnx_model(onnx_task: OnnxTask) -> Result<()> {
    let task_file_name = &config::PATHS
        .get()
        .ok_or(Error::config_paths_not_initialized())?
        .task_file_name;

    let task_dir = &config::PATHS
        .get()
        .ok_or(Error::config_paths_not_initialized())?
        .task_dir_path;

    let save_path = format!("{}/{}", task_dir, task_file_name);
    let model_url = String::from_utf8(onnx_task.storage_location_identifier.0)?;

    let client = Client::builder()
        .user_agent("cyborg-miner")
        .build()?;

    let head_resp = client.head(&model_url).send()?;
    if !head_resp.status().is_success() {
        return Err(format!("HEAD request failed with status {}", head_resp.status()).into());
    }

    let total_size = head_resp
        .headers()
        .get(CONTENT_LENGTH)
        .ok_or("Content-Length header missing")?
        .to_str()?
        .parse::<u64>()?;

    println!("Total file size: {} bytes", total_size);

    let path = Path::new(&save_path);
    let mut downloaded: u64 = if path.exists() {
        std::fs::metadata(path)?.len()
    } else {
        0
    };

    println!("Already downloaded: {} bytes", downloaded);

    if downloaded == total_size {
        println!("File already fully downloaded.");
        return Ok(());
    }

    println!("Saving file to path: {:?}", path);

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(path)?;

    file.seek(SeekFrom::Start(downloaded))?;

    while downloaded < total_size {
        let end = std::cmp::min(downloaded + CHUNK_SIZE - 1, total_size - 1);
        let range_header = format!("bytes={}-{}", downloaded, end);

        println!("Requesting range: {}", range_header);

        let mut resp = client
            .get(&model_url)
            .header(RANGE, range_header)
            .send()?;

        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(format!("Failed to download chunk: HTTP {}", resp.status()).into());
        }

        let chunk_size = std::io::copy(&mut resp, &mut file)?;
        if chunk_size == 0 {
            break;
        }

        downloaded += chunk_size;
        println!("Downloaded {} / {} bytes", downloaded, total_size);
    }

    println!("Model Download complete!");

    Ok(())
}