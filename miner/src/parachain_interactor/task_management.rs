use crate::{
    config,
    error::{Error, Result},
    substrate_interface::{
        self,
        api::{neuro_zk, runtime_types::bounded_collections::bounded_vec::BoundedVec},
    },
    types::Miner,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn confirm_task_reception(miner: &Miner) -> Result<()> {
    let client = config::get_parachain_client()?;
    let config_path = &config::get_paths()?.identity_path;
    let keypair = &miner.keypair;
    let current_task = miner
        .current_task
        .as_ref()
        .ok_or(Error::no_current_task())?
        .id
        .clone();

    let task_confirmation = substrate_interface::api::tx()
        .task_management()
        .confirm_task_reception(current_task);

    println!("Transaction Details:");
    println!("Module: {:?}", task_confirmation.pallet_name());
    println!("Call: {:?}", task_confirmation.call_name());
    println!("Parameters: {:?}", task_confirmation.call_data());

    let worker_registration_events = client
        .tx()
        .sign_and_submit_then_watch_default(&task_confirmation, keypair)
        .await
        .map(|e| {
            println!(
                "Task reception confirmation submitted, waiting for transaction to be finalized..."
            );
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let registration_event = worker_registration_events
        .find_first::<substrate_interface::api::task_management::events::TaskReceptionConfirmed>(
    )?;

    if let Some(event) = registration_event {
        println!("Task reception confirmed: {event:?}");
    } else {
        println!("Task reception confirmation failed!");
    }

    Ok(())
}

pub async fn stop_task_and_vacate_miner() -> Result<()> {
    //TODO implement a tokio::sync::watch for the inference task
    println!("Task stop and vacate miner is unimplemented!!!!");

    Ok(())
}

pub async fn submit_zkml_proof(miner: &Miner, proof: Vec<u8>) -> Result<()> {
    let proof: BoundedVec<u8> = BoundedVec::from(BoundedVec(proof));

    let client = config::get_parachain_client()?;
    let keypair = &miner.keypair;
    let current_task = miner
        .current_task
        .as_ref()
        .ok_or(Error::no_current_task())?
        .id
        .clone();

    let proof_submission = substrate_interface::api::tx()
        .neuro_zk()
        .submit_proof(current_task, proof);

    println!("Transaction Details:");
    println!("Module: {:?}", proof_submission.pallet_name());
    println!("Call: {:?}", proof_submission.call_name());
    println!("Parameters: {:?}", proof_submission.call_data());

    let proof_submission_events = client
        .tx()
        .sign_and_submit_then_watch_default(&proof_submission, keypair)
        .await
        .map(|e| {
            println!(
                "Task reception confirmation submitted, waiting for transaction to be finalized..."
            );
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let proof_submission_event = proof_submission_events
        .find_first::<substrate_interface::api::neuro_zk::events::NzkProofSubmitted>(
    )?;

    if let Some(event) = proof_submission_event {
        println!("Task reception confirmed: {event:?}");
    } else {
        println!("Task reception confirmation failed!");
    }

    Ok(())
}
