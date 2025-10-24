// Test grid layouts.

--- grid-columns-sizings-rect render ---
#let cell(width, color) = rect(width: width, height: 2cm, fill: color)
#set page(width: 100pt, height: 140pt)
#grid(
  columns: (auto, 1fr, 3fr, 0.25cm, 3%, 2mm + 10%),
  cell(0.5cm, rgb("2a631a")),
  cell(100%,  forest),
  cell(100%,  conifer),
  cell(100%,  rgb("ff0000")),
  cell(100%,  rgb("00ff00")),
  cell(80%,   rgb("00faf0")),
  cell(1cm,   rgb("00ff00")),
  cell(0.5cm, rgb("2a631a")),
  cell(100%,  forest),
  cell(100%,  conifer),
  cell(100%,  rgb("ff0000")),
  cell(100%,  rgb("00ff00")),
)

--- grid-gutter-fr render ---
#set rect(inset: 0pt)
#grid(
  columns: (auto, auto, 40%),
  column-gutter: 1fr,
  row-gutter: 1fr,
  rect(fill: eastern)[dddaa aaa aaa],
  rect(fill: conifer)[ccc],
  rect(fill: rgb("dddddd"))[aaa],
)

--- grid-row-sizing-manual-align render ---
#set page(height: 3cm, margin: 0pt)
#grid(
  columns: (1fr,),
  rows: (1fr, auto, 2fr),
  [],
  align(center)[A bit more to the top],
  [],
)

--- grid-finance render ---
// Test using the `grid` function to create a finance table.
#set page(width: 11cm, height: 2.5cm)
#grid(
  columns: 5,
  column-gutter: (2fr, 1fr, 1fr),
  row-gutter: 6pt,
  [*Quarter*],
  [Expenditure],
  [External Revenue],
  [Financial ROI],
  [_total_],
  [*Q1*],
  [173,472.57 \$],
  [472,860.91 \$],
  [51,286.84 \$],
  [_350,675.18 \$_],
  [*Q2*],
  [93,382.12 \$],
  [439,382.85 \$],
  [-1,134.30 \$],
  [_344,866.43 \$_],
  [*Q3*],
  [96,421.49 \$],
  [238,583.54 \$],
  [3,497.12 \$],
  [_145,659.17 \$_],
)
// Test grid cells that overflow to the next region.

--- grid-cell-breaking render ---
#set page(width: 5cm, height: 3cm)
#grid(
  columns: 2,
  row-gutter: 8pt,
  [Lorem ipsum dolor sit amet.

  Aenean commodo ligula eget dolor. Aenean massa. Penatibus et magnis.],
  [Text that is rather short],
  [Fireflies],
  [Critical],
  [Decorum],
  [Rampage],
)

--- grid-consecutive-rows-breaking render ---
// Test a column that starts overflowing right after another row/column did
// that.
#set page(width: 5cm, height: 2cm)
#grid(
  columns: 4 * (1fr,),
  row-gutter: 10pt,
  column-gutter: (0pt, 10%),
  align(top, image("/assets/images/rhino.png")),
  align(top, rect(inset: 0pt, fill: eastern, align(right)[LoL])),
  [rofl],
  [\ A] * 3,
  [Ha!\ ] * 3,
)

--- grid-same-row-multiple-columns-breaking render ---
// Test two columns in the same row overflowing by a different amount.
#set page(width: 5cm, height: 2cm)
#grid(
  columns: 3 * (1fr,),
  row-gutter: 8pt,
  column-gutter: (0pt, 10%),
  [A], [B], [C],
  [Ha!\ ] * 6,
  [rofl],
  [\ A] * 3,
  [hello],
  [darkness],
  [my old]
)

--- grid-nested-breaking render ---
// Test grid within a grid, overflowing.
#set page(width: 5cm, height: 2.25cm)
#grid(
  columns: 4 * (1fr,),
  row-gutter: 10pt,
  column-gutter: (0pt, 10%),
  [A], [B], [C], [D],
  grid(columns: 2, [A], [B], [C\ ]*3, [D]),
  align(top, rect(inset: 0pt, fill: eastern, align(right)[LoL])),
  [rofl],
  [E\ ]*4,
)

--- grid-column-sizing-auto-base render ---
// Test that auto and relative columns use the correct base.
#grid(
  columns: (auto, 60%),
  rows: (auto, auto),
  rect(width: 50%, height: 0.5cm, fill: conifer),
  rect(width: 100%, height: 0.5cm, fill: eastern),
  rect(width: 50%, height: 0.5cm, fill: forest),
)

--- grid-column-sizing-fr-base render ---
// Test that fr columns use the correct base.
#grid(
  columns: (1fr,) * 4,
  rows: (1cm,),
  rect(width: 50%, fill: conifer),
  rect(width: 50%, fill: forest),
  rect(width: 50%, fill: conifer),
  rect(width: 50%, fill: forest),
)

--- grid-column-sizing-mixed-base render ---
// Test that all three kinds of rows use the correct bases.
#set page(height: 4cm, margin: 0cm)
#grid(
  rows: (1cm, 1fr, 1fr, auto),
  rect(height: 50%, width: 100%, fill: conifer),
  rect(height: 50%, width: 100%, fill: forest),
  rect(height: 50%, width: 100%, fill: conifer),
  rect(height: 25%, width: 100%, fill: forest),
)

--- grid-trailing-linebreak-region-overflow render ---
// Test that trailing linebreak doesn't overflow the region.
#set page(height: 2cm)
#grid[
  Hello \
  Hello \
  Hello \

  World
]

--- grid-breaking-expand-vertically render ---
// Test that broken cell expands vertically.
#set page(height: 2.25cm)
#grid(
  columns: 2,
  gutter: 10pt,
  align(bottom)[A],
  [
    Top
    #align(bottom)[
      Bottom \
      Bottom

      Top
    ]
  ],
  align(top)[B],
)

--- grid-complete-rows render ---
// Ensure grids expand enough for the given rows.
#grid(
  columns: (2em, 2em),
  rows: (2em,) * 4,
  fill: red,
  stroke: aqua,
  [a]
)

--- grid-auto-shrink render ---
// Test iterative auto column shrinking.
#set page(width: 210mm - 2 * 2.5cm + 2 * 10pt)
#set text(11pt)
#table(
  columns: 4,
  [Hello!],
  [Hello there, my friend!],
  [Hello there, my friends! Hi!],
  [Hello there, my friends! Hi! What is going on right now?],
)

--- issue-grid-base-auto-row render ---
// Test that grid base for auto rows makes sense.
#set page(height: 150pt)
#table(
  columns: (1.5cm, auto),
  rows: (auto, auto),
  rect(width: 100%, fill: red),
  rect(width: 100%, fill: blue),
  rect(width: 100%, height: 50%, fill: green),
)

--- issue-grid-base-auto-row-list render ---
#rect(width: 100%, height: 1em)
- #rect(width: 100%, height: 1em)
  - #rect(width: 100%, height: 1em)

--- issue-grid-skip render ---
// Grid now skips a remaining region when one of the cells
// doesn't fit into it at all.
#set page(height: 100pt)
#grid(
  columns: (2cm, auto),
  rows: (auto, auto),
  rect(width: 100%, fill: red),
  rect(width: 100%, fill: blue),
  rect(width: 100%, height: 80%, fill: green),
  [hello \ darkness #parbreak() my \ old \ friend \ I],
  rect(width: 100%, height: 20%, fill: blue),
  polygon(fill: red, (0%, 0%), (100%, 0%), (100%, 20%))
)

--- issue-grid-skip-list render ---
#set page(height: 60pt)
#lines(2)
- #lines(2)

--- issue-grid-double-skip render ---
// Ensure that the list does not jump to the third page.
#set page(height: 70pt)
#v(40pt)
The following:
+ A
+ B

--- issue-grid-gutter-skip render ---
// Ensure gutter rows at the top or bottom of a region are skipped.
#set page(height: 10em)

#table(
  row-gutter: 1.5em,
  inset: 0pt,
  rows: (1fr, auto),
  [a],
  [],
  [],
  [f],
  [e\ e],
  [],
  [a]
)

--- issue-3917-grid-with-infinite-width render ---
// https://github.com/typst/typst/issues/1918
#set page(width: auto)
#context layout(available => {
  let infinite-length = available.width
  // Error: 3-50 cannot create grid with infinite width
  grid(gutter: infinite-length, columns: 2)[A][B]
})

--- issue-7103-wrong-state-calculation render ---
#set page(paper: "a10")

#let st = state("st", 0)

#let fn() = {
  st.update(i => i + 1)
  lorem(11)
  st.update(i => i - 1)
}

#grid(fn())

#fn()

Result: #context st.get()
