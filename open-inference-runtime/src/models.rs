use std::io::{self, BufReader, copy, Read, Write};
use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;
use std::path::{Path, PathBuf};
use std::fs::{File, remove_file};
use sha2::{Digest, Sha256};
use base64::{engine::general_purpose, Engine as _};




/// Handles extraction of model files from a tar.gz or zip archive
pub struct ModelExtractor {
    archive_path: PathBuf,
    output_folder: PathBuf,
}

impl ModelExtractor {
    pub fn new(model_name: &str,base_path:PathBuf) -> io::Result<Self> {
        let tar_gz_path = Path::new(&base_path).join(format!("{}.tar.gz", model_name));
        let zip_path = Path::new(&base_path).join(format!("{}.zip", model_name));
        let extracted_path = Path::new(&base_path).join(model_name);

        // Check if already extracted
        if extracted_path.is_dir() {
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
            output_folder: PathBuf::from(base_path),
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
        remove_file(&self.archive_path)?;
    
        // 🧠 Compute hash of model.onnx
        let model_name = self.archive_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_model")
            .to_string();
    
            let model_path = self.output_folder.join(&model_name).join("1").join("model.onnx");
            let output_blob_path = self.output_folder.join(&model_name).join("model_id.wasmhash");
            
            if model_path.exists() {
                match Self::hash_model_file(&model_path, &output_blob_path) {
                    Ok(_) => println!(),
                    Err(e) => eprintln!("❌ Failed to hash model file: {}", e),
                }
            }

    
        Ok(())
    }
    

     /// Extracts all files from the tar.gz archive to the specified output folder
     fn extract_tar_gz(&self) -> io::Result<()> {
        let archive_file = File::open(&self.archive_path)?;
        let decoder = GzDecoder::new(BufReader::new(archive_file));
        let mut archive = Archive::new(decoder);

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();
            let output_path = self.output_folder.join(&path);

            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&output_path)?;
                continue;
            }

            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut out_file = File::create(&output_path)?;
            copy(&mut entry, &mut out_file)?;
        }
        Ok(())
    }
    pub fn hash_model_file(model_path: &Path, output_blob_path: &Path) -> io::Result<()> {
        // Read model bytes
        let mut file = File::open(model_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // Compute SHA-256
        let sha256 = Sha256::digest(&buffer);
        let model_id = sha256.to_vec();
        let base64_hash = general_purpose::STANDARD.encode(&sha256);
        let hex_model_id = hex::encode(&model_id);

        // Print to stdout
        println!("Model ID (hex): {}", hex_model_id);
        println!("Base64 Hash: {}", base64_hash);

        // Write hex model ID to the output path
        let mut output_file = File::create(output_blob_path)?;
        output_file.write_all(hex_model_id.as_bytes())?;
        output_file.sync_all()?;

        Ok(())
    }




    /// Extracts all files from the .zip archive to the specified output folder
    #[allow(deprecated)]
    fn extract_zip(&self) -> io::Result<()> {
        let archive_file = File::open(&self.archive_path)?;
        let mut archive = ZipArchive::new(archive_file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let out_path = self.output_folder.join(file.sanitized_name());

            if file.is_dir() {
                std::fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = File::create(&out_path)?;
                copy(&mut file, &mut out_file)?;
            }
        }
        Ok(())
    }

}
