use subxt::{events::EventDetails, PolkadotConfig};
use crate::{
    error::{Error, Result}, 
    types::Miner
};
use crate::substrate_interface;
use crate::traits::ParachainInteractor;

pub async fn process_event(
    miner: &mut Miner, 
    event: &EventDetails<PolkadotConfig>
) -> Result<()> {
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

            if *assigned_miner == miner.miner_identity {

                let task_fid_string = String::from_utf8(task_scheduled.task.0)?;

                miner.write_log(format!("New task scheduled for worker: {}", task_fid_string).as_str());

                //TODO spawn thread downloading model

                //TODO spawn thread running inference when download thread done
            }
        }
        Err(e) => {
            println!("Error decoding WorkerStatusUpdated event: {:?}", e);
            return Err(Error::Subxt(e.into()));
        }
        _ => {} // Skip non-matching events
    }

    /*
    //TODO activate this after subxt gen
    // Check for SubmittedCompletedTask event to check if worker was assigned to verify task
    match event.as_event::<substrate_interface::api::neuro_zk::events::ProofRequested>() {
        Ok(Some(requested_proof)) => {
            let task_id = &submitted_task.task_id;

            if *prover == self.current_task {
                //TODO request nzk engine to generate proof
                //TODO submit the proof
            }
        }
        Err(e) => {
            println!("Error decoding SubmittedCompletedTask event: {:?}", e);
            return Err(Error::Subxt(e.into()));
        }
        _ => {} // Skip non-matching events
    }

    //TODO activate this after subxt gen
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