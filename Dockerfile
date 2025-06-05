# ---- Build stage ----
FROM rustlang/rust:nightly-slim as builder

WORKDIR /build

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    perl \
    && rm -rf /var/lib/apt/lists/*

# Copy entire source repo
COPY . .

# Build the miner binary
RUN cargo build --release --manifest-path miner/Cargo.toml

# ---- Final image ----
FROM debian:bookworm-slim

ENV LOG_FILE_PATH=./miner/logs/miner.log
ENV TASK_FILE_NAME=archive.tar.gz
ENV TASK_DIR_PATH=./miner/current_task
ENV IDENTITY_FILE_PATH=./miner/config/miner_identity.json
ENV TASK_OWNER_FILE_PATH=./miner/config/task_owner.json

WORKDIR /cyborg-miner

# Copy binaries
COPY --from=builder /build/target/release/cyborg-miner ./cyborg-miner
COPY cyborg-agent ./cyborg-agent

# Copy the archive
COPY miner/current_task/archive.tar.gz ./current-task/archive.tar.gz

# Copy config (directory only)
COPY miner/config ./config

EXPOSE 8080 8081

CMD ["sh", "-c", "./cyborg-agent & exec ./cyborg-miner start-miner --parachain-url ws://127.0.0.1:34989 --account-seed //Dave"]

