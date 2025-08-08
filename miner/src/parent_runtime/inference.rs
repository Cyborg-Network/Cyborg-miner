use crate::config;
use crate::parent_runtime::server_control::SHUTDOWN_SENDER;
use crate::{
    config::get_paths,
    error::{Error, Result},
    types::{CurrentTask, TaskType},
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    routing::get,
    serve, Router,
};
use futures::{SinkExt, StreamExt};
use neuro_zk_runtime::NeuroZKEngine;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use tokio::{
    net::TcpListener,
    sync::{watch, Mutex},
};
use open_inference_runtime::{TritonClient,TensorData};

#[derive(Clone)]
pub enum InferenceEngine {
    OpenInference(Arc<Mutex<TritonClient>>),
    NeuroZk(Arc<Mutex<NeuroZKEngine>>),
}

#[derive(Clone)]
struct AppState {
    task: CurrentTask,
    engine: InferenceEngine,
    status: Arc<watch::Receiver<EngineStatus>>,
}

#[derive(Debug, Clone)]
enum EngineStatus {
    Idle,
    Initializing,
    Ready,
    Failed(String),
}

pub async fn spawn_inference_server(
    task: &CurrentTask,
    port: Option<u16>,
) -> Result<tokio::task::JoinHandle<()>> {
    tracing::info!("Spawning inference server for task {}", task.id);

    let (status_tx, status_rx) = watch::channel(EngineStatus::Idle);
    let paths = get_paths()?;
    
    let engine = match task.task_type {
        TaskType::OpenInference => {
            let triton_client = TritonClient::new("http://localhost:8000/v2",PathBuf::from(&paths.task_dir_path))
            .await
            .map_err(|e| Error::Custom(format!("Failed to create Triton client: {}", e.to_string())))?;
            InferenceEngine::OpenInference(Arc::new(Mutex::new(triton_client)))
        }

        TaskType::NeuroZk => {
            let neurozk_engine = NeuroZKEngine::new(PathBuf::from(format!(
                "{}/{}",
                paths.task_dir_path, paths.task_file_name
            )))
            .map_err(|e| Error::Custom(format!("Failed to create engine: {}", e.to_string())))?;
            InferenceEngine::NeuroZk(Arc::new(Mutex::new(neurozk_engine)))
        }
    };

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    {
        let mut global_sender = SHUTDOWN_SENDER.lock().unwrap();
        *global_sender = Some(shutdown_tx.clone());
    }

    
        let engine_clone = engine.clone();
        let status_tx = status_tx.clone();

        tokio::spawn(async move {
            let _ = status_tx.send(EngineStatus::Initializing);

            match &engine_clone {
                InferenceEngine::OpenInference(client) => {
                    let _ = status_tx.send(EngineStatus::Ready);
                }
                InferenceEngine::NeuroZk(engine_clone) => {
                    match engine_clone.lock().await.setup().await {
                        Ok(()) => {
                            let _ = status_tx.send(EngineStatus::Ready);
                        }
                        Err(e) => {
                            let _ = status_tx.send(EngineStatus::Failed(e.to_string()));
                        }
                    }
                }
            }
        });

    let state = AppState {
        task: task.clone(),
        engine: engine,
        status: Arc::new(status_rx),
    };

    let mut default_port: u16 = 3000;
    if let Some(port) = port {
        default_port = port
    }

    let route_path = format!("/{}", &task.id);
    let state_clone = state.clone();

    let handle = tokio::spawn(async move {
        let mut rx = Arc::clone(&state_clone.status).as_ref().clone();

        loop {
            if let EngineStatus::Ready = *rx.borrow() {
                break;
            }

            if let Err(e) = rx.changed().await {
                tracing::error!("Error while setting up inference engine, please contact support.");
                println!("Error setting up inference engine: {}", e);
                break;
            }
        }

        let app = Router::new()
            .route(&route_path, get(ws_handler))
            .with_state(state);

        let listener = match TcpListener::bind(format!("0.0.0.0:{}", default_port)).await {
            Ok(listener) => listener,
            Err(e) => {
                tracing::error!("Error while setting up inference engine, please contact support.");
                println!("Failed to bind to port {}: {}", default_port, e);
                return;
            }
        };

        let tailnet = match config::get_tailscale_net() {
            Ok(net) => net,
            Err(e) => {
                tracing::error!("Error while setting up inference engine, please contact support.");
                println!("Failed to get tailscale net: {}", e);
                return;
            }
        };

        let hostname = match std::process::Command::new("hostname").output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
            Err(e) => {
                tracing::error!("Error while setting up inference engine, please contact support.");
                println!("Failed to get hostname: {}", e);
                return;
            }
        };

        println!("Starting inference server...");
        tracing::info!("Inference engine ready, miner is reachable at https://{}.{}/{}", hostname, tailnet, route_path);

        if let Err(e) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async move {
                shutdown_rx.changed().await.ok();
                println!("Shutdown signal received, stopping inference server!");
            })
            .await
        {
            tracing::error!("Server failed to start: {}", e);
            return;
        };
    });

    Ok(handle)
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
    let current_status = state.status.borrow().clone();
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
            println!("Sending response: {}", response);
            async move {
                let _ = sender
                    .lock()
                    .await
                    .send(Message::Text(response.into()))
                    .await;
            }
        }
    };

    match current_status {
        EngineStatus::Ready => {
            match &state.engine {
                InferenceEngine::OpenInference(client) => {
                    let client = client.lock().await;
                    if let Err(e)=client.run(request_stream,response_stream).await{
                        tracing::error!("Error running inference ingine: {}",e);
                    }
                }
                InferenceEngine::NeuroZk(engine) => {
                    let engine = engine.lock().await;
                    if let Err(e) = engine.run(request_stream, response_stream).await {
                        tracing::error!("Error running NeuroZK inference engine: {}", e);
                    }
                }
            }
        }
        EngineStatus::Initializing => {
            sender
                .lock()
                .await
                .send(Message::Text("Engine is initializing...".into()))
                .await
                .ok();
        }
        EngineStatus::Failed(ref err) => {
            sender
                .lock()
                .await
                .send(Message::Text(
                    format!("Engine failed to initialize: {}", err).into(),
                ))
                .await
                .ok();
        }
        EngineStatus::Idle => {
            sender
                .lock()
                .await
                .send(Message::Text("Engine has not started.".into()))
                .await
                .ok();
        }
    }

    Ok(())
}
