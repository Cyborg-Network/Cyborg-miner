use flate2::read::GzDecoder;
use std::fs::{remove_file, File};
use std::io::{self, copy, BufReader};
use std::path::{ PathBuf};
use tar::Archive;
use zip::ZipArchive;

pub struct ModelExtractor {
    archive_path: PathBuf,
    output_folder: PathBuf,
}

impl ModelExtractor {
    pub fn new(model_name: &str, base_path: PathBuf) -> io::Result<Self> {
        let extracted_path = base_path.join(model_name);

        if extracted_path.is_dir() {
            println!("✅ Model already extracted at: {:?}", extracted_path);
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Model already extracted",
            ));
        }

        let tar_gz_path = base_path.join(format!("{}.tar.gz", model_name));
        let zip_path = base_path.join(format!("{}.zip", model_name));

        let archive_path = if tar_gz_path.exists() {
            tar_gz_path
        } else if zip_path.exists() {
            zip_path
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Model archive not found",
            ));
        };

        let extractor = Self {
            archive_path: archive_path.clone(),
            output_folder: base_path,
        };

        extractor.extract_model()?;

        Ok(extractor)
    }

    pub fn extract_model(&self) -> io::Result<()> {
        let extension = self
            .archive_path
            .extension()
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

        println!("🗑️ Deleting archive {:?}", self.archive_path);
        remove_file(&self.archive_path)?;

        Ok(())
    }

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
