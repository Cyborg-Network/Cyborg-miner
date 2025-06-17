use crate::config::{CESS_GATEWAY, PATHS};
use crate::error::{Error, Result};
use cess_rust_sdk::gateway::file::download_encrypt;
use cess_rust_sdk::subxt::ext::sp_core::{sr25519::Pair as PairS, Pair};
use cess_rust_sdk::utils::account::get_pair_address_as_ss58_address;
use cess_rust_sdk::utils::str::get_random_code;
use tracing::info;

pub async fn download_model_archive(cess_fid: &str, cipher: &str) -> Result<()> {
    //! The extraction of the archive will be left up to the individual runtimes, as they might treat it differently
    /*
    println!("Starting download model archive: {}", cess_fid);

    info!("Retrieving model archive with fid: {}...", &cess_fid);

    let (task_file_name, task_dir_path) = {
        let paths = &PATHS.get()
        .ok_or(Error::config_paths_not_initialized())?;

        (&paths.task_file_name, &paths.task_dir_path)
    };

    std::fs::create_dir_all(task_dir_path)?;

    let task_dir_path = task_dir_path.to_str()
        .ok_or(Error::Custom(String::from("Could not convert task dir path to str")))?;


    let output_path = format!("{}/{}", task_dir_path, task_file_name);

    let gateway = &CESS_GATEWAY
        .read()
        .await;

    let test_mnemonic = "bottom drive obey lake curtain smoke basket hold race lonely fit walk//Alice";
    let pair = PairS::from_string(test_mnemonic, None).unwrap();
    let acc = get_pair_address_as_ss58_address(pair.clone()).unwrap();
    let message = get_random_code(16).unwrap();
    let signed_msg = pair.sign(message.as_bytes());
    let _ = download_encrypt(
        gateway,
        cess_fid,
        &acc,
        &message,
        signed_msg,
        &output_path,
        cipher,
    )
    .await?;

    */

    info!("Model archive retrieved from CESS!");

    Ok(())
}
