# Phase 5: Edge Cases and Polish

**Goal:** Handle all edge cases for production-ready wrap-floats.

## Page Breaks

### Behavior
When a paragraph with wrap exclusions spans a page break:
1. Clear wrap state at the page boundary
2. Continue paragraph on new page without exclusions (unless new wrap-floats exist)
3. Wrap-floats do not migrate across pages

### Implementation
```rust
impl WrapState {
    /// Called when moving to a new region/page.
    fn clear(&mut self) {
        self.floats.clear();
    }
}

// In Distributor, when region changes:
fn advance_region(&mut self) {
    self.wrap_state.clear();
    // ... existing logic
}
```

### Test Cases
```typst
// --- wrap-float-page-break ---
#set page(height: 150pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(100)
// Text should wrap on page 1, flow normally on page 2
```

## Column Layouts

### Behavior
- Column-scoped wrap-floats affect only their column
- Parent-scoped wrap-floats are positioned relative to the page and affect the overlapping column

### Implementation Notes
Column support works automatically due to the existing architecture:

1. **Column-scoped wrap-floats**: Each column gets its own `Distributor` with its own `WrapState`.
   Wrap-floats added to one column's `WrapState` don't affect other columns.

2. **Parent-scoped wrap-floats**: Positioned relative to the full page via `Composer::float()`.
   The float appears in the appropriate column based on its position, and text in that
   column wraps around it.

No special coordinate transformation code was needed because:
- Each column is a separate region with its own distributor
- Parent-scoped floats are handled by the existing float insertion system
- The wrap-float is registered in the correct column's `WrapState` when it's laid out

### Test Cases
```typst
// --- wrap-float-columns ---
// Column-scoped: float positioned within first column, only that column wraps
#set page(width: 240pt, height: 240pt)
#columns(2)[
  #place(top + right, float: true, wrap: true, scope: "column",
    rect(width: 40pt, height: 60pt, fill: aqua))
  #lorem(80)
]

// --- wrap-float-columns-parent ---
// Parent-scoped: float positioned relative to page, right column wraps
#set page(width: 240pt, height: 240pt)
#columns(2)[
  #place(top + right, float: true, wrap: true, scope: "parent",
    rect(width: 50pt, height: 60pt, fill: aqua))
  #lorem(80)
]
```

## Footnotes with Wrap-Floats

### Behavior
- Footnotes reduce available region height
- Wrap exclusions must respect the reduced region
- Footnote markers in wrapped text work normally

### Test Cases
```typst
// --- wrap-float-with-footnote ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))
Text with a footnote#footnote[This is the footnote content.] that wraps.
#lorem(30)

// --- wrap-float-footnote-bottom ---
#set page(height: 200pt, width: 200pt)
#place(bottom + right, float: true, wrap: true,
  rect(width: 60pt, height: 40pt, fill: aqua))
Text near bottom#footnote[Footnote here.] with wrap and footnote.
#lorem(20)
```

## RTL and BiDi Support

### Behavior
- RTL text wraps around floats correctly
- Logical alignments (`start`/`end`) are resolved based on text direction
- Physical alignments (`left`/`right`) work as expected regardless of direction
- Exclusion zones are computed in physical space after alignment resolution

### Implementation Notes
**No `text_dir` parameter is needed in `WrapFloat::from_placed()`.**

The alignment is already resolved to physical space before reaching `from_placed()`:
1. User specifies alignment like `top + start` or `top + left`
2. In `Collector::place()`, alignment is resolved via `.resolve(styles)`
3. This calls `HAlignment::fix(text_dir)` which converts logical to physical:
   - `start` in LTR → `FixedAlignment::Start` (left)
   - `start` in RTL → `FixedAlignment::End` (right)
   - `left` always → `FixedAlignment::Start` (left)
   - `right` always → `FixedAlignment::End` (right)
4. `PlacedChild.align_x` stores the resolved physical alignment
5. `WrapFloat::from_placed()` receives physical alignment, no direction flip needed

```rust
// Current implementation - works correctly for RTL
impl WrapFloat {
    pub fn from_placed(
        frame: &Frame,
        y: Abs,
        align_x: FixedAlignment,  // Already resolved to physical space
        clearance: Abs,
    ) -> Self {
        let width = frame.width() + clearance;
        let (left_margin, right_margin) = match align_x {
            FixedAlignment::Start => (width, Abs::zero()),  // Float on left
            FixedAlignment::End => (Abs::zero(), width),    // Float on right
            FixedAlignment::Center => (width / 2.0, width / 2.0),
        };
        Self { y, height: frame.height(), left_margin, right_margin }
    }
}
```

### Test Cases
```typst
// --- wrap-float-rtl ---
// Physical left alignment with RTL text - float stays on visual left
#set page(height: 200pt, width: 200pt)
#set text(dir: rtl)
#place(top + left, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(60)

// --- wrap-float-rtl-start ---
// Logical start alignment with RTL text - float appears on visual RIGHT
// because "start" in RTL means the right side
#set page(height: 200pt, width: 200pt)
#set text(dir: rtl)
#place(top + start, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: forest))
#lorem(60)
```

## User-Visible Warnings

### Warning Conditions
1. Wrap-float too wide (> 2/3 of region width, i.e., 66.7%) - falls back to regular float
2. Wrap-float gap too narrow (< 1/6 of region width, i.e., ~16.7%) - warns but proceeds
3. Text overflows wrap-float gap (content wider than available space) - warns at paragraph level
4. Wrap layout did not converge (after 3 iterations)
5. Wrap layout oscillating (detected via break pattern history)

### Implementation
```rust
// Too wide - falls back to regular float
self.composer.engine.sink.warn(warning!(
    placed.span(),
    "wrap-float too wide ({:.1}pt > {:.1}pt limit); treating as regular float",
    frame.width().to_pt(), max_wrap_width.to_pt()
));

// Gap too narrow - warns about problematic layout
self.composer.engine.sink.warn(warning!(
    placed.span(),
    "wrap-float leaves too little room for text ({:.1}pt gap < {:.1}pt minimum)",
    gap.to_pt(), min_gap.to_pt()
));

// Text overflow - detected during line breaking
self.composer.engine.sink.warn(warning!(
    par.elem.span(),
    "text overflows wrap-float gap; consider reducing float size or clearance"
));

// Non-convergence
self.composer.engine.sink.warn(warning!(
    par.elem.span(),
    "wrap layout did not converge after {} iterations; output may be suboptimal",
    MAX_WRAP_ITER
));

// Unsupported context
self.composer.engine.sink.warn(warning!(
    wf.elem.span(),
    "wrap-floats in headers/footers are not supported; treating as normal float"
));
```

## Edge Case Tests

Edge case tests are in `tests/suite/layout/flow/wrap-float-adversarial.typ` alongside other
wrap-float tests (following the existing test organization pattern).

Key edge case tests:
- `wrap-float-too-wide` - Falls back to regular float with warning
- `wrap-float-narrow-gap` - Warns when gap < 1/6 of width
- `wrap-float-single-word` - Warns when text content overflows gap
- `wrap-float-zero-height` - Degenerate exclusion zone, doesn't crash
- `wrap-float-negative-clearance` - Negative values clamped to zero
- `wrap-float-no-clearance` - Zero clearance, text adjacent to float
- `wrap-float-empty-paragraph-adjacent` - Empty paragraph doesn't crash
- `wrap-float-very-short-paragraph` - Single-line paragraph handling

## Exit Criteria

- [x] Page breaks work correctly with wrap-floats (test: wrap-float-page-break)
- [x] Column layouts work correctly (tests: wrap-float-columns, wrap-float-columns-parent)
- [x] Footnotes work correctly with wrap-floats (test: wrap-float-with-footnote)
- [x] RTL/BiDi text wraps correctly (tests: wrap-float-rtl, wrap-float-rtl-start)
- [x] Too-wide and non-convergence warnings are emitted
- [x] Edge case tests added (wrap-float-zero-height, wrap-float-negative-clearance)

## Dependencies

- [Phase 4: Variable-Width Knuth-Plass](WRAP_PHASE_4.md) must be complete

## Next Phase

[Phase 6: Documentation and Release](WRAP_PHASE_6.md)
