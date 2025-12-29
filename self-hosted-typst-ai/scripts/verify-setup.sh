#!/bin/bash
# Verification script for Self-Hosted Typst AI
# Run after setup.sh completes

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================"
echo "  Verifying Typst AI Setup"
echo "========================================"
echo ""

PASS=0
FAIL=0

check() {
    local name="$1"
    local cmd="$2"
    local expected="$3"

    printf "%-30s" "$name..."

    if result=$(eval "$cmd" 2>/dev/null); then
        if [ -z "$expected" ] || echo "$result" | grep -q "$expected"; then
            echo -e "${GREEN}✓ PASS${NC}"
            ((PASS++))
            return 0
        fi
    fi

    echo -e "${RED}✗ FAIL${NC}"
    ((FAIL++))
    return 1
}

# Check Docker containers
echo "=== Docker Containers ==="
check "code-server running" "docker ps --filter name=typst-editor --format '{{.Status}}'" "Up"
check "ollama running" "docker ps --filter name=typst-ollama --format '{{.Status}}'" "Up"
check "typst-mcp running" "docker ps --filter name=typst-mcp --format '{{.Status}}'" "Up"
echo ""

# Check services responding
echo "=== Service Health ==="
check "Editor accessible" "curl -s -o /dev/null -w '%{http_code}' http://localhost:8080" "200\|401"
check "Ollama API" "curl -s http://localhost:11434/api/tags" "models"
check "MCP server" "curl -s -o /dev/null -w '%{http_code}' http://localhost:3000" ""
echo ""

# Check Typst in container
echo "=== Typst Compiler ==="
check "Typst installed" "docker exec typst-editor typst --version" "typst"
check "Can compile" "docker exec typst-editor typst compile /home/coder/projects/example/main.typ /tmp/test.pdf && echo ok" "ok"
echo ""

# Check AI models
echo "=== AI Models ==="
check "Models available" "curl -s http://localhost:11434/api/tags | grep -c name" ""

# List available models
echo ""
echo "Available models:"
curl -s http://localhost:11434/api/tags 2>/dev/null | grep -o '"name":"[^"]*"' | cut -d'"' -f4 | while read model; do
    echo "  - $model"
done
echo ""

# Test AI generation
echo "=== AI Generation Test ==="
printf "%-30s" "AI responds..."

AI_RESPONSE=$(curl -s http://localhost:11434/api/generate \
    -d '{
        "model": "llama3.2:3b",
        "prompt": "Write a simple Typst table with 2 columns. Only output the code, nothing else.",
        "stream": false
    }' 2>/dev/null | grep -o '"response":"[^"]*"' | head -1)

if [ -n "$AI_RESPONSE" ]; then
    echo -e "${GREEN}✓ PASS${NC}"
    ((PASS++))
else
    echo -e "${YELLOW}⚠ SKIP (model may still be loading)${NC}"
fi
echo ""

# Summary
echo "========================================"
echo "  Results: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC}"
echo "========================================"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}All checks passed!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Open http://localhost:8080 in your browser"
    echo "  2. Password: $(grep EDITOR_PASSWORD .env 2>/dev/null | cut -d= -f2 || echo 'typst123')"
    echo "  3. Open projects/example/main.typ"
    echo "  4. Press Ctrl+L to chat with AI"
    echo ""
else
    echo -e "${RED}Some checks failed.${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  docker-compose logs code-server"
    echo "  docker-compose logs ollama"
    echo ""
fi
