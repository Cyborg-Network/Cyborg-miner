FROM rustlang/rust:nightly-slim as builder

WORKDIR /build

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    perl \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release --manifest-path miner/Cargo.toml

FROM ubuntu:24.04

ENV LOG_FILE_PATH=miner/logs/miner.log
ENV TASK_FILE_NAME=archive.tar.gz
ENV TASK_DIR_PATH=miner/current_task
ENV IDENTITY_FILE_PATH=miner/config/miner_identity.json
ENV TASK_OWNER_FILE_PATH=miner/config/task_owner.json

WORKDIR /cyborg-miner

RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/cyborg-miner ./cyborg-miner
COPY cyborg-agent ./cyborg-agent

COPY miner/current_task/archive.tar.gz ./miner/current_task/archive.tar.gz

EXPOSE 8080 8081 3000

CMD ["sh", "-c", "./cyborg-agent run & exec ./cyborg-miner start-miner --parachain-url ws://127.0.0.1:34989 --account-seed //Dave"]

