use std::fmt::Debug;

// Contains all the possible transactions to the parachain, kept out of the `Miner` struct for so that they can contain data that is not the current data (eg. a previous taskId)
use crate::config;
use crate::error::Error;
use crate::specs;
use crate::substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use subxt_signer::sr25519::Keypair;
use substrate_interface::api::neuro_zk::{Error as NzkError};
use crate::error::Result;
use crate::substrate_interface::{self, api::runtime_types::cyborg_primitives::worker::WorkerType};
use crate::types::MinerData;

/// Registers a worker node on the blockchain.
///
/// # Returns
/// A `Result` containing a `String` witht the miner identity if successful, or an `Error` if registration fails.
pub async fn register(keypair: Keypair) -> Result<String> {
    let client = config::get_parachain_client()?;

    let worker_specs = specs::gather_worker_spec().await?;

    let tx = substrate_interface::api::tx()
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
                let worker_identity_json = serde_json::to_string(&MinerData {
                    miner_owner: event.creator.clone().to_string(),
                    miner_identity: event.worker.clone(),
                })?;
                println!("Miner registered successfully: {event:?}");

                return Ok(worker_identity_json)
            } else {
                return Err(Error::Custom("Miner registration event not found, cannot bootstrap miner".to_string()))
            }
        },
        Err(e) => {
            //TODO The parachain should return the required worker data with the `WorkerExists` error, so that the worker can create it's config file from the error in case it encounters it
            return Err(Error::Custom(format!("Check for acceptable error should occur here, instead this error was returned: {}", e.to_string())))
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
    
    let error = NzkError::ProofAlreadySubmitted;

    Ok(())
}

pub async fn confirm_task_reception(keypair: Keypair, current_task: u64) -> Result<()> {
    let client = config::get_parachain_client()?;

    let tx = substrate_interface::api::tx()
        .task_management()
        .confirm_task_reception(
            current_task
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
            println!("Task reception confirmation submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await;

    match tx_submission {
        Ok(e) => {
            let event = e
                .find_first::<substrate_interface::api::task_management::events::TaskReceptionConfirmed>(
            )?;

            if let Some(event) = event {
                println!("Task reception confirmed: {event:?}");
            } else {
                println!("No task reception event found!");
            }
        },
        Err(e) => {
            //TODO add an acceptable error check here - currently miners can infinitely confirm task reception without the parachain throwing an error, so there is no acceptable error that we can check
            return Err(Error::Subxt(e));
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


// This expects the pallet and the corresponding error like this: "ExamplePallet::ExampleError", currently not sure if there is a better way to inspect errors in subxt
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
