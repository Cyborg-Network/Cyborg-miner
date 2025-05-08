use ezkl::{
    commands::Commands::{GenWitness, GetSrs, Prove},
    execute::run,
};
use serde_json::Value;
use std::{fs::File, path::{Path, PathBuf}};
use tokio::sync::mpsc::{Receiver, Sender};
use std::io::{copy, BufReader};
use flate2::read::GzDecoder;
use tar::Archive;

#[derive(Debug)]
pub struct NeuroZKEngine {
    model_archive_path: PathBuf,
    current_witness_path: PathBuf,
}

const MODEL_PATH: &str = "circuit.ezkl";
const SETTINGS_PATH: &str = "settings.json";
const PROVING_KEY_PATH: &str = "pk.key";
const WITNESS_PATH: &str = "witness.json";
const PROOF_PATH: &str = "proof.json";

impl NeuroZKEngine {
    /// Takes a stream of inference data and starts performing inference, proving inference on request by submitting a ZK SNARK to the blockchain.
    ///
    /// # Arguments
    /// * `&self`
    /// * `request_stream` - An iterable receiver of data to perform inference on
    /// * `response_stream` - An iterable sender of inference responses
    ///
    /// # Returns
    /// A result containing either the inference output stream, or an Error `Result<(), Box<dyn std::error::Error>>`
    pub async fn run(
        &self,
        mut request_stream: Receiver<Value>,
        response_stream: Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {

        let neuro_zk_task_path = std::env::var("NZK_TASK_PATH")?;

        if !self.check_files_exists(&neuro_zk_task_path, 
            [
                MODEL_PATH, 
                SETTINGS_PATH, 
                PROVING_KEY_PATH, 
            ]
        ) {
            self.extract_model(
            self.model_archive_path,
                &neuro_zk_task_path,
                MODEL_PATH,
                PROVING_KEY_PATH,
                SETTINGS_PATH
            ).await? 
        }

        if let (Some(_), Some(_)) = (&self.compiled_model_path, &self.settings_path) {
            while let Some(request) = request_stream.recv().await {
                println!("Processing inference for request: {}", request);

                let result = self
                    .generate_inference_result(
                        serde_json::to_string(&request).ok()
                    )
                    .await?;
                response_stream.send(result).await?;
            }
        }

        Ok(())
    }

    /// Extracts the model currently loaded into the miner. Fails if `init_model` has not been called.
    ///
    /// # Arguments
    /// * `&self`
    ///
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    async fn extract_model(
        &self, 
        model_archive_location: PathBuf,
        prefix: &str, 
        model_file_name: &str,
        proving_key_file_name: &str,
        settings_file_name: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let archive_file = File::open(model_archive_location)?;
        let decoder = GzDecoder::new(BufReader::new(archive_file));
        let mut archive = Archive::new(decoder);

        let targets = [
            model_file_name,
            proving_key_file_name,
            settings_file_name,
        ];

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?;
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if targets.contains(&file_name) {
                    let output_path = Path::new(prefix).join(file_name);
                    let mut out_file = File::create(output_path)?;
                    copy(&mut entry, &mut out_file)?;
                }
            }
        }

        Ok(())
    }

    /// Checks if all of the necessary files exist in the given directory.
    ///
    /// # Arguments
    /// * `&self`
    /// * `prefix` - The directory to check
    /// * `nzk_files` - An array of file names
    ///
    /// # Returns
    /// A `bool` indicating wether all of the files exist
    fn check_files_exists(&self, prefix: &str, nzk_files: [&str; 3]) -> bool {
        let res = true;

        for file_path in nzk_files {
            if !std::fs::metadata(format!("{}/{}", prefix, file_path)).is_ok() {
                return false
            }
        }

        res
    }

    /// Takes input and proves inference on the model currently loaded into the miner. Fails if `init_model` has not been called. Should be called intermittently to request a proof of correct model execution.
    ///
    /// # Arguments
    /// * `model_location` - The location of the model currently loaded into the miner
    ///
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    pub async fn prove_inference(
        &self,
        model_path: Option<PathBuf>,
        proving_key_path: Option<PathBuf>,
        srs_path: Option<PathBuf>
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let proof = run(Prove {
            witness: Option<self.current_witness_path.clone()>,
            compiled_circuit: model_path,
            pk_path: (proving_key_path),
            proof_path: None,
            srs_path,
            proof_type: (ezkl::pfsys::ProofType::Single),
            check_mode: None
        }).await?;

        //TODO Insert appropriate byte vector proof here OR return path of the proof and let the parachain interactor handle it
        Ok(vec![1, 2, 3])
    }



    /// Takes input and performs inference on the model currently loaded into the miner. Fails if `init_model` has not been called. Should be called for the vast majority of inference requests.
    ///
    /// # Arguments
    /// * `&self`
    /// * `data` - The input used to run inference on the model in circuit form
    ///
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    async fn generate_inference_result(
        &self,
        model_path: Option<PathBuf>,
        input_data: Option<String>,
        srs_path: Option<PathBuf>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let witness = run(GenWitness {
            data: input_data,
            compiled_circuit: model_path,
            output: None,
            vk_path: None,
            srs_path,
        })
        .await?;

        Ok(witness)
    }
}
