use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::Keypair;
use subxt::utils::H256;

use substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use crate::{substrate_interface, error::Result};

#[derive(Debug)]
enum Transaction {
    SubmitResult {
        completed_hash: H256,
        result_cid: BoundedVec<u8>,
        task_id: u64,
    },
    SubmitResultVerification {
        completed_hash: H256,
        task_id: u64,
    },
    SubmitResultResolution {
        completed_hash: H256,
        task_id: u64,
    },
}

#[derive(Clone)]
pub struct TransactionQueue {
    inner: Arc<Mutex<VecDeque<Transaction>>>,
}

impl TransactionQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn add_submit_result(
        &self,
        completed_hash: H256,
        result_cid: BoundedVec<u8>,
        task_id: u64,
    ) {
        let mut queue = self.inner.lock().unwrap();
        queue.push_back(Transaction::SubmitResult {
            completed_hash,
            result_cid,
            task_id,
        });
    }

    pub fn add_submit_verification(&self, completed_hash: H256, task_id: u64) {
        let mut queue = self.inner.lock().unwrap();
        queue.push_back(Transaction::SubmitResultVerification {
            completed_hash,
            task_id,
        });
    }

    pub fn add_submit_resolution(&self, completed_hash: H256, task_id: u64) {
        let mut queue = self.inner.lock().unwrap();
        queue.push_back(Transaction::SubmitResultResolution {
            completed_hash,
            task_id,
        });
    }

    pub async fn process_next(
        &self,
        api: &OnlineClient<PolkadotConfig>,
        signer_keypair: &Keypair,
    ) -> Result<()> {
        let transaction = {
            let mut queue = self.inner.lock().unwrap();
            queue.pop_front()
        };

        if let Some(transaction) = transaction {
            match transaction {
                Transaction::SubmitResult {
                    completed_hash,
                    result_cid,
                    task_id,
                } => {
                    submit_result_internal(api, signer_keypair, completed_hash, result_cid, task_id)
                        .await?;
                }
                Transaction::SubmitResultVerification {
                    completed_hash,
                    task_id,
                } => {
                    submit_result_verification_internal(api, signer_keypair, completed_hash, task_id)
                        .await?;
                }
                Transaction::SubmitResultResolution {
                    completed_hash,
                    task_id,
                } => {
                    submit_result_resolution_internal(api, signer_keypair, completed_hash, task_id)
                        .await?;
                }
            }
        }

        Ok(())
    }
}

lazy_static::lazy_static! {
    pub static ref TRANSACTION_QUEUE: TransactionQueue = TransactionQueue::new();
}

async fn submit_result_internal(
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

    let submission_event = 
        result_submission_events.find_first::<substrate_interface::api::task_management::events::SubmittedCompletedTask>()?;
    if let Some(event) = submission_event {
        println!("Task submitted successfully: {event:?}");
    } else {
        println!("Task submission failed");
    }

    Ok(())
}

pub async fn submit_result(
    _api: &OnlineClient<PolkadotConfig>, 
    _signer_keypair: &Keypair, 
    completed_hash: H256,
    result_cid: BoundedVec<u8>,
    task_id: u64, 
) -> Result<()> {
    TRANSACTION_QUEUE.add_submit_result(completed_hash, result_cid, task_id);
    Ok(())
}

async fn submit_result_verification_internal(
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

    let submission_event = 
        verification_submission_events.find_first::<substrate_interface::api::task_management::events::VerifiedCompletedTask>()?;
    if let Some(event) = submission_event {
        println!("Task submitted successfully: {event:?}");
    } else {
        println!("Task submission failed");
    }

    Ok(())
}

pub async fn submit_result_verification(
    _api: &OnlineClient<PolkadotConfig>, 
    _signer_keypair: &Keypair, 
    completed_hash: H256,
    task_id: u64, 
) -> Result<()> {
    TRANSACTION_QUEUE.add_submit_verification(completed_hash, task_id);
    Ok(())
}

async fn submit_result_resolution_internal(
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

    let submission_event = 
        resolution_submission_events.find_first::<substrate_interface::api::task_management::events::ResolvedCompletedTask>()?;
    if let Some(event) = submission_event {
        println!("Task submitted successfully: {event:?}");
    } else {
        println!("Task submission failed");
    }

    Ok(())
}

pub async fn submit_result_resolution(
    _api: &OnlineClient<PolkadotConfig>, 
    _signer_keypair: &Keypair, 
    completed_hash: H256,
    task_id: u64, 
) -> Result<()> {
    TRANSACTION_QUEUE.add_submit_resolution(completed_hash, task_id);
    Ok(())
}

pub async fn process_transactions(
    api: &OnlineClient<PolkadotConfig>,
    signer_keypair: &Keypair,
) -> Result<()> {
    TRANSACTION_QUEUE.process_next(api, signer_keypair).await
}