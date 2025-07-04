use crate::config::{self /* , CESS_GATEWAY*/, PATHS};
use crate::crypto::dhx::MinerDH;
use crate::error::{Error, Result};
//use cess_rust_sdk::gateway::file::{download, download_encrypt};
//use cess_rust_sdk::polkadot::runtime_apis::asset_conversion_api::types::get_reserves::output;
//use cess_rust_sdk::subxt::ext::sp_core::{sr25519::Pair as PairS, Pair};
//use cess_rust_sdk::utils::account::get_pair_address_as_ss58_address;
//use cess_rust_sdk::utils::str::get_random_code;
//use tracing::info;
use crate::crypto::aes::decrypt;
use aes_gcm::aead::generic_array::GenericArray;
use futures_util::StreamExt;
use reqwest::Client;
use std::fs;
use std::path::Path;
use subxt_signer::sr25519::Keypair;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use x25519_dalek::PublicKey;

// This is currently out of use until CESS is fixed
/*
pub async fn download_model_archive(cess_fid: &str, cipher: &str) -> Result<()> {
    //! The extraction of the archive will be left up to the individual runtimes, as they might treat it differently
    println!("Starting download model archive: {}", cess_fid);

    info!("Retrieving model archive with fid: {}...", &cess_fid);

    let (task_file_name, task_dir_path) = {
        let paths = &PATHS.get()
        .ok_or(Error::config_paths_not_initialized())?;

        (&paths.task_file_name, &paths.task_dir_path)
    };

    std::fs::create_dir_all(task_dir_path)?;

    let output_path = format!("{}/{}", task_dir_path, task_file_name);

    let gateway = &CESS_GATEWAY
        .read()
        .await;

    let test_mnemonic = "bottom drive obey lake curtain smoke basket hold race lonely fit walk//Alice";
    let pair = PairS::from_string(test_mnemonic, None).unwrap();
    let acc = get_pair_address_as_ss58_address(pair.clone()).unwrap();
    let message = get_random_code(16).unwrap();
    let signed_msg = pair.sign(message.as_bytes());
    let _ = download(
        gateway,
        cess_fid,
        &acc,
        &message,
        signed_msg,
        &output_path,
    )
    .await?;

    info!("Model archive retrieved from CESS!");

    Ok(())
}
*/

pub async fn download_model_archive(
    storage_identifier: &str,
    cipher: &str,
    keypair: &Keypair,
) -> Result<()> {
    let (task_file_name, task_dir_path) = {
        let paths = &PATHS.get().ok_or(Error::config_paths_not_initialized())?;

        (&paths.task_file_name, &paths.task_dir_path)
    };
    std::fs::create_dir_all(task_dir_path)?;

    let base_storage_location = config::get_storage_location()?;
    let blob_url = format!("{}/{}", base_storage_location, storage_identifier);
    println!("Downloading model archive from: {}", blob_url);

    let output_path = format!("{}/{}", task_dir_path, task_file_name);
    println!("Saving model archive to: {}", output_path);

    let client = Client::new();
    let response = client.get(blob_url).send().await?;

    if !response.status().is_success() {
        return Err(Error::Custom(format!(
            "Failed to download blob: {}",
            response.status()
        )));
    }

    if !fs::metadata(&task_dir_path).is_ok() {
        return Err(Error::Custom(format!(
            "Directory does not exist: {}",
            task_dir_path
        )));
    }

    let mut stream = response.bytes_stream();
    let file_path = Path::new(&output_path);

    println!("File path: {}", file_path.display());
    let mut file = File::create(&file_path).await?;

    tracing::info!("Starting model download...");

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?
    }

    let miner_dh = MinerDH::new(keypair);
    let gatekeeper_pub = PublicKey::from(*GenericArray::from_slice(cipher.as_bytes()));

    let shared_secret = miner_dh.derive_shared_secret(gatekeeper_pub);

    let encrypted_data = std::fs::read(&output_path)?;
    let decrypted_data = decrypt(&encrypted_data, &shared_secret)
        .map_err(|e| Error::Custom(format!("Decryption failed: {}", e)))?;

    std::fs::write(&output_path, decrypted_data)?;

    tracing::info!("✅ Model successfully retrieved and decrypted!");
    Ok(())
}
