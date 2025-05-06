use crate::{error::Result, substrate_interface};
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};

// Struct that contains the data that the worker needs to execute a task
pub struct CyborgTask {
    pub id: u64,
    pub owner: AccountId32,
    pub cid: String,
}

pub async fn get_task(api: &OnlineClient<PolkadotConfig>, task_id: u64) -> Result<CyborgTask> {
    let task_address = substrate_interface::api::storage()
        .task_management()
        .tasks(task_id);

    let task_query = api
        .storage()
        .at_latest()
        .await?
        .fetch(&task_address)
        .await?;

    if let Some(task) = task_query {
        let ipfs_cid_string = String::from_utf8(task.metadata.0)?;

        Ok(CyborgTask {
            id: task_id,
            owner: task.task_owner,
            cid: ipfs_cid_string,
        })
    } else {
        Err("Task not found".into())
    }
}
