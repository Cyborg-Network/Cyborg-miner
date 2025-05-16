use crate::{error::Result, types::TaskType};
use axum::{extract::{ws::{Message, WebSocket, WebSocketUpgrade}, ConnectInfo, State}, routing::get, serve, Router};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use neuro_zk_runtime::NeuroZKEngine;
use futures::{SinkExt, StreamExt};
use open_inference_runtime::client::TritonClient;
use serde_json::Value;

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
    let (sender, receiver) = socket.split();

    // Wrap the sender in an Arc<Mutex> to allow safe shared access.
    let sender = Arc::new(Mutex::new(sender));

    // Response stream to send back data to the client
    let response_stream = {
        let sender = Arc::clone(&sender);
        move |response: String| {
            let sender = Arc::clone(&sender);
            async move {
                let _ = sender.lock().await.send(Message::Text(response.into())).await;
            }
        }
    };

    // Match on the task type and handle separately
    match state.task.1 {
        TaskType::NeuroZk => {
            let mut receiver = receiver; 
            let engine = NeuroZKEngine::new(PathBuf::from(""));
            let request_stream = Box::pin(async_stream::stream! {
                while let Some(Ok(msg)) = receiver.next().await {
                    if let Message::Text(text) = msg {
                        yield text.to_string();
                    }
                }
            });

            let _ = engine.run(request_stream, response_stream);
            Ok(())
        },
        TaskType::OpenInference => {
            println!("Starting Open Inference Task...");

            
            let mut receiver = receiver;

            while let Some(Ok(msg)) = receiver.next().await {
                if let Message::Text(text) = msg {
                    match serde_json::from_str::<Value>(&text) {
                        Ok(value) => {
                            if let (Some(model_name), Some(archive_path), Some(extract_to), Some(input_file)) = (
                                value["model_name"].as_str(),
                                value["archive_path"].as_str(),
                                value["extract_to"].as_str(),
                                value["input_file"].as_str()
                            ) {
                                let client = TritonClient::new("http://localhost:8000".to_string());
                                match client.run_inference(model_name, archive_path, extract_to, input_file).await {
                                    Ok(result) => {
                                        response_stream(format!("Inference Result: {:?}", result)).await;
                                    },
                                    Err(e) => {
                                        response_stream(format!("Open Inference Error: {:?}", e)).await;
                                    }
                                }
                            } else {
                                response_stream("Missing parameters".to_string()).await;
                            }
                        },
                        Err(e) => {
                            response_stream(format!("JSON Parse Error: {:?}", e)).await;
                        }
                    }
                }
            }
            Ok(())
        },
        _ => Err("Unknown task type".into()),
    }
}

