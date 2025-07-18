// Contains all the possible transactions to the parachain, kept out of the `Miner` struct for so that they can contain data that is not the current data (eg. a previous taskId)

use std::fmt::Debug;
use crate::config;
use crate::error::Error;
use crate::specs;
use crate::substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use crate::utils::substrate_queries::get_miner_by_domain;
use subxt::utils::AccountId32;
use subxt_signer::sr25519::Keypair;
use substrate_interface::api::neuro_zk::{Error as NzkError};
use substrate_interface::api::edge_connect::{Error as EdgeConnectError};
use crate::error::Result;
use crate::substrate_interface::{self, api::runtime_types::cyborg_primitives::worker::WorkerType};

/// Registers a worker node on the blockchain.
///
/// # Returns
/// A `Result` containing a `String` witht the miner identity if successful, or an `Error` if registration fails.
pub async fn register(keypair: Keypair) -> Result<(AccountId32, u64)> {
    let client = config::get_parachain_client()?;

    let worker_specs = specs::gather_worker_spec().await?;

    let tx = substrate_interface::api::tx()
        .edge_connect()
        .register_worker(
            WorkerType::Executable,
            BoundedVec::from(BoundedVec(worker_specs.domain.clone().as_bytes().to_vec())),
            worker_specs.latitude,
            worker_specs.longitude,
            worker_specs.ram,
            worker_specs.storage,
            worker_specs.cpu,
        );

    println!("Transaction Details:");
    println!("Module: {:?}", tx.pallet_name());
    println!("Call: {:?}", tx.call_name());
    println!("Parameters: {:?}", tx.call_data());

    let tx_submission = client
        .tx()
        .sign_and_submit_then_watch_default(&tx, &keypair)
        .await
        .map(|e| {
            println!("Miner registration submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await;

    match tx_submission {
        Ok(e) => {
            let tx_event = e
                .find_first::<substrate_interface::api::edge_connect::events::WorkerRegistered>(
            )?;

            if let Some(event) = tx_event {
                println!("Miner registered successfully: {event:?}");

                return Ok((event.worker.0, event.worker.1))
            } else {
                return Err(Error::Custom("Miner registration event not found, cannot bootstrap miner".to_string()))
            }
        },
        Err(e) => {
            if let Err(e) = check_for_acceptable_error(EdgeConnectError::WorkerExists, e) {
               return Err(Error::Custom(e.to_string())) 
            } else {
                match get_miner_by_domain(client, &worker_specs.domain).await {
                    Ok((miner_id, miner_owner)) => {
                        println!("Registered miner found: {miner_id}, {miner_owner}"); 

                        return Ok((miner_id, miner_owner))
                    },
                    Err(e) => {
                        return Err(Error::Custom(format!("UNRECOVERABLE ERROR: Cannot bootstrap miner: {e}")));
                    }
                }
            }; 
        },
    }
}

/// Submits a zkml (Zero Knowledge Machine Learning) proof to the blockchain.
///
/// # Arguments
/// * `proof` - A `Vec<u8>` containing the zkml proof.
///
/// # Returns
/// A `Result` indicating `Ok(())` if the result is successfully submitted, or an `Error` if it fails.
pub async fn submit_proof(proof: Vec<u8>, keypair: Keypair, current_task: u64) -> Result<()> {
    let proof: BoundedVec<u8> = BoundedVec::from(BoundedVec(proof));

    let client = config::get_parachain_client()?;

    let tx = substrate_interface::api::tx()
        .neuro_zk()
        .submit_proof(current_task, proof);

    println!("Transaction Details:");
    println!("Module: {:?}", tx.pallet_name());
    println!("Call: {:?}", tx.call_name());
    println!("Parameters: {:?}", tx.call_data());

    let tx_submission = client
        .tx()
        .sign_and_submit_then_watch_default(&tx, &keypair)
        .await
        .map(|e| {
            println!(
                "Proof submitted, waiting for transaction to be finalized..."
            );
            e
        })?
        .wait_for_finalized_success()
        .await;

    match tx_submission {
        Ok(e) => {
            let tx_event = e
                .find_first::<substrate_interface::api::neuro_zk::events::NzkProofSubmitted>(
            )?;

            if let Some(event) = tx_event {
                println!("Proof submission confirmed: {event:?}");
            } else {
                println!("No proof submission event found!");
            }
        },
        Err(e) => {
           check_for_acceptable_error(NzkError::ProofAlreadySubmitted, e)?; 
        },
    }

    Ok(())
}

pub async fn confirm_task_reception(keypair: Keypair, current_task: u64) -> Result<()> {
    let client = config::get_parachain_client()?;

    let tx = substrate_interface::api::tx()
        .task_management()
        .confirm_task_reception(current_task);

    println!("Transaction Details:");
    println!("Module: {:?}", tx.pallet_name());
    println!("Call: {:?}", tx.call_name());
    println!("Parameters: {:?}", tx.call_data());

    let tx_submission = client
        .tx()
        .sign_and_submit_then_watch_default(&tx, &keypair)
        .await
        .map(|e| {
            println!("Task reception confirmation submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await;

    match tx_submission {
        Ok(e) => {
            let event = e
                .find_first::<substrate_interface::api::task_management::events::TaskReceptionConfirmed>()?;

            if let Some(event) = event {
                println!("Task reception confirmed: {event:?}");
            } else {
                println!("No task reception event found!");
            }
        },
        Err(e) => {
            // Handle the case where task reception was already confirmed
            check_for_acceptable_error(
                substrate_interface::api::task_management::Error::TaskReceptionAlreadyConfirmed,
                e
            )?;
        },
    }

    Ok(())
}

/// Vacates a miner erasing current user data and resetting the miner state.
///
/// # Returns
/// A `Result` indicating `Ok(())` if the session vacates successfully, or an `Error` if it fails.
pub async fn confirm_miner_vacation(keypair: Keypair, task_id: u64) -> Result<()> {
    let client = config::get_parachain_client()?;

    let tx = substrate_interface::api::tx()
        .task_management()
        .confirm_miner_vacation(task_id);

    println!("Transaction Details:");
    println!("Module: {:?}", tx.pallet_name());
    println!("Call: {:?}", tx.call_name());
    println!("Parameters: {:?}", tx.call_data());

    let tx_submission = client
        .tx()
        .sign_and_submit_then_watch_default(&tx, &keypair)
        .await
        .map(|e| {
            println!("Miner vacation confirmation submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await;

    match tx_submission {
        Ok(e) => {
            let tx_event = e
                .find_first::<substrate_interface::api::task_management::events::MinerVacated>(
            )?;

            if let Some(event) = tx_event {
                println!("Miner vacation confirmed: {event:?}");
            } else {
                println!("No miner vacation event found!");
            }
        },
        Err(e) => {
           check_for_acceptable_error("TaskManagement::InvalidTaskState", e)?; 
        },
    }

    Ok(())
}


// This takes in a generic that implements debug as the errors that will be put in here are different types of errors
/// Lets acceptable errors pass through so that the transaction queue doesn't repeat them, because the transaction already succeeded. In some cases for example, the parachain
/// will accept a transaction, but return an error anyway which will cause the transaction queue to re-queue the transaction. Upon trying again, the transaction will be rejected again, 
/// because the transaction DID already succeed previously. The function is a workaround for this. It checks the returned error and if it is an error of this sort it lets it pass, 
/// causing the transaction queue to not re-queue the transaction.
fn check_for_acceptable_error<T: Debug>(expected_error: T, e: subxt::Error) -> Result<()> {
    match e {
        subxt::Error::Runtime(err) => {
            match err {
                subxt::error::DispatchError::Module(returned_error) => {
                    let returned_error_details = returned_error.details()
                        .map_err(|err| Error::Custom(err.to_string()))?;

                    let returned_error_string = returned_error_details.variant.name.to_string();
                    let expected_error_string = format!("{:?}", expected_error);

                    println!("Error details - returned error: {:?}", returned_error_string);
                    println!("Error details - expected error: {:?}", expected_error_string);

                    if returned_error_string == expected_error_string {
                        return Ok(()) 
                    } else {
                        return Err(Error::Custom(returned_error.to_string()))
                    }
                },
                _ =>  return Err(Error::Custom(err.to_string())),
            };
        },
        _ => return Err(e.into()),
    }
}
