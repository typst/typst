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
