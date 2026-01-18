# Wrap-Float Performance Benchmarks

Benchmarks to measure the performance impact of wrap-floats on document compilation.

## Running Benchmarks

```bash
# Build release binary first
cargo build --release -p typst-cli

# Run benchmarks
./benches/run-benchmarks.sh
```

## Benchmark Files

| File | Description |
|------|-------------|
| `wrap-float-baseline.typ` | No wrap-floats (baseline for comparison) |
| `wrap-float-simple.typ` | Single wrap-float with moderate content |
| `wrap-float-complex.typ` | Multiple (3) wrap-floats at different positions |
| `wrap-float-long-para.typ` | Single long paragraph wrapping around a float |

## Performance Targets

| Scenario | Target | Rationale |
|----------|--------|-----------|
| Baseline | 0% | Must not regress documents without wrap-floats |
| Simple (1 float) | <20% | Typical use case |
| Complex (3 floats) | <50% | Heavy use case |
| Long paragraph | <100% | Stress test for variable-width K-P algorithm |

## Sample Results

```
Scenario                         Time (s)   Regression     Target
--------                         --------   ----------     ------
Baseline (no floats)                0.102         0.0%         0%
Simple (1 float)                    0.102         0.0%       <20%
Complex (3 floats)                  0.109         6.9%       <50%
Long paragraph                      0.226       121.6%      <100%
```

## Notes

- **Simple and complex cases perform well** - 0% and ~7% regression respectively
- **Long paragraph exceeds target** - This is a pathological case where a single
  paragraph spans multiple line-width zones. Real documents typically have shorter
  paragraphs that don't stress the variable-width algorithm as much.
- The long paragraph benchmark may trigger overflow warnings, which is expected
  for this stress test scenario.

## Performance Characteristics

The wrap-float feature uses a variable-width Knuth-Plass algorithm with an
iterative refinement loop:

1. **No wrap-floats**: Uses the standard fast path (no regression)
2. **With wrap-floats**: Uses variable-width line breaking, with up to 3
   iterations to converge on the final layout
3. **Long paragraphs**: Most affected because the K-P algorithm complexity
   scales with paragraph length when variable widths are involved
