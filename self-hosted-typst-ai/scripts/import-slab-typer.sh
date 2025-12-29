#!/bin/bash
# Manual import script for Slab-Typer from HuggingFace GGUF
# Run this if the automatic pull doesn't work

set -e

OLLAMA_HOST="${OLLAMA_HOST:-http://localhost:11434}"
MODEL_DIR="/tmp/slab-typer"

echo "=========================================="
echo "Importing Slab-Typer from HuggingFace"
echo "=========================================="

# Create temp directory
mkdir -p "$MODEL_DIR"
cd "$MODEL_DIR"

# Download the GGUF file from HuggingFace
echo "Downloading Slab-Typer GGUF model..."
echo "This may take a few minutes depending on your connection..."

# Using huggingface-cli or wget
if command -v huggingface-cli &> /dev/null; then
    huggingface-cli download rkstgr/Slab-Typer-1.5B-Base-v2-GGUF \
        --local-dir "$MODEL_DIR" \
        --include "*.gguf"
else
    # Fallback to wget - get the Q4_K_M quantization (good balance)
    wget -c "https://huggingface.co/rkstgr/Slab-Typer-1.5B-Base-v2-GGUF/resolve/main/slab-typer-1.5b-base-v2.Q4_K_M.gguf" \
        -O slab-typer.gguf
fi

# Find the GGUF file
GGUF_FILE=$(find "$MODEL_DIR" -name "*.gguf" | head -1)

if [ -z "$GGUF_FILE" ]; then
    echo "Error: No GGUF file found!"
    exit 1
fi

echo "Found GGUF: $GGUF_FILE"

# Create Modelfile pointing to the GGUF
cat > "$MODEL_DIR/Modelfile" << EOF
FROM $GGUF_FILE

PARAMETER temperature 0.2
PARAMETER top_p 0.9
PARAMETER num_ctx 4096
PARAMETER repeat_penalty 1.1

SYSTEM """You are Slab-Typer, a specialized AI assistant for Typst document markup.

Typst syntax essentials:
- #set page(paper: "a4") - page setup
- #set text(font: "Linux Libertine") - font setup
- = Heading, == Subheading - headings
- *bold*, _italic_ - text formatting
- \$ x^2 + y^2 = z^2 \$ - math
- #table(columns: 3, [A], [B], [C]) - tables
- #figure(image("file.png"), caption: [Caption]) - figures
- #import "@preview/pkg:ver": * - packages

Generate clean, valid Typst code."""

TEMPLATE """{{ if .System }}<|im_start|>system
{{ .System }}<|im_end|>
{{ end }}<|im_start|>user
{{ .Prompt }}<|im_end|>
<|im_start|>assistant
{{ .Response }}<|im_end|>"""
EOF

# Create the model in Ollama
echo ""
echo "Creating model in Ollama..."
cd "$MODEL_DIR"

# Use Ollama CLI if available, otherwise use API
if command -v ollama &> /dev/null; then
    ollama create slab-typer -f Modelfile
else
    # Copy GGUF to Ollama container and create there
    echo "Copying to Ollama container..."
    docker cp "$GGUF_FILE" typst-ollama:/tmp/slab-typer.gguf
    docker cp "$MODEL_DIR/Modelfile" typst-ollama:/tmp/Modelfile

    docker exec typst-ollama ollama create slab-typer -f /tmp/Modelfile
fi

echo ""
echo "=========================================="
echo "Slab-Typer imported successfully!"
echo ""
echo "Test it with:"
echo "  curl $OLLAMA_HOST/api/generate -d '{"
echo "    \"model\": \"slab-typer\","
echo "    \"prompt\": \"Create a simple table with 3 columns\""
echo "  }'"
echo "=========================================="

# Cleanup
rm -rf "$MODEL_DIR"
