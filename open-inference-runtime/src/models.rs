use serde::{Deserialize, Serialize};
use std::io::{self, BufReader, copy, Read, Write};
use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;
use std::path::{Path, PathBuf};
use std::fs::{File, remove_file};
use sha2::{Digest, Sha256};


// const BASE_PATH: &str = "/var/lib/cyborg/miner/current_task/";

const BASE_PATH: &str = "/home/ronnie/Model";


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
            let output_blob_path = self.output_folder.join(&model_name).join("verifier_hash.wasmhash");
            
            if model_path.exists() {
                println!("🔍 Found model file at: {:?}", model_path);
                match Self::hash_model_file(&model_path, &output_blob_path) {
                    Ok(_) => println!("✅ WASM hash blob saved: {:?}", output_blob_path),
                    Err(e) => eprintln!("❌ Failed to hash model file: {}", e),
                }
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


    /// Computes SHA-256  and writes them to a verifier_key.txt file
    pub fn hash_model_file(model_path: &Path, output_blob_path: &Path) -> io::Result<()> {
        // Read model bytes
        let mut file = File::open(model_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
    
        // Compute SHA-256 and SHA-512
        let sha256 = Sha256::digest(&buffer);
    
        // Combine hashes into binary blob format
        let mut blob: Vec<u8> = Vec::new();
    
        // Optional header to identify file type/version
        blob.extend(b"WASM_HASH_V1");           // 12 bytes header
        blob.push(0);                           // 1-byte null delimiter
    
        blob.extend(&(sha256.len() as u32).to_le_bytes());  // length of SHA-256
        blob.extend(&sha256);                               // actual SHA-256 bytes
    
    
        // Write to output .wasmhash file
        let mut output_file = File::create(output_blob_path)?;
        output_file.write_all(&blob)?;
        output_file.sync_all()?; // Flush buffer
    
        // Set to read-only (Unix)
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

pub fn verify_model_blob(model_name: &str) -> io::Result<()> {
    let extracted_path = Path::new(BASE_PATH).join(model_name);
    let model_path = extracted_path.join("1").join("model.onnx");
    let blob_path = extracted_path.join("verifier_hash.wasmhash");
    
    // Step 1: Read the model and compute SHA-256
    let mut model_file = File::open(model_path)?;
    let mut model_data = Vec::new();
    model_file.read_to_end(&mut model_data)?;

    let model_sha256 = Sha256::digest(&model_data);

    // Step 2: Read the blob
    let mut blob_file = File::open(blob_path)?;
    let mut blob = Vec::new();
    blob_file.read_to_end(&mut blob)?;

    // Step 3: Validate header
    let expected_header = b"WASM_HASH_V1\0";
    if !blob.starts_with(expected_header) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid blob header"));
    }

    let mut cursor = expected_header.len();

    // Step 4: Read SHA-256 length
    if blob.len() < cursor + 4 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Truncated SHA-256 length"));
    }

    let sha256_len = u32::from_le_bytes(blob[cursor..cursor+4].try_into().unwrap()) as usize;
    cursor += 4;

    // Step 5: Read SHA-256 value
    if blob.len() < cursor + sha256_len {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Truncated SHA-256 value"));
    }

    let stored_sha256 = &blob[cursor..cursor + sha256_len];

    if model_sha256.as_slice() == stored_sha256 {
        println!("✅ Hash verification passed");
        Ok(())
    } else {
        eprintln!("❌ Hash mismatch: model file has been tampered or is different");
        std::process::exit(1);
    }
}