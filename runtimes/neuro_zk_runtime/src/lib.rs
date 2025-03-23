use std::{fs::File, path::PathBuf};
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::Keypair;
use tokio::sync::mpsc::{Receiver, Sender};
use ezkl::{
    commands::Commands::{
        CompileCircuit, GenSettings, GetSrs, GenWitness, Prove, Setup
    }, execute::run, Commitments, RunArgs
};
use serde_json::Value;

#[derive(Debug)]
pub struct NeuroZKEngine {
    model_path: Option<PathBuf>,
    compiled_model_path: Option<PathBuf>,
    settings_path: Option<PathBuf>,
    srs_path: Option<PathBuf>,
    current_witness_path: Option<PathBuf>,
    proving_key_path: Option<PathBuf>,
    subxt_api: Option<OnlineClient<PolkadotConfig>>,
    signer_keypair: Keypair
}

//const MODEL_PATH: &str = "model.onnx";
//const SETTINGS_PATH: &str = "settings.json";
//const COMPILED_MODEL_PATH: &str = "compiled_model.ezkl";

impl NeuroZKEngine {
    /// Checks if a compiled model is already present. If it is, it will continue with the `run` function, otherwise it will attempt to re-compile the model.
    /// 
    pub async fn start_neuro_zk(
        &self, 
        model_path: PathBuf,
        compiled_model_path: PathBuf,
        settings_path: PathBuf,
        srs_path: PathBuf,
        current_witness_path: PathBuf,
        proving_key_path: PathBuf,
        subxt_api: OnlineClient<PolkadotConfig>,
        signer_keypair: Keypair,
        input: Receiver<Value>,
        output: Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {

        let fresh_start_files = [model_path];
        let restart_files = [compiled_model_path, settings_path, srs_path];

        // Tries to open all files necessary for neuro zk to run. If it fails it will re-generate the files needed.
        if restart_files.iter().all(|file| File::open(file).is_ok()) {
            let _ = self.run(input, output).await;
        } else if fresh_start_files.iter().all(|file| File::open(file).is_ok()) {
            let _ = self.setup().await; 
            let _ = self.run(input, output).await;
        } else {
            return Err("Could not find necessary files for neuro zk to run".into())
        }

        Ok(())
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
    async fn run(
        &self, 
        mut request_stream: Receiver<Value>,
        response_stream: Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(_), Some(_)) = (&self.compiled_model_path, &self.settings_path) {
            while let Some(request) = request_stream.recv().await {
                println!("Processing inference for request: {}", request);
            
                let result = self.run_inference(serde_json::to_string(&request).ok()).await?;
                response_stream.send(result).await?;
            }
        }
        
        Ok(())
    }

    /// Generates the required files to prove inference and compiles the model from `.ONNX` format into an `.ezkl` circuit.
    /// 
    /// # Arguments
    /// * `&self`
    /// 
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    async fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = run(GenSettings { 
            model: self.model_path.clone(), 
            settings_path: self.settings_path.clone(), 
            args: (RunArgs::default()) 
        }).await?;

        let _ = run(CompileCircuit { 
            model: self.model_path.clone(), 
            compiled_circuit: self.compiled_model_path.clone(), 
            settings_path: self.settings_path.clone(), 
        }).await?;

        let _ = run(GetSrs { 
            srs_path: self.srs_path.clone(), 
            settings_path: self.settings_path.clone(), 
            logrows: None, 
            commitment: Some(Commitments::KZG),
        }).await?;

        let _ = run(Setup { 
            compiled_circuit: self.compiled_model_path.clone(), 
            srs_path: self.srs_path.clone(), 
            vk_path: self.vk_path.clone(), 
            pk_path: self.pk_path.clone(), 
            witness: None, 
            disable_selector_compression: None 
        }).await?;

        Ok(())
    }

    /*

    /// Takes input and proves inference on the model currently loaded into the miner. Fails if `init_model` has not been called. Should be called intermittently to request a proof of correct model execution.
    /// 
    /// # Arguments
    /// * `model_location` - The location of the model currently loaded into the miner
    /// 
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    async fn prove_inference(&self) -> Result<(), Box<dyn std::error::Error>> {
        let proof = run(Prove { 
            witness: (self.current_witness_path.clone()), 
            compiled_circuit: (self.compiled_model_path.clone()), 
            pk_path: (self.proving_key_path.clone()), 
            proof_path: (None), 
            srs_path: (self.srs_path.clone()), 
            proof_type: (ezkl::pfsys::ProofType::Single), 
            check_mode: (None) 
        }).await?;

        let proof_submission_tx = substrate_interface::api::tx()
            .task_management()
            .submit_completed_task(
                task_id, 
                completed_hash, 
                result_cid, 
            );

        let proof_submission_events= api
            .tx()
            .sign_and_submit_then_watch_default(&result_submission_tx, signer_keypair)
            .await
            .map(|e| {
                println!("Result submitted, waiting for transaction to be finalized...");
                e
            })?
            .wait_for_finalized_success()
            .await?;

        let submission_event = 
            proof_submission_events.find_first::<substrate_interface::api::task_management::events::SubmittedCompletedTask>()?;
        if let Some(event) = submission_event {
            println!("Task submitted successfully: {event:?}");
        } else {
            println!("Task submission failed");
        }

        Ok(()) 
    }

    */

    /// Takes input and performs inference on the model currently loaded into the miner. Fails if `init_model` has not been called. Should be called for the vast majority of inference requests.
    /// 
    /// # Arguments
    /// * `&self`
    /// * `data` - The input used to run inference on the model in circuit form
    /// 
    /// # Returns
    /// `Result<(), Box<dyn std::error::Error>>`
    async fn run_inference(
        &self, 
        data: Option<String>,  
    ) -> Result<String, Box<dyn std::error::Error>> {
        
        let witness = run(GenWitness { 
            data, 
            compiled_circuit: self.model_path.clone(), 
            output: (None), 
            vk_path: (None), 
            srs_path: (self.srs_path.clone()) 
        }).await?;

        Ok(witness)
    }
}