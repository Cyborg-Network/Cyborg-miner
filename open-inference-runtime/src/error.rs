use thiserror::Error;
use reqwest::StatusCode;

/// Custom error type for Triton client operations.
#[derive(Error, Debug)]
pub enum TritonError {
    #[error("HTTP request failed with status code: {0}")]
    Http(StatusCode),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Failed to deserialize response: {0}")]
    DeserializeError(#[from] serde_json::Error),

    #[error("Invalid response received: {0}")]
    InvalidResponse(&'static str),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
