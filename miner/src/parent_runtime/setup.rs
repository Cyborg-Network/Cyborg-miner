use crate::substrate_interface::api::runtime_types::cyborg_primitives::task::FlashInferTask;
use crate::{
    error::Result, 
    parent_runtime::storage_interactor, 
    substrate_interface::api::{
        runtime_types::cyborg_primitives::task::OpenInferenceTask, 
        task_management::events::task_scheduled::TaskKind,
    }
};

pub async fn process_task(task_kind: TaskKind) -> Result<()> {
    match task_kind {
        TaskKind::OpenInference(oi_task) => {
            match oi_task {
                OpenInferenceTask::Onnx(onnx_task) => {
                    let _ = storage_interactor::onnx::download_onnx_model(onnx_task).await?;
                    Ok(())
                },
            }
        }
        TaskKind::NeuroZK(_nzk_task) => {
            // TODO implement NZK
            //let _ = storage_interactor::azure::download_nzk_model(nzk_task).await?;
            Ok(())
        }
        TaskKind::FlashInferInfer(fi_task) => {
            match fi_task {
                FlashInferTask::Huggingface(huggingface_task) => {
                    println!("Received FlashInfer Huggingface Task, passing download responsibility on to docker container.");
                    Ok(())
                }
            }
        }
    }
}

