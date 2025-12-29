#!/bin/bash
set -e

# Ensure directories exist
mkdir -p /home/coder/.continue
mkdir -p /home/coder/.local/share/code-server/User

# Copy Continue config if not exists
if [ ! -f /home/coder/.continue/config.json ]; then
    cp /home/coder/.continue/config.json.default /home/coder/.continue/config.json 2>/dev/null || true
fi

# Wait for Ollama to be ready
echo "Waiting for Ollama service..."
until curl -s http://ollama:11434/api/tags > /dev/null 2>&1; do
    sleep 2
done
echo "Ollama is ready!"

# Start code-server
exec /usr/bin/entrypoint.sh --bind-addr 0.0.0.0:8080 /home/coder/projects
