# Implementation Plan: Text Wrap Around Floating Figures in Typst

## Executive Summary

This plan enables **floating figures with text wrap** using a principled model:
wrap-floats are first-class flow items, line breaking supports variable widths
with Knuth-Plass, and paragraphs are measured then committed to avoid height
estimation errors.

### Key Architectural Changes

1) **Distinct Wrap-Float Kind**
Introduce a dedicated float kind (`wrap-float`) that shares placement rules with
normal floats but is handled as a flow item for exclusion computation. This
keeps semantics crisp and avoids unintentional regressions for existing floats.

2) **Two-Phase Paragraph Layout**
Paragraphs are measured (line breaks + metrics) then committed (frames).
Measurements depend on the active exclusion map (width(y)).

3) **Variable-Width Knuth-Plass**
Extend the optimized breaker to support a width function per line, preserving
quality while enabling wrap.

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

This mirrors how `SingleChild` and `MultiChild` already work for blocks.

---

## Part 0: Prerequisite Refactoring (Phase 0)

Before implementing wrap-floats, we must refactor the inline layout system to
separate `Preparation` creation from line breaking. This de-risks the main
implementation and ensures we have the building blocks in place.

### 0.1 Problem Statement

Currently, `layout_par()` in `inline/mod.rs` performs these steps atomically:

```rust
// Current flow (inline/mod.rs:151-178)
fn layout_inline_impl(...) -> SourceResult<Fragment> {
    let config = configuration(...);
    let (text, segments, spans) = collect(...)?;      // 1. Collect text
    let p = prepare(engine, &config, &text, ...)?;    // 2. Prepare (BiDi, shape)
    let lines = linebreak(engine, &p, width);         // 3. Break lines
    finalize(engine, &p, &lines, region, ...)         // 4. Create frames
}
```

The `Preparation` struct is created and consumed within this function. For
wrap-floats, we need to:
- Preserve `Preparation` across measure/commit calls
- Support re-breaking with different width constraints
- Track line metrics without creating frames

### 0.2 Refactoring Goals

1. Extract `Preparation` creation into a separate, cacheable step
2. Make line breaking independent of frame creation
3. Add line metrics computation without frame creation
4. Ensure backward compatibility (existing tests pass unchanged)

### 0.3 New Internal API

**File: `crates/typst-layout/src/inline/mod.rs`**

Add new internal functions that separate the phases:

```rust
/// Phase 1: Collect and prepare text for line breaking.
/// This is expensive (BiDi analysis, shaping) and should be cached.
pub fn prepare_par(
    elem: &Packed<ParElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
) -> SourceResult<PreparedPar> {
    let arenas = Arenas::default();
    let children = realize(...)?;
    let config = configuration(...);
    let (text, segments, spans) = collect(...)?;
    let p = prepare(engine, &config, &text, segments, spans)?;
    Ok(PreparedPar { config, p, region, expand, situation })
}

/// Phase 2: Break lines with a given width (possibly variable).
/// This is relatively cheap if shaping is already done.
pub fn break_lines(
    engine: &Engine,
    prepared: &PreparedPar,
    width: Abs,
) -> Vec<Line> {
    linebreak(engine, &prepared.p, width - prepared.config.hanging_indent)
}

/// Phase 3: Compute line metrics without creating frames.
/// Used for measure phase in wrap-float layout.
pub fn measure_lines(
    prepared: &PreparedPar,
    lines: &[Line],
) -> Vec<LineMetrics> {
    lines.iter().map(|line| LineMetrics {
        height: line.height(),
        ascent: line.ascent(),
        descent: line.descent(),
        width: line.width,
    }).collect()
}

/// Phase 4: Create frames from lines (the "commit" phase).
pub fn finalize_lines(
    engine: &mut Engine,
    prepared: &PreparedPar,
    lines: &[Line],
    locator: &mut SplitLocator,
) -> SourceResult<Fragment> {
    finalize(engine, &prepared.p, lines, prepared.region, prepared.expand, locator)
}

/// Wrapper struct holding prepared paragraph state.
pub struct PreparedPar<'a> {
    config: Config,
    p: Preparation<'a>,
    region: Size,
    expand: bool,
    situation: ParSituation,
}

/// Line metrics for measure phase.
#[derive(Debug, Clone, Copy)]
pub struct LineMetrics {
    pub height: Abs,
    pub ascent: Abs,
    pub descent: Abs,
    pub width: Abs,
}
```

### 0.4 Backward Compatibility Layer

The existing `layout_par()` function remains unchanged but now delegates to
the new API:

```rust
pub fn layout_par(...) -> SourceResult<Fragment> {
    let prepared = prepare_par(elem, engine, locator.clone(), ...)?;
    let lines = break_lines(engine, &prepared, region.x);
    finalize_lines(engine, &prepared, &lines, &mut locator.split())
}
```

### 0.5 Preparation Lifetime Challenge

The `Preparation` struct contains borrowed references:
```rust
pub struct Preparation<'a> {
    pub text: &'a str,
    pub config: &'a Config,
    // ...
}
```

**Solution:** The `PreparedPar` struct owns the `Config` and text storage,
with `Preparation` borrowing from it. This requires careful lifetime management:

```rust
pub struct PreparedPar {
    // Owned data
    config: Config,
    text: String,
    arenas: Arenas,
    // Derived data (borrows from above)
    // Note: This requires self-referential struct handling
}
```

**Implementation approach:** Use `ouroboros` crate or manual unsafe to create
self-referential struct, OR restructure to pass owned data through the pipeline.

**Recommended approach:** Store `Packed<ParElem>` and re-derive `Preparation`
when needed. This is slightly slower but avoids self-referential complexity.
Cache the `Preparation` at the comemo level instead.

### 0.6 Phase 0 Exit Criteria

1. All existing paragraph tests pass unchanged
2. `prepare_par` + `break_lines` + `finalize_lines` produces identical output
   to the current single-pass `layout_par`
3. Line metrics from `measure_lines` match actual frame heights
4. No performance regression > 5% on paragraph-heavy documents

### 0.7 Phase 0 Test Plan

**File: `tests/suite/layout/inline/prepare-api.typ`**

```typst
// --- prepare-api-basic ---
// Verify that the new API produces identical output
#set page(width: 200pt, height: auto)
#lorem(50)

// --- prepare-api-justified ---
#set page(width: 200pt, height: auto)
#set par(justify: true)
#lorem(50)

// --- prepare-api-bidi ---
#set page(width: 200pt, height: auto)
#set text(lang: "ar")
مرحبا بالعالم #text(lang: "en")[Hello World] مرحبا

// --- prepare-api-mixed-sizes ---
#set page(width: 200pt, height: auto)
Normal text #text(size: 20pt)[BIG] normal #text(size: 8pt)[small] normal.
```

---

## Part 1: New ParChild Structure

### 1.1 The ParChild Type (Measure + Commit)

**File: `crates/typst-layout/src/flow/collect.rs`**

Add new struct after `LineChild` (around line 375):

```rust
/// A child that encapsulates a prepared paragraph, laid out on demand.
///
/// Unlike `LineChild` which contains already-rendered frames, `ParChild`
/// stores the original paragraph element and defers layout until distribution.
/// This enables variable-width line breaking for wrap-floats.
#[derive(Debug)]
pub struct ParChild<'a> {
    /// The paragraph element.
    pub elem: &'a Packed<ParElem>,
    /// The style chain.
    pub styles: StyleChain<'a>,
    /// The locator for this paragraph (used for both measure and commit).
    /// We store the locator and use `relayout()` for commit to maintain
    /// consistent location tracking.
    pub locator: Locator<'a>,
    /// Whether to expand.
    pub expand: bool,
    /// The paragraph situation (first, consecutive, other).
    pub situation: ParSituation,
    /// Paragraph spacing (above/below).
    pub spacing: Abs,
    /// Line leading.
    pub leading: Abs,
    /// Text alignment.
    pub align: Axes<FixedAlignment>,
    /// Widow/orphan cost settings.
    pub costs: Costs,
}

/// Result of measuring a paragraph (line breaks + metrics, no frames).
#[derive(Debug, Clone)]
pub struct ParMeasureResult {
    /// Per-line metrics (height, ascent, descent, width).
    pub metrics: Vec<LineMetrics>,
    /// Total height of the paragraph (sum of line heights + leading).
    pub total_height: Abs,
    /// Height information for widow/orphan prevention.
    pub line_heights: LineHeights,
    /// The line break positions (byte offsets) for reconstruction.
    pub break_positions: Vec<usize>,
    /// Hash of the measurement inputs for cache validation.
    pub input_hash: u128,
}

/// Heights of edge lines for widow/orphan calculations.
#[derive(Debug, Clone, Copy, Default)]
pub struct LineHeights {
    pub front_1: Abs,
    pub front_2: Abs,
    pub back_2: Abs,
    pub back_1: Abs,
    pub len: usize,
}

impl LineHeights {
    /// Compute from a list of line heights.
    pub fn from_heights(heights: &[Abs]) -> Self {
        let len = heights.len();
        Self {
            front_1: heights.first().copied().unwrap_or_default(),
            front_2: heights.get(1).copied().unwrap_or_default(),
            back_2: heights.get(len.saturating_sub(2)).copied().unwrap_or_default(),
            back_1: heights.last().copied().unwrap_or_default(),
            len,
        }
    }
}

/// Result of committing a paragraph (actual frames).
#[derive(Debug)]
pub struct ParCommitResult {
    /// The laid-out line frames.
    pub frames: Vec<Frame>,
    /// Per-frame "need" for widow/orphan prevention.
    pub needs: Vec<Abs>,
}

impl<'a> ParChild<'a> {
    /// Measure the paragraph with optional width exclusions.
    ///
    /// This performs line breaking and computes metrics but does NOT create
    /// frames. The result can be used to determine layout and then committed.
    ///
    /// **Locator handling:** Measurement uses the stored locator for cache
    /// consistency. The actual frame creation in `commit` uses `locator.relayout()`
    /// to ensure introspection locations are stable.
    pub fn measure(
        &self,
        engine: &mut Engine,
        region: Size,
        exclusions: Option<&ParExclusions>,
    ) -> SourceResult<ParMeasureResult> {
        measure_par_impl(
            engine.routines,
            engine.world,
            engine.introspector.into_raw(),
            engine.traced,
            TrackedMut::reborrow_mut(&mut engine.sink),
            self.elem,
            self.locator.track(),
            self.styles,
            region,
            self.expand,
            self.situation,
            exclusions.cloned(),
        )
    }

    /// Commit measured lines into frames.
    ///
    /// This MUST be called with a `ParMeasureResult` from a prior `measure()`
    /// call on the same paragraph. The `input_hash` is validated to ensure
    /// consistency.
    ///
    /// **Locator handling:** Uses `locator.relayout()` to create frames with
    /// consistent locations. This ensures that introspection queries return
    /// stable results even if the paragraph is re-measured with different
    /// exclusions.
    pub fn commit(
        &self,
        engine: &mut Engine,
        measured: &ParMeasureResult,
        region: Size,
        exclusions: Option<&ParExclusions>,
    ) -> SourceResult<ParCommitResult> {
        commit_par_impl(
            engine,
            self.elem,
            self.locator.relayout(),
            self.styles,
            region,
            self.expand,
            self.situation,
            exclusions,
            measured,
            self.leading,
            self.costs,
        )
    }

    /// Convenience method: measure and immediately commit.
    /// Used when there are no exclusions (backward-compatible path).
    pub fn layout(
        &self,
        engine: &mut Engine,
        region: Size,
    ) -> SourceResult<ParCommitResult> {
        let measured = self.measure(engine, region, None)?;
        self.commit(engine, &measured, region, None)
    }

    /// Compute the "need" height for a specific line (widow/orphan prevention).
    pub fn compute_need(
        &self,
        line_index: usize,
        line_heights: &LineHeights,
        leading: Abs,
    ) -> Abs {
        let len = line_heights.len;
        let prevent_orphans = self.costs.orphan() > Ratio::zero() && len >= 2;
        let prevent_widows = self.costs.widow() > Ratio::zero() && len >= 2;
        let prevent_all = len == 3 && prevent_orphans && prevent_widows;

        if prevent_all && line_index == 0 {
            line_heights.front_1 + leading + line_heights.front_2 + leading + line_heights.back_1
        } else if prevent_orphans && line_index == 0 {
            line_heights.front_1 + leading + line_heights.front_2
        } else if prevent_widows && line_index >= 2 && line_index + 2 == len {
            line_heights.back_2 + leading + line_heights.back_1
        } else {
            // Default: just this line's height
            match line_index {
                0 => line_heights.front_1,
                i if i == len - 1 => line_heights.back_1,
                1 => line_heights.front_2,
                i if i == len - 2 => line_heights.back_2,
                _ => Abs::zero(), // Middle lines use actual height from metrics
            }
        }
    }
}
```

### 1.2 Locator Handling Strategy

The locator problem is critical: we must ensure that introspection (`location()`,
`query()`) returns consistent results regardless of how many times we measure.

**Solution:** Two-tier locator usage:

1. **Measure phase:** Use `self.locator.track()` for cache key computation.
   This ensures the same paragraph always gets the same cache entry.

2. **Commit phase:** Use `self.locator.relayout()` for frame creation.
   This creates a fresh locator chain that produces stable locations.

**Invariant:** A paragraph's introspection-visible location is determined by
its position in the source, not by how many times it was measured.

**Validation:** Add debug assertions that verify location consistency:

```rust
#[cfg(debug_assertions)]
fn validate_locations(frames: &[Frame], expected_base: Location) {
    for frame in frames {
        for (_, item) in frame.items() {
            if let FrameItem::Tag(Tag::Start(elem, loc)) = item {
                debug_assert!(
                    loc.is_descendant_of(expected_base),
                    "Location mismatch in paragraph layout"
                );
            }
        }
    }
}
```

### 1.3 Update Child Enum

**File: `crates/typst-layout/src/flow/collect.rs`**

Modify the `Child` enum (around line 347):

```rust
pub enum Child<'a> {
    /// An introspection tag.
    Tag(&'a Tag),
    /// Relative spacing with a specific weakness level.
    Rel(Rel<Abs>, u8),
    /// Fractional spacing with a specific weakness level.
    Fr(Fr, u8),
    /// A paragraph, measured on demand (new in wrap-float).
    Par(BumpBox<'a, ParChild<'a>>),
    /// An already layouted line of a paragraph (used for inline mode only).
    Line(BumpBox<'a, LineChild>),
    /// A wrap-enabled floating element in-flow.
    WrapFloat(BumpBox<'a, WrapFloatChild<'a>>),
    /// An unbreakable block.
    Single(BumpBox<'a, SingleChild<'a>>),
    /// A breakable block.
    Multi(BumpBox<'a, MultiChild<'a>>),
    /// An absolutely or floatingly placed element.
    Placed(BumpBox<'a, PlacedChild<'a>>),
    /// A place flush.
    Flush,
    /// An explicit column break.
    Break(bool),
}
```

### 1.4 Update Collector::par()

**File: `crates/typst-layout/src/flow/collect.rs`**

Replace the `par` method (lines 158-185):

```rust
/// Collect a paragraph into a [`ParChild`] for deferred layout.
fn par(
    &mut self,
    elem: &'a Packed<ParElem>,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    let spacing = elem.spacing.resolve(styles);
    let leading = elem.leading.resolve(styles);
    let align = styles.resolve(AlignElem::alignment);
    let costs = styles.get(TextElem::costs);

    // Add spacing before paragraph
    self.output.push(Child::Rel(spacing.into(), 4));

    // Store paragraph for deferred layout
    self.output.push(Child::Par(self.boxed(ParChild {
        elem,
        styles,
        locator: self.locator.next(&elem.span()),
        expand: self.expand,
        situation: self.par_situation,
        spacing,
        leading,
        align,
        costs,
    })));

    // Add spacing after paragraph
    self.output.push(Child::Rel(spacing.into(), 4));
    self.par_situation = ParSituation::Consecutive;

    Ok(())
}
```

### 1.5 Memoized Measurement Implementation

**File: `crates/typst-layout/src/flow/collect.rs`**

```rust
/// The cached, internal implementation of paragraph measurement.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn measure_par_impl(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    elem: &Packed<ParElem>,
    locator: Tracked<Locator>,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
    exclusions: Option<ParExclusions>,
) -> SourceResult<ParMeasureResult> {
    let introspector = Protected::from_raw(introspector);
    let link = LocatorLink::new(locator);
    let locator = Locator::link(&link);
    let mut engine = Engine {
        routines,
        world,
        introspector,
        traced,
        sink,
        route: Route::default(),
    };

    crate::inline::measure_par_with_exclusions(
        elem, &mut engine, locator, styles, region, expand, situation, exclusions,
    )
}

/// Commit measured lines into frames (not memoized - creates unique frames).
fn commit_par_impl(
    engine: &mut Engine,
    elem: &Packed<ParElem>,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
    exclusions: Option<&ParExclusions>,
    measured: &ParMeasureResult,
    leading: Abs,
    costs: Costs,
) -> SourceResult<ParCommitResult> {
    crate::inline::commit_par(
        engine, elem, locator, styles, region, expand, situation,
        exclusions, measured, leading, costs,
    )
}
```

---

## Part 2: Exclusion Data Structures

### 2.1 Coordinate System Specification

**All coordinates in this system are relative to the "inner flow origin":**

```
┌─────────────────────────────────────────┐
│  Page                                   │
│  ┌───────────────────────────────────┐  │
│  │  Top Insertions (floats)          │  │  ← page_insertions.top_size
│  ├───────────────────────────────────┤  │
│  │  Inner Flow Origin (y=0)    ──────┼──┼── This is the coordinate origin
│  │  ┌─────────────────────────────┐  │  │
│  │  │  Content Region             │  │  │
│  │  │                             │  │  │
│  │  │    Paragraph at y=50pt      │  │  │  ← ParExclusions use this y
│  │  │    ┌─────────┐              │  │  │
│  │  │    │ WrapFloat│  Text wraps │  │  │  ← WrapFloat.y is relative to
│  │  │    │ at y=30pt│  around it  │  │  │     inner flow origin
│  │  │    └─────────┘              │  │  │
│  │  │                             │  │  │
│  │  └─────────────────────────────┘  │  │
│  ├───────────────────────────────────┤  │
│  │  Bottom Insertions (floats, fn)   │  │  ← page_insertions.bottom_size
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

**Key invariants:**

1. `WrapFloat.y` is in inner-flow coordinates (y=0 at top of content region)
2. `ParExclusions` zones are in paragraph-relative coordinates (y=0 at paragraph top)
3. When computing exclusions for a paragraph at `par_y`, transform:
   `exclusion_zone.y_start = wrap_float.y - par_y`
4. Footnotes/bottom insertions reduce available height but don't change y=0

### 2.2 ParExclusions Type

**File: `crates/typst-library/src/layout/regions.rs`**

Add after line 159:

```rust
/// Width exclusions for text wrapping around floats.
///
/// Coordinates are relative to the paragraph's top-left corner.
/// Use `from_wrap_floats()` to convert from region coordinates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ParExclusions {
    /// Exclusion zones sorted by y_start.
    pub zones: Vec<ExclusionZone>,
}

/// A single rectangular exclusion zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExclusionZone {
    /// Y-offset from paragraph top where exclusion starts (in raw Abs units).
    pub y_start: i64,
    /// Y-offset from paragraph top where exclusion ends (in raw Abs units).
    pub y_end: i64,
    /// Width excluded from left side (in raw Abs units).
    pub left: i64,
    /// Width excluded from right side (in raw Abs units).
    pub right: i64,
}

impl ParExclusions {
    /// Check if there are no exclusions.
    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    /// Create exclusions from wrap-float positions.
    ///
    /// Converts region-relative float coordinates to paragraph-relative
    /// exclusion zones. Only includes floats that overlap the paragraph's
    /// vertical extent.
    ///
    /// # Arguments
    /// * `par_y` - Paragraph's y-position in region coordinates
    /// * `par_height` - Estimated paragraph height (for overlap detection)
    /// * `wrap_floats` - List of positioned wrap-floats in region coordinates
    pub fn from_wrap_floats(
        par_y: Abs,
        par_height: Abs,
        wrap_floats: &[WrapFloat],
    ) -> Self {
        let mut zones = Vec::with_capacity(wrap_floats.len());
        let par_top = par_y.to_raw();
        let par_bottom = (par_y + par_height).to_raw();

        for wf in wrap_floats {
            let wf_top = wf.y.to_raw();
            let wf_bottom = (wf.y + wf.height).to_raw();

            // Skip floats that don't overlap this paragraph
            if wf_bottom <= par_top || wf_top >= par_bottom {
                continue;
            }

            // Convert to paragraph-relative coordinates, clamped to paragraph bounds
            let rel_start = (wf_top - par_top).max(0);
            let rel_end = (wf_bottom - par_top).min(par_bottom - par_top);

            zones.push(ExclusionZone {
                y_start: rel_start,
                y_end: rel_end,
                left: wf.left_margin.to_raw(),
                right: wf.right_margin.to_raw(),
            });
        }

        // Sort by y_start for efficient lookup
        zones.sort_by_key(|z| z.y_start);

        Self { zones }
    }

    /// Get available width at a given y-offset within the paragraph.
    ///
    /// If multiple exclusions overlap at this y, takes the maximum from each side.
    pub fn available_width(&self, base_width: Abs, y: Abs) -> Abs {
        let y_raw = y.to_raw();
        let mut left_excluded = 0i64;
        let mut right_excluded = 0i64;

        for zone in &self.zones {
            // Early exit: zones are sorted, so if y < y_start, no more matches
            if y_raw < zone.y_start {
                break;
            }
            if y_raw < zone.y_end {
                left_excluded = left_excluded.max(zone.left);
                right_excluded = right_excluded.max(zone.right);
            }
        }

        let total_excluded = Abs::from_raw(left_excluded) + Abs::from_raw(right_excluded);
        (base_width - total_excluded).max(Abs::zero())
    }

    /// Get left offset at a given y-offset (for text positioning).
    pub fn left_offset(&self, y: Abs) -> Abs {
        let y_raw = y.to_raw();
        let mut left = 0i64;

        for zone in &self.zones {
            if y_raw < zone.y_start {
                break;
            }
            if y_raw < zone.y_end {
                left = left.max(zone.left);
            }
        }

        Abs::from_raw(left)
    }

    /// Check if any exclusion is active at this y-offset.
    pub fn has_exclusion_at(&self, y: Abs) -> bool {
        let y_raw = y.to_raw();
        self.zones.iter().any(|z| y_raw >= z.y_start && y_raw < z.y_end)
    }

    /// Get the next y-position where exclusions change (for line breaking).
    pub fn next_boundary(&self, y: Abs) -> Option<Abs> {
        let y_raw = y.to_raw();
        self.zones
            .iter()
            .flat_map(|z| [z.y_start, z.y_end])
            .filter(|&boundary| boundary > y_raw)
            .min()
            .map(Abs::from_raw)
    }
}

/// A positioned wrap-float in region coordinates.
#[derive(Debug, Clone)]
pub struct WrapFloat {
    /// Top y-coordinate in region (inner-flow) coordinates.
    pub y: Abs,
    /// Height of the float.
    pub height: Abs,
    /// Width excluded from left (float width + clearance, or 0 if right-aligned).
    pub left_margin: Abs,
    /// Width excluded from right (float width + clearance, or 0 if left-aligned).
    pub right_margin: Abs,
}

impl WrapFloat {
    /// Create a wrap-float from a placed child and its computed position.
    pub fn from_placed(
        frame: &Frame,
        y: Abs,
        align_x: FixedAlignment,
        clearance: Abs,
    ) -> Self {
        let width = frame.width() + clearance;
        let (left_margin, right_margin) = match align_x {
            FixedAlignment::Start => (width, Abs::zero()),
            FixedAlignment::End => (Abs::zero(), width),
            FixedAlignment::Center => {
                // Center-aligned wrap-floats exclude from both sides
                let half = width / 2.0;
                (half, half)
            }
        };
        Self {
            y,
            height: frame.height(),
            left_margin,
            right_margin,
        }
    }
}
```

### 2.3 Wrap Parameter on PlaceElem

**File: `crates/typst-library/src/layout/place.rs`**

Add to `PlaceElem` struct:

```rust
/// Whether text should wrap around this floating element.
///
/// When enabled with `float: true`, paragraphs will have shortened
/// lines adjacent to this element. Only effective when horizontal
/// alignment is `left` or `right` (center-aligned wrapping is experimental).
///
/// Wrap-floats do not consume vertical space in the flow; text flows
/// around them. This differs from normal floats which reserve space
/// at the top or bottom of the page/column.
///
/// ```example
/// #set page(height: 200pt)
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

---

## Part 3: Distribution Changes (In-Flow Wrap Floats)

### 3.1 New Flow Item: WrapFloat

**File: `crates/typst-layout/src/flow/collect.rs`**

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

**Collection rule:** In `Collector::place()`, check for `wrap: true`:

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

### 3.2 WrapState in Distributor

**File: `crates/typst-layout/src/flow/distribute.rs`**

Add wrap-float state tracking:

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

    /// Clear all wrap-floats (called at region boundaries).
    fn clear(&mut self) {
        self.floats.clear();
    }
}
```

### 3.3 Distributor::par() Implementation

**File: `crates/typst-layout/src/flow/distribute.rs`**

Add handler for `Child::Par`:

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
            // Iterative refinement for height-dependent exclusions
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

    /// Iterative refinement for paragraphs affected by wrap exclusions.
    ///
    /// The circular dependency (line height affects exclusions, exclusions
    /// affect line breaks) is resolved by iteration:
    /// 1. Measure with current exclusion estimate
    /// 2. Recompute exclusions from actual line heights
    /// 3. Re-measure if exclusions changed
    /// 4. Stop when stable or after MAX_WRAP_ITER
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

    /// Emit paragraph line frames as distributor items.
    fn emit_paragraph_lines(
        &mut self,
        par: &ParChild<'_>,
        commit: &ParCommitResult,
        measure: &ParMeasureResult,
    ) -> FlowResult<()> {
        for (i, frame) in commit.frames.iter().enumerate() {
            if i > 0 {
                // Inter-line spacing
                self.regions.size.y -= par.leading;
                self.items.push(Item::Abs(par.leading, 5));
            }

            // Handle footnotes in this line
            self.composer.footnotes(
                &self.regions,
                frame,
                frame.height(),
                false,  // lines are not breakable
                true,   // migratable
            )?;

            // Reduce available space
            self.regions.size.y -= frame.height();
            self.flush_tags();
            self.items.push(Item::Frame(frame.clone(), par.align));
        }

        Ok(())
    }

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
                // Should have been caught during collection
                unreachable!("wrap-float with align_y = Custom(None)")
            }
        }
    }

    /// Get current y-position in the flow.
    fn current_y(&self) -> Abs {
        self.regions.full - self.regions.size.y
    }
}

/// Maximum ratio of page width a wrap-float can occupy.
const MAX_WRAP_WIDTH_RATIO: f64 = 0.5;

/// Layout a wrap-float's content.
fn layout_wrap_float(
    engine: &mut Engine,
    wf: &WrapFloatChild<'_>,
    base: Size,
) -> SourceResult<Frame> {
    let align = wf.elem.alignment.get(wf.styles).unwrap_or(Alignment::CENTER);
    let aligned = AlignElem::alignment.set(align).wrap();
    let styles = wf.styles.chain(&aligned);

    crate::layout_frame(
        engine,
        &wf.elem.body,
        wf.locator.relayout(),
        styles,
        Region::new(base, Axes::splat(false)),
    )
}
```

### 3.4 Update Item Enum for WrapFloat

**File: `crates/typst-layout/src/flow/distribute.rs`**

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

### 3.5 Finalize: Render WrapFloats

In `Distributor::finalize()`, handle `Item::WrapFloat`:

```rust
Item::WrapFloat(frame, y, align_x, delta) => {
    let x = align_x.position(size.x - frame.width());
    let pos = Point::new(x, y) + delta.zip_map(size, Rel::relative_to).to_point();
    output.push_frame(pos, frame);
}
```

---

## Part 4: Variable-Width Knuth-Plass Algorithm

### 4.1 Algorithm Overview

The standard Knuth-Plass algorithm assumes a constant line width. For wrap-floats,
we need variable widths based on vertical position. This section specifies the
algorithm changes in detail.

### 4.2 Key Insight: Per-Line Width Lookup

Instead of a single `width: Abs`, we use a width function:

```rust
/// Width available for a line at the given y-offset.
type WidthFn<'a> = &'a dyn Fn(Abs) -> Abs;
```

The challenge: we don't know line y-positions until we've broken lines, but
breaking depends on widths. This is resolved by:

1. **First pass:** Break with uniform width, measure line heights
2. **Second pass:** Use measured heights to compute per-line widths
3. **Re-break:** If widths differ significantly, re-break and re-measure
4. **Converge:** Stop when line breaks stabilize

### 4.3 Modified Knuth-Plass Implementation

**File: `crates/typst-layout/src/inline/linebreak.rs`**

```rust
/// Performs line breaking with variable widths.
///
/// # Arguments
/// * `engine` - Layout engine
/// * `p` - Prepared paragraph
/// * `base_width` - Default width (when no exclusions)
/// * `exclusions` - Optional exclusion zones
///
/// # Algorithm
/// 1. If no exclusions, use standard K-P
/// 2. Otherwise, use iterative refinement
#[typst_macros::time]
pub fn linebreak_variable_width<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    base_width: Abs,
    exclusions: Option<&ParExclusions>,
) -> Vec<Line<'a>> {
    match exclusions {
        None => linebreak(engine, p, base_width),
        Some(excl) if excl.is_empty() => linebreak(engine, p, base_width),
        Some(excl) => linebreak_with_exclusions(engine, p, base_width, excl),
    }
}

/// Line breaking with exclusion zones.
fn linebreak_with_exclusions<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    base_width: Abs,
    exclusions: &ParExclusions,
) -> Vec<Line<'a>> {
    // Configuration
    const MAX_ITERATIONS: usize = 3;
    const CONVERGENCE_THRESHOLD: Abs = Abs::pt(0.5);

    // Track line heights from previous iteration
    let mut prev_heights: Vec<Abs> = vec![];
    let mut lines: Vec<Line<'a>> = vec![];

    for iteration in 0..MAX_ITERATIONS {
        // Compute width function from current height estimates
        let width_at = |y: Abs| -> Abs {
            exclusions.available_width(base_width, y)
        };

        // Compute per-line widths based on estimated y-positions
        let line_widths = compute_line_widths(
            &prev_heights,
            p.config.font_size, // Default line height estimate
            exclusions,
            base_width,
        );

        // Break lines with these widths
        lines = if line_widths.iter().all(|&w| w == base_width) {
            // No exclusions affect this paragraph, use standard K-P
            linebreak(engine, p, base_width)
        } else {
            linebreak_variable(engine, p, &line_widths)
        };

        // Measure actual line heights
        let heights: Vec<Abs> = lines.iter().map(|line| line.height()).collect();

        // Check convergence
        if heights.len() == prev_heights.len() {
            let max_diff = heights.iter()
                .zip(&prev_heights)
                .map(|(a, b)| (*a - *b).abs())
                .fold(Abs::zero(), Abs::max);

            if max_diff < CONVERGENCE_THRESHOLD {
                break;
            }
        }

        prev_heights = heights;
    }

    lines
}

/// Compute width available for each line based on y-positions.
fn compute_line_widths(
    prev_heights: &[Abs],
    default_height: Abs,
    exclusions: &ParExclusions,
    base_width: Abs,
) -> Vec<Abs> {
    if prev_heights.is_empty() {
        // First iteration: estimate based on default line height
        // Generate enough widths for a reasonable paragraph
        let max_lines = 100;
        let mut widths = Vec::with_capacity(max_lines);
        let mut y = Abs::zero();

        for _ in 0..max_lines {
            widths.push(exclusions.available_width(base_width, y));
            y += default_height;
        }

        widths
    } else {
        // Use actual heights from previous iteration
        let mut widths = Vec::with_capacity(prev_heights.len() + 10);
        let mut y = Abs::zero();

        for &height in prev_heights {
            widths.push(exclusions.available_width(base_width, y));
            y += height;
        }

        // Add a few extra in case line count increases
        let avg_height = prev_heights.iter().sum::<Abs>() / prev_heights.len() as f64;
        for _ in 0..10 {
            widths.push(exclusions.available_width(base_width, y));
            y += avg_height;
        }

        widths
    }
}

/// Knuth-Plass with per-line width constraints.
///
/// This is a modified version of `linebreak_optimized` that accepts
/// different widths for each line index.
#[typst_macros::time]
fn linebreak_variable<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    line_widths: &[Abs],
) -> Vec<Line<'a>> {
    // For very simple cases, fall back to simple breaking
    if should_use_simple_breaking(p, line_widths) {
        return linebreak_simple_variable(engine, p, line_widths);
    }

    let metrics = CostMetrics::compute(p);

    // Modified K-P: width depends on line index
    linebreak_optimized_variable(engine, p, line_widths, &metrics)
}

/// Check if we should fall back to simple breaking.
fn should_use_simple_breaking(p: &Preparation, line_widths: &[Abs]) -> bool {
    // Guardrail 1: Very long paragraphs
    const MAX_TEXT_LEN: usize = 10_000;
    if p.text.len() > MAX_TEXT_LEN {
        return true;
    }

    // Guardrail 2: Highly variable widths (complex exclusions)
    if line_widths.len() >= 2 {
        let min = line_widths.iter().copied().fold(Abs::inf(), Abs::min);
        let max = line_widths.iter().copied().fold(Abs::zero(), Abs::max);
        let variance_ratio = (max - min) / max;

        const MAX_VARIANCE_RATIO: f64 = 0.5;
        if variance_ratio > MAX_VARIANCE_RATIO {
            return true;
        }
    }

    // Guardrail 3: Explicit config
    // (Would need to add a config option for this)

    false
}

/// Simple greedy line breaking with variable widths.
fn linebreak_simple_variable<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    line_widths: &[Abs],
) -> Vec<Line<'a>> {
    let mut lines = Vec::with_capacity(16);
    let mut start = 0;
    let mut last = None;
    let mut line_index = 0;

    let get_width = |idx: usize| -> Abs {
        line_widths.get(idx).copied().unwrap_or_else(|| {
            line_widths.last().copied().unwrap_or(Abs::inf())
        })
    };

    breakpoints(p, |end, breakpoint| {
        let width = get_width(line_index);
        let mut attempt = line(engine, p, start..end, breakpoint, lines.last());

        if !width.fits(attempt.width) && let Some((last_attempt, last_end)) = last.take() {
            lines.push(last_attempt);
            line_index += 1;
            start = last_end;
            attempt = line(engine, p, start..end, breakpoint, lines.last());
        }

        if breakpoint == Breakpoint::Mandatory || !width.fits(attempt.width) {
            lines.push(attempt);
            line_index += 1;
            start = end;
            last = None;
        } else {
            last = Some((attempt, end));
        }
    });

    if let Some((line, _)) = last {
        lines.push(line);
    }

    lines
}

/// Optimized K-P with variable line widths.
///
/// **Key difference from standard K-P:** The width constraint for each
/// candidate line depends on which line index it would become.
///
/// **Pruning modification:** Active-set pruning is DISABLED when widths
/// vary significantly, as the assumption "shorter lines have higher ratios"
/// no longer holds.
fn linebreak_optimized_variable<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    line_widths: &[Abs],
    metrics: &CostMetrics,
) -> Vec<Line<'a>> {
    struct Entry<'a> {
        pred: usize,
        total: Cost,
        line: Line<'a>,
        end: usize,
        line_index: usize,
    }

    let get_width = |idx: usize| -> Abs {
        line_widths.get(idx).copied().unwrap_or_else(|| {
            line_widths.last().copied().unwrap_or(Abs::inf())
        })
    };

    // Check if widths vary enough to disable pruning
    let widths_vary = line_widths.windows(2)
        .any(|w| (w[0] - w[1]).abs() > Abs::pt(1.0));

    let mut table = vec![Entry {
        pred: 0,
        total: 0.0,
        line: Line::empty(),
        end: 0,
        line_index: 0,
    }];

    let mut active = 0;
    let mut prev_end = 0;

    breakpoints(p, |end, breakpoint| {
        let mut best: Option<Entry> = None;

        for (pred_index, pred) in table.iter().enumerate().skip(active) {
            let start = pred.end;
            let unbreakable = prev_end == start;
            let this_line_index = pred.line_index + if pred_index == 0 { 0 } else { 1 };
            let width = get_width(this_line_index);

            let attempt = line(engine, p, start..end, breakpoint, Some(&pred.line));

            let (line_ratio, line_cost) = ratio_and_cost(
                p,
                metrics,
                width,  // Use per-line width
                &pred.line,
                &attempt,
                breakpoint,
                unbreakable,
            );

            // Modified pruning: only prune if widths are uniform
            if !widths_vary && line_ratio < metrics.min_ratio && active == pred_index {
                active += 1;
            }

            let total = pred.total + line_cost;

            if best.as_ref().is_none_or(|best| best.total >= total) {
                best = Some(Entry {
                    pred: pred_index,
                    total,
                    line: attempt,
                    end,
                    line_index: this_line_index,
                });
            }
        }

        if breakpoint == Breakpoint::Mandatory {
            active = table.len();
        }

        table.extend(best);
        prev_end = end;
    });

    // Retrace the best path
    let mut lines = Vec::with_capacity(16);
    let mut idx = table.len() - 1;

    while idx != 0 {
        table.truncate(idx + 1);
        let entry = table.pop().unwrap();
        lines.push(entry.line);
        idx = entry.pred;
    }

    lines.reverse();
    lines
}
```

### 4.4 Cumulative Estimates for Variable Widths

The approximate K-P pass (`linebreak_optimized_approximate`) cannot be used
directly with variable widths because the cumulative metrics assume constant
width. For variable-width cases, we:

1. Skip the approximate pass
2. Use a higher upper bound (or INFINITY)
3. Accept potentially slower performance for wrapped paragraphs

```rust
fn linebreak_optimized_variable<'a>(...) -> Vec<Line<'a>> {
    // Skip approximate pass for variable widths - use direct bounded search
    // with INFINITY bound (no pruning based on approximate cost)
    linebreak_optimized_bounded_variable(engine, p, line_widths, metrics, Cost::INFINITY)
}
```

---

## Part 5: Known Limitations (V1)

The following limitations apply to the initial wrap-float implementation:

### 5.1 Content That Does NOT Wrap

- **Tables:** Block tables flow below wrap-floats, not around them
- **Block math:** Display equations do not wrap
- **Code blocks:** Preformatted code does not wrap
- **Lists:** List items do not wrap (each item is a block)
- **Images/figures:** Inline images break normally; block figures flow below

Only **paragraphs** (`ParElem`) support text wrapping.

### 5.2 Unsupported Configurations

- **Center-aligned wrap-floats:** Experimental, may produce poor results
- **Overlapping wrap-floats:** Same-side stacking works; complex overlaps may fail
- **Nested wrap-floats:** Wrap-floats inside wrap-floats are not supported
- **Wrap-floats in headers/footers:** Will be treated as normal floats

### 5.3 Performance Limitations

- **Very long paragraphs (>10k chars):** Fall back to simple line breaking
- **Many wrap-floats (>5 per page):** May cause noticeable slowdown
- **Complex exclusion shapes:** High width variance triggers simple breaking

### 5.4 Future Work

- Extend wrapping to lists and other block types
- Support for arbitrary exclusion shapes (non-rectangular)
- Improved K-P for highly variable widths
- User-configurable wrap behavior per content type

---

## Part 6: Testing Strategy

### 6.1 Test File Organization

All wrap-float tests go in `tests/suite/layout/place/` to match existing
float tests:

```
tests/suite/layout/place/
├── wrap-float-basic.typ        # Basic functionality
├── wrap-float-position.typ     # Positioning (top/bottom/auto)
├── wrap-float-multipage.typ    # Page breaks
├── wrap-float-columns.typ      # Column layouts
├── wrap-float-edge.typ         # Edge cases
└── wrap-float-stress.typ       # Performance/stress tests
```

### 6.2 Phase 0 Tests (Prerequisite Refactoring)

**File: `tests/suite/layout/inline/prepare-split.typ`**

```typst
// Verify that split prepare/break/finalize matches monolithic layout

// --- prepare-split-basic ---
#set page(width: 200pt, height: auto)
#lorem(20)

// --- prepare-split-justified ---
#set page(width: 200pt, height: auto)
#set par(justify: true)
#lorem(20)

// --- prepare-split-hyphenation ---
#set page(width: 100pt, height: auto)
#set text(hyphenate: true)
Incomprehensibilities and counterrevolutionaries.

// --- prepare-split-bidi ---
#set page(width: 200pt, height: auto)
Hello مرحبا World العالم mixed text.

// --- prepare-split-inline-elements ---
#set page(width: 200pt, height: auto)
Text with #box[inline box] and #h(1em) spacing.
```

### 6.3 Phase 1 Tests (ParChild Structure)

**File: `tests/suite/layout/flow/par-child.typ`**

```typst
// Verify ParChild measure/commit matches LineChild layout

// --- par-child-basic ---
#set page(width: 200pt, height: auto)
#lorem(30)

// --- par-child-widow-orphan ---
#set page(width: 200pt, height: 100pt)
#set text(costs: (widow: 100%, orphan: 100%))
First paragraph here.

#lorem(20)

// --- par-child-spacing ---
#set page(width: 200pt, height: auto)
#set par(spacing: 20pt, leading: 10pt)
First paragraph.

Second paragraph.
```

### 6.4 Phase 3 Tests (Wrap-Float Support)

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

### 6.5 Edge Case Tests

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

// --- wrap-float-with-footnote ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))
Text with a footnote#footnote[This is the footnote content.] that wraps.
#lorem(30)

// --- wrap-float-page-break ---
#set page(height: 150pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(100)

// --- wrap-float-mixed-sizes ---
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 80pt, fill: aqua))
Normal text #text(size: 20pt)[BIG TEXT] normal #text(size: 8pt)[small] normal.
#lorem(40)

// --- wrap-float-rtl ---
#set page(height: 200pt, width: 200pt)
#set text(dir: rtl, lang: "ar")
#place(top + left, float: true, wrap: true,
  rect(width: 60pt, height: 60pt, fill: aqua))
هذا نص عربي يلتف حول الشكل العائم. #lorem(30)
```

### 6.6 Stress Tests

**File: `tests/suite/layout/place/wrap-float-stress.typ`**

```typst
// --- wrap-float-long-paragraph ---
#set page(height: auto, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 60pt, height: 100pt, fill: aqua))
#lorem(500)

// --- wrap-float-many-floats ---
#set page(height: 400pt, width: 200pt)
#for i in range(5) {
  place(top + right, float: true, wrap: true, dy: i * 70pt,
    rect(width: 40pt, height: 30pt, fill: color.mix((aqua, i * 20%))))
}
#lorem(200)

// --- wrap-float-iteration-stress ---
// Forces multiple iterations of the refinement loop
#set page(height: 200pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 80pt, height: 100pt, fill: aqua))
#text(size: 8pt)[Small text]
#text(size: 16pt)[Big text]
#text(size: 8pt)[Small again]
#text(size: 16pt)[Big again]
#lorem(40)
```

### 6.7 Backward Compatibility Tests

Verify existing documents are unchanged:

```bash
# Run full test suite, should have zero regressions
cargo test --package typst-tests

# Specifically check existing float tests
cargo test --package typst-tests -- place
```

---

## Part 7: Performance Benchmarks

### 7.1 Benchmark Documents

Create benchmark documents in `benches/`:

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
// Same content, no wrap-floats (baseline)
#set page(height: 800pt, width: 600pt)
#lorem(1000)
```

### 7.2 Acceptance Criteria

| Scenario | Max Regression |
|----------|----------------|
| No wrap-floats (baseline) | 0% (must be identical) |
| Single wrap-float | < 20% |
| Multiple wrap-floats | < 50% |
| Long paragraph with wrap | < 100% |

### 7.3 Profiling Points

Instrument these functions for performance analysis:

```rust
#[typst_macros::time(name = "wrap-float-measure")]
fn measure_par_impl(...) { ... }

#[typst_macros::time(name = "wrap-float-linebreak")]
fn linebreak_variable(...) { ... }

#[typst_macros::time(name = "wrap-float-refine")]
fn refine_paragraph_measure(...) { ... }
```

---

## Part 8: Implementation Sequence (Revised)

### Phase 0: Prerequisite Refactoring
**Goal:** Separate Preparation creation from line breaking

1. Add `prepare_par()`, `break_lines()`, `measure_lines()`, `finalize_lines()`
2. Refactor `layout_par()` to use new API
3. Add `LineMetrics` struct
4. Verify all existing tests pass

**Exit criteria:**
- [ ] All paragraph tests pass unchanged
- [ ] New API produces identical output
- [ ] No performance regression > 5%

**Test files:** `tests/suite/layout/inline/prepare-split.typ`

### Phase 1: ParChild Structure
**Goal:** Deferred paragraph layout in flow

1. Add `ParChild` struct to `collect.rs`
2. Add `ParMeasureResult`, `ParCommitResult`, `LineHeights`
3. Update `Child` enum with `Par` variant
4. Update `Collector::par()` to create `ParChild`
5. Add `measure_par_impl` (memoized)
6. Implement locator handling for measure/commit

**Exit criteria:**
- [ ] `Child::Par` layout matches `Child::Line` output
- [ ] Introspection locations are stable
- [ ] Golden tests pass

**Test files:** `tests/suite/layout/flow/par-child.typ`

### Phase 2: Exclusion Data Structures
**Goal:** Foundation for wrap geometry

1. Add `ParExclusions`, `ExclusionZone` to `regions.rs`
2. Add `WrapFloat` struct
3. Add coordinate system documentation
4. Unit tests for exclusion computation

**Exit criteria:**
- [ ] `ParExclusions::available_width()` works correctly
- [ ] Coordinate transforms are documented
- [ ] Unit tests pass

### Phase 3a: Wrap Parameter
**Goal:** Add `wrap` parameter with no behavior change

1. Add `wrap: bool` to `PlaceElem`
2. Parse and store in `PlacedChild`
3. No behavior change yet (wrap=true same as wrap=false)

**Exit criteria:**
- [ ] Parameter parses correctly
- [ ] No behavior change for existing documents

### Phase 3b: WrapFloatChild Collection
**Goal:** Separate collection path for wrap-floats

1. Add `WrapFloatChild` struct
2. Update `Collector::place()` to create `Child::WrapFloat`
3. Wrap-floats still behave as normal floats (no exclusions yet)

**Exit criteria:**
- [ ] Wrap-floats render at correct positions
- [ ] No pagination changes

### Phase 3c: WrapState and Exclusion Integration
**Goal:** Wrap-floats affect paragraph layout

1. Add `WrapState` to `Distributor`
2. Implement `Distributor::wrap_float()`
3. Implement `Distributor::par()` with exclusion support
4. Add iterative refinement for height-dependent exclusions

**Exit criteria:**
- [ ] Text wraps around wrap-floats
- [ ] Iteration converges within 3 passes
- [ ] Edge cases handled (too wide, narrow gap)

**Test files:** `tests/suite/layout/place/wrap-float-basic.typ`

### Phase 4: Variable-Width Knuth-Plass
**Goal:** Quality line breaking with exclusions

1. Add `linebreak_variable_width()`
2. Implement `linebreak_with_exclusions()` (iterative)
3. Implement `linebreak_variable()` (per-line widths)
4. Add guardrails and fallbacks
5. Disable active-set pruning for variable widths

**Exit criteria:**
- [ ] K-P produces good line breaks with exclusions
- [ ] Guardrails trigger on complex cases
- [ ] Performance acceptable

**Test files:** `tests/suite/layout/place/wrap-float-stress.typ`

### Phase 5: Edge Cases and Polish
**Goal:** Production-ready wrap-floats

1. Handle page breaks correctly
2. Handle column layouts
3. Handle footnotes with wrap-floats
4. Add RTL/BiDi support
5. Add user-visible warnings

**Exit criteria:**
- [ ] All edge case tests pass
- [ ] Warnings emitted for fallback cases
- [ ] RTL works correctly

**Test files:** `tests/suite/layout/place/wrap-float-edge.typ`

### Phase 6: Documentation and Release
**Goal:** Ship wrap-floats

1. Add user documentation
2. Add examples to docs
3. Performance benchmarks pass acceptance criteria
4. Remove feature flag (if used)

**Exit criteria:**
- [ ] Full test suite passes
- [ ] Documentation complete
- [ ] Performance benchmarks pass

---

## Part 9: Files to Modify

| File | Changes |
|------|---------|
| `crates/typst-layout/src/inline/mod.rs` | Add `prepare_par`, `break_lines`, `measure_lines`, `finalize_lines`, `measure_par_with_exclusions`, `commit_par` |
| `crates/typst-layout/src/inline/linebreak.rs` | Add `linebreak_variable_width`, `linebreak_with_exclusions`, `linebreak_variable`, modify K-P for variable widths |
| `crates/typst-layout/src/flow/collect.rs` | Add `ParChild`, `WrapFloatChild`, `ParMeasureResult`, `ParCommitResult`, update `Child` enum, modify `par()`, `place()` |
| `crates/typst-layout/src/flow/distribute.rs` | Add `WrapState`, `Item::WrapFloat`, `Distributor::par()`, `Distributor::wrap_float()`, modify `finalize()` |
| `crates/typst-library/src/layout/regions.rs` | Add `ParExclusions`, `ExclusionZone`, `WrapFloat` |
| `crates/typst-library/src/layout/place.rs` | Add `wrap` parameter to `PlaceElem` |

---

## Part 10: Risk Assessment

### Critical Risks

1. **Locator stability:** Measure/commit must produce consistent locations
   - Mitigation: Use `relayout()` for commit, add debug assertions

2. **K-P convergence:** Variable-width K-P may not converge
   - Mitigation: Iteration limit, simple-breaking fallback, warnings

3. **Performance regression:** Two-phase layout is inherently slower
   - Mitigation: Memoization, skip refinement when no exclusions

4. **Backward compatibility:** Must not change existing float behavior
   - Mitigation: Extensive test suite, wrap defaults to false

### Medium Risks

1. **Complex exclusion shapes:** May produce poor layouts
   - Mitigation: Guardrails, simple-breaking fallback

2. **Interaction with footnotes:** Could cause relayout loops
   - Mitigation: Test thoroughly, document limitations

3. **Column layout complexity:** Parent-scope wrap-floats across columns
   - Mitigation: Clear coordinate system, targeted tests

### Low Risks

1. **RTL/BiDi edge cases:** May need iteration
2. **User confusion:** Wrap vs non-wrap float semantics
3. **Documentation gaps:** Need clear examples

---

## Appendix A: Glossary

- **Wrap-float:** A floating element with `wrap: true` that text flows around
- **Exclusion zone:** A rectangular region where text cannot be placed
- **Inner flow origin:** The coordinate system origin (y=0) at the top of the content region, below top insertions
- **Measure phase:** Computing line breaks and metrics without creating frames
- **Commit phase:** Creating actual frames from measured line breaks
- **Refinement iteration:** Re-measuring a paragraph when exclusions change due to line height changes
