use crate::substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::Keypair;

// Datastructure for worker registration persistence
#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, Serialize, Deserialize)]
pub struct MinerData {
    pub miner_owner: String,
    pub miner_identity: (AccountId32, u64),
}

#[derive(Clone)]
pub struct CurrentTask((u64, TaskType));

#[derive(Clone)]
pub enum TaskType {
    OpenInference,
    NeuroZk,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskOwner {
    pub task_owner: String,
}

pub struct MinerConfig {
    pub domain: BoundedVec<u8>,
    pub latitude: i32,
    pub longitude: i32,
    pub ram: u64,
    pub storage: u64,
    pub cpu: u16,
}

#[derive(Deserialize)]
pub struct IpResponse {
    pub ip: String,
}

pub struct AccountKeypair(pub Keypair);

/// Represents a client for interacting with the Cyborg blockchain.
///
/// This struct is used to interact with the Cyborg blockchain, manage key pairs,
/// and optionally communicate with IPFS or node URIs.
pub struct Miner {
    pub(crate) client: OnlineClient<PolkadotConfig>,
    pub(crate) keypair: Keypair,
    pub parent_runtime: ParentRuntime,
    pub cess_gateway: String,
    pub parachain_url: String,
    pub miner_identity: (AccountId32, u64),
    pub creator: AccountId32,
    pub log_path: PathBuf,
    pub task_path: PathBuf,
    pub config_path: PathBuf,
    pub task_owner_path: PathBuf,
    pub current_task: Option<(u64, TaskType)>,
}

pub struct ParentRuntime {
    pub task: Option<(u64, TaskType)>,
    //This is kept as an option, because it might be user dynamic in the future
    pub port: Option<u16>,
}
