# Phase 4: Variable-Width Knuth-Plass Algorithm

**Goal:** Extend the Knuth-Plass line breaking algorithm to support variable line widths for quality text wrapping around floats.

## Algorithm Overview

The standard Knuth-Plass algorithm assumes a constant line width. For wrap-floats,
we need variable widths based on vertical position. The key insight is to use
a width function instead of a single value.

## The Challenge: Circular Dependency

We don't know line y-positions until we've broken lines, but breaking depends
on widths. This is resolved by iteration:

1. **First pass:** Break with uniform width, measure line heights
2. **Second pass:** Use measured heights to compute per-line widths
3. **Re-break:** If widths differ significantly, re-break and re-measure
4. **Converge:** Stop when line breaks stabilize

## Entry Point

**File: `crates/typst-layout/src/inline/linebreak.rs`**

*Find existing linebreak:* `grep -n "^pub fn linebreak" crates/typst-layout/src/inline/linebreak.rs`
*Add new function near it.*

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
```

## Iterative Line Breaking

```rust
/// Line breaking with exclusion zones.
fn linebreak_with_exclusions<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    base_width: Abs,
    exclusions: &ParExclusions,
) -> Vec<Line<'a>> {
    const MAX_ITERATIONS: usize = 3;
    const CONVERGENCE_THRESHOLD: Abs = Abs::pt(0.5);

    let mut prev_heights: Vec<Abs> = vec![];
    let mut lines: Vec<Line<'a>> = vec![];

    for iteration in 0..MAX_ITERATIONS {
        // Compute per-line widths based on estimated y-positions
        let line_widths = compute_line_widths(
            &prev_heights,
            p.config.font_size,
            exclusions,
            base_width,
        );

        // Break lines with these widths
        lines = if line_widths.iter().all(|&w| w == base_width) {
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
```

## Width Computation

```rust
/// Compute width available for each line based on y-positions.
fn compute_line_widths(
    prev_heights: &[Abs],
    default_height: Abs,
    exclusions: &ParExclusions,
    base_width: Abs,
) -> Vec<Abs> {
    if prev_heights.is_empty() {
        // First iteration: estimate based on default line height
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

        // Add extra in case line count increases
        let avg_height = prev_heights.iter().sum::<Abs>() / prev_heights.len() as f64;
        for _ in 0..10 {
            widths.push(exclusions.available_width(base_width, y));
            y += avg_height;
        }

        widths
    }
}
```

## Guardrails

```rust
/// Check if we should fall back to simple breaking.
fn should_use_simple_breaking(p: &Preparation, line_widths: &[Abs]) -> bool {
    // Guardrail 1: Very long paragraphs
    const MAX_TEXT_LEN: usize = 10_000;
    if p.text.len() > MAX_TEXT_LEN {
        return true;
    }

    // Guardrail 2: Highly variable widths
    if line_widths.len() >= 2 {
        let min = line_widths.iter().copied().fold(Abs::inf(), Abs::min);
        let max = line_widths.iter().copied().fold(Abs::zero(), Abs::max);
        let variance_ratio = (max - min) / max;

        const MAX_VARIANCE_RATIO: f64 = 0.5;
        if variance_ratio > MAX_VARIANCE_RATIO {
            return true;
        }
    }

    false
}
```

## Simple Greedy Breaking (Fallback)

```rust
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
```

## Modified Knuth-Plass

```rust
/// Knuth-Plass with per-line width constraints.
///
/// Key difference from standard K-P: The width constraint for each
/// candidate line depends on which line index it would become.
///
/// Pruning modification: Active-set pruning is DISABLED when widths
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
                width,
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

## Approximate Pass Limitation

The approximate K-P pass (`linebreak_optimized_approximate`) cannot be used
directly with variable widths because cumulative metrics assume constant width.

For variable-width cases:
1. Skip the approximate pass
2. Use a higher upper bound (or INFINITY)
3. Accept potentially slower performance for wrapped paragraphs

```rust
fn linebreak_optimized_variable<'a>(...) -> Vec<Line<'a>> {
    // Skip approximate pass - use direct bounded search with INFINITY bound
    linebreak_optimized_bounded_variable(engine, p, line_widths, metrics, Cost::INFINITY)
}
```

## Performance Considerations

| Scenario | Expected Performance |
|----------|---------------------|
| No exclusions | Same as current (fast path) |
| Single wrap-float | ~1.5-2x slower |
| Multiple wrap-floats | ~2-3x slower |
| Long paragraph + wrap | May fall back to simple |

## Test Plan

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

// --- wrap-float-convergence ---
// Test that iteration converges
#set page(height: 300pt, width: 200pt)
#place(top + right, float: true, wrap: true,
  rect(width: 70pt, height: 150pt, fill: aqua))
#for i in range(10) {
  [Line #i with varying content length. ]
  if calc.odd(i) { text(size: 14pt)[Larger] }
}
```

## Exit Criteria

- [ ] K-P produces good line breaks with exclusions
- [ ] Guardrails trigger appropriately on complex cases
- [ ] Performance acceptable (< 3x for typical cases)
- [ ] Iteration converges within 3 passes
- [ ] Simple fallback produces acceptable results

## Dependencies

- [Phase 3: Distribution Changes](WRAP_PHASE_3.md) must be complete

## Next Phase

[Phase 5: Edge Cases and Polish](WRAP_PHASE_5.md)
