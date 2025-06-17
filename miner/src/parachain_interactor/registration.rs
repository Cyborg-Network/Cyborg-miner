use crate::config;
use crate::error::Result;
use crate::substrate_interface;
use crate::traits::ParachainInteractor;
use crate::types::Miner;
use crate::utils::tx_builder::register;
use crate::utils::tx_queue::TxOutput;
use serde::Deserialize;
use subxt::utils::AccountId32;
use std::fs;

#[derive(Deserialize)]
struct Identity {
    miner_owner: String,
    miner_identity: (AccountId32, u64),
}

pub async fn confirm_registration(miner: &Miner) -> Result<bool> {
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
            return Ok(true);
        }
    }

    println!("Miner is not registered");
    Ok(false)
}

pub async fn start_miner(miner: &mut Miner) -> Result<()> {
    println!("Starting miner...");

    println!("Waiting for tasks...");

    let client = config::get_parachain_client()?;
    let tx_queue = config::get_tx_queue()?;

    match miner.confirm_registration().await {
        Ok(true) => println!("Miner already registered"), 
        Ok(false) => {
            let keypair = miner.keypair.clone();
            let rx = tx_queue.enqueue( move || {
                let keypair = keypair.clone();
                async move {
                    let result = register(keypair).await?;
                    Ok(TxOutput::Message(result))
                }
            })
            .await?;

            match rx.await {
                Ok(Ok(TxOutput::Message(data))) => {
                    miner.update_identity_file(&config::get_paths()?.identity_path, &data)?;
                },
                Ok(Ok(TxOutput::Success)) => println!("Missing identity string from registration event"),
                Ok(Err(e)) => println!("Error registering miner: {}", e),
                Err(_) => println!("Response channel dropped."),
            }
        },
        Err(e) => {
            println!("Error confirming miner registration: {}, registering...", e);
            let keypair = miner.keypair.clone();
            let rx = tx_queue.enqueue( move || {
                let keypair = keypair.clone();
                async move {
                    let result = register(keypair).await?;
                    Ok(TxOutput::Message(result))
                }
            })
            .await?;

            match rx.await {
                Ok(Ok(TxOutput::Message(data))) => {
                    miner.update_identity_file(&config::get_paths()?.identity_path, &data)?;
                },
                Ok(Ok(TxOutput::Success)) => println!("Missing identity string from registration event"),
                Ok(Err(e)) => println!("Error registering miner: {}", e),
                Err(_) => println!("Response channel dropped."),
            }
        }
    }

    let mut blocks = client.blocks().subscribe_finalized().await?;

    while let Some(Ok(block)) = blocks.next().await {
        println!("New block imported: {:?}", block.hash());

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

    /* 
    // -----------------------------------------------DELETE-----------------

    //TODO uncomment this and remove the hardcoded cipher after subxt is regen
    //let storage_encryption_cipher = &task_scheduled.cipher;
    let storage_encryption_cipher = "password";
    let task_fid_string = "f".to_string();

    miner.current_task = Some(CurrentTask {
        id: 0,
        //TODO uncomment after subxt regen
        //task_type: task_scheduled.task_type,
        task_type: TaskType::NeuroZk,
    });

    info!("New task scheduled for worker: {}", task_fid_string);

    let parent_runtime_clone = Arc::clone(&miner.parent_runtime);
    let current_task_clone = miner.current_task.clone();

    println!("Current task: {current_task_clone:?}");

    if let Some(current_task) = current_task_clone {
        let handle_2 = tokio::spawn(async move {
            if let Err(e) = parent_runtime_clone
                .read()
                .await
                .download_model_archive(&task_fid_string, storage_encryption_cipher)
                .await
            {
                println!("Error downloading model archive: {}", e);
            };

            let handle = parent_runtime_clone
                .read()
                .await
                .spawn_inference_server(&current_task)
                .await;

            match handle {
                Ok(handle) => {
                    handle.await.ok();
                    println!("Inference server exited");
                }
                Err(e) => {
                    eprintln!("Error starting inference server: {}", e);
                }
            }
        });

        handle_2.await.map_err(|e| Error::Custom(e.to_string()))?;
    } else {
        return Err(Error::Custom("No current task".to_string()));
    }

    // -----------------------------------------------DELETE TO HERE-----------------

    */
    Ok(())
}
