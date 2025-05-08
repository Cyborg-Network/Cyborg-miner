use crate::{error::Result, types::TaskType};
use axum::{extract::{ws::WebSocketUpgrade, ConnectInfo, State}, routing::get, serve, Router};
use std::{net::SocketAddr, path::PathBuf};
use tokio::net::TcpListener;
use neuro_zk_runtime::NeuroZKEngine;
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

async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
) -> Result<impl axum::response::IntoResponse> {
    match state.task.1 {
        TaskType::NeuroZk => {
            let engine = NeuroZKEngine::new(PathBuf::from(""));

            //let _ = engine.run(request_stream, response_stream)?;

            Ok("".into())
       },
       //TODO add OI entry point
       _ => return Err("Unknown task type".into())
   }
}
