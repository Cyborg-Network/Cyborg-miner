use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom};
use reqwest::blocking::Client;
use reqwest::header::{RANGE, CONTENT_LENGTH};
use std::error::Error;
use std::path::Path;

const CHUNK_SIZE: u64 = 100 * 1024 * 1024;

pub fn download_hf_model(
    // TODO replace with huggingface task
    model_id: &str,
    filename: &str,
    revision: &str,
    save_path: &str,
) -> Result<(), Box<dyn Error>> {
    let url = format!(
        "https://huggingface.co/{}/resolve/{}/{}",
        model_id, revision, filename
    );

    let client = Client::builder()
        .user_agent("cyborg-miner")
        .build()?;

    let head_resp = client.head(&url).send()?;
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

    let path = Path::new(save_path);
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
            .get(&url)
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

    println!("Download complete!");

    Ok(())
}