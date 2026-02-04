#!/bin/bash
# Sam DM Client Installer for Linux
# Usage: curl -sSL https://raw.githubusercontent.com/yhc007/sam-dm/main/dm-client/install.sh | bash

set -e

echo "🦊 Sam DM Client Installer"
echo "=========================="

# Check requirements
command -v cargo >/dev/null 2>&1 || {
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
}

# Create install directory
INSTALL_DIR="/opt/sam-dm"
sudo mkdir -p $INSTALL_DIR
sudo chown $USER:$USER $INSTALL_DIR

# Clone or download
cd /tmp
if [ -d "sam-dm-client" ]; then
    rm -rf sam-dm-client
fi

echo "Downloading dm-client source..."
git clone --depth 1 https://github.com/yhc007/sam-dm.git sam-dm-client 2>/dev/null || {
    echo "Downloading from release..."
    curl -sSL https://api.coreon.build/downloads/dm-client.tar.gz -o dm-client.tar.gz
    tar -xzf dm-client.tar.gz
}

# Build
cd sam-dm-client/dm-client 2>/dev/null || cd dm-client
echo "Building dm-client..."
cargo build --release

# Install binary
sudo cp target/release/dm-client /usr/local/bin/
sudo chmod +x /usr/local/bin/dm-client

# Create config directory
CONFIG_DIR="/etc/sam-dm"
sudo mkdir -p $CONFIG_DIR

# Interactive setup
echo ""
echo "=== Configuration ==="
read -p "DM Server URL [https://api.coreon.build]: " SERVER_URL
SERVER_URL=${SERVER_URL:-https://api.coreon.build}

read -p "API Key: " API_KEY
if [ -z "$API_KEY" ]; then
    echo "Error: API Key is required!"
    exit 1
fi

read -p "Service Directory [/var/www/app]: " SERVICE_DIR
SERVICE_DIR=${SERVICE_DIR:-/var/www/app}

read -p "Restart Command [systemctl restart app]: " RESTART_CMD
RESTART_CMD=${RESTART_CMD:-systemctl restart app}

read -p "Poll Interval (seconds) [60]: " POLL_INTERVAL
POLL_INTERVAL=${POLL_INTERVAL:-60}

# Create config file
sudo tee $CONFIG_DIR/config.env > /dev/null << EOF
# Sam DM Client Configuration
DM_SERVER_URL=$SERVER_URL
DM_API_KEY=$API_KEY
DM_SERVICE_DIR=$SERVICE_DIR
DM_RESTART_COMMAND=$RESTART_CMD
DM_POLL_INTERVAL=$POLL_INTERVAL
DM_BACKUP_DIR=/var/backups/sam-dm
RUST_LOG=info,dm_client=debug
EOF

# Create backup directory
sudo mkdir -p /var/backups/sam-dm

# Create systemd service
sudo tee /etc/systemd/system/sam-dm-client.service > /dev/null << EOF
[Unit]
Description=Sam DM Client
After=network.target

[Service]
Type=simple
EnvironmentFile=/etc/sam-dm/config.env
ExecStart=/usr/local/bin/dm-client
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable sam-dm-client
sudo systemctl start sam-dm-client

echo ""
echo "✅ Sam DM Client installed successfully!"
echo ""
echo "Commands:"
echo "  sudo systemctl status sam-dm-client  # Check status"
echo "  sudo systemctl restart sam-dm-client # Restart"
echo "  sudo journalctl -u sam-dm-client -f  # View logs"
echo ""
echo "Config: /etc/sam-dm/config.env"
