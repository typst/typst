--- grid-tags-rowspan pdftags ---
#grid(
  columns: 4,
  stroke: 1pt,
  rows: 3,
  // the code cell should come first in the reading order
  grid.cell(rowspan: 3)[`code`], [b], [c], [d],
  // the underline cell should come second to last
  [b], grid.cell(x: 2, y: 1, colspan: 2, rowspan: 2, underline[text]),
  [b],
)

--- grid-tags-cell-breaking pdftags ---
// The second paragraph contains marked content from page 1 and 2
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

--- grid-tags-rowspan-split-3 pdftags ---
#set page(height: 5em)
#grid(
  columns: 2,
  fill: red,
  inset: 0pt,
  grid.cell(fill: orange, rowspan: 10, place(bottom)[*Z*] + [x\ ] * 10 + place(bottom)[*ZZ*]),
  ..([y],) * 10,
  [a], [b],
)
