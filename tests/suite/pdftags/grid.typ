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
