# Open Inference Runtime

Open Inference Runtime is a Rust-based library designed to seamlessly integrate with NVIDIA Triton Inference Server. It handles model extraction, server interaction, model management, and inference operations.

## Features:

* **Model Extraction**

  * Supports extraction from `.tar.gz` and `.zip` archives.
  * Automatically detects and extracts model files to the specified directory.

* **Server Health Checks**

  * Check if the Triton server is live and ready.

* **Model Management**

  * Load, unload, and list models in Triton.

* **Model Metadata**

  * Fetch metadata for specified models.

* **Inference Operations**

  * Execute inference with aligned inputs.


## Usage

1. Initialize a `TritonClient` with the server URL.
2. Perform health checks to ensure the server is running.
3. Manage models by loading, unloading, or listing them.
4. Fetch metadata for detailed model information.
5. Execute inference on loaded models with specified inputs.

## Function Descriptions

### Model Extraction

Handles extraction of models from `.tar.gz` or `.zip` archives and deletes the archive after extraction.

### Server Health Checks

Check if the Triton server is live (`is_server_live`) and ready (`is_server_ready`).

### Model Management

Manage models in Triton:

* Load a model with `load_model`.
* Unload a model with `unload_model`.
* List all currently loaded models with `list_models`.

### Model Metadata

Fetch detailed metadata for any loaded model with `get_model_metadata`.

### Inference Operations

Execute inference on a specified model with aligned input tensors using `run_inference`.

