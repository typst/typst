--- grid-footer ---
#set page(width: auto, height: 15em)
#set text(6pt)
#set table(inset: 2pt, stroke: 0.5pt)
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..range(0, 5).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten(),
  table.footer(
    table.hline(start: 2, end: 3, stroke: yellow),
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.cell(colspan: 5)[*Cool Zone*]
  )
)

--- grid-footer-gutter-and-no-repeat ---
// Gutter & no repetition
#set page(width: auto, height: 16em)
#set text(6pt)
#set table(inset: 2pt, stroke: 0.5pt)
#table(
  columns: 5,
  gutter: 2pt,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..range(0, 5).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten(),
  table.footer(
    repeat: false,
    table.hline(start: 2, end: 3, stroke: yellow),
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.cell(colspan: 5)[*Cool Zone*]
  )
)

--- grid-cell-override-in-header-and-footer ---
#table(
  table.header(table.cell(stroke: red)[Hello]),
  table.footer(table.cell(stroke: aqua)[Bye]),
)

--- grid-cell-override-in-header-and-footer-with-gutter ---
#table(
  gutter: 3pt,
  table.header(table.cell(stroke: red)[Hello]),
  table.footer(table.cell(stroke: aqua)[Bye]),
)

--- grid-footer-top-stroke ---
// Footer's top stroke should win when repeated, but lose at the last page.
#set page(height: 10em)
#table(
  stroke: green,
  table.header(table.cell(stroke: red)[Hello]),
  table.cell(stroke: yellow)[Hi],
  table.cell(stroke: yellow)[Bye],
  table.cell(stroke: yellow)[Ok],
  table.footer[Bye],
)

--- grid-footer-relative-row-sizes ---
// Relative lengths
#set page(height: 10em)
#table(
  rows: (30%, 30%, auto),
  [C],
  [C],
  table.footer[*A*][*B*],
)

--- grid-footer-cell-with-y ---
#grid(
  grid.footer(grid.cell(y: 2)[b]),
  grid.cell(y: 0)[a],
  grid.cell(y: 1)[c],
)

--- grid-footer-cell-with-x ---
#grid(
  columns: 2,
  stroke: black,
  inset: 5pt,
  grid.cell(x: 1)[a],
  // Error: 3-56 footer must end at the last row
  grid.footer(grid.cell(x: 0)[b1], grid.cell(x: 0)[b2]),
  // This should skip the footer
  grid.cell(x: 1)[c]
)

--- grid-footer-no-expand-with-col-and-row-pos-cell ---
#grid(
  columns: 2,
  [a], [],
  [b], [],
  fill: (_, y) => if calc.odd(y) { blue } else { red },
  inset: 5pt,
  grid.cell(x: 1, y: 3, rowspan: 4)[b],
  grid.cell(y: 2, rowspan: 2)[a],
  grid.footer(),
  // Error: 3-27 cell would conflict with footer spanning the same position
  // Hint: 3-27 try reducing the cell's rowspan or moving the footer
  grid.cell(x: 1, y: 7)[d],
)

--- grid-footer-no-expand-with-row-pos-cell ---
#grid(
  columns: 2,
  [a], [],
  [b], [],
  fill: (_, y) => if calc.odd(y) { blue } else { red },
  inset: 5pt,
  grid.cell(x: 1, y: 3, rowspan: 4)[b],
  grid.cell(y: 2, rowspan: 2)[a],
  grid.footer(),
  // Error: 3-33 cell would conflict with footer spanning the same position
  // Hint: 3-33 try reducing the cell's rowspan or moving the footer
  grid.cell(y: 6, rowspan: 2)[d],
)

--- grid-footer-moved-to-bottom-of-rowspans ---
#grid(
  columns: 2,
  [a], [],
  [b], [],
  stroke: red,
  inset: 5pt,
  grid.cell(x: 1, y: 3, rowspan: 4)[b],
  grid.cell(y: 2, rowspan: 2)[a],
  grid.footer(),
  grid.cell(y: 4)[d],
  grid.cell(y: 5)[e],
  grid.cell(y: 6)[f],
)

--- grid-footer-not-at-last-row ---
// Error: 2:3-2:19 footer must end at the last row
#grid(
  grid.footer([a]),
  [b],
)

--- grid-footer-not-at-last-row-two-columns ---
// Error: 3:3-3:19 footer must end at the last row
#grid(
  columns: 2,
  grid.footer([a]),
  [b],
)

--- grid-footer-overlap ---
#grid(
  columns: 2,
  grid.header(),
  grid.footer(grid.cell(y: 2)[a]),
  // Error: 3-39 cell would conflict with footer spanning the same position
  // Hint: 3-39 try reducing the cell's rowspan or moving the footer
  grid.cell(x: 1, y: 1, rowspan: 2)[a],
)

--- grid-footer-multiple ---
// Error: 4:3-4:19 cannot have more than one footer
#grid(
  [a],
  grid.footer([a]),
  grid.footer([b]),
)

--- table-footer-in-grid ---
// Error: 3:3-3:20 cannot use `table.footer` as a grid footer
// Hint: 3:3-3:20 use `grid.footer` instead
#grid(
  [a],
  table.footer([a]),
)

--- grid-footer-in-table ---
// Error: 3:3-3:19 cannot use `grid.footer` as a table footer
// Hint: 3:3-3:19 use `table.footer` instead
#table(
  [a],
  grid.footer([a]),
)

--- grid-footer-in-grid-header ---
// Error: 14-28 cannot place a grid footer within another footer or header
#grid.header(grid.footer[a])

--- table-footer-in-grid-header ---
// Error: 14-29 cannot place a table footer within another footer or header
#grid.header(table.footer[a])

--- grid-footer-in-table-header ---
// Error: 15-29 cannot place a grid footer within another footer or header
#table.header(grid.footer[a])

--- table-footer-in-table-header ---
// Error: 15-30 cannot place a table footer within another footer or header
#table.header(table.footer[a])

--- grid-footer-in-grid-footer ---
// Error: 14-28 cannot place a grid footer within another footer or header
#grid.footer(grid.footer[a])

--- table-footer-in-grid-footer ---
// Error: 14-29 cannot place a table footer within another footer or header
#grid.footer(table.footer[a])

--- grid-footer-in-table-footer ---
// Error: 15-29 cannot place a grid footer within another footer or header
#table.footer(grid.footer[a])

--- table-footer-in-table-footer ---
// Error: 15-30 cannot place a table footer within another footer or header
#table.footer(table.footer[a])

--- grid-header-in-grid-footer ---
// Error: 14-28 cannot place a grid header within another header or footer
#grid.footer(grid.header[a])

--- table-header-in-grid-footer ---
// Error: 14-29 cannot place a table header within another header or footer
#grid.footer(table.header[a])

--- grid-header-in-table-footer ---
// Error: 15-29 cannot place a grid header within another header or footer
#table.footer(grid.header[a])

--- table-header-in-table-footer ---
// Error: 15-30 cannot place a table header within another header or footer
#table.footer(table.header[a])

--- grid-header-footer-block-with-fixed-height ---
#set page(height: 17em)
#table(
  rows: (auto, 2.5em, auto),
  table.header[*Hello*][*World*],
  block(width: 2em, height: 10em, fill: red),
  table.footer[*Bye*][*World*],
)

--- grid-header-footer-and-rowspan-non-contiguous-1 ---
// Rowspan sizing algorithm doesn't do the best job at non-contiguous content
// ATM.
#set page(height: 20em)

#table(
  rows: (auto, 2.5em, 2em, auto, 5em, 2em, 2.5em),
  table.header[*Hello*][*World*],
  table.cell(rowspan: 3, lorem(20)),
  table.footer[*Ok*][*Bye*],
)

--- grid-header-footer-and-rowspan-non-contiguous-2 ---
// This should look right
#set page(height: 20em)

#table(
  rows: (auto, 2.5em, 2em, auto),
  gutter: 3pt,
  table.header[*Hello*][*World*],
  table.cell(rowspan: 3, lorem(20)),
  table.footer[*Ok*][*Bye*],
)
--- grid-header-and-footer-lack-of-space ---
// Test lack of space for header + text.
#set page(height: 9em + 2.5em + 1.5em)

#table(
  rows: (auto, 2.5em, auto, auto, 10em, 2.5em, auto),
  gutter: 3pt,
  table.header[*Hello*][*World*],
  table.cell(rowspan: 3, lorem(30)),
  table.footer[*Ok*][*Bye*],
)

--- grid-header-and-footer-orphan-prevention ---
// Orphan header prevention test
#set page(height: 13em)
#v(8em)
#grid(
  columns: 3,
  gutter: 5pt,
  grid.header(
    [*Mui*], [*A*], grid.cell(rowspan: 2, fill: orange)[*B*],
    [*Header*], [*Header* #v(0.1em)],
  ),
  ..([Test], [Test], [Test]) * 7,
  grid.footer(
    [*Mui*], [*A*], grid.cell(rowspan: 2, fill: orange)[*B*],
    [*Footer*], [*Footer* #v(0.1em)],
  ),
)

--- grid-header-and-footer-empty ---
// Empty footer should just be a repeated blank row
#set page(height: 8em)
#table(
  columns: 4,
  align: center + horizon,
  table.header(),
  ..range(0, 2).map(i => (
    [John \##i],
    table.cell(stroke: green)[123],
    table.cell(stroke: blue)[456],
    [789]
  )).flatten(),
  table.footer(),
)

--- grid-header-and-footer-containing-rowspan ---
// When a footer has a rowspan with an empty row, it should be displayed
// properly
#set page(height: 14em, width: auto)

#let count = counter("g")
#table(
  rows: (auto, 2em, auto, auto),
  table.header(
    [eeec],
    table.cell(rowspan: 2, count.step() + context count.display()),
  ),
  [d],
  block(width: 5em, fill: yellow, lorem(7)),
  [d],
  table.footer(
    [eeec],
    table.cell(rowspan: 2, count.step() + context count.display()),
  )
)
#context count.display()

--- grid-nested-with-footers ---
// Nested table with footer should repeat both footers
#set page(height: 10em, width: auto)
#table(
  table(
    [a\ b\ c\ d],
    table.footer[b],
  ),
  table.footer[a],
)

--- grid-nested-footers ---
#set page(height: 12em, width: auto)
#table(
  [a\ b\ c\ d],
  table.footer(table(
    [c],
    [d],
    table.footer[b],
  ))
)

--- grid-footer-rowspan ---
// General footer-only tests
#set page(height: 9em)
#table(
  columns: 2,
  [a], [],
  [b], [],
  [c], [],
  [d], [],
  [e], [],
  table.footer(
    [*Ok*], table.cell(rowspan: 2)[test],
    [*Thanks*]
  )
)

--- grid-footer-bare-1 ---
#set page(height: 5em)
#table(
  table.footer[a][b][c]
)

--- grid-footer-bare-2 ---
#table(table.footer[a][b][c])

#table(
  gutter: 3pt,
  table.footer[a][b][c]
)

--- grid-footer-stroke-edge-cases ---
// Test footer stroke priority edge case
#set page(height: 10em)
#table(
  columns: 2,
  stroke: black,
  ..(table.cell(stroke: aqua)[d],) * 8,
  table.footer(
    table.cell(rowspan: 2, colspan: 2)[a],
    [c], [d]
  )
)

--- grid-footer-hline-and-vline-1 ---
// Footer should appear at the bottom. Red line should be above the footer.
// Green line should be on the left border.
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 2,
  inset: 1.5pt,
  table.cell(y: 0)[a],
  table.cell(x: 1, y: 1)[a],
  table.cell(y: 2)[a],
  table.footer(
    table.hline(stroke: red),
    table.vline(stroke: green),
    [b],
    [c]
  ),
)

--- grid-footer-hline-and-vline-2 ---
// Table should be just one row. [c] appears at the third column.
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 3,
  inset: 1.5pt,
  table.footer(
    table.cell(y: 0)[a],
    table.hline(stroke: red),
    table.hline(y: 1, stroke: aqua),
    table.cell(y: 0)[b],
    [c]
  )
)

--- grid-footer-below-rowspans ---
// Footer should go below the rowspans.
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 2,
  inset: 1.5pt,
  table.cell(rowspan: 2)[a], table.cell(rowspan: 2)[b],
  table.footer()
)

--- grid-footer-row-pos-cell-inside-conflicts-with-row-before ---
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 3,
  inset: 1.5pt,
  table.cell(y: 0)[a],
  table.footer(
    table.hline(stroke: red),
    table.hline(y: 1, stroke: aqua),
    // Error: 5-24 cell would cause footer to expand to non-empty row 0
    // Hint: 5-24 try moving its cells to available rows
    table.cell(y: 0)[b],
    [c]
  )
)

--- grid-footer-auto-pos-cell-inside-conflicts-with-row-after ---
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 2,
  inset: 1.5pt,
  table.cell(y: 1)[a],
  table.footer(
    [b], [c],
    // Error: 6-7 cell would cause footer to expand to non-empty row 1
    // Hint: 6-7 try moving its cells to available rows
    [d],
  ),
)

--- grid-footer-row-pos-cell-inside-conflicts-with-row-after ---
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 2,
  inset: 1.5pt,
  table.cell(y: 2)[a],
  table.footer(
    [b], [c],
    // Error: 5-24 cell would cause footer to expand to non-empty row 2
    // Hint: 5-24 try moving its cells to available rows
    table.cell(y: 3)[d],
  ),
)

--- grid-footer-conflicts-with-empty-header ---
#table(
  columns: 2,
  table.header(),
  table.footer(
    // Error: 5-24 cell would cause footer to expand to non-empty row 0
    // Hint: 5-24 try moving its cells to available rows
    table.cell(y: 0)[a]
  ),
)

--- issue-5359-column-override-stays-inside-footer ---
#table(
  columns: 3,
  [Outside],
  table.footer(
    [A], table.cell(x: 1)[B], [C],
    table.cell(x: 1)[D],
  ),
)
