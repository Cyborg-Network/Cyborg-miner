use crate::{
    error::Result,
    types::{AccountKeypair, Miner, MinerData, ParentRuntime},
};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use subxt::utils::AccountId32;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::{sr25519::Keypair as SR25519Keypair, SecretUri};

/// A builder pattern for constructing a `Miner` instance.
///
/// This builder allows for flexible configuration of the Miner,
/// including setting the parachain node URL, keypair, and CESS gateway.
pub struct MinerBuilder<Keypair> {
    parachain_url: Option<String>,
    keypair: Keypair,
    cess_gateway: Option<String>,
    identity: (AccountId32, u64),
    creator: AccountId32,
    log_path: PathBuf,
    config_path: PathBuf,
    task_path: PathBuf,
    task_owner_path: PathBuf,
}

pub struct NoKeypair;

/// Default implementation for the `MinerBuilder` when no keypair is provided.
///
/// This initializes the builder with default values where parachain node URL, CESS gateway, and current task are None
/// and the keypair is set to `NoKeypair`.
impl Default for MinerBuilder<NoKeypair> {
    fn default() -> Self {
        MinerBuilder {
            parachain_url: None,
            keypair: NoKeypair,
            cess_gateway: None,
            identity: (AccountId32::from([0u8; 32]), 0),
            creator: AccountId32::from([0u8; 32]),
            log_path: PathBuf::from("./"),
            config_path: PathBuf::from("./"),
            task_path: PathBuf::from("./"),
            task_owner_path: PathBuf::from("./"),
        }
    }
}

impl<Keypair> MinerBuilder<Keypair> {
    /// Sets the parachain node URL for the miner to connect to.
    ///
    /// # Arguments
    /// * `url` - A string representing the WebSocket URL of the node.
    ///
    /// # Returns
    /// A `MinerBuilder` instance with the parachain_url set.
    pub fn parachain_url(mut self, url: String) -> Self {
        self.parachain_url = Some(url);
        self
    }

    /// Sets the keypair for the miner using a provided seed phrase.
    ///
    /// # Arguments
    /// * `seed` - A string slice representing the seed phrase for generating the keypair.
    ///
    /// # Returns
    /// A `Result` that, if successful, contains a new `MinerBuilder` instance with an `AccountKeypair`.
    pub fn keypair(self, seed: &str) -> Result<MinerBuilder<AccountKeypair>> {
        println!("Keypair: {}", seed);
        let uri = SecretUri::from_str(seed).expect("Keypair was not set correctly");
        let keypair = SR25519Keypair::from_uri(&uri).expect("Keypair from URI failed");

        Ok(MinerBuilder {
            parachain_url: self.parachain_url,
            keypair: AccountKeypair(keypair),
            cess_gateway: self.cess_gateway,
            identity: self.identity,
            creator: self.creator,
            log_path: self.log_path,
            config_path: self.config_path,
            task_path: self.task_path,
            task_owner_path: self.task_owner_path,
        })
    }

    /// Sets the CESS gateway for the miner to use.
    ///
    /// # Arguments
    /// * `cess_gateway` - An optionally (in case it is not set as environment variable) provided string representing the CESS gateway URL.
    ///     
    /// # Returns
    /// A `MinerBuilder` instance with the CESS gateway set.
    pub async fn cess_gateway(mut self, cess_gateway: Option<String>) -> Self {
        let gateway;

        if let Some(cess_gateway) = cess_gateway {
            gateway = cess_gateway;
        } else {
            gateway = env::var("MINER_CESS_GATEWAY")
                .expect("Not able to process MINER_CESS_GATEWAY environment variable - please check if it is set.");
        }

        println!("CESS GATEWAY: {}", gateway);

        self.cess_gateway = Some(gateway);
        self
    }

    /// Sets the identity and the creator of the miner they are kept separate because the way that IDs are generated for the workers is subject to change.
    ///
    /// # Arguments
    /// * `config` - A `MinerData` struct containing the identity and the creator of the worker.
    ///
    /// # Returns
    /// A `MinerBuilder` instance with the identity and the creator set.
    pub fn config(mut self, config: MinerData) -> Self {
        self.identity = config.miner_identity;
        self.creator = AccountId32::from_str(&config.miner_owner).unwrap();
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
    /// A `MinerBuilder` instance with the required paths set.
    pub fn paths(
        mut self,
        log_path: String,
        config_path: String,
        task_path: String,
        task_owner_path: String,
    ) -> Self {
        self.log_path = PathBuf::from(log_path);
        self.config_path = PathBuf::from(config_path);
        self.task_path = PathBuf::from(task_path);
        self.task_owner_path = PathBuf::from(task_owner_path);
        self
    }
}

impl MinerBuilder<AccountKeypair> {
    /// Builds the `Miner` using the provided configurations.
    ///
    /// # Returns
    /// A `Result` that, if successful, contains the constructed `Miner`.
    pub async fn build(self) -> Result<Miner> {
        match &self.parachain_url {
            Some(url) => {
                // Create an online client that connects to the specified Substrate node URL.
                let client = OnlineClient::<PolkadotConfig>::from_url(url).await?;

                Ok(Miner {
                    parent_runtime: ParentRuntime{ task: None , port: None, },
                    client,
                    keypair: self.keypair.0,
                    cess_gateway: self.cess_gateway
                        .expect("Failed to initialize IPFS client, cannot run worker without connection to IPFS."),
                    parachain_url: self.parachain_url
                        .expect("Node URI was not set, cannot run worker without an endpoint connecting it to cyborg network."),
                    miner_identity: self.identity,
                    creator: self.creator,
                    log_path: self.log_path,
                    config_path: self.config_path,
                    task_path: self.task_path,
                    task_owner_path: self.task_owner_path,
                    current_task: None,
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
        let builder = MinerBuilder::default().parachain_url("ws://127.0.0.1:9988".to_string());
        assert_eq!(
            builder.parachain_url,
            Some("ws://127.0.0.1:9988".to_string())
        );

        // Test setting both node URI and keypair.
        let builder = MinerBuilder::default()
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
        let builder = MinerBuilder::default()
            .parachain_url("ws://127.0.0.1:9988".to_string())
            .keypair("//Alice")
            .unwrap();

        let uri_alice = SecretUri::from_str("//Alice").unwrap();
        let expected_public_key = SR25519Keypair::from_uri(&uri_alice)
            .expect("keypair was not set correctly")
            .public_key();

        assert_eq!(
            builder.keypair.0.public_key().to_account_id(),
            expected_public_key.to_account_id()
        );
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
        let builder = MinerBuilder::default()
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
        assert_eq!(
            builder.task_owner_path,
            PathBuf::from("/tmp/cyborg.task_owner")
        );

        // Test setting the paths without a keypair.
        let builder = MinerBuilder::default()
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
        assert_eq!(
            builder.task_owner_path,
            PathBuf::from("/tmp/cyborg.task_owner")
        );

        Ok(())
    }
}
