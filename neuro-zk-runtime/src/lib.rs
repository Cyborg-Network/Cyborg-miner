use ezkl::{
    commands::Commands::{GenWitness, GetSrs, Prove},
    execute::run,
    Commitments,
};
use zstd::stream::read::Decoder;
use futures::{stream::StreamExt, Future, Stream};
use std::io::{copy, BufReader};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};
use tar::Archive;

#[derive(Debug)]
pub struct NeuroZKEngine {
    model_archive_path: PathBuf,
    task_dir_string: String,
}

const MODEL_PATH: &str = "network.ezkl";
const SETTINGS_PATH: &str = "settings.json";
const PROVING_KEY_PATH: &str = "pk.key";
const PROOF_INPUT_PATH: &str = "input.json";
const PROOF_WITNESS_PATH: &str = "proof-witness.json";
const WITNESS_PATH: &str = "witness.json";
const SRS_PATH: &str = "kzg.srs";

impl NeuroZKEngine {
    /// Creates a new `NeuroZKEngine` instance.
    ///
    /// # Arguments
    /// * `model_archive_path` - The path to the model archive
    ///
    /// # Returns
    /// A new `NeuroZKEngine` instance
    pub fn new(model_archive_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(parent_dir) = model_archive_path.clone().parent() {
            let task_dir_string = parent_dir.to_str().expect("Invalid model archive path");

            Ok(Self {
                model_archive_path,
                task_dir_string: task_dir_string.to_string(),
            })
        } else {
            return Err("Invalid model archive path".into());
        }
    }

    pub async fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.extract_model(
            &self.model_archive_path,
            &self.task_dir_string,
            PROOF_INPUT_PATH,
            MODEL_PATH,
            PROVING_KEY_PATH,
            SETTINGS_PATH,
        )
        .await?;

        self.check_or_get_srs(&self.task_dir_string, SRS_PATH, SETTINGS_PATH)
            .await?;

        Ok(())
    }

    /// Takes a stream of inference data and starts performing inference, proving inference on request by submitting a ZK SNARK to the blockchain.
    ///
    /// # Arguments
    /// * `&self`
    /// * `request_stream` - The stream of inference data
    /// * `response_closure` - A closure that takes a string and returns a future that resolves to ()
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
        while let Some(request) = request_stream.next().await {
            println!("Processing inference for request: {}", request);

            let response: String;

            match self
                .generate_inference_result(
                    &self.task_dir_string,
                    MODEL_PATH,
                    SRS_PATH,
                    WITNESS_PATH,
                    request.clone(),
                )
                .await
            {
                Ok(result) => {
                    response = result;
                }
                Err(e) => {
                    println!("Failed to generate inference result, likely EZKL version mismatch OR incorrect request format! Error: {}", e);
                    response =
                        "Failed to generate inference result, likely incorrect request format!"
                            .to_string();
                }
            }

            println!("Generated inference result: {}", response);

            response_closure(response).await;
        }

        Ok(())
    }

    /// Extracts the model currently loaded into the miner. Fails if `init_model` has not been called.
    ///
    /// # Arguments
    /// * `&self`
    /// * `model_archive_location` - The path to the model archive
    /// * `prefix` - The directory for operations on NZK related files
    /// * `model_file_name` - The name of the model file
    /// * `proving_key_file_name` - The name of the proving key file
    /// * `settings_file_name` - The name of the settings file
    ///
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    async fn extract_model(
        &self,
        model_archive_location: &PathBuf,
        prefix: &str,
        proof_input_file_name: &str,
        model_file_name: &str,
        proving_key_file_name: &str,
        settings_file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.check_files_exists(
            prefix,
            [
                proof_input_file_name,
                model_file_name,
                proving_key_file_name,
                settings_file_name,
            ],
        ) {
            return Ok(());
        };

        println!("Opening archive at: {:?}", model_archive_location);
        if !model_archive_location.exists() {
            return Err("Model archive path does not exist".into());
        }
        let archive_file = File::open(model_archive_location)?;
        let decoder = Decoder::new(BufReader::new(archive_file))?;
        let mut archive = Archive::new(decoder);

        let targets = [
            proof_input_file_name,
            model_file_name,
            proving_key_file_name,
            settings_file_name,
        ];

        for entry_result in archive.entries()? {
            println!("Extracting entry...");
            let mut entry = entry_result?;
            println!("Entry name...");
            let path = entry.path()?;
            println!("Entry path: {:?}...", path);
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                println!("File name: {:?}...", file_name);
                if targets.contains(&file_name) {
                    println!("Found target file: {:?}...", file_name);
                    let output_path = Path::new(prefix).join(file_name);
                    println!("Extracting to: {:?}", output_path);
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
    /// * `prefix` - The directory for operations on NZK related files
    /// * `srs_path` - The path to save the SRS to
    /// * `settings_path` - The path to the settings file
    ///
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    async fn check_or_get_srs(
        &self,
        prefix: &str,
        srs_path: &str,
        settings_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let srs_path = PathBuf::from(format!("{}/{}", prefix, srs_path));
        let settings_path = PathBuf::from(format!("{}/{}", prefix, settings_path));

        if !std::fs::metadata(&srs_path).is_ok() {
            run(GetSrs {
                settings_path: Some(settings_path),
                srs_path: Some(srs_path),
                commitment: Some(Commitments::KZG),
                logrows: None,
            })
            .await?;

            Ok(())
        } else {
            Ok(())
        }
    }

    /// Checks if all of the necessary files exist in the given directory.
    ///
    /// # Arguments
    /// * `&self`
    /// * `prefix` - The directory for operations on NZK related files
    /// * `nzk_files` - An array of file names
    ///
    /// # Returns
    /// A `bool` indicating wether all of the files exist
    fn check_files_exists(&self, prefix: &str, nzk_files: [&str; 4]) -> bool {
        let res = true;

        for file_path in nzk_files {
            if !std::fs::metadata(format!("{}/{}", prefix, file_path)).is_ok() {
                return false;
            }
        }

        res
    }

    /// Takes input and proves inference on the model currently loaded into the miner. Fails if `init_model` has not been called. Should be called intermittently to request a proof of correct model execution.
    ///
    /// # Arguments
    /// * `&self`
    /// * `prefix` - The directory for operations on NZK related files
    /// * `model_path` - The location of the model currently loaded into the miner
    /// * `proving_key_path` - The location of the proving key currently loaded into the miner
    /// * `srs_path` - The location of the SRS currently loaded into the miner
    /// * `witness_path` - The location of the witness currently loaded into the miner
    ///
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    pub async fn prove_inference(
        &self,
        prefix: &str,
        model_path: &str,
        proving_key_path: &str,
        srs_path: &str,
        proof_witness_path: &str,
        proof_input_path: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let model_path = PathBuf::from(format!("{}/{}", prefix, model_path));
        let proving_key_path = PathBuf::from(format!("{}/{}", prefix, proving_key_path));
        let srs_path = PathBuf::from(format!("{}/{}", prefix, srs_path));
        let proof_input_path = PathBuf::from(format!("{}/{}", prefix, proof_input_path));
        let proof_witness_path = PathBuf::from(format!("{}/{}", prefix, proof_witness_path));

        let input_string = fs::read_to_string(proof_input_path)?;

        let _ = run(GenWitness {
            data: Some(ezkl::commands::DataField(input_string)),
            compiled_circuit: Some(model_path.clone()),
            output: Some(proof_witness_path.clone()),
            vk_path: None,
            srs_path: Some(srs_path.clone()),
        })
        .await?;

        let proof = run(Prove {
            witness: Some(proof_witness_path),
            compiled_circuit: Some(model_path),
            pk_path: Some(proving_key_path),
            proof_path: None,
            srs_path: Some(srs_path),
            proof_type: (ezkl::pfsys::ProofType::Single),
            check_mode: None,
        })
        .await?;

        Ok(proof)
    }

    /// Takes input and performs inference on the model currently loaded into the miner. Fails if `init_model` has not been called. Should be called for the vast majority of inference requests.
    ///
    /// # Arguments
    /// * `&self`
    /// * `prefix` - The directory for operations on NZK related files
    /// * `model_path` - The path to the model currently loaded into the miner
    /// * `srs_path` - The path to the srs currently loaded into the miner
    /// * `witness_path` - The path to the witness currently loaded into the miner
    /// * `input_data` - The input used to run inference on the model in circuit form
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

        println!("Generating inference result for: {}", input_data);

        let witness = run(GenWitness {
            data: Some(ezkl::commands::DataField(input_data)),
            compiled_circuit: Some(model_path),
            output: None,
            vk_path: None,
            srs_path: None,
        })
        .await?;

        Ok(witness)
    }
}
