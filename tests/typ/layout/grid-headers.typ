#set page(width: auto, height: 15em)
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..range(0, 15).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

---
// Disable repetition
#set page(width: auto, height: 15em)
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow),
    repeat: false
  ),
  ..range(0, 15).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

---
#set page(width: auto, height: 15em)
#table(
  columns: 5,
  align: center + horizon,
  gutter: 3pt,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow),
  ),
  ..range(0, 15).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

---
#set page(width: auto, height: 15em)
#table(
  columns: 4,
  align: center + horizon,
  table.header(),
  ..range(0, 15).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789])).flatten()
)

---
#set page(height: 15em)
#table(
  rows: (auto, 2.5em, auto),
  table.header(
    [*Hello*],
    [*World*]
  ),
  block(width: 2em, height: 20em, fill: red)
)

---
#set page(height: 25em)

#table(
  rows: (auto, 2.5em, 4em, auto, 10em),
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lorem(80))
)

---
#set page(height: 25em)

#table(
  rows: (auto, 2.5em, 4em, auto, 10em),
  gutter: 3pt,
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lorem(80))
)

---
#set page(height: 25em)

#table(
  rows: (auto, 2.5em, 4em, auto),
  gutter: 3pt,
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lorem(80))
)

---
#set page(height: 25em, margin: (top: 20pt + 3pt))

#table(
  rows: (auto, 4em, auto, 10em),
  gutter: 3pt,
  table.cell(rowspan: 3, lorem(80))
)

---
// Orphan header test
#set page(height: 12em)
#v(8em)
#grid(
  columns: 3,
  grid.header(
    [*Mui*], [*A*], grid.cell(rowspan: 2, fill: orange)[*B*],
    [*Header*], [*Header* #v(0.1em)]
  ),
  ..([Test], [Test], [Test]) * 20
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
  [C],
  [C],
  [C],
  [C],
  [C],
  [C],
  [C],
  [C],
)

---
#set page(height: 9em)

#table(
  rows: (auto, 2.5em, auto, auto, 10em),
  gutter: 3pt,
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lorem(80))
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
  gutter: 3pt,
  grid.header(
    [a],
    [b]
  )
)

---
// Table with just a header should work normally
#table(
  columns: 2,
  gutter: 3pt,
  table.header(
    [a], [b],
    [c], [d]
  )
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
// Error: 14-28 cannot place a grid header within another header
#grid.header(grid.header[a])

---
// Error: 14-29 cannot place a table header within another header
#grid.header(table.header[a])

---
// Error: 15-29 cannot place a grid header within another header
#table.header(grid.header[a])

---
// Error: 15-30 cannot place a table header within another header
#table.header(table.header[a])
