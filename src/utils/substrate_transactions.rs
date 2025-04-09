use substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::Keypair;
use crate::{substrate_interface, error::Result};
use subxt::utils::H256;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

// Shared transaction queue
lazy_static::lazy_static! {
    static ref TRANSACTION_QUEUE: Arc<Mutex<VecDeque<QueuedTransaction>>> = Arc::new(Mutex::new(VecDeque::new()));
    static ref QUEUE_SEMAPHORE: Arc<Semaphore> = Arc::new(Semaphore::new(1));
}

pub async fn submit_result(
    api: &OnlineClient<PolkadotConfig>, 
    signer_keypair: &Keypair, 
    completed_hash: H256,
    result_cid: BoundedVec<u8>,
    task_id: u64, 
) -> Result<()> {
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

     // Add to queue instead of submitting directly
     let mut queue = TRANSACTION_QUEUE.lock().unwrap();
     queue.push_back(QueuedTransaction {
         api: api.clone(),
         signer_keypair: signer_keypair.clone(),
         tx: result_submission_tx,
     });

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

pub async fn submit_result_verification(
    api: &OnlineClient<PolkadotConfig>, 
    signer_keypair: &Keypair, 
    completed_hash: H256,
    task_id: u64, 
) -> Result<()> {
    let verification_submission_tx = substrate_interface::api::tx()
        .task_management()
        .verify_completed_task(
            task_id, 
            completed_hash
        );

    println!("Transaction Details:");
    println!("Module: {:?}", verification_submission_tx.pallet_name());
    println!("Call: {:?}", verification_submission_tx.call_name());
    println!("Parameters: {:?}", verification_submission_tx.call_data());

    let mut queue = TRANSACTION_QUEUE.lock().unwrap();
    queue.push_back(QueuedTransaction {
        api: api.clone(),
        signer_keypair: signer_keypair.clone(),
        tx: verification_submission_tx,
    });

    let verification_submission_events= api
        .tx()
        .sign_and_submit_then_watch_default(&verification_submission_tx, signer_keypair)
        .await
        .map(|e| {
            println!("Result submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let submission_event = 
        verification_submission_events.find_first::<substrate_interface::api::task_management::events::VerifiedCompletedTask>()?;
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
        .resolve_completed_task(
            task_id, 
            completed_hash
        );

    println!("Transaction Details:");
    println!("Module: {:?}", resolution_submission_tx.pallet_name());
    println!("Call: {:?}", resolution_submission_tx.call_name());
    println!("Parameters: {:?}", resolution_submission_tx.call_data());

    let mut queue = TRANSACTION_QUEUE.lock().unwrap();
    queue.push_back(QueuedTransaction {
        api: api.clone(),
        signer_keypair: signer_keypair.clone(),
        tx: resolution_submission_tx,
    });

    let resolution_submission_events= api
        .tx()
        .sign_and_submit_then_watch_default(&resolution_submission_tx, signer_keypair)
        .await
        .map(|e| {
            println!("Result submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let submission_event = 
        resolution_submission_events.find_first::<substrate_interface::api::task_management::events::ResolvedCompletedTask>()?;
    if let Some(event) = submission_event {
        println!("Task submitted successfully: {event:?}");
    } else {
        println!("Task submission failed");
    }

    Ok(())
}

struct QueuedTransaction {
    api: OnlineClient<PolkadotConfig>,
    signer_keypair: Keypair,
    tx: subxt::tx::Payload,
}

async fn process_transaction_queue() {
    loop {
        let permit = QUEUE_SEMAPHORE.acquire().await.unwrap();
        let next_tx = {
            let mut queue = TRANSACTION_QUEUE.lock().unwrap();
            queue.pop_front()
        };

        if let Some(queued_tx) = next_tx {
            let _ = queued_tx.api
                .tx()
                .sign_and_submit_then_watch_default(&queued_tx.tx, &queued_tx.signer_keypair)
                .await
                .and_then(|e| Ok(e.wait_for_finalized_success()));
        }
        
        drop(permit); 
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}


pub fn init_transaction_processor() {
    tokio::spawn(process_transaction_queue());
}
