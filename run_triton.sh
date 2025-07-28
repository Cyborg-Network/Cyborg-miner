#!/bin/bash

set -e

# Need to set the model repository of jetson .
MODEL_REPO="$(pwd)"
echo "[*] Using model repository: $MODEL_REPO"

# Ensure the extract directory exists
if [ ! -d "$MODEL_REPO" ]; then
    echo "[!] Model repository folder '$MODEL_REPO' does not exist. Creating it..."
    mkdir -p "$MODEL_REPO"
    echo "[✓] Created empty model directory."
fi

# Check for Docker
if ! command -v docker &> /dev/null; then
    echo "[!] Docker is not installed. Installing Docker..."
    sudo apt-get update
    sudo apt-get install -y apt-transport-https ca-certificates curl gnupg lsb-release

    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg

    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] \
      https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | \
      sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

    sudo apt-get update
    sudo apt-get install -y docker-ce docker-ce-cli containerd.io
    echo "[✓] Docker installed."
else
    echo "[✓] Docker is already installed."
fi

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "[!] Rust is not installed. Installing Rust..."
    curl https://sh.rustup.rs -sSf | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "[✓] Rust installed."
else
    echo "[✓] Rust is already installed."
fi

# Pull Triton image
echo "[*] Pulling Triton server image..."
sudo docker pull nvcr.io/nvidia/tritonserver:25.06-py3

# Run Triton Inference Server
echo "[🚀] Starting Triton server..."
sudo docker run --rm -p8000:8000 -p8001:8001 -p8002:8002 \
    -v "$MODEL_REPO":/models \
    nvcr.io/nvidia/tritonserver:25.06-py3 \
    tritonserver --model-repository=/models --model-control-mode=explicit
