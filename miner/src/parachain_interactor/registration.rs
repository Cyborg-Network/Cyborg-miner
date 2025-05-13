use crate::config;
use crate::error::Result;
use crate::parachain_interactor::identity;
use crate::specs;
use crate::substrate_interface;
use crate::substrate_interface::api::polkadot_xcm::storage::types::queries;
use crate::substrate_interface::api::runtime_types::cyborg_primitives::worker::{WorkerType, Worker};
use crate::traits::ParachainInteractor;
use crate::types::{Miner, MinerData};
use log::info;
use std::process::id;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn confirm_registration(miner: Arc<RwLock<Miner>>) -> Result<bool> {
    let client = config::get_parachain_client()?;
    let identity = &miner.read().await.miner_identity;

    let miner_registration_confirmation_query = substrate_interface::api::storage()
        .edge_connect()
        .executable_workers(&identity.0, &identity.1);

    let result = client
        .storage()
        .at_latest()
        .await?
        .fetch(&miner_registration_confirmation_query)
        .await?;

    if let Some(_) = result {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn register_miner(miner: Arc<RwLock<Miner>>) -> Result<()> {
    let client = config::get_parachain_client()?;
    let config_path = &config::get_paths()?.identity_path;
    let keypair = &miner.read().await.keypair;

    let worker_specs = specs::gather_worker_spec().await?;

    let worker_registration = substrate_interface::api::tx()
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
    println!("Module: {:?}", worker_registration.pallet_name());
    println!("Call: {:?}", worker_registration.call_name());
    println!("Parameters: {:?}", worker_registration.call_data());

    let worker_registration_events = client
        .tx()
        .sign_and_submit_then_watch_default(&worker_registration, keypair)
        .await
        .map(|e| {
            println!("Miner registration submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let registration_event = worker_registration_events
        .find_first::<substrate_interface::api::edge_connect::events::WorkerRegistered>(
    )?;

    if let Some(event) = registration_event {
        let worker_identity_json = serde_json::to_string(&MinerData {
            miner_owner: event.creator.clone().to_string(),
            miner_identity: event.worker.clone(),
        })?;

        miner.update_identity_file(config_path, &worker_identity_json)?;

        println!("Miner registered successfully: {event:?}");
    } else {
        println!("Miner registration failed");
    }

    Ok(())
}

pub async fn start_miner(miner: Arc<RwLock<Miner>>) -> Result<()> {
    println!("Starting miner...");

    miner.write_log("Waiting for tasks...");

    let client = config::get_parachain_client()?; 

    if !miner.confirm_registration().await? {
        miner.register_miner().await?
    }

    let mut blocks = client.blocks().subscribe_finalized().await?;

    while let Some(Ok(block)) = blocks.next().await {
        info!("New block imported: {:?}", block.hash());

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
