use crate::error::{Error, Result};
use crate::substrate_interface::api::runtime_types::cyborg_primitives::worker::WorkerType;
use crate::types::Miner;
use crate::{config, substrate_interface};

pub async fn suspend_miner(miner: &Miner) -> Result<()> {
    let client = config::get_parachain_client()?;
    let miner_id = miner.miner_identity.
        as_ref().
        ok_or(Error::identity_not_initialized())?.
        1;

    // TODO This needs a special function and miners need a quarantine or other way to punish suspicious behavior
    let worker_suspension = substrate_interface::api::tx()
        .edge_connect()
        .toggle_worker_visibility(WorkerType::Executable, miner_id,  false);

    println!("Transaction Details:");
    println!("Module: {:?}", worker_suspension.pallet_name());
    println!("Call: {:?}", worker_suspension.call_name());
    println!("Parameters: {:?}", worker_suspension.call_data());

    let keypair = &miner.keypair;

    let miner_suspension_events = client
        .tx()
        .sign_and_submit_then_watch_default(&worker_suspension, keypair)
        .await
        .map(|e| {
            println!("Miner suspension submitted, waiting for transaction to be finalized...");
            e
        })?
        .wait_for_finalized_success()
        .await?;

    let suspension_event = miner_suspension_events
        .find_first::<substrate_interface::api::edge_connect::events::WorkerStatusUpdated>(
    )?;

    if let Some(event) = suspension_event {

        println!("Miner suspended successfully: {event:?}");
    } else {
        println!("Miner suspension failed");
    }

    Ok(())
}