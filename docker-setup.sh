#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# docker-setup.sh — Build the Mauns Docker image and run a container with
# the current directory mounted as the workspace.
# ---------------------------------------------------------------------------

IMAGE_NAME="ghcr.io/mauns/mauns"
IMAGE_TAG="${MAUNS_IMAGE_TAG:-latest}"
CONTAINER_NAME="mauns-workspace"
WORKSPACE="$(pwd)"
TASK="${MAUNS_TASK:-list files in the current directory}"

# ---------------------------------------------------------------------------
# Validate required environment
# ---------------------------------------------------------------------------

if [ -z "${CLAUDE_API_KEY:-}" ] && [ -z "${OPENAI_API_KEY:-}" ]; then
  echo "[error] At least one of CLAUDE_API_KEY or OPENAI_API_KEY must be set."
  echo "        export CLAUDE_API_KEY=sk-ant-..."
  echo "        export OPENAI_API_KEY=sk-..."
  exit 1
fi

# ---------------------------------------------------------------------------
# Build the image
# ---------------------------------------------------------------------------

echo "[mauns] Building Docker image ${IMAGE_NAME}:${IMAGE_TAG}..."
docker build \
  --tag "${IMAGE_NAME}:${IMAGE_TAG}" \
  --file Dockerfile \
  "$(dirname "$0")"

echo "[mauns] Image built successfully."

# ---------------------------------------------------------------------------
# Remove any existing stopped container with the same name
# ---------------------------------------------------------------------------

if docker container inspect "${CONTAINER_NAME}" &>/dev/null; then
  echo "[mauns] Removing existing container '${CONTAINER_NAME}'..."
  docker rm -f "${CONTAINER_NAME}" >/dev/null
fi

# ---------------------------------------------------------------------------
# Run the container
# ---------------------------------------------------------------------------

echo "[mauns] Starting container with workspace: ${WORKSPACE}"
echo "[mauns] Task: ${TASK}"
echo

docker run \
  --rm \
  --interactive \
  --tty \
  --name "${CONTAINER_NAME}" \
  --volume "${WORKSPACE}:/workspace" \
  --workdir "/workspace" \
  --env "CLAUDE_API_KEY=${CLAUDE_API_KEY:-}" \
  --env "OPENAI_API_KEY=${OPENAI_API_KEY:-}" \
  --env "GITHUB_TOKEN=${GITHUB_TOKEN:-}" \
  --env "MAUNS_PROVIDER=${MAUNS_PROVIDER:-anthropic}" \
  --env "MAUNS_LOG=${MAUNS_LOG:-info}" \
  "${IMAGE_NAME}:${IMAGE_TAG}" \
  run "${TASK}" ${MAUNS_FLAGS:-}
