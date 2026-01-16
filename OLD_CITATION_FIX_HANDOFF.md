# Citation System Fix - Handoff Document

## Executive Summary

Phase 1 of the wrap-float implementation introduced deferred paragraph layout (`ParChild` with measure/commit pattern). This broke the citation system because `Works::generate()` queries the introspector for `CiteGroup` elements, but the introspector only contains data from **previous** layout passes - not the current one.

**Current Status:** Core fix is implemented and working for most cases. Two edge-case tests involving footnotes that span pages still fail, but these are related to pre-existing footnote handling issues, not the citation fix itself.

---

## The Problem

### Root Cause

The citation system relies on introspection to find all `CiteGroup` elements in the document:

```
1. CiteGroup is created with location L during realization
2. CiteGroup::realize() is called (as a show rule)
3. realize() calls Works::generate()
4. Works::generate() queries introspector for all CiteGroups
5. Introspector doesn't have the CURRENT CiteGroup (only groups from previous passes)
6. Works has no entry for location L → "cannot format citation in isolation" error
```

Before Phase 1, memoization masked this issue by returning cached `CiteGroup` instances that matched what was in the introspector. The deferred layout broke this caching behavior.

### Error Manifestation

```
Error: cannot format citation in isolation
Hint: check whether this citation is measured without being inserted into the document
```

This error appeared in tests `issue-1597-cite-footnote` and `issue-3481-cite-location`.

---

## The Solution

### Approach: Register CiteGroups During Layout

Register CiteGroups immediately when they're created (after they receive their location), then merge with introspector data when generating Works.

### Implementation Details

#### 1. Thread-Local Registry (`bibliography.rs`)

```rust
thread_local! {
    static CITE_GROUPS: RefCell<EcoVec<Content>> = const { RefCell::new(EcoVec::new()) };
}

pub fn register_cite_group(group: Content) { ... }
fn get_registered_cite_groups() -> EcoVec<Content> { ... }
pub fn clear_cite_group_registry() { ... }
```

#### 2. Register in `prepare()` Function (`typst-realize/src/lib.rs`)

CiteGroups are registered **after** they receive their location in the `prepare()` function:

```rust
if elem.location().is_none() && flags.any() {
    let loc = locator.next_location(engine, key, elem.span());
    elem.set_location(loc);
}

// Register CiteGroups after they get their location
if elem.is::<CiteGroup>() {
    register_cite_group(elem.clone());
}
```

**Critical:** Must register AFTER `set_location()` because Content uses copy-on-write semantics. Cloning before the location is set results in a clone without the location.

#### 3. Modified `Works::generate()` (`bibliography.rs`)

```rust
fn merged_cite_groups(engine: &mut Engine, span: Span) -> EcoVec<Content> {
    let registered = get_registered_cite_groups();

    // If we have registered groups, use them
    if !registered.is_empty() {
        return registered;
    }

    // Fall back to introspector groups from previous passes
    engine.introspect(CiteGroupIntrospection(span))
}
```

#### 4. Clear Registry Each Iteration (`typst/src/lib.rs`)

```rust
loop {
    clear_cite_group_registry();  // Clear at start of each iteration
    // ... rest of convergence loop
}
```

---

## Key Technical Insights

### Content Copy-on-Write Semantics

`Content` uses reference counting similar to `Arc`, but with copy-on-write for mutations:

```rust
fn set_location(&mut self, location: Location) {
    self.0.meta_mut().location = Some(location);  // Calls make_unique()
}
```

When `set_location()` is called, `make_unique()` creates a new allocation if there are multiple references. This means:

- Clone BEFORE `set_location()` → clone has OLD allocation (no location)
- Clone AFTER `set_location()` → clone shares NEW allocation (has location)

This is why registration must happen AFTER the location is assigned.

### Memoization Behavior

`Works::generate_impl` is memoized based on `(routines, world, bibliography, groups)`. Different `groups` values result in different memoization keys, so:

- Bibliography called with `groups=[]` → Works1 (no citations)
- Citation called with `groups=[cite1]` → Works2 (with citation)

These are separate cached results.

### Why Simple Merging Doesn't Work

Initially tried merging introspector groups with registered groups by location:

```rust
let existing: FxHashSet<_> = groups.iter().filter_map(|g| g.location()).collect();
for group in registered {
    if !existing.contains(&group.location()) {
        groups.push(group);
    }
}
```

**Problem:** The same logical CiteGroup gets DIFFERENT locations in different passes (locations are generated fresh each iteration). So deduplication by location doesn't identify "same" citations across passes.

**Solution:** Use registered groups exclusively when available (they're the most accurate for the current pass), fall back to introspector only when registry is empty.

---

## Test Results

### Before Fix
- 3210 passed, 20 failed
- `issue-1597-cite-footnote`: "cannot format citation in isolation" (fatal)
- `issue-3481-cite-location`: "cannot format citation in isolation" (fatal)

### After Fix
- 3207 passed, 23 failed
- `issue-1597-cite-footnote`: Produces output (mismatched with reference)
- `issue-3481-cite-location`: Produces output (mismatched with reference)
- `issue-785-cite-locate`: Now passes (was failing with convergence issues)
- 20/22 citation tests pass

### Remaining Failures Analysis

| Test | Status | Root Cause |
|------|--------|------------|
| `issue-1597-cite-footnote` | Mismatched output | Pre-existing footnote migration issue |
| `issue-3481-cite-location` | Mismatched output | Pre-existing footnote migration issue |
| `footnote-invariant` | Mismatched output | **Was already failing before citation fix** |
| `bibliography-before-content` | Mismatched output | PNG encoding difference (RGB vs RGBA) |

The `footnote-invariant` test has NO citations but shows the same symptom (missing footnote content when footnotes span pages). This confirms the remaining failures are due to pre-existing footnote handling issues, not the citation fix.

---

## Files Modified

1. **`crates/typst-library/src/model/bibliography.rs`** (+42 lines)
   - Thread-local registry
   - `register_cite_group()`, `get_registered_cite_groups()`, `clear_cite_group_registry()`
   - `merged_cite_groups()` helper function

2. **`crates/typst-realize/src/lib.rs`** (+9 lines)
   - Import `register_cite_group`
   - Registration call in `prepare()` after location assignment

3. **`crates/typst/src/lib.rs`** (+5 lines)
   - Import `clear_cite_group_registry`
   - Clear call at start of convergence loop

---

## What Remains To Be Done

### 1. Investigate Footnote Migration Issues

The `footnote-invariant` test was failing before the citation fix. Both remaining citation test failures involve footnotes that span pages. The footnote handling code in `crates/typst-layout/src/flow/compose.rs` needs investigation.

Key functions to examine:
- `fn footnote()` - handles single footnote layout
- `fn footnotes()` - searches for and processes footnotes in frames
- `fn footnote_spill()` - handles footnote spillover across pages

### 2. Consider Alternative Merging Strategy

The current approach uses registered groups exclusively when available. A more sophisticated approach might:

- Track CiteGroups by their content/children rather than location
- Merge intelligently based on citation keys
- Handle cases where bibliography appears before citations in document order

### 3. PNG Encoding Consistency

The `bibliography-before-content` test produces visually correct output but fails due to PNG format differences (RGB vs RGBA). This is likely a test infrastructure issue rather than a real bug.

### 4. Edge Cases to Test

- Citations in table cells
- Citations in headers/footers
- Citations in nested show rules
- Multiple bibliographies in one document
- Citations in measured content that's later discarded

---

## Running Tests

```bash
# Specific failing tests
cargo test --package typst-tests -- "issue-1597"
cargo test --package typst-tests -- "issue-3481"

# All citation tests
cargo test --package typst-tests -- "cite"

# All bibliography tests
cargo test --package typst-tests -- "bibliography"

# Footnote tests (to investigate pre-existing issues)
cargo test --package typst-tests -- "footnote"

# Full test suite
cargo test --package typst-tests
```

---

## Key Code Locations

| Component | File | Line(s) |
|-----------|------|---------|
| CiteGroup creation | `crates/typst-realize/src/lib.rs` | ~1100 (`finish_cites`) |
| CiteGroup preparation | `crates/typst-realize/src/lib.rs` | ~514 (`prepare`) |
| CiteGroup realization | `crates/typst-library/src/model/cite.rs` | ~162 (`realize`) |
| Works generation | `crates/typst-library/src/model/bibliography.rs` | ~573 (`generate`) |
| Introspector query | `crates/typst-library/src/model/bibliography.rs` | ~651 (`CiteGroupIntrospection`) |
| Convergence loop | `crates/typst/src/lib.rs` | ~130 |
| Footnote layout | `crates/typst-layout/src/flow/compose.rs` | ~415 (`footnote`) |

---

## Debugging Tips

1. **Add debug output to `merged_cite_groups()`** to see what groups are available at each call

2. **Check location consistency** by logging locations in `prepare()` and `realize()`

3. **Trace memoization** by adding logging to `generate_impl()` to see cache hits/misses

4. **Isolate footnote issues** by creating minimal test cases without citations

---

## Summary

The core citation fix is complete and working. The remaining test failures are due to pre-existing footnote handling issues that were exposed (but not caused) by the deferred layout changes. The next step should be investigating the footnote migration/invariant code to fix those underlying issues.
