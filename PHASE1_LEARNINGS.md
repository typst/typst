# Phase 1 Learnings: Deferred Paragraph Layout

This document captures hard-won insights from implementing and debugging Phase 1 of the wrap-float feature. Future agents should read this before working on related code.

## Related Documentation

- `WRAP_OVERVIEW.md` - High-level wrap-float feature plan
- `WRAP_PHASE_*.md` - Detailed phase plans
- `OLD_CITATION_FIX_HANDOFF.md` - Earlier agent's analysis (superseded by this doc)
- `OLD_CITATION_INTROSPECTION_DEEP_DIVE.md` - Deep technical analysis of citation/introspection (historical)

---

## Key Architectural Concepts

### 1. The Convergence Loop

Typst uses an iterative layout system:
1. Layout runs, elements get locations, introspector is empty
2. Elements are collected into introspector at end of pass
3. Next pass: introspection queries return data from previous pass
4. Repeat until stable (max 5 iterations)

**Critical insight:** Within a single pass, the introspector has STALE data from the previous pass. Newly created elements aren't in the introspector until the pass completes.

### 2. Memoization and Cache Keys

Many layout functions use `#[comemo::memoize]`. The cache key includes all `Tracked<T>` parameters.

**Pitfall:** If you call a memoized function with different parameters at different points in a pass, you get DIFFERENT cached results. This can cause inconsistency.

Example: `Works::generate_impl` is memoized with `groups: EcoVec<Content>`. If bibliography uses `groups=[A,B,C]` and a citation uses `groups=[X,Y,Z]`, they get different `Works` instances with potentially different disambiguation.

### 3. Content Copy-on-Write Semantics

`Content` uses reference counting with copy-on-write:
```rust
fn set_location(&mut self, location: Location) {
    self.0.meta_mut().location = Some(location);  // Calls make_unique()
}
```

**Pitfall:** If you clone Content BEFORE setting location, the clone has the OLD allocation (no location). Clone AFTER to share the location.

---

## The Citation System

### How Citations Work

1. `CiteElem` (@citation) is parsed
2. During realization, adjacent citations are grouped into `CiteGroup`
3. `CiteGroup` gets a location via `prepare()`
4. `CiteGroup::realize()` calls `Works::generate()` to get formatted citation
5. `Works::generate()` queries for ALL CiteGroups to compute disambiguation (n.d.-a, n.d.-b, etc.)
6. Citation is looked up by location in `Works.citations` map

### The Problem We Fixed

Phase 1 moved paragraph realization from memoized `layout_par_impl` to non-memoized `measure_par_with_exclusions`. This meant:
- CiteGroups created fresh each layout attempt (not cached)
- On first pass, introspector empty → Works can't find citations
- Error: "cannot format citation in isolation"

### The Solution: Citation Registry

```
crates/typst-library/src/model/bibliography.rs:
  - Thread-local CITE_GROUPS registry
  - register_cite_group() - called during prepare()
  - clear_cite_group_registry() - called at convergence loop start

crates/typst-realize/src/lib.rs:
  - Register CiteGroup after it gets location in prepare()

crates/typst/src/lib.rs:
  - Clear registry at start of each convergence iteration
```

### Critical: Merging vs Preferring

**Wrong approach:** Prefer registered groups over introspector
- Causes inconsistency: bibliography uses introspector (complete), citations use registered (partial)
- Result: incorrect disambiguation

**Correct approach:** Merge both sources
- Start with introspector groups (complete set for disambiguation)
- Add registered groups not already present (for current-pass lookups)
- Deduplicate by location

```rust
fn merged_cite_groups(engine: &mut Engine, span: Span) -> EcoVec<Content> {
    let registered = get_registered_cite_groups();
    let introspector_groups = engine.introspect(CiteGroupIntrospection(span));

    if registered.is_empty() { return introspector_groups; }
    if introspector_groups.is_empty() { return registered; }

    // Merge: introspector first, then registered not already present
    let existing: HashSet<_> = introspector_groups.iter()
        .filter_map(|g| g.location()).collect();

    let mut result = introspector_groups;
    for group in registered {
        if let Some(loc) = group.location() {
            if !existing.contains(&loc) {
                result.push(group);
            }
        }
    }
    result
}
```

---

## Footnote Migration

### How Footnotes Work

1. Content with footnote marker is laid out
2. `self.frame()` calls `self.composer.footnotes()`
3. Footnotes are extracted from frame and laid out
4. If footnote doesn't fit, migration may trigger: `Err(Stop::Finish(false))`
5. Content moves to next region, footnote follows

### The Footnote Invariant

Footnote marker and first line of entry must be on same page. If they can't both fit, the CONTENT migrates (not just the footnote).

### The Problem We Fixed

In `par_spill()`, when processing paragraph frames:
```rust
while let Some((frame, need)) = spill.frames.next() {
    // ... checks ...
    self.frame(frame, spill.align, false, false)?;  // Can return Err!
}
```

If `self.frame()` returns error (footnote migration), the frame is LOST:
- Frame consumed from iterator
- Error propagates up
- `par()` calls `advance()` to skip `Child::Par`
- Frame never processed in next region

### The Fix

Save frame on error:
```rust
if let Err(err) = self.frame(frame.clone(), spill.align, false, false) {
    let remaining: Vec<_> = std::iter::once((frame, need))
        .chain(spill.frames).collect();
    self.composer.work.par_spill = Some(ParSpill {
        frames: remaining.into_iter(),
        align: spill.align,
        leading: spill.leading,
    });
    return Err(err);
}
```

**Key insight:** Any time you consume something from an iterator and then call a fallible operation, you need to handle putting it back on error.

---

## Debugging Techniques

### 1. Targeted Logging

Add `eprintln!` at key points:
```rust
eprintln!("[CITE] merged_cite_groups: registered={}, introspector={}",
          registered.len(), introspector_groups.len());
```

This revealed that citations were using partial registered sets.

### 2. Understanding the Flow

For citation issues, trace:
1. When is CiteGroup created? (realization)
2. When does it get location? (prepare)
3. When is Works::generate called? (realize)
4. What groups does it see? (merged_cite_groups)
5. What location is looked up? (CiteGroup::realize)

### 3. Test Isolation

Run specific failing tests:
```bash
cargo test --package typst-tests -- "issue-1597"
cargo test --package typst-tests -- "footnote-invariant"
```

### 4. Visual Diff Analysis

Compare live vs reference PNG output to understand WHAT is wrong before diving into WHY.

---

## Common Pitfalls

1. **Assuming introspector has current data** - It doesn't. It has PREVIOUS pass data.

2. **Forgetting memoization implications** - Different inputs = different cached results = potential inconsistency.

3. **Not handling errors in loops** - If you consume from iterator then call fallible function, handle putting it back.

4. **Location instability across passes** - Same logical element gets DIFFERENT locations in different passes. Can't dedupe across passes by location.

5. **Order of operations in prepare()** - Must set location BEFORE cloning if you need the clone to have the location.

---

## Test Categories

| Test Pattern | What It Tests |
|-------------|---------------|
| `footnote-*` | Footnote layout and migration |
| `cite-*`, `bibliography-*` | Citation system |
| `flow-*` | General flow layout |
| `converge-*` | Convergence behavior |
| `wrap-float-*` | Phase 2+ features (expected to fail in Phase 1) |

---

## Files to Know

| File | Purpose |
|------|---------|
| `crates/typst-layout/src/flow/distribute.rs` | Distribution of children into regions |
| `crates/typst-layout/src/flow/compose.rs` | Footnote handling, float placement |
| `crates/typst-layout/src/flow/collect.rs` | Collection of children, ParChild |
| `crates/typst-library/src/model/bibliography.rs` | Citation registry, Works generation |
| `crates/typst-library/src/model/cite.rs` | CiteGroup realization |
| `crates/typst-realize/src/lib.rs` | Element realization, prepare() |
| `crates/typst/src/lib.rs` | Convergence loop |

---

## The Stop Type and Control Flow

### Stop Variants

```rust
enum Stop {
    Finish(bool),  // bool = forced. false = can retry, true = hard stop
    Relayout(...), // Need to relayout with different parameters
}
```

**`Stop::Finish(false)`** - "Soft" finish. Current region is done, move to next. Used for:
- Content doesn't fit
- Footnote migration needed
- Widow/orphan prevention

**`Stop::Finish(true)`** - "Hard" finish. Stop processing entirely.

### The `advance()` Pattern

In the distributor, `advance()` moves past the current `Child` in the work queue.

**Call `advance()` when:** You've saved state (spill) for the current child to continue in next region.

**DON'T call `advance()` when:** The child should be retried from scratch in next region.

The bug we fixed: `par()` called `advance()` unconditionally on `Stop::Finish`, but `par_spill()` wasn't always saving the frames. Result: frames lost.

---

## The Measure/Commit Pattern (Phase 1)

### Why Deferred Layout?

Before Phase 1:
```
collect() → layout_par_impl (MEMOIZED) → LineChild with frames
```

After Phase 1:
```
collect() → ParChild (deferred)
distribute() → par.measure() / par.commit() (NOT memoized) → frames
```

**Why?** Phase 2+ needs to re-measure paragraphs with different exclusion zones (for wrap-floats). Memoized layout can't handle varying exclusions.

### Consequence for Citations

Realization now happens inside `measure_par_with_exclusions()` which is NOT memoized. This means:
- CiteGroups created fresh each call
- No cache to ensure same instance returned
- Hence the need for the citation registry

---

## Critical Edge Case: Bibliography Before Content

This pattern breaks naive "prefer registered" approach:

```typst
#bibliography("/works.bib")  // Renders FIRST
@citation1 @citation2        // Registered LATER
```

Timeline within a pass:
1. Bibliography renders → registered empty → uses introspector (3 groups from prev pass)
2. Citation1 created → registered has 1
3. Citation1 renders → uses registered (1 group!) → wrong disambiguation
4. Citation2 created → registered has 2
5. Citation2 renders → uses registered (2 groups) → still wrong

**Solution:** Always merge. Introspector provides complete set for disambiguation, registered provides current-pass locations for lookup.

---

## Edge Case Tests That Changed Behavior

Two tests now PASS where they expected ERRORS:

### `converge-bibliography-2`
- Tests non-convergence with dynamically appearing bibliography
- Expected: "cannot format citation in isolation"
- Now: Citation works (our fix makes it succeed)

### `measure-citation-in-flow-different-span`
- Tests citation measurement with different spans
- Expected: "cannot format citation in isolation"
- Now: Citation works

**Resolution:** These tests have been updated to expect success. The error annotations were removed and reference outputs regenerated.

---

## Verified Implementation Details

### Widow/Orphan Logic Duplication

The widow/orphan `need` computation exists in two places:
- `collect.rs` `lines()` method (for inline content via `run_inline()`)
- `distribute.rs` `par()` method (for deferred paragraphs via `Child::Par`)

**Verification result:** Both implementations are functionally identical. The only difference is defensive coding style (distributor uses `.get().map_or()` vs direct indexing). Duplication is acceptable (~15 lines each) given different output formats (`Child::Line` vs `(Frame, Abs)` pairs).

### Leading Handling in par_spill

The `first` flag in `par_spill()` controls inter-line spacing:

```rust
fn par_spill(&mut self, mut spill: ParSpill) -> FlowResult<()> {
    let mut first = true;
    while let Some((frame, need)) = spill.frames.next() {
        if !first {
            self.rel(spill.leading.into(), 5);  // Leading between lines
        }
        first = false;
        // ... process frame
    }
}
```

**Key insight:** Each call to `par_spill()` resets `first = true`. This means:
- No leading before the first line in each region (correct)
- Leading only added between lines within a region
- Matches old `Child::Line` behavior where `Child::Rel(leading)` was consumed in previous region

### Locator Correctness with relayout()

`ParChild` uses the locator correctly for introspection stability:

```rust
// In ParChild (collect.rs)
pub fn measure(&self, engine: &mut Engine) -> SourceResult<ParMeasureResult> {
    measure_par_with_exclusions(engine, self.locator.relayout(), ...)  // relayout()!
}

pub fn commit(&self, engine: &mut Engine, ...) -> SourceResult<ParCommitResult> {
    layout_par_with_exclusions(engine, self.locator.relayout(), ...)  // relayout()!
}
```

**Why this matters:**
- `relayout()` produces identical locations across multiple calls
- Same content measured/committed multiple times → same location
- Introspection queries see consistent element identities
- All locate/query/label/ref tests pass

---

## Test Execution Notes

### Thread Contention Warning

The thread-local citation registry can cause test hangs at high parallelism:

```bash
# May hang on some citation tests
cargo test --package typst-tests

# Recommended: limit threads
cargo test --package typst-tests -- --num-threads 4
```

The `issue-785-cite-locate` test is particularly sensitive. It passes when run alone but can hang when run with many parallel threads due to citation registry contention.

### Current Test Status (Phase 1 Complete)

```bash
# Full suite with thread limit
cargo test --package typst-tests -- --num-threads 4
# Result: 3213 passed, 17 failed (all wrap-float-* tests)
```

The 17 failures are all `wrap-float-*` tests, which are expected to fail until Phase 2+ implements the wrap-float feature.

---

## Verification Checklist

After making changes to citation/footnote code:

```bash
# Core citation tests (should all pass)
cargo test --package typst-tests -- "cite" --num-threads 4

# Core footnote tests (should all pass)
cargo test --package typst-tests -- "footnote" --num-threads 4

# Introspection tests (should all pass)
cargo test --package typst-tests -- "locate" "query" "label" "ref" --num-threads 4

# Flow and paragraph tests
cargo test --package typst-tests -- "flow" --num-threads 4
cargo test --package typst-tests -- "paragraph" --num-threads 4

# Full suite (expect 17 failures, all wrap-float-* tests)
cargo test --package typst-tests -- --num-threads 4
```

---

## Design Rationale

### Why Thread-Local Registry?

Options considered:
1. **Pass through Engine** - Would require changing many function signatures
2. **Global static** - Thread-safety issues with parallel compilation
3. **Thread-local** - Simple, works with current architecture, cleared per iteration

Thread-local chosen for minimal invasiveness. Future consideration: if Typst moves to async/parallel layout, this needs revisiting.

### Performance Note

`merged_cite_groups()` allocates a HashSet when both sources have data:
```rust
let existing: HashSet<_> = introspector_groups.iter()
    .filter_map(|g| g.location()).collect();
```

This happens on every `Works::generate()` call. For documents with many citations, consider:
- Caching the merged result per-pass
- Using a more efficient data structure
- Profiling to see if it matters in practice

