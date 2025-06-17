use crate::{
    config,
    error::Result,
    types::{AccountKeypair, Miner, MinerData, ParentRuntime},
};
use std::{fs, str::FromStr, sync::Arc};
use subxt::utils::AccountId32;
use subxt_signer::{sr25519::Keypair as SR25519Keypair, SecretUri};
use tokio::sync::RwLock;
use tracing::warn;

/// A builder pattern for constructing a `Miner` instance.
///
/// This builder allows for flexible configuration of the Miner,
/// including setting the parachain node URL, keypair, and CESS gateway.
pub struct MinerBuilder<Keypair> {
    parachain_url: Option<String>,
    keypair: Keypair,
    identity: Option<(AccountId32, u64)>,
    creator: Option<AccountId32>,
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
            identity: None,
            creator: None,
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
    pub fn keypair(self, keypair: SR25519Keypair) -> MinerBuilder<AccountKeypair> {

        MinerBuilder {
            parachain_url: self.parachain_url,
            keypair: AccountKeypair(keypair),
            identity: self.identity,
            creator: self.creator,
        }
    }

    /// Sets the identity and the creator of the miner they are kept separate because the way that IDs are generated for the workers is subject to change.
    ///
    /// # Arguments
    /// * `config` - A `MinerData` struct containing the identity and the creator of the worker.
    ///
    /// # Returns
    /// A `MinerBuilder` instance with the identity and the creator set.
    pub fn config(mut self) -> Result<Self> {
        let mut identity: Option<(AccountId32, u64)> = None;
        let mut creator: Option<AccountId32> = None;

        if let Some(paths) = config::PATHS.get() {
            match fs::read_to_string(&paths.identity_path)
                .and_then(|s| serde_json::from_str::<MinerData>(&s).map_err(|e| e.into()))
            {
                Ok(config) => {
                    identity = Some(config.miner_identity.clone());
                    creator = Some(config.miner_identity.0);
                }
                Err(e) => {
                    warn!("No miner identity present, identity will be set when registering...")
                }
            }
        } else {
            warn!("No miner identity present, identity will be set when registering...");
        }

        self.identity = identity;
        self.creator = creator;
        Ok(self)
    }
}

impl MinerBuilder<AccountKeypair> {
    /// Builds the `Miner` using the provided configurations.
    ///
    /// # Returns
    /// A `Result` that, if successful, contains the constructed `Miner`.
    pub async fn build(self) -> Result<Miner> {
        Ok(Miner {
            parent_runtime: Arc::new(RwLock::new(ParentRuntime { port: None })),
            keypair: self.keypair.0,
            miner_identity: self.identity,
            creator: self.creator,
            current_task: None,
            log_failure_count: 0,
        })
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

        let uri = SecretUri::from_str("//Alice").unwrap();
        let keypair = SR25519Keypair::from_uri(&uri).expect("keypair was not set correctly");

        // Test setting both node URI and keypair.
        let builder = MinerBuilder::default()
            .parachain_url("ws://127.0.0.1:9988".to_string())
            .keypair(keypair);

        assert_eq!(
            builder.parachain_url,
            Some("ws://127.0.0.1:9988".to_string())
        );
    }

    #[tokio::test]
    async fn test_keypair() {
        let uri = SecretUri::from_str("//Alice").unwrap();
        let keypair = SR25519Keypair::from_uri(&uri).expect("keypair was not set correctly");

        // Test setting the keypair in the builder.
        let builder = MinerBuilder::default()
            .parachain_url("ws://127.0.0.1:9988".to_string())
            .keypair(keypair);

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
        let uri = SecretUri::from_str("//Alice").unwrap();
        let keypair = SR25519Keypair::from_uri(&uri).expect("keypair was not set correctly");

        // Test setting the paths in the builder.
        let builder = MinerBuilder::default()
            .parachain_url("ws://127.0.0.1:9944".to_string())
            .keypair(keypair);

        // Test setting the paths without a keypair.
        let builder = MinerBuilder::default().parachain_url("ws://127.0.0.1:9944".to_string());

        Ok(())
    }
}
