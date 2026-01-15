# Phase 0: Prerequisite Refactoring

**Goal:** Enable paragraph measurement without frame creation, supporting variable-width line breaking.

## The Actual Problem

Looking at the current code in `crates/typst-layout/src/inline/mod.rs`:

```rust
fn layout_inline_impl<'a>(...) -> SourceResult<Fragment> {
    let config = configuration(...);
    let (text, segments, spans) = collect(...)?;   // Produces owned String
    let p = prepare(engine, &config, &text, ...)?; // Borrows &str, &Config
    let lines = linebreak(engine, &p, width);      // Borrows &Preparation
    finalize(engine, &p, &lines, ...)              // Creates frames
}
```

The `Preparation<'a>` struct borrows from local variables:
- `text: &'a str` - borrows from the `String` created by `collect()`
- `config: &'a Config` - borrows from the `Config` created locally
- `bidi: Option<BidiInfo<'a>>` - borrows from `text`

**This is why we can't just "return Preparation from a function"** - it would be returning references to dropped locals.

## The Solution: Don't Store Preparation

The key insight: **we don't need to store `Preparation` across calls**. Instead:

1. Create a new memoized function `measure_par_with_exclusions` that:
   - Takes exclusions as a parameter
   - Does collect → prepare → linebreak → metrics internally
   - Returns owned `ParMeasureResult` (no borrows)

2. Create `commit_par` that:
   - Re-does collect → prepare (cheap due to memoization)
   - Uses stored break positions to reconstruct lines
   - Calls finalize to create frames

3. Rely on **comemo's caching** to make the re-preparation cheap:
   - The expensive work is in `shape_range()` inside `prepare()`
   - Comemo caches at the `layout_par_impl` level
   - With same inputs, shaping results are cached

## Implementation Strategy

### Step 1: Add `ParMeasureResult` and `ParCommitResult`

**File: `crates/typst-layout/src/inline/mod.rs`**

*Find insertion point:* Search for `pub fn layout_par` - add types above it,
or search for `pub enum ParSituation` and add near it.

```rust
/// Result of measuring a paragraph without creating frames.
#[derive(Debug, Clone, Hash)]
pub struct ParMeasureResult {
    /// Per-line metrics.
    pub line_metrics: Vec<LineMetrics>,
    /// Total height including leading.
    pub total_height: Abs,
    /// Break positions as byte offsets into the text.
    /// Used to reconstruct lines in commit phase.
    pub break_points: Vec<BreakInfo>,
}

/// Information about a single line break.
#[derive(Debug, Clone, Hash)]
pub struct BreakInfo {
    /// End byte offset of this line.
    pub end: usize,
    /// The breakpoint type.
    pub breakpoint: SerializableBreakpoint,
}

/// Breakpoint that can be hashed (for memoization).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SerializableBreakpoint {
    Normal,
    Mandatory,
    Hyphen(u8, u8),
}

impl From<Breakpoint> for SerializableBreakpoint {
    fn from(bp: Breakpoint) -> Self {
        match bp {
            Breakpoint::Normal => Self::Normal,
            Breakpoint::Mandatory => Self::Mandatory,
            Breakpoint::Hyphen(a, b) => Self::Hyphen(a, b),
        }
    }
}

impl From<SerializableBreakpoint> for Breakpoint {
    fn from(bp: SerializableBreakpoint) -> Self {
        match bp {
            SerializableBreakpoint::Normal => Self::Normal,
            SerializableBreakpoint::Mandatory => Self::Mandatory,
            SerializableBreakpoint::Hyphen(a, b) => Self::Hyphen(a, b),
        }
    }
}

/// Metrics for a single line.
#[derive(Debug, Clone, Copy, Hash)]
pub struct LineMetrics {
    pub width: Abs,
    pub height: Abs,
    pub ascent: Abs,
    pub descent: Abs,
}

/// Result of committing a measured paragraph.
pub struct ParCommitResult {
    pub frames: Vec<Frame>,
}
```

### Step 2: Add the Measurement Function

**File: `crates/typst-layout/src/inline/mod.rs`**

```rust
use typst_library::layout::ParExclusions;  // From Phase 2

/// Measure a paragraph with optional exclusion zones.
///
/// This performs line breaking and computes metrics, but does NOT create frames.
/// The result can be used to determine layout positions, then committed later.
pub fn measure_par_with_exclusions(
    elem: &Packed<ParElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
    exclusions: Option<ParExclusions>,
) -> SourceResult<ParMeasureResult> {
    measure_par_impl(
        elem,
        engine.routines,
        engine.world,
        engine.introspector.into_raw(),
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        locator.track(),
        styles,
        region,
        expand,
        situation,
        exclusions,
    )
}

/// The internal, memoized implementation.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn measure_par_impl(
    elem: &Packed<ParElem>,
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    locator: Tracked<Locator>,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
    exclusions: Option<ParExclusions>,
) -> SourceResult<ParMeasureResult> {
    let introspector = Protected::from_raw(introspector);
    let link = LocatorLink::new(locator);
    let mut locator = Locator::link(&link).split();
    let mut engine = Engine {
        routines,
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route),
    };

    let arenas = Arenas::default();
    let children = (engine.routines.realize)(
        RealizationKind::LayoutPar,
        &mut engine,
        &mut locator,
        &arenas,
        &elem.body,
        styles,
    )?;

    measure_inline_impl(
        &mut engine,
        &children,
        styles,
        region,
        expand,
        Some(situation),
        exclusions,
        &ConfigBase {
            justify: elem.justify.get(styles),
            linebreaks: elem.linebreaks.get(styles),
            first_line_indent: elem.first_line_indent.get(styles),
            hanging_indent: elem.hanging_indent.resolve(styles),
        },
    )
}

/// Internal measurement implementation.
fn measure_inline_impl<'a>(
    engine: &mut Engine,
    children: &[Pair<'a>],
    shared: StyleChain<'a>,
    region: Size,
    expand: bool,
    par: Option<ParSituation>,
    exclusions: Option<ParExclusions>,
    base: &ConfigBase,
) -> SourceResult<ParMeasureResult> {
    let config = configuration(base, children, shared, par);
    let (text, segments, spans) = collect(children, engine, &mut dummy_locator(), &config, region)?;
    let p = prepare(engine, &config, &text, segments, spans)?;

    // Choose line breaking strategy based on exclusions
    let lines = if let Some(ref excl) = exclusions {
        // Variable-width line breaking (Phase 4)
        linebreak_variable_width(engine, &p, region.x - config.hanging_indent, Some(excl))
    } else {
        // Standard line breaking
        linebreak(engine, &p, region.x - config.hanging_indent)
    };

    // Extract metrics without creating frames
    let leading = shared.resolve(ParElem::leading);
    let mut total_height = Abs::zero();
    let mut line_metrics = Vec::with_capacity(lines.len());
    let mut break_points = Vec::with_capacity(lines.len());

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            total_height += leading;
        }

        let metrics = LineMetrics {
            width: line.width,
            height: line.height(),
            ascent: line.ascent(),
            descent: line.descent(),
        };
        total_height += metrics.height;
        line_metrics.push(metrics);

        break_points.push(BreakInfo {
            end: line.end,
            breakpoint: line.breakpoint.into(),
        });
    }

    Ok(ParMeasureResult {
        line_metrics,
        total_height,
        break_points,
    })
}

// Temporary: dummy locator for measurement pass
// In real implementation, need to handle this properly
fn dummy_locator() -> SplitLocator<'static> {
    // TODO: This needs proper implementation
    unimplemented!("Need to handle locator in measure pass")
}
```

### Step 3: The Locator Problem

**This is the tricky part.** The `collect()` function takes a `&mut SplitLocator` and uses it to track locations of inline elements. During measurement, we don't want to "consume" locator slots because we'll need them again during commit.

**Solution options:**

**Option A: Phantom locator for measurement**
```rust
// During measure: use a "phantom" locator that tracks but doesn't commit
// During commit: use the real locator

// This requires adding a "phantom" mode to SplitLocator or creating a wrapper
```

**Option B: Re-collect during commit**
```rust
// During measure: collect with real locator, but don't finalize
// During commit: re-collect (comemo caches it), then finalize

// Simpler but slightly wasteful
```

**Option C: Store locator state**
```rust
// During measure: record which locator slots were used
// During commit: replay the same slots

// Complex but precise
```

**Recommended: Option B** - Re-collect during commit. The `collect()` call is not expensive compared to shaping, and comemo will cache the realization step. This keeps the code simple.

### Step 4: Add Commit Function

```rust
/// Commit a measured paragraph into frames.
///
/// This should be called with the same inputs as the corresponding
/// `measure_par_with_exclusions` call.
pub fn commit_par(
    elem: &Packed<ParElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
    exclusions: Option<&ParExclusions>,
    measured: &ParMeasureResult,
) -> SourceResult<ParCommitResult> {
    // Re-create the internal state (comemo caches the expensive parts)
    let arenas = Arenas::default();
    let mut split_locator = locator.split();

    let children = (engine.routines.realize)(
        RealizationKind::LayoutPar,
        engine,
        &mut split_locator,
        &arenas,
        &elem.body,
        styles,
    )?;

    let base = ConfigBase {
        justify: elem.justify.get(styles),
        linebreaks: elem.linebreaks.get(styles),
        first_line_indent: elem.first_line_indent.get(styles),
        hanging_indent: elem.hanging_indent.resolve(styles),
    };
    let config = configuration(&base, &children, styles, Some(situation));

    let (text, segments, spans) = collect(&children, engine, &mut split_locator, &config, region)?;
    let p = prepare(engine, &config, &text, segments, spans)?;

    // Reconstruct lines from stored break points
    let lines = reconstruct_lines(engine, &p, &measured.break_points);

    // Create frames
    let fragment = finalize(engine, &p, &lines, region, expand, &mut split_locator)?;

    Ok(ParCommitResult {
        frames: fragment.into_frames(),
    })
}

/// Reconstruct Line objects from stored break information.
fn reconstruct_lines<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    break_points: &[BreakInfo],
) -> Vec<Line<'a>> {
    let mut lines = Vec::with_capacity(break_points.len());
    let mut start = 0;

    for (i, info) in break_points.iter().enumerate() {
        let prev_line = if i > 0 { lines.last() } else { None };
        let l = line(engine, p, start..info.end, info.breakpoint.into(), prev_line);
        lines.push(l);
        start = info.end;
    }

    lines
}
```

### Step 5: Add Variable-Width Linebreak Stub

**File: `crates/typst-layout/src/inline/linebreak.rs`**

*Find insertion point:* Search for `pub fn linebreak<'a>` - add new function near it.

```rust
use typst_library::layout::ParExclusions;

/// Line breaking with variable widths for wrap-float support.
///
/// If exclusions is None or empty, delegates to standard linebreak().
/// Otherwise, uses iterative refinement with per-line widths.
pub fn linebreak_variable_width<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    base_width: Abs,
    exclusions: Option<&ParExclusions>,
) -> Vec<Line<'a>> {
    match exclusions {
        None => linebreak(engine, p, base_width),
        Some(excl) if excl.is_empty() => linebreak(engine, p, base_width),
        Some(excl) => {
            // Phase 4 will implement this properly
            // For now, fall back to simple breaking with minimum width
            let min_width = compute_min_width(base_width, excl);
            linebreak(engine, p, min_width)
        }
    }
}

fn compute_min_width(base_width: Abs, exclusions: &ParExclusions) -> Abs {
    // Conservative: use the minimum available width across all y positions
    // Phase 4 will do this properly with per-line widths
    let mut y = Abs::zero();
    let mut min = base_width;
    let step = Abs::pt(12.0); // Approximate line height

    for _ in 0..100 {
        min = min.min(exclusions.available_width(base_width, y));
        y += step;
    }

    min.max(Abs::pt(20.0)) // Minimum reasonable width
}
```

### Step 6: Update Existing `layout_par` to Use New Functions

```rust
/// Layouts the paragraph (existing API, unchanged behavior).
pub fn layout_par(
    elem: &Packed<ParElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
) -> SourceResult<Fragment> {
    // Measure without exclusions
    let measured = measure_par_with_exclusions(
        elem, engine, locator.clone(), styles, region, expand, situation, None
    )?;

    // Commit immediately
    let result = commit_par(
        elem, engine, locator, styles, region, expand, situation, None, &measured
    )?;

    Ok(Fragment::frames(result.frames))
}
```

**Wait - this changes the memoization behavior!** The existing `layout_par_impl` is memoized as a unit. Splitting it into measure + commit means we need to ensure the same caching behavior.

**Better approach:** Keep `layout_par` using the existing `layout_par_impl` for backward compatibility. Only use the new measure/commit path when exclusions are involved.

```rust
/// Layouts the paragraph.
pub fn layout_par(...) -> SourceResult<Fragment> {
    // Keep using the original memoized implementation for non-wrap case
    layout_par_impl(...)
}

// The new functions are ONLY used by the wrap-float code path
pub fn measure_par_with_exclusions(...) -> SourceResult<ParMeasureResult> { ... }
pub fn commit_par(...) -> SourceResult<ParCommitResult> { ... }
```

## Line Struct Modification Required

**Important discovery:** The current `Line` struct does NOT store `end` or `breakpoint`. These are parameters passed to the `line()` function but not stored in the result:

```rust
// Find with: grep "pub struct Line<'a>" crates/typst-layout/src/inline/line.rs
pub struct Line<'a> {
    pub items: Items<'a>,
    pub width: Abs,
    pub justify: bool,
    pub dash: Option<Dash>,
    // NO end or breakpoint fields!
}

// Find with: grep "^pub fn line<'a>" crates/typst-layout/src/inline/line.rs
pub fn line<'a>(
    engine: &Engine,
    p: &'a Preparation,
    range: Range,        // Contains start..end
    breakpoint: Breakpoint,  // Passed in but not stored
    pred: Option<&Line<'a>>,
) -> Line<'a>
```

**Required change:** Add `end` and `breakpoint` fields to `Line`:

```rust
pub struct Line<'a> {
    pub items: Items<'a>,
    pub width: Abs,
    pub justify: bool,
    pub dash: Option<Dash>,
    // NEW FIELDS:
    pub end: usize,           // End byte offset in text
    pub breakpoint: Breakpoint,  // How this line ends
}
```

Then update `line()` to store these:
```rust
pub fn line<'a>(..., range: Range, breakpoint: Breakpoint, ...) -> Line<'a> {
    // ... existing code ...
    Line {
        items,
        width,
        justify,
        dash,
        end: range.end,      // NEW
        breakpoint,          // NEW
    }
}
```

And update `Line::empty()`:
```rust
pub fn empty() -> Self {
    Self {
        items: Items::new(),
        width: Abs::zero(),
        justify: false,
        dash: None,
        end: 0,                          // NEW
        breakpoint: Breakpoint::Mandatory,  // NEW
    }
}
```

**Why this matters:** Without these fields, we cannot reconstruct lines from stored break positions. The measurement phase needs to record where breaks occurred, and the commit phase needs to recreate identical lines.

## Exit Criteria

- [ ] **Line struct modified:** `end: usize` and `breakpoint: Breakpoint` fields added to `Line<'a>`
- [ ] **Line function updated:** `line()` stores the new fields, `Line::empty()` initializes them
- [ ] `ParMeasureResult` struct defined and hashable
- [ ] `measure_par_with_exclusions` returns metrics without creating frames
- [ ] `commit_par` creates frames from measurement result
- [ ] `linebreak_variable_width` stub exists (full implementation in Phase 4)
- [ ] Existing `layout_par` behavior unchanged (same test output)
- [ ] All existing inline/paragraph tests pass with Line struct changes

## Test Plan

**File: `tests/suite/layout/inline/measure-commit.typ`**

```typst
// Verify measure+commit produces same output as direct layout

// --- measure-commit-basic ---
#set page(width: 200pt, height: auto)
#lorem(30)

// --- measure-commit-justified ---
#set page(width: 200pt, height: auto)
#set par(justify: true)
#lorem(30)

// --- measure-commit-hyphenation ---
#set page(width: 100pt, height: auto)
#set text(hyphenate: true)
Incomprehensibilities and counterrevolutionaries.

// --- measure-commit-mixed ---
#set page(width: 200pt, height: auto)
Normal #text(size: 20pt)[BIG] normal #text(size: 8pt)[small] end.
```

## What Phase 1 Needs From This

Phase 1's `ParChild` will call:
```rust
// In ParChild::measure()
measure_par_with_exclusions(self.elem, engine, self.locator, ...)

// In ParChild::commit()
commit_par(self.elem, engine, self.locator.relayout(), ...)
```

The functions must:
1. Accept `Option<ParExclusions>` (from Phase 2)
2. Return owned results (no lifetime issues)
3. Be memoized appropriately

## Dependencies

- None (this is the first phase)
- Phase 2 types (`ParExclusions`) can be stubbed initially

## Risks

1. **Locator handling:** The measure pass shouldn't permanently consume locator slots. Solution: re-collect during commit.

2. **Line reconstruction:** We store break points and reconstruct `Line` objects. If `line()` doesn't produce identical results, output differs. Mitigation: use the same `line()` function.

3. **Memoization changes:** Don't break existing caching. Solution: keep `layout_par` using original path.

## Next Phase

[Phase 1: ParChild Structure](WRAP_PHASE_1.md)
