#!/usr/bin/env bash
set -euo pipefail

# =================================== SHARED CONFIG ==========================================
REPO="Cyborg-Network/Cyborg-miner"
MINER_ASSET_NAME="cyborg-miner"
PLATFORM="linux"
ARCH="aarch64"
TAG=$(curl -s https://api.github.com/repos/${REPO}/releases/latest | grep -Po '"tag_name": "\K.*?(?=")')
ASSET="${MINER_ASSET_NAME}-${PLATFORM}-${ARCH}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

# File names as they appear after installation
MINER_FILE_NAME="cyborg-miner"
AGENT_FILE_NAME="cyborg-agent"
SETUP_SCRIPT_FILE_NAME="setup.sh"

# Paths for the files
BIN_DIR="/usr/local/bin"
SCRIPT_DIR="/var/lib/cyborg/miner/scripts"

# Full paths
MINER_BINARY_PATH="$BIN_DIR/$MINER_FILE_NAME"
AGENT_BINARY_PATH="$BIN_DIR/$AGENT_FILE_NAME"
SETUP_SCRIPT_PATH="$SCRIPT_DIR/$SETUP_SCRIPT_FILE_NAME"

# Ports to be opened at the end of the script
MINER_INFERENCE_PORT=3000
AGENT_HTTP_PORT=8080
AGENT_WS_PORT=8081

# Service files
MINER_SERVICE_FILE="/etc/systemd/system/$MINER_FILE_NAME.service"
AGENT_SERVICE_FILE="/etc/systemd/system/$AGENT_FILE_NAME.service"

# ENV variables for the miner
MINER_TASK_DIR="/var/lib/cyborg/miner/task"
MINER_CONFIG_DIR="/etc/cyborg/miner"
MINER_LOG_DIR="/var/log/cyborg/miner"
MINER_UPDATE_PATH="/var/lib/cyborg/miner/update/cyborg-miner.new"
STAGE_DIR="/var/lib/cyborg/miner/update"

# User
USER="cyborg-user"

# ======================================= UTIL ===============================================================
download_and_extract() {
    TMP_DIR=$(mktemp -d)
    trap "rm -rf \"$TMP_DIR\"" EXIT

    echo "Downloading latest release: $TAG..."
    curl -L "$URL" -o "$TMP_DIR/release.tar.gz"
    tar -xf "$TMP_DIR/release.tar.gz" -C "$TMP_DIR"
    
    MINER_BIN=$(find "$TMP_DIR" -type f -executable -name '*miner*' | head -n 1)
    AGENT_BIN=$(find "$TMP_DIR" -type f -executable -name '*agent*' | head -n 1)
    SETUP_SCRIPT=$(find "$TMP_DIR" -type f -executable -name '*setup*' | head -n 1)

    if [[ -z "$MINER_BIN" || -z "$AGENT_BIN" || -z "$SETUP_SCRIPT" ]]; then
        echo "Required files not found."
        exit 1
    fi 

    chmod +x "$MINER_BIN" "$AGENT_BIN" "$SETUP_SCRIPT"
}

prepare_environment() {
    echo "Preparing file system structure..."

    for dir in \
        "$BIN_DIR" \
        "$SCRIPT_DIR" \
        "$MINER_TASK_DIR" \
        "$MINER_CONFIG_DIR" \
        "$MINER_LOG_DIR" \
        "$STAGE_DIR" \
        "/var/log/cyborg/agent" \
        "/var/lib/cyborg" \
        "/var/log/cyborg" \
        "/etc/cyborg"
    do
        if [[ ! -d "$dir" ]]; then
            echo "Creating directory: $dir"
            sudo mkdir -p "$dir"
        fi
    done

    if ! id "$USER" &>/dev/null; then
        echo "Creating system user: $USER"
        sudo useradd -r -s /bin/false "$USER"
    fi

    # Set ownership and permissions, only if needed
    echo "Setting ownership and permissions..."
    sudo chown -R "$USER:$USER" /var/lib/cyborg /var/log/cyborg /etc/cyborg "$STAGE_DIR"
    sudo chmod -R 700 /var/lib/cyborg /var/log/cyborg /etc/cyborg "$STAGE_DIR"
}

prepare_triton() {
    echo "[*] Triton model repository directory: $MINER_TASK_DIR"

    # Ensure the extract directory exists
    if [ ! -d "$MINER_TASK_DIR" ]; then
        echo "[!]Triton Model repository folder '$MINER_TASK_DIR' does not exist. Creating it..."
        mkdir -p "$MINER_TASK_DIR"
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

    # Pull Triton image
    echo "[*] Pulling Triton server image..."
    sudo docker pull nvcr.io/nvidia/tritonserver:25.06-py3

    # Run Triton Inference Server
    echo "[🚀] Starting Triton server..."
    sudo docker run --rm -p8000:8000 -p8001:8001 -p8002:8002 \
        -v "$MINER_TASK_DIR":/models \
        nvcr.io/nvidia/tritonserver:25.06-py3 \
        tritonserver --model-repository=/models --model-control-mode=explicit
}

install() {
    echo "Initiating miner registration..."

    download_and_extract
    prepare_environment

    echo "Moving the miner to $BIN_DIR..."
    echo "Moving the agent to $BIN_DIR..."
    echo "Moving the setup script to $SCRIPT_DIR..."

    sudo mv "$MINER_BIN" "$MINER_BINARY_PATH"
    sudo mv "$AGENT_BIN" "$AGENT_BINARY_PATH"
    sudo mv "$SETUP_SCRIPT" "$SETUP_SCRIPT_PATH"

    if [[ -z "${PARACHAIN_URL:-}" ]]; then
    read -p "Please provide an endpoint to the parachain that the worker will be registered on: " PARACHAIN_URL
    fi

    if [[ -z "${ACCOUNT_SEED:-}" ]]; then
    read -p "Please enter the seed phrase of the account that will be managing the worker node: " ACCOUNT_SEED
    fi

    if ! id "$USER" &>/dev/null; then
        sudo useradd -r -s /bin/false "$USER"
    fi

    echo "Creating systemd service for worker node: $MINER_SERVICE_FILE"
    sudo bash -c "cat > $MINER_SERVICE_FILE" << EOL
    [Unit]
    Description=Service running the cyborg-miner.
    After=network.target

    [Service]
    User=$USER
    Group=$USER
    Environment=PARACHAIN_URL=$PARACHAIN_URL
    Environment="ACCOUNT_SEED=\"$ACCOUNT_SEED\""
    Environment=LOG_FILE_PATH=$MINER_LOG_DIR/miner.log
    Environment=TASK_FILE_NAME=archive.tar.zst
    Environment=TASK_DIR_PATH=$MINER_TASK_DIR
    Environment=IDENTITY_FILE_PATH=$MINER_CONFIG_DIR/miner_identity.json
    Environment=TASK_OWNER_FILE_PATH=$MINER_CONFIG_DIR/task_owner.json
    Environment=UPDATE_STAGER_PATH=$MINER_UPDATE_PATH
    ExecStart=$MINER_BINARY_PATH start-miner --parachain-url \$PARACHAIN_URL --account-seed "\$ACCOUNT_SEED"
    Restart=always
    SuccessExitStatus=75
    RestartSec=3

    [Install]
    WantedBy=multi-user.target
EOL

    echo "systemd service for $MINER_FILE_NAME created successfully!"

    echo "Creating systemd service for agent: $AGENT_SERVICE_FILE"
    sudo bash -c "cat > $AGENT_SERVICE_FILE" << EOL
    [Unit]
    Description=Agent that is able to check the health of the miner, provide required info to the cyborg-parachain, and stream usage metrics and logs of the cyborg node.
    After=network.target

    [Service]
    User=$USER
    Group=$USER
    ExecStart=$AGENT_BINARY_PATH run
    Restart=always
    RestartSec=3

    [Install]
    WantedBy=multi-user.target
EOL

    echo "Agent service created successfully!"

    echo "Reloading systemd, enabling and starting $MINER_FILE_NAME and $AGENT_FILE_NAME services..."
    sudo systemctl daemon-reexec
    sudo systemctl daemon-reload
    sudo systemctl enable "$MINER_FILE_NAME"
    sudo systemctl enable "$AGENT_FILE_NAME"
    sudo systemctl restart "$MINER_FILE_NAME"
    sudo systemctl restart "$AGENT_FILE_NAME"

    sudo systemctl status "$MINER_FILE_NAME" --no-pager
    sudo systemctl status "$AGENT_FILE_NAME" --no-pager

    echo "Cyborg Miner and Agent are installed and running. Binaries are located at $MINER_BINARY_PATH and $AGENT_BINARY_PATH. Now attempting to open Port $AGENT_HTTP_PORT, $AGENT_WS_PORT and $MINER_INFERENCE_PORT to enable communication with Cyborg Connect and provide an inference endpoint."

    if command -v ufw &> /dev/null; then
        FIREWALL="ufw"
    elif command -v firewall-cmd &> /dev/null; then
        FIREWALL="firewalld"
    elif command -v iptables &> /dev/null; then
        FIREWALL="iptables"
    else
        echo "Firewall management tool not detected. Please open $AGENT_HTTP_PORT, $AGENT_WS_PORT and $MINER_INFERENCE_PORT manually for the miner to work."
        echo "If in doubt, refer to the documentation of your firewall management tool for instructions."
    fi

    open_ports_ufw() {
        sudo ufw allow $AGENT_WS_PORT
        sudo ufw allow $AGENT_HTTP_PORT
        sudo ufw allow $MINER_INFERENCE_PORT
        echo "Ports opened in UFW."
    }

    # Function to open ports with firewalld
    open_ports_firewalld() {
        sudo firewall-cmd --permanent --add-port=$AGENT_HTTP_PORT/tcp
        sudo firewall-cmd --permanent --add-port=$AGENT_WS_PORT/tcp
        sudo firewall-cmd --permanent --add-port=$MINER_INFERENCE_PORT/tcp
        sudo firewall-cmd --reload
        echo "Ports opened in firewalld."
    }

    # Function to open ports with iptables
    open_ports_iptables() {
        sudo iptables -A INPUT -p tcp --dport $AGENT_HTTP_PORT -j ACCEPT
        sudo iptables -A INPUT -p tcp --dport $AGENT_WS_PORT -j ACCEPT
        sudo iptables -A INPUT -p tcp --dport $MINER_INFERENCE_PORT -j ACCEPT
        # Note: Rules added with iptables are not persistent across reboots unless saved.
        echo "Ports opened in iptables."
    }

    if [[ -n "${FIREWALL:-}" ]]; then
        case $FIREWALL in
            "ufw")
            open_ports_ufw
            ;;
            "firewalld")
            open_ports_firewalld
            ;;
            "iptables")
            open_ports_iptables
            ;;
        esac
    fi

    CRON_JOB="0 * * * * $SETUP_SCRIPT_PATH stage-update >> /var/log/cyborg/miner/updater.log 2>&1"

    echo "Update cron job installed: $CRON_JOB"

    ( sudo crontab -l 2>/dev/null | grep -v "$SETUP_SCRIPT_FILE_NAME"; echo "$CRON_JOB" ) | sudo crontab -
}

stage_update() {
    download_and_extract
    prepare_environment

    if [[ -z "$MINER_BIN" || -z "$AGENT_BIN" || -z "$SETUP_SCRIPT" ]]; then
    echo "Required files not found in archive."
    exit 1
    fi

    MINER_TARGET="$STAGE_DIR/$(basename "$MINER_BIN").new"
    AGENT_TARGET="$STAGE_DIR/$(basename "$AGENT_BIN").new"
    SETUP_TARGET="$STAGE_DIR/$(basename "$SETUP_SCRIPT").new"

    echo "Staging miner to $MINER_TARGET"
    cp "$MINER_BIN" "$MINER_TARGET"

    echo "Staging agent to $AGENT_TARGET"
    cp "$AGENT_BIN" "$AGENT_TARGET"

    echo "Staging update script to $SETUP_TARGET"
    cp "$SETUP_SCRIPT" "$SETUP_TARGET"

    echo "Update files staged to: $STAGE_DIR"
}

# ======================================== DISPATCH ==================================================

case "${1:-install}" in
  install)
    install
    ;;
  stage-update)
    stage_update
    ;;
  *)
    echo "Usage: $0 {install|stage-update}"
    exit 1
    ;;
esac
