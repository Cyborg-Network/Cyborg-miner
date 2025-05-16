use crate::{error::Result, types::TaskType};
use axum::{extract::{ws::{Message, WebSocket, WebSocketUpgrade}, ConnectInfo, State}, routing::get, serve, Router};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use neuro_zk_runtime::NeuroZKEngine;
use futures::{SinkExt, StreamExt};
use open_inference_runtime;

#[derive(Clone)]
struct AppState {
    task: (u64, TaskType),
}

pub async fn spawn_inference_server(task: &Option<(u64, TaskType)>, port: Option<u16>) -> Result<()> {
    if let Some(task) = task {
        let state = AppState {
            task: task.clone(),
        };

        let mut default_port: u16 = 3000;

        if let Some(port) = port {
           default_port = port 
        }

        let app = Router::new()
            .route(&format!("/inference/{}", &task.0), get(ws_handler))
            .with_state(state);

        let listener = TcpListener::bind(format!("127.0.0.1:{}", default_port)).await?;

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

#[axum_macros::debug_handler]
async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| {
        let state = state.clone();

        async move {
            if let Err(e) = handle_socket(socket, state).await {
                eprintln!("WebSocket handling error: {:?}", e);
            }
        }
    })
}

async fn handle_socket(socket: WebSocket, state: AppState) -> Result<()> {
    let (sender, mut receiver) = socket.split();

    let request_stream = Box::pin(async_stream::stream! {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                yield text.to_string();
            }
        }
    });

    let sender = Arc::new(Mutex::new(sender));

    let response_stream = {
        let sender = Arc::clone(&sender);
        move |response: String| {
            let sender = Arc::clone(&sender);
            async move {
                let _ = sender.lock().await.send(Message::Text(response.into())).await;
            }
        }
    };

    match state.task.1 {
        TaskType::NeuroZk => {

            let engine = NeuroZKEngine::new(PathBuf::from(""));

            let _ = engine.run(request_stream, response_stream);
            
            Ok(())
       },
       //TODO add OI entry point
       _ => return Err("Unknown task type".into())
    }
}
