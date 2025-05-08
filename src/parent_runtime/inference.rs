use crate::{error::Result, types::TaskType};
use axum::{routing::get, serve, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use neuro_zk_runtime;
use open_inference_runtime;

#[derive(Clone)]
struct AppState {
    task: (u64, TaskType),
}

pub async fn spawn_inference_server(task: &Option<(u64, TaskType)>, port: Option<u16>) -> Result<()> {
    if let (Some(task), Some(port)) = (task, port) {
        let state = AppState {
            task: task.clone(),
        };

        let app = Router::new()
            .route(&format!("/inference/{}", &task.0), get(ws_handler))
            .with_state(state);

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

        println!("listening on {}", listener.local_addr().unwrap());

        serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

        Ok(())
    } else {
        Err("No task or port provided".into())
    }
}

async fn ws_handler(state: AppState) -> Result<impl axum::response::IntoResponse> {
   match state.task.1 {
       TaskType::NeuroZk => {
            let engine = neuro_zk_runtime::NeuroZKEngine {
                model_archive_path: String::from(""),
                current_witness_path: String::from(""),
            };

            engine.start_engie(model_path, compiled_model_path, settings_path, srs_path, current_witness_path, proving_key_path, subxt_api, signer_keypair, input, output)
       },
       //TODO add OI entry point
       _ => return Err("Unknown task type".into())
   }
}
