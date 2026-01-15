# Phase 6: Documentation and Release

**Goal:** Ship wrap-floats with complete documentation and verified performance.

## User Documentation

### Parameter Documentation

**File: `crates/typst-library/src/layout/place.rs`**

*Find with:* `grep -n "pub wrap:" crates/typst-library/src/layout/place.rs`
(This should exist after Phase 2)

```rust
/// Whether text should wrap around this floating element.
///
/// When enabled with `float: true`, paragraphs will have shortened
/// lines adjacent to this element. Only effective when horizontal
/// alignment is `left` or `right`.
///
/// Wrap-floats do not consume vertical space in the flow; text flows
/// around them. This differs from normal floats which reserve space
/// at the top or bottom of the page/column.
///
/// *Note:* Only paragraph text wraps around floats. Block elements
/// like tables, code blocks, and lists flow below the float.
///
/// ```example
/// #set page(height: 200pt, width: 200pt)
/// #place(
///   top + right,
///   float: true,
///   wrap: true,
///   clearance: 10pt,
///   rect(width: 60pt, height: 80pt, fill: aqua),
/// )
/// #lorem(50)
/// ```
#[default(false)]
pub wrap: bool,
```

### Guide Content

Add to docs or release notes:

```markdown
## Text Wrap Around Floats

You can now make text wrap around floating figures:

​```typst
#place(
  top + right,
  float: true,
  wrap: true,
  clearance: 8pt,
  image("figure.png", width: 80pt),
)
#lorem(100)
​```

### Parameters

- `wrap: true` - Enable text wrapping (requires `float: true`)
- `clearance` - Space between the float and wrapped text

### Alignment

- `left` or `start` - Float on left, text wraps on right
- `right` or `end` - Float on right, text wraps on left
- `center` - Experimental, may produce poor results

### Limitations

Only paragraph text wraps. These elements flow below the float:
- Tables
- Code blocks
- Lists
- Block equations
- Other figures

### Tips

- Keep wrap-floats under 50% of page width for best results
- Use `clearance` to add breathing room around the float
- Combine with `scope: "column"` for multi-column layouts
```

## Examples

**File: `docs/examples/wrap-float.typ`** (or inline in docs)

```typst
// Basic wrap-float
#set page(height: 250pt, width: 300pt)

#place(
  top + right,
  float: true,
  wrap: true,
  clearance: 12pt,
  rect(
    width: 80pt,
    height: 100pt,
    fill: gradient.linear(aqua, blue),
    radius: 4pt,
  ),
)

= Document Title

#lorem(80)

// Multiple wrap-floats
#pagebreak()

#place(top + left, float: true, wrap: true,
  image("photo1.png", width: 70pt))

#place(top + right, float: true, wrap: true, dy: 40pt,
  image("photo2.png", width: 70pt))

#lorem(100)
```

## Performance Benchmarks

### Benchmark Documents

**`benches/wrap-float-simple.typ`:**
```typst
#set page(height: 800pt, width: 600pt)
#place(top + right, float: true, wrap: true,
  rect(width: 150pt, height: 200pt))
#lorem(1000)
```

**`benches/wrap-float-complex.typ`:**
```typst
#set page(height: 800pt, width: 600pt)
#for i in range(3) {
  place(top + if calc.odd(i) { left } else { right },
    float: true, wrap: true, dy: i * 150pt,
    rect(width: 120pt, height: 100pt))
}
#lorem(2000)
```

**`benches/no-wrap-baseline.typ`:**
```typst
#set page(height: 800pt, width: 600pt)
#lorem(1000)
```

### Acceptance Criteria

| Scenario | Max Regression |
|----------|----------------|
| No wrap-floats (baseline) | 0% (must be identical) |
| Single wrap-float | < 20% |
| Multiple wrap-floats | < 50% |
| Long paragraph with wrap | < 100% |

### Running Benchmarks

```bash
# Run performance comparison
cargo bench --bench layout -- wrap

# Profile wrap-float layout
TYPST_TIMING=1 cargo run -- compile benches/wrap-float-complex.typ
```

## Test Coverage Summary

Ensure all test files pass:

```
tests/suite/layout/inline/prepare-api.typ      # Phase 0
tests/suite/layout/inline/prepare-split.typ    # Phase 0
tests/suite/layout/flow/par-child.typ          # Phase 1
tests/suite/layout/place/wrap-float-basic.typ  # Phase 3
tests/suite/layout/place/wrap-float-edge.typ   # Phase 5
tests/suite/layout/place/wrap-float-stress.typ # Phase 4
```

### Run Full Test Suite

```bash
# All tests
cargo test --package typst-tests

# Wrap-specific tests
cargo test --package typst-tests -- wrap

# Existing float tests (regression check)
cargo test --package typst-tests -- place
```

## Release Checklist

### Code Quality
- [ ] All tests pass
- [ ] No new warnings
- [ ] Documentation complete
- [ ] Examples render correctly

### Performance
- [ ] Benchmark: no-wrap baseline unchanged
- [ ] Benchmark: single wrap-float < 20% regression
- [ ] Benchmark: complex wrap < 50% regression
- [ ] No timeout issues on stress tests

### Backward Compatibility
- [ ] Existing documents unchanged (wrap defaults to false)
- [ ] Existing float tests pass
- [ ] No breaking API changes

### Documentation
- [ ] Parameter docs complete
- [ ] Examples provided
- [ ] Limitations documented
- [ ] Release notes written

## Release Notes Template

```markdown
## New Features

### Text Wrap Around Floats

Floating figures can now have text wrap around them using the new
`wrap` parameter on `place()`:

​```typst
#place(
  top + right,
  float: true,
  wrap: true,
  clearance: 10pt,
  image("figure.png", width: 100pt),
)

#lorem(50)
​```

The `clearance` parameter controls spacing between the float and text.

**Note:** Only paragraph text wraps around floats. Block elements
(tables, code blocks, lists) flow below the float.
```

## Exit Criteria

- [ ] Full test suite passes
- [ ] Documentation complete and reviewed
- [ ] Performance benchmarks pass acceptance criteria
- [ ] Release notes written
- [ ] Feature flag removed (if used)

## Dependencies

- [Phase 5: Edge Cases and Polish](WRAP_PHASE_5.md) must be complete

## Completion

Once all criteria are met, wrap-floats are ready for release.
