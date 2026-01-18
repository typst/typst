#!/bin/bash
# Wrap-float performance benchmarks
# Measures compilation time for baseline vs wrap-float variants
#
# Usage: ./benches/run-benchmarks.sh
#
# Prerequisites:
#   - Build with: cargo build --release -p typst-cli

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TYPST="${TYPST:-./target/release/typst}"
ITERATIONS="${ITERATIONS:-3}"
TMPDIR=$(mktemp -d)

cleanup() {
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

echo "=== Wrap-Float Performance Benchmarks ==="
echo "Iterations: $ITERATIONS"
echo ""

# Check typst exists
if [ ! -x "$TYPST" ]; then
    echo "Building typst..."
    cargo build --release -p typst-cli 2>/dev/null
fi

run_benchmark() {
    local file="$1"
    local total=0

    for _ in $(seq 1 $ITERATIONS); do
        local start=$(python3 -c 'import time; print(time.time())')
        $TYPST compile "$file" "$TMPDIR/out.pdf" 2>/dev/null
        local end=$(python3 -c 'import time; print(time.time())')
        local elapsed=$(python3 -c "print($end - $start)")
        total=$(python3 -c "print($total + $elapsed)")
    done

    python3 -c "print(f'{$total / $ITERATIONS:.3f}')"
}

echo "Running benchmarks..."
echo ""

baseline=$(run_benchmark "$SCRIPT_DIR/wrap-float-baseline.typ")
simple=$(run_benchmark "$SCRIPT_DIR/wrap-float-simple.typ")
complex=$(run_benchmark "$SCRIPT_DIR/wrap-float-complex.typ")
longpara=$(run_benchmark "$SCRIPT_DIR/wrap-float-long-para.typ")

echo "=== Results ==="
echo ""
printf "%-30s %10s %12s %10s\n" "Scenario" "Time (s)" "Regression" "Target"
printf "%-30s %10s %12s %10s\n" "--------" "--------" "----------" "------"

calc_pct() {
    python3 -c "
baseline=$baseline
val=$1
if baseline > 0:
    pct = ((val - baseline) / baseline) * 100
    print(f'{pct:.1f}%')
else:
    print('N/A')
"
}

printf "%-30s %10s %12s %10s\n" "Baseline (no floats)" "$baseline" "0.0%" "0%"
printf "%-30s %10s %12s %10s\n" "Simple (1 float)" "$simple" "$(calc_pct $simple)" "<20%"
printf "%-30s %10s %12s %10s\n" "Complex (3 floats)" "$complex" "$(calc_pct $complex)" "<50%"
printf "%-30s %10s %12s %10s\n" "Long paragraph" "$longpara" "$(calc_pct $longpara)" "<100%"

echo ""
echo "Note: Long paragraph may show overflow warning (expected for stress test)"
