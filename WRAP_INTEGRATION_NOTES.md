# Wrap-Float Integration Notes

**Purpose:** This document defines the contracts between phases. Each phase agent
should verify their inputs match what the previous phase produces, and their
outputs match what the next phase expects.

**Critical rule:** Don't trust line numbers in any docs. Search for actual
code patterns instead.

---

## Before You Start: Verify Baseline

Run this before making any changes:
```bash
cd /Users/kevin/Documents/newstart/typst/typst
cargo build --package typst-layout
cargo test --package typst-tests -- paragraph --test-threads=1
```

If this fails, STOP. Fix the baseline first or ask for help.

---

## Recommended Phase Sequence

**Do NOT run all phases in parallel.** Dependencies matter.

```
Week 1: Phase 0 (alone, verify tests pass after)
        Phase 2 (can run in parallel - just data structures)

Week 2: Phase 1 (needs Phase 0 done)
        Phase 3a, 3b (plumbing, needs Phase 2)

Week 3: Phase 3c (integration - needs Phase 1, 2, 3a/3b)
        Phase 4 (can partially parallel with 3c)

Week 4: Phase 5, Phase 6
```

**After each phase: run the test suite.** Don't proceed if tests fail.

---

## How to Find Code Locations

Line numbers become stale. Use these search patterns instead:

| To find... | Search command |
|------------|----------------|
| `Line` struct | `grep "pub struct Line<'a>" crates/typst-layout/src/inline/line.rs` |
| `line()` function | `grep "^pub fn line<'a>" crates/typst-layout/src/inline/line.rs` |
| `Preparation` struct | `grep "pub struct Preparation" crates/typst-layout/src/inline/prepare.rs` |
| `prepare()` function | `grep "^pub fn prepare" crates/typst-layout/src/inline/prepare.rs` |
| `linebreak()` function | `grep "^pub fn linebreak" crates/typst-layout/src/inline/linebreak.rs` |
| `Child` enum | `grep "pub enum Child" crates/typst-layout/src/flow/collect.rs` |
| `Collector::par` method | `grep "fn par\(" crates/typst-layout/src/flow/collect.rs` |
| `PlaceElem` struct | `grep "pub struct PlaceElem" crates/typst-library/src/layout/place.rs` |
| `layout_par` function | `grep "^pub fn layout_par" crates/typst-layout/src/inline/mod.rs` |
| `layout_inline_impl` | `grep "^fn layout_inline_impl" crates/typst-layout/src/inline/mod.rs` |
| `Distributor` struct | `grep "struct Distributor" crates/typst-layout/src/flow/distribute.rs` |

**General tips:**
- Use `grep -n` to get line numbers for current code
- Use `grep -A 10` to see context after match
- Search for `pub struct X` or `pub fn x` for definitions
- Search for `impl X` for method implementations

---

## Phase Dependency Graph

```
Phase 0 ──→ Phase 1 ──→ Phase 3c
              ↑            ↑
          Phase 2 ────────┘
              ↑
          Phase 3a,3b
              ↑
          Phase 4 (consumes exclusions from 3c, provides lines back)
```

---

## Boundary 0→1: Inline API for ParChild

**Phase 0 produces** (in `crates/typst-layout/src/inline/mod.rs`):

```rust
// Phase 1 will call these functions from ParChild::measure() and ParChild::commit()

/// Measure a paragraph with optional exclusions. Returns metrics, no frames.
pub fn measure_par_with_exclusions(
    elem: &Packed<ParElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
    expand: bool,
    situation: ParSituation,
    exclusions: Option<ParExclusions>,  // From Phase 2
) -> SourceResult<ParMeasureResult>;    // Defined in Phase 1

/// Commit measured paragraph to frames.
pub fn commit_par(
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
) -> SourceResult<ParCommitResult>;     // Defined in Phase 1
```

**Phase 1 expects:** These two functions to exist and work. If Phase 0 changes
the signature, Phase 1 breaks.

**Verification:** Phase 1 agent should grep for `measure_par_with_exclusions`
and `commit_par` to confirm they exist with compatible signatures.

---

## Boundary 1→3: ParChild in Distribution

**Phase 1 produces** (in `crates/typst-layout/src/flow/collect.rs`):

```rust
pub struct ParChild<'a> {
    pub elem: &'a Packed<ParElem>,
    pub styles: StyleChain<'a>,
    pub locator: Locator<'a>,
    pub expand: bool,
    pub situation: ParSituation,
    pub spacing: Abs,
    pub leading: Abs,
    pub align: Axes<FixedAlignment>,
    pub costs: Costs,
}

impl<'a> ParChild<'a> {
    pub fn measure(
        &self,
        engine: &mut Engine,
        region: Size,
        exclusions: Option<&ParExclusions>,  // From Phase 2
    ) -> SourceResult<ParMeasureResult>;

    pub fn commit(
        &self,
        engine: &mut Engine,
        measured: &ParMeasureResult,
        region: Size,
        exclusions: Option<&ParExclusions>,
    ) -> SourceResult<ParCommitResult>;
}

pub struct ParMeasureResult {
    pub metrics: Vec<LineMetrics>,
    pub total_height: Abs,
    pub line_heights: LineHeights,
    pub break_positions: Vec<usize>,  // For convergence check
    pub input_hash: u128,             // For cache validation
}

pub struct ParCommitResult {
    pub frames: Vec<Frame>,
    pub needs: Vec<Abs>,  // For widow/orphan
}
```

**Phase 3 expects:** `Child::Par(ParChild)` variant in the `Child` enum, and
the measure/commit methods to accept `Option<&ParExclusions>`.

**Verification:** Phase 3 agent should confirm `Child::Par` exists and
`ParChild::measure` accepts exclusions.

---

## Boundary 2→1,3: Exclusion Types

**Phase 2 produces** (in `crates/typst-library/src/layout/regions.rs`):

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ParExclusions {
    pub zones: Vec<ExclusionZone>,
}

impl ParExclusions {
    pub fn is_empty(&self) -> bool;
    pub fn from_wrap_floats(par_y: Abs, par_height: Abs, wrap_floats: &[WrapFloat]) -> Self;
    pub fn available_width(&self, base_width: Abs, y: Abs) -> Abs;
    pub fn left_offset(&self, y: Abs) -> Abs;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExclusionZone {
    pub y_start: i64,  // Raw Abs units, paragraph-relative
    pub y_end: i64,
    pub left: i64,
    pub right: i64,
}

#[derive(Debug, Clone)]
pub struct WrapFloat {
    pub y: Abs,           // Region-relative
    pub height: Abs,
    pub left_margin: Abs,
    pub right_margin: Abs,
}
```

**Phase 1 expects:** `ParExclusions` to exist for the measure/commit signatures.

**Phase 3 expects:** `WrapFloat` and `ParExclusions::from_wrap_floats()` for
building exclusions in the distributor.

**Phase 4 expects:** `ParExclusions::available_width()` for the linebreaker.

**Verification:** All downstream phases should confirm these types are exported
from `typst_library::layout`.

---

## Boundary 3→4: Exclusions to Linebreaker

**Phase 3 passes** (via Phase 1's measure call):

```rust
// In Distributor::par(), Phase 3 calls:
let measure = par.measure(engine, region, Some(&exclusions))?;

// Which eventually calls (in Phase 0's inline code):
linebreak_variable_width(engine, &prepared.p, base_width, Some(&exclusions))
```

**Phase 4 produces** (in `crates/typst-layout/src/inline/linebreak.rs`):

```rust
pub fn linebreak_variable_width<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    base_width: Abs,
    exclusions: Option<&ParExclusions>,
) -> Vec<Line<'a>>;
```

**Integration point:** Phase 0's `measure_par_with_exclusions` must call
Phase 4's `linebreak_variable_width`. This is the trickiest integration:

```rust
// In Phase 0's measure_par_with_exclusions:
let lines = if let Some(excl) = exclusions {
    linebreak_variable_width(engine, &p, width, Some(excl))
} else {
    linebreak(engine, &p, width)  // Existing function
};
```

**Verification:** Phase 4 agent should confirm Phase 0 has a call site ready
for `linebreak_variable_width`, or coordinate with Phase 0 to add it.

---

## Boundary 3→3: WrapState Internal

**Phase 3 internal contract** (within `distribute.rs`):

```rust
struct WrapState {
    floats: Vec<WrapFloat>,
}

impl WrapState {
    fn add(&mut self, wf: WrapFloat);
    fn exclusions_for(&self, par_y: Abs, par_height: Abs) -> Option<ParExclusions>;
    fn clear(&mut self);
}
```

**Invariant:** `WrapState::exclusions_for` must call `ParExclusions::from_wrap_floats`
from Phase 2. If Phase 2 changes that function signature, Phase 3 breaks.

---

## Data Flow Summary

```
User writes: #place(float: true, wrap: true, ...)
                          │
                          ▼
              ┌─────────────────────┐
              │ Collector::place()  │  Phase 3a/3b
              │ Creates WrapFloatChild │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ Distributor::wrap_float() │  Phase 3c
              │ Layouts float, creates WrapFloat │
              │ Adds to WrapState   │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ Distributor::par()  │  Phase 3c
              │ Calls WrapState::exclusions_for() │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ ParExclusions::from_wrap_floats() │  Phase 2
              │ Converts WrapFloat → ExclusionZone │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ ParChild::measure() │  Phase 1
              │ Passes exclusions to inline │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ measure_par_with_exclusions() │  Phase 0
              │ Calls linebreak_variable_width │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ linebreak_variable_width() │  Phase 4
              │ Uses exclusions.available_width() │
              │ Returns Vec<Line>   │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ Back up the stack   │
              │ Lines → Metrics → Frames │
              └─────────────────────┘
```

---

## Critical Invariants

### 1. Coordinate Systems

- `WrapFloat.y` is in **region coordinates** (y=0 at inner flow origin)
- `ExclusionZone.y_start/y_end` is in **paragraph-relative coordinates**
- `ParExclusions::from_wrap_floats()` does the transform

**If any phase uses the wrong coordinate system, text will wrap at wrong positions.**

### 2. Locator Consistency

- `ParChild::measure()` uses `self.locator.track()` for cache keys
- `ParChild::commit()` uses `self.locator.relayout()` for frame creation
- **Same paragraph must produce same introspection locations regardless of
  how many times it was measured**

### 3. Convergence

The refinement loop in Phase 3c:
```
measure → get height → compute exclusions → re-measure → check if breaks changed
```

Must terminate. `break_positions` in `ParMeasureResult` is used for convergence
check. If Phase 1 doesn't populate this correctly, Phase 3c may infinite loop.

### 4. Memoization

`measure_par_impl` is memoized with `#[comemo::memoize]`. The cache key includes:
- `elem` (the paragraph)
- `locator` (position in document)
- `region` (available size)
- `exclusions` (the wrap zones)

**If exclusions change, the cache must miss.** Phase 2's `ParExclusions` must
implement `Hash` correctly.

---

## Suggested Integration Checkpoints

1. **After Phase 0:** Run full test suite. Zero regressions allowed.

2. **After Phase 1:** Verify `Child::Par` produces identical output to old
   `Child::Line` approach. Run paragraph-heavy test files.

3. **After Phase 2:** Unit tests for `ParExclusions` pass. Types compile.

4. **After Phase 3a+3b:** `wrap: true` parses, wrap-floats render (but don't
   affect text yet). No crashes.

5. **After Phase 3c:** Text actually wraps. This is the first time the full
   pipeline executes. **Expect bugs here.**

6. **After Phase 4:** Line breaking quality improves. Compare output with
   simple vs optimized breaking.

---

## BLOCKER: Phase 0 Locator Problem

Phase 0 has an `unimplemented!()` for the locator handling in measurement:

```rust
fn dummy_locator() -> SplitLocator<'static> {
    unimplemented!("Need to handle locator in measure pass")
}
```

**This is not optional. Phase 0 agent MUST solve this.** Options:
1. Use real locator in measure, use `locator.relayout()` in commit
2. Create a phantom/dry-run mode for SplitLocator
3. Skip locator-dependent work in measure, do it all in commit

Pick one and implement it. Do not leave `unimplemented!()`.

---

## Critical: Line Struct Modification

**Phase 0 must modify the `Line` struct** in `line.rs` to add `end` and `breakpoint` fields.

Currently:
```rust
pub struct Line<'a> {
    pub items: Items<'a>,
    pub width: Abs,
    pub justify: bool,
    pub dash: Option<Dash>,
    // NO end or breakpoint!
}
```

Required:
```rust
pub struct Line<'a> {
    pub items: Items<'a>,
    pub width: Abs,
    pub justify: bool,
    pub dash: Option<Dash>,
    pub end: usize,              // NEW - end byte offset
    pub breakpoint: Breakpoint,  // NEW - how line ends
}
```

**Without this change, line reconstruction in commit phase is impossible.**

---

## Questions Each Agent Should Ask

### Phase 0 Agent
- How do I handle the `Preparation` lifetime? (Answer: don't store it, re-create in commit)
- Does `measure_par_with_exclusions` need to exist, or can I modify `layout_par`? (Answer: add new function, keep existing one unchanged)
- Where does `linebreak_variable_width` get called? (Answer: from measure_inline_impl when exclusions present)
- **Critical:** Have I added `end` and `breakpoint` to the `Line` struct?

### Phase 1 Agent
- Does Phase 0's API exist yet? What are the actual function signatures?
- How do I integrate with the existing `Collector::par()` method?
- What's the current `Child` enum? Does `Child::Line` exist?

### Phase 2 Agent
- Where should `ParExclusions` live? `regions.rs` or somewhere else?
- Is `Abs::to_raw()` / `Abs::from_raw()` the right way to handle coordinates?
- Do I need to export these types from the library crate?

### Phase 3 Agent
- Does `ParChild` exist yet? What's its actual API?
- How does the current distributor handle paragraphs?
- Where's the region boundary handling for clearing wrap state?

### Phase 4 Agent
- What's the current `linebreak` function signature?
- Does `Preparation` have access to font size for height estimates?
- Can I modify `linebreak_optimized` or should I create a parallel function?

---

## If Something Doesn't Match

If your phase's expected inputs don't exist or have different signatures:

1. **Document the discrepancy** in your phase's output
2. **Create a stub** with the expected signature if possible
3. **Flag it for the integration pass**

Don't silently change the contract - that breaks downstream phases.
