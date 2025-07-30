// models.rs
use serde::{Deserialize, Serialize};
use std::fs::{File, metadata};
use std::io::{self, BufReader, copy};
use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;
use std::path::{Path, PathBuf};

/// Represents a model available in Triton
#[derive(Debug, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub version: Option<String>,
    pub platform: Option<String>,
}

/// Represents the status of a model in Triton
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus {
    pub name: String,
    pub version: String,
    pub last_inference: u64,
    pub inference_count: u64,
    pub execution_count: u64,
    pub memory_usage: Vec<MemoryUsage>,
}

/// Represents memory usage statistics for a model
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub kind: String,
    pub bytes: u64,
}

/// Handles extraction of model files from a tar.gz or zip archive
pub struct ModelExtractor {
    archive_path: PathBuf,
    output_folder: PathBuf,
}

impl ModelExtractor {
    pub fn new(archive_path: &str, output_folder: &str) -> Self {
        Self {
            archive_path: PathBuf::from(archive_path),
            output_folder: PathBuf::from(output_folder),
        }
    }

    /// Main extraction handler that chooses the right method
    pub fn extract_model(&self) -> io::Result<()> {
        let extension = self.archive_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension {
            "gz" => self.extract_tar_gz(),
            "zip" => self.extract_zip(),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported archive format",
            )),
        }?;

        // Delete archive after extraction
        remove_file(&self.archive_path)?;

        // 🧠 Compute hash of model.onnx
        // let model_name = self
        //     .archive_path
        //     .file_stem()
        //     .and_then(|s| s.to_str())
        //     .unwrap_or("unknown_model")
        //     .to_string();

        // let model_path = self
        //     .output_folder
        //     .join(&model_name)
        //     .join("1")
        //     .join("model.onnx");
        // let output_blob_path = self
        //     .output_folder
        //     .join(&model_name)
        //     .join("model_id.wasmhash");

        // if model_path.exists() {
        //     match Self::hash_model_file(&model_path, &output_blob_path) {
        //         Ok(_) => println!(),
        //         Err(e) => eprintln!("❌ Failed to hash model file: {}", e),
        //     }
        // }

        Ok(())
    }

    /// Extracts all files from the tar.gz archive to the specified output folder
    fn extract_tar_gz(&self) -> io::Result<()> {
        println!("🔍 Detected .tar.gz format. Extracting...");
        let archive_file = File::open(&self.archive_path)?;
        let decoder = GzDecoder::new(BufReader::new(archive_file));
        let mut archive = Archive::new(decoder);

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();
            let output_path = self.output_folder.join(&path);

            if entry.header().entry_type().is_dir() {
                println!("📂 Creating directory {:?}", output_path);
                std::fs::create_dir_all(&output_path)?;
                continue;
            }

            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut out_file = File::create(&output_path)?;
            copy(&mut entry, &mut out_file)?;
            println!("Extracted {:?} to {:?}", path, &self.output_folder);
        }
        Ok(())
    }
    // pub fn hash_model_file(model_path: &Path, output_blob_path: &Path) -> io::Result<()> {
    //     // Read model bytes
    //     let mut file = File::open(model_path)?;
    //     let mut buffer = Vec::new();
    //     file.read_to_end(&mut buffer)?;

    //     // Compute SHA-256
    //     let sha256 = Sha256::digest(&buffer);
    //     let model_id = sha256.to_vec();
    //     let base64_hash = general_purpose::STANDARD.encode(&sha256);
    //     let hex_model_id = hex::encode(&model_id);

    //     // Print to stdout
    //     println!("Model ID (hex): {}", hex_model_id);
    //     println!("Base64 Hash: {}", base64_hash);

    //     // Write hex model ID to the output path
    //     let mut output_file = File::create(output_blob_path)?;
    //     output_file.write_all(hex_model_id.as_bytes())?;
    //     output_file.sync_all()?;

    //     Ok(())
    // }

    /// Extracts all files from the .zip archive to the specified output folder
    fn extract_zip(&self) -> io::Result<()> {
        println!("🔍 Detected .zip format. Extracting...");
        let archive_file = File::open(&self.archive_path)?;
        let mut archive = ZipArchive::new(archive_file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let out_path = self.output_folder.join(file.sanitized_name());

            if file.is_dir() {
                println!("📂 Creating directory {:?}", out_path);
                std::fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = File::create(&out_path)?;
                copy(&mut file, &mut out_file)?;
                println!("Extracted {:?} to {:?}", file.name(), &self.output_folder);
            }
        }
        Ok(())
    }
}
