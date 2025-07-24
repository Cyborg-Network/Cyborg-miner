use crate::{
    error::Result,
    parachain_interactor::{
        behavior_control, event_processor, identity, registration::{self, RegistrationStatus}, task_management,
    },
    parent_runtime::{cess_interactor, inference, proof},
    types::{CurrentTask, Miner, ParentRuntime},
};
use async_trait::async_trait;
use subxt::events::EventDetails;
use subxt::PolkadotConfig;
use tokio::task::JoinHandle;

#[async_trait]
pub trait InferenceServer {
    /// Downloads a model archive (containing the model and potential additional data eg. proving key) from CESS
    ///
    /// # Arguments
    /// * `fid` - A `&str` representing the CESS fid (fiile ID) of the model archive
    ///
    /// # Returns
    /// A `Result` containing `Ok(())` if the model archive is successfully downloaded, or an `Error` if it fails.
    async fn download_model_archive(&self, fid: &str, cipher: &str) -> Result<()>;

    /// Starts performing inference, selecting the correct inference engine based on the task type
    ///
    /// # Arguments
    /// * `input` - An `impl Stream<Item = Result<Message, tungstenite::Error>> + Unpin` representing the input stream of messages.
    ///
    /// # Returns
    /// An `impl Stream<Item = Result<Message, tungstenite::Error>>` representing the output stream of messages.
    async fn spawn_inference_server(&self, current_task: &CurrentTask) -> Result<JoinHandle<()>>;

    /// Generates a zkml proof for the model currently in execution.
    ///
    /// # Returns
    /// A `Result` containing a vector of bytes representing the proof.
    async fn generate_proof(&self) -> Result<Vec<u8>>;
}

#[async_trait]
impl InferenceServer for ParentRuntime {
    async fn download_model_archive(&self, cess_fid: &str, cipher: &str) -> Result<()> {
        cess_interactor::download_model_archive(cess_fid, cipher).await
    }

    async fn spawn_inference_server(&self, current_task: &CurrentTask) -> Result<JoinHandle<()>> {
        inference::spawn_inference_server(current_task, self.port).await
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
    async fn confirm_registration(&self) -> Result<RegistrationStatus>;

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

    /// Vacates a miner erasing current user data and resetting the miner state.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the session vacates successfully, or an `Error` if it fails.
    async fn stop_task_and_vacate_miner(&self) -> Result<()>;

    /// Attempts to update the miner identity file.
    ///
    /// # Arguments
    /// * `file_path` - A `&str` representing the path to the config file.
    /// * `content` - A `&str` representing the content to be written to the config file.
    fn update_identity_file(&self, path: &str, content: &str) -> Result<()>;

    //TODO this might also notify the user that the miner has been corrupted and that the current task should be pulled
    /// Suspends the miner by sending a transaction to the parachain that deactivates the miner for further tasks..
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the miner is successfully suspended, or an `Error` if it fails.
    async fn suspend_miner(&self) -> Result<()>;
}

/// Implementation of `ParachainInteractor` trait for `Miner`.
#[async_trait]
impl ParachainInteractor for Miner {
    async fn confirm_registration(&self) -> Result<RegistrationStatus> {
        registration::confirm_registration(self).await
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
        task_management::submit_zkml_proof(self, proof).await
    }

    fn update_identity_file(&self, path: &str, content: &str) -> Result<()> {
        identity::update_identity_file(path, content)
    }

    async fn suspend_miner(&self) -> Result<()> {
        behavior_control::suspend_miner(self).await
    }
}
