use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::Keypair;
use subxt::utils::H256;

use substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use crate::{substrate_interface, error::Result};

#[derive(Debug)]
pub enum TransactionType {
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

#[derive(Debug)]
enum Transaction {
    SubmitResult {
        completed_hash: H256,
        result_cid: BoundedVec<u8>,
        task_id: u64,
        retry_count: u32,
    },
    SubmitResultVerification {
        completed_hash: H256,
        task_id: u64,
        retry_count: u32,
    },
    SubmitResultResolution {
        completed_hash: H256,
        task_id: u64,
        retry_count: u32,
    },
}

const RETRY_DELAY_MS: u64 = 1000;

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
            retry_count: 0,
        });
    }

    pub fn add_submit_verification(&self, completed_hash: H256, task_id: u64) {
        let mut queue = self.inner.lock().unwrap();
        queue.push_back(Transaction::SubmitResultVerification {
            completed_hash,
            task_id,
            retry_count: 0,
        });
    }

    pub fn add_submit_resolution(&self, completed_hash: H256, task_id: u64) {
        let mut queue = self.inner.lock().unwrap();
        queue.push_back(Transaction::SubmitResultResolution {
            completed_hash,
            task_id,
            retry_count: 0,
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
            let result = match transaction {
                Transaction::SubmitResult {
                    completed_hash,
                    ref result_cid,
                    task_id,
                    retry_count,
                } => {
                    let result = submit_result_internal(api, signer_keypair, completed_hash, result_cid.clone(), task_id).await;
                    (result, completed_hash, task_id, retry_count)
                }
                Transaction::SubmitResultVerification {
                    completed_hash,
                    task_id,
                    retry_count,
                } => {
                    let result = submit_result_verification_internal(api, signer_keypair, completed_hash, task_id).await;
                    (result, completed_hash, task_id, retry_count)
                }
                Transaction::SubmitResultResolution {
                    completed_hash,
                    task_id,
                    retry_count,
                } => {
                    let result = submit_result_resolution_internal(api, signer_keypair, completed_hash, task_id).await;
                    (result, completed_hash, task_id, retry_count)
                }
            };

            match result {
                (Ok(_), _, _, _) => Ok(()),
                (Err(e), _completed_hash,_task_idd, retry_count) => {
                    if Self::is_nonce_error(&e) {
                        let delay_ms = std::cmp::min(
                            10_000,
                            RETRY_DELAY_MS * 2u64.pow(std::cmp::min(retry_count as u32, 10)), 
                        );
                        
                        println!("Nonce error detected, retrying transaction (attempt {}). Sleeping for {}ms...", 
                            retry_count + 1, delay_ms);
                        
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        
                        let mut queue = self.inner.lock().unwrap();
                        let new_transaction = match transaction {
                            Transaction::SubmitResult { completed_hash, result_cid, task_id, .. } => {
                                Transaction::SubmitResult {
                                    completed_hash,
                                    result_cid,
                                    task_id,
                                    retry_count: retry_count + 1,
                                }
                            }
                            Transaction::SubmitResultVerification { completed_hash, task_id, .. } => {
                                Transaction::SubmitResultVerification {
                                    completed_hash,
                                    task_id,
                                    retry_count: retry_count + 1,
                                }
                            }
                            Transaction::SubmitResultResolution { completed_hash, task_id, .. } => {
                                Transaction::SubmitResultResolution {
                                    completed_hash,
                                    task_id,
                                    retry_count: retry_count + 1,
                                }
                            }
                        };
                        queue.push_front(new_transaction);
                        Ok(())
                    } else {
                        println!("Non-nonce error detected ({}), retrying transaction...", e);
                        let mut queue = self.inner.lock().unwrap();
                        queue.push_front(transaction);
                        Ok(())
                    }
                }
            }
        } else {
            Ok(())
        }
    }

    fn is_nonce_error(error: &crate::error::Error) -> bool {
        error.to_string().contains("InvalidTransaction") && 
            (error.to_string().contains("Stale") || error.to_string().contains("nonce"))
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
            result_cid.clone(), 
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

pub async fn submit_tx(
    _api: &OnlineClient<PolkadotConfig>,
    _signer_keypair: &Keypair,
    tx_type: TransactionType,
) -> Result<()> {
    match tx_type {
        TransactionType::SubmitResult {
            completed_hash,
            result_cid,
            task_id,
        } => {
            TRANSACTION_QUEUE.add_submit_result(completed_hash, result_cid, task_id);
            Ok(())
        }
        TransactionType::SubmitResultVerification {
            completed_hash,
            task_id,
        } => {
            TRANSACTION_QUEUE.add_submit_verification(completed_hash, task_id);
            Ok(())
        }
        TransactionType::SubmitResultResolution {
            completed_hash,
            task_id,
        } => {
            TRANSACTION_QUEUE.add_submit_resolution(completed_hash, task_id);
            Ok(())
        }
    }
}

pub async fn process_transactions(
    api: &OnlineClient<PolkadotConfig>,
    signer_keypair: &Keypair,
) -> Result<()> {
    TRANSACTION_QUEUE.process_next(api, signer_keypair).await
}