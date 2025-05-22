// models.rs
use serde::{Deserialize, Serialize};
use std::io::{self, BufReader, copy, Read, Write};
use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;
use std::path::{Path, PathBuf};
use std::fs::{File, metadata, remove_file};
use sha2::{Digest, Sha256, Sha512};

const BASE_PATH: &str = "/var/lib/cyborg/miner/current_task/";

// const BASE_PATH: &str = "/home/ronnie/Model";


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
    pub fn new(model_name: &str) -> io::Result<Self> {
        let tar_gz_path = Path::new(BASE_PATH).join(format!("{}.tar.gz", model_name));
        let zip_path = Path::new(BASE_PATH).join(format!("{}.zip", model_name));
        let extracted_path = Path::new(BASE_PATH).join(model_name);

        // Check if already extracted
        if extracted_path.is_dir() {
            println!("✅ Model already extracted at: {:?}", extracted_path);
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "Model already extracted"));
        }

        let archive_path = if tar_gz_path.exists() {
            tar_gz_path
        } else if zip_path.exists() {
            zip_path
        } else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Model archive not found"));
        };

        Ok(Self {
            archive_path,
            output_folder: PathBuf::from(BASE_PATH),
        })
    }

    pub fn extract_model(&self) -> io::Result<()> {
        let extension = self.archive_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
    
        match extension {
            "gz" => self.extract_tar_gz(),
            "zip" => self.extract_zip(),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported archive format")),
        }?;
    
        // Delete archive after extraction
        println!("🗑️ Deleting archive {:?}", self.archive_path);
        remove_file(&self.archive_path)?;
    
        // 🧠 Compute hash of model.onnx
        let model_name = self.archive_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_model")
            .to_string();
    
        let model_path = self.output_folder.join(&model_name).join("1").join("model.onnx");
        let output_txt_path = self.output_folder.join(&model_name).join("verifier_key.txt");
    
        if model_path.exists() {
            println!("🔍 Found model file at: {:?}", model_path);
            match Self::hash_model_file(&model_path, &output_txt_path) {
                Ok(_) => println!("✅ Hash written to: {:?}", output_txt_path),
                Err(e) => eprintln!("❌ Failed to hash model file: {}", e),
            }
        } else {
            eprintln!("❌ model.onnx not found at expected path: {:?}", model_path);
        }
    
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
            println!("✅ Extracted {:?} to {:?}", path, &self.output_folder);
        }
        Ok(())
    }

    fn hash_model_file(model_path: &Path, output_path: &Path) -> io::Result<()> {
        let mut file = File::open(model_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
    
        let sha256 = Sha256::digest(&buffer);
        let sha512 = Sha512::digest(&buffer);
    
        let sha256_hex = format!("{:x}", sha256);
        let sha512_hex = format!("{:x}", sha512);
    
        let mut output_file = File::create(output_path)?;
        writeln!(output_file, "SHA-256: {}", sha256_hex)?;
        writeln!(output_file, "SHA-512: {}", sha512_hex)?;
    
        Ok(())
    }

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
                println!("✅ Extracted {:?} to {:?}", file.name(), &self.output_folder);
            }
        }
        Ok(())
    }

}