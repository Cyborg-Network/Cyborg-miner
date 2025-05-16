use crate::substrate_interface;
use crate::traits::{InferenceServer, ParachainInteractor};
use crate::{
    error::{Error, Result},
    types::Miner,
};
use cess_rust_sdk::polkadot::asset_conversion::storage;
use subxt::{events::EventDetails, PolkadotConfig};
use tracing::info;
use std::sync::Arc;

pub async fn process_event(miner: &Miner, event: &EventDetails<PolkadotConfig>) -> Result<()> {

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
            let identity = &miner.miner_identity
                .as_ref()
                .ok_or(Error::identity_not_initialized())?;

            if &assigned_miner == identity {
                //TODO uncomment this and remove the hardcoded cipher after subxt is regen
                //let storage_encryption_cipher = &task_scheduled.cipher;
                let storage_encryption_cipher = "password";
                let task_fid_string = String::from_utf8(task_scheduled.task.0)?;

                info!("New task scheduled for worker: {}", task_fid_string);

                let parent_runtime_clone = Arc::clone(&miner.parent_runtime);

                tokio::spawn(async move {
                    if let Err(e) = parent_runtime_clone.read().await.
                        download_model_archive(&task_fid_string, storage_encryption_cipher).await {
                            eprintln!("Error downloading model archive: {}", e);
                        };

                    if let Err(e) = parent_runtime_clone.read().await.perform_inference().await {
                        eprintln!("Error performing inference: {}", e)
                    };
                });
            }
        }
        Err(e) => {
            println!("Error decoding WorkerStatusUpdated event: {:?}", e);
            return Err(Error::Subxt(e.into()));
        }
        _ => {} // Skip non-matching events
    }

    if let Some(current_task) = &miner.current_task {
        match event.as_event::<substrate_interface::api::neuro_zk::events::ProofRequested>() {
            Ok(Some(requested_proof)) => {
                let task_id = &requested_proof.task_id;

                if *task_id == current_task.0 {
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
