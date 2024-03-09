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
