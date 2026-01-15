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
- Parent-scoped wrap-floats can affect multiple columns (complex case)

### Implementation
```rust
fn compute_wrap_float_y(&self, wf: &WrapFloatChild<'_>, ...) -> FlowResult<Abs> {
    match wf.scope {
        PlacementScope::Column => {
            // Y is relative to column's inner flow origin
            self.compute_column_relative_y(wf)
        }
        PlacementScope::Parent => {
            // Y is relative to page's inner flow origin
            // Must transform to column coordinates for exclusions
            self.compute_page_relative_y(wf)
        }
    }
}
```

### Test Cases
```typst
// --- wrap-float-columns ---
#set page(height: 200pt, width: 300pt, columns: 2)
#place(top + right, float: true, wrap: true, scope: "column",
  rect(width: 40pt, height: 60pt, fill: aqua))
#lorem(80)

// --- wrap-float-columns-parent ---
#set page(height: 200pt, width: 300pt, columns: 2)
#place(top + right, float: true, wrap: true, scope: "parent",
  rect(width: 80pt, height: 60pt, fill: aqua))
#lorem(80)
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
- Left-aligned float in RTL becomes start-aligned (visual right)
- Exclusion zones respect text direction

### Implementation
```rust
impl WrapFloat {
    pub fn from_placed(
        frame: &Frame,
        y: Abs,
        align_x: FixedAlignment,
        clearance: Abs,
        text_dir: Dir,  // Add text direction
    ) -> Self {
        let width = frame.width() + clearance;

        // Flip for RTL
        let (left_margin, right_margin) = match (align_x, text_dir) {
            (FixedAlignment::Start, Dir::LTR) => (width, Abs::zero()),
            (FixedAlignment::Start, Dir::RTL) => (Abs::zero(), width),
            (FixedAlignment::End, Dir::LTR) => (Abs::zero(), width),
            (FixedAlignment::End, Dir::RTL) => (width, Abs::zero()),
            (FixedAlignment::Center, _) => (width / 2.0, width / 2.0),
        };

        Self { y, height: frame.height(), left_margin, right_margin }
    }
}
```

### Test Cases
```typst
// --- wrap-float-rtl ---
#set page(height: 200pt, width: 200pt)
#set text(dir: rtl, lang: "ar")
#place(top + left, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))
هذا نص عربي يلتف حول الشكل العائم. #lorem(30)

// --- wrap-float-bidi ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))
English text مع نص عربي mixed together wrapping around float.
#lorem(20)
```

## User-Visible Warnings

### Warning Conditions
1. Wrap-float too wide (> 50% of region width)
2. Wrap layout did not converge
3. Wrap-float in unsupported context (header/footer)

### Implementation
```rust
// Too wide
self.composer.engine.sink.warn(warning!(
    wf.elem.span(),
    "wrap-float is too wide ({:.1}pt > {:.1}pt max), treating as normal float",
    frame.width().to_pt(), max_width.to_pt()
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

**File: `tests/suite/layout/place/wrap-float-edge.typ`**

```typst
// --- wrap-float-too-wide ---
// Should fall back to normal float with warning
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 120pt, height: 50pt, fill: aqua))
#lorem(30)

// --- wrap-float-narrow-gap ---
// Text forced below when gap too narrow
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 160pt, height: 50pt, fill: aqua))
#lorem(30)

// --- wrap-float-mixed-sizes ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 80pt, fill: aqua))
Normal text #text(size: 20pt)[BIG TEXT] normal #text(size: 8pt)[small] normal.
#lorem(40)

// --- wrap-float-empty-paragraph ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))

#lorem(20)

// --- wrap-float-single-word ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 150pt, height: 60pt, fill: aqua))
Supercalifragilisticexpialidocious

// --- wrap-float-zero-height ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 0pt, fill: aqua))
#lorem(30)

// --- wrap-float-negative-clearance ---
// Should be clamped to zero
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true, clearance: -10pt,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(30)
```

## Exit Criteria

- [ ] Page breaks work correctly with wrap-floats
- [ ] Column layouts work correctly
- [ ] Footnotes work correctly with wrap-floats
- [ ] RTL/BiDi text wraps correctly
- [ ] All warnings are emitted appropriately
- [ ] All edge case tests pass

## Dependencies

- [Phase 4: Variable-Width Knuth-Plass](WRAP_PHASE_4.md) must be complete

## Next Phase

[Phase 6: Documentation and Release](WRAP_PHASE_6.md)
