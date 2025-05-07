use crate::error::Result;
use axum::{
    Router, 
    routing::get,
    serve
};
use tokio::net::TcpListener;
use std::net::SocketAddr;

pub async fn perform_inference(task: &Option<String>, port: Option<u16>) -> Result<()>{
    if let (Some(task), Some(port)) = (task, port) {
        let app = Router::new().route(&format!("/inference/{}", &task), get(ws_handler));

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .await?;

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

async fn ws_handler() {
    println!("ws handler unimplemented");
}