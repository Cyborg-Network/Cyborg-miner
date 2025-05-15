## ⚠️ Note
For testing the whole network, please refer to [Local Testing](https://github.com/Cyborg-Network/cyborg-parachain/blob/master/Local%20Testing.md#local-setup)
## Overview
The Cyborg Miner is the one of the Cyborg Worker types, contributing compute resources to Cyborg Network, a decentralized compute platform designed to harness computational resources from distributed nodes around the world. By joining the network, users can either provide computational power to contribute to the network's infrastructure or consume computational resources for task execution.
## Usage 
#### Requirements
- A machine with internet access, running either Ubuntu 22 or 24
- A pre-funded account on the Cyborg Parachain
- A Pinata IPFS API key and secret
#### Installation

##### Method 1: Docker
1. Clone the repository and navigate into the docker directory
```
git clone https://github.com/Cyborg-Network/Cyborg-miner.git
cd Cyborg-miner/docker
```
2. Open the Dockerfile and replace the empty environment variables with the following data
```
ENV PARACHAIN_URL=ws://127.0.0.1:9988
ENV ACCOUNT_SEED="bottom drive obey lake curtain smoke basket hold race lonely fit walk//Alice"
ENV CYBORG_MINER_IPFS_API_URL=https://fuchsia-academic-stoat-866.mypinata.cloud
ENV CYBORG_MINER_IPFS_API_KEY=21021fa56da65b48c301
ENV CYBORG_MINER_IPFS_API_SECRET=6df0a896d2c37606f53ae39f02333484be86d429a898e7c38fb8e4f67da16cb2
```
3. Build the Docker image
	Since this step will already register the worker, the zombienet parachain testnet will need to be running at this point. We are using the `--network="host"` flag here to avoid having to open additional ports to the docker container, since the worker will be sending requests to the parachain. If the worker needs to be registered again for some reason after this image has been built (for example because the zombienet parachain testnet was restarted), it can be done via the `Provide Compute` section of Cyborg Connect
```
docker build -t cyborg-miner:local --network="host" .
```
4. Run the docker image
```
docker run <image-id> --network="host"
```
##### Method 2: Via Installation Script
1. Download the installation script from: https://github.com/Cyborg-Network/Cyborg-miner/blob/tom/standalone-worker-subxt/cyborg-miner-installer.sh
2. In the terminal, navigate to the location in which the script was saved, make the script executable and run it with elevated privileges: 
```
   cd $SCRIPT_LOCATION/
   sudo chmod +x cyborg-miner-installer.sh
   sudo bash cyborg-miner-installer.sh
```
3. Follow the instructions of the script. When prompted for the URL of the parachain the Cyborg Miner should connect to, provide the following url to register the worker on the Cyborg testnet: `wss://fraa-dancebox-3131-rpc.a.dancebox.tanssi.network`. Please note that quotation marks are NOT required for the account seed.

The script will perform the same actions that are outlined in Method 2 (except for the fact, that it will not clone the github repository, but download a binary containing the Cyborg Miner).
##### Method 3: Compile From Source
##### Installation Requirements
1. Have the rust toolchain installed
2. Have nightly features enabled
###### Steps to install
1. Clone the Cyborg Miner repository, navigate to it and compile the code: 
```
git clone https://github.com/Cyborg-Network/Cyborg-miner.git`
cd Cyborg-miner
cargo build --release
```
2. Download the Cyborg Agent: 
   https://server.cyborgnetwork.io:8080/assets/cyborg-agent
   The Agent is required for the Cyborg Miner and Cyborg Connect to be able to communicate off-chain data.
3. Make the `cyborg-agent` and `cyborg-miner` executable and move them to  `/usr/local/bin`:
```
chmod +x cyborg-agent
chmod +x cyborg-miner
sudo mv cyborg-agent /usr/local/bin/
sudo mv cyborg-miner /usr/local/bin/
```
4. Create a user that manages the Cyborg executables:
```
sudo useradd -r -s /bin/false cyborg-user
```
5. Create the directories where the files managed by the cyborg executables will exist and give the newly created user the permissions required to manage these directories:
```
sudo mkdir -p /var/lib/cyborg/miner/packages
sudo mkdir -p /var/lib/cyborg/miner/config
sudo mkdir -p /var/lib/cyborg/miner/logs
sudo chown -R cyborg-user:cyborg-user /var/lib/cyborg
sudo chmod -R 700 /var/lib/cyborg
```
6. Register the worker (replace the variables with your data):
```
sudo cyborg-miner registration --parachain-url "$PARACHAIN_URL" --account-seed "$ACCOUNT_SEED" --ipfs-url "$IPFS_URL" --ipfs-api-key "$IPFS_KEY" --ipfs-api-secret "$IPFS_SECRET"
```
7. Create a DBus configuration file to set up an internal communication channel between the Cyborg Miner and the Cyborg Agent:
```
sudo bash -c "cat > $WORKER_DBUS_FILE" << EOL
<!DOCTYPE busconfig PUBLIC
 "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>

  <policy context="default">
    <allow own="com.cyborg.CyborgAgent"/>
  </policy>

  <policy context="default">
    <allow send_interface="com.cyborg.AgentZkInterface"/>
  </policy>

</busconfig>
EOL
```
8. Create `systemd` configuration files for the Cyborg Agent and the Cyborg Miner, so that the executables don't need to be handled manually. Note that in the commands the variables in `Enviroment` will need to be replaced, but NOT the others.
```
sudo bash -c "cat > /etc/systemd/system/cyborg-miner.service" << EOL
[Unit]
Description=Binary that will execute compute requests from the cyborg-parachain.
After=network.target

[Service]
User=cyborg-user
Group=cyborg-user
Environment=PARACHAIN_URL=$PARACHAIN_URL
Environment="ACCOUNT_SEED=\"$ACCOUNT_SEED\""
Environment="CYBORG_MINER_IPFS_API_URL=$IPFS_URL"
Environment="CYBORG_MINER_IPFS_API_KEY=$IPFS_KEY"
Environment="CYBORG_MINER_IPFS_API_SECRET=$IPFS_SECRET"
ExecStart=$WORKER_BINARY_PATH startmining --parachain-url \$PARACHAIN_URL --account-seed "\$ACCOUNT_SEED"
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOL



sudo bash -c "cat > /etc/systemd/system/cyborg-agent.service" << EOL
[Unit]
Description=Agent that is able to check the health of the node, provide reuired info to the cyborg-parachain, and stream usage metrics and logs of the cyborg node.
After=network.target

[Service]
User=cyborg-user
Group=cyborg-user
ExecStart=$AGENT_BINARY_PATH run
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOL
```
9. Reload the systems daemons, and start the newly created `systemd` services. This will prompt the Cyborg Miner to start listening for tasks:
```
sudo systemctl daemon-reload
sudo systemctl enable cyborg-miner
sudo systemctl enable cyborg-agent
sudo systemctl start cyborg-miner
sudo systemctl start cyborg-agent
```
10. Open ports `8080` and `8081` in your firewall. The commands for this depend on your firewall, so if there is uncertainty please consult the documentation of your firewall. These ports need to be open because the agent needs to:
	1. Communicate with Cyborg Connect to send usage data and logs
	2. Respond to requests from the Cyborg Oracle Feeder which will occasionally query the Cyborg Agent to provide an information on the health and uptime of the Cyborg Miner

Congratulations, your machine is now a Cyborg Miner! It will listen to the Cyborg Parachain, execute tasks that were assigned to it and verify the results of other Nodes.

## Testing
##### Requirements
1. Have the rust toolchain installed
2. Have nightly features enabled
##### Steps to Test
1. Clone the Cyborg Miner repository, navigate to it and run the unit tests: 
```
git clone https://github.com/Cyborg-Network/Cyborg-miner.git`
cd Cyborg-miner
cargo test
```

