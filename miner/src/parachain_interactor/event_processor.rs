use crate::config::{self, get_paths};
use crate::parachain_interactor::identity::update_identity_file;
use crate::parent_runtime::inference::CURRENT_SERVER;
use crate::substrate_interface;
use crate::traits::{InferenceServer, ParachainInteractor};
use crate::types::CurrentTask;
use crate::utils::tx_builder::{confirm_miner_vacation, confirm_task_reception};
use crate::utils::tx_queue::TxOutput;
use crate::{
    error::{Error, Result},
    types::{Miner, MinerData},
};
use std::sync::Arc;
use serde::Serialize;
use subxt::utils::AccountId32;
use subxt::{events::EventDetails, PolkadotConfig};
use std::fs;

#[derive(Serialize)]
struct TaskOwner {
    address: AccountId32,
}

pub async fn process_event(miner: &mut Miner, event: &EventDetails<PolkadotConfig>) -> Result<()> {
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
    match event.as_event::<substrate_interface::api::edge_connect::events::WorkerStatusUpdated>() {
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
            let assigned_miner = &task_scheduled.assigned_worker;
            let identity_path = &get_paths()?.identity_path;

            let file_content = fs::read_to_string(identity_path)?;
            let miner_data: MinerData = serde_json::from_str(&file_content)?;

            if assigned_miner == &miner_data.miner_identity {
                println!("New task scheduled: {:?}", task_scheduled.task_id);

                let task_owner_string = serde_json::to_string(&TaskOwner{
                    address: task_scheduled.task_owner,
                })?;

                let task_owner_path = &get_paths()?.task_owner_path;

                update_identity_file(
                    task_owner_path,
                    &task_owner_string,
                )?;

                miner.current_task = Some(CurrentTask {
                    id: task_scheduled.task_id,
                    task_type: task_scheduled.task_kind,
                });

                let parent_runtime_clone = Arc::clone(&miner.parent_runtime);
                let current_task_clone = miner.current_task.clone();

                if let Some(current_task) = current_task_clone {
                    tokio::spawn(async move {
                        if let Err(e) = parent_runtime_clone
                            .read()
                            .await
                            .process_task(current_task.clone().task_type)
                            .await
                        {
                            println!("Error downloading model archive: {}", e);
                        };

                        if let Err(e) = parent_runtime_clone
                            .read()
                            .await
                            .spawn_inference_server(&current_task)
                            .await
                        {
                            println!("Error performing inference: {}", e)
                        };
                    });
                } else {
                    return Err(Error::Custom("No current task".to_string()));
                }

                let tx_queue = config::get_tx_queue()?;
                let keypair = miner.keypair.clone();
                let task_id = task_scheduled.task_id;

                let rx = tx_queue.enqueue(move || {
                    let keypair = keypair.clone();
                    async move {
                        let _ = confirm_task_reception(keypair, task_id).await?;
                        Ok(TxOutput::Success)
                    }
                }).await?;

                // Handle response 
                match rx.await {
                    Ok(Ok(TxOutput::Success)) => println!("Task reception confirmed"),
                    Ok(Err(e)) => println!("Error confirming task reception: {}", e),
                    _ => println!("Unexpected response for task confirmation"),
                }
            }
        }
        Err(e) => {
            println!("Error decoding TaskScheduled event: {:?}", e);
            return Err(Error::Subxt(e.into()));
        }
        _ => {} // Skip non-matching events
    }

    if let Some(current_task) = &miner.current_task {
        let current_task_id = current_task.id;

        match event.as_event::<substrate_interface::api::task_management::events::TaskStopRequested>() {
            Ok(Some(requested_task_stop)) => {
                let task_id = &requested_task_stop.task_id;

                if *task_id == current_task_id {
                    let server_control = CURRENT_SERVER
                        .lock()
                        .await
                        .take()
                        .ok_or(Error::Custom("There is no inference server initialized in CURRENT_SERVER!".to_string()))?;

                    let task_dir = &config::PATHS
                        .get()
                        .ok_or(Error::config_paths_not_initialized())?
                        .task_dir_path;
                    
                    server_control.shutdown(task_dir).await?;

                    let task_owner_path = &get_paths()?.task_owner_path;
                    update_identity_file(task_owner_path, "")?;

                    miner.current_task = None;

                    let tx_queue = config::get_tx_queue()?;
                    let keypair = miner.keypair.clone();
                    let current_task_id_copy = current_task_id;

                    let rx = tx_queue.enqueue(move || {
                        let keypair = keypair.clone();
                        async move {
                            let _ = confirm_miner_vacation(keypair, current_task_id_copy).await?;
                            Ok(TxOutput::Success)
                        }
                    }).await?;

                    match rx.await {
                        Ok(Ok(TxOutput::Success)) => println!("Miner vacation confirmed!"),
                        Ok(Err(e)) => println!("Error confirming miner vacation: {}", e),
                        _ => println!("Unexpected response for miner vacation confirmation"),
                    }
                }
            }
            Err(e) => {
                println!("Error decoding TaskStopRequested event: {:?}", e);
                return Err(Error::Subxt(e.into()));
            }
            _ => {} // Skip non-matching events
        }
    }

    if let Some(current_task) = &miner.current_task {
        match event.as_event::<substrate_interface::api::neuro_zk::events::NzkProofRequested>() {
            Ok(Some(requested_proof)) => {
                let task_id = &requested_proof.task_id;

                if *task_id == current_task.id {
                    let proof = miner.parent_runtime.read().await.generate_proof().await?;
                    let _ = miner.submit_zkml_proof(proof).await?;
                }
            }
            Err(e) => {
                println!("Error decoding SubmittedCompletedTask event: {:?}", e);
                return Err(Error::Subxt(e.into()));
            }
            _ => {} // Skip non-matching events
        }
    }

    /*
    //TODO check if proof was submitted (after parachain update)
    // Check for SubmittedCompletedTask event to check if worker was assigned to verify task
    match event.as_event::<substrate_interface::api::neuro_zk::events::ProofSubmitted>() {
        Ok(Some(submitted_proof)) => {
            let prover = &submitted_task.prover;

            if *prover == self.identity {
                //TODO add an proof submission state somewhere that tracks if the proof was submitted or not (wait 60sec otherwise retry)
                //TODO set the above mentioned state to submitted
            }
        }
        Err(e) => {
            println!("Error decoding SubmittedCompletedTask event: {:?}", e);
            return Err(Error::Subxt(e.into()));
        }
        _ => {} // Skip non-matching events
    }
    */

    Ok(())
}
