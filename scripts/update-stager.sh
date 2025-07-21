#!/usr/bin/env bash
set -euo pipefail

REPO="Cyborg-Network/Cyborg-miner"
PLATFORM="linux"
ARCH="aarch64"
TAG=$(curl -s https://api.github.com/repos/${REPO}/releases/latest | grep -Po '"tag_name": "\K.*?(?=")')
ASSET="cyborg-miner-${PLATFORM}-${ARCH}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading latest release: $TAG..."
curl -L "$URL" -o "$TMP_DIR/release.tar.gz"
tar -xf "$TMP_DIR/release.tar.gz" -C "$TMP_DIR"

# Locate binaries
MINER_BIN=$(find "$TMP_DIR" -type f -executable -name '*miner*' | head -n 1)
AGENT_BIN=$(find "$TMP_DIR" -type f -executable -name '*agent*' | head -n 1)

if [[ -z "$MINER_BIN" || -z "$AGENT_BIN" ]]; then
  echo "Required binaries not found in archive."
  exit 1
fi

# Replace existing binaries
sudo systemctl stop "$(basename "$MINER_BIN")" || true
sudo systemctl stop "$(basename "$AGENT_BIN")" || true

sudo mv "$MINER_BIN" "/usr/local/bin/$(basename "$MINER_BIN")"
sudo mv "$AGENT_BIN" "/usr/local/bin/$(basename "$AGENT_BIN")"
sudo chmod +x /usr/local/bin/$(basename "$MINER_BIN")
sudo chmod +x /usr/local/bin/$(basename "$AGENT_BIN")

sudo systemctl start "$(basename "$MINER_BIN")"
sudo systemctl start "$(basename "$AGENT_BIN")"

echo "Miner and agent binaries updated and restarted."