#!/bin/bash
# Setup script for Ollama models
# Run this after docker-compose up

set -e

OLLAMA_HOST="${OLLAMA_HOST:-http://localhost:11434}"

echo "=========================================="
echo "Setting up Ollama models for Typst AI"
echo "=========================================="

# Wait for Ollama to be ready
echo "Waiting for Ollama..."
until curl -s "$OLLAMA_HOST/api/tags" > /dev/null 2>&1; do
    sleep 2
    echo "  Still waiting..."
done
echo "Ollama is ready!"

# Create Modelfile for Slab-Typer
echo ""
echo "Creating Slab-Typer Modelfile..."

cat > /tmp/Modelfile.slab-typer << 'EOF'
# Slab-Typer: Typst-specialized LLM
# Based on Qwen2.5-Coder with LoRA training for Typst

FROM rkstgr/slab-typer-1.5b-base-v2-gguf:latest

# Set optimal parameters for Typst code generation
PARAMETER temperature 0.2
PARAMETER top_p 0.9
PARAMETER top_k 40
PARAMETER num_ctx 4096
PARAMETER repeat_penalty 1.1
PARAMETER stop "<|endoftext|>"
PARAMETER stop "<|im_end|>"

SYSTEM """You are Slab-Typer, a specialized AI assistant for Typst document markup. Typst is a modern typesetting system that combines the power of LaTeX with a friendlier syntax.

Key Typst syntax rules:
- Use #set for configuration (e.g., #set page(paper: "a4"))
- Use #let for variable definitions
- Use #show for custom styling rules
- Math: $ inline $ or $ display equation $
- Tables: #table(columns: N, [...cells...])
- Figures: #figure(image("path.png"), caption: [...])
- Import packages: #import "@preview/package:version": *
- Headings: = Level 1, == Level 2, etc.
- Bold: *text*, Italic: _text_
- Code: `inline` or ```lang block```

Always generate valid, compilable Typst code."""

TEMPLATE """{{ if .System }}<|im_start|>system
{{ .System }}<|im_end|>
{{ end }}{{ if .Prompt }}<|im_start|>user
{{ .Prompt }}<|im_end|>
{{ end }}<|im_start|>assistant
{{ .Response }}<|im_end|>"""
EOF

# Try to pull or create the model
echo ""
echo "Setting up Slab-Typer model..."

# First, try to pull directly from registry
if curl -s -X POST "$OLLAMA_HOST/api/pull" -d '{"name": "rkstgr/slab-typer"}' | grep -q "success"; then
    echo "Pulled Slab-Typer from registry"
else
    echo "Note: Slab-Typer may need manual GGUF import"
    echo "See: https://huggingface.co/rkstgr/Slab-Typer-1.5B-Base-v2-GGUF"
fi

# Pull fallback models
echo ""
echo "Pulling Llama 3.2 (fallback model)..."
curl -X POST "$OLLAMA_HOST/api/pull" -d '{"name": "llama3.2:3b"}' &

echo ""
echo "Pulling Qwen 2.5 Coder (for complex code)..."
curl -X POST "$OLLAMA_HOST/api/pull" -d '{"name": "qwen2.5-coder:7b"}' &

wait

echo ""
echo "=========================================="
echo "Model setup complete!"
echo ""
echo "Available models:"
curl -s "$OLLAMA_HOST/api/tags" | jq -r '.models[].name'
echo "=========================================="
