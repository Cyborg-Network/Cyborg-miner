use crate::config::{self, get_flash_infer_port};
use crate::substrate_interface::api::runtime_types::cyborg_primitives::task::{FlashInferTask, TaskKind};
use crate::{
    config::get_paths,
    error::{Error, Result},
    types::CurrentTask,
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    routing::get, Router
};
use futures::{SinkExt, StreamExt};
use neuro_zk_runtime::NeuroZKEngine;
use flash_infer_runtime::FlashInferEngine;
use once_cell::sync::Lazy;
use tokio::sync::oneshot;
use std::{
    net::SocketAddr, 
    path::{PathBuf, Path}, 
    sync::Arc,
    time::Duration,
};
use tokio::{
    time::timeout,
    net::TcpListener,
    sync::{watch, Mutex},
};
use open_inference_runtime::TritonClient;

#[derive(Clone)]
pub enum InferenceEngine {
    OpenInference(Arc<Mutex<TritonClient>>),
    NeuroZk(Arc<Mutex<NeuroZKEngine>>),
    FlashInference(Arc<Mutex<FlashInferEngine>>),
}

impl InferenceEngine {
    pub async fn kill_engine(&self, task_dir: &str) -> Result<()> {
        match self {
            InferenceEngine::OpenInference(_client) => {
                /* 
                client.lock().await.unload_model(model_name).await.map_err(|e| {
                    Error::Custom(format!("Failed to unload model: {}", e.to_string()))
                })?;
                */

                let dir_path = Path::new(task_dir);

                if dir_path.exists() {
                    std::fs::remove_dir_all(dir_path)?;
                    println!("Task directory {:?} deleted successfully.", dir_path);
                } else {
                    println!("Cannot delete task directory. Directory {:?} does not exist.", dir_path);
                }

                Ok(())
            }
            InferenceEngine::NeuroZk(_engine) => {
                todo!("Implement kill_engine for NeuroZk")
            }
            InferenceEngine::FlashInference(engine) => {
                engine.lock().await.kill_engine().await
                    .map_err(|e| Error::Custom(format!("Failed to kill engine: {}", e.to_string())))?;
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
struct AppState {
    task: CurrentTask,
    engine: InferenceEngine,
    status: Arc<watch::Receiver<EngineStatus>>,
    shutdown: watch::Receiver<bool>,
}

#[derive(Debug, Clone, PartialEq)]
enum EngineStatus {
    Idle,
    Initializing,
    Ready,
    Failed(String),
}

pub struct RunningInferenceServer {
    pub handle: tokio::task::JoinHandle<()>,
    pub shutdown_sender: watch::Sender<bool>,
    pub shutdown_done_rx: oneshot::Receiver<()>,
    pub engine: InferenceEngine,
    pub model_name: String,
}

impl RunningInferenceServer {
    pub async fn shutdown(self, task_dir: &str) -> Result<()> {
        self.engine.kill_engine(task_dir).await?;
        let _ = self.shutdown_sender.send(true);

        match timeout(Duration::from_secs(3), self.shutdown_done_rx).await {
            Ok(Ok(())) => {
                tracing::info!("Server shut down gracefully");
            }
            Ok(Err(_)) => {
                println!("Server shutdown channel closed unexpectedly");
            }
            Err(_) => {
                tracing::warn!("Server did not shut down in time, aborting task!");
                self.handle.abort(); // Force kill the axum server task
            }
        }

        Ok(())
    }
}

pub static CURRENT_SERVER: Lazy<Mutex<Option<RunningInferenceServer>>> = Lazy::new(|| Mutex::new(None));

pub async fn spawn_inference_server(
    task: &CurrentTask,
    port: Option<u16>,
) -> Result</*tokio::task::JoinHandle<()>*/()> {
    tracing::info!("Spawning inference server for task {}", task.id);

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let (shutdown_done_tx, shutdown_done_rx) = oneshot::channel::<()>();

    let (status_tx, status_rx) = watch::channel(EngineStatus::Idle);
    let paths = get_paths()?;
    
    let engine = match &task.task_type {
        TaskKind::OpenInference(_) => {
            let triton_client = TritonClient::new("http://localhost:8000/v2",PathBuf::from(&paths.task_dir_path))
                .await
                .map_err(|e| Error::Custom(format!("Failed to create Triton client: {}", e.to_string())))?;
            InferenceEngine::OpenInference(Arc::new(Mutex::new(triton_client)))
        }
        TaskKind::NeuroZK(_) => {
            let neurozk_engine = NeuroZKEngine::new(PathBuf::from(format!(
                "{}/{}",
                paths.task_dir_path, paths.task_file_name
            )))
            .map_err(|e| Error::Custom(format!("Failed to create engine: {}", e.to_string())))?;
            InferenceEngine::NeuroZk(Arc::new(Mutex::new(neurozk_engine)))
        }
        TaskKind::FlashInferInfer(fi) => {
            match fi {
                FlashInferTask::Huggingface(hf) => {
                    let hf_identifier = String::from_utf8(hf.hf_identifier.0.clone())?;
                    let flash_infer_port = get_flash_infer_port()?;
                    let fi_engine = FlashInferEngine::new(&hf_identifier, *flash_infer_port)
                        .map_err(|e| Error::Custom(format!("Failed to create engine: {}", e.to_string())))?;
                    InferenceEngine::FlashInference(Arc::new(Mutex::new(fi_engine)))
                }
            }
        }
    };
    
    let engine_clone = engine.clone();
    let status_tx = status_tx.clone();

    tokio::spawn(async move {
        let _ = status_tx.send(EngineStatus::Initializing);

        match &engine_clone {
            InferenceEngine::OpenInference(_) => {
                let _ = status_tx.send(EngineStatus::Ready);
            }
            InferenceEngine::NeuroZk(engine_clone) => {
                match engine_clone.lock().await.setup().await {
                    Ok(()) => {
                        let _ = status_tx.send(EngineStatus::Ready);
                    }
                    Err(e) => {
                        println!("Error setting up inference engine: {}", e);
                        let _ = status_tx.send(EngineStatus::Failed(e.to_string()));
                    }
                }
            }
            InferenceEngine::FlashInference(engine_clone) => {
                match engine_clone.lock().await.setup().await {
                    Ok(()) => {
                        let _ = status_tx.send(EngineStatus::Ready);
                    }
                    Err(e) => {
                        println!("Error setting up inference engine: {}", e);
                        let _ = status_tx.send(EngineStatus::Failed(e.to_string()));
                    } 
                }
            }
        }
    });

    let state = AppState {
        task: task.clone(),
        engine: engine.clone(),
        status: Arc::new(status_rx),
        shutdown: shutdown_rx.clone()
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

        tracing::info!("Inference engine ready, miner is reachable at wss://{}.{}/inference{}", hostname, tailnet, route_path);

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

        let _ = shutdown_done_tx.send(());
    });

    *CURRENT_SERVER.lock().await = Some(RunningInferenceServer {
        handle,
        shutdown_sender: shutdown_tx,
        shutdown_done_rx: shutdown_done_rx,
        engine: engine.clone(),
        model_name: "model".to_string().clone(),
    });

    Ok(())
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
    let (mut sender, mut receiver) = socket.split();
    let mut shutdown_rx = state.shutdown.clone();
    let current_status = state.status.borrow().clone();

    if current_status != EngineStatus::Ready {
        let msg = match current_status {
            EngineStatus::Initializing => "Inference engine is initializing.".to_string(),
            EngineStatus::Failed(ref err) => format!("Inference engine failed to run: {}.", err),
            EngineStatus::Idle => "Inference engine is idle.".to_string(),
            EngineStatus::Ready => unreachable!(),
        };
        let _ = sender.send(Message::Text(msg.into())).await;
        return Ok(());
    }

    let sender = Arc::new(Mutex::new(sender));
    let shutdown_sender = Arc::clone(&sender);

    let request_stream = Box::pin(async_stream::stream! {
        loop {
            tokio::select! {
                msg = receiver.next() => {
                    if let Some(Ok(Message::Text(text))) = msg {
                        yield text.to_string();
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("Shutdown signal received, closing websocket");
                        let _ = shutdown_sender.lock().await.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
        }
    });

    let response_stream = {
        let sender = Arc::clone(&sender);
        move |response: String| {
            let sender = Arc::clone(&sender);
            println!("Sending response: {}", response);
            async move {
                let _ = sender.lock().await.send(Message::Text(response.into())).await;
            }
        }
    };

    match &state.engine {
        InferenceEngine::OpenInference(ref client) => {
            if let Err(e) = client.lock().await.run(request_stream, response_stream).await {
                tracing::error!("Error running OpenInference engine: {}", e);
            }
        }
        InferenceEngine::NeuroZk(ref engine) => {
            if let Err(e) = engine.lock().await.run(request_stream, response_stream).await {
                tracing::error!("Error running NeuroZK inference engine: {}", e);
            }
        }
        InferenceEngine::FlashInference(ref engine) => {
            if let Err(e) = engine.lock().await.run(request_stream, response_stream).await {
                tracing::error!("Error running FlashInfer engine: {}", e);
            }
        }
    }

    Ok(())
}
