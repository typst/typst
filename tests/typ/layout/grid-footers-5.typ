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

---
#set page(height: 5em)
#table(
  table.footer[a][b][c]
)

---
#table(table.footer[a][b][c])

#table(
  gutter: 3pt,
  table.footer[a][b][c]
)

---
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

---
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
  ),
  table.cell(x: 1, y: 3)[c]
)

---
// Table should be just one row. [c] appears at the third column.
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 3,
  inset: 1.5pt,
  table.cell(y: 0)[a],
  table.footer(
    table.hline(stroke: red),
    table.hline(y: 1, stroke: aqua),
    table.cell(y: 0)[b],
    [c]
  )
)

---
// Footer should go below the rowspans.
#set page(margin: 2pt)
#set text(6pt)
#table(
  columns: 2,
  inset: 1.5pt,
  table.cell(rowspan: 2)[a], table.cell(rowspan: 2)[b],
  table.footer()
)
