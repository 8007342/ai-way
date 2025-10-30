#!/bin/bash

# Set up logging directory and file
LOG_DIR="./logs"
LOG_FILE="${LOG_DIR}/$(date '+%Y%m%d-%H%M')-ai-way.log"

# Colors for pretty output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Create logs directory if it doesn't exist
mkdir -p "$LOG_DIR"

# Function for pretty printing
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

# Check if docker-compose.yml exists
if [ ! -f "docker-compose.yml" ]; then
    echo -e "${RED}[ERROR]${NC} docker-compose.yml not found in current directory"
    exit 1
fi

# Check if running as root
if [ "$(id -u)" = "0" ]; then
    echo -e "${RED}[ERROR]${NC} This script should not be run as root"
    exit 1
fi

# Pull the latest image with progress
print_status "Pulling latest Open WebUI image..."
docker pull ghcr.io/open-webui/open-webui:main

# Start docker-compose with logging
print_status "Starting ${GREEN}docker-compose..."
print_status "Logs will be saved to: ${RED}$LOG_FILE"

# Run docker-compose and tee output to log file
docker-compose up 2>&1 | tee -a "$LOG_FILE"

print_status "${GREEN}Byeeeee!!"