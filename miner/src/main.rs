/// The main function serves as the entry point for the Cyborg Miner application.
/// It parses command-line arguments using Clap and executes the corresponding subcommand.
///
/// # Commands:
///
/// - `startminer`: Starts a mining session with the provided parachain URL URL, and account seed
///
/// # Errors:
///
/// Returns a `Box<dyn Error>` in case of failure, which could include errors from client building, registration, or mining operations.
///
/// # Usage:
///
/// Run the executable with appropriate arguments to start mining.
mod builder;
mod cli;
mod config;
mod error;
mod log;
mod parachain_interactor;
mod parent_runtime;
mod specs;
mod substrate_interface;
mod traits;
mod types;
mod utils;
mod crypto;

use builder::MinerBuilder;
use clap::Parser;
use cli::{Cli, Commands};
use config::run_config;
use error::Result;
use subxt_signer::SecretUri;
use traits::ParachainInteractor;
use subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Match on the provided subcommand and execute the corresponding action.
    match &cli.command {
        // Handle the "start_miner" subcommand.
        Some(Commands::StartMiner {
            parachain_url,
            account_seed,
        }) => {
            let _log_guard = log::init_logger();

            let uri = SecretUri::from_str(account_seed).expect("Keypair was not set correctly");
            let keypair = Keypair::from_uri(&uri).expect("Keypair from URI failed");

            run_config(parachain_url, keypair.clone()).await;

            // Build the Miner using the provided parachain URL, account seed, and CESS gateway.
            let mut miner = MinerBuilder::default()
                .parachain_url(parachain_url.to_string())
                .keypair(keypair)
                .config()?
                .build()
                .await?;

            // Start the mining session using the built miner.
            miner.start_miner().await?;
        }

        _ => {
            println!("No command provided. Exiting.");
        }
    }
    Ok(())
}
