#!/usr/bin/env bash
set -euo pipefail

# Setup for location of the miner - the agent binary is included in the release tarball
REPO="Cyborg-Network/Cyborg-miner"
MINER_ASSET_NAME="cyborg-miner"
PLATFORM="linux"
ARCH="aarch64"
TAG=$(curl -s https://api.github.com/repos/${REPO}/releases/latest | grep -Po '"tag_name": "\K.*?(?=")')
ASSET="${MINER_ASSET_NAME}-${PLATFORM}-${ARCH}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

BIN_DIR="$(dirname "$(realpath "$0")")"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading latest release: $TAG..."
curl -L "$URL" -o "$TMP_DIR/release.tar.gz"
tar -xf "$TMP_DIR/release.tar.gz" -C "$TMP_DIR"

MINER_BIN=$(find "$TMP_DIR" -type f -executable -name '*miner*' | head -n 1)
AGENT_BIN=$(find "$TMP_DIR" -type f -executable -name '*agent*' | head -n 1)

if [[ -z "$MINER_BIN" || -z "$AGENT_BIN" ]]; then
  echo "Required binaries not found in archive."
  exit 1
fi

MINER_BINARY_NAME=$(basename "$MINER_BIN")
AGENT_BINARY_NAME=$(basename "$AGENT_BIN")

# Ports to be opened at the end of the script
MINER_INFERENCE_PORT=3000
AGENT_HTTP_PORT=8080
AGENT_WS_PORT=8081

# Service files
AGENT_SERVICE_FILE="/etc/systemd/system/$AGENT_BINARY_NAME.service"
MINER_SERVICE_FILE="/etc/systemd/system/$MINER_BINARY_NAME.service"

# Paths for the binaries
MINER_BINARY_PATH="/usr/local/bin/$MINER_BINARY_NAME"
AGENT_BINARY_PATH="/usr/local/bin/$AGENT_BINARY_NAME"

chmod +x "$MINER_BIN"
chmod +x "$AGENT_BIN"

echo "Moving the miner to /usr/local/bin..."
echo "Moving the agent to /usr/local/bin..."

sudo mv "$MINER_BIN" "$MINER_BINARY_PATH"
sudo mv "$AGENT_BIN" "$AGENT_BINARY_PATH"

echo "Initiating miner registration..."

if [[ -z "${PARACHAIN_URL:-}" ]]; then
  read -p "Please provide an endpoint to the parachain that the worker will be registered on: " PARACHAIN_URL
fi

if [[ -z "${ACCOUNT_SEED:-}" ]]; then
  read -p "Please enter the seed phrase of the account that will be managing the worker node: " ACCOUNT_SEED
fi

if ! id "cyborg" &>/dev/null; then
    sudo useradd -r -s /bin/false cyborg
fi

# Create task directory
sudo mkdir -p /var/lib/cyborg/miner/task

# Create config directories
sudo mkdir -p /etc/cyborg/miner

# Create log directories
sudo mkdir -p /var/log/cyborg/miner
sudo mkdir -p /var/log/cyborg/agent

# Set ownership and permissions
sudo chown -R cyborg:cyborg /var/lib/cyborg /var/log/cyborg /etc/cyborg
sudo chmod -R 700 /var/lib/cyborg /var/log/cyborg /etc/cyborg

sudo "$MINER_BINARY_PATH" registration --parachain-url "$PARACHAIN_URL" --account-seed "$ACCOUNT_SEED"

echo "Creating systemd service for worker node: $MINER_SERVICE_FILE"
sudo bash -c "cat > $MINER_SERVICE_FILE" << EOL
[Unit]
Description=Service running the cyborg-miner.
After=network.target

[Service]
User=cyborg
Group=cyborg
Environment=PARACHAIN_URL=$PARACHAIN_URL
Environment="ACCOUNT_SEED=\"$ACCOUNT_SEED\""
ExecStart=$MINER_BINARY_PATH startmining --parachain-url \$PARACHAIN_URL --account-seed "\$ACCOUNT_SEED"
Restart=always
SuccessExitStatus=75
RestartSec=3

[Install]
WantedBy=multi-user.target
EOL

echo "Worker node service created successfully!"

echo "Creating systemd service for agent: $AGENT_SERVICE_FILE"
sudo bash -c "cat > $AGENT_SERVICE_FILE" << EOL
[Unit]
Description=Agent that is able to check the health of the miner, provide required info to the cyborg-parachain, and stream usage metrics and logs of the cyborg node.
After=network.target

[Service]
User=cyborg
Group=cyborg
ExecStart=$AGENT_BINARY_PATH run
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOL

echo "Agent service created successfully!"

echo "Reloading systemd, enabling and starting $MINER_BINARY_NAME and $AGENT_BINARY_NAME services..."
sudo systemctl daemon-reload
sudo systemctl enable "$MINER_BINARY_NAME"
sudo systemctl enable "$AGENT_BINARY_NAME"
sudo systemctl start "$MINER_BINARY_NAME"
sudo systemctl start "$AGENT_BINARY_NAME"

sudo systemctl status "$MINER_BINARY_NAME" --no-pager
sudo systemctl status "$AGENT_BINARY_NAME" --no-pager

echo "Cyborg Worker Node and Agent are installed and running. Binaries are located at $MINER_BINARY_PATH and $AGENT_BINARY_PATH. Now attempting to open Port $AGENT_HTTP_PORT, $AGENT_WS_PORT and $MINER_INFERENCE_PORT to enable communication with Cyborg Connect and provide an inference endpoint."

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

CRON_JOB="0 * * * * /usr/local/bin/cyborg-miner-updater.sh >> /var/log/cyborg/miner/updater.log 2>&1"

( sudo crontab -l 2>/dev/null | grep -v 'cyborg-miner-updater.sh'; echo "$CRON_JOB" ) | sudo crontab -
