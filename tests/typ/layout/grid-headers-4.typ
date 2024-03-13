// When a header has a rowspan with an empty row, it should be displayed
// properly
#set page(height: 10em)

#let count = counter("g")
#table(
  rows: (auto, 2em, auto, auto),
  table.header(
    [eeec],
    table.cell(rowspan: 2, count.step() + count.display()),
  ),
  [d],
  block(width: 5em, fill: yellow, lorem(15)),
  [d]
)
#count.display()

---
// Ensure header expands to fit cell placed in it after its declaration
#set page(height: 10em)
#table(
  columns: 2,
  table.header(
    [a], [b],
    [c],
  ),
  table.cell(x: 1, y: 1, rowspan: 2, lorem(80))
)

---
// Nested table with header should repeat both headers
#set page(height: 10em)
#table(
  table.header(
    [a]
  ),
  table(
    table.header(
      [b]
    ),
    [a\ b\ c\ d]
  )
)

---
#set page(height: 12em)
#table(
  table.header(
    table(
      table.header(
        [b]
      ),
      [c],
      [d]
    )
  ),
  [a\ b]
)

---
// Test header stroke priority edge case (last header row removed)
#set page(height: 8em)
#table(
  columns: 2,
  stroke: black,
  gutter: (auto, 3pt),
  table.header(
    [c], [d],
  ),
  ..(table.cell(stroke: aqua)[d],) * 8,
)

---
// Yellow line should be kept here
#set text(6pt)
#table(
  column-gutter: 3pt,
  inset: 1pt,
  table.header(
    [a],
    table.hline(stroke: yellow),
  ),
  table.cell(rowspan: 2)[b]
)

---
// Red line should be kept here
#set page(height: 6em)
#set text(6pt)
#table(
  column-gutter: 3pt,
  inset: 1pt,
  table.header(
    table.hline(stroke: red, position: bottom),
    [a],
  ),
  [a],
  table.cell(stroke: aqua)[b]
)

---
#set page(height: 7em)
#set text(6pt)
#let full-block = block(width: 2em, height: 100%, fill: red)
#table(
  columns: 3,
  inset: 1.5pt,
  table.header(
    [a], full-block, table.cell(rowspan: 2, full-block),
    [b]
  )
)
