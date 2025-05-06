use crate::{error::Result, worker::{CyborgClient, IpResponse, WorkerData}};
use std::path::PathBuf;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::{SecretUri, sr25519::Keypair as SR25519Keypair};
use std::str::FromStr;
use pinata_sdk::PinataApi;
use std::env;

pub struct NoKeypair;
pub struct AccountKeypair(SR25519Keypair);

/// A builder pattern for constructing a `CyborgClient` instance.
///
/// This builder allows for flexible configuration of the Cyborg client,
/// including setting the node URI, keypair, and IPFS URI.
pub struct CyborgClientBuilder<Keypair> {
    parachain_url: Option<String>,
    keypair: Keypair,
    ipfs_client: Option<PinataApi>,
    identity: (AccountId32, u64),
    creator: AccountId32,
    log_path: PathBuf,
    config_path: PathBuf,
    task_path: PathBuf,
    task_owner_path: PathBuf,
}

/// Default implementation for the `CyborgClientBuilder` when no keypair is provided.
///
/// This initializes the builder with default values where node URI IPFS URI, and current task are None
/// and the keypair is set to `NoKeypair`.
impl Default for CyborgClientBuilder<NoKeypair> {
    fn default() -> Self {
        CyborgClientBuilder {
            parachain_url: None,
            keypair: NoKeypair,
            ipfs_client: None,
            identity: (AccountId32::from([0u8; 32]), 0),
            creator: AccountId32::from([0u8; 32]),
            log_path: PathBuf::from("./"),
            config_path: PathBuf::from("./"),
            task_path: PathBuf::from("./"),
            task_owner_path: PathBuf::from("./"),
        }
    }
}

impl<Keypair> CyborgClientBuilder<Keypair> {
    /// Sets the node URI for the client to connect to.
    ///
    /// # Arguments
    /// * `url` - A string representing the WebSocket URL of the node.
    /// 
    /// # Returns
    /// A `CyborgClientBuilder` instance with the parachain_url set.
    pub fn parachain_url(mut self, url: String) -> Self {
        self.parachain_url = Some(url);
        self
    }

    /// Sets the keypair for the client using a provided seed phrase.
    ///
    /// # Arguments
    /// * `seed` - A string slice representing the seed phrase for generating the keypair.
    ///
    /// # Returns
    /// A `Result` that, if successful, contains a new `CyborgClientBuilder` instance with an `AccountKeypair`.
    pub fn keypair(
        self,
        seed: &str,
    ) -> Result<CyborgClientBuilder<AccountKeypair>> {
        println!("Keypair: {}", seed);
        let uri = SecretUri::from_str(seed)
            .expect("Keypair was not set correctly");
        let keypair = SR25519Keypair::from_uri(&uri)
            .expect("Keypair from URI failed");

        Ok(CyborgClientBuilder {
            parachain_url: self.parachain_url,
            keypair: AccountKeypair(keypair),
            ipfs_client: self.ipfs_client,
            identity: self.identity,
            creator: self.creator,
            log_path: self.log_path,
            config_path: self.config_path,
            task_path: self.task_path,
            task_owner_path: self.task_owner_path,
        })
    }

    /// Sets the IPFS URI for the client to use.
    ///
    /// # Arguments
    /// * `ipfs_url` - An optionally (in case it is not set as environment variable) provided string representing the IPFS server URL.
    /// * `ipfs_api_key` - An optionally (in case it is not set as environment variable) provided string representing the IPFS API key.
    /// * `ipfs_api_secret` - An optionally (in case it is not set as environment variable) provided string representing the IPFS API secret.
    ///     
    /// # Returns
    /// A `CyborgClientBuilder` instance with the IPFS URI set.
    pub async fn ipfs_api(mut self, ipfs_url: Option<String>, ipfs_api_key: Option<String>, ipfs_api_secret: Option<String>) -> Self {
        let url;
        let key;
        let secret;

        if let Some(ipfs_url) = ipfs_url {
            url = ipfs_url;
        } else {
            url = env::var("CYBORG_WORKER_NODE_IPFS_API_URL")
                .expect("Not able to process CYBORG_WORKER_NODE_IPFS_URL environment variable - please check if it is set.");
        }

        if let Some(ipfs_api_key) = ipfs_api_key {
            key = ipfs_api_key;
        } else {
            key = env::var("CYBORG_WORKER_NODE_IPFS_API_KEY")
                .expect("Not able to process CYBORG_WORKER_NODE_IPFS_API_KEY environment variable - please check if it is set.");
        }

        if let Some(ipfs_api_secret) = ipfs_api_secret {        
            secret = ipfs_api_secret;
        } else {
            secret = env::var("CYBORG_WORKER_NODE_IPFS_API_SECRET")
                .expect("Not able to process CYBORG_WORKER_NODE_IPFS_API_SECRET environment variable - please check if it is set.");
        }

        println!("IPFS API URL: {}", url);
        println!("IPFS API KEY: {}", key);
        println!("IPFS API SECRET: {}", secret);

        let api = PinataApi::new(key, secret)
            .expect("Not able to create IPFS API client");

        let result = api.test_authentication().await;

        if let Ok(_) = result {
            println!("IPFS authentication successful");
        } else {
            panic!("IPFS authentication failed, therefore cannot start worker.");
        }

        self.ipfs_client = Some(api);
        self
    }

    /// Sets the identity and the creator of the worker they are kept separate because the way that IDs are generated for the workers is subject to change.
    ///
    /// # Arguments
    /// * `config` - A `WorkerData` struct containing the identity and the creator of the worker.
    /// 
    /// # Returns
    /// A `CyborgClientBuilder` instance with the identity and the creator set.
    pub fn config(mut self, config: WorkerData) -> Self {
        self.identity = config.worker_identity;
        self.creator = AccountId32::from_str(&config.worker_owner).unwrap();
        self
    }

    /// Sets the paths for the log, config, and task files.
    ///
    /// # Arguments
    /// * `log_path` - A string representing the path to the log file.
    /// * `config_path` - A string representing the path to the config file.
    /// * `task_path` - A string representing the path to the task file.
    /// * `task_owner_path` - A string representing the path to the task owner file.
    /// 
    /// # Returns
    /// A `CyborgClientBuilder` instance with the required paths set.
    pub fn paths(mut self, log_path: String, config_path: String, task_path: String, task_owner_path: String) -> Self {
        self.log_path = PathBuf::from(log_path);
        self.config_path = PathBuf::from(config_path);
        self.task_path = PathBuf::from(task_path);
        self.task_owner_path = PathBuf::from(task_owner_path);
        self
    }
}

impl CyborgClientBuilder<AccountKeypair> {
    /// Builds the `CyborgClient` using the provided configurations.
    ///
    /// # Returns
    /// A `Result` that, if successful, contains the constructed `CyborgClient`.
    pub async fn build(self) -> Result<CyborgClient> {
        match &self.parachain_url {
            Some(url) => {
                // Create an online client that connects to the specified Substrate node URL.
                let client = OnlineClient::<PolkadotConfig>::from_url(url).await?;

                let current_ip = reqwest::get("https://api.ipify.org?format=json")
                .await?
                .json::<IpResponse>()
                .await?
                .ip;

                Ok(CyborgClient {
                    client,
                    keypair: self.keypair.0,
                    ipfs_client: self.ipfs_client.expect("Failed to initialize IPFS client, cannot run worker without connection to IPFS."),
                    node_uri: self.parachain_url.expect("Node URI was not set, cannot run worker without an endpoint connecting it to cyborg network."),
                    identity: self.identity,
                    creator: self.creator,
                    log_path: self.log_path,
                    config_path: self.config_path,
                    task_path: self.task_path,
                    task_owner_path: self.task_owner_path,
                    current_task: None,
                    current_ip,
                })
            }
            None => Err("No node URI provided. Please specify a node URI to connect.".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_uri() {
        // Test setting the node URI in the builder.
        let builder = CyborgClientBuilder::default().parachain_url("ws://127.0.0.1:9988".to_string());
        assert_eq!(builder.parachain_url, Some("ws://127.0.0.1:9988".to_string()));

        // Test setting both node URI and keypair.
        let builder = CyborgClientBuilder::default()
            .parachain_url("ws://127.0.0.1:9988".to_string())
            .keypair("//Alice");

        assert_eq!(
            builder.unwrap().parachain_url,
            Some("ws://127.0.0.1:9988".to_string())
        );
    }

    #[tokio::test]
    async fn test_keypair() {
        // Test setting the keypair in the builder.
        let builder = CyborgClientBuilder::default()
            .parachain_url("ws://127.0.0.1:9988".to_string())
            .keypair("//Alice")
            .unwrap();

        let uri_alice = SecretUri::from_str("//Alice").unwrap();
        let expected_public_key = SR25519Keypair::from_uri(&uri_alice)
            .expect("keypair was not set correctly")
            .public_key();

        assert_eq!(builder.keypair.0.public_key().to_account_id(), expected_public_key.to_account_id());
    }

    /* #[tokio::test]
    async fn test_ipfs_uri() -> Result<()> {
        // Test setting the IPFS URI in the builder.
        let builder = CyborgClientBuilder::default()
            .parachain_url("ws://127.0.0.1:9944".to_string())
            .keypair("//Alice")?
            .ipfs_api(Some("http://127.0.0.1:5001".to_string()), Some("KEY".to_string()), Some("SECRET".to_string())).await;

        assert!(builder.ipfs_client.is_some());

        // Test setting the IPFS URI without a keypair.
        let builder = CyborgClientBuilder::default()
            .parachain_url("ws://127.0.0.1:9944".to_string())
            .ipfs_api(Some("http://127.0.0.1:5001".to_string()), Some("KEY".to_string()), Some("SECRET".to_string())).await;

        assert!(builder.ipfs_client.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_config() -> Result<()> {
        let uri_alice = SecretUri::from_str("//Alice").unwrap();
        let expected_key = SR25519Keypair::from_uri(&uri_alice)
            .expect("keypair was not set correctly");

        let mock_config = WorkerData {
            worker_owner: "Alice".to_string(),
            worker_identity: (AccountId32::from(expected_key.public_key()), 0),
        };
        // Test setting the config in the builder.
        let builder = CyborgClientBuilder::default()
            .parachain_url("ws://127.0.0.1:9944".to_string())
            .keypair("//Alice")?
            .config(mock_config.clone());

        assert_eq!(builder.identity, (AccountId32::from_str("Alice").unwrap(), 0));
        assert_eq!(builder.creator, AccountId32::from_str("Alice").unwrap());

        // Test setting the config without a keypair.
        let builder = CyborgClientBuilder::default()
            .parachain_url("ws://127.0.0.1:9944".to_string())
            .config(mock_config.clone());

        assert_eq!(builder.identity, (AccountId32::from_str("Alice").unwrap(), 0));
        assert_eq!(builder.creator, AccountId32::from_str("Alice").unwrap());

        Ok(())
    }
    */

    #[tokio::test]
    async fn test_paths() -> Result<()> {

        // Test setting the paths in the builder.
        let builder = CyborgClientBuilder::default()
            .parachain_url("ws://127.0.0.1:9944".to_string())
            .keypair("//Alice")?
            .paths(
                "/tmp/cyborg.log".to_string(),
                "/tmp/cyborg.config".to_string(),
                "/tmp/cyborg.task".to_string(),
                "/tmp/cyborg.task_owner".to_string(),
            );

        assert_eq!(builder.log_path, PathBuf::from("/tmp/cyborg.log"));
        assert_eq!(builder.config_path, PathBuf::from("/tmp/cyborg.config"));
        assert_eq!(builder.task_path, PathBuf::from("/tmp/cyborg.task"));
        assert_eq!(builder.task_owner_path, PathBuf::from("/tmp/cyborg.task_owner"));

        // Test setting the paths without a keypair.
        let builder = CyborgClientBuilder::default()
            .parachain_url("ws://127.0.0.1:9944".to_string())
            .paths(
                "/tmp/cyborg.log".to_string(),
                "/tmp/cyborg.config".to_string(),
                "/tmp/cyborg.task".to_string(),
                "/tmp/cyborg.task_owner".to_string(),
            );

        assert_eq!(builder.log_path, PathBuf::from("/tmp/cyborg.log"));
        assert_eq!(builder.config_path, PathBuf::from("/tmp/cyborg.config"));
        assert_eq!(builder.task_path, PathBuf::from("/tmp/cyborg.task"));
        assert_eq!(builder.task_owner_path, PathBuf::from("/tmp/cyborg.task_owner"));

        Ok(())
    }
}