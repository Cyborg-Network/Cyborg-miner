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
use std::collections::HashMap;
use neuro_zk_runtime::NeuroZKEngine;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{watch, Mutex},
};
use open_inference_runtime::{TritonClient, client::TensorData};

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
    let (status_tx, status_rx) = watch::channel(EngineStatus::Idle);
    let paths = get_paths()?;
    // let engine = Arc::new(Mutex::new(
    //     NeuroZKEngine::new(PathBuf::from(format!(
    //         "{}/{}",
    //         paths.task_dir_path, paths.task_file_name
    //     )))
    //     .map_err(|e| Error::Custom(format!("Failed to create engine: {}", e.to_string())))?,
    // ));
     let engine = match task.task_type {
        TaskType::OpenInference => {
            let triton_client = TritonClient::new("http://localhost:8000/v2",&paths.task_file_name,PathBuf::from(&paths.task_dir_path))
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

    
    {
        let engine = engine.clone();
        let status_tx = status_tx.clone();

        tokio::spawn(async move {
            let _ = status_tx.send(EngineStatus::Initializing);

            match &engine {
                InferenceEngine::OpenInference(client) => {
                    let _ = status_tx.send(EngineStatus::Ready);
                }
                InferenceEngine::NeuroZk(engine) => {
                    match engine.lock().await.setup().await {
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
    }

    

    let state = AppState {
        task: task.clone(),
        engine: engine,
        status: Arc::new(status_rx),
    };

    let mut default_port: u16 = 3000;

    if let Some(port) = port {
        default_port = port
    }

    let app = Router::new()
        .route(&format!("/inference/{}", &task.id), get(ws_handler))
        .with_state(state);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", default_port)).await?;

    println!("listening on {}", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        println!("Starting inference server...");
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            shutdown_rx.changed().await.ok();
            println!("Shutdown signal received, stopping inference server!");
        })
        .await
        .expect("Server failed to start...");
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
                        tracing::error!("Error running Nvidia Inference: {}",e);
                    }
                }
                InferenceEngine::NeuroZk(engine) => {
                    let engine = engine.lock().await;
                    if let Err(e) = engine.run(request_stream, response_stream).await {
                        tracing::error!("Error running NeuroZK inference: {}", e);
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