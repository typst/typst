# Text Wrap Around Floating Figures: Implementation Overview

## Executive Summary

This project enables **floating figures with text wrap** in Typst using a principled model:
wrap-floats are first-class flow items, line breaking supports variable widths
with Knuth-Plass, and paragraphs are measured then committed to avoid height
estimation errors.

## Key Architectural Changes

1. **Distinct Wrap-Float Kind**: Introduce a dedicated float kind (`wrap-float`) that
   shares placement rules with normal floats but is handled as a flow item for
   exclusion computation.

2. **Two-Phase Paragraph Layout**: Paragraphs are measured (line breaks + metrics)
   then committed (frames). Measurements depend on the active exclusion map.

3. **Variable-Width Knuth-Plass**: Extend the optimized breaker to support a width
   function per line, preserving quality while enabling wrap.

## Pipeline Changes

**Current flow:**
```
collect.rs: ParElem → layout_par() → LineChild (frames only)
distribute.rs: Position LineChild frames
```

**New flow:**
```
collect.rs: ParElem → ParChild (stores element, styles, locator)
distribute.rs: Flow items (incl. WrapFloat) → ParChild.measure(width(y))
               → ParChild.commit(lines) → LineChild frames, then position
```

## Phase Overview

| Phase | Name | Goal | Key Deliverables |
|-------|------|------|------------------|
| 0 | [Prerequisite Refactoring](WRAP_PHASE_0.md) | Separate Preparation from line breaking | `prepare_par()`, `break_lines()`, `finalize_lines()` |
| 1 | [ParChild Structure](WRAP_PHASE_1.md) | Deferred paragraph layout | `ParChild`, `ParMeasureResult`, measure/commit API |
| 2 | [Exclusion Data Structures](WRAP_PHASE_2.md) | Foundation for wrap geometry | `ParExclusions`, `ExclusionZone`, `WrapFloat` |
| 3 | [Distribution Changes](WRAP_PHASE_3.md) | In-flow wrap-floats | `WrapFloatChild`, `WrapState`, exclusion integration |
| 4 | [Variable-Width Knuth-Plass](WRAP_PHASE_4.md) | Quality line breaking with exclusions | `linebreak_variable_width()`, iterative refinement |
| 5 | [Edge Cases and Polish](WRAP_PHASE_5.md) | Production-ready wrap-floats | Page breaks, columns, footnotes, RTL |
| 6 | [Documentation and Release](WRAP_PHASE_6.md) | Ship wrap-floats | User docs, examples, benchmarks |

## Files to Modify

| File | Changes |
|------|---------|
| `crates/typst-layout/src/inline/mod.rs` | Add `prepare_par`, `break_lines`, `measure_lines`, `finalize_lines`, `measure_par_with_exclusions`, `commit_par` |
| `crates/typst-layout/src/inline/linebreak.rs` | Add `linebreak_variable_width`, `linebreak_with_exclusions`, `linebreak_variable`, modify K-P for variable widths |
| `crates/typst-layout/src/flow/collect.rs` | Add `ParChild`, `WrapFloatChild`, `ParMeasureResult`, `ParCommitResult`, update `Child` enum, modify `par()`, `place()` |
| `crates/typst-layout/src/flow/distribute.rs` | Add `WrapState`, `Item::WrapFloat`, `Distributor::par()`, `Distributor::wrap_float()`, modify `finalize()` |
| `crates/typst-library/src/layout/regions.rs` | Add `ParExclusions`, `ExclusionZone`, `WrapFloat` |
| `crates/typst-library/src/layout/place.rs` | Add `wrap` parameter to `PlaceElem` |

## Known Limitations (V1)

### Content That Does NOT Wrap
- Tables, block math, code blocks, lists, block figures
- Only **paragraphs** (`ParElem`) support text wrapping

### Unsupported Configurations
- Center-aligned wrap-floats (experimental)
- Overlapping wrap-floats with complex shapes
- Nested wrap-floats
- Wrap-floats in headers/footers

### Performance Limitations
- Very long paragraphs (>10k chars): Fall back to simple line breaking
- Many wrap-floats (>5 per page): May cause noticeable slowdown

## Risk Summary

See [WRAP_RISK_REVIEW.md](WRAP_RISK_REVIEW.md) for detailed risk analysis.

**Critical Risks:**
1. Locator stability across measure/commit
2. K-P convergence with variable widths
3. Performance regression from two-phase layout
4. Backward compatibility with existing floats

## Glossary

- **Wrap-float**: A floating element with `wrap: true` that text flows around, rather than displacing content vertically like a normal float.

- **Exclusion zone**: A rectangular region where text cannot be placed. Defined by y-range and left/right margins in paragraph-relative coordinates.

- **Inner flow origin**: The coordinate system origin (y=0) at the top of the content region, below any top insertions (floats, headers). All wrap-float positions are relative to this origin.

- **Measure phase**: Computing line breaks and metrics without creating frames. Returns `ParMeasureResult` with heights, widths, and break positions. Allows layout decisions before committing to final output.

- **Commit phase**: Creating actual frames from a prior measurement. Uses stored break positions to reconstruct identical lines, then runs `finalize()` to produce frames.

- **Refinement iteration**: Re-measuring a paragraph when exclusions change due to updated line height estimates. The measure→exclusions→re-measure loop converges when break positions stabilize (typically 1-3 iterations).

- **Variable-width line breaking**: Knuth-Plass algorithm modified to accept different target widths for each line, enabling text to wrap around irregular exclusion shapes.

## Related Documents

- [WRAP_INTEGRATION_NOTES.md](WRAP_INTEGRATION_NOTES.md) - **Critical:** Contracts between phases, data flow, invariants
- [WRAP_RISK_REVIEW.md](WRAP_RISK_REVIEW.md) - Adversarial risk analysis
- [ARCHITECTURE_LAYOUT_OVERVIEW.md](ARCHITECTURE_LAYOUT_OVERVIEW.md) - Background on Typst layout
- [OLD_WRAP_IMPLEMENTATION_PLAN.md](OLD_WRAP_IMPLEMENTATION_PLAN.md) - Original detailed implementation plan (archived)
