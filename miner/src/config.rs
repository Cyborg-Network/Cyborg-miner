use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use tokio::sync::RwLock;
use std::sync::Arc;
use std::{
    path::PathBuf,
    env,
};
use subxt::utils::AccountId32;
use subxt::OnlineClient;
use subxt::PolkadotConfig;

use crate::error::{Error, Result};

//TODO put this in evironment variables
const LOG_PATH: &str = "/var/lib/cyborg/worker-node/logs/worker_log.txt";
const TASK_PATH: &str = "/var/lib/cyborg/worker-node/task/current_task";
const TASK_OWNER_PATH: &str = "/var/lib/cyborg/worker-node/task/task_owner.json";
const IDENTITY_PATH: &str = "/var/lib/cyborg/worker-node/identity.json";

#[derive(Debug)]
pub struct Paths {
    pub log_path: PathBuf,
    pub task_file_name: String,
    pub task_dir_path: PathBuf,
    pub task_owner_path: PathBuf,
    pub identity_path: PathBuf,
}

#[derive(Deserialize, Debug)]
struct MinerIdentity {
    owner: AccountId32, 
    id: u32
}

// We're setting a few global variables here for easy access throughout
pub static PATHS: OnceCell<Paths> = 
    OnceCell::new();
pub static PARACHAIN_CLIENT: OnceCell<OnlineClient<PolkadotConfig>> = 
    OnceCell::new();
pub static CESS_GATEWAY: Lazy<Arc<RwLock<String>>> = 
    Lazy::new(|| Arc::new(RwLock::new(String::from("https://deoss-sgp.cess.network"))));


/// Runs the configuration for the miner, everything in this function will fail fast to ensure correct setup when starting the miner
/// 
/// # Arguments
/// * `parachain_url` - A string representing the URL of the parachain node to connect to.
/// * `account_seed` - A string representing the seed phrase for generating the keypair.
pub async fn run_config(parachain_url: &str) {
    dotenv::dotenv().ok();

    let log_path = PathBuf::from(env::var("LOG_FILE_PATH").expect("LOG_PATH must be set"));
    let task_file_name = String::from(env::var("TASK_FILE_NAME").expect("TASK_FILE_NAME must be set"));
    let task_dir_path = PathBuf::from(env::var("TASK_DIR_PATH").expect("TASK_DIR_PATH must be set"));
    let identity_path = PathBuf::from(env::var("IDENTITY_FILE_PATH").expect("IDENTITY_PATH must be set"));
    let task_owner_path = PathBuf::from(env::var("TASK_OWNER_FILE_PATH").expect("TASK_OWNER_PATH must be set"));

    PATHS.set(Paths {
        log_path,
        task_file_name,
        task_dir_path,
        task_owner_path,
        identity_path,
    }).expect("Paths are already initialized!");

    let client = OnlineClient::<PolkadotConfig>::from_url(parachain_url).await.expect("Failed to connect to parachain node");

    PARACHAIN_CLIENT.set(client).expect("Client is already initialized!");
}

pub fn get_parachain_client() -> Result<&'static OnlineClient<PolkadotConfig>> {
    PARACHAIN_CLIENT
        .get()
        .ok_or(Error::parachain_client_not_intitialized())
}

pub fn get_paths() -> Result<&'static Paths> {
    PATHS
        .get()
        .ok_or(Error::config_paths_not_initialized())
}

pub async fn get_cess_gateway() -> String {
    CESS_GATEWAY
        .read()
        .await
        .clone()
}

pub async fn set_cess_gateway(url: &str) {
    let mut write_guard = CESS_GATEWAY
        .write()
        .await;
        
    *write_guard = url.to_string();
}