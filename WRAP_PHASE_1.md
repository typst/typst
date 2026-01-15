# Phase 1: ParChild Structure

**Goal:** Implement deferred paragraph layout in the flow system with measure/commit API.

## Overview

This phase introduces `ParChild`, a new flow child type that stores paragraph
data for deferred layout. Unlike the current approach where paragraphs become
`LineChild` frames at collection time, `ParChild` delays layout until
distribution when y-positions and exclusions are known.

## The ParChild Type

**File: `crates/typst-layout/src/flow/collect.rs`**

*Find insertion point:* Search for `pub struct LineChild` - add `ParChild` near it.
Or search for `pub enum Child` to find the enum that needs updating.

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
```

## Measure and Commit Results

```rust
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
```

## ParChild Methods

```rust
impl<'a> ParChild<'a> {
    /// Measure the paragraph with optional width exclusions.
    ///
    /// Performs line breaking and computes metrics but does NOT create frames.
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
    /// Uses `locator.relayout()` to ensure introspection locations are stable.
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
}
```

## Locator Handling Strategy

The locator problem is critical: introspection (`location()`, `query()`) must
return consistent results regardless of how many times we measure.

**Solution: Two-tier locator usage**

1. **Measure phase:** Use `self.locator.track()` for cache key computation.
   This ensures the same paragraph always gets the same cache entry.

2. **Commit phase:** Use `self.locator.relayout()` for frame creation.
   This creates a fresh locator chain that produces stable locations.

**Invariant:** A paragraph's introspection-visible location is determined by
its position in the source, not by how many times it was measured.

**Validation (debug builds):**
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

## Update Child Enum

**File: `crates/typst-layout/src/flow/collect.rs`**

*Find with:* `grep "pub enum Child" crates/typst-layout/src/flow/collect.rs`

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

## Update Collector::par()

**File: `crates/typst-layout/src/flow/collect.rs`**

*Find with:* `grep -n "fn par\(" crates/typst-layout/src/flow/collect.rs`

Replace the `par` method:

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

## Memoized Measurement Implementation

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
```

## Exit Criteria

- [ ] `Child::Par` layout matches `Child::Line` output for existing documents
- [ ] Introspection locations are stable across measure/commit cycles
- [ ] Golden tests pass with no visual differences
- [ ] Widow/orphan prevention works correctly with `ParChild`

## Test Plan

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

## Dependencies

- [Phase 0: Prerequisite Refactoring](WRAP_PHASE_0.md) must be complete

## Next Phase

[Phase 2: Exclusion Data Structures](WRAP_PHASE_2.md)
