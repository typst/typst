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

---
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

---
#table(
  table.header(table.cell(stroke: red)[Hello]),
  table.footer(table.cell(stroke: aqua)[Bye]),
)

---
#table(
  gutter: 3pt,
  table.header(table.cell(stroke: red)[Hello]),
  table.footer(table.cell(stroke: aqua)[Bye]),
)

---
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

---
// Relative lengths
#set page(height: 10em)
#table(
  rows: (30%, 30%, auto),
  [C],
  [C],
  table.footer[*A*][*B*],
)

---
#grid(
  grid.footer(grid.cell(y: 2)[b]),
  grid.cell(y: 0)[a],
  grid.cell(y: 1)[c],
)

---
// Ensure footer properly expands
#grid(
  columns: 2,
  [a], [],
  [b], [],
  grid.cell(x: 1, y: 3, rowspan: 4)[b],
  grid.cell(y: 2, rowspan: 2)[a],
  grid.footer(),
  grid.cell(y: 4)[d],
  grid.cell(y: 5)[e],
  grid.cell(y: 6)[f],
)

---
// Error: 2:3-2:19 footer must end at the last row
#grid(
  grid.footer([a]),
  [b],
)

---
// Error: 3:3-3:19 footer must end at the last row
#grid(
  columns: 2,
  grid.footer([a]),
  [b],
)

---
// Error: 4:3-4:19 footer would conflict with a cell placed before it at column 1 row 0
// Hint: 4:3-4:19 try reducing that cell's rowspan or moving the footer
#grid(
  columns: 2,
  grid.header(),
  grid.footer([a]),
  grid.cell(x: 1, y: 0, rowspan: 2)[a],
)

---
// Error: 4:3-4:19 cannot have more than one footer
#grid(
  [a],
  grid.footer([a]),
  grid.footer([b]),
)

---
// Error: 3:3-3:20 cannot use `table.footer` as a grid footer; use `grid.footer` instead
#grid(
  [a],
  table.footer([a]),
)

---
// Error: 3:3-3:19 cannot use `grid.footer` as a table footer; use `table.footer` instead
#table(
  [a],
  grid.footer([a]),
)

---
// Error: 14-28 cannot place a grid footer within another footer or header
#grid.header(grid.footer[a])

---
// Error: 14-29 cannot place a table footer within another footer or header
#grid.header(table.footer[a])

---
// Error: 15-29 cannot place a grid footer within another footer or header
#table.header(grid.footer[a])

---
// Error: 15-30 cannot place a table footer within another footer or header
#table.header(table.footer[a])

---
// Error: 14-28 cannot place a grid footer within another footer or header
#grid.footer(grid.footer[a])

---
// Error: 14-29 cannot place a table footer within another footer or header
#grid.footer(table.footer[a])

---
// Error: 15-29 cannot place a grid footer within another footer or header
#table.footer(grid.footer[a])

---
// Error: 15-30 cannot place a table footer within another footer or header
#table.footer(table.footer[a])

---
// Error: 14-28 cannot place a grid header within another header or footer
#grid.footer(grid.header[a])

---
// Error: 14-29 cannot place a table header within another header or footer
#grid.footer(table.header[a])

---
// Error: 15-29 cannot place a grid header within another header or footer
#table.footer(grid.header[a])

---
// Error: 15-30 cannot place a table header within another header or footer
#table.footer(table.header[a])
