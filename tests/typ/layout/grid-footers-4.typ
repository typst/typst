// When a footer has a rowspan with an empty row, it should be displayed
// properly
#set page(height: 14em, width: auto)

#let count = counter("g")
#table(
  rows: (auto, 2em, auto, auto),
  table.header(
    [eeec],
    table.cell(rowspan: 2, count.step() + count.display()),
  ),
  [d],
  block(width: 5em, fill: yellow, lorem(7)),
  [d],
  table.footer(
    [eeec],
    table.cell(rowspan: 2, count.step() + count.display()),
  )
)
#count.display()

---
// Nested table with footer should repeat both footers
#set page(height: 10em, width: auto)
#table(
  table(
    [a\ b\ c\ d],
    table.footer[b],
  ),
  table.footer[a],
)

---
#set page(height: 12em, width: auto)
#table(
  [a\ b\ c\ d],
  table.footer(table(
    [c],
    [d],
    table.footer[b],
  ))
)
