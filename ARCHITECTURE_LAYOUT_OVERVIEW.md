# Typst Layout Architecture Overview (Flow, Floats, Paragraphs)

This document summarizes the layout architecture relevant to wrap-float work.
It is intentionally scoped to the flow pipeline and inline paragraph layout.

## 1) High-Level Pipeline

Typical layout runs through these stages:

1) Realization: Content is realized into layout pairs (content + styles).
2) Flow layout: The realized pairs are laid out into frames via:
   - collect: build simplified "children" for flow layout.
   - compose: handle out-of-flow insertions (floats, footnotes) and columns.
   - distribute: pack children into regions, producing aligned items.
3) Inline layout: Paragraphs are shaped and line-broken into frames.

The key loop is per region (and per column when columns are enabled):

- collect(children, base, expand, mode) -> Vec<Child>
- compose(...regions...) -> distribute(...) -> Frame
- repeat for next region until work is done

## 2) Flow Layout Roles

### Collect (flow/collect.rs)

Collect converts realized content into flow children. It does light work to
prepare layout so distribution can be fast.

- Paragraphs are currently laid out into LineChild frames at collection time.
- Blocks become SingleChild (unbreakable) or MultiChild (breakable).
- Placed elements become PlacedChild (floats or absolute).

### Compose (flow/compose.rs)

Compose orchestrates layout across columns and pages. It owns:

- Insertions: top/bottom floats, footnotes, and their height effects.
- Relayout triggers: floats and footnotes can cause relayout of a region.
- The column/page loop: composing one column uses distribute(), then merges
  insertions with the inner frame.

### Distribute (flow/distribute.rs)

Distribute consumes Child items and fits them into the current region:

- Maintains remaining region size, handles spacing collapse, and alignments.
- Emits Item::Frame and Item::Placed that are aligned and finalized later.
- Handles line widow/orphan logic for paragraph LineChild frames.

## 3) Inline Layout Roles

Inline layout (inline/mod.rs) handles paragraphs and inline flows:

- collect: shape text and gather inline segments into a Preparation.
- linebreak: choose breakpoints (simple or Knuth-Plass).
- finalize: commit lines into frames with alignment and justification.

Paragraph layout is memoized with comemo; it uses locator tracking for
introspection and cross-references.

## 4) Floats and Insertions (Current Behavior)

Floats are out-of-flow insertions managed by compose:

- PlacedChild::layout computes the frame for a float in a chosen scope.
- Composer::float places the float into a top/bottom insertion area.
- Insertions finalize by merging float frames with the inner flow frame.
- Floats can be queued if they do not fit, causing relayout in later regions.

Implications:

- Float placement is decided in compose, not in distribute.
- Vertical space is reserved by insertion areas (top/bottom sizes).
- Distribution is not aware of float geometry unless explicitly passed in.

## 5) Paragraphs (Current Behavior)

Paragraphs are laid out during collection:

- ParElem is turned into LineChild frames at collect time.
- Distribute positions LineChild frames and applies widow/orphan rules.

This means paragraph layout cannot depend on region y-positions or float
geometry (because those are decided later).

## 6) Why Wrap-Floats Need Changes

Text wrap requires variable line widths based on float positions. That implies:

- Paragraph layout must happen during distribution, when y-positions are known.
- Line breaking must accept a width function width(y) rather than a constant.
- Float geometry must be accessible where paragraphs are laid out.

This touches all three layers:

- Collect: paragraphs must be deferred (store ParElem + styles + locator).
- Distribute: compute exclusions from wrap-floats and lay out paragraphs.
- Inline: support variable-width line breaking and two-phase layout.

## 7) Key Interaction Points

- Locator / Introspection: layout functions require locators; moving layout to
  distribution must preserve locator tracking and comemo memoization.
- Columns: compose can lay out multiple columns; wrap geometry must be expressed
  in the same coordinate space as paragraphs in each region.
- Footnotes: insertions consume vertical space; wrap exclusions must respect
  the reduced inner region.

## 8) Terminology Reference

- Region: the current rectangular space being filled by distribute.
- Child: simplified flow element produced by collect.
- Item: positioned output within distribute before final alignment.
- Insertion: out-of-flow element that is merged into the final frame.

## 9) Extension Strategy for Wrap

The wrap-float plan intentionally changes how floats are represented:

- Wrap floats become in-flow items (or a new float kind) so that exclusions can
  be computed where paragraphs are laid out.
- Paragraphs use measure + commit to avoid line-height estimation errors.
- Knuth-Plass is extended to support variable widths for quality.

This reshapes the pipeline but keeps the overall collect -> compose ->
 distribute structure intact.
