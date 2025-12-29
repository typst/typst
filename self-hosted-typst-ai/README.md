# Self-Hosted Typst AI Document System

A fully self-hosted, privacy-focused document creation system that lets you describe documents in natural language and have AI generate Typst markup for you. Think "Cursor for documents."

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Your Linux Server                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │  code-server    │  │    Ollama       │  │  Typst MCP   │ │
│  │  (Web VS Code)  │  │  + Slab-Typer   │  │   Server     │ │
│  │  + Tinymist     │  │  + Llama 3.2    │  │  (optional)  │ │
│  │  + Continue.dev │  │                 │  │              │ │
│  │  Port: 8080     │  │  Port: 11434    │  │  Port: 3000  │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Docker & Docker Compose
- 8GB+ RAM (16GB recommended for larger models)
- NVIDIA GPU (optional, but recommended for faster inference)

### 1. Clone and Configure

```bash
cd /path/to/this/directory

# Set your editor password
echo "EDITOR_PASSWORD=your-secure-password" > .env

# Make scripts executable
chmod +x scripts/*.sh
```

### 2. Start the Stack

```bash
# Start all services
docker-compose up -d

# Watch logs
docker-compose logs -f
```

### 3. Set Up AI Models

```bash
# Install Slab-Typer and fallback models
./scripts/setup-ollama-models.sh

# If Slab-Typer doesn't pull automatically:
./scripts/import-slab-typer.sh
```

### 4. Access the Editor

Open http://localhost:8080 in your browser.

Password: The one you set in `.env` (default: `typst123`)

## Using the AI Assistant

### Chat Mode (Sidebar)

1. Press `Ctrl+L` to open Continue chat
2. Type natural language descriptions:

```
Create a professional report template with:
- A4 paper size
- Header with page numbers
- Title "Quarterly Report"
- Sections for Executive Summary, Data Analysis, and Conclusions
```

### Inline Completions

1. Type a comment describing what you want:
```typst
// Add a table with employee names, departments, and salaries
```
2. Press `Tab` to accept AI suggestions

### Custom Commands

Use `/` commands in the chat:

| Command | Description |
|---------|-------------|
| `/typst` | Convert description to Typst code |
| `/table` | Generate a table from description |
| `/fix-typst` | Fix syntax errors in selected code |
| `/latex-to-typst` | Convert LaTeX to Typst |
| `/document` | Create a complete document structure |

### Example Prompts

**Tables:**
```
Add a table with columns for Product, Price, and Quantity.
Include 5 rows of sample electronics data.
Use alternating row colors.
```

**Math:**
```
Add the formula for standard deviation with proper mathematical notation
```

**Figures:**
```
Create a figure placeholder with caption "Sales Growth 2024"
```

**Full Documents:**
```
Create a two-column academic paper template with:
- Abstract section
- Introduction
- Methodology
- Results
- Bibliography placeholder
Use the IEEE format
```

## Configuration

### Continue.dev Settings

Edit `config/continue-config.json` to:

- Add/remove AI models
- Customize prompts
- Configure context providers
- Adjust generation parameters

### VS Code Settings

Edit `config/settings.json` for editor preferences.

### Adding More Models

```bash
# Pull additional models into Ollama
docker exec typst-ollama ollama pull codellama:13b
docker exec typst-ollama ollama pull mistral:7b

# List available models
docker exec typst-ollama ollama list
```

## GPU Acceleration

### NVIDIA GPU

The docker-compose.yml includes NVIDIA GPU support. Ensure you have:

1. NVIDIA drivers installed
2. nvidia-container-toolkit installed:
```bash
sudo apt-get install -y nvidia-container-toolkit
sudo systemctl restart docker
```

### CPU-Only Mode

Remove the `deploy.resources` section from `docker-compose.yml` for CPU-only operation.

## Typst Packages

### Installing Packages

Packages are auto-downloaded when used. In your Typst file:

```typst
#import "@preview/tablex:0.0.8": tablex
#import "@preview/cetz:0.2.2": canvas, plot
```

### Popular Packages

| Package | Purpose |
|---------|---------|
| `tablex` | Advanced tables |
| `cetz` | Diagrams and plots |
| `codelst` | Code listings |
| `showybox` | Colored boxes |
| `modern-cv` | Resume templates |

Browse more at: https://typst.app/universe

## Troubleshooting

### Editor won't load
```bash
docker-compose logs code-server
# Check if port 8080 is in use
sudo lsof -i :8080
```

### AI not responding
```bash
# Check Ollama status
curl http://localhost:11434/api/tags

# Check if models are loaded
docker exec typst-ollama ollama list
```

### Typst compilation errors
```bash
# Check Typst version in container
docker exec typst-editor typst --version

# Test compilation
docker exec typst-editor typst compile /home/coder/projects/example/main.typ
```

### Continue.dev not working
1. Reload VS Code: `Ctrl+Shift+P` → "Reload Window"
2. Check Continue logs: `Ctrl+Shift+P` → "Continue: View Logs"

## Production Deployment

### With HTTPS (Recommended)

1. Point your domain's DNS to your server
2. Update `Caddyfile` with your domain
3. Caddy will automatically obtain SSL certificates

### Behind Nginx

If you have existing Nginx, add to your config:

```nginx
location /typst/ {
    proxy_pass http://localhost:8080/;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
}
```

## File Structure

```
self-hosted-typst-ai/
├── docker-compose.yml      # Main orchestration
├── Dockerfile.code-server  # Editor container
├── Dockerfile.typst-mcp    # MCP server container
├── Caddyfile              # Reverse proxy config
├── .env                   # Environment variables
├── config/
│   ├── settings.json      # VS Code settings
│   └── continue-config.json # AI assistant config
├── scripts/
│   ├── entrypoint.sh      # Editor startup
│   ├── setup-ollama-models.sh # Model installation
│   └── import-slab-typer.sh   # Manual model import
└── projects/              # Your Typst documents
    └── example/
        └── main.typ       # Sample document
```

## Resources

- [Typst Documentation](https://typst.app/docs/)
- [Typst Package Universe](https://typst.app/universe)
- [Slab-Typer Model](https://huggingface.co/rkstgr/Slab-Typer-1.5B-Base-v2-GGUF)
- [Continue.dev Docs](https://docs.continue.dev/)
- [Ollama](https://ollama.com/)
- [Typst MCP Server](https://github.com/johannesbrandenburger/typst-mcp)

## License

This setup guide is provided as-is. Individual components have their own licenses:
- Typst: Apache 2.0
- code-server: MIT
- Continue.dev: Apache 2.0
- Ollama: MIT
