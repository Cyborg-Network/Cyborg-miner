use crate::{error::Result, types::TaskType};
use axum::{extract::{ws::{Message, WebSocket, WebSocketUpgrade}, ConnectInfo, State}, routing::get, serve, Router};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use neuro_zk_runtime::NeuroZKEngine;
use futures::{SinkExt, StreamExt};
use open_inference_runtime::client::TritonClient;
use open_inference_runtime::client::TensorData;
use std::collections::HashMap;
use base64::decode;

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
            let mut receiver = receiver;
            println!("Starting Open Inference Task...");
            let client = TritonClient::new("http://localhost:8000/v2".to_string());

            while let Some(Ok(msg)) = receiver.next().await {
                if let Message::Binary(data) = msg {
                    match serde_json::from_slice::<Value>(&data) {
                        Ok(value) => {
                            if let Some(model_name) = value["model_name"].as_str() {
                                if let Some(inputs) = value["inputs"].as_object() {
                                    let parsed_inputs = inputs.iter().map(|(k, v)| {
                                        if let Some(base64_str) = v.as_str() {
                                            match decode(base64_str) {
                                                Ok(decoded) => (k.clone(), TensorData::U8(decoded)),
                                                Err(_) => (k.clone(), TensorData::Str(vec![base64_str.to_string()])),
                                            }
                                        } else if v.is_array() {
                                            let values: Vec<_> = v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect();
                                            (k.clone(), TensorData::F32(values))
                                        } else {
                                            (k.clone(), TensorData::Str(vec![v.to_string()]))
                                        }
                                    }).collect::<HashMap<String, TensorData>>();

                                    match client.align_inputs(model_name, parsed_inputs).await {
                                        Ok(input_data) => {
                                            let flattened_inputs = input_data
                                                .into_iter()
                                                .map(|(k, (tensor_data, _))| (k, tensor_data))
                                                .collect::<HashMap<String, TensorData>>();
                                    
                                            match client.run_inference(model_name, flattened_inputs).await {
                                                Ok(result) => response_stream(format!("Inference Result: {:?}", result)).await,
                                                Err(e) => response_stream(format!("Open Inference Error: {:?}", e)).await,
                                            }
                                        },
                                        Err(e) => {
                                            response_stream(format!("Input Alignment Error: {:?}", e)).await;
                                        }
                                    }
                                    
                                } else {
                                    response_stream("Missing 'inputs' parameter".to_string()).await;
                                }
                            } else {
                                response_stream("Missing 'model_name' parameter".to_string()).await;
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
