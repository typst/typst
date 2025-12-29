#!/bin/bash
# Quick setup script for Self-Hosted Typst AI
# Run: chmod +x setup.sh && ./setup.sh

set -e

echo "========================================"
echo "  Self-Hosted Typst AI - Quick Setup"
echo "========================================"
echo ""

# Check prerequisites
check_command() {
    if ! command -v $1 &> /dev/null; then
        echo "❌ $1 is required but not installed."
        return 1
    else
        echo "✓ $1 found"
        return 0
    fi
}

echo "Checking prerequisites..."
check_command docker || exit 1
check_command docker-compose || docker compose version > /dev/null 2>&1 || { echo "❌ docker-compose not found"; exit 1; }
echo "✓ docker-compose found"
echo ""

# Check for NVIDIA GPU
HAS_GPU=false
if command -v nvidia-smi &> /dev/null; then
    if nvidia-smi > /dev/null 2>&1; then
        echo "✓ NVIDIA GPU detected"
        HAS_GPU=true
    fi
else
    echo "ℹ No NVIDIA GPU detected (will use CPU-only mode)"
fi
echo ""

# Create .env if not exists
if [ ! -f .env ]; then
    echo "Creating .env file..."
    read -p "Enter editor password (default: typst123): " PASSWORD
    PASSWORD=${PASSWORD:-typst123}
    echo "EDITOR_PASSWORD=$PASSWORD" > .env
    echo "TZ=UTC" >> .env
    echo "✓ .env created"
else
    echo "✓ .env already exists"
fi
echo ""

# Make scripts executable
chmod +x scripts/*.sh

# Create directories
mkdir -p projects config/code-server config/vscode

# Choose compose file
if [ "$HAS_GPU" = true ]; then
    COMPOSE_FILE="docker-compose.yml"
    echo "Using GPU-enabled configuration..."
else
    COMPOSE_FILE="docker-compose.cpu.yml"
    echo "Using CPU-only configuration..."
fi
echo ""

# Build and start services
echo "Building and starting services..."
echo "(This may take 5-10 minutes on first run)"
echo ""

docker-compose -f $COMPOSE_FILE build
docker-compose -f $COMPOSE_FILE up -d

echo ""
echo "Waiting for services to start..."
sleep 10

# Setup Ollama models
echo ""
echo "Setting up AI models..."
./scripts/setup-ollama-models.sh

echo ""
echo "========================================"
echo "  Setup Complete!"
echo "========================================"
echo ""
echo "Access your editor at: http://localhost:8080"
echo "Password: $(grep EDITOR_PASSWORD .env | cut -d= -f2)"
echo ""
echo "Quick start:"
echo "  1. Open http://localhost:8080 in your browser"
echo "  2. Open the example project: projects/example/main.typ"
echo "  3. Press Ctrl+L to open AI chat"
echo "  4. Try: 'Add a table with columns for name, age, city'"
echo ""
echo "See PROMPTS.md for more example prompts!"
echo ""
echo "Commands:"
echo "  Stop:    docker-compose -f $COMPOSE_FILE down"
echo "  Restart: docker-compose -f $COMPOSE_FILE restart"
echo "  Logs:    docker-compose -f $COMPOSE_FILE logs -f"
echo "========================================"
