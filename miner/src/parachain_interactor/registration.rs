use crate::config;
use crate::error::{Error, Result};
use crate::substrate_interface;
use crate::utils::tx_builder::register;
use crate::utils::tx_queue::TxOutput;
use crate::traits::ParachainInteractor;
use crate::types::{Miner, MinerData};
use serde::Deserialize;
use std::fs;
use subxt::utils::AccountId32;

#[derive(Deserialize)]
#[allow(dead_code)]
struct Identity {
    miner_owner: String,
    miner_identity: (AccountId32, u64),
}

pub enum RegistrationStatus{
    Registered(AccountId32, u64),
    Unknown,
}

pub async fn confirm_registration(_: &Miner) -> Result<RegistrationStatus> {
    let client = config::get_parachain_client()?;

    let identity_path = &config::get_paths()?.identity_path;
    let identity_file_content = fs::read_to_string(identity_path)?;
    let identity: Identity = serde_json::from_str(&identity_file_content)?;
    let identity = identity.miner_identity;

    println!("Confirming miner registration...");

    println!("identity: {:?}", identity);

    // Since there seems to be a bug in subxt that should have been resolved (and we possibly won't have a separate storage map for querying workers by id)
    let miner_registration_confirmation_query = substrate_interface::api::storage()
        .edge_connect()
        .executable_workers_iter();

    let mut result = client
        .storage()
        .at_latest()
        .await?
        .iter(miner_registration_confirmation_query)
        .await?;

    while let Some(Ok(miner)) = result.next().await {
        if miner.value.owner == identity.0 && miner.value.id == identity.1 {
            return Ok(RegistrationStatus::Registered(identity.0, identity.1));
        }
    }

    println!("Miner is not registered");
    Ok(RegistrationStatus::Unknown)
}

pub async fn start_miner(miner: &mut Miner) -> Result<()> {
    println!("Starting miner...");

    println!("Waiting for tasks...");

    let client = config::get_parachain_client()?;
    let tx_queue = config::get_tx_queue()?;

    match miner.confirm_registration().await {
        Ok(RegistrationStatus::Registered(owner, id)) => {
            miner.miner_identity = Some((owner, id));
        }, 
        Ok(RegistrationStatus::Unknown) => {
            let keypair = miner.keypair.clone();
            let rx = tx_queue.enqueue( move || {
                let keypair = keypair.clone();
                async move {
                    let result = register(keypair).await?;
                    Ok(TxOutput::RegistrationInfo(result))
                }
            })
            .await?;

            match rx.await {
                Ok(Ok(TxOutput::RegistrationInfo(data))) => {
                    miner.miner_identity = Some(data.clone());
                    let miner_identity_json = serde_json::to_string(&MinerData {
                        miner_owner: data.0.to_string(),
                        miner_identity: (data.0, data.1),
                    })?;
                    miner.update_identity_file(&config::get_paths()?.identity_path, &miner_identity_json)?;
                },
                Ok(Err(e)) => println!("Error registering miner: {}", e),
                Err(_) => println!("Response channel dropped."),
                _ => println!("Missing identity string from registration event"),
            }
        },
        Err(e) => {
            println!("Error confirming miner registration: {}, registering...", e);
            let keypair = miner.keypair.clone();
            let rx = tx_queue.enqueue( move || {
                let keypair = keypair.clone();
                async move {
                    let result = register(keypair).await?;
                    Ok(TxOutput::RegistrationInfo(result))
                }
            })
            .await?;

            match rx.await {
                Ok(Ok(TxOutput::RegistrationInfo(data))) => {
                    miner.miner_identity = Some(data.clone());
                    let miner_identity_json = serde_json::to_string(&MinerData {
                        miner_owner: data.0.to_string(),
                        miner_identity: (data.0, data.1),
                    })?;
                    miner.update_identity_file(&config::get_paths()?.identity_path, &miner_identity_json)?;
                },
                Ok(Err(e)) => println!("Error registering miner: {}", e),
                Err(_) => println!("Response channel dropped."),
                _ => println!("Missing identity data from registration event"),
            }
        }
    }

    let mut blocks = client.blocks().subscribe_finalized().await?;

    while let Some(Ok(block)) = blocks.next().await {
        println!("New block imported: {:?}", block.hash());
        let miner_identity = miner.miner_identity.clone()
            .ok_or(Error::Custom("Miner identity not present!!!".to_string()))?;
        println!("Active miner identity: {:?}", miner_identity);

        let events = block.events().await?;

        for event in events.iter() {
            match event {
                Ok(ev) => {
                    if let Err(e) = miner.process_event(&ev).await {
                        println!("Error processing event: {:?}", e);
                    }
                }
                Err(e) => eprintln!("Error decoding event: {:?}", e),
            }
        }
    }

    Ok(())
}
