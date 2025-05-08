use crate::{
    error::Result,
    parachain_interactor::{
        cess_interactor, config, event_processor, logs, registration, task_management,
    },
    parent_runtime::{inference, proof},
    types::{Miner, ParentRuntime, TaskType},
};
use async_trait::async_trait;
use std::path::PathBuf;
use subxt::events::EventDetails;
use subxt::PolkadotConfig;

#[async_trait]
pub trait InferenceServer {
    /// Starts performing inference, selecting the correct inference engine based on the task type
    ///
    /// # Arguments
    /// * `input` - An `impl Stream<Item = Result<Message, tungstenite::Error>> + Unpin` representing the input stream of messages.
    ///
    /// # Returns
    /// An `impl Stream<Item = Result<Message, tungstenite::Error>>` representing the output stream of messages.
    async fn perform_inference(&self) -> Result<()>;

    /// Generates a zkml proof for the model currently in execution.
    ///
    /// # Returns
    /// A `Result` containing a vector of bytes representing the proof.
    async fn generate_proof(&self) -> Result<Vec<u8>>;
}

#[async_trait]
impl InferenceServer for ParentRuntime {
    async fn perform_inference(&self) -> Result<()> {
        inference::spawn_inference_server(&self.task, self.port).await
    }

    async fn generate_proof(&self) -> Result<Vec<u8>> {
        proof::generate_proof().await
    }
}

#[async_trait]
/// A trait for blockchain client operations, such as registering a worker, starting mining sessions, and processing events.
///
/// Provides an asynchronous API for interacting with a blockchain, which enables clients to register workers,
/// initiate mining sessions, and handle blockchain events with asynchronous operations.
pub trait ParachainInteractor {
    /// Confirms the registration of a worker node on the blockchain.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(true)` if successful, or an `Error` if confirmation fails.
    async fn confirm_registration(&self) -> Result<bool>;

    /// Registers a worker node on the blockchain.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if successful, or an `Error` if registration fails.
    async fn register_miner(&self) -> Result<()>;

    /// Starts a miner by subscribing to events and listening to finalized blocks.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the session starts successfully, or an `Error` if it fails.
    async fn start_miner(&mut self) -> Result<()>;

    /// Processes an event received from the blockchain.
    ///
    /// # Arguments
    /// * `event` - A reference to an `EventDetails` object containing details of the blockchain event.
    ///
    /// # Returns
    /// An `Option<String>` containing relevant information derived from the event, or `None` if no information is extracted.
    async fn process_event(&mut self, event: &EventDetails<PolkadotConfig>) -> Result<()>;

    /// Submits a zkml (Zero Knowledge Machine Learning) proof to the blockchain.
    ///
    /// # Arguments
    /// * `proof` - A `Vec<u8>` containing the zkml proof.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the result is successfully submitted, or an `Error` if it fails.
    async fn submit_zkml_proof(&self, proof: Vec<u8>) -> Result<()>;

    /// Downloads a model archive (containing the model and potential additional data eg. proving key) from CESS
    ///
    /// # Arguments
    /// * `fid` - A `&str` representing the CESS fid (fiile ID) of the model archive
    ///
    /// # Returns
    /// A `Result` containing `Ok(())` if the model archive is successfully downloaded, or an `Error` if it fails.
    async fn download_model_archive(&mut self, cid: &str, task_type: TaskType) -> Result<()>;

    /// Vacates a miner erasing current user data and resetting the miner state.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the session vacates successfully, or an `Error` if it fails.
    async fn stop_task_and_vacate_miner(&self) -> Result<()>;

    /// Writes a message to the log.
    ///
    /// # Arguments
    /// * `message` - A `&str` representing the message to be written to the log.
    fn write_log(&self, message: &str);

    /// Resets the log so that the log file is empty when a new task is assinged for execution.
    fn reset_log(&self);

    /// Attempts to update the worker config file.
    ///
    /// # Arguments
    /// * `file_path` - A `&str` representing the path to the config file.
    /// * `content` - A `&str` representing the content to be written to the config file.
    fn update_config_file(&self, path: &PathBuf, content: &str) -> Result<()>;
}

/// Implementation of `ParachainInteractor` trait for `Miner`.
#[async_trait]
impl ParachainInteractor for Miner {
    async fn confirm_registration(&self) -> Result<bool> {
        registration::confirm_registration().await
    }

    async fn register_miner(&self) -> Result<()> {
        registration::register_miner(self).await
    }

    async fn start_miner(&mut self) -> Result<()> {
        registration::start_miner(self).await
    }

    async fn process_event(&mut self, event: &EventDetails<PolkadotConfig>) -> Result<()> {
        event_processor::process_event(self, event).await
    }

    async fn stop_task_and_vacate_miner(&self) -> Result<()> {
        task_management::stop_task_and_vacate_miner().await
    }

    async fn submit_zkml_proof(&self, proof: Vec<u8>) -> Result<()> {
        task_management::submit_zkml_proof(proof).await
    }

    async fn download_model_archive(&mut self, cess_fid: &str, task_type: TaskType) -> Result<()> {
        cess_interactor::download_model_archive(self, cess_fid).await
    }

    fn write_log(&self, message: &str) {
        logs::write_log(self, message);
    }

    fn reset_log(&self) {
        logs::reset_log(self);
    }

    fn update_config_file(&self, path: &PathBuf, content: &str) -> Result<()> {
        config::update_config_file(self, path, content)
    }
}
