use async_trait::async_trait;
use pinata_sdk::PinByJson;
use pinata_sdk::PinataApi;
use sp_core::blake2_256;
//use zip::unstable::write;
//use std::path;
use crate::substrate_interface::api::runtime_types::cyborg_primitives::worker::WorkerType;
use crate::utils::substrate_queries::{get_task, CyborgTask};
use crate::utils::substrate_transactions::submit_tx;
use crate::utils::substrate_transactions::TransactionType;
use chrono::Local;
use fs2::FileExt;
use std::path::PathBuf;
use std::process::Output;
use subxt::events::EventDetails;
use subxt::utils::H256;
//use subxt::ext::jsonrpsee::async_client::ClientBuilder;

use codec::{Decode, Encode};
use log::info;
//use sc_client_api::BlockchainEvents;
use serde::{Deserialize, Serialize};
//use sp_api::ProvideRuntimeApi;
//use sp_blockchain::HeaderBackend;
//use sp_runtime::traits::Block;
use reqwest::get;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::Keypair;

use crate::{
    error::{Error, Result},
    specs, substrate_interface,
};

// Datastructure for worker registration persistence
#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, Serialize, Deserialize)]
pub struct WorkerData {
    pub worker_owner: String,
    pub worker_identity: (AccountId32, u64),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskOwner {
    pub task_owner: String,
}

#[allow(dead_code)]
enum ExecutionType {
    Execution,
    Verfication,
    Resolution,
}

pub struct WorkerConfig {
    pub domain: BoundedVec<u8>,
    pub latitude: i32,
    pub longitude: i32,
    pub ram: u64,
    pub storage: u64,
    pub cpu: u16,
}

#[derive(Deserialize)]
pub struct IpResponse {
    pub ip: String,
}

#[async_trait]
/// A trait for blockchain client operations, such as registering a worker, starting mining sessions, and processing events.
///
/// Provides an asynchronous API for interacting with a blockchain, which enables clients to register workers,
/// initiate mining sessions, and handle blockchain events with asynchronous operations.
pub trait BlockchainClient {
    /// Registers a worker node on the blockchain.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if successful, or an `Error` if registration fails.
    async fn register_worker(&self) -> Result<()>;

    /// Starts a mining session on the blockchain by subscribing to events and listening to finalized blocks.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the session starts successfully, or an `Error` if it fails.
    async fn start_mining_session(&mut self) -> Result<()>;

    /// Processes an event received from the blockchain.
    ///
    /// # Arguments
    /// * `event` - A reference to an `EventDetails` object containing details of the blockchain event.
    ///
    /// # Returns
    /// An `Option<String>` containing relevant information derived from the event, or `None` if no information is extracted.
    async fn process_event(&mut self, event: &EventDetails<PolkadotConfig>) -> Result<()>;

    /// Calls functions to publish the result on IPFS and publish the result hash onchain.
    ///
    /// # Arguments
    /// * `result` - A reference to an `Output` object containing the result of the task execution.
    /// * `task_id` - A `u64` representing the ID of the task that was executed.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the result is successfully submitted, or an `Error` if it fails.
    async fn process_execution_result(&self, result: Output, task_id: u64) -> Result<()>;

    /// Publishes a result on IPFS.
    ///
    /// # Arguments
    /// * `result` - A `String` containing the result of the task execution.
    /// * `ipfs_client` - A reference to an `PinataApi` object for interacting with IPFS.
    ///
    /// # Returns
    /// A `String` containing the IPFS CID where the result is stored.
    async fn publish_on_ipfs(&self, result: String, ipfs_client: &PinataApi) -> Result<String>;

    /// Submits a result to the blockchain.
    ///
    /// # Arguments
    /// * `cid` - A `String` containing the IPFS CID where the result is stored, returned by `publish_on_ipfs`.
    /// * `task_id` - A `u64` representing the ID of the task that was executed.
    /// * `result` - A `String` containing the result of the task execution.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the result is successfully submitted, or an `Error` if it fails.
    async fn submit_to_chain(&self, cid: String, task_id: u64, result: String) -> Result<()>;

    /// Downloads and executes a work package from IPFS.
    ///
    /// # Arguments
    /// * `cid` - A `&str` representing the IPFS CID of the work package.
    ///
    /// # Returns
    /// A `Result` containing the `Output` of the executed work package, or an `Error` if it fails.
    async fn download_and_execute_work_package(&self, cid: &str) -> Result<std::process::Output>;

    /// Sets the current task being processed by the worker.
    ///
    /// # Arguments
    /// * `task_id` - A `u64` representing the ID of the task being processed.
    /// * `task_ipfs_cid_bytes` - A `Vec<u8>` representing the IPFS CID of the task.
    /// * `task_owner` - A `AccountId32` representing the owner of the task.
    fn prepare_for_task_execution(
        &mut self,
        task_id: u64,
        task_cid: String,
        task_owner: AccountId32,
    ) -> Result<()>;

    /// Executes a task in different ways, depending on what role (execution, verification, resolution) was assigned to the worker.
    ///
    /// # Arguments
    /// * `task` - A `Task` object representing the task to be processed.
    /// * `execution_type` - An `ExecutionType` indicating the role assigned to the worker.
    ///
    /// # Returns
    /// A `Result` indicating `Ok(())` if the task is processed successfully, or an `Error` if it fails.
    async fn process_task(
        &self,
        task_owner: &AccountId32,
        task_ipfs_cid_bytes: &String,
    ) -> Result<std::process::Output>;

    /// Calls `process_task` with `ExecutionType::Execution`. Calls `submit_result_onchain` with the result.

    async fn execute_task(&self) -> Result<()>;

    async fn verify_task(&self) -> Result<()>;

    async fn resolve_task(&self) -> Result<()>;

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

/// Represents a client for interacting with the Cyborg blockchain.
///
/// This struct is used to interact with the Cyborg blockchain, manage key pairs,
/// and optionally communicate with IPFS or node URIs.
pub struct CyborgClient {
    pub(crate) client: OnlineClient<PolkadotConfig>,
    pub(crate) keypair: Keypair,
    pub ipfs_client: PinataApi,
    #[allow(dead_code)]
    pub node_uri: String,
    pub identity: (AccountId32, u64),
    #[allow(dead_code)]
    pub creator: AccountId32,
    pub log_path: PathBuf,
    pub task_path: PathBuf,
    pub config_path: PathBuf,
    pub task_owner_path: PathBuf,
    pub current_task: Option<CyborgTask>,
    pub current_ip: String,
}

impl CyborgClient {
    async fn check_ip_change(&mut self) -> Result<bool> {
        let new_ip = self.get_current_ip().await?;
        if new_ip != self.current_ip {
            self.current_ip = new_ip;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn get_current_ip(&self) -> Result<String> {
        Ok(reqwest::get("https://api.ipify.org?format=json")
            .await?
            .json::<IpResponse>()
            .await?
            .ip)
    }

    #[allow(dead_code)]
    async fn handle_ip_change(&self) -> Result<()> {
        // Stop mining by returning an error that will break the mining loop
        Err(Error::Custom("IP address changed - stopping worker".into()))
    }
}

/// Implementation of the `BlockchainClient` trait for `CyborgClient`.
#[async_trait]
impl BlockchainClient for CyborgClient {
    /// Registers a worker with the Cyborg parachain.
    ///
    /// # Returns
    /// A `Result` indicating success or an error if registration fails.
    async fn register_worker(&self) -> Result<()> {
        let worker_specs = specs::gather_worker_spec().await?;

        let worker_registration = substrate_interface::api::tx()
            .edge_connect()
            .register_worker(
                WorkerType::Executable,
                worker_specs.domain,
                worker_specs.latitude,
                worker_specs.longitude,
                worker_specs.ram,
                worker_specs.storage,
                worker_specs.cpu,
            );

        println!("Transaction Details:");
        println!("Module: {:?}", worker_registration.pallet_name());
        println!("Call: {:?}", worker_registration.call_name());
        println!("Parameters: {:?}", worker_registration.call_data());

        let worker_registration_events = self
            .client
            .tx()
            .sign_and_submit_then_watch_default(&worker_registration, &self.keypair)
            .await
            .map(|e| {
                println!(
                    "Worker registration submitted, waiting for transaction to be finalized..."
                );
                e
            })?
            .wait_for_finalized_success()
            .await?;

        let registration_event = worker_registration_events
            .find_first::<substrate_interface::api::edge_connect::events::WorkerRegistered>(
        )?;

        if let Some(event) = registration_event {
            let worker_file_json = serde_json::to_string(&WorkerData {
                worker_owner: event.creator.clone().to_string(),
                worker_identity: event.worker.clone(),
            })?;

            self.update_config_file(&self.config_path, &worker_file_json)?;

            println!("Worker registered successfully: {event:?}");
        } else {
            println!("Worker registration failed");
        }

        Ok(())
    }

    // for event in events.iter() {
    //     match event {
    //         Ok(ev) => {
    //             if let Err(e) = self.process_event(&ev).await {
    //                 println!("Error processing event: {:?}", e);
    //             }
    //         }
    //         Err(e) => eprintln!("Error decoding event: {:?}", e),
    //     }
    // }

    /// Starts a mining session by subscribing to finalized blocks and listening for events.
    ///
    /// # Returns
    /// A `Result` indicating success or an error if starting the session fails.
    async fn start_mining_session(&mut self) -> Result<()> {
        println!("Starting mining session...");
        self.write_log("Waiting for tasks...");

        info!("============ event_listener_tester ============");

        // Initialize current IP
        self.current_ip = self.get_current_ip().await?;
        self.write_log(&format!("Initial IP address: {}", self.current_ip));

        let mut blocks = self.client.blocks().subscribe_finalized().await?;
        let mut ip_check_interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Check every minute

        loop {
            tokio::select! {
                // IP check branch
                _ = ip_check_interval.tick() => {
                    match self.check_ip_change().await {
                        Ok(true) => {
                            self.write_log(&format!("IP address changed to: {}", self.current_ip));
                            self.write_log("Stopping worker due to IP address change");
                            return Err(Error::Custom("IP address changed - stopping worker".into()));
                        }
                        Ok(false) => {
                            // IP unchanged, continue mining
                        }
                        Err(e) => {
                            self.write_log(&format!("Error checking IP address: {}", e));
                            // Continue mining despite IP check failure
                        }
                    }
                }

                // Transaction processing branch
                _ = async {
                    if let Err(e) = crate::utils::substrate_transactions::process_transactions(
                        &self.client,
                        &self.keypair,
                    )
                    .await
                    {
                        println!("Error processing transactions: {:?}", e);
                        self.write_log(&format!("Error processing transactions: {}", e));
                    }
                } => {}

                // Block processing branch
                block = blocks.next() => {
                    match block {
                        Some(Ok(block)) => {
                            let events = match block.events().await {
                                Ok(events) => events,
                                Err(e) => {
                                    self.write_log(&format!("Error getting block events: {}", e));
                                    continue;
                                }
                            };

                            for event in events.iter() {
                                match event {
                                    Ok(ev) => {
                                        if let Err(e) = self.process_event(&ev).await {
                                            self.write_log(&format!("Error processing event: {}", e));
                                        }
                                    }
                                    Err(e) => {
                                        self.write_log(&format!("Error decoding event: {}", e));
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            self.write_log(&format!("Error receiving block: {}", e));
                        }
                        None => {
                            self.write_log("Block subscription ended");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Processes an event from the blockchain.
    ///
    /// # Arguments
    /// * `event` - A reference to an `EventDetails` object containing event information.
    ///
    /// # Returns
    /// An `Option<String>` that may contain information derived from the event.
    async fn process_event(&mut self, event: &EventDetails<PolkadotConfig>) -> Result<()> {
        // subscription_builder.subscribe_to::<cyborg_node::pallet_task_management::events::TaskScheduled>();
        // subscription_builder.subscribe_to::<cyborg_node::pallet_task_management::events::SubmittedCompletedTask>();
        // subscription_builder.subscribe_to::<cyborg_node::pallet_task_management::events::VerifierResolverAssigned>();
        // subscription_builder.subscribe_to::<cyborg_node::pallet_task_management::events::VerifiedCompletedTask>();
        // subscription_builder.subscribe_to::<cyborg_node::pallet_task_management::events::ResolvedCompletedTask>();
        // subscription_builder.subscribe_to::<cyborg_node::pallet_task_management::events::TaskReassigned>();

        // Check for WorkerRegistered event
        match event.as_event::<substrate_interface::api::edge_connect::events::WorkerRegistered>() {
            Ok(Some(worker_registered)) => {
                let creator = &worker_registered.creator;
                let worker = &worker_registered.worker;
                let domain = &worker_registered.domain;

                println!(
                    "Worker Registered: Creator: {:?}, Worker: {:?}, Domain: {:?}",
                    creator, worker, domain
                );
            }
            Err(e) => {
                println!("Error decoding WorkerRegistered event: {:?}", e);
                return Err(Error::Subxt(e.into()));
            }
            _ => {} // Skip non-matching events
        }

        // Check for WorkerRemoved event
        match event.as_event::<substrate_interface::api::edge_connect::events::WorkerRemoved>() {
            Ok(Some(worker_removed)) => {
                let creator = &worker_removed.creator;
                let worker_id = &worker_removed.worker_id;

                println!(
                    "Worker Removed: Creator: {:?}, Worker ID: {:?}",
                    creator, worker_id
                );
            }
            Err(e) => {
                println!("Error decoding WorkerRemoved event: {:?}", e);
                return Err(Error::Subxt(e.into()));
            }
            _ => {} // Skip non-matching events
        }

        // Check for WorkerStatusUpdated event
        match event
            .as_event::<substrate_interface::api::edge_connect::events::WorkerStatusUpdated>()
        {
            Ok(Some(status_updated)) => {
                let creator = &status_updated.creator;
                let worker_id = &status_updated.worker_id;
                let worker_status = &status_updated.worker_status;

                println!(
                    "Worker Status Updated: Creator: {:?}, Worker ID: {:?}, Status: {:?}",
                    creator, worker_id, worker_status
                );
            }
            Err(e) => {
                println!("Error decoding WorkerStatusUpdated event: {:?}", e);
                return Err(Error::Subxt(e.into()));
            }
            _ => {} // Skip non-matching events
        }

        // Check for TaskScheduled event
        match event.as_event::<substrate_interface::api::task_management::events::TaskScheduled>() {
            Ok(Some(task_scheduled)) => {
                let assigned_worker = &task_scheduled.assigned_worker;

                if *assigned_worker == self.identity {
                    let task_cid_string = String::from_utf8(task_scheduled.task.0)?;

                    self.write_log(
                        format!("New task scheduled for worker: {}", task_cid_string).as_str(),
                    );

                    self.prepare_for_task_execution(
                        task_scheduled.task_id,
                        task_cid_string,
                        task_scheduled.task_owner,
                    )?;

                    match self.execute_task().await {
                        Ok(output) => {
                            self.write_log(format!("Operation sucessful: {:?}", output).as_str());
                        }
                        Err(e) => {
                            self.write_log(format!("Failed to execute command: {}", e).as_str());
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error decoding WorkerStatusUpdated event: {:?}", e);
                return Err(Error::Subxt(e.into()));
            }
            _ => {} // Skip non-matching events
        }

        // Check for SubmittedCompletedTask event to check if worker was assigned to verify task
        match event
            .as_event::<substrate_interface::api::task_management::events::SubmittedCompletedTask>()
        {
            Ok(Some(submitted_task)) => {
                let assigned_verifier = &submitted_task.assigned_verifier;

                if *assigned_verifier == self.identity {
                    let task = get_task(&self.client, submitted_task.task_id).await?;

                    self.prepare_for_task_execution(task.id, task.cid, task.owner)?;

                    match self.verify_task().await {
                        Ok(output) => {
                            self.write_log(format!("Operation sucessful: {:?}", output).as_str());
                        }
                        Err(e) => {
                            self.write_log(format!("Failed to execute command: {}", e).as_str());
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error decoding SubmittedCompletedTask event: {:?}", e);
                return Err(Error::Subxt(e.into()));
            }
            _ => {} // Skip non-matching events
        }

        // Check for VerifierResolverAssigned event to check if worker was assigned to resolve task
        match event.as_event::<substrate_interface::api::task_management::events::VerifierResolverAssigned>() {
            Ok(Some(verified_task)) => {
                let assigned_resolver = &verified_task.assigned_resolver;

                if *assigned_resolver == self.identity {
                    let task = get_task(&self.client, verified_task.task_id).await?;

                    self.prepare_for_task_execution(task.id, task.cid, task.owner)?;

                    match self.resolve_task().await {
                        Ok(output) => {
                            self.write_log(format!("Task resolution sucessful: {:?}", output).as_str());
                        }
                        Err(e) => {
                            self.write_log(format!("Task resolution failed: {}", e).as_str());
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error decoding VerifierResolverAssigned event: {:?}", e);
                return Err(Error::Subxt(e.into()));
            }
            _ => {} // Skip non-matching events
        }

        Ok(())
    }

    fn prepare_for_task_execution(
        &mut self,
        task_id: u64,
        task_cid: String,
        task_owner: AccountId32,
    ) -> Result<()> {
        let owner_file_json = serde_json::to_string(&TaskOwner {
            task_owner: task_owner.clone().to_string(),
        })?;

        self.update_config_file(&self.task_owner_path, &owner_file_json)?;

        self.reset_log();

        self.current_task = Some(CyborgTask {
            id: task_id,
            cid: task_cid,
            owner: task_owner,
        });

        Ok(())
    }

    async fn execute_task(&self) -> Result<()> {
        if let Some(task) = &self.current_task {
            self.write_log("New task assigned for execution, processing...");

            let task_result = self.process_task(&task.owner, &task.cid).await?;

            if let Ok(()) = self.process_execution_result(task_result, task.id).await {
                println!("Result submitted to chain successfully");
                Ok(())
            } else {
                println!("Failed to submit result to chain");
                Err("Failed to submit result to chain".into())
            }
        } else {
            Err("No current task".into())
        }
    }

    async fn verify_task(&self) -> Result<()> {
        if let Some(task) = &self.current_task {
            self.write_log("New task assigned for verification, processing...");

            let task_result = self.process_task(&task.owner, &task.cid).await?;

            let result_raw_data = String::from_utf8(task_result.stdout)?;

            let completed_hash = H256::from(blake2_256(result_raw_data.as_bytes()));

            let task_id = task.id;

            submit_tx(
                &self.client,
                &self.keypair,
                TransactionType::SubmitResultVerification {
                    completed_hash,
                    task_id,
                },
            )
            .await?;

            Ok(())
        } else {
            Err("No current task".into())
        }
    }

    async fn resolve_task(&self) -> Result<()> {
        if let Some(task) = &self.current_task {
            self.write_log("New task assigned for resolution, processing...");

            let task_result = self.process_task(&task.owner, &task.cid).await?;

            let result_raw_data = String::from_utf8(task_result.stdout)?;

            let completed_hash = H256::from(blake2_256(result_raw_data.as_bytes()));

            let task_id = task.id;

            submit_tx(
                &self.client,
                &self.keypair,
                TransactionType::SubmitResultResolution {
                    completed_hash,
                    task_id,
                },
            )
            .await?;

            Ok(())
        } else {
            Err("No current task".into())
        }
    }

    async fn process_task(
        &self,
        task_owner: &AccountId32,
        task_ipfs_cid_bytes: &String,
    ) -> Result<std::process::Output> {
        self.reset_log();

        self.write_log("New task assigned for execution, processing...");

        let task_owner = task_owner;

        let owner_file_json = serde_json::to_string(&TaskOwner {
            task_owner: task_owner.clone().to_string(),
        })?;

        let config_dir_path = Path::new("/var/lib/cyborg/worker-node/config");
        let file_path = config_dir_path.join("task_owner.json");

        if !fs::metadata(&config_dir_path).is_ok() {
            fs::create_dir_all(&config_dir_path)?;
        }

        // Write content to the file (will overwrite existing content)
        fs::write(&file_path, owner_file_json)?;

        //let task_ipfs_cid = String::from_utf8_lossy(task_ipfs_cid_bytes);

        //println!("Ipfs hash: {:?}", task_ipfs_cid);

        let result = self
            .download_and_execute_work_package(&task_ipfs_cid_bytes)
            .await;

        result
    }

    async fn process_execution_result(&self, result: Output, task_id: u64) -> Result<()> {
        dbg!(&result);
        let result_raw_data = String::from_utf8(result.stdout)?;
        dbg!(&result_raw_data);

        self.write_log(format!("Result: {}", &result_raw_data).as_str());

        self.write_log("Submitting result onchain...");

        let cid = self
            .publish_on_ipfs(result_raw_data.clone(), &self.ipfs_client)
            .await?;

        let _ = self.submit_to_chain(cid, task_id, result_raw_data).await?;

        println!("Result submitted to chain successfully");
        self.write_log("Result submitted to chain successfully!");

        Ok(())
    }

    async fn publish_on_ipfs(&self, result: String, ipfs_client: &PinataApi) -> Result<String> {
        println!("Publishing on IPFS: {:?}", result);

        let ipfs_res = ipfs_client.pin_json(PinByJson::new(result)).await?;

        println!("Published on IPFS: {:?}", ipfs_res);

        Ok(ipfs_res.ipfs_hash)
    }

    async fn submit_to_chain(
        &self,
        result: String,
        task_id: u64,
        task_output: String,
    ) -> Result<()> {
        let result_cid: BoundedVec<u8> = BoundedVec::from(BoundedVec(result.as_bytes().to_vec()));

        let completed_hash = H256::from(blake2_256(task_output.as_bytes()));

        submit_tx(
            &self.client,
            &self.keypair,
            TransactionType::SubmitResult {
                completed_hash,
                result_cid,
                task_id,
            },
        )
        .await?;

        Ok(())
    }

    #[allow(unused_attributes)]
    #[allow(future_incompatible)]
    async fn download_and_execute_work_package(
        &self,
        ipfs_cid: &str,
    ) -> Result<std::process::Output> {
        println!("Starting download of ipfs hash: {}", ipfs_cid);

        self.write_log(format!("Retrieving work package with cid: {}...", &ipfs_cid).as_str());

        // TODO: validate its a valid ipfs hash
        let url = format!("https://ipfs.io/ipfs/{}", ipfs_cid);

        let response = get(&url).await?;

        if !response.status().is_success() {
            eprintln!("Error: {}", response.status());
            return Err(Error::Custom(format!(
                "Failed to download work package, server responded with {}",
                response.status()
            )));
        }

        if let Some(parent) = &self.task_path.parent() {
            match fs::create_dir_all(parent) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Failed to create directory: {}", e);
                    return Err(Error::Io(e));
                }
            }
        }

        let mut file = File::create(&self.task_path)?;

        let response_bytes = response.bytes().await?;

        println!(
            "Downloaded {} bytes from IPFS gateway.",
            response_bytes.len()
        );

        file.write_all(&response_bytes)?;

        // File needs to be dropped, else there will be a race condition and the file will not be executable
        drop(file);

        let mut perms = fs::metadata(&self.task_path)?.permissions();

        perms.set_mode(perms.mode() | 0o111);

        fs::set_permissions(&self.task_path, perms)?;

        self.write_log("Work package retrieved!");

        self.write_log("Executing work package...");

        let execution = Command::new(&self.task_path)
            .stdout(Stdio::piped())
            .spawn()?;

        // TODO: This only permits the execution of tasks with one ouput - need to establish a standard for measuring intermittent results
        if let Some(output) = execution.wait_with_output().ok() {
            self.write_log("Work package executed!");
            return Ok(output);
        } else {
            return Err(Error::Custom("Failed to execute work package".to_string()));
        }
    }

    #[allow(unused_attributes)]
    #[allow(future_incompatible)]
    fn write_log(&self, message: &str) {
        println!("Log: {}", message);
        if let Ok(mut file) = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.log_path)
        {
            if let Err(e) = file.lock_exclusive() {
                println!("Failed to lock file: {}", e);
                return;
            }

            let now = Local::now();
            let formatted_message = format!("{} - {}\n", now.format("%Y-%m-%d %H:%M:%S"), message);

            if let Err(e) = file.write_all(formatted_message.as_bytes()) {
                println!("Failed to write to file: {}", e);
                return;
            }

            if let Err(e) = file.unlock() {
                println!("Failed to unlock file: {}", e);
                return;
            }
        } else {
            println!("Failed to open file");
            return;
        }
    }

    #[allow(unused_attributes)]
    #[allow(future_incompatible)]
    fn reset_log(&self) {
        if let Ok(file) = File::create(&self.log_path) {
            if let Err(e) = file.lock_exclusive() {
                println!("Failed reset log file: {}", e);
                return;
            }

            if let Err(e) = file.set_len(0) {
                println!("Failed to reset log file: {}", e);
                return;
            }

            if let Err(e) = file.unlock() {
                println!("Failed to reset log file: {}", e);
                return;
            }
        }
    }

    fn update_config_file(&self, path: &PathBuf, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, content)?;
        Ok(())
    }
}

// TODO: Add integration tests as unit tests aren't appropriate here

/*

TODO: Implement function for verifying worker registration once edge-connect pallet is updated, might look something like this

pub async fn verify_worker_registration(
    api: &OnlineClient<PolkadotConfig>,
    worker_data: WorkerData,
    signer: AccountId32
) -> Result<bool, Box<dyn std::error::Error>> {
    let worker_storage_query = cyborg_metadata::storage().edge_connect().worker_clusters(signer.clone(), 0);
    let worker = api
        .storage()
        .at_latest()
        .await?
        .fetch(&worker_storage_query)
        .await?;

    println!("Worker Details: {:?}", worker);

    println!("worker: {:?}", worker);

    // If worker data exists in the storage, decode and verify the domain
    if let Some(_worker) = worker {
        Ok(true)
    } else {
        Ok(false)
    }
}

*/

/*

pub async fn submit_result_onchain(
    api: &OnlineClient<PolkadotConfig>,
    signer_keypair: &Keypair,
    ipfs_client: &PinataApi,
    result: Output,
    task_id: u64,
) {
    dbg!(&result);
    let result_raw_data = String::from_utf8(result.stdout).expect("Invalid UTF-8 output");
    dbg!(&result_raw_data);

    write_log(format!("Result: {}", &result_raw_data).as_str());

    write_log("Submitting result onchain...");

    let cid = publish_on_ipfs(result_raw_data.clone(), ipfs_client).await;
    let chain_result = submit_to_chain(api, signer_keypair, cid, task_id, result_raw_data).await;

    match chain_result {
        Ok(_) => {
            println!("Result submitted to chain successfully");
            write_log("Result submitted to chain successfully!");
        }
        Err(e) => {
            println!("Failed to submit result to chain: {:?}", e);
        }
    }
}

pub async fn publish_on_ipfs(result: String, ipfs_client: &PinataApi) -> String {
    println!("Publishing on IPFS: {:?}", result);

    let ipfs_res = ipfs_client.pin_json(PinByJson::new(result)).await;
    match ipfs_res {
        Ok(res) => {
            println!("Published on IPFS: {:?}", res);
            res.ipfs_hash
        }
        Err(e) => {
            println!("Failed to publish on IPFS: {:?}", e);
            String::new()
        }
    }
}

pub async fn submit_to_chain(api: &OnlineClient<PolkadotConfig>, signer_keypair: &Keypair, result: String, task_id: u64, task_output: String)
    -> Result<(), Box<dyn std::error::Error>>
{
    let result_cid: BoundedVec<u8> = BoundedVec::from(BoundedVec(result.as_bytes().to_vec()));

    let completed_hash = H256::from(blake2_256(task_output.as_bytes()));

    let result_submission_tx = substrate_interface::api::tx()
        .task_management()
        .submit_completed_task(
            task_id,
            completed_hash,
            result_cid,
        );

    println!("Transaction Details:");
    println!("Module: {:?}", result_submission_tx.pallet_name());
    println!("Call: {:?}", result_submission_tx.call_name());
    println!("Parameters: {:?}", result_submission_tx.call_data());

    let result_submission_events= api
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
        result_submission_events.find_first::<substrate_interface::api::task_management::events::SubmittedCompletedTask>()?;
    if let Some(event) = submission_event {
        println!("Task submitted successfully: {event:?}");
    } else {
        println!("Task submission failed");
    }

    Ok(())
}

pub async fn download_and_execute_work_package(
    ipfs_cid: &str,
) -> Option<Result<std::process::Output, std::io::Error>> {
    info!("ipfs_hash: {}", ipfs_cid);
    println!("Starting download of ipfs hash: {}", ipfs_cid);
    info!("============ downloading_file ============");

    write_log("Retrieving work package...");

    // TODO: validate its a valid ipfs hash
    let url = format!("https://ipfs.io/ipfs/{}", ipfs_cid);

    let response = get(&url).await;

    match response {
        Ok(response) => {
            if !response.status().is_success() {
                eprintln!("Error: {}", response.status());
                return None;
            }

            let response_bytes = match response.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Failed to read response bytes: {}", e);
                    return None;
                }
            };

            println!("Downloaded {} bytes from Crust's IPFS gateway.", response_bytes.len());

            let dir_path = Path::new("/var/lib/cyborg/worker-node/packages");
            let file_path = dir_path.join(WORK_PACKAGE_DIR);

            if !dir_path.exists() {
                if let Err(e) = fs::create_dir_all(&dir_path) {
                    eprintln!("Failed to create directory: {}", e);
                    return None;
                }
            }

            let mut file = match File::create(&file_path) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Failed to create file: {}", e);
                    return None;
                }
            };

            if let Err(e) = file.write_all(&response_bytes) {
                eprintln!("Failed to write to file: {}", e);
                return None;
            }

            // File needs to be dropped, else there will be a race condition and the file will not be executable
            drop(file);

            let mut perms = match fs::metadata(&file_path) {
                Ok(meta) => meta.permissions(),
                Err(e) => {
                    eprintln!("Failed to retrieve file metadata: {}", e);
                    return None;
                }
            };
            perms.set_mode(perms.mode() | 0o111);

            if let Err(e) = fs::set_permissions(&file_path, perms) {
                eprintln!("Failed to set file permissions: {}", e);
                return None;
            }

            write_log("Work package retrieved!");

            write_log("Executing work package...");

            match Command::new(&file_path).stdout(Stdio::piped()).spawn() {
                Ok(child_process) => {
                    write_log("Work package executed!");
                    Some(Ok(child_process.wait_with_output().ok()?))
                }
                Err(e) => {
                    eprintln!("Failed to execute command: {}", e);
                    None
            }
        }
        }
        Err(e) => {
            println!("Error: {}", e);
            return None;
        }
    }
}

async fn download_and_extract_zk_files(ipfs_cid: &str) -> Option<ZkFiles> {
    let url = format!("https://ipfs.io/ipfs/{}", ipfs_cid);

    write_log("Retrieving ZK files...");

    let response = get(&url).await.unwrap();

    if !response.status().is_success() {
        return None;
    }

    let bytes = response.bytes().await.unwrap();

    let reader = std::io::Cursor::new(bytes);

    let mut archive = ZipArchive::new(reader).unwrap();

    let mut unpacked_files = ZkFiles {
        zk_public_input: None,
        zk_circuit: None,
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        let filename = file.name().to_string();

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();

        match filename.as_str() {
            "zk_public_input.json" => unpacked_files.zk_public_input = Some(String::from_utf8(buffer).unwrap()),
            "zk_circuit.circom" => unpacked_files.zk_circuit = Some(String::from_utf8(buffer).unwrap()),
            _ => {
                println!("Unexpected file in zip: {}", filename);
            }
        }
    }

    write_log("ZK files retrieved!");

    Some(unpacked_files)
}

/// Can send stages 1-4 of the zk-verification process to the cyborg-agent, which will send it to the frontend
async fn emit_zk_update(stage: u8, connection: &Connection) -> Result<(), Box<dyn Error>> {


    let cxt = SignalEmitter::new(
        connection,
        "/com/cyborg/CyborgAgent",
    )?;

    cxt.emit("com.cyborg.AgentZkInterface", "ZkUpdate", &stage).await?;
    Ok(())
}

async fn wait_and_send_update() -> zbus::Result<()> {

    let connection = Connection::system().await?;

    let well_known_name = BusName::try_from("com.cyborg.CyborgAgent")?;
    connection.request_name(well_known_name).await?;

    let loopvec = [1,2,3,4];

    loop {
        for i in loopvec {
            if let Err(e) = emit_zk_update(i, &connection).await {
                println!("Error while sending signal: {}", e);
            }
            println!("Waiting for 10 seconds...");
            sleep(Duration::from_secs(10)).await;
        }
    }
}

*/

/*

fn worker_retain_after_restart(reg_event: EventWorkerRegistered) -> Option<WorkerData> {
    let registered_worker_data = WorkerData {
        creator: reg_event.creator.to_ss58check(),
        worker: reg_event.worker,
        domain: String::from_utf8_lossy(reg_event.domain.as_bytes_ref()).to_string(),
        domain_encoded: reg_event.domain.into(),
    };

    let registered_worker_json = serde_json::to_string_pretty(&registered_worker_data);
    info!("{:?}", &registered_worker_json);

    use std::{fs::File, path::Path};

    let config_path = Path::new(CONFIG_FILE_NAME);
    match File::create(config_path) {
        Err(e) => {
            error!("{}", e);
            None
        }
        Ok(mut created_file) => {
            created_file
                .write_all(registered_worker_json.unwrap().as_bytes())
                .unwrap_or_else(|_| panic!("Unable to write file : {:?}", config_path.to_str()));
            info!(
                "✅✅Saved worker registration data to file: {:?}✅✅ ",
                config_path.to_str()
            );
            Some(registered_worker_data)
        }
    }
}
    */
