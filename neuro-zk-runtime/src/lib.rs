use ezkl::{
    commands::Commands::{GenWitness, GetSrs, Prove}, execute::run, Commitments
};
use std::{fs::File, path::{Path, PathBuf}};
use std::io::{copy, BufReader};
use flate2::read::GzDecoder;
use tar::Archive;
use futures::{
    Stream, 
    Future,
    stream::StreamExt
};

#[derive(Debug)]
pub struct NeuroZKEngine {
    model_archive_path: PathBuf,
}

const MODEL_PATH: &str = "circuit.ezkl";
const SETTINGS_PATH: &str = "settings.json";
const PROVING_KEY_PATH: &str = "pk.key";
const WITNESS_PATH: &str = "witness.json";
const PROOF_PATH: &str = "proof.json";
const SRS_PATH: &str = "srs.json";

impl NeuroZKEngine {
    /// Creates a new `NeuroZKEngine` instance.
    /// 
    /// # Arguments
    /// * `model_archive_path` - The path to the model archive
    /// 
    /// # Returns
    /// A new `NeuroZKEngine` instance
    pub fn new(model_archive_path: PathBuf) -> Self {
        Self {
            model_archive_path,
        }
    }
    
    /// Takes a stream of inference data and starts performing inference, proving inference on request by submitting a ZK SNARK to the blockchain.
    ///
    /// # Arguments
    /// * `&self`
    /// * `request_stream` - An iterable receiver of data to perform inference on
    /// * `response_stream` - An iterable sender of inference responses
    ///
    /// # Returns
    /// A result containing either the inference output stream, or an Error `Result<(), Box<dyn std::error::Error>>`
    pub async fn run<S, C, CFut>(
        &self,
        mut request_stream: S,
        mut response_closure: C,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        S: Stream<Item = String> + Unpin + Send + 'static,
        C: FnMut(String) -> CFut + Send + 'static,
        CFut: Future<Output = ()> + Send + 'static,
    {

        let neuro_zk_task_dir = std::env::var("NZK_TASK_PATH")?;

        self.extract_model(
            &self.model_archive_path,
            &neuro_zk_task_dir,
            MODEL_PATH,
            PROVING_KEY_PATH,
            SETTINGS_PATH,
        ).await?;

        self.check_or_get_srs(
            &neuro_zk_task_dir, 
            SRS_PATH, 
            SETTINGS_PATH
        ).await?;

        while let Some(request) = request_stream.next().await {
            println!("Processing inference for request: {}", request);

            if let Some(data) = serde_json::to_string(&request).ok() {
                let result = self
                    .generate_inference_result(
                        &neuro_zk_task_dir,
                        MODEL_PATH,
                        SRS_PATH,
                        WITNESS_PATH,
                        data
                    ).await?;

                response_closure(result).await;
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
        model_archive_location: &PathBuf,
        prefix: &str, 
        model_file_name: &str,
        proving_key_file_name: &str,
        settings_file_name: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.check_files_exists(
            prefix, 
            [
                model_file_name, 
                proving_key_file_name, 
                settings_file_name
            ]
        ) {
             return Ok(()) 
        };

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

    /// Downloads the SRS and saves it to the fs
    ///
    /// # Arguments
    /// * `&self`
    ///
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    async fn check_or_get_srs(
        &self,
        prefix: &str, 
        srs_path: &str, 
        settings_path: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let srs_path = PathBuf::from(format!("{}/{}", prefix, srs_path));
        let settings_path = PathBuf::from(format!("{}/{}", prefix, settings_path));

        if !std::fs::metadata(&srs_path).is_ok() {
            run(GetSrs {
                settings_path: Some(settings_path),
                srs_path: Some(srs_path),
                commitment: Some(Commitments::KZG),
                logrows: None
            }).await?;

            Ok(())
        } else {
            Ok(())
        }
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
    async fn prove_inference(
        &self,
        prefix: &str,
        model_path: &str,
        proving_key_path: &str,
        proof_path: &str,
        srs_path: &str,
        witness_path: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let model_path = PathBuf::from(format!("{}/{}", prefix, model_path));
        let proving_key_path = PathBuf::from(format!("{}/{}", prefix, proving_key_path));
        let srs_path = PathBuf::from(format!("{}/{}", prefix, srs_path));
        let proof_path = PathBuf::from(format!("{}/{}", prefix, proof_path));
        let witness_path = PathBuf::from(format!("{}/{}", prefix, witness_path));

        let proof = run(Prove {
            witness: Some(witness_path),
            compiled_circuit: Some(model_path),
            pk_path: Some(proving_key_path),
            proof_path: Some(proof_path),
            srs_path: Some(srs_path),
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
        prefix: &str,
        model_path: &str,
        srs_path: &str,
        witness_path: &str,
        input_data: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let model_path = PathBuf::from(format!("{}/{}", prefix, model_path));
        let srs_path = PathBuf::from(format!("{}/{}", prefix, srs_path));
        let witness_path = PathBuf::from(format!("{}/{}", prefix, witness_path));

        let witness = run(GenWitness {
            data: Some(ezkl::commands::DataField(input_data)),
            compiled_circuit: Some(model_path),
            output: Some(witness_path),
            vk_path: None,
            srs_path: Some(srs_path),
        })
        .await?;

        Ok(witness)
    }
}
