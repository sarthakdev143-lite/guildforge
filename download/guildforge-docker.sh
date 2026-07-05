#!/bin/bash
#
# GuildForge — Docker wrapper
#
# Usage:
#   ./guildforge-docker.sh --help
#   ./guildforge-docker.sh validate config.yaml
#   ./guildforge-docker.sh plan config.yaml
#   ./guildforge-docker.sh apply --auto-approve config.yaml
#
# Token:
#   Set GUILDFORGE_BOT_TOKEN env var, or mount a token file.
#
# State:
#   State is persisted in ./guildforge-state/ (mounted volume).

set -e

IMAGE_NAME="guildforge"
IMAGE_TAG="latest"
CONTAINER_NAME="${IMAGE_NAME}:${IMAGE_TAG}"

# Build the image if it doesn't exist
if ! docker image inspect "$CONTAINER_NAME" >/dev/null 2>&1; then
    echo "Building Docker image (first run only)..."
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    docker build -t "$CONTAINER_NAME" -f "$SCRIPT_DIR/Dockerfile" "$SCRIPT_DIR"
fi

# Run the container with:
# - Current directory mounted at /workspace
# - State persisted in ./guildforge-state/
# - Token passed via env var if set
# - Config files accessible from the current directory

mkdir -p ./guildforge-state

docker run --rm -it \
    -v "$(pwd):/workspace" \
    -v "$(pwd)/guildforge-state:/state" \
    -w /workspace \
    -e GUILDFORGE_STATE_FILE=/state/guildforge.db \
    ${GUILDFORGE_BOT_TOKEN:+-e GUILDFORGE_BOT_TOKEN="$GUILDFORGE_BOT_TOKEN"} \
    ${GUILDFORGE_GUILD_ID:+-e GUILDFORGE_GUILD_ID="$GUILDFORGE_GUILD_ID"} \
    "$CONTAINER_NAME" \
    "$@"
