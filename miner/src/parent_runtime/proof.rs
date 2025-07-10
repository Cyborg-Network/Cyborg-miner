use std::path::PathBuf;

use crate::{config::get_paths, error::{Error, Result}};
use neuro_zk_runtime::{self, NeuroZKEngine};

pub async fn generate_proof() -> Result<Vec<u8>> {
    let paths = get_paths()?;

    let engine = NeuroZKEngine::new(
        PathBuf::from(format!("{}/{}", paths.task_dir_path, paths.task_file_name))
        ).map_err(
            |e| Error::Custom(format!("Failed to create engine: {}", e.to_string()))
        )?;

    let proof = engine.prove_inference(
        &paths.task_dir_path, 
        "network.ezkl", 
        "pk.key", 
        "kzg.srs", 
        "proof-witness.json", 
        "input.json"
    )
    .await
    .map_err(|e| Error::Custom(format!("Failed to generate proof: {}", e.to_string())))?;

    Ok(proof.into())
}
