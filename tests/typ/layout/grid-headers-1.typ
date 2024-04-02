#set page(width: auto, height: 12em)
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..range(0, 6).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

---
// Disable repetition
#set page(width: auto, height: 12em)
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow),
    repeat: false
  ),
  ..range(0, 6).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

---
#set page(width: auto, height: 12em)
#table(
  columns: 5,
  align: center + horizon,
  gutter: 3pt,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow),
  ),
  ..range(0, 6).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

---
// Relative lengths
#set page(height: 10em)
#table(
  rows: (30%, 30%, auto),
  table.header(
    [*A*],
    [*B*]
  ),
  [C],
  [C]
)

---
#grid(
  grid.cell(y: 1)[a],
  grid.header(grid.cell(y: 0)[b]),
  grid.cell(y: 2)[c]
)

---
// When the header is the last grid child, it shouldn't include the gutter row
// after it, because there is none.
#grid(
  columns: 2,
  gutter: 3pt,
  grid.header(
    [a], [b],
    [c], [d]
  )
)

---
#set page(height: 14em)
#let t(n) = table(
  columns: 3,
  align: center + horizon,
  gutter: 3pt,
  table.header(
    table.cell(colspan: 3)[*Cool Zone #n*],
    [*Name*], [*Num*], [*Data*]
  ),
  ..range(0, 5).map(i => ([\##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456])).flatten()
)
#grid(
  gutter: 3pt,
  t(0),
  t(1)
)

---
// Test line positioning in header
#table(
  columns: 3,
  stroke: none,
  table.hline(stroke: red, end: 2),
  table.vline(stroke: red, end: 3),
  table.header(
    table.hline(stroke: aqua, start: 2),
    table.vline(stroke: aqua, start: 3), [*A*], table.hline(stroke: orange), table.vline(stroke: orange), [*B*],
    [*C*], [*D*]
  ),
  [a], [b],
  [c], [d],
  [e], [f]
)

---
// Error: 3:3-3:19 header must start at the first row
// Hint: 3:3-3:19 remove any rows before the header
#grid(
  [a],
  grid.header([b])
)

---
// Error: 4:3-4:19 header must start at the first row
// Hint: 4:3-4:19 remove any rows before the header
#grid(
  columns: 2,
  [a],
  grid.header([b])
)

---
// Error: 3:3-3:19 cannot have more than one header
#grid(
  grid.header([a]),
  grid.header([b]),
  [a],
)

---
// Error: 2:3-2:20 cannot use `table.header` as a grid header; use `grid.header` instead
#grid(
  table.header([a]),
  [a],
)

---
// Error: 2:3-2:19 cannot use `grid.header` as a table header; use `table.header` instead
#table(
  grid.header([a]),
  [a],
)

---
// Error: 14-28 cannot place a grid header within another header or footer
#grid.header(grid.header[a])

---
// Error: 14-29 cannot place a table header within another header or footer
#grid.header(table.header[a])

---
// Error: 15-29 cannot place a grid header within another header or footer
#table.header(grid.header[a])

---
// Error: 15-30 cannot place a table header within another header or footer
#table.header(table.header[a])
