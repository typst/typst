# Phase 3: Distribution Changes (In-Flow Wrap Floats)

**Goal:** Integrate wrap-floats into the distribution system so they create exclusion zones that affect paragraph layout.

This phase has three sub-phases:
- **3a:** Add `wrap` parameter with no behavior change
- **3b:** Separate collection path for wrap-floats
- **3c:** Full exclusion integration

---

## Phase 3a: Wrap Parameter

**Goal:** Add `wrap` parameter with no behavior change yet.

### Changes

1. Add `wrap: bool` to `PlaceElem` (done in Phase 2)
2. Parse and store in `PlacedChild`
3. No behavior change yet (wrap=true same as wrap=false)

### Exit Criteria
- [ ] Parameter parses correctly
- [ ] No behavior change for existing documents

---

## Phase 3b: WrapFloatChild Collection

**Goal:** Create separate collection path for wrap-floats.

### WrapFloatChild Type

**File: `crates/typst-layout/src/flow/collect.rs`**

*Find insertion point:* Search for `pub struct PlacedChild` - add `WrapFloatChild` near it.

```rust
/// A wrap-enabled floating element that participates in flow layout.
///
/// Unlike `PlacedChild` which becomes an insertion handled by the composer,
/// `WrapFloatChild` is a first-class flow item. It:
/// - Gets a position during distribution
/// - Creates exclusion zones for subsequent paragraphs
/// - Does NOT consume vertical space (text wraps around it)
#[derive(Debug)]
pub struct WrapFloatChild<'a> {
    /// Horizontal alignment (left/right/center).
    pub align_x: FixedAlignment,
    /// Vertical alignment hint (top/bottom/auto).
    pub align_y: Smart<Option<FixedAlignment>>,
    /// Placement scope (column or parent).
    pub scope: PlacementScope,
    /// Clearance around the float.
    pub clearance: Abs,
    /// Delta offsets (dx/dy).
    pub delta: Axes<Rel<Abs>>,
    /// The place element.
    pub elem: &'a Packed<PlaceElem>,
    /// Styles.
    pub styles: StyleChain<'a>,
    /// Locator.
    pub locator: Locator<'a>,
}
```

### Collection Rule

**File: `crates/typst-layout/src/flow/collect.rs`**

*Find with:* `grep -n "fn place\(" crates/typst-layout/src/flow/collect.rs`

In `Collector::place()`, check for `wrap: true`:

```rust
fn place(&mut self, elem: &'a Packed<PlaceElem>, styles: StyleChain<'a>) -> SourceResult<()> {
    let float = elem.float.get(styles);
    let wrap = elem.wrap.get(styles);

    // ... existing validation ...

    if float && wrap {
        // Wrap-floats become flow items, not insertions
        self.output.push(Child::WrapFloat(self.boxed(WrapFloatChild {
            align_x,
            align_y,
            scope,
            clearance,
            delta,
            elem,
            styles,
            locator: self.locator.next(&elem.span()),
        })));
    } else {
        // Normal placed elements (existing behavior)
        self.output.push(Child::Placed(self.boxed(PlacedChild { ... })));
    }

    Ok(())
}
```

### Exit Criteria
- [ ] Wrap-floats render at correct positions
- [ ] No pagination changes from existing behavior

---

## Phase 3c: WrapState and Exclusion Integration

**Goal:** Make wrap-floats actually affect paragraph layout.

### ⚠️ Critical Issues from Phase 1 Learnings

Before implementing, address these issues discovered during Phase 1:

#### Issue 1: Citation Registry Duplication

The iterative refinement loop calls `par.measure()` up to 3 times. Each call triggers realization → `prepare()` → `register_cite_group()`. This causes duplicate CiteGroup registrations.

**Required fix:** Add deduplication to the citation registry:
```rust
// In bibliography.rs, modify register_cite_group()
pub fn register_cite_group(group: Content) {
    CITE_GROUPS.with(|cell| {
        let mut groups = cell.borrow_mut();
        // Deduplicate by location to handle multiple measure() calls
        if let Some(loc) = group.location() {
            if !groups.iter().any(|g| g.location() == Some(loc)) {
                groups.push(group);
            }
        } else {
            groups.push(group);
        }
    });
}
```

#### Issue 2: ParSpill Loses Exclusion Context

When a paragraph breaks across regions, `par_spill` processes remaining frames in the new region. But those frames were computed with the OLD region's exclusions.

**Required fix:** Re-measure remaining content in the new region:
```rust
// In par_spill(), if exclusions changed between regions:
fn par_spill(&mut self, mut spill: ParSpill) -> FlowResult<()> {
    // Check if this region has different exclusions than when frames were computed
    if spill.had_exclusions && self.wrap_state.floats.is_empty() {
        // Frames were computed with exclusions, but this region has none.
        // The frames have wrong widths. Need to re-layout.
        // Option A: Return error to trigger full re-measure of paragraph
        // Option B: Accept slight visual inconsistency (pragmatic)
        // For Phase 3, we accept Option B with a warning.
    }
    // ... rest of implementation
}
```

**Long-term fix:** Store `ParChild` reference in `ParSpill` to enable re-measurement.

#### Issue 3: Thread Contention

Phase 1 requires `--num-threads 4` due to citation registry contention. With 3x measure calls per paragraph, this may worsen. Document this limitation and consider lock-free registry design for future.

---

### WrapState in Distributor

**File: `crates/typst-layout/src/flow/distribute.rs`**

*Find insertion point:* Search for `struct Distributor` - add `WrapState` nearby or as a field.

```rust
/// State for tracking wrap-float exclusions during distribution.
#[derive(Debug, Default)]
struct WrapState {
    /// Active wrap-floats in region coordinates.
    floats: Vec<WrapFloat>,
}

impl WrapState {
    /// Add a wrap-float to the exclusion map.
    fn add(&mut self, wf: WrapFloat) {
        self.floats.push(wf);
    }

    /// Build exclusions for a paragraph at the given y-position.
    fn exclusions_for(&self, par_y: Abs, par_height_estimate: Abs) -> Option<ParExclusions> {
        if self.floats.is_empty() {
            return None;
        }
        let excl = ParExclusions::from_wrap_floats(par_y, par_height_estimate, &self.floats);
        if excl.is_empty() { None } else { Some(excl) }
    }

    /// Clear wrap-floats that are fully above the region boundary.
    /// Floats that span into the next region are preserved (with adjusted y).
    ///
    /// # Arguments
    /// * `region_height` - Height of the completed region
    fn clear_for_region_break(&mut self, region_height: Abs) {
        self.floats.retain_mut(|wf| {
            let float_bottom = wf.y + wf.height;
            if float_bottom <= region_height {
                // Float fully in previous region, remove it
                false
            } else if wf.y >= region_height {
                // Float fully in next region, adjust y
                wf.y -= region_height;
                true
            } else {
                // Float spans regions, adjust y and clip
                let new_height = float_bottom - region_height;
                wf.y = Abs::zero();
                wf.height = new_height;
                true
            }
        });
    }

    /// Check if any exclusions are active.
    fn has_exclusions(&self) -> bool {
        !self.floats.is_empty()
    }
}
```

### Distributor::par() Implementation

```rust
impl Distributor<'_, '_, '_, '_, '_> {
    /// Processes a paragraph with potential wrap exclusions.
    fn par(&mut self, par: &'b ParChild<'a>) -> FlowResult<()> {
        let current_y = self.current_y();

        // Phase 1: Measure without exclusions to get height estimate
        let initial_measure = par.measure(
            self.composer.engine,
            self.regions.base().into(),
            None,
        )?;

        // Phase 2: Check if we need exclusions
        let exclusions = self.wrap_state.exclusions_for(
            current_y,
            initial_measure.total_height,
        );

        // Phase 3: If exclusions exist, re-measure with them
        let (measure_result, final_exclusions) = if let Some(excl) = exclusions {
            let refined = self.refine_paragraph_measure(par, current_y, &excl)?;
            (refined.0, Some(refined.1))
        } else {
            (initial_measure, None)
        };

        // Phase 4: Check if paragraph fits
        if !self.regions.size.y.fits(measure_result.total_height)
            && self.regions.may_progress()
        {
            return Err(Stop::Finish(false));
        }

        // Phase 5: Commit and emit line frames
        let commit_result = par.commit(
            self.composer.engine,
            &measure_result,
            self.regions.base().into(),
            final_exclusions.as_ref(),
        )?;

        // Phase 6: Emit lines as items
        self.emit_paragraph_lines(par, &commit_result, &measure_result)?;

        Ok(())
    }
}
```

### Iterative Refinement

The circular dependency (line height affects exclusions, exclusions affect line breaks) is resolved by iteration.

**Prerequisites (from Phase 2 API Changes):**
- `ParMeasureResult.break_positions: Vec<usize>` must exist for convergence detection
- `par.measure()` must accept `Option<&ParExclusions>`

**Note on `relayout()`:** Each `par.measure()` call uses `self.locator.relayout()`. This is safe to call multiple times - it produces identical locations each time. This is the intended behavior from Phase 1.

```rust
/// Iterative refinement for paragraphs affected by wrap exclusions.
///
/// NOTE: This calls measure() up to MAX_WRAP_ITER times. Each call:
/// - Uses locator.relayout() (safe, produces same locations)
/// - Triggers realization (citation registry must deduplicate!)
fn refine_paragraph_measure(
    &mut self,
    par: &ParChild<'_>,
    par_y: Abs,
    initial_exclusions: &ParExclusions,
) -> SourceResult<(ParMeasureResult, ParExclusions)> {
    const MAX_WRAP_ITER: usize = 3;

    let mut exclusions = initial_exclusions.clone();
    let mut prev_breaks: Option<Vec<usize>> = None;

    for iteration in 0..MAX_WRAP_ITER {
        let measure = par.measure(
            self.composer.engine,
            self.regions.base().into(),
            Some(&exclusions),
        )?;

        // Check for convergence: same line breaks as previous iteration
        // Requires ParMeasureResult.break_positions field (see Phase 2)
        if let Some(prev) = &prev_breaks {
            if *prev == measure.break_positions {
                return Ok((measure, exclusions));
            }
        }
        prev_breaks = Some(measure.break_positions.clone());

        // Recompute exclusions with actual line heights
        exclusions = self.wrap_state
            .exclusions_for(par_y, measure.total_height)
            .unwrap_or_default();

        // If no exclusions remain, we're done
        if exclusions.is_empty() {
            let final_measure = par.measure(
                self.composer.engine,
                self.regions.base().into(),
                None,
            )?;
            return Ok((final_measure, ParExclusions::default()));
        }
    }

    // Fallback: use last measurement, emit warning
    self.composer.engine.sink.warn(warning!(
        par.elem.span(),
        "wrap layout did not converge after {} iterations",
        MAX_WRAP_ITER
    ));

    let final_measure = par.measure(
        self.composer.engine,
        self.regions.base().into(),
        Some(&exclusions),
    )?;
    Ok((final_measure, exclusions))
}
```

### Distributor::wrap_float() Implementation

```rust
/// Processes a wrap-float.
fn wrap_float(&mut self, wf: &'b WrapFloatChild<'a>) -> FlowResult<()> {
    // Determine base size for layout
    let base = match wf.scope {
        PlacementScope::Column => self.regions.base(),
        PlacementScope::Parent => self.composer.page_base,
    };

    // Layout the float content
    let frame = layout_wrap_float(self.composer.engine, wf, base)?;

    // Validate: reject if too wide
    let max_width = base.x * MAX_WRAP_WIDTH_RATIO;
    if frame.width() > max_width {
        self.composer.engine.sink.warn(warning!(
            wf.elem.span(),
            "wrap-float too wide ({} > {}), treating as normal float",
            frame.width(), max_width
        ));
        // Fall back to normal float behavior
        return self.placed_from_wrap(wf);
    }

    // Compute y-position
    let y = self.compute_wrap_float_y(wf, &frame)?;

    // Create exclusion entry
    let wrap_float = WrapFloat::from_placed(
        &frame,
        y,
        wf.align_x,
        wf.clearance,
    );
    self.wrap_state.add(wrap_float);

    // Store for final rendering (doesn't consume vertical space)
    self.flush_tags();
    self.items.push(Item::WrapFloat(frame, y, wf.align_x, wf.delta));

    Ok(())
}

/// Maximum ratio of page width a wrap-float can occupy.
const MAX_WRAP_WIDTH_RATIO: f64 = 0.5;
```

### Y-Position Computation

```rust
/// Compute y-position for a wrap-float.
fn compute_wrap_float_y(
    &self,
    wf: &WrapFloatChild<'_>,
    frame: &Frame,
) -> FlowResult<Abs> {
    let region_height = self.regions.full;
    let float_height = frame.height();

    match wf.align_y {
        Smart::Auto => {
            // Near source position: current y
            Ok(self.current_y())
        }
        Smart::Custom(Some(FixedAlignment::Start)) => {
            // Top of region
            Ok(Abs::zero())
        }
        Smart::Custom(Some(FixedAlignment::End)) => {
            // Bottom of region (above bottom insertions)
            Ok(region_height - float_height - self.composer.column_insertions.bottom_size)
        }
        Smart::Custom(Some(FixedAlignment::Center)) => {
            // Center of region
            Ok((region_height - float_height) / 2.0)
        }
        Smart::Custom(None) => {
            unreachable!("wrap-float with align_y = Custom(None)")
        }
    }
}

/// Get current y-position in the flow.
fn current_y(&self) -> Abs {
    self.regions.full - self.regions.size.y
}
```

### Update Item Enum

**File: `crates/typst-layout/src/flow/distribute.rs`**

*Find with:* `grep -n "enum Item" crates/typst-layout/src/flow/distribute.rs`

```rust
enum Item<'a, 'b> {
    Tag(&'a Tag),
    Abs(Abs, u8),
    Fr(Fr, u8, Option<&'b SingleChild<'a>>),
    Frame(Frame, Axes<FixedAlignment>),
    Placed(Frame, &'b PlacedChild<'a>),
    /// A wrap-float: frame, y-position, x-alignment, delta.
    WrapFloat(Frame, Abs, FixedAlignment, Axes<Rel<Abs>>),
}
```

### Finalize: Render WrapFloats

In `Distributor::finalize()`:

```rust
Item::WrapFloat(frame, y, align_x, delta) => {
    let x = align_x.position(size.x - frame.width());
    let pos = Point::new(x, y) + delta.zip_map(size, Rel::relative_to).to_point();
    output.push_frame(pos, frame);
}
```

## Test Plan

**File: `tests/suite/layout/place/wrap-float-basic.typ`**

```typst
// --- wrap-float-right ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(60)

// --- wrap-float-left ---
#set page(height: 200pt, width: 200pt)
#place(top + left, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(60)

// --- wrap-float-bottom ---
#set page(height: 200pt, width: 200pt)
#place(bottom + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(60)

// --- wrap-float-auto-position ---
#set page(height: 300pt, width: 200pt)
Before the float.
#place(right, float: true, wrap: true,
  rect(width: 50pt, height: 50pt, fill: aqua))
Text that wraps around the float which appears near this position.
#lorem(40)

// --- wrap-float-multiple-same-side ---
#set page(height: 300pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 50pt, height: 40pt, fill: aqua))
#place(top + right, float: true, wrap: true, dy: 60pt,
  rect(width: 50pt, height: 40pt, fill: teal))
#lorem(60)

// --- wrap-float-opposite-sides ---
#set page(height: 300pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 50pt, height: 50pt, fill: aqua))
#place(top + left, float: true, wrap: true, dy: 30pt,
  rect(width: 50pt, height: 50pt, fill: teal))
#lorem(80)
```

## ParSpill with Exclusions

When a paragraph breaks across regions with wrap-floats, special handling is needed.

### The Problem

1. Region 1 has wrap-float → paragraph measured with exclusions
2. Frames 0-5 placed, frames 6-10 saved to `ParSpill`
3. Region 2 has no wrap-floats → different exclusions
4. Frames 6-10 have wrong widths (computed for Region 1's exclusions)

### Mitigation (Phase 3)

For initial implementation, accept this limitation with documentation:

```rust
struct ParSpill {
    frames: std::vec::IntoIter<(Frame, Abs)>,
    align: Axes<FixedAlignment>,
    leading: Abs,
    /// Whether frames were computed with exclusions.
    /// If true and current region has no exclusions, text may appear indented.
    had_exclusions: bool,
}
```

Add test for this edge case with expected behavior documented.

### Future Improvement

Store `ParChild` reference in `ParSpill` to enable re-measurement:
```rust
struct ParSpill<'a> {
    par: &'a ParChild<'a>,
    remaining_content_start: usize,  // Where to resume
    // ... rest
}
```

This allows re-measuring remaining content with new region's exclusions.

---

## Known Limitations

Document these for users:

1. **Thread count:** Run tests with `--num-threads 4` due to citation registry contention
2. **Paragraph spanning regions:** If a paragraph with wrap-float exclusions breaks across pages, continuation may have slight visual inconsistency
3. **Citations in wrapped paragraphs:** Multiple measure iterations cause duplicate registry entries (mitigated by deduplication, but adds overhead)

---

## Exit Criteria

- [ ] Text wraps around wrap-floats correctly
- [ ] Iteration converges within 3 passes for normal cases
- [ ] Edge cases handled: too-wide floats, narrow gaps
- [ ] Wrap-floats do not consume vertical space
- [ ] Page breaks work correctly with wrap-floats
- [ ] Citation registry deduplication implemented (Issue 1)
- [ ] WrapState.clear_for_region_break() handles spanning floats
- [ ] ParSpill.had_exclusions flag added
- [ ] Tests pass with `--num-threads 4`

## Dependencies

- [Phase 2: Exclusion Data Structures](WRAP_PHASE_2.md) must be complete
- Phase 2 API changes implemented (commit signature, break_positions)

## Next Phase

[Phase 4: Variable-Width Knuth-Plass](WRAP_PHASE_4.md)
