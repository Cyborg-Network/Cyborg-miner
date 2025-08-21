use bollard::Docker;
use bollard::query_parameters::{
    CreateContainerOptionsBuilder/* , RemoveContainerOptions*/, StartContainerOptions/* , StopContainerOptions,*/
};
use bollard::models::{HostConfig, PortBinding, ContainerCreateBody};
use futures::stream::TryStreamExt;
use std::collections::HashMap;
use std::default::Default;
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
                OpenInferenceTask::Huggingface(huggingface_task) => {
                    let _ = spawn_container(&huggingface_task.model_name).await?;
                    Ok(())
                }
                _ => {
                    tracing::error!("Unsupported task type!");
                    return Err("Unsupported task type!".into());
                }
            }
        }
        TaskKind::NeuroZK(_nzk_task) => {
            // TODO implement NZK
            //let _ = storage_interactor::azure::download_nzk_model(nzk_task).await?;
            Ok(())
        }
    }
}

async fn spawn_container(_model_name: &String) -> Result<()> {
    let image = "torch-infer:local";

    let docker = Docker::connect_with_local_defaults()?;

    let mut stream = docker.create_image(
        Some(bollard::query_parameters::CreateImageOptions {
            from_image: Some(image.to_string()),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(pull_result) = stream.try_next().await? {
        if let Some(status) = pull_result.status {
            println!("pull: {}", status);
        }
    }

    let container_name = "torch-infer";

    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        "8000/tcp".to_string(),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some("8000".to_string()),
        }]),
    );

    let config = ContainerCreateBody {
        image: Some(image.to_string()),
        host_config: Some(HostConfig {
            port_bindings: Some(port_bindings),
            ..Default::default()
        }),
        ..Default::default()
    };

    let create_contaienr_options = CreateContainerOptionsBuilder::new()
        .name(container_name)
        .build();

    let container = docker
        .create_container(
            Some(create_contaienr_options),
            config
        )
        .await?;

    println!("Created container {}", container.id);

    docker
        .start_container(&container.id, None::<StartContainerOptions>)
        .await?;
    println!("Started container {}", container.id);

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    /*

    docker
        .stop_container(&container.id, Some(StopContainerOptions { t: 5 }))
        .await?;
    println!("Stopped container {}", container.id);

    docker
        .remove_container(
            &container.id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await?;
    println!("Removed container {}", container.id);

    */

    Ok(())
}