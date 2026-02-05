#!/bin/bash
# Sam DM Server Starter

cd /Volumes/T7/Moltbot/sam-dm/dm-server

# Load environment
export DATABASE_URL="postgres://paulyu@127.0.0.1/sam_dm"
export SERVER_HOST="0.0.0.0"
export SERVER_PORT="3000"
export ARTIFACT_DIR="./artifacts"
export RUST_LOG="info,dm_server=debug"

# Start server
exec ./target/release/dm-server
