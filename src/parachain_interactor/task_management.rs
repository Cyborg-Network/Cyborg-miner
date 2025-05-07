use crate::{
    error::Result, 
    substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec
};

pub async fn confirm_task_reception() -> Result<()> {
    println!("Task reception confirmation is unimplemented!!!!");

    Ok(())
}

pub async fn stop_task_and_vacate_miner() -> Result<()> {
    println!("Task stop and vacate miner is unimplemented!!!!");

    Ok(())    
}

pub async fn submit_zkml_proof(proof: Vec<u8>) -> Result<()> {
    let suxt_proof: BoundedVec<u8> = BoundedVec::from(BoundedVec(proof));

    //TODO implement submission logic

    Ok(())
}