# Wrap-Float Risk Review (Adversarial Pass)

This document highlights failure modes and hardening steps for the wrap-float
plan, focusing on the new in-flow wrap-float model, two-phase paragraph layout,
and variable-width Knuth-Plass.

## 1) Flow Semantics and Pagination

### Risk: Wrap floats change pagination invariants
- In-flow wrap floats that do not consume vertical space can allow a region to
  appear "overfull" when measured by actual content height.
- If layout assumes consumed height for all flow children, wrap-floats may be
  skipped or mispositioned during page breaks.

Hardening:
- Define explicit "non-consuming" flow items in distribute.
- Audit any code that assumes items map 1:1 to vertical consumption.
- Add tests where wrap-floats appear near region boundaries.

### Risk: Interaction with footnotes and insertions
- Footnote insertion reduces the inner region height; wrap exclusion bands must
  be computed against the reduced inner region origin, not the page origin.

Hardening:
- Make region-to-page transforms explicit and verified in code.
- Add a test with footnotes + wrap floats near the bottom of a column.

## 2) Columns and Parent-Scope Floats

### Risk: Parent-scoped wrap floats affect multiple columns inconsistently
- If wrap bands are computed in page coordinates but applied per-column, text
  could wrap against a float that is visually in another column.

Hardening:
- Centralize a coordinate transform API: region_to_page and page_to_region.
- Add a test with parent-scoped wrap float across multi-column layout.

## 3) Variable-Width Knuth-Plass

### Risk: DP cost model under variable widths
- The original Knuth-Plass assumes fixed line width; using width(y) can cause
  unexpected line breaks or instability if widths oscillate.

Hardening:
- Add a guardrail that limits per-line width variance (max delta) or falls back
  to simple breaking for highly irregular exclusions.
- Instrument DP with debug metrics (candidate counts, line count, cost spread).

### Risk: Complexity regressions
- DP state size can grow quickly with variable widths and long paragraphs.

Hardening:
- Cap line count and breakpoint count; fall back to simple breaking when exceeded.
- Cache measurement results by region + exclusions + text hash.

## 4) Measure/Commit Split

### Risk: Locator/introspection inconsistency
- Measuring without committing may skip side effects needed by introspection.

Hardening:
- Ensure measure uses the same locator path as commit, and commit performs all
  tagging/annotation work.
- Add a test that uses `location()` / `query()` over wrapped paragraphs.

### Risk: Line metrics drift
- If line metrics computed during measure do not match commit results, the
  computed widow/orphan checks or y offsets could be wrong.

Hardening:
- In commit, assert that line metrics are consistent within tolerance.
- Use the same shaping outputs for measure and commit.

## 5) Wrap-Float Placement Rules

### Risk: Alignment rules diverge from existing floats
- If wrap-floats use new rules, users see inconsistent positioning.

Hardening:
- Encode shared placement rules in a single function used by both float kinds.
- Add tests for top/bottom + left/right align combinations.

### Risk: Negative offsets and delta handling
- `dx/dy` and `alignment` may shift floats into unexpected areas and violate
  exclusion assumptions.

Hardening:
- Apply delta to both rendered frame and exclusion band.
- Add tests with `dx/dy` on wrap floats.

## 6) Content Types

### Risk: Non-paragraph content around wrap floats
- Lists, tables, and block math may not wrap (by design), but can create
  confusing gaps if they are adjacent to a wrap float.

Hardening:
- Define and document that only paragraphs wrap.
- Add tests for list/paragraph adjacency to wrap floats.

## 7) Failure Mode Tests (Must-Have)

- Wrap float near page bottom + footnotes.
- Parent-scoped wrap float across columns.
- Mixed font sizes in wrapped paragraphs.
- Multiple wrap floats with overlapping vertical bands.
- Wrap float with `dx/dy` offsets.
- Long paragraph with many breakpoints (guardrail fallback).

## 8) Phase Gates (Adversarial)

Each phase should pass a stress test before proceeding:

- Phase 1: Measure/commit matches current paragraph layout (no wrap).
- Phase 2: Wrap floats do not affect pagination when wrap disabled.
- Phase 3: Variable-width K-P produces stable output under stress tests.
- Phase 4: Columns + parent-scope wrap floats behave deterministically.
