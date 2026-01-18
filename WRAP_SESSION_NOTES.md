# Wrap-Float Implementation Session Notes

**Date:** 2026-01-18
**Status:** Phase 5 complete

## Key Architectural Insights

### Alignment Resolution (Critical Understanding)
User-facing alignment (`start`/`end`/`left`/`right`) is resolved to physical space
**before** reaching wrap-float code:

```
User: top + start (with RTL text)
  ↓
Collector::place() calls .resolve(styles)
  ↓
HAlignment::fix(text_dir) converts logical → physical
  - start in RTL → FixedAlignment::End (visual right)
  - left always → FixedAlignment::Start (visual left)
  ↓
PlacedChild.align_x stores PHYSICAL alignment
  ↓
WrapFloat::from_placed() receives physical alignment
  - No text_dir parameter needed
  - Start = left margin, End = right margin
```

This means WRAP_PHASE_5.md's original RTL spec was WRONG (it suggested double-flipping).

### Column Layout Support
Works automatically due to architecture:
- Each column gets its own `Distributor` with its own `WrapState`
- Column-scoped floats: only affect their column (natural isolation)
- Parent-scoped floats: positioned at page level, affect overlapping column

### Warning Thresholds
- Too-wide: > 2/3 (66.7%) of region width, NOT 50%
- Non-convergence: after 3 iterations (MAX_WRAP_ITER)
- Oscillation: detected via break pattern history

## Completed in This Session

1. **typst-on8** (Column layout): Was already implemented, added test
2. **typst-wmy** (RTL/BiDi): Was already implemented correctly, added test + fixed docs
3. **Compiler warning**: Fixed Preparation visibility (pub → pub(super))
4. **WRAP_PHASE_5.md**: Updated to match actual implementation

## Completed Work

### typst-ltt: Edge Case Tests (DONE)
Investigation revealed:
- Tests properly belong in `tests/suite/layout/flow/wrap-float-adversarial.typ` (not `layout/place/`)
- Spec suggested wrong location; updated WRAP_PHASE_5.md to match actual organization
- **Bug found**: Negative clearance was NOT clamped to zero (spec said it should be)
- **Fix applied**: Added `clearance.max(Abs::zero())` in `WrapFloat::from_placed()`
- Added tests: `wrap-float-zero-height`, `wrap-float-negative-clearance`

### Narrow-Gap Warning (NEW)
Added warning when wrap-float gap is too narrow for reasonable text:
- **Threshold**: Gap < 1/6 of base width (~33pt for 200pt page)
- **Location**: `distribute.rs` in `wrap_float()` function
- **Test**: `wrap-float-narrow-gap`

### Text Overflow Warning (NEW)
Added warning when text content overflows the wrap-float gap (e.g., single word too wide):
- **Detection**: During line breaking, tracks when `line.width > available_width`
- **Implementation**: Added `has_overfull` to `VariableWidthResult` and `ParMeasureResult`
- **Location**: `linebreak.rs` detects, `distribute.rs` emits warning
- **Test**: `wrap-float-single-word`

Key insight: The previous attempt compared frame widths (failed - frames are positioned at full
width). The correct approach is to track overfull during line breaking where we have both the
natural line content width and the available width for that line.

### typst-ouz: Phase 5 Epic (DONE)
All exit criteria met.

---

## Handoff Notes for Phase 6

### Test File Locations (IMPORTANT)
The Phase 6 spec lists incorrect test file paths. Actual locations:
- `tests/suite/layout/flow/wrap-float.typ` - Basic tests
- `tests/suite/layout/flow/wrap-float-adversarial.typ` - Edge cases, warnings
- `tests/suite/layout/flow/place.typ` - Contains `place-wrap-float-basic`

**NOT** in `tests/suite/layout/place/` as the spec suggests.

### Warning Thresholds
These are somewhat arbitrary and may need tuning based on user feedback:
- **Too wide**: > 2/3 (66.7%) → falls back to regular float
- **Narrow gap**: < 1/6 (16.7%) → warns but proceeds
- **Overfull tolerance**: 0.5pt (for floating point comparison)

### Architecture Insights Worth Knowing
1. **Alignment resolution happens early** - `HAlignment::fix(text_dir)` converts logical→physical in `Collector::place()`. Wrap-float code receives physical alignment.

2. **Column isolation is automatic** - Each column has its own `Distributor` with its own `WrapState`. No special column handling needed.

3. **Frame width ≠ content width** - Frames returned from `par.commit()` are positioned at full width. To detect overfull, check during line breaking where `line.width` is actual content width.

4. **Iterative refinement** - Wrap-float paragraphs may need up to 3 iterations (MAX_WRAP_ITER) because line breaks depend on exclusions, which depend on paragraph height, which depends on line breaks.

### Known Limitations (Document These)
- Only paragraph text wraps (tables, lists, code blocks flow below)
- Center-aligned wrap-floats are "experimental" - may produce poor results
- No warning for words that are close to but not exceeding the gap

### Performance Considerations
- The iterative refinement loop (up to 3 iterations) is the main cost
- Long paragraphs use divide-and-conquer (MAX_CHUNK_SIZE = 5000) to bound O(n²)
- Consider profiling with real-world documents, not just synthetic tests

### Test Count
41 wrap-float tests currently pass (as of Phase 5 completion).

## Project Tracking Lessons Learned

1. **Beads were created without investigation** - led to tasks for already-done work
2. **Spec docs can be wrong** - WRAP_PHASE_5.md RTL section was incorrect
3. **Tests can pass by accident** - wrap-float-rtl passed but didn't test logical alignment
4. **Always verify implementation matches spec** before closing tasks

## Test Commands
```bash
# Run all wrap-float tests
cargo test --package typst-tests -- wrap-float

# Run specific test
cargo test --package typst-tests -- wrap-float-rtl-start

# Update reference images
cargo test --package typst-tests -- wrap-float-columns --update
```

## Key Files
- `crates/typst-layout/src/flow/distribute.rs` - WrapState, wrap_float(), par()
- `crates/typst-library/src/layout/regions.rs` - WrapFloat, ParExclusions
- `crates/typst-library/src/layout/align.rs` - HAlignment::fix() resolution
- `crates/typst-layout/src/flow/collect.rs` - PlacedChild, alignment resolution
