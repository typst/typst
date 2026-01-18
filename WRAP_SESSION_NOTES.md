# Wrap-Float Implementation Session Notes

**Date:** 2026-01-18
**Status:** Phase 5 nearly complete

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

## Remaining Work

### typst-ltt: Edge Case Test Consolidation
Most edge case tests exist but are scattered across:
- `tests/suite/layout/flow/wrap-float.typ`
- `tests/suite/layout/flow/wrap-float-adversarial.typ`

Task: Consolidate into `tests/suite/layout/place/wrap-float-edge.typ` per spec.
Missing tests that should be added:
- wrap-float-zero-height
- wrap-float-negative-clearance

### typst-ouz: Phase 5 Epic
Can be closed once typst-ltt is complete.

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
