// Adversarial wrap-float demos and tests.

--- wrap-float-with-footnote paged ---
#set page(width: 200pt, height: 200pt)
#set footnote(numbering: "1")
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: aqua))
Text with a footnote#footnote[This is the footnote content.] that wraps around the float.
#lorem(30)

--- wrap-float-footnote-bottom paged ---
#set page(width: 220pt, height: 220pt)
#set footnote(numbering: "1")
#place(bottom + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(30) #footnote[Footnote text that should stay with the reference.]
#lorem(30)

--- wrap-float-list-adjacent paged ---
#set page(width: 220pt, height: 220pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
+ First list item wraps around the float and continues.
+ Second item wraps as well and should align cleanly.

--- wrap-float-table-nearby paged ---
#set page(width: 240pt, height: 240pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(20)
#table(
  columns: 3,
  [A], [B], [C],
  [1], [2], [3],
  [4], [5], [6],
)
#lorem(20)

--- wrap-float-long-paragraph-guardrail paged large ---
#set page(width: 260pt, height: 260pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 70pt, height: 90pt, fill: aqua))
#lorem(220)

--- wrap-float-overlap-zones paged ---
#set page(width: 200pt, height: 200pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 40pt, height: 50pt, fill: aqua))
#place(top + right, float: true, wrap: true, dy: 30pt, clearance: 8pt,
  rect(width: 50pt, height: 50pt, fill: forest))
#lorem(60)

--- wrap-float-rtl paged ---
// RTL text with physical left alignment - float on visual left.
#set page(width: 220pt, height: 220pt)
#set text(dir: rtl)
#place(top + left, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(60)

--- wrap-float-rtl-start paged ---
// RTL text with logical start alignment - float should appear on visual RIGHT
// because "start" in RTL means the right side.
#set page(width: 220pt, height: 220pt)
#set text(dir: rtl)
#place(top + start, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: forest))
#lorem(60)

--- wrap-float-parent-columns-offset paged ---
#set page(width: 240pt, height: 240pt)
#columns(2)[
  #place(top + right, float: true, wrap: true, scope: "parent", dx: -6pt, dy: 6pt,
    rect(width: 50pt, height: 60pt, fill: aqua))
  // Warning: 4-13 text overflows wrap-float gap; consider reducing float size or clearance
  #lorem(80)
]

--- wrap-float-inline-boxes paged ---
#set page(width: 240pt, height: 240pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(20) #box(fill: yellow, inset: 2pt)[Inline box] #lorem(20)

--- wrap-float-math-inline paged ---
#set page(width: 240pt, height: 240pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(10) $ a^2 + b^2 = c^2 $ #lorem(20)

--- wrap-float-code-block paged ---
#set page(width: 240pt, height: 260pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua))
#lorem(20)
```rust
fn main() {
    println!("hello");
}
```
#lorem(20)

// === PAGINATION TESTS ===

--- wrap-float-page-break paged ---
// Text should wrap on page 1, flow normally on page 2 (no wrap-float exclusions).
// Paragraph is re-broken on page 2 with full width.
#set page(width: 200pt, height: 150pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: aqua))
#lorem(100)

--- wrap-float-page-break-reflow paged ---
// When a paragraph with wrap exclusions spans a page break, continuation is
// re-broken with full width (no exclusions on page 2).
#set page(width: 180pt, height: 140pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 50pt, height: 60pt, fill: aqua))
#lorem(80)

--- wrap-float-deferred-to-second-page paged ---
// Float and content both on second page - wrapping should work.
#set page(width: 180pt, height: 120pt)
#v(100pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 50pt, height: 50pt, fill: aqua))
#lorem(40)

--- wrap-float-appears-on-continuation-page paged ---
// Paragraph starts on page 1 (no wrap-float), continues on page 2 (has wrap-float).
// The continuation should be re-broken to wrap around the float on page 2.
#set page(width: 180pt, height: 120pt)
#lorem(30)
#pagebreak()
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 50pt, height: 50pt, fill: aqua))
#lorem(40)

// === EDGE CASE TESTS ===

--- wrap-float-very-short-paragraph paged ---
// Very short paragraph (one or two lines) near wrap-float.
#set page(width: 200pt, height: 140pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: aqua))
Short text here.

Next paragraph wraps around.

--- wrap-float-empty-paragraph-adjacent paged ---
// Empty or nearly empty paragraph adjacent to wrap-float should not crash.
#set page(width: 200pt, height: 140pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: aqua))

#lorem(20)

--- wrap-float-near-boundary paged ---
// Float placed very close to the bottom region boundary.
#set page(width: 200pt, height: 120pt)
#place(bottom + right, float: true, wrap: true, clearance: 4pt,
  rect(width: 50pt, height: 30pt, fill: aqua))
#lorem(30)

// === INTEGRATION TESTS ===

--- wrap-float-with-citation paged ---
// Citation in wrapped paragraph tests introspection integration.
#set page(width: 220pt, height: 200pt)
#show bibliography: none
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 70pt, fill: aqua))
As noted in prior research @netwok, the text wraps around the floating element.
#lorem(30)
#bibliography("/assets/bib/works.bib")

--- wrap-float-clearance-visual paged ---
// Verify clearance creates visible gap between float and text.
#set page(width: 200pt, height: 180pt)
#place(top + right, float: true, wrap: true, clearance: 16pt,
  rect(width: 60pt, height: 60pt, fill: aqua, stroke: 1pt))
#lorem(40)

--- wrap-float-no-clearance paged ---
// Zero clearance - text should be immediately adjacent to float.
#set page(width: 200pt, height: 180pt)
#place(top + right, float: true, wrap: true, clearance: 0pt,
  rect(width: 60pt, height: 60pt, fill: aqua, stroke: 1pt))
#lorem(40)

--- wrap-float-text-below-full-width paged ---
// Lines below the float should return to full width.
#set page(width: 200pt, height: 200pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 40pt, fill: aqua))
First paragraph wraps around the float. #lorem(10)

Second paragraph is below the float and should use full page width. #lorem(30)

// === WARNING TESTS ===

--- wrap-float-too-wide paged ---
// Float wider than 2/3 of region triggers warning and falls back to regular float.
#set page(width: 200pt, height: 200pt)
// Warning: 2-91 wrap-float too wide (130.0pt > 120.0pt limit); treating as regular float
#place(top + right, float: true, wrap: true, rect(width: 130pt, height: 50pt, fill: aqua))
#lorem(30)

--- wrap-float-zero-height paged ---
// Zero-height float creates a degenerate exclusion zone that doesn't affect text.
// Should not crash or cause layout issues.
#set page(width: 200pt, height: 180pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 0pt, fill: aqua, stroke: 1pt))
#lorem(30)

--- wrap-float-negative-clearance paged ---
// Negative clearance is clamped to zero - text should be adjacent to float.
// This tests that negative values don't create invalid negative margins.
#set page(width: 200pt, height: 180pt)
#place(top + right, float: true, wrap: true, clearance: -10pt,
  rect(width: 60pt, height: 60pt, fill: aqua, stroke: 1pt))
#lorem(40)

--- wrap-float-narrow-gap paged ---
// Float + clearance leaves less than 1/6 of page width for text.
// Warns that layout is problematic - user should adjust their design.
#set page(width: 200pt, height: 180pt)
// Warning: 2-108 wrap-float leaves too little room for text (10.0pt gap < 30.0pt minimum)
#place(top + right, float: true, wrap: true, clearance: 70pt, rect(width: 100pt, height: 60pt, fill: aqua))
// Warning: 2-11 text overflows wrap-float gap; consider reducing float size or clearance
#lorem(30)

--- wrap-float-single-word paged ---
// Single long word that can't fit in the wrap-float gap.
// Warns that text overflows the available space.
#set page(width: 200pt, height: 180pt)
#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 120pt, height: 60pt, fill: aqua))
// Warning: 1-56 text overflows wrap-float gap; consider reducing float size or clearance
Supercalifragilisticexpialidocious is a very long word.
