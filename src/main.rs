/// The main function serves as the entry point for the Cyborg Client application.
/// It parses command-line arguments using Clap and executes the corresponding subcommand.
///
/// # Commands:
///
/// - `registration`: Registers a worker with the given blockchain node URL and account seed.
/// - `startmining`: Starts a mining session with the provided blockchain node URL, account seed, and IPFS URL.
///
/// # Errors:
///
/// Returns a `Box<dyn Error>` in case of failure, which could include errors from client building, registration, or mining operations.
///
/// # Usage:
///
/// Run the executable with appropriate subcommands to register or start mining a worker.
mod builder;
mod cli;
mod specs;
mod substrate_interface;
mod worker;

use crate::worker::BlockchainClient;
use builder::CyborgClientBuilder;
use clap::Parser;
use cli::{Cli, Commands};
use std::fs;
mod error;
mod utils;

pub use self::error::{Error, Result};
//use subxt::ext::jsonrpsee::core::client::error;

const CONFIG_PATH: &str = "/var/lib/cyborg/worker-node/config/worker_config.json";
const LOG_PATH: &str = "/var/lib/cyborg/worker-node/logs/worker_log.txt";
const TASK_PATH: &str = "/var/lib/cyborg/worker-node/task/current_task";
const TASK_OWNER_PATH: &str = "/var/lib/cyborg/worker-node/task/task_owner.json";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Match on the provided subcommand and execute the corresponding action.
    match &cli.command {
        // Handle the "registration" subcommand.
        Some(Commands::Registration {
            parachain_url,
            account_seed,
            ipfs_url,
            ipfs_api_key,
            ipfs_api_secret,
        }) => {
            println!("Registering worker with API URL: {}", parachain_url);

            // Build the Cyborg client using the provided API URL and account seed.
            let client = CyborgClientBuilder::default()
                .parachain_url(parachain_url.to_string())
                .keypair(account_seed)?
                .ipfs_api(
                    Some(ipfs_url.to_string()),
                    Some(ipfs_api_key.to_string()),
                    Some(ipfs_api_secret.to_string()),
                )
                .await
                .paths(
                    LOG_PATH.to_string(),
                    CONFIG_PATH.to_string(),
                    TASK_PATH.to_string(),
                    TASK_OWNER_PATH.to_string(),
                )
                .build()
                .await?;

            // Register the worker using the built client.
            client.register_worker().await?;
        }

        // Handle the "startmining" subcommand.
        Some(Commands::Startmining {
            parachain_url,
            account_seed,
            //ipfs_url,
        }) => {
            println!("Starting mining session. Parachain URL: {}", parachain_url);

            let config_string =
                fs::read_to_string("/var/lib/cyborg/worker-node/config/worker_config.json")?;

            let config: worker::WorkerData = serde_json::from_str(&config_string)?;

            println!("Config: {config:?}");

            // Build the Cyborg client using the provided API URL, account seed, and IPFS URL.
            let mut client = CyborgClientBuilder::default()
                .parachain_url(parachain_url.to_string())
                .keypair(account_seed)?
                .ipfs_api(None, None, None)
                .await
                .config(config)
                .paths(
                    LOG_PATH.to_string(),
                    CONFIG_PATH.to_string(),
                    TASK_PATH.to_string(),
                    TASK_OWNER_PATH.to_string(),
                )
                .build()
                .await?;

            // Start the mining session using the built client.
            client.start_mining_session().await?;
        }

        _ => {
            println!("No command provided. Exiting.");
        }
    }
    Ok(())
}
