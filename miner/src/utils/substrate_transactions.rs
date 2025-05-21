use crate::{error::Result, substrate_interface};
use substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use subxt::utils::H256;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::Keypair;

pub async fn submit_result(
    api: &OnlineClient<PolkadotConfig>,
    signer_keypair: &Keypair,
    completed_hash: H256,
    result_cid: BoundedVec<u8>,
    task_id: u64,
) -> Result<()> {
    let result_submission_tx = substrate_interface::api::tx()
        .task_management()
        .submit_completed_task(task_id, completed_hash, result_cid);

    println!("Transaction Details:");
    println!("Module: {:?}", result_submission_tx.pallet_name());
    println!("Call: {:?}", result_submission_tx.call_name());
    println!("Parameters: {:?}", result_submission_tx.call_data());

    let result_submission_events = api
        .tx()
        .sign_and_submit_then_watch_default(&result_submission_tx, signer_keypair)
        .await
        .map(|e| {
            println!("Result submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let submission_event = result_submission_events.find_first::<substrate_interface::api::task_management::events::SubmittedCompletedTask>()?;
    if let Some(event) = submission_event {
        println!("Task submitted successfully: {event:?}");
    } else {
        println!("Task submission failed");
    }

    Ok(())
}

pub async fn submit_result_verification(
    api: &OnlineClient<PolkadotConfig>,
    signer_keypair: &Keypair,
    completed_hash: H256,
    task_id: u64,
) -> Result<()> {
    let verification_submission_tx = substrate_interface::api::tx()
        .task_management()
        .verify_completed_task(task_id, completed_hash);

    println!("Transaction Details:");
    println!("Module: {:?}", verification_submission_tx.pallet_name());
    println!("Call: {:?}", verification_submission_tx.call_name());
    println!("Parameters: {:?}", verification_submission_tx.call_data());

    let verification_submission_events = api
        .tx()
        .sign_and_submit_then_watch_default(&verification_submission_tx, signer_keypair)
        .await
        .map(|e| {
            println!("Result submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let submission_event = verification_submission_events.find_first::<substrate_interface::api::task_management::events::VerifiedCompletedTask>()?;
    if let Some(event) = submission_event {
        println!("Task submitted successfully: {event:?}");
    } else {
        println!("Task submission failed");
    }

    Ok(())
}

pub async fn submit_result_resolution(
    api: &OnlineClient<PolkadotConfig>,
    signer_keypair: &Keypair,
    completed_hash: H256,
    task_id: u64,
) -> Result<()> {
    let resolution_submission_tx = substrate_interface::api::tx()
        .task_management()
        .resolve_completed_task(task_id, completed_hash);

    println!("Transaction Details:");
    println!("Module: {:?}", resolution_submission_tx.pallet_name());
    println!("Call: {:?}", resolution_submission_tx.call_name());
    println!("Parameters: {:?}", resolution_submission_tx.call_data());

    let resolution_submission_events = api
        .tx()
        .sign_and_submit_then_watch_default(&resolution_submission_tx, signer_keypair)
        .await
        .map(|e| {
            println!("Result submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let submission_event = resolution_submission_events.find_first::<substrate_interface::api::task_management::events::ResolvedCompletedTask>()?;
    if let Some(event) = submission_event {
        println!("Task submitted successfully: {event:?}");
    } else {
        println!("Task submission failed");
    }

    Ok(())
}
