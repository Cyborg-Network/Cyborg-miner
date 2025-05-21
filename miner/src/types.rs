use crate::substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use subxt::utils::AccountId32;
use subxt_signer::sr25519::Keypair;
use std::sync::Arc;
use tokio::sync::RwLock;

// Datastructure for worker registration persistence
#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, Serialize, Deserialize)]
pub struct MinerData {
    pub miner_owner: String,
    pub miner_identity: (AccountId32, u64),
}

#[derive(Clone, Debug)]
pub struct CurrentTask {
    pub id: u64, 
    pub task_type: TaskType
}

#[derive(Clone, Debug)]
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
    // Some fields wrapped in an Arc to eg. keep extraction out of an RwLock before await cheap
    pub(crate) keypair: Keypair,
    pub parent_runtime: Arc<RwLock<ParentRuntime>>,
    pub miner_identity: Option<(AccountId32, u64)>,
    pub creator: Option<AccountId32>,
    pub current_task: Option<CurrentTask>,
    pub log_failure_count: u8,
}

pub struct ParentRuntime {
    //This is kept as an option, because it might be user dynamic in the future
    pub port: Option<u16>,
}
